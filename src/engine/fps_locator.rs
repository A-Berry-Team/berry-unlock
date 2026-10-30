//! Locates the Genshin Impact FPS variable inside a running process.
//!
//! Technique (inherited from KeqingUnlock): the game executes
//!
//! ```text
//! B9 3C 00 00 00        mov  ecx, 60
//! E8 ?? ?? ?? ??        call set_target_frame_rate
//! 80 ...                cmp  byte ptr [...], ...
//! ```
//!
//! `set_target_frame_rate` is (possibly through `jmp`/`call` thunks) a function
//! beginning with `89 0D rel32` = `mov [rip+rel32], ecx`, which stores the FPS
//! into a global. We resolve that global's address.
//!
//! Hardening over the original:
//! * every candidate hit is tried (the original only looked at the first);
//! * the resolved address must lie inside the module image and be readable,
//!   so a false-positive pattern can never make us write to arbitrary memory;
//! * relative-offset maths uses wrapping arithmetic on integer addresses
//!   instead of pointer arithmetic on foreign addresses.

use super::memory::MemoryReader;
use super::pattern::{scan_region, Pattern};
use super::pe;
use crate::error::{Error, Result};

pub const MAX_JUMP_DEPTH: usize = 32;

pub fn setter_pattern() -> Pattern {
    Pattern::new(&[
        Some(0xB9), Some(0x3C), Some(0x00), Some(0x00), Some(0x00), // mov ecx, 60
        Some(0xE8), None, None, None, None,                         // call rel32
        Some(0x80),                                                 // cmp byte ptr
    ])
    .expect("setter pattern has fixed bytes")
}

pub fn locate_fps_variable<R: MemoryReader>(reader: &R, module_base: usize) -> Result<usize> {
    let mut header = vec![0u8; pe::HEADER_READ_SIZE];
    let got = reader.read_memory(module_base, &mut header);
    if got == 0 {
        return Err(Error::PatternNotFound("cannot read the game module header".into()));
    }
    let image = pe::parse(&header[..got])?;
    let image_end = module_base
        .checked_add(image.size_of_image as usize)
        .ok_or_else(|| Error::InvalidData("module size overflows the address space".into()))?;

    let pattern = setter_pattern();
    for section in image.sections.iter().filter(|s| s.is_executable()) {
        let Some(start) = module_base.checked_add(section.virtual_address as usize) else { continue };
        let found = scan_region(reader, start, section.scan_len(), &pattern, |hit| {
            resolve_from_hit(reader, hit, module_base, image_end)
        });
        if let Some(addr) = found {
            return Ok(addr);
        }
    }
    Err(Error::PatternNotFound("Failed to locate FPS variable in game memory.".into()))
}

fn resolve_from_hit<R: MemoryReader>(reader: &R, hit: usize, lo: usize, hi: usize) -> Option<usize> {
    // `hit` points at `mov ecx, 60`; the call instruction is 5 bytes later.
    let mut cursor = hit.checked_add(5)?;
    let mut insn = [0u8; 6];
    let mut settled = false;

    for _ in 0..MAX_JUMP_DEPTH {
        if !reader.read_exact_at(cursor, &mut insn) {
            return None;
        }
        match insn[0] {
            0xE8 | 0xE9 => {
                let rel = i32::from_le_bytes([insn[1], insn[2], insn[3], insn[4]]);
                cursor = cursor.wrapping_add(5).wrapping_add(rel as isize as usize);
            }
            _ => {
                settled = true;
                break;
            }
        }
    }
    if !settled || insn[0] != 0x89 || insn[1] != 0x0D {
        return None;
    }

    let disp = i32::from_le_bytes([insn[2], insn[3], insn[4], insn[5]]);
    let var = cursor.wrapping_add(6).wrapping_add(disp as isize as usize);
    if var < lo || var.checked_add(4)? > hi {
        return None;
    }
    reader.read_u32_at(var)?; // must be readable
    Some(var)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::memory::mock::{pe_header, MockMemory};

    const BASE: usize = 0x1_4000_0000;
    const TEXT_RVA: usize = 0x1000;
    const VAR_RVA: usize = 0x4000;

    fn put_hit(text: &mut [u8], hit: usize, target: usize) {
        text[hit..hit + 5].copy_from_slice(&[0xB9, 0x3C, 0, 0, 0]);
        text[hit + 5] = 0xE8;
        let rel = (target as i64 - (hit as i64 + 10)) as i32;
        text[hit + 6..hit + 10].copy_from_slice(&rel.to_le_bytes());
        text[hit + 10] = 0x80;
        text[hit + 11] = 0x3D;
    }

    fn put_setter(text: &mut [u8], at: usize, var_rva: usize) {
        let disp = (var_rva as i64 - ((TEXT_RVA + at) as i64 + 6)) as i32;
        text[at] = 0x89;
        text[at + 1] = 0x0D;
        text[at + 2..at + 6].copy_from_slice(&disp.to_le_bytes());
    }

    fn image(text: Vec<u8>) -> MockMemory {
        MockMemory::new()
            .map(BASE, pe_header(0x6000, &[(TEXT_RVA as u32, 0x2000, 0x6000_0020)]))
            .map(BASE + TEXT_RVA, text)
            .map(BASE + VAR_RVA, 60u32.to_le_bytes().to_vec())
    }

    #[test]
    fn resolves_through_jump_thunk_and_skips_decoy() {
        let mut text = vec![0x90u8; 0x2000];
        // Decoy: call lands on NOPs, never a `mov [rip+x], ecx`.
        put_hit(&mut text, 0x20, 0x800);
        // Real hit: call -> thunk (jmp) -> setter.
        put_hit(&mut text, 0x100, 0x400);
        let rel = (0x600i64 - (0x400i64 + 5)) as i32;
        text[0x400] = 0xE9;
        text[0x401..0x405].copy_from_slice(&rel.to_le_bytes());
        put_setter(&mut text, 0x600, VAR_RVA);

        let mem = image(text);
        assert_eq!(locate_fps_variable(&mem, BASE).unwrap(), BASE + VAR_RVA);
    }

    #[test]
    fn rejects_variable_outside_image() {
        let mut text = vec![0x90u8; 0x2000];
        put_hit(&mut text, 0x100, 0x600);
        put_setter(&mut text, 0x600, 0x9000); // beyond SizeOfImage (0x6000)
        let mem = image(text).map(BASE + 0x9000, vec![0; 4]);
        assert!(locate_fps_variable(&mem, BASE).is_err());
    }

    #[test]
    fn errors_when_pattern_absent() {
        let mem = image(vec![0x90u8; 0x2000]);
        assert!(matches!(locate_fps_variable(&mem, BASE), Err(Error::PatternNotFound(_))));
    }

    #[test]
    fn errors_on_unreadable_module() {
        assert!(locate_fps_variable(&MockMemory::new(), BASE).is_err());
    }
}
