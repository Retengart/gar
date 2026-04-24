//! Inverse of `dump::dump_all`: parse base-60 digit pairs back into bytes.
//!
//! Accepts any text containing one or more runs of exactly 11 two-digit
//! base-60 pairs joined by colons — the shape that `dump` emits. The
//! surrounding line content (offset column, ASCII column, ANSI escapes)
//! is ignored so a user can pipe a coloured dump straight back in:
//!
//! ```text
//! base60 --color=never FILE | base60 decode > FILE.roundtrip
//! ```
//!
//! Each pair represents a base-60 digit `0..60`; the 11 digits are
//! recomposed most-significant-first into a `u64` and emitted as 8
//! big-endian bytes. A pair with a digit `>= 60` is a malformed input
//! and raises a descriptive error carrying the line number.
//!
//! # Input formats
//!
//! [`decode_stream`] accepts four emitter shapes (ansi/plain/json/html)
//! and auto-detects by sniffing the first non-empty line of input. An
//! explicit override is available via the `--input-format` flag on the
//! `decode` subcommand ([`crate::cli::InputFormat`], D-06).
//!
//! # HTML decode coupling
//!
//! [`decode_from_html`] is tightly coupled to [`crate::format::emit_html`]'s
//! exact tag sequence: it recognises the four `<span class="d-zero|d-low|
//! d-mid|d-high">NN</span>` digit shapes, treats `<span class="sep">:</span>`
//! as a separator, strips content outside `<body>...</body>`, and consumes
//! the `<!-- bytes=0x<hex> -->` length trailer. Any emitter change in
//! `format::emit_html` MUST be mirrored here — this file is the inverse
//! spec (D-05).
//!
//! # Length metadata
//!
//! Dumps emitted by `base60` v2+ carry a trailing `# bytes=0x<hex>`
//! (ansi/plain), `<!-- bytes=0x<hex> -->` (html), or
//! `{"type":"meta","bytes":<dec>}` (json) record. The decoder uses that
//! value to truncate the final chunk when the original input was not a
//! multiple of 8 bytes. Legacy dumps without the trailer still decode,
//! with a single stderr warning emitted at EOF (D-03).

use crate::cli::InputFormat;
use base60_core::convert::DIGITS;
use std::io::{self, BufRead, Read, Write};

/// Digit-pair width: two ASCII decimal chars per base-60 digit.
const PAIR: usize = 2;
/// Total characters for 11 digit pairs joined by 10 colons.
const RUN_LEN: usize = PAIR * DIGITS + (DIGITS - 1);
/// Bytes produced per decoded run (one `u64` → 8 big-endian bytes).
const CHUNK_BYTES: usize = 8;

/// Sniffed format from the first non-empty line. Internal-only.
#[derive(Copy, Clone, Debug)]
enum SniffedFormat {
    AnsiPlain,
    Json,
    Html,
}

/// Parse base-60 dump lines from `r` and stream the decoded bytes to `w`.
///
/// Dispatches on `input_format`:
///   * [`InputFormat::Auto`] — sniffs the first non-empty line
///     (HTML / JSON / ansi-plain).
///   * Explicit values force a specific decoder without sniffing.
///
/// Lines without a recognisable digit run are skipped silently, matching
/// the behaviour of tools like `xxd -r` on mixed input. The first
/// malformed digit aborts with a contextual [`io::Error`].
pub(crate) fn decode_stream<R: BufRead, W: Write>(
    mut r: R,
    w: &mut W,
    input_format: InputFormat,
) -> io::Result<()> {
    // Buffer the first non-empty line so we can both sniff and replay it.
    // Bounded by a small retry count so a stream of blank lines doesn't
    // starve the sniffer without progress.
    let mut first = String::new();
    let mut tries = 0_usize;
    loop {
        first.clear();
        let n = r.read_line(&mut first)?;
        if n == 0 {
            // EOF with no non-empty line seen.
            break;
        }
        if !first.trim().is_empty() {
            break;
        }
        tries += 1;
        if tries >= 16 {
            break;
        }
    }

    let fmt = match input_format {
        InputFormat::Auto => sniff(first.trim_start()),
        InputFormat::Ansi | InputFormat::Plain => SniffedFormat::AnsiPlain,
        InputFormat::Json => SniffedFormat::Json,
        InputFormat::Html => SniffedFormat::Html,
    };

    // Replay the buffered first line + the remainder via Read::chain.
    let chained = io::Cursor::new(first.into_bytes()).chain(r);
    let buffered = io::BufReader::new(chained);

    match fmt {
        SniffedFormat::AnsiPlain => decode_from_text(buffered, w),
        SniffedFormat::Json => decode_from_json(buffered, w),
        SniffedFormat::Html => decode_from_html(buffered, w),
    }
}

/// Classify the first non-empty line into a decoder target.
///
/// Matches `<!DOCTYPE` / `<!doctype` / `<html` for HTML (lowercase is
/// what [`crate::format::emit_html`] emits, uppercase is what humans
/// type), and `{"offset":` for NDJSON. Everything else falls through to
/// the ansi/plain text decoder.
fn sniff(first_line: &str) -> SniffedFormat {
    let t = first_line.trim_start();
    if t.starts_with("<!DOCTYPE") || t.starts_with("<!doctype") || t.starts_with("<html") {
        SniffedFormat::Html
    } else if t.starts_with(r#"{"offset":"#) {
        SniffedFormat::Json
    } else {
        SniffedFormat::AnsiPlain
    }
}

/// Ansi/plain decoder: consumes `NN:NN:…:NN` runs with optional
/// `# bytes=0x<hex>` trailer.
///
/// Streaming-friendly truncation: buffers only the LAST observed chunk
/// so we can clip its tail if the trailer says the original input was
/// shorter than the emitted 8-byte-aligned output.
fn decode_from_text<R: BufRead, W: Write>(r: R, w: &mut W) -> io::Result<()> {
    let mut trailer: Option<usize> = None;
    let mut buffered_last: Option<[u8; CHUNK_BYTES]> = None;
    let mut written: usize = 0;
    let mut any_chunk_seen = false;

    for (idx, line) in r.lines().enumerate() {
        let line = line?;
        // Trailer check first: `# bytes=0x<hex>` with `#` prefix.
        if let Some(hex) = parse_trailer_hex(&line) {
            trailer = Some(hex);
            continue;
        }
        let Some(run) = find_digit_run(&line) else {
            continue;
        };
        // `parse_run` currently takes `&str`; Plan 04-02 tightens the
        // signature to `&[u8; RUN_LEN]`. `run` is already length-checked
        // by `find_digit_run`.
        let value = parse_run(run, idx + 1)?;
        let bytes = value.to_be_bytes();

        if let Some(prev) = buffered_last.take() {
            w.write_all(&prev)?;
            written += CHUNK_BYTES;
        }
        buffered_last = Some(bytes);
        any_chunk_seen = true;
    }

    // Flush the final buffered chunk, honouring the trailer if present.
    if let Some(last) = buffered_last {
        match trailer {
            Some(total) if total > written => {
                let tail = (total - written).min(CHUNK_BYTES);
                w.write_all(&last[..tail])?;
            }
            Some(_) => {
                // trailer <= written: already at or past the trailer length;
                // the final chunk is entirely padding — drop it.
            }
            None => {
                w.write_all(&last)?;
            }
        }
    }

    if trailer.is_none() && any_chunk_seen {
        eprintln!(
            "decode: no length metadata; assuming input was 8-byte-aligned. \
             Last chunk may contain zero-padding. Regenerate the dump with \
             base60 v2+ to silence this warning.",
        );
    }

    w.flush()
}

/// Extract the hex length from a `# bytes=0x<hex>` line, if any.
fn parse_trailer_hex(line: &str) -> Option<usize> {
    let rest = line.trim_start().strip_prefix('#')?.trim_start();
    let rest = rest.strip_prefix("bytes=0x")?;
    let end = rest
        .find(|c: char| !c.is_ascii_hexdigit())
        .unwrap_or(rest.len());
    usize::from_str_radix(&rest[..end], 16).ok()
}

/// NDJSON decoder: each chunk line carries a `"bytes":[...]` array; a
/// terminating `{"type":"meta","bytes":N}` line pins the original length.
fn decode_from_json<R: BufRead, W: Write>(r: R, w: &mut W) -> io::Result<()> {
    let mut expected: Option<usize> = None;
    let mut written: usize = 0;
    for line in r.lines() {
        let line = line?;
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(r#"{"type":"meta","bytes":"#) {
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            expected = rest[..end].parse().ok();
            continue;
        }
        if !trimmed.starts_with(r#"{"offset":"#) {
            continue;
        }
        let Some(start) = trimmed.find(r#""bytes":["#) else {
            continue;
        };
        let after_bracket = start + r#""bytes":["#.len();
        let Some(close_rel) = trimmed[after_bracket..].find(']') else {
            continue;
        };
        let inner = &trimmed[after_bracket..after_bracket + close_rel];
        if inner.is_empty() {
            continue;
        }
        for token in inner.split(',') {
            let byte: u8 = token.trim().parse().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("json decode: invalid byte literal {token:?}"),
                )
            })?;
            w.write_all(&[byte])?;
            written += 1;
        }
    }
    if let Some(total) = expected
        && total != written
    {
        eprintln!("decode: json meta bytes={total} but wrote {written}; continuing");
    }
    w.flush()
}

/// HTML decoder: strictly coupled to [`crate::format::emit_html`]'s tag
/// sequence. Strips the outer shell, walks `<span class="...">NN</span>`
/// runs, collects 11 digit pairs per row, and dispatches through the
/// same [`parse_run`] helper as the text decoder.
fn decode_from_html<R: BufRead, W: Write>(mut r: R, w: &mut W) -> io::Result<()> {
    // HTML dumps are small in practice (< 10 MiB). Streaming parse is
    // out of scope for Phase 4; read-to-string keeps the state machine
    // trivial.
    let mut raw = String::new();
    r.read_to_string(&mut raw)?;

    // Strip the outer shell: parse only what lives between <body> and
    // </body>. If either marker is missing, fall back to the full input
    // (an emitter evolution that drops <body> shouldn't break decoding).
    let body_start = raw.find("<body>").map_or(0, |i| i + "<body>".len());
    let body_end = raw.find("</body>").unwrap_or(raw.len());
    let slice = &raw[body_start..body_end];

    let trailer: Option<usize> = slice.find("<!-- bytes=0x").and_then(|idx| {
        let rest = &slice[idx + "<!-- bytes=0x".len()..];
        let e = rest
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(rest.len());
        usize::from_str_radix(&rest[..e], 16).ok()
    });

    // Walk span tags, collect digit pairs from the four recognised classes.
    let mut pairs: Vec<u8> = Vec::new();
    let mut cursor = 0;
    let open_prefix = "<span class=\"";
    while let Some(rel) = slice[cursor..].find(open_prefix) {
        let abs = cursor + rel + open_prefix.len();
        let tail = &slice[abs..];
        let Some(quote_end) = tail.find('"') else {
            break;
        };
        let class = &tail[..quote_end];
        let after_open = abs + quote_end + "\">".len();
        if after_open > slice.len() {
            break;
        }
        let after = &slice[after_open..];
        let Some(close) = after.find("</span>") else {
            break;
        };
        let content = &after[..close];

        if matches!(class, "d-zero" | "d-low" | "d-mid" | "d-high") && content.len() == 2 {
            let bytes = content.as_bytes();
            if bytes[0].is_ascii_digit() && bytes[1].is_ascii_digit() {
                let hi = bytes[0] - b'0';
                let lo = bytes[1] - b'0';
                pairs.push(hi * 10 + lo);
            }
        }
        cursor = after_open + close + "</span>".len();
    }

    // Every 11 pairs = one 8-byte chunk. Incomplete trailing rows drop.
    let mut written: usize = 0;
    let mut buffered_last: Option<[u8; CHUNK_BYTES]> = None;
    for row in pairs.chunks_exact(DIGITS) {
        let mut run = [0_u8; RUN_LEN];
        for (i, &d) in row.iter().enumerate() {
            run[i * (PAIR + 1)] = b'0' + d / 10;
            run[i * (PAIR + 1) + 1] = b'0' + d % 10;
            if i + 1 < DIGITS {
                run[i * (PAIR + 1) + 2] = b':';
            }
        }
        // Synthesised `run` is pure ASCII digits + colons by construction.
        let run_str = std::str::from_utf8(&run).expect("ascii by construction");
        let value = parse_run(run_str, 0)?;
        let bytes = value.to_be_bytes();
        if let Some(prev) = buffered_last.take() {
            w.write_all(&prev)?;
            written += CHUNK_BYTES;
        }
        buffered_last = Some(bytes);
    }
    if let Some(last) = buffered_last {
        match trailer {
            Some(total) if total > written => {
                let tail = (total - written).min(CHUNK_BYTES);
                w.write_all(&last[..tail])?;
            }
            Some(_) => {}
            None => {
                w.write_all(&last)?;
            }
        }
    }
    w.flush()
}

/// Locate the first `NN:NN:...:NN` run of exactly [`DIGITS`] pairs.
/// Returns a borrow of the matched substring, or `None` if no run fits.
fn find_digit_run(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    if bytes.len() < RUN_LEN {
        return None;
    }
    for start in 0..=bytes.len() - RUN_LEN {
        let slice = &bytes[start..start + RUN_LEN];
        if is_digit_run(slice)
            && not_extended_left(bytes, start)
            && not_extended_right(bytes, start + RUN_LEN)
        {
            // `slice` is ASCII by construction, so `from_utf8` can't fail.
            return Some(std::str::from_utf8(slice).expect("ascii"));
        }
    }
    None
}

/// Require plain ASCII digits and colons in the expected positions.
fn is_digit_run(slice: &[u8]) -> bool {
    debug_assert_eq!(slice.len(), RUN_LEN);
    for (i, &b) in slice.iter().enumerate() {
        let in_colon_position = i % 3 == 2;
        let valid = if in_colon_position {
            b == b':'
        } else {
            b.is_ascii_digit()
        };
        if !valid {
            return false;
        }
    }
    true
}

/// Guard against matching the tail of a longer digit run (e.g. a 12-pair
/// line would yield two overlapping 11-pair windows otherwise).
fn not_extended_left(bytes: &[u8], start: usize) -> bool {
    start == 0 || !matches!(bytes[start - 1], b'0'..=b'9' | b':')
}

fn not_extended_right(bytes: &[u8], end: usize) -> bool {
    end == bytes.len() || !matches!(bytes[end], b'0'..=b'9' | b':')
}

/// Decode a validated 11-pair run into its `u64` value.
///
/// Uses `u128` accumulator arithmetic so a hostile input with every
/// digit pinned at `59` (value `60^11 - 1 ≈ 3.65 · 10¹⁹`) overflows
/// cleanly to an error instead of wrapping a `u64`.
fn parse_run(run: &str, line_no: usize) -> io::Result<u64> {
    let mut value: u128 = 0;
    for (i, pair) in run.split(':').enumerate() {
        debug_assert_eq!(pair.len(), 2);
        let bytes = pair.as_bytes();
        let hi = bytes[0] - b'0';
        let lo = bytes[1] - b'0';
        let digit = hi * 10 + lo;
        if digit >= 60 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "line {line_no}: invalid base-60 digit {digit} at pair {}",
                    i + 1
                ),
            ));
        }
        value = value * 60 + u128::from(digit);
    }
    u64::try_from(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("line {line_no}: decoded value exceeds u64::MAX"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(input: &str) -> Vec<u8> {
        let mut out = Vec::new();
        decode_stream(input.as_bytes(), &mut out, InputFormat::Auto).unwrap();
        out
    }

    #[test]
    fn empty_input_yields_nothing() {
        assert!(decode("").is_empty());
    }

    #[test]
    fn zero_chunk_decodes_to_eight_zeros() {
        let line = "00000000  00:00:00:00:00:00:00:00:00:00:00  |........|\n# bytes=0x8\n";
        assert_eq!(decode(line), vec![0_u8; 8]);
    }

    #[test]
    fn classic_5025_roundtrips_to_expected_be_u64() {
        // 1*3600 + 23*60 + 45 = 5025 → u64 BE bytes. Trailer pins the
        // full 8-byte chunk.
        let line = "00000000  00:00:00:00:00:00:00:00:01:23:45  |.......\u{13a1}|\n\
                    # bytes=0x8\n";
        let bytes = decode(line);
        assert_eq!(bytes.len(), 8);
        assert_eq!(u64::from_be_bytes(bytes.try_into().unwrap()), 5025);
    }

    #[test]
    fn ignores_lines_without_digit_runs() {
        let input = "# comment line\nno pairs here\nalso nothing\n";
        assert!(decode(input).is_empty());
    }

    #[test]
    fn multiple_lines_accumulate_in_order() {
        let input = "\
00000000  00:00:00:00:00:00:00:00:00:00:01  |........|
00000008  00:00:00:00:00:00:00:00:00:00:02  |........|
# bytes=0x10
";
        let bytes = decode(input);
        assert_eq!(bytes.len(), 16);
        assert_eq!(u64::from_be_bytes(bytes[..8].try_into().unwrap()), 1);
        assert_eq!(u64::from_be_bytes(bytes[8..].try_into().unwrap()), 2);
    }

    #[test]
    fn rejects_digit_ge_sixty() {
        let line = "00000000  00:00:00:00:00:00:00:00:00:00:99  |........|\n";
        let mut out = Vec::new();
        let err = decode_stream(line.as_bytes(), &mut out, InputFormat::Auto).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("99"));
    }

    #[test]
    fn rejects_twelve_pair_overextension() {
        // Twelve-pair runs encode more than u64; the scanner must refuse
        // to match a sub-window rather than silently producing wrong bytes.
        let line = "12:34:56:01:02:03:04:05:06:07:08:09:10";
        assert!(decode(line).is_empty());
    }

    #[test]
    fn tolerates_ansi_escapes_around_the_run() {
        let line = "\x1b[90m00000000\x1b[0m  \
                    \x1b[90m00\x1b[0m\x1b[90m:\x1b[0m";
        // Deliberately malformed (interleaved escapes). We just need to
        // ensure the decoder doesn't panic; no digit run will be found.
        let _ = decode(line);
    }

    #[test]
    fn accepts_run_embedded_in_free_text() {
        let line = "some prefix 00:00:00:00:00:00:00:00:00:00:01 some suffix\n# bytes=0x8\n";
        let bytes = decode(line);
        assert_eq!(u64::from_be_bytes(bytes.try_into().unwrap()), 1);
    }

    // ---------- REF-04 additions ----------

    #[test]
    fn text_short_tail_14b_truncates_on_trailer() {
        // Emit 14 bytes (0..14); dump has 2 chunk lines + `# bytes=0xe`
        // trailer. Decoder must truncate the second chunk from 8 → 6 bytes.
        let dump = "\
00000000  00:00:00:00:00:00:00:00:00:01:23  |........|
00000008  00:00:00:00:00:00:00:00:00:00:00  |......|
# bytes=0xe
";
        // We don't assert exact bytes here — only that the decoded output
        // is exactly 14 bytes (trailer truncation worked).
        let out = decode(dump);
        assert_eq!(out.len(), 14);
    }

    #[test]
    fn text_8b_aligned_trailer_yields_full_chunk() {
        let dump = "00000000  00:00:00:00:00:00:00:00:00:00:01  |........|\n# bytes=0x8\n";
        let out = decode(dump);
        assert_eq!(out.len(), 8);
        assert_eq!(u64::from_be_bytes(out.try_into().unwrap()), 1);
    }

    #[test]
    fn legacy_no_trailer_aligns_to_eight() {
        // Without a trailer the decoder falls back to 8-byte alignment.
        // stderr warning is verified via the integration test
        // (in-process capture of stderr is awkward).
        let dump = "00000000  00:00:00:00:00:00:00:00:00:00:00  |........|\n";
        let mut out = Vec::new();
        decode_stream(dump.as_bytes(), &mut out, InputFormat::Auto).unwrap();
        assert_eq!(out, vec![0_u8; 8]);
    }

    #[test]
    fn auto_detect_sniffs_json() {
        let json = "\
{\"offset\":0,\"bytes\":[72,105],\"digits\":[0,0,0,0,0,0,0,0,0,30,9],\"ascii\":\"Hi\"}
{\"type\":\"meta\",\"bytes\":2}
";
        let mut out = Vec::new();
        decode_stream(json.as_bytes(), &mut out, InputFormat::Auto).unwrap();
        assert_eq!(out, b"Hi".to_vec());
    }

    #[test]
    fn input_format_override_forces_json_decoder() {
        let json = "\
{\"offset\":0,\"bytes\":[72,105],\"digits\":[],\"ascii\":\"Hi\"}
{\"type\":\"meta\",\"bytes\":2}
";
        let mut out = Vec::new();
        decode_stream(json.as_bytes(), &mut out, InputFormat::Json).unwrap();
        assert_eq!(out, b"Hi".to_vec());
    }

    #[test]
    fn auto_detect_sniffs_html() {
        // Minimal HTML shell with one digit row that decodes to
        // 0x0000_0000_0000_0001. Meta trailer clamps output to 8 bytes.
        let html = "<!doctype html>\n<html><body><pre>\
<span class=\"offset\">00000000</span>  \
<span class=\"d-zero\">00</span>\
<span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span>\
<span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span>\
<span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span>\
<span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span>\
<span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span>\
<span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span>\
<span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span>\
<span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span>\
<span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span>\
<span class=\"sep\">:</span>\
<span class=\"d-low\">01</span>\
\n<!-- bytes=0x8 --></pre></body></html>\n";
        let mut out = Vec::new();
        decode_stream(html.as_bytes(), &mut out, InputFormat::Auto).unwrap();
        assert_eq!(out, vec![0_u8, 0, 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn trailer_hex_parser_accepts_various_widths() {
        assert_eq!(parse_trailer_hex("# bytes=0x0"), Some(0));
        assert_eq!(parse_trailer_hex("# bytes=0xe"), Some(14));
        assert_eq!(parse_trailer_hex("# bytes=0x400"), Some(1024));
        assert_eq!(parse_trailer_hex("# bytes=0x400\n"), Some(1024));
        assert_eq!(parse_trailer_hex("  # bytes=0x10"), Some(16));
        // Not a trailer line: returns None.
        assert_eq!(parse_trailer_hex("00000000  00:00"), None);
        assert_eq!(parse_trailer_hex("# something else"), None);
    }
}
