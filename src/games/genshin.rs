//! Genshin Impact: patches the in-memory FPS variable of `GenshinImpact.exe`.

use std::time::{Duration, Instant};

use crate::cli::{Options, RunArgs};
use crate::engine::fps_locator::locate_fps_variable;
use crate::engine::memory::MemoryReader;
use crate::error::{Error, Result};
use crate::platform::process::{wait_for_module, RemoteProcess};
use crate::{log_debug, log_info, log_warn};

const TARGET_EXE: &str = "GenshinImpact.exe";
/// How long we keep re-scanning after `init_delay` (the code is only readable
/// once the game has unpacked itself; Proton start-up times vary a lot).
const LOCATE_WINDOW: Duration = Duration::from_secs(20);
const LOCATE_STEP_MS: u32 = 500;
const MAX_CONSECUTIVE_FAILURES: u32 = 50;
/// Requested FPS at/above this value is written as 60 on the first write
/// (KeqingUnlock behaviour kept for launcher compatibility).
const SENTINEL_FPS: u32 = 500;

fn exited() -> Error {
    Error::ProcessExited(format!("{TARGET_EXE} exited"))
}

pub fn run(options: &Options, args: &RunArgs) -> Result<()> {
    let (process, module_base) = wait_for_module(TARGET_EXE, options.wait_timeout)?;

    if args.init_delay_ms > 0 {
        log_info!("waiting {} ms before scanning", args.init_delay_ms);
        if process.wait_exit(args.init_delay_ms.min(u32::MAX as u64) as u32) {
            return Err(exited());
        }
    }

    let var = locate_with_retry(&process, module_base)?;
    log_info!("FPS variable at 0x{var:X} (current value: {:?})", process.read_u32_at(var));

    let target = args.target_fps;
    let first = if target >= SENTINEL_FPS { 60 } else { target };
    process.write_u32(var, first)?;

    if args.refresh_delay_ms == 0 {
        log_info!("one-shot mode: done");
        return Ok(());
    }

    let delay = args.refresh_delay_ms as u32;
    let mut failures = 0u32;
    loop {
        // Doubles as the sleep: returns immediately when the game exits.
        if process.wait_exit(delay) {
            log_info!("{TARGET_EXE} exited; stopping");
            return Ok(());
        }
        let ok = match process.read_u32_at(var) {
            Some(cur) if cur == target => true,
            Some(_) => process.write_u32(var, target).is_ok(),
            None => false,
        };
        if ok {
            failures = 0;
        } else {
            failures += 1;
            log_debug!("refresh failed ({failures}/{MAX_CONSECUTIVE_FAILURES})");
            if failures >= MAX_CONSECUTIVE_FAILURES {
                return Err(Error::InvalidData(format!(
                    "lost access to the game's memory after {MAX_CONSECUTIVE_FAILURES} consecutive failures"
                )));
            }
        }
    }
}

fn locate_with_retry(process: &RemoteProcess, module_base: usize) -> Result<usize> {
    let deadline = Instant::now() + LOCATE_WINDOW;
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match locate_fps_variable(process, module_base) {
            Ok(addr) => return Ok(addr),
            Err(e) if Instant::now() >= deadline => return Err(e),
            Err(e) => {
                log_debug!("scan attempt {attempt} failed: {e}");
                if attempt == 1 {
                    log_warn!("FPS variable not found yet; retrying for up to {}s", LOCATE_WINDOW.as_secs());
                }
                if process.wait_exit(LOCATE_STEP_MS) {
                    return Err(exited());
                }
            }
        }
    }
}
