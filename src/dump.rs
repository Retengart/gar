//! Hex-dump-style line renderer: `offset  base-60 digits  |ASCII|`.
//!
//! The streaming path ([`dump_all`]) writes directly into a buffered writer
//! without allocating per-line strings; [`format_line`] is kept as a
//! convenience for the interactive viewer where a [`String`] is needed.

use crate::convert::{DIGIT_STR_WIDTH, u64_to_base60, write_digits};
use std::io::{self, BufWriter, Write};

/// Number of bytes consumed per output line.
///
/// One line ≡ one big-endian [`u64`] ≡ one base-60 number of up to
/// [`crate::convert::DIGITS`] digits.
pub(crate) const CHUNK: usize = 8;

/// Width of the zero-padded hex offset column.
const OFFSET_WIDTH: usize = 8;

/// ASCII representation of a non-printable byte.
const DOT: u8 = b'.';

/// Parse `bytes` (length `1..=CHUNK`, right-padded with zeros) as a
/// big-endian [`u64`].
#[inline]
fn be_u64(bytes: &[u8]) -> u64 {
    debug_assert!(!bytes.is_empty() && bytes.len() <= CHUNK);
    let mut padded = [0_u8; CHUNK];
    padded[..bytes.len()].copy_from_slice(bytes);
    u64::from_be_bytes(padded)
}

/// Write a single dump line to `w`, terminated by a newline.
///
/// Layout:
///
/// ```text
/// 00000008  00:00:00:00:00:00:00:00:01:10:20  |...Claude.|
/// ```
#[inline]
pub(crate) fn write_line<W: Write>(w: &mut W, offset: u64, bytes: &[u8]) -> io::Result<()> {
    debug_assert!(bytes.len() <= CHUNK);
    let digits = u64_to_base60(be_u64(bytes));

    // Offset + two-space gap.
    write!(w, "{offset:0OFFSET_WIDTH$x}  ")?;
    write_digits(w, &digits)?;

    // Two-space gap before the ASCII column and the opening delimiter.
    w.write_all(b"  |")?;
    let mut ascii = [DOT; CHUNK];
    for (dst, &src) in ascii.iter_mut().zip(bytes) {
        *dst = if src.is_ascii_graphic() || src == b' ' {
            src
        } else {
            DOT
        };
    }
    w.write_all(&ascii[..bytes.len()])?;
    w.write_all(b"|\n")
}

/// Stream the whole dump to `w`, one line per 8-byte chunk, with
/// `base_offset` added to every displayed offset.
///
/// The caller may supply an unbuffered writer; this function wraps it in a
/// [`BufWriter`] internally so hot-path writes coalesce into syscalls.
pub(crate) fn dump_all<W: Write>(data: &[u8], base_offset: u64, w: W) -> io::Result<()> {
    let mut out = BufWriter::new(w);
    for (idx, chunk) in data.chunks(CHUNK).enumerate() {
        // `idx * CHUNK` never overflows usize in practice (`data.len()`
        // already fits). The `u64` cast is lossless on 64-bit targets and
        // saturating-equivalent on 32-bit ones because `usize` ≤ `u64`.
        let offset = base_offset.saturating_add((idx * CHUNK) as u64);
        write_line(&mut out, offset, chunk)?;
    }
    out.flush()
}

/// Allocating wrapper for non-streaming consumers (e.g. the TUI).
///
/// Returns the rendered line **without** a trailing newline so it can be
/// embedded directly into a [`ratatui::text::Line`].
#[must_use]
pub(crate) fn format_line(offset: u64, bytes: &[u8]) -> String {
    let mut buf = Vec::with_capacity(OFFSET_WIDTH + 2 + DIGIT_STR_WIDTH + 4 + CHUNK);
    write_line(&mut buf, offset, bytes).expect("writing to Vec cannot fail");
    // write_line appends '\n'; strip it for display.
    if buf.last() == Some(&b'\n') {
        buf.pop();
    }
    // SAFETY: write_line only emits ASCII bytes.
    unsafe { String::from_utf8_unchecked(buf) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(offset: u64, bytes: &[u8]) -> String {
        let mut buf = Vec::new();
        write_line(&mut buf, offset, bytes).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn zero_chunk_line() {
        let s = line(0, &[0_u8; 8]);
        assert_eq!(
            s,
            "00000000  00:00:00:00:00:00:00:00:00:00:00  |........|\n"
        );
    }

    #[test]
    fn chunk_5025_encodes_correctly() {
        // Big-endian bytes for `5025_u64`: 0x00_00_00_00_00_00_13_a1.
        let mut bytes = [0_u8; 8];
        bytes[6] = 0x13;
        bytes[7] = 0xa1;
        let s = line(0, &bytes);
        assert!(s.contains("00:00:00:00:00:00:00:00:01:23:45"));
    }

    #[test]
    fn short_chunk_is_right_padded_with_zeros() {
        let s = line(0x10, &[0x01, 0x00, 0x00]);
        assert!(s.starts_with("00000010  "));
        assert!(s.ends_with("|...|\n"));
    }

    #[test]
    fn ascii_column_shows_printable_and_space() {
        let s = line(0, b"Hi there");
        assert!(s.ends_with("|Hi there|\n"));
    }

    #[test]
    fn ascii_column_dots_control_and_high_bytes() {
        let s = line(0, &[0x00, 0x1f, b'A', 0x7f, 0xff, b'z', b' ', b'~']);
        assert!(s.ends_with("|..A..z ~|\n"));
    }

    #[test]
    fn dump_all_emits_one_line_per_chunk() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        dump_all(&data, 0x100, &mut buf).unwrap();
        let rendered = String::from_utf8(buf).unwrap();
        assert_eq!(rendered.lines().count(), 3);
        assert!(rendered.starts_with("00000100  "));
        assert!(rendered.lines().nth(1).unwrap().starts_with("00000108  "));
        assert!(rendered.lines().nth(2).unwrap().starts_with("00000110  "));
    }

    #[test]
    fn format_line_matches_write_line_without_newline() {
        let bytes = b"abcdefgh";
        let written = line(0x42, bytes);
        let formatted = format_line(0x42, bytes);
        assert_eq!(written.trim_end_matches('\n'), formatted);
    }
}
