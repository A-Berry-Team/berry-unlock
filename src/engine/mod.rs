//! Portable engine: everything here is pure logic over byte slices, strings and
//! the `MemoryReader` trait, so it can be unit-tested on any host.

pub mod fps_locator;
pub mod fsutil;
pub mod ini_edit;
pub mod json_patch;
pub mod memory;
pub mod pattern;
pub mod pe;
pub mod reg_match;

#[cfg(windows)]
pub mod wide;
