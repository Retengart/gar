//! Non-ANSI output formats for the viewer.
//!
//! [`dump::dump_all`](crate::dump::dump_all) handles the native
//! terminal rendering; this module provides the alternative shapes a
//! user can request with `--format`:
//!
//! | Format | Characteristics |
//! |--------|-----------------|
//! | `plain`  | Same layout as the default, no ANSI escapes. |
//! | `json`   | Newline-delimited JSON — one object per 8-byte chunk. |
//! | `html`   | Self-contained HTML report with inline CSS heatmap. |
//!
//! The JSON schema is intentionally small so `jq` pipelines can consume
//! it without a companion library. HTML output mirrors the terminal's
//! Sumerian heat-map palette via inline CSS classes.

use crate::chunk::{CHUNK, clamp_filled, is_printable, prepare, read_chunk, skip_bytes};
use gar_core::lens::Lens;
use std::io::{self, BufReader, BufWriter, Read, Write};

/// Emit newline-delimited JSON (ndjson): one object per dump line.
///
/// Each object carries:
/// * `offset`  — absolute byte address (integer)
/// * `bytes`   — the raw bytes of this chunk, one integer per byte
/// * `digits`  — 11 base-60 digits most-significant first
/// * `ascii`   — printable ASCII rendering, with non-printable bytes as `.`
/// * `lens`    — the active lens overlay (omitted when no lens is active)
///
/// Hand-rolled to avoid pulling in `serde_json` for the small, fixed
/// schema — the only strings needing escaping are `ascii` and `lens`.
pub(crate) fn emit_json<W: Write>(
    data: &[u8],
    base_offset: u64,
    w: W,
    lens: Option<&dyn Lens>,
) -> io::Result<()> {
    let mut out = BufWriter::new(w);
    for (idx, chunk) in data.chunks(CHUNK).enumerate() {
        let offset = base_offset.saturating_add((idx * CHUNK) as u64);
        let (chunk_be, digits) = prepare(chunk);

        out.write_all(b"{\"offset\":")?;
        write!(out, "{offset}")?;

        out.write_all(b",\"bytes\":[")?;
        for (i, &b) in chunk.iter().enumerate() {
            if i > 0 {
                out.write_all(b",")?;
            }
            write!(out, "{b}")?;
        }
        out.write_all(b"]")?;

        out.write_all(b",\"digits\":[")?;
        for (i, &d) in digits.iter().enumerate() {
            if i > 0 {
                out.write_all(b",")?;
            }
            write!(out, "{d}")?;
        }
        out.write_all(b"]")?;

        out.write_all(b",\"ascii\":\"")?;
        write_json_string(&mut out, &ascii_rendering(chunk))?;
        out.write_all(b"\"")?;

        if let Some(lens) = lens {
            out.write_all(b",\"lens\":\"")?;
            write_json_string(&mut out, &lens.render(chunk_be))?;
            out.write_all(b"\"")?;
        }

        out.write_all(b"}\n")?;
    }
    // REF-04 (D-01, D-04). Decimal in JSON per convention; emitter stays
    // hand-rolled so the shape matches this file's existing precedent
    // (see the module docstring).
    writeln!(out, r#"{{"type":"meta","bytes":{}}}"#, data.len())?;
    out.flush()
}

/// Emit a self-contained HTML document with the same layout as the
/// terminal renderer and CSS that reproduces the heat-map palette.
pub(crate) fn emit_html<W: Write>(
    data: &[u8],
    base_offset: u64,
    w: W,
    lens: Option<&dyn Lens>,
) -> io::Result<()> {
    let mut out = BufWriter::new(w);
    out.write_all(HTML_PROLOGUE.as_bytes())?;

    for (idx, chunk) in data.chunks(CHUNK).enumerate() {
        let offset = base_offset.saturating_add((idx * CHUNK) as u64);
        let (chunk_be, digits) = prepare(chunk);

        write!(out, "<span class=\"offset\">{offset:08x}</span>  ")?;

        for (i, &d) in digits.iter().enumerate() {
            if i > 0 {
                out.write_all(b"<span class=\"sep\">:</span>")?;
            }
            write!(out, "<span class=\"{}\">{d:02}</span>", digit_class(d))?;
        }

        out.write_all(b"  <span class=\"delim\">|</span>")?;
        for &b in chunk {
            if is_printable(b) {
                out.write_all(b"<span class=\"print\">")?;
                write_html_char(&mut out, b as char)?;
                out.write_all(b"</span>")?;
            } else {
                out.write_all(b"<span class=\"dot\">.</span>")?;
            }
        }
        out.write_all(b"<span class=\"delim\">|</span>")?;

        if let Some(lens) = lens {
            out.write_all(b"  <span class=\"lens\">")?;
            write_html_string(&mut out, &lens.render(chunk_be))?;
            out.write_all(b"</span>")?;
        }

        out.write_all(b"\n")?;
    }

    // REF-04 (D-01, D-04). HTML uses hex (matches ansi/plain). Sits
    // between `</pre>` and `</body></html>` — still valid HTML5 placement.
    writeln!(out, "<!-- bytes=0x{:x} -->", data.len())?;
    out.write_all(HTML_EPILOGUE.as_bytes())?;
    out.flush()
}

/// Emit newline-delimited JSON from a streaming reader, processing
/// 8-byte chunks as they arrive without materialising the full input.
///
/// `skip` bytes are discarded from the front of the reader and also
/// used as the starting offset. `length` optionally caps the number of
/// bytes to process after skipping.
///
/// # Errors
///
/// Propagates any [`io::Error`] returned by the underlying reader or writer.
pub(crate) fn emit_json_stream<R: Read, W: Write>(
    reader: R,
    skip: u64,
    length: Option<u64>,
    w: W,
    lens: Option<&dyn Lens>,
) -> io::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut out = BufWriter::new(w);

    skip_bytes(&mut reader, skip)?;

    let mut offset = skip;
    let mut total: u64 = 0;
    let mut remaining = length;
    let mut chunk_buf = [0u8; CHUNK];

    loop {
        if remaining.is_some_and(|r| r == 0) {
            break;
        }

        let filled = read_chunk(&mut reader, &mut chunk_buf)?;
        if filled == 0 {
            break;
        }

        let actual = clamp_filled(filled, &mut remaining);
        total += actual as u64;

        let chunk = &chunk_buf[..actual];
        let (chunk_be, digits) = prepare(chunk);

        out.write_all(b"{\"offset\":")?;
        write!(out, "{offset}")?;

        out.write_all(b",\"bytes\":[")?;
        for (i, &b) in chunk.iter().enumerate() {
            if i > 0 {
                out.write_all(b",")?;
            }
            write!(out, "{b}")?;
        }
        out.write_all(b"]")?;

        out.write_all(b",\"digits\":[")?;
        for (i, &d) in digits.iter().enumerate() {
            if i > 0 {
                out.write_all(b",")?;
            }
            write!(out, "{d}")?;
        }
        out.write_all(b"]")?;

        out.write_all(b",\"ascii\":\"")?;
        write_json_string(&mut out, &ascii_rendering(chunk))?;
        out.write_all(b"\"")?;

        if let Some(lens) = lens {
            out.write_all(b",\"lens\":\"")?;
            write_json_string(&mut out, &lens.render(chunk_be))?;
            out.write_all(b"\"")?;
        }

        out.write_all(b"}\n")?;
        offset = offset.saturating_add(CHUNK as u64);
    }

    writeln!(out, r#"{{"type":"meta","bytes":{total}}}"#)?;
    out.flush()
}

/// Emit a self-contained HTML document from a streaming reader.
///
/// Behaviour mirrors [`emit_json_stream`] but outputs HTML.
///
/// # Errors
///
/// Propagates any [`io::Error`] returned by the underlying reader or writer.
pub(crate) fn emit_html_stream<R: Read, W: Write>(
    reader: R,
    skip: u64,
    length: Option<u64>,
    w: W,
    lens: Option<&dyn Lens>,
) -> io::Result<()> {
    let mut reader = BufReader::new(reader);
    let mut out = BufWriter::new(w);

    out.write_all(HTML_PROLOGUE.as_bytes())?;

    skip_bytes(&mut reader, skip)?;

    let mut offset = skip;
    let mut total: u64 = 0;
    let mut remaining = length;
    let mut chunk_buf = [0u8; CHUNK];

    loop {
        if remaining.is_some_and(|r| r == 0) {
            break;
        }

        let filled = read_chunk(&mut reader, &mut chunk_buf)?;
        if filled == 0 {
            break;
        }

        let actual = clamp_filled(filled, &mut remaining);
        total += actual as u64;

        let chunk = &chunk_buf[..actual];
        let (chunk_be, digits) = prepare(chunk);

        write!(out, "<span class=\"offset\">{offset:08x}</span>  ")?;

        for (i, &d) in digits.iter().enumerate() {
            if i > 0 {
                out.write_all(b"<span class=\"sep\">:</span>")?;
            }
            write!(out, "<span class=\"{}\">{d:02}</span>", digit_class(d))?;
        }

        out.write_all(b"  <span class=\"delim\">|</span>")?;
        for &b in chunk {
            if is_printable(b) {
                out.write_all(b"<span class=\"print\">")?;
                write_html_char(&mut out, b as char)?;
                out.write_all(b"</span>")?;
            } else {
                out.write_all(b"<span class=\"dot\">.</span>")?;
            }
        }
        out.write_all(b"<span class=\"delim\">|</span>")?;

        if let Some(lens) = lens {
            out.write_all(b"  <span class=\"lens\">")?;
            write_html_string(&mut out, &lens.render(chunk_be))?;
            out.write_all(b"</span>")?;
        }

        out.write_all(b"\n")?;
        offset = offset.saturating_add(CHUNK as u64);
    }

    writeln!(out, "<!-- bytes=0x{total:x} -->")?;
    out.write_all(HTML_EPILOGUE.as_bytes())?;
    out.flush()
}

const HTML_PROLOGUE: &str = "<!doctype html>
<html lang=\"en\"><head><meta charset=\"utf-8\">
<title>gar dump</title>
<style>
  body { background: #111; color: #ddd; }
  pre  { font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 14px; line-height: 1.3; }
  .offset { color: #888; }
  .sep    { color: #666; }
  .delim  { color: #177; }
  .print  { color: #0cc; }
  .dot    { color: #666; }
  .lens   { color: #c0c; }
  .d-zero { color: #666; }
  .d-low  { color: #6c6; }
  .d-mid  { color: #cc6; }
  .d-high { color: #c66; }
</style></head><body><pre>\n";

const HTML_EPILOGUE: &str = "</pre></body></html>\n";

const fn digit_class(d: u8) -> &'static str {
    match d {
        0 => "d-zero",
        1..20 => "d-low",
        20..40 => "d-mid",
        _ => "d-high",
    }
}

/// Dot-rendering of non-printable bytes, matching the terminal column.
fn ascii_rendering(chunk: &[u8]) -> String {
    let mut s = String::with_capacity(chunk.len());
    for &b in chunk {
        if is_printable(b) {
            s.push(b as char);
        } else {
            s.push('.');
        }
    }
    s
}

/// Escape `s` into a JSON string body (without surrounding quotes),
/// handling the six required special chars plus control bytes via `\u00xx`.
fn write_json_string<W: Write>(w: &mut W, s: &str) -> io::Result<()> {
    for c in s.chars() {
        match c {
            '"' => w.write_all(b"\\\"")?,
            '\\' => w.write_all(b"\\\\")?,
            '\n' => w.write_all(b"\\n")?,
            '\r' => w.write_all(b"\\r")?,
            '\t' => w.write_all(b"\\t")?,
            '\x08' => w.write_all(b"\\b")?,
            '\x0c' => w.write_all(b"\\f")?,
            c if (c as u32) < 0x20 => write!(w, "\\u{:04x}", c as u32)?,
            c => {
                // Write as UTF-8; JSON mandates UTF-8 on the wire.
                let mut buf = [0; 4];
                w.write_all(c.encode_utf8(&mut buf).as_bytes())?;
            }
        }
    }
    Ok(())
}

fn write_html_string<W: Write>(w: &mut W, s: &str) -> io::Result<()> {
    for c in s.chars() {
        write_html_char(w, c)?;
    }
    Ok(())
}

fn write_html_char<W: Write>(w: &mut W, c: char) -> io::Result<()> {
    match c {
        '&' => w.write_all(b"&amp;"),
        '<' => w.write_all(b"&lt;"),
        '>' => w.write_all(b"&gt;"),
        '"' => w.write_all(b"&quot;"),
        '\'' => w.write_all(b"&#39;"),
        c => {
            let mut buf = [0; 4];
            w.write_all(c.encode_utf8(&mut buf).as_bytes())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gar_core::lens::{AngleLens, TimeLens};

    fn json(data: &[u8], lens: Option<&dyn Lens>) -> String {
        let mut buf = Vec::new();
        emit_json(data, 0, &mut buf, lens).unwrap();
        String::from_utf8(buf).unwrap()
    }

    fn html(data: &[u8], lens: Option<&dyn Lens>) -> String {
        let mut buf = Vec::new();
        emit_html(data, 0, &mut buf, lens).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn json_empty_input_is_empty_output() {
        // REF-04 (D-02): the meta trailer is always emitted, even for
        // empty input, so the "empty" case is exactly one line.
        let out = json(&[], None);
        assert_eq!(out, "{\"type\":\"meta\",\"bytes\":0}\n");
    }

    #[test]
    fn json_emits_one_object_per_chunk() {
        let data: Vec<u8> = (0..24).collect();
        let out = json(&data, None);
        // 3 chunk lines + 1 trailing meta line.
        assert_eq!(out.lines().count(), 4);
        for line in out.lines() {
            assert!(line.starts_with('{'));
            assert!(line.ends_with('}'));
        }
    }

    #[test]
    fn json_emits_meta_line_at_end() {
        let out = json(b"hello", None);
        let last = out.lines().last().unwrap();
        assert_eq!(last, r#"{"type":"meta","bytes":5}"#);
    }

    #[test]
    fn json_contains_offset_and_bytes_fields() {
        let line = json(b"Hi there", None);
        assert!(line.contains("\"offset\":0"));
        assert!(line.contains("\"bytes\":[72,105,32,116,104,101,114,101]"));
        assert!(line.contains("\"ascii\":\"Hi there\""));
    }

    #[test]
    fn json_escapes_ascii_control_bytes_as_dot() {
        // Non-printables get dotted in the ascii field, matching the
        // terminal renderer — the raw bytes live in `bytes`.
        let out = json(&[0, 0x1f, b'A', 0x7f], None);
        assert!(out.contains("\"ascii\":\"..A.\""));
    }

    #[test]
    fn json_includes_lens_field_when_present() {
        let out = json(&[0_u8; 8], Some(&TimeLens::default()));
        assert!(out.contains("\"lens\":"));
        // Time lens output contains the 𒁹 wedge separator.
        assert!(out.contains("𒁹"));
    }

    #[test]
    fn json_omits_lens_field_when_none() {
        let out = json(&[0_u8; 8], None);
        assert!(!out.contains("\"lens\""));
    }

    #[test]
    fn json_escapes_backslash_and_quote_in_lens_output() {
        // Force a payload containing escape-hazard characters via a
        // custom lens so we cover the write_json_string branches.
        struct TrickyLens;
        impl Lens for TrickyLens {
            fn render(&self, _: u64) -> String {
                "he said \"hi\"\nand\\escaped".to_owned()
            }
        }
        let out = json(&[0_u8; 8], Some(&TrickyLens));
        assert!(out.contains(r#"\"hi\""#));
        assert!(out.contains(r"\\escaped"));
        assert!(out.contains(r"\n"));
    }

    #[test]
    fn html_document_has_prologue_and_epilogue() {
        let out = html(&[0_u8; 8], None);
        assert!(out.starts_with("<!doctype html>"));
        assert!(out.contains("<pre>"));
        // REF-04: trailer comment sits between `</pre>` and the epilogue.
        assert!(out.contains("<!-- bytes=0x8 -->\n"));
        assert!(out.ends_with("</pre></body></html>\n"));
    }

    #[test]
    fn html_document_includes_length_comment() {
        let out = html(b"hello", None);
        assert!(
            out.contains("<!-- bytes=0x5 -->\n"),
            "length comment missing; html was: {out}",
        );
        assert!(out.ends_with("</pre></body></html>\n"));
    }

    #[test]
    fn html_applies_heatmap_classes_to_digits() {
        // Zero bytes → all digit-zero classes.
        let out = html(&[0_u8; 8], None);
        assert!(out.contains("class=\"d-zero\""));
    }

    #[test]
    fn html_escapes_angle_brackets_and_ampersands() {
        let out = html(b"a<b&c>d", None);
        assert!(out.contains("&lt;"));
        assert!(out.contains("&amp;"));
        assert!(out.contains("&gt;"));
    }

    #[test]
    fn html_includes_lens_span_when_lens_active() {
        let out = html(&3_600_000_u64.to_be_bytes(), Some(&AngleLens));
        assert!(out.contains("class=\"lens\""));
        assert!(out.contains("001°00′00.000″"));
    }

    #[test]
    fn json_stream_matches_buffered_for_full_input() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        emit_json_stream(data.as_slice(), 0, None, &mut buf, None).unwrap();
        let streamed = String::from_utf8(buf).unwrap();
        let buffered = json(&data, None);
        assert_eq!(streamed, buffered);
    }

    #[test]
    fn json_stream_matches_buffered_for_short_input() {
        let data = b"hello";
        let mut buf = Vec::new();
        emit_json_stream(data.as_slice(), 0, None, &mut buf, None).unwrap();
        let streamed = String::from_utf8(buf).unwrap();
        let buffered = json(data, None);
        assert_eq!(streamed, buffered);
    }

    #[test]
    fn json_stream_handles_empty_input() {
        let mut buf = Vec::new();
        emit_json_stream(b"".as_slice(), 0, None, &mut buf, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert_eq!(out, "{\"type\":\"meta\",\"bytes\":0}\n");
    }

    #[test]
    fn json_stream_respects_skip() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        emit_json_stream(data.as_slice(), 8, None, &mut buf, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        // First object should have offset 8.
        assert!(out.contains("\"offset\":8"));
        // Meta trailer should report 16 bytes (24 - 8 skipped).
        assert!(out.contains(r#""bytes":16}"#));
    }

    #[test]
    fn json_stream_respects_length() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        emit_json_stream(data.as_slice(), 0, Some(16), &mut buf, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        // 2 chunk lines + 1 meta line.
        assert_eq!(out.lines().count(), 3);
        assert!(out.contains(r#""bytes":16}"#));
    }

    #[test]
    fn json_stream_with_lens_matches_buffered() {
        let data: Vec<u8> = (0..16).collect();
        let lens = TimeLens::default();
        let mut buf = Vec::new();
        emit_json_stream(data.as_slice(), 0, None, &mut buf, Some(&lens)).unwrap();
        let streamed = String::from_utf8(buf).unwrap();
        let mut buf2 = Vec::new();
        emit_json(&data, 0, &mut buf2, Some(&lens)).unwrap();
        let buffered = String::from_utf8(buf2).unwrap();
        assert_eq!(streamed, buffered);
    }

    #[test]
    fn html_stream_matches_buffered_for_full_input() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        emit_html_stream(data.as_slice(), 0, None, &mut buf, None).unwrap();
        let streamed = String::from_utf8(buf).unwrap();
        let buffered = html(&data, None);
        assert_eq!(streamed, buffered);
    }

    #[test]
    fn html_stream_handles_empty_input() {
        let mut buf = Vec::new();
        emit_html_stream(b"".as_slice(), 0, None, &mut buf, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.starts_with("<!doctype html>"));
        assert!(out.contains("<!-- bytes=0x0 -->\n"));
        assert!(out.ends_with("</pre></body></html>\n"));
    }

    #[test]
    fn html_stream_respects_skip() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        emit_html_stream(data.as_slice(), 8, None, &mut buf, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        // First line offset should be 8.
        assert!(out.contains("00000008"));
        // Trailer should report 16 bytes.
        assert!(out.contains("<!-- bytes=0x10 -->\n"));
    }

    #[test]
    fn html_stream_respects_length() {
        let data: Vec<u8> = (0..24).collect();
        let mut buf = Vec::new();
        emit_html_stream(data.as_slice(), 0, Some(16), &mut buf, None).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("<!-- bytes=0x10 -->\n"));
    }

    #[test]
    fn html_stream_with_lens_matches_buffered() {
        let data: Vec<u8> = (0..16).collect();
        let lens = AngleLens;
        let mut buf = Vec::new();
        emit_html_stream(data.as_slice(), 0, None, &mut buf, Some(&lens)).unwrap();
        let streamed = String::from_utf8(buf).unwrap();
        let mut buf2 = Vec::new();
        emit_html(&data, 0, &mut buf2, Some(&lens)).unwrap();
        let buffered = String::from_utf8(buf2).unwrap();
        assert_eq!(streamed, buffered);
    }
}
