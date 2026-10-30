//! Bounds-checked PE32+ header parser (replaces the original's unchecked
//! struct casts over a raw buffer).

use crate::error::{Error, Result};

pub const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;
/// How many bytes of the module start we read to parse headers.
pub const HEADER_READ_SIZE: usize = 0x1000;

const MAX_SECTIONS: usize = 96;
const MACHINE_AMD64: u16 = 0x8664;
const PE32_PLUS_MAGIC: u16 = 0x20B;
const SECTION_HEADER_SIZE: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    pub virtual_address: u32,
    pub virtual_size: u32,
    pub raw_size: u32,
    pub characteristics: u32,
}

impl Section {
    pub fn is_executable(&self) -> bool {
        self.characteristics & IMAGE_SCN_MEM_EXECUTE != 0
    }

    /// Bytes worth scanning in memory for this section.
    pub fn scan_len(&self) -> usize {
        self.virtual_size.max(self.raw_size) as usize
    }
}

#[derive(Debug)]
pub struct PeImage {
    pub size_of_image: u32,
    pub sections: Vec<Section>,
}

fn u16_at(b: &[u8], off: usize) -> Option<u16> {
    let s = b.get(off..off.checked_add(2)?)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    let s = b.get(off..off.checked_add(4)?)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub fn parse(header: &[u8]) -> Result<PeImage> {
    let bad = |what: &str| Error::InvalidData(format!("invalid PE image: {what}"));

    if u16_at(header, 0) != Some(0x5A4D) {
        return Err(bad("missing MZ signature"));
    }
    let nt = u32_at(header, 0x3C).ok_or_else(|| bad("truncated DOS header"))? as usize;
    if nt < 0x40 {
        return Err(bad("e_lfanew out of range"));
    }
    if header.get(nt..nt.saturating_add(4)) != Some(&b"PE\0\0"[..]) {
        return Err(bad("missing PE signature"));
    }
    if u16_at(header, nt + 4) != Some(MACHINE_AMD64) {
        return Err(bad("not an x86-64 image"));
    }
    let section_count = u16_at(header, nt + 6).ok_or_else(|| bad("truncated file header"))? as usize;
    let opt_size = u16_at(header, nt + 20).ok_or_else(|| bad("truncated file header"))? as usize;
    if u16_at(header, nt + 24) != Some(PE32_PLUS_MAGIC) {
        return Err(bad("not a PE32+ optional header"));
    }
    let size_of_image = u32_at(header, nt + 24 + 56).ok_or_else(|| bad("truncated optional header"))?;
    if section_count == 0 || section_count > MAX_SECTIONS {
        return Err(bad("implausible section count"));
    }

    let table = nt + 24 + opt_size;
    let mut sections = Vec::with_capacity(section_count);
    for i in 0..section_count {
        let o = table + i * SECTION_HEADER_SIZE;
        if o + SECTION_HEADER_SIZE > header.len() {
            return Err(bad("section table exceeds header buffer"));
        }
        sections.push(Section {
            virtual_size: u32_at(header, o + 8).ok_or_else(|| bad("truncated section"))?,
            virtual_address: u32_at(header, o + 12).ok_or_else(|| bad("truncated section"))?,
            raw_size: u32_at(header, o + 16).ok_or_else(|| bad("truncated section"))?,
            characteristics: u32_at(header, o + 36).ok_or_else(|| bad("truncated section"))?,
        });
    }
    Ok(PeImage { size_of_image, sections })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::memory::mock::pe_header;

    #[test]
    fn parses_synthetic_header() {
        let h = pe_header(0x6000, &[(0x1000, 0x2000, 0x6000_0020), (0x4000, 0x1000, 0xC000_0040)]);
        let img = parse(&h).unwrap();
        assert_eq!(img.size_of_image, 0x6000);
        assert_eq!(img.sections.len(), 2);
        assert!(img.sections[0].is_executable());
        assert!(!img.sections[1].is_executable());
        assert_eq!(img.sections[0].scan_len(), 0x2000);
        assert_eq!(img.sections[1].virtual_address, 0x4000);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(&[]).is_err());
        assert!(parse(&[0u8; 0x1000]).is_err());
        let mut h = pe_header(0x6000, &[(0x1000, 0x2000, 0x2000_0000)]);
        h[0x80] = b'X';
        assert!(parse(&h).is_err());
    }

    #[test]
    fn rejects_wrong_machine_and_huge_section_count() {
        let mut h = pe_header(0x6000, &[(0x1000, 0x2000, 0x2000_0000)]);
        h[0x84..0x86].copy_from_slice(&0x014Cu16.to_le_bytes());
        assert!(parse(&h).is_err());
        let mut h = pe_header(0x6000, &[(0x1000, 0x2000, 0x2000_0000)]);
        h[0x86..0x88].copy_from_slice(&5000u16.to_le_bytes());
        assert!(parse(&h).is_err());
    }

    #[test]
    fn rejects_e_lfanew_beyond_buffer() {
        let mut h = pe_header(0x6000, &[(0x1000, 0x2000, 0x2000_0000)]);
        h[0x3C..0x40].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
        assert!(parse(&h).is_err());
    }
}
