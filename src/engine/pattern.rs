//! Masked byte-pattern search.
//!
//! Compared to the original scanner:
//! * correct on partial-match restarts (the original dropped the byte that broke
//!   a partial match, so overlapping candidates could be missed);
//! * SIMD-accelerated anchor search via `memchr`;
//! * 1 MiB chunks with exact `pattern_len - 1` overlap, so matches that straddle
//!   chunk borders are found;
//! * lazy: stops at the first hit the caller accepts.

use memchr::memchr;

use super::memory::MemoryReader;

pub const SCAN_CHUNK: usize = 1 << 20;

#[derive(Debug, Clone)]
pub struct Pattern {
    bytes: Vec<u8>,
    fixed: Vec<bool>,
    anchor: usize,
}

impl Pattern {
    /// `None` entries are wildcards. Returns `None` if the pattern has no fixed byte.
    pub fn new(spec: &[Option<u8>]) -> Option<Self> {
        let anchor = spec.iter().position(|b| b.is_some())?;
        Some(Pattern {
            bytes: spec.iter().map(|b| b.unwrap_or(0)).collect(),
            fixed: spec.iter().map(|b| b.is_some()).collect(),
            anchor,
        })
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn matches_at(&self, hay: &[u8], start: usize) -> bool {
        match hay.get(start..start + self.bytes.len()) {
            Some(window) => window
                .iter()
                .zip(&self.bytes)
                .zip(&self.fixed)
                .all(|((h, p), fixed)| !*fixed || h == p),
            None => false,
        }
    }

    /// First match at index >= `from`.
    pub fn find_from(&self, hay: &[u8], from: usize) -> Option<usize> {
        let n = self.bytes.len();
        if hay.len() < n || from > hay.len() - n {
            return None;
        }
        let last_start = hay.len() - n;
        let anchor_byte = self.bytes[self.anchor];
        let search_end = last_start + self.anchor + 1;
        let mut cursor = from + self.anchor;
        while cursor < search_end {
            let rel = memchr(anchor_byte, &hay[cursor..search_end])?;
            let start = cursor + rel - self.anchor;
            if self.matches_at(hay, start) {
                return Some(start);
            }
            cursor += rel + 1;
        }
        None
    }

    pub fn find(&self, hay: &[u8]) -> Option<usize> {
        self.find_from(hay, 0)
    }
}

/// Scans `len` bytes at `base` inside `reader`. `visit` receives every match
/// address in ascending order (no duplicates) and returns `Some(v)` to stop.
pub fn scan_region<R: MemoryReader, T>(
    reader: &R,
    base: usize,
    len: usize,
    pattern: &Pattern,
    mut visit: impl FnMut(usize) -> Option<T>,
) -> Option<T> {
    let n = pattern.len();
    if n == 0 || len < n {
        return None;
    }
    let overlap = n - 1;
    let mut buf = vec![0u8; SCAN_CHUNK + overlap];
    let mut offset = 0usize;
    let mut last_reported: Option<usize> = None;

    while offset < len {
        let want = (len - offset).min(SCAN_CHUNK + overlap);
        let addr = base.checked_add(offset)?;
        let got = reader.read_memory(addr, &mut buf[..want]);
        if got >= n {
            let mut from = 0usize;
            while let Some(pos) = pattern.find_from(&buf[..got], from) {
                let hit = addr.checked_add(pos)?;
                if last_reported.map_or(true, |l| hit > l) {
                    last_reported = Some(hit);
                    if let Some(v) = visit(hit) {
                        return Some(v);
                    }
                }
                from = pos + 1;
            }
        }
        offset += SCAN_CHUNK;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::memory::mock::MockMemory;

    fn pat(spec: &[Option<u8>]) -> Pattern {
        Pattern::new(spec).unwrap()
    }

    #[test]
    fn finds_exact_and_wildcard() {
        let p = pat(&[Some(1), None, Some(3)]);
        assert_eq!(p.find(&[9, 1, 7, 3, 0]), Some(1));
        assert_eq!(p.find(&[1, 2, 4]), None);
        assert_eq!(p.find(&[1]), None);
    }

    #[test]
    fn handles_overlapping_partial_matches() {
        // The original matcher missed this: "B9 B9 3C" must match at index 1.
        let p = pat(&[Some(0xB9), Some(0x3C)]);
        assert_eq!(p.find(&[0xB9, 0xB9, 0x3C]), Some(1));
    }

    #[test]
    fn find_from_skips_earlier_matches() {
        let p = pat(&[Some(5), Some(6)]);
        let hay = [5, 6, 0, 5, 6];
        assert_eq!(p.find_from(&hay, 0), Some(0));
        assert_eq!(p.find_from(&hay, 1), Some(3));
        assert_eq!(p.find_from(&hay, 4), None);
    }

    #[test]
    fn rejects_all_wildcard_pattern() {
        assert!(Pattern::new(&[None, None]).is_none());
    }

    #[test]
    fn scan_crosses_chunk_boundary() {
        let mut data = vec![0u8; SCAN_CHUNK * 2 + 100];
        let at = SCAN_CHUNK - 2; // straddles the first chunk border
        data[at..at + 5].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x01]);
        let mem = MockMemory::new().map(0x10_0000, data.clone());
        let p = pat(&[Some(0xDE), Some(0xAD), None, Some(0xEF), Some(0x01)]);
        let hit = scan_region(&mem, 0x10_0000, data.len(), &p, Some);
        assert_eq!(hit, Some(0x10_0000 + at));
    }

    #[test]
    fn scan_reports_each_hit_once_in_order() {
        let mut data = vec![0u8; SCAN_CHUNK + 64];
        for at in [10usize, SCAN_CHUNK - 1, SCAN_CHUNK + 3] {
            data[at..at + 2].copy_from_slice(&[0xAA, 0xBB]);
        }
        let mem = MockMemory::new().map(0x1000, data.clone());
        let p = pat(&[Some(0xAA), Some(0xBB)]);
        let mut seen = Vec::new();
        let r: Option<()> = scan_region(&mem, 0x1000, data.len(), &p, |a| {
            seen.push(a);
            None
        });
        assert!(r.is_none());
        assert_eq!(seen, vec![0x1000 + 10, 0x1000 + SCAN_CHUNK - 1, 0x1000 + SCAN_CHUNK + 3]);
    }

    #[test]
    fn scan_on_unreadable_memory_finds_nothing() {
        let mem = MockMemory::new();
        let p = pat(&[Some(1), Some(2)]);
        assert_eq!(scan_region(&mem, 0x5000, 0x4000, &p, Some), None);
    }
}
