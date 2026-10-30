//! User-visible output. Message boxes are only used on Windows/Wine and can be
//! disabled with `--no-dialog`.

use crate::log_warn;

#[cfg(windows)]
fn show(title: &str, text: &str, informational: bool) {
    crate::platform::dialog::message_box(title, text, informational);
}

#[cfg(not(windows))]
fn show(title: &str, text: &str, _informational: bool) {
    eprintln!("[{title}] {text}");
}

/// Warning/error notice. Always logged; shown as a dialog unless disabled.
pub fn notice(dialogs: bool, title: &str, text: &str) {
    log_warn!("{title}: {text}");
    if dialogs {
        show(title, text, false);
    }
}

/// Output of `games` / `--help` / `--version`.
/// Console builds print to stdout; the default GUI build uses a message box.
pub fn print_info(dialogs: bool, text: &str) {
    if cfg!(feature = "console") || !cfg!(windows) {
        println!("{text}");
    } else if dialogs {
        show("BerryUnlock", text, true);
    }
}
