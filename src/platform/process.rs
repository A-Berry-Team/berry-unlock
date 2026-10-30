//! Process discovery and remote memory access.

use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW, MODULEENTRY32W,
    PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};

use super::handle_is_null;
use crate::engine::memory::MemoryReader;
use crate::engine::wide::wide_to_string;
use crate::error::{Error, Result};
use crate::{log_debug, log_info, log_warn};

// Least privilege (the original requested PROCESS_ALL_ACCESS).
const PROCESS_VM_OPERATION: u32 = 0x0008;
const PROCESS_VM_READ: u32 = 0x0010;
const PROCESS_VM_WRITE: u32 = 0x0020;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const SYNCHRONIZE: u32 = 0x0010_0000;
const ACCESS: u32 =
    PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE;

const WAIT_TIMEOUT: u32 = 0x0000_0102;
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Owned toolhelp snapshot, closed on drop.
struct Snapshot(HANDLE);

impl Snapshot {
    fn new(flags: u32, pid: u32) -> Option<Self> {
        // SAFETY: plain FFI call with value arguments.
        let h = unsafe { CreateToolhelp32Snapshot(flags, pid) };
        if h == INVALID_HANDLE_VALUE || handle_is_null(h) {
            None
        } else {
            Some(Snapshot(h))
        }
    }
}

impl Drop for Snapshot {
    fn drop(&mut self) {
        // SAFETY: handle was returned by CreateToolhelp32Snapshot and is closed once.
        unsafe { CloseHandle(self.0) };
    }
}

pub fn find_pid_by_name(name: &str) -> Option<u32> {
    let snap = Snapshot::new(TH32CS_SNAPPROCESS, 0)?;
    // SAFETY: PROCESSENTRY32W is plain old data; all-zero is a valid value.
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    // SAFETY: `entry` is valid for writes and dwSize is set as required.
    let mut more = unsafe { Process32FirstW(snap.0, &mut entry) } != 0;
    while more {
        if wide_to_string(&entry.szExeFile).eq_ignore_ascii_case(name) {
            return Some(entry.th32ProcessID);
        }
        // SAFETY: as above.
        more = unsafe { Process32NextW(snap.0, &mut entry) } != 0;
    }
    None
}

pub fn find_module_base(pid: u32, module: &str) -> Option<usize> {
    let snap = Snapshot::new(TH32CS_SNAPMODULE, pid)?;
    // SAFETY: MODULEENTRY32W is plain old data; all-zero is a valid value.
    let mut entry: MODULEENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<MODULEENTRY32W>() as u32;
    // SAFETY: `entry` is valid for writes and dwSize is set as required.
    let mut more = unsafe { Module32FirstW(snap.0, &mut entry) } != 0;
    while more {
        if wide_to_string(&entry.szModule).eq_ignore_ascii_case(module) {
            return Some(entry.modBaseAddr as usize);
        }
        // SAFETY: as above.
        more = unsafe { Module32NextW(snap.0, &mut entry) } != 0;
    }
    None
}

/// Handle to another process with only the rights we need.
pub struct RemoteProcess {
    handle: HANDLE,
    pid: u32,
}

impl RemoteProcess {
    pub fn open(pid: u32) -> Result<Self> {
        // SAFETY: plain FFI call with value arguments.
        let handle = unsafe { OpenProcess(ACCESS, 0, pid) };
        if handle_is_null(handle) {
            // SAFETY: GetLastError has no preconditions.
            Err(Error::Os { context: "OpenProcess", code: unsafe { GetLastError() } })
        } else {
            Ok(RemoteProcess { handle, pid })
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Waits up to `ms` for the process to exit. Returns true if it exited
    /// (or the wait failed), false on timeout. Doubles as an interruptible sleep.
    pub fn wait_exit(&self, ms: u32) -> bool {
        // SAFETY: valid process handle with SYNCHRONIZE access.
        unsafe { WaitForSingleObject(self.handle, ms) != WAIT_TIMEOUT }
    }

    pub fn write_u32(&self, address: usize, value: u32) -> Result<()> {
        let mut written: usize = 0;
        // SAFETY: the source is a live local of exactly 4 bytes; the destination
        // is validated by the kernel (failure is reported, not UB).
        let ok = unsafe {
            WriteProcessMemory(
                self.handle,
                address as *const c_void,
                &value as *const u32 as *const c_void,
                size_of::<u32>(),
                &mut written,
            )
        };
        if ok != 0 && written == size_of::<u32>() {
            Ok(())
        } else {
            // SAFETY: GetLastError has no preconditions.
            Err(Error::Os { context: "WriteProcessMemory", code: unsafe { GetLastError() } })
        }
    }
}

impl MemoryReader for RemoteProcess {
    fn read_memory(&self, address: usize, buf: &mut [u8]) -> usize {
        let mut read: usize = 0;
        // SAFETY: `buf` is valid for `buf.len()` bytes of writes.
        let ok = unsafe {
            ReadProcessMemory(
                self.handle,
                address as *const c_void,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
                &mut read,
            )
        };
        // A failed call may still have copied a prefix (ERROR_PARTIAL_COPY).
        if ok != 0 || read > 0 {
            read.min(buf.len())
        } else {
            0
        }
    }
}

impl Drop for RemoteProcess {
    fn drop(&mut self) {
        // SAFETY: handle came from OpenProcess and is closed exactly once.
        unsafe { CloseHandle(self.handle) };
    }
}

/// Polls until `exe` is running and its main module is queryable.
pub fn wait_for_module(exe: &str, timeout: Option<Duration>) -> Result<(RemoteProcess, usize)> {
    let start = Instant::now();
    let mut reported_open_error = false;
    let mut last_error: Option<Error> = None;
    log_info!("waiting for {exe}");

    loop {
        if let Some(pid) = find_pid_by_name(exe) {
            match RemoteProcess::open(pid) {
                Ok(process) => {
                    if let Some(base) = find_module_base(pid, exe) {
                        log_info!("{exe}: pid {} module base 0x{base:X}", process.pid());
                        return Ok((process, base));
                    }
                    log_debug!("{exe} (pid {pid}) found but its module list is not ready");
                }
                Err(e) => {
                    if !reported_open_error {
                        log_warn!("cannot open {exe} (pid {pid}): {e}. Is BerryUnlock running inside the same Wine prefix?");
                        reported_open_error = true;
                    }
                    last_error = Some(e);
                }
            }
        }
        if let Some(limit) = timeout {
            if start.elapsed() >= limit {
                let detail = last_error.map(|e| format!(" Last error: {e}.")).unwrap_or_default();
                return Err(Error::Timeout(format!(
                    "{exe} did not become ready within {} s.{detail}",
                    limit.as_secs()
                )));
            }
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}
