//! Minimal registry wrapper (HKEY_CURRENT_USER, raw values).

use std::ptr::{null, null_mut};

use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
};

use crate::engine::wide::to_wide;
use crate::error::{Error, Result};

const ERROR_SUCCESS: u32 = 0;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_MORE_DATA: u32 = 234;
const ERROR_NO_MORE_ITEMS: u32 = 259;

const KEY_QUERY_VALUE: u32 = 0x0001;
const KEY_SET_VALUE: u32 = 0x0002;
const KEY_ENUMERATE_SUB_KEYS: u32 = 0x0008;
const KEY_NOTIFY: u32 = 0x0010;
const READ_ACCESS: u32 = KEY_QUERY_VALUE | KEY_ENUMERATE_SUB_KEYS | KEY_NOTIFY;

/// Registry value names are at most 16,383 characters.
const MAX_VALUE_NAME: usize = 16_384;

fn os_err(context: &'static str, code: u32) -> Error {
    Error::Os { context, code }
}

pub struct RegistryKey {
    hkey: HKEY,
}

impl RegistryKey {
    pub fn open_hkcu(subkey: &str, write: bool) -> Result<Self> {
        let access = if write { READ_ACCESS | KEY_SET_VALUE } else { READ_ACCESS };
        let wide = to_wide(subkey);
        // SAFETY: HKEY is plain data; it is overwritten on success.
        let mut hkey: HKEY = unsafe { std::mem::zeroed() };
        // SAFETY: `wide` is NUL-terminated and `hkey` is valid for writes.
        let rc = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, wide.as_ptr(), 0, access, &mut hkey) };
        match rc {
            ERROR_SUCCESS => Ok(RegistryKey { hkey }),
            ERROR_FILE_NOT_FOUND => Err(Error::NotFound(format!("registry key HKCU\\{subkey} not found"))),
            code => Err(os_err("RegOpenKeyExW", code)),
        }
    }

    pub fn value_names(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        let mut buf = vec![0u16; MAX_VALUE_NAME];
        let mut index = 0u32;
        loop {
            let mut len = buf.len() as u32;
            // SAFETY: `buf` is valid for `len` UTF-16 units; the other out
            // parameters are optional and passed as null.
            let rc = unsafe {
                RegEnumValueW(self.hkey, index, buf.as_mut_ptr(), &mut len, null(), null_mut(), null_mut(), null_mut())
            };
            match rc {
                ERROR_SUCCESS => names.push(String::from_utf16_lossy(&buf[..(len as usize).min(buf.len())])),
                ERROR_NO_MORE_ITEMS => return Ok(names),
                code => return Err(os_err("RegEnumValueW", code)),
            }
            index += 1;
        }
    }

    /// Returns `(value_type, bytes)`.
    pub fn get_raw(&self, name: &str) -> Result<(u32, Vec<u8>)> {
        let wide = to_wide(name);
        let mut value_type: u32 = 0;
        let mut size: u32 = 0;
        // SAFETY: size query: null data pointer with a valid size out-parameter.
        let rc = unsafe { RegQueryValueExW(self.hkey, wide.as_ptr(), null(), &mut value_type, null_mut(), &mut size) };
        if rc != ERROR_SUCCESS && rc != ERROR_MORE_DATA {
            return Err(os_err("RegQueryValueExW", rc));
        }
        for _ in 0..4 {
            let mut data = vec![0u8; size as usize + 2];
            let mut cap = data.len() as u32;
            // SAFETY: `data` is valid for `cap` bytes of writes.
            let rc = unsafe {
                RegQueryValueExW(self.hkey, wide.as_ptr(), null(), &mut value_type, data.as_mut_ptr(), &mut cap)
            };
            match rc {
                ERROR_SUCCESS => {
                    data.truncate(cap as usize);
                    return Ok((value_type, data));
                }
                // The value grew between calls: retry with the new size.
                ERROR_MORE_DATA => size = cap,
                code => return Err(os_err("RegQueryValueExW", code)),
            }
        }
        Err(os_err("RegQueryValueExW", ERROR_MORE_DATA))
    }

    pub fn set_raw(&self, name: &str, value_type: u32, data: &[u8]) -> Result<()> {
        let len = u32::try_from(data.len()).map_err(|_| Error::InvalidData("registry value too large".into()))?;
        let wide = to_wide(name);
        // SAFETY: `data` is valid for `len` bytes of reads.
        let rc = unsafe { RegSetValueExW(self.hkey, wide.as_ptr(), 0, value_type, data.as_ptr(), len) };
        if rc == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(os_err("RegSetValueExW", rc))
        }
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        // SAFETY: key was opened by RegOpenKeyExW and is closed exactly once.
        unsafe { RegCloseKey(self.hkey) };
    }
}
