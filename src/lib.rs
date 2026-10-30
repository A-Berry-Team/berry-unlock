//! BerryUnlock core library.
//!
//! Layout:
//! * `engine`   - portable logic (PE parsing, pattern scanning, JSON/INI patching).
//!                Compiles and is unit-tested on any host.
//! * `platform` - Win32 / Wine bindings. **The only place `unsafe` is allowed.**
//! * `games`    - one module per supported game, plus dispatch.
//! * `cli`, `app`, `ui`, `logging`, `error`, `game_id` - plumbing.

#![deny(unsafe_code)]

pub mod app;
pub mod cli;
pub mod engine;
pub mod error;
pub mod game_id;
pub mod games;
pub mod logging;
pub mod ui;

#[cfg(windows)]
pub mod platform;
