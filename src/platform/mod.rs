//! Win32 / Wine bindings via `windows-sys` (raw FFI, zero runtime overhead).
//!
//! This is the ONLY module of the crate allowed to contain `unsafe`. Every
//! unsafe block wraps exactly one FFI call (or a zero-initialised POD struct)
//! behind a safe, RAII-managed API.

#![allow(unsafe_code)]

pub mod dialog;
pub mod process;
pub mod registry;

use windows_sys::Win32::Foundation::HANDLE;

/// Works whether `HANDLE` is an integer or a pointer in the bindings in use.
pub(crate) fn handle_is_null(h: HANDLE) -> bool {
    h as usize == 0
}
