//! 8-byte chunk decoding primitives shared by every renderer.

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
