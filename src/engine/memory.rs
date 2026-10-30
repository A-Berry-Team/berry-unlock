//! Abstraction over "something whose memory we can read" so the scanner and the
//! FPS locator can be tested against an in-memory fake instead of a live process.

pub trait MemoryReader {
    /// Reads up to `buf.len()` bytes at `address`; returns how many were read
    /// (0 on failure). Partial reads are allowed.
    fn read_memory(&self, address: usize, buf: &mut [u8]) -> usize;

    fn read_exact_at(&self, address: usize, buf: &mut [u8]) -> bool {
        self.read_memory(address, buf) == buf.len()
    }

    fn read_u32_at(&self, address: usize) -> Option<u32> {
        let mut b = [0u8; 4];
        if self.read_exact_at(address, &mut b) {
            Some(u32::from_le_bytes(b))
        } else {
            None
        }
    }
}

#[cfg(test)]
pub mod mock {
    use super::MemoryReader;

    /// Sparse fake address space made of independent regions.
    #[derive(Default)]
    pub struct MockMemory {
        regions: Vec<(usize, Vec<u8>)>,
    }

    impl MockMemory {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn map(mut self, address: usize, bytes: Vec<u8>) -> Self {
            self.regions.push((address, bytes));
            self
        }
    }

    impl MemoryReader for MockMemory {
        fn read_memory(&self, address: usize, buf: &mut [u8]) -> usize {
            for (base, data) in &self.regions {
                if address >= *base && address < base + data.len() {
                    let off = address - base;
                    let n = buf.len().min(data.len() - off);
                    buf[..n].copy_from_slice(&data[off..off + n]);
                    return n;
                }
            }
            0
        }
    }

    /// Builds a minimal PE32+ header page. `sections` = (rva, virtual_size, characteristics).
    pub fn pe_header(size_of_image: u32, sections: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut h = vec![0u8; 0x1000];
        h[0..2].copy_from_slice(b"MZ");
        h[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        let nt = 0x80;
        h[nt..nt + 4].copy_from_slice(b"PE\0\0");
        h[nt + 4..nt + 6].copy_from_slice(&0x8664u16.to_le_bytes());
        h[nt + 6..nt + 8].copy_from_slice(&(sections.len() as u16).to_le_bytes());
        h[nt + 20..nt + 22].copy_from_slice(&0xF0u16.to_le_bytes());
        h[nt + 24..nt + 26].copy_from_slice(&0x20Bu16.to_le_bytes());
        h[nt + 24 + 56..nt + 24 + 60].copy_from_slice(&size_of_image.to_le_bytes());
        let table = nt + 24 + 0xF0;
        for (i, (rva, vsize, ch)) in sections.iter().enumerate() {
            let o = table + i * 40;
            h[o + 8..o + 12].copy_from_slice(&vsize.to_le_bytes());
            h[o + 12..o + 16].copy_from_slice(&rva.to_le_bytes());
            h[o + 16..o + 20].copy_from_slice(&vsize.to_le_bytes());
            h[o + 36..o + 40].copy_from_slice(&ch.to_le_bytes());
        }
        h
    }
}
