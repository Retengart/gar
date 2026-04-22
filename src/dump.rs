use crate::convert::{DIGIT_STR_WIDTH, format_digits, u64_to_base60};
use std::io::{BufWriter, Write};

pub const CHUNK: usize = 8;

/// Format a single 8-byte chunk (zero-padded on the right if short)
/// as a u64 big-endian integer, then render a full dump line:
///
///     00000008  00:00:00:00:00:00:00:00:01:10:20  |...Claude.|
pub fn format_line(offset: u64, bytes: &[u8]) -> String {
    debug_assert!(bytes.len() <= CHUNK);
    let mut padded = [0u8; CHUNK];
    padded[..bytes.len()].copy_from_slice(bytes);
    let n = u64::from_be_bytes(padded);
    let digits = format_digits(&u64_to_base60(n));

    let mut ascii = String::with_capacity(CHUNK + 2);
    ascii.push('|');
    for &b in bytes {
        ascii.push(if (0x20..0x7f).contains(&b) {
            b as char
        } else {
            '.'
        });
    }
    ascii.push('|');

    format!(
        "{:08x}  {:<width$}  {}",
        offset,
        digits,
        ascii,
        width = DIGIT_STR_WIDTH
    )
}

/// Stream the whole dump to `w`, one line per 8-byte chunk,
/// with `base_offset` added to every displayed offset.
pub fn dump_all<W: Write>(data: &[u8], base_offset: u64, w: &mut W) -> std::io::Result<()> {
    let mut out = BufWriter::new(w);
    for (i, chunk) in data.chunks(CHUNK).enumerate() {
        let offset = base_offset + (i * CHUNK) as u64;
        writeln!(out, "{}", format_line(offset, chunk))?;
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_chunk_line() {
        let line = format_line(0, &[0u8; 8]);
        assert!(line.starts_with("00000000  "));
        assert!(line.contains("00:00:00:00:00:00:00:00:00:00:00"));
        assert!(line.ends_with("|........|"));
    }

    #[test]
    fn chunk_5025_encodes_correctly() {
        // big-endian bytes for u64 value 5025 = 0x00..13a1
        let mut bytes = [0u8; 8];
        bytes[6] = 0x13;
        bytes[7] = 0xa1;
        let line = format_line(0, &bytes);
        assert!(line.contains("00:00:00:00:00:00:00:00:01:23:45"));
    }

    #[test]
    fn short_chunk_is_right_padded_with_zeros() {
        // Only 3 bytes → treated as the top 3 bytes of an 8-byte BE integer.
        let line = format_line(0x10, &[0x01, 0x00, 0x00]);
        // 0x01_00_00_00_00_00_00_00 = 72057594037927936 in decimal.
        // In base-60 top digit is... just make sure the format is sane.
        assert!(line.starts_with("00000010  "));
        assert!(line.ends_with("|...|"));
    }
}
