//! Hex-dump-style line renderer: `offset  base-60 digits  |ASCII|`.
//!
//! Two rendering paths share the same heat-map palette:
//!
//! * [`dump_all`] — streaming, allocation-free, ANSI-coloured when the
//!   caller supplies a coloured [`Palette`].
//! * [`styled_line`] — returns a ratatui [`Line`] with per-token [`Span`]s
//!   for the interactive viewer.

use crate::chunk::{CHUNK, be_u64, pad_chunk};
use crate::color::{
    self, Palette, delim_style, digit_style, dot_style, lens_style, offset_style, printable_style,
    sep_style,
};
use base60_core::convert::{DIGITS, u64_to_base60};
use base60_core::lens::Lens;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

/// Width of the zero-padded hex offset column.
const OFFSET_WIDTH: usize = 8;

/// ASCII representation of a non-printable byte.
const DOT: u8 = b'.';

/// Write a single dump line to `w`, terminated by a newline.
///
/// Layout:
///
/// ```text
/// 00000008  00:00:00:00:00:00:00:00:01:10:20  |...Claude.|
/// ```
///
/// Pass [`crate::color::PALETTE_NONE`] for monochrome output or
/// [`crate::color::PALETTE_ANSI`] for ANSI-coloured output. With the
/// `NONE` palette the palette writes are empty-slice no-ops, so
/// coloured and monochrome paths share one code path without a runtime
/// branch per token.
#[inline]
pub(crate) fn write_line<W: Write>(
    w: &mut W,
    offset: u64,
    bytes: &[u8],
    palette: &Palette,
    lens: Option<&dyn Lens>,
) -> io::Result<()> {
    debug_assert!(bytes.len() <= CHUNK);
    let chunk_be = be_u64(pad_chunk(bytes));
    let digits = u64_to_base60(chunk_be);

    // Offset column.
    w.write_all(palette.offset.as_bytes())?;
    write!(w, "{offset:0OFFSET_WIDTH$x}")?;
    w.write_all(palette.reset.as_bytes())?;
    w.write_all(b"  ")?;

    // Base-60 digit pairs, coloured per heat-map tier. When the palette is
    // NONE all `write_all`s emit zero bytes, so there is no per-token cost.
    for (i, &d) in digits.iter().enumerate() {
        if i > 0 {
            w.write_all(palette.sep.as_bytes())?;
            w.write_all(b":")?;
            w.write_all(palette.reset.as_bytes())?;
        }
        w.write_all(palette.digit(d).as_bytes())?;
        let hi = b'0' + d / 10;
        let lo = b'0' + d % 10;
        w.write_all(&[hi, lo])?;
        w.write_all(palette.reset.as_bytes())?;
    }

    // ASCII column.
    w.write_all(b"  ")?;
    w.write_all(palette.delim.as_bytes())?;
    w.write_all(b"|")?;
    w.write_all(palette.reset.as_bytes())?;

    for &src in bytes {
        let printable = src.is_ascii_graphic() || src == b' ';
        w.write_all(
            if printable {
                palette.printable
            } else {
                palette.dot
            }
            .as_bytes(),
        )?;
        w.write_all(&[if printable { src } else { DOT }])?;
        w.write_all(palette.reset.as_bytes())?;
    }

    w.write_all(palette.delim.as_bytes())?;
    w.write_all(b"|")?;
    w.write_all(palette.reset.as_bytes())?;

    // Optional semantic overlay. Rendered once per line and forwarded as a
    // single `write_all`; the lens allocates its own string, so the cost
    // only applies when a lens is active.
    if let Some(lens) = lens {
        w.write_all(b"  ")?;
        w.write_all(palette.lens.as_bytes())?;
        lens.render_to(chunk_be, w)?;
        w.write_all(palette.reset.as_bytes())?;
    }

    w.write_all(b"\n")
}

/// Stream the whole dump to `w`, one line per 8-byte chunk, with
/// `base_offset` added to every displayed offset.
///
/// The caller may supply an unbuffered writer; this function wraps it in a
/// [`BufWriter`] internally so hot-path writes coalesce into syscalls.
///
/// Widened to `pub` so the `#[doc(hidden)] pub mod __bench` re-export in
/// `crate::lib` can surface it to `crates/base60-cli/benches/`. The
/// enclosing `mod dump` is private at crate root, so this function is
/// still unreachable from the public API (Phase 5 PERF-06, TEST-02 SC5).
///
/// # Errors
///
/// Propagates any [`io::Error`] returned by the underlying writer.
#[allow(unreachable_pub)]
pub fn dump_all<W: Write>(
    data: &[u8],
    base_offset: u64,
    w: W,
    palette: &Palette,
    lens: Option<&dyn Lens>,
) -> io::Result<()> {
    let mut out = BufWriter::new(w);
    for (idx, chunk) in data.chunks(CHUNK).enumerate() {
        // `idx * CHUNK` never overflows usize in practice (`data.len()`
        // already fits). The `u64` cast is lossless on 64-bit targets and
        // saturating-equivalent on 32-bit ones because `usize` ≤ `u64`.
        let offset = base_offset.saturating_add((idx * CHUNK) as u64);
        write_line(&mut out, offset, chunk, palette, lens)?;
    }
    // REF-04 (D-01, D-04): length trailer. `#` prefix keeps
    // `decode::find_digit_run` from matching this line — `#` is neither
    // an ASCII digit nor a colon, so the scanner cannot construct a run
    // that overlaps it. Always emitted, including for empty input (D-02).
    writeln!(out, "# bytes=0x{:x}", data.len())?;
    out.flush()
}

/// Stream dump output from a reader, processing 8-byte chunks as they arrive.
///
/// Unlike [`dump_all`], this function never materialises the full input in
/// memory. It reads in [`CHUNK`]-sized increments and writes each dump
/// line immediately, making it suitable for piped stdin where the input
/// size is unbounded.
///
/// `skip` is both the number of bytes to discard from the start and the
/// displayed offset of the first line. `length` optionally caps the
/// number of bytes to process after skipping.
///
/// # Errors
///
/// Propagates any [`io::Error`] returned by the underlying reader or writer.
pub(crate) fn dump_reader<R: Read, W: Write>(
    reader: R,
    skip: u64,
    length: Option<u64>,
    w: W,
    palette: &Palette,
    lens: Option<&dyn Lens>,
) -> io::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut out = BufWriter::new(w);

    // Discard `skip` bytes from the start.
    let mut to_skip = skip;
    while to_skip > 0 {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }
        let consume = usize::try_from(to_skip)
            .unwrap_or(usize::MAX)
            .min(buf.len());
        reader.consume(consume);
        to_skip -= consume as u64;
    }

    let mut offset = skip;
    let mut total: u64 = 0;
    let mut remaining = length;
    let mut chunk_buf = [0u8; CHUNK];

    loop {
        if remaining.is_some_and(|r| r == 0) {
            break;
        }

        let mut filled = 0;
        while filled < CHUNK {
            match reader.read(&mut chunk_buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }

        if filled == 0 {
            break;
        }

        if let Some(rem) = remaining.as_mut() {
            let actual = filled.min(usize::try_from(*rem).unwrap_or(usize::MAX));
            filled = actual;
            *rem -= actual as u64;
        }

        total += filled as u64;
        write_line(&mut out, offset, &chunk_buf[..filled], palette, lens)?;
        offset = offset.saturating_add(CHUNK as u64);
    }

    writeln!(out, "# bytes=0x{total:x}")?;
    out.flush()
}

/// Build a ratatui [`Line`] of styled [`Span`]s for the interactive viewer.
///
/// Unlike [`write_line`], this path targets a `Vec<Span>` so each token
/// carries its own [`Style`], letting the terminal do the rendering with
/// true-color fidelity where supported. When `cursor_in_line` is `Some`,
/// the corresponding byte in the ASCII column is rendered with reversed
/// video so the cursor is obvious regardless of colour scheme.
pub(crate) fn styled_line(
    offset: u64,
    bytes: &[u8],
    lens: Option<&dyn Lens>,
    cursor_in_line: Option<usize>,
) -> Line<'static> {
    debug_assert!(bytes.len() <= CHUNK);
    let chunk_be = be_u64(pad_chunk(bytes));
    let digits = u64_to_base60(chunk_be);

    // 1 offset + 1 gap + (DIGITS digit spans + DIGITS-1 separator spans)
    // + 1 gap + 1 opening delim + CHUNK ascii spans + 1 closing delim
    // + up to 2 optional lens spans (gap + rendered).
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(5 + DIGITS * 2 + CHUNK + 2);

    spans.push(Span::styled(
        format!("{offset:0OFFSET_WIDTH$x}"),
        offset_style(),
    ));
    spans.push(Span::raw("  "));

    let sep = sep_style();
    for (i, &d) in digits.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(":", sep));
        }
        spans.push(Span::styled(format!("{d:02}"), digit_style(d)));
    }

    spans.push(Span::raw("  "));
    let delim = delim_style();
    spans.push(Span::styled("|", delim));
    let print = printable_style();
    let dot = dot_style();
    for (i, &src) in bytes.iter().enumerate() {
        let base = if src.is_ascii_graphic() || src == b' ' {
            print
        } else {
            dot
        };
        // Reverse-video the exact cursor byte so the viewer can see where
        // hjkl motion lands without needing a second colour scheme.
        let style = if cursor_in_line == Some(i) {
            base.add_modifier(Modifier::REVERSED)
        } else {
            base
        };
        let ch = if src.is_ascii_graphic() || src == b' ' {
            // `src` is ASCII, so this `char` cast is exact.
            src as char
        } else {
            '.'
        };
        spans.push(Span::styled(String::from(ch), style));
    }
    spans.push(Span::styled("|", delim));

    if let Some(lens) = lens {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(lens.render(chunk_be), lens_style()));
    }

    Line::from(spans)
}

/// Convenience style helpers re-exported so the TUI can pick them up
/// without reaching into [`crate::color`] directly.
#[inline]
pub(crate) const fn border_style() -> Style {
    color::border_style()
}

#[inline]
pub(crate) const fn title_style() -> Style {
    color::title_style()
}

#[inline]
pub(crate) const fn status_style() -> Style {
    color::status_style()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{PALETTE_ANSI, PALETTE_NONE};
    use base60_core::lens::{AngleLens, TimeLens};

    fn line_mono(offset: u64, bytes: &[u8]) -> String {
        let mut buf = Vec::new();
        write_line(&mut buf, offset, bytes, &PALETTE_NONE, None).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn line_ansi(offset: u64, bytes: &[u8]) -> String {
        let mut buf = Vec::new();
        write_line(&mut buf, offset, bytes, &PALETTE_ANSI, None).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn line_mono_with_lens(offset: u64, bytes: &[u8], lens: &dyn Lens) -> String {
        let mut buf = Vec::new();
        write_line(&mut buf, offset, bytes, &PALETTE_NONE, Some(lens)).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn mono_matches_legacy_layout() {
        let s = line_mono(0, &[0_u8; 8]);
        assert_eq!(
            s,
            "00000000  00:00:00:00:00:00:00:00:00:00:00  |........|\n"
        );
    }

    #[test]
    fn mono_chunk_5025_encodes_correctly() {
        let mut bytes = [0_u8; 8];
        bytes[6] = 0x13;
        bytes[7] = 0xa1;
        let s = line_mono(0, &bytes);
        assert!(s.contains("00:00:00:00:00:00:00:00:01:23:45"));
    }

    #[test]
    fn mono_short_chunk_is_right_padded_with_zeros() {
        let s = line_mono(0x10, &[0x01, 0x00, 0x00]);
        assert!(s.starts_with("00000010  "));
        assert!(s.ends_with("|...|\n"));
    }

    #[test]
    fn mono_ascii_printable_and_space() {
        let s = line_mono(0, b"Hi there");
        assert!(s.ends_with("|Hi there|\n"));
    }

    #[test]
    fn mono_ascii_control_and_high_bytes_become_dots() {
        let s = line_mono(0, &[0x00, 0x1f, b'A', 0x7f, 0xff, b'z', b' ', b'~']);
        assert!(s.ends_with("|..A..z ~|\n"));
    }

    #[test]
    fn dump_all_emits_one_line_per_chunk() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        dump_all(&data, 0x100, &mut buf, &PALETTE_NONE, None).unwrap();
        let rendered = String::from_utf8(buf).unwrap();
        // Three dump lines + one `# bytes=` trailer line.
        assert_eq!(rendered.lines().count(), 4);
        assert!(rendered.starts_with("00000100  "));
        assert!(rendered.lines().nth(1).unwrap().starts_with("00000108  "));
        assert!(rendered.lines().nth(2).unwrap().starts_with("00000110  "));
    }

    #[test]
    fn dump_all_emits_length_trailer_hex_on_plain() {
        // 14 bytes = 0xe; short tail under 8-alignment exercises the
        // non-multiple-of-8 path (D-02 always-emit invariant).
        let data: Vec<u8> = (0..14).collect();
        let mut buf = Vec::new();
        dump_all(&data, 0, &mut buf, &PALETTE_NONE, None).unwrap();
        let rendered = String::from_utf8(buf).unwrap();
        assert!(
            rendered.ends_with("# bytes=0xe\n"),
            "trailer missing or wrong; tail: {:?}",
            &rendered[rendered.len().saturating_sub(40)..]
        );
    }

    #[test]
    fn dump_all_emits_length_trailer_hex_on_ansi() {
        let data: Vec<u8> = (0..14).collect();
        let mut buf = Vec::new();
        dump_all(&data, 0, &mut buf, &PALETTE_ANSI, None).unwrap();
        let rendered = String::from_utf8(buf).unwrap();
        assert!(rendered.contains("# bytes=0xe\n"));
    }

    #[test]
    fn dump_all_emits_trailer_for_empty_input() {
        let mut buf = Vec::new();
        dump_all(b"", 0, &mut buf, &PALETTE_NONE, None).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "# bytes=0x0\n");
    }

    #[test]
    fn mono_with_time_lens_appends_overlay() {
        let s = line_mono_with_lens(0, &[0_u8; 8], &TimeLens::default());
        assert!(s.ends_with("|........|  0d 00𒁹 00:00\n"));
    }

    #[test]
    fn mono_with_angle_lens_appends_overlay() {
        let mut bytes = [0_u8; 8];
        // Encode 3_600_000 (= 1°) as big-endian u64.
        bytes[..].copy_from_slice(&3_600_000_u64.to_be_bytes());
        let s = line_mono_with_lens(0, &bytes, &AngleLens);
        assert!(s.ends_with("001°00′00.000″\n"));
    }

    #[test]
    fn ansi_contains_reset_sequences() {
        let s = line_ansi(0, &[0x00, 0x01, 0x14, 0x28, 0x3b, 0x7f, b'z', b' ']);
        // At minimum, we expect SGR reset and the heat-map tier colours.
        assert!(s.contains("\x1b[0m"));
        assert!(s.contains("\x1b[32m")); // green (low)
        assert!(s.contains("\x1b[33m")); // yellow (mid)
        assert!(s.contains("\x1b[31m")); // red (high)
        assert!(s.contains("\x1b[36m")); // cyan (printable)
    }

    #[test]
    fn ansi_payload_characters_still_match_mono() {
        // Strip every ANSI CSI sequence; what remains must be byte-identical
        // to the monochrome rendering.
        let bytes = b"Hi there";
        let ansi = line_ansi(0x42, bytes);
        let mono = line_mono(0x42, bytes);
        assert_eq!(strip_ansi(&ansi), mono);
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'[') {
                i += 2;
                while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                    i += 1;
                }
                // fall through to consume the CSI final byte (or the null
                // terminator if the sequence is unterminated at EOF).
            } else {
                out.push(bytes[i] as char);
            }
            i += 1;
        }
        out
    }

    #[test]
    fn styled_line_has_expected_span_count() {
        let bytes = b"abcdefgh";
        let line = styled_line(0, bytes, None, None);
        // 1 offset + 1 gap + 11 digits + 10 separators + 1 gap
        // + 1 open delim + 8 ascii + 1 close delim = 34.
        assert_eq!(
            line.spans.len(),
            1 + 1 + DIGITS + (DIGITS - 1) + 1 + 1 + CHUNK + 1
        );
    }

    #[test]
    fn styled_line_with_lens_adds_two_spans() {
        let bytes = b"abcdefgh";
        let plain = styled_line(0, bytes, None, None);
        let lensed = styled_line(0, bytes, Some(&TimeLens::default()), None);
        // Exactly two extra spans: a two-space gap and the lens content.
        assert_eq!(lensed.spans.len(), plain.spans.len() + 2);
    }

    #[test]
    fn styled_line_text_matches_mono_line() {
        let bytes = b"Hi there";
        let line = styled_line(0x42, bytes, None, None);
        let joined: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        let mono = line_mono(0x42, bytes);
        assert_eq!(joined, mono.trim_end_matches('\n'));
    }

    #[test]
    fn styled_line_cursor_flags_exact_byte_with_reversed_video() {
        let bytes = b"abcdefgh";
        let line = styled_line(0, bytes, None, Some(3));
        // The ascii spans are the last 8 spans before the closing delim;
        // find them by filtering for single-char ascii-letter content.
        let ascii_spans: Vec<&Span<'_>> = line
            .spans
            .iter()
            .filter(|s| {
                s.content.len() == 1
                    && s.content
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic())
            })
            .collect();
        assert_eq!(ascii_spans.len(), 8);
        // Byte 3 ('d') must have REVERSED; its neighbours must not.
        assert!(
            ascii_spans[3]
                .style
                .add_modifier
                .contains(Modifier::REVERSED)
        );
        assert!(
            !ascii_spans[2]
                .style
                .add_modifier
                .contains(Modifier::REVERSED)
        );
        assert!(
            !ascii_spans[4]
                .style
                .add_modifier
                .contains(Modifier::REVERSED)
        );
    }

    #[test]
    fn dump_reader_matches_dump_all_for_full_input() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        dump_reader(
            data.as_slice(),
            0,
            None,
            &mut buf,
            &PALETTE_NONE,
            None,
        )
        .unwrap();
        let streamed = String::from_utf8(buf).unwrap();

        let mut buf2 = Vec::new();
        dump_all(&data, 0, &mut buf2, &PALETTE_NONE, None).unwrap();
        let buffered = String::from_utf8(buf2).unwrap();

        assert_eq!(streamed, buffered);
    }

    #[test]
    fn dump_reader_emits_trailer_for_empty_input() {
        let mut buf = Vec::new();
        dump_reader(b"".as_slice(), 0, None, &mut buf, &PALETTE_NONE, None).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "# bytes=0x0\n");
    }

    #[test]
    fn dump_reader_respects_skip() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        dump_reader(
            data.as_slice(),
            8,
            None,
            &mut buf,
            &PALETTE_NONE,
            None,
        )
        .unwrap();
        let rendered = String::from_utf8(buf).unwrap();
        // First line offset should be 8 (the skip amount).
        assert!(rendered.starts_with("00000008  "));
        // Trailer should report 16 bytes (24 - 8 skipped).
        assert!(rendered.contains("# bytes=0x10\n"));
    }

    #[test]
    fn dump_reader_respects_length() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        dump_reader(
            data.as_slice(),
            0,
            Some(16),
            &mut buf,
            &PALETTE_NONE,
            None,
        )
        .unwrap();
        let rendered = String::from_utf8(buf).unwrap();
        // Two 8-byte lines + trailer.
        assert_eq!(rendered.lines().count(), 3);
        assert!(rendered.contains("# bytes=0x10\n"));
    }
}
