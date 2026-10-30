// GUI subsystem by default: no console window under Wine/Proton.
// Build with `--features console` for a terminal-friendly binary.
#![cfg_attr(all(windows, not(feature = "console")), windows_subsystem = "windows")]

fn main() {
    let args = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned());
    std::process::exit(berry_core::app::run(args));
}
