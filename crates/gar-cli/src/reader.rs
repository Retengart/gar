//! Input handling: memory-map a file or slurp stdin, then apply
//! `--skip`/`--length` bounds with saturating arithmetic.

use anyhow::{Context, Result};
use memmap2::Mmap;
use std::fs::File;
use std::io::{Read, stdin};
use std::path::Path;

/// Owned byte buffer returned by [`load`].
///
/// Keeps the mapping alive for the lifetime of any slice borrowed via
/// [`Bytes::as_slice`].
pub(crate) enum Bytes {
    /// Memory-mapped range `start..end` over the backing file.
    Mapped { map: Mmap, start: usize, end: usize },
    /// Owned buffer (from stdin or a small file).
    Owned(Vec<u8>),
}

impl std::fmt::Debug for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mapped { start, end, .. } => f
                .debug_struct("Mapped")
                .field("start", start)
                .field("end", end)
                .finish_non_exhaustive(),
            Self::Owned(v) => f.debug_tuple("Owned").field(&v.len()).finish(),
        }
    }
}

impl Bytes {
    #[must_use]
    pub(crate) fn as_slice(&self) -> &[u8] {
        match self {
            Self::Mapped { map, start, end } => &map[*start..*end],
            Self::Owned(v) => v,
        }
    }
}

/// Load `path` (via mmap) or stdin (via `read_to_end`) and apply the
/// half-open range `[skip, skip + length)`. Out-of-range values saturate to
/// the input size rather than panicking.
pub(crate) fn load(path: Option<&Path>, skip: u64, length: Option<u64>) -> Result<Bytes> {
    path.map_or_else(|| load_stdin(skip, length), |p| load_file(p, skip, length))
}

fn load_file(path: &Path, skip: u64, length: Option<u64>) -> Result<Bytes> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    // SAFETY: mmap is `unsafe` because another process could mutate the
    // backing file underneath us. For a read-only viewer the worst outcome
    // is stale bytes on screen, which is acceptable.
    let map = unsafe { Mmap::map(&file) }.with_context(|| format!("mmap {}", path.display()))?;
    let (start, end) = clamp_range(map.len(), skip, length);
    Ok(Bytes::Mapped { map, start, end })
}

fn load_stdin(skip: u64, length: Option<u64>) -> Result<Bytes> {
    let mut buf = Vec::new();
    stdin().read_to_end(&mut buf).context("read stdin")?;
    let (start, end) = clamp_range(buf.len(), skip, length);
    buf.truncate(end);
    buf.drain(..start);
    Ok(Bytes::Owned(buf))
}

/// Clamp `skip` and `length` against `total`, saturating any arithmetic that
/// would otherwise overflow [`usize`].
fn clamp_range(total: usize, skip: u64, length: Option<u64>) -> (usize, usize) {
    let start = usize::try_from(skip).unwrap_or(usize::MAX).min(total);
    let end = length.map_or(total, |n| {
        let n = usize::try_from(n).unwrap_or(usize::MAX);
        start.saturating_add(n).min(total)
    });
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_no_limits() {
        assert_eq!(clamp_range(100, 0, None), (0, 100));
    }

    #[test]
    fn clamp_skip_past_total() {
        assert_eq!(clamp_range(100, 1_000, None), (100, 100));
    }

    #[test]
    fn clamp_length_truncates_to_total() {
        assert_eq!(clamp_range(100, 80, Some(50)), (80, 100));
    }

    #[test]
    fn clamp_saturates_huge_values() {
        assert_eq!(clamp_range(100, u64::MAX, Some(u64::MAX)), (100, 100));
    }

    #[test]
    fn clamp_normal_window() {
        assert_eq!(clamp_range(100, 10, Some(30)), (10, 40));
    }
}
