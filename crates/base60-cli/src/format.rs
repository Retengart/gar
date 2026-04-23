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

use crate::chunk::{CHUNK, be_u64, pad_chunk};
use base60_core::convert::u64_to_base60;
use base60_core::lens::Lens;
use std::io::{self, BufWriter, Write};

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
        let chunk_be = be_u64(pad_chunk(chunk));
        let digits = u64_to_base60(chunk_be);

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
        let chunk_be = be_u64(pad_chunk(chunk));
        let digits = u64_to_base60(chunk_be);

        write!(out, "<span class=\"offset\">{offset:08x}</span>  ")?;

        for (i, &d) in digits.iter().enumerate() {
            if i > 0 {
                out.write_all(b"<span class=\"sep\">:</span>")?;
            }
            write!(out, "<span class=\"{}\">{d:02}</span>", digit_class(d))?;
        }

        out.write_all(b"  <span class=\"delim\">|</span>")?;
        for &b in chunk {
            if b.is_ascii_graphic() || b == b' ' {
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

    out.write_all(HTML_EPILOGUE.as_bytes())?;
    out.flush()
}

const HTML_PROLOGUE: &str = "<!doctype html>
<html lang=\"en\"><head><meta charset=\"utf-8\">
<title>base60 dump</title>
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
        if b.is_ascii_graphic() || b == b' ' {
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
    use base60_core::lens::{AngleLens, TimeLens};

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
        assert!(json(&[], None).is_empty());
    }

    #[test]
    fn json_emits_one_object_per_chunk() {
        let data: Vec<u8> = (0..24).collect();
        let out = json(&data, None);
        assert_eq!(out.lines().count(), 3);
        for line in out.lines() {
            assert!(line.starts_with('{'));
            assert!(line.ends_with('}'));
        }
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
}
