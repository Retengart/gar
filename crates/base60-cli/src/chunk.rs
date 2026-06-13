//! 8-byte chunk decoding primitives shared by every renderer.

use std::io::{self, BufRead, BufReader, Read};

/// Number of bytes consumed per output line.
///
/// One line ≡ one big-endian [`u64`] ≡ one base-60 number of up to
/// [`base60_core::convert::DIGITS`] digits.
pub(crate) const CHUNK: usize = 8;

/// Right-pad a short byte slice to a full [`CHUNK`]-wide array with zero bytes.
///
/// `bytes.len()` must be in `1..=CHUNK`; longer slices are a programmer
/// error. Callers that slice out of `data.chunks(CHUNK)` are always safe;
/// a zero-length slice at this boundary indicates a bug in the caller,
/// not in the input data.
#[inline]
#[must_use]
pub(crate) fn pad_chunk(bytes: &[u8]) -> [u8; CHUNK] {
    debug_assert!(!bytes.is_empty() && bytes.len() <= CHUNK);
    let mut padded = [0_u8; CHUNK];
    padded[..bytes.len()].copy_from_slice(bytes);
    padded
}

/// Decode an 8-byte big-endian chunk as a [`u64`].
#[inline]
#[must_use]
pub(crate) const fn be_u64(bytes: [u8; CHUNK]) -> u64 {
    u64::from_be_bytes(bytes)
}

/// Right-pad and decode a byte slice as a big-endian [`u64`] in one step.
///
/// Combines [`pad_chunk`] + [`be_u64`] — the most common hot-path pattern
/// across renderers. Returns `(chunk_be, digits)` so callers get both
/// the raw `u64` and the pre-computed base-60 digit array.
#[inline]
#[must_use]
pub(crate) fn prepare(bytes: &[u8]) -> (u64, [u8; base60_core::convert::DIGITS]) {
    let chunk_be = be_u64(pad_chunk(bytes));
    let digits = base60_core::convert::u64_to_base60(chunk_be);
    (chunk_be, digits)
}

/// Return `true` if `b` is a printable ASCII character (graphic or space).
#[inline]
#[must_use]
pub(crate) const fn is_printable(b: u8) -> bool {
    b.is_ascii_graphic() || b == b' '
}

/// Discard `n` bytes from the reader using buffered reads.
pub(crate) fn skip_bytes(reader: &mut BufReader<impl Read>, mut n: u64) -> io::Result<()> {
    while n > 0 {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }
        let consume = usize::try_from(n).unwrap_or(usize::MAX).min(buf.len());
        reader.consume(consume);
        n -= consume as u64;
    }
    Ok(())
}

/// Read up to `CHUNK` bytes, returning how many were filled.
pub(crate) fn read_chunk(
    reader: &mut BufReader<impl Read>,
    buf: &mut [u8; CHUNK],
) -> io::Result<usize> {
    let mut filled = 0;
    while filled < CHUNK {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

/// Clamp `filled` against the `remaining` budget, updating it in place.
pub(crate) fn clamp_filled(filled: usize, remaining: &mut Option<u64>) -> usize {
    remaining.as_mut().map_or(filled, |rem| {
        let actual = filled.min(usize::try_from(*rem).unwrap_or(usize::MAX));
        *rem -= actual as u64;
        actual
    })
}
