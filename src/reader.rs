use anyhow::{Context, Result};
use memmap2::Mmap;
use std::fs::File;
use std::io::{Read, stdin};
use std::path::Path;

/// Load the contents of a file (via mmap) or stdin (via read-to-end)
/// and apply `skip` / `length` trimming. Returns the resulting byte buffer.
///
/// For files we prefer `mmap`: lazy page-faulting lets us open huge files
/// without copying them into RAM upfront.
pub enum Bytes {
    Mapped(Mmap, usize, usize), // (map, start, end)
    Owned(Vec<u8>),
}

impl Bytes {
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Bytes::Mapped(m, start, end) => &m[*start..*end],
            Bytes::Owned(v) => v,
        }
    }
}

pub fn load(path: Option<&Path>, skip: u64, length: Option<u64>) -> Result<Bytes> {
    match path {
        Some(p) => {
            let file = File::open(p).with_context(|| format!("open {}", p.display()))?;
            // SAFETY: mmap is only unsafe because the backing file could be
            // modified by another process; for a read-only viewer this is
            // acceptable — the worst case is stale bytes.
            let map = unsafe { Mmap::map(&file) }
                .with_context(|| format!("mmap {}", p.display()))?;
            let total = map.len();
            let start = usize::try_from(skip).unwrap_or(usize::MAX).min(total);
            let end = match length {
                Some(n) => start
                    .saturating_add(usize::try_from(n).unwrap_or(usize::MAX))
                    .min(total),
                None => total,
            };
            Ok(Bytes::Mapped(map, start, end))
        }
        None => {
            let mut buf = Vec::new();
            stdin().read_to_end(&mut buf).context("read stdin")?;
            let total = buf.len();
            let start = usize::try_from(skip).unwrap_or(usize::MAX).min(total);
            let end = match length {
                Some(n) => start
                    .saturating_add(usize::try_from(n).unwrap_or(usize::MAX))
                    .min(total),
                None => total,
            };
            buf.truncate(end);
            buf.drain(..start);
            Ok(Bytes::Owned(buf))
        }
    }
}
