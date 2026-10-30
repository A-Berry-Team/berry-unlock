//! Minimal dependency-free logger: stderr + optional append-only log file.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

const MAX_LOG_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

struct Logger {
    start: Instant,
    verbose: bool,
    file: Mutex<Option<File>>,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Initialise once. The log file is truncated if it grew beyond 1 MiB.
pub fn init(path: Option<&Path>, verbose: bool) {
    let file = path.and_then(|p| {
        let too_big = std::fs::metadata(p).map(|m| m.len() > MAX_LOG_BYTES).unwrap_or(false);
        let mut opts = OpenOptions::new();
        opts.create(true);
        if too_big {
            opts.write(true).truncate(true);
        } else {
            opts.append(true);
        }
        opts.open(p).ok()
    });
    let _ = LOGGER.set(Logger { start: Instant::now(), verbose, file: Mutex::new(file) });
}

pub fn log(level: Level, args: fmt::Arguments<'_>) {
    let Some(l) = LOGGER.get() else { return };
    if level == Level::Debug && !l.verbose {
        return;
    }
    let line = format!("[{:>9.3}s] {:<5} {}\n", l.start.elapsed().as_secs_f64(), level.as_str(), args);
    let _ = std::io::stderr().write_all(line.as_bytes());
    if let Ok(mut guard) = l.file.lock() {
        if let Some(f) = guard.as_mut() {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

#[macro_export]
macro_rules! log_debug { ($($arg:tt)*) => { $crate::logging::log($crate::logging::Level::Debug, format_args!($($arg)*)) }; }
#[macro_export]
macro_rules! log_info { ($($arg:tt)*) => { $crate::logging::log($crate::logging::Level::Info, format_args!($($arg)*)) }; }
#[macro_export]
macro_rules! log_warn { ($($arg:tt)*) => { $crate::logging::log($crate::logging::Level::Warn, format_args!($($arg)*)) }; }
#[macro_export]
macro_rules! log_error { ($($arg:tt)*) => { $crate::logging::log($crate::logging::Level::Error, format_args!($($arg)*)) }; }
