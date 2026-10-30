//! Message boxes (kept topmost so they are not hidden behind a fullscreen game).

use std::mem::zeroed;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;

use crate::engine::wide::to_wide;

const MB_OK: u32 = 0x0000;
const MB_ICONWARNING: u32 = 0x0030;
const MB_ICONINFORMATION: u32 = 0x0040;
const MB_SETFOREGROUND: u32 = 0x0001_0000;
const MB_TOPMOST: u32 = 0x0004_0000;

pub fn message_box(title: &str, text: &str, informational: bool) {
    let title = to_wide(title);
    let text = to_wide(text);
    let icon = if informational { MB_ICONINFORMATION } else { MB_ICONWARNING };
    // SAFETY: both buffers are NUL-terminated and outlive the call; a null
    // owner window is explicitly allowed by MessageBoxW.
    unsafe {
        let owner: HWND = zeroed();
        MessageBoxW(owner, text.as_ptr(), title.as_ptr(), MB_OK | icon | MB_SETFOREGROUND | MB_TOPMOST);
    }
}
