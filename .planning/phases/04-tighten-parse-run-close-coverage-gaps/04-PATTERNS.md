# Phase 4: Tighten `parse_run` + Close Coverage Gaps - Pattern Map

**Mapped:** 2026-04-24
**Files analyzed:** 11 (3 NEW + 8 EDIT)
**Analogs found:** 11 / 11 (every file has a strong in-repo analog)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/base60-cli/tests/reader.rs` (NEW) | integration-test | request-response (spawn + assert) | `crates/base60-cli/tests/cli.rs` | exact (same crate, same harness, same spawn helper) |
| `crates/base60-cli/tests/tui.rs` (NEW) | integration-test | event-driven (env-mutating + in-process TestBackend) | `crates/base60-cli/src/lib.rs` tests (`#[serial(env)]` block, lines 180-231) + `crates/base60-cli/tests/cli.rs` (structure) | role-match (in-process, not spawn) |
| `crates/base60-cli/tests/persist.rs` (NEW) | integration-test | env-driven fallback ladder | `crates/base60-cli/src/lib.rs` `#[serial(env)]` tests (lines 189-218) | role-match (env idiom exact; file layout from `tests/cli.rs`) |
| `crates/base60-cli/src/decode.rs` (EDIT) | parser / transform | request-response (BufRead → byte stream) | existing body (decode.rs:30-119); `format::emit_json`/`emit_html` are the inverse analogs | exact (self) |
| `crates/base60-cli/src/cli.rs` (EDIT) | config / arg-parse | declarative | existing `DecodeArgs` (cli.rs:264-268) + `ViewArgs.format` declaration (cli.rs:234-240) | exact |
| `crates/base60-cli/src/dump.rs` (EDIT) | emitter | streaming write | `dump_all` itself (dump.rs:115-131) — post-loop trailer | exact (self) |
| `crates/base60-cli/src/format.rs` (EDIT) | emitter | streaming write | `emit_json` (format.rs:33-79), `emit_html` (format.rs:83-129) | exact (self) |
| `crates/base60-cli/src/chunk.rs` (EDIT, possible) | utility | const / helper | `chunk.rs` existing (pad_chunk, be_u64) | exact |
| `crates/base60-cli/tests/common/mod.rs` (EDIT) | test-helper (constants) | declarative | constants already in file (mod.rs:223-234) | exact (2-line flip) |
| `crates/base60-cli/tests/cli.rs` (EDIT) | integration-test | request-response | existing `decoder_invalid_digit_99_error_contains_the_digit` (cli.rs:155-167) | exact (tightening within file) |
| `crates/base60-cli/Cargo.toml` (EDIT) | config | declarative | existing `[dev-dependencies]` block (Cargo.toml:30-34) | exact |

## Pattern Assignments

### `crates/base60-cli/tests/reader.rs` (NEW — Plan 04-03)

**Analog:** `crates/base60-cli/tests/cli.rs` (ONLY analog — same harness, same `mod common;` pull-in, same `base60_cmd()` spawn helper, env-free).

**Header pattern** (mirror `tests/cli.rs:1-14`):
```rust
//! Integration tests for the `reader` module (mmap + stdin + file-open-error).
//!
//! Plan 04-03 (TEST-05).

mod common;

use common::base60_cmd;
use predicates::prelude::PredicateBooleanExt;
```

**Spawn + black-box assert pattern** (copy shape from `cli.rs:21-31` `stdin_piped_dump_produces_output`):
```rust
#[test]
fn stdin_piped_dump_produces_output() {
    base60_cmd()
        .args(["--color=never", "--format=plain"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::is_empty().not());
}
```
→ For `reader.rs` new tests: mmap path uses `tempfile::NamedTempFile::new()` + `tmp.write_all(...)` + `.arg(tmp.path())`; stdin path uses `.write_stdin(bytes)`; error path uses `.arg("/definitely/does/not/exist/nope.bin").assert().failure().stderr(...)`. All black-box via `base60_cmd()` (NOT by calling `reader::load` directly — preserves `pub(crate)` narrow surface per Phase 3 D-07).

**Error-path assertion pattern** (copy from `cli.rs:92-100` `color_never_suppresses_ansi_with_clicolor_force` — two-substring `and` predicate):
```rust
.stderr(predicates::str::contains("open").and(predicates::str::contains("nope.bin")));
```
→ The `anyhow::Context` message is produced at `reader.rs:52`: `format!("open {}", path.display())`. Asserting on both `"open"` and the bare filename covers Windows path-separator differences.

**Env-free constraint:** `tests/reader.rs` MUST NOT mutate env (no `#[serial(env)]` needed). This contrasts with Plan 04-04 files.

**Tempfile pattern** (Context7-verified, `tempfile 3.x`):
```rust
use std::io::Write;
let mut tmp = tempfile::NamedTempFile::new().expect("mktemp");
tmp.write_all(b"hello world").expect("write");
tmp.flush().expect("flush");
// tmp.path() → &Path; dropped at end of scope, file auto-deleted.
```

---

### `crates/base60-cli/tests/tui.rs` (NEW — Plan 04-04)

**Primary analog (file structure):** `crates/base60-cli/tests/cli.rs` (`mod common;`, `use common::base60_cmd;`, one `#[test]` per behaviour).

**Primary analog (env-mutation idiom):** `crates/base60-cli/src/lib.rs:189-218` `#[serial(env)]` block.

**Env-mutation pattern to copy verbatim** (`lib.rs:189-218`):
```rust
#[test]
#[serial(env)]
fn auto_with_no_color_env_is_mono() {
    // SAFETY: Rust 2024 marks `env::remove_var` unsafe because parallel
    // threads may observe a half-updated environment. Cargo runs each
    // `#[test]` on its own thread but within the same process, so tests
    // touching env vars must not run concurrently. The risk here is
    // limited to this small set of env-sensitive tests; they only read
    // their own variable and clean up after themselves.
    unsafe { std::env::set_var("NO_COLOR", "1") };
    assert!(!is_ansi(pick_palette(ColorChoice::Auto, true)));
    unsafe { std::env::remove_var("NO_COLOR") };
}
```
→ Every TUI test body follows: `#[test]` + `#[serial(env)]` + `unsafe { set_var(...) }` + SAFETY comment + test body + `unsafe { remove_var(...) }` (cleanup even on panic via `tempdir` drop handles the underlying fs cleanup).

**Required `use` imports (new to this file, not in any analog)** — the planner must add these fresh because no existing file drives ratatui's `TestBackend`:
```rust
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use serial_test::serial;
use std::io::Write;
```

**TUI seam requirement** (Pitfall 4): `src/tui.rs` currently hardcodes `ratatui::run` + `event::read()` at lines 72-89. To make the TUI driveable from a test, extract a `pub(crate) fn run_with_terminal<B: Backend, F: FnMut() -> io::Result<Option<Event>>>` seam. The test passes `TestBackend::new(80, 24)` + a closure popping from a pre-built `Vec<Event>`. Production `run` becomes a thin wrapper delegating to `run_with_terminal` with `crossterm::event::read` as the event source.

**Drive sequence (critical correction from CONTEXT D-15 hint):** the CONTEXT suggests `b1` for bookmarks but the TUI bookmark mode is `m<letter>` (verified at `tui.rs:361` + `tui.rs:428-454` — `BookmarkSet` rejects digits with "bookmarks use a-z, got '1'"). Correct sequence is `j j j j j m a q`.

**State-file glob pattern** (copy from Pitfall 6 remediation — simpler than re-exporting `persist::state_file`):
```rust
let state_dir = tmpdir.path().join("base60");
let entries: Vec<_> = std::fs::read_dir(&state_dir).unwrap().filter_map(Result::ok).collect();
assert_eq!(entries.len(), 1, "expected one state file");
let contents = std::fs::read_to_string(entries[0].path()).unwrap();
assert!(contents.contains("cursor=40"));
assert!(contents.contains("bookmarks=a:40"));
```
→ `persist::serialize` output shape verified at `persist.rs:95-113` (`scroll=`, `cursor=`, `lens=`, `bookmarks=a:<byte>`).

---

### `crates/base60-cli/tests/persist.rs` (NEW — Plan 04-04)

**Analog:** `crates/base60-cli/src/lib.rs:189-218` (env-mutation idiom) + `tests/cli.rs` (file structure).

**Header:**
```rust
//! Integration tests for `persist::state_base_dir` XDG → HOME fallback.
//!
//! Plan 04-04 (TEST-05). Every test is `#[serial(env)]` because all
//! three env vars (`XDG_STATE_HOME`, `HOME`) are process-global.

mod common;

use serial_test::serial;
```

**Exact test body template** (mirror `lib.rs:210-218` structure):
```rust
#[test]
#[serial(env)]
fn state_goes_to_xdg_when_set() {
    let tmpdir = tempfile::tempdir().unwrap();
    // SAFETY: #[serial(env)] guarantees no concurrent env access.
    unsafe { std::env::set_var("XDG_STATE_HOME", tmpdir.path()) };
    unsafe { std::env::remove_var("HOME") };

    drive_tui_to_quit_with_fixture(/* shared helper */);

    assert!(tmpdir.path().join("base60").is_dir(),
        "state dir should be under XDG_STATE_HOME/base60/");
    // SAFETY: #[serial(env)] guarantees no concurrent env access.
    unsafe { std::env::remove_var("XDG_STATE_HOME") };
}
```

**Source-of-truth for the fallback ladder:** `persist.rs:72-80`:
```rust
fn state_base_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os(XDG_STATE_HOME)
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg).join(APP_SUBDIR));
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(FALLBACK_SUBDIR).join(APP_SUBDIR))
}
```
→ Three tests pin three branches: (1) XDG set → `<xdg>/base60/`; (2) XDG unset, HOME set → `<home>/.local/state/base60/`; (3) both unset → `None` / no-op. `APP_SUBDIR = "base60"`, `FALLBACK_SUBDIR = ".local/state"`.

**Shared helper placement (Claude's Discretion per CONTEXT):** planner may put `drive_tui_to_quit_with_fixture` inside `tests/common/mod.rs` (shared with `tests/tui.rs`) OR keep it local per file. Recommend putting it in `tests/common/mod.rs` — one spawn path, reused by both new test files.

**Windows caveat** (Pitfall 8 / Example 8 note): `persist.rs:78` reads `HOME` literally. Windows CI usually exposes `USERPROFILE`, not `HOME`; however, because these tests SET `HOME` explicitly via `env::set_var`, the tests work on all 3 OSes. If Windows behaviour diverges, gate with `#[cfg(not(windows))]`.

---

### `crates/base60-cli/src/decode.rs` (EDIT — Plans 04-01 REF-04 + 04-02 REF-03)

**Analog for existing body preservation:** self (decode.rs:94-119 `parse_run`, decode.rs:30-40 `decode_stream`).

**Imports pattern (extend current imports at lines 17-18):**
```rust
use base60_core::convert::DIGITS;
use std::io::{self, BufRead, Write};
```
→ Planner adds `use crate::cli::InputFormat;` for the new dispatch parameter (REF-04 D-06).

**Core pattern to preserve (`u128` accumulator — decode.rs:94-119):** planner MUST keep the `u128` overflow-detection arithmetic verbatim. Only the outer loop driver and the digit-validity check location change (REF-03 D-09).

**New signature after REF-03** (derived from research Pattern 3, verified bytes-of-run arithmetic):
```rust
fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64> {
    let mut value: u128 = 0;
    for i in 0..DIGITS {
        let pair_start = i * (PAIR + 1);  // 3 per pair: 2 digits + 1 colon
        let hi_byte = run[pair_start];
        let lo_byte = run[pair_start + 1];
        if !hi_byte.is_ascii_digit() || !lo_byte.is_ascii_digit() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("line {line_no}: non-digit byte at pair {}", i + 1),
            ));
        }
        let hi = hi_byte - b'0';
        let lo = lo_byte - b'0';
        let digit = hi * 10 + lo;
        if digit >= 60 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("line {line_no}: invalid base-60 digit {digit} at pair {}", i + 1),
            ));
        }
        value = value * 60 + u128::from(digit);
    }
    u64::try_from(value).map_err(|_| io::Error::new(
        io::ErrorKind::InvalidData,
        format!("line {line_no}: decoded value exceeds u64::MAX"),
    ))
}
```
→ Format strings for `invalid base-60 digit` and `decoded value exceeds u64::MAX` MUST remain character-identical — they are pinned by `tests/cli.rs` after Plan 04-02's tightening (D-10 full-message contains).

**Caller-site change** (decode.rs:36 — only caller):
```rust
// Before:
let value = parse_run(run, idx + 1)?;
// After (planner's option A — try_into at call site):
let value = parse_run(run.try_into().expect("find_digit_run guarantees RUN_LEN"), idx + 1)?;
// After (planner's option B — change find_digit_run to return &[u8; RUN_LEN]):
//   preferred per research Pattern 3 note.
```

**JSON decoder pattern** — `format::emit_json` (format.rs:33-79) is the INVERSE analog. Every emitted line matches `^{"offset":<N>,"bytes":[...]...}$`. Decode helper:
```rust
pub(crate) fn decode_from_json<R: BufRead, W: Write>(r: R, w: &mut W) -> io::Result<()> {
    let mut expected_total: Option<usize> = None;
    let mut written = 0_usize;
    for (idx, line) in r.lines().enumerate() {
        let line = line?;
        if line.starts_with("{\"type\":\"meta\"") {
            // parse the decimal `bytes` field; stash in expected_total
        } else if line.starts_with("{\"offset\":") {
            // locate "\"bytes\":[" substring; parse comma-separated integers until ']';
            // w.write_all(&parsed_bytes)?;
            // written += parsed_bytes.len();
        }
        // other lines silently skipped (matches decode.rs:27 tolerance)
    }
    if let Some(total) = expected_total && total != written {
        eprintln!("decode: meta bytes={total} but wrote {written}; continuing");
    }
    w.flush()
}
```
→ Hand-rolled per Don't-Hand-Roll (research line 267); matches `format.rs:31` docstring precedent.

**HTML decoder pattern** — `format::emit_html` (format.rs:83-129) + `digit_class` (format.rs:151-158) are the INVERSE analogs. Observable tag shapes: `<span class="d-zero|d-low|d-mid|d-high">NN</span>` (digit pair), `<span class="sep">:</span>` (ignored separator), `<span class="offset">HEX</span>` (ignored), `<span class="print|dot">C</span>` (ignored ASCII column), `<span class="delim">|</span>` (ignored), `<!-- bytes=0x<hex> -->` (length metadata), `<!doctype html>` ... `</body>` (shell). The ~60-line state machine consumes spans, recognises the four digit-class tags, parses `NN`, collects 11 per row, converts via existing `parse_run` on a synthesised `[u8; RUN_LEN]`.

**Auto-detect pattern** (research Pattern 2, verified signature shape):
```rust
fn sniff(first_line: &str) -> SniffedFormat {
    let t = first_line.trim_start();
    if t.starts_with("<!DOCTYPE") || t.starts_with("<html") || t.starts_with("<!doctype") {
        SniffedFormat::Html
    } else if t.starts_with("{\"offset\":") {
        SniffedFormat::Json
    } else {
        SniffedFormat::AnsiPlain
    }
}
```
→ Case-insensitive `<!DOCTYPE` match needed because `emit_html` uses lowercase `<!doctype html>` (format.rs:131).

**`decode_stream` signature extension** (Pitfall 8 remediation — first-line peek):
```rust
pub(crate) fn decode_stream<R: BufRead, W: Write>(
    mut r: R,
    w: &mut W,
    input_format: InputFormat,  // NEW param threaded from run_decode
) -> io::Result<()> {
    let mut first = String::new();
    r.read_line(&mut first)?;
    let fmt = match input_format {
        InputFormat::Auto => sniff(&first),
        /* ... explicit overrides ... */
    };
    // Construct a chain: first-line bytes + remainder of r
    let chained = std::io::Read::chain(first.as_bytes(), r);
    match fmt {
        SniffedFormat::AnsiPlain => decode_from_text(io::BufReader::new(chained), w),
        SniffedFormat::Json => decode_from_json(io::BufReader::new(chained), w),
        SniffedFormat::Html => decode_from_html(io::BufReader::new(chained), w),
    }
}
```

**Legacy-dump warning pattern (D-03)** — `decode_from_text` emits to `stderr` at end-of-input if no `# bytes=` line was seen:
```rust
eprintln!("decode: no length metadata; assuming input was 8-byte-aligned. Last chunk may contain zero-padding.");
```
→ No analog in existing CLI — planner picks the exact wording (Claude's Discretion per CONTEXT). Test assertion pins on a substring (e.g., `"no length metadata"`).

**Inline unit-test pattern** (preserve existing `#[cfg(test)] mod tests` at decode.rs:121-201):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn decode(input: &str) -> Vec<u8> {
        let mut out = Vec::new();
        decode_stream(input.as_bytes(), &mut out, InputFormat::Auto).unwrap();
        out
    }
    // ... add: json_roundtrip_inline, html_roundtrip_inline, auto_detect_* ...
}
```
→ New unit tests land inside this existing module block; no new test file at crate root.

---

### `crates/base60-cli/src/cli.rs` (EDIT — Plan 04-01 REF-04)

**Analog:** existing `Format` enum declaration (cli.rs:118-139) + existing `ValueEnum` derive pattern (every enum in the file).

**Declaration pattern to copy** (cli.rs:118-131):
```rust
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum InputFormat {
    /// Auto-detect from the first non-empty line of input.
    #[default]
    Auto,
    /// ANSI-coloured dump (same decode path as `plain`).
    Ansi,
    /// Monochrome dump.
    Plain,
    /// NDJSON (one object per chunk + trailing meta line).
    Json,
    /// Self-contained HTML document (strips shell, parses span classes).
    Html,
}
```
→ `Default = Auto` matches D-06; rustdoc summary style mirrors `Format`'s variants (cli.rs:118-131).

**Flag-on-struct pattern to copy** (cli.rs:234-240 `ViewArgs.format`):
```rust
/// Input format for the dump being decoded. `auto` sniffs the first
/// non-empty line (html/json/ansi-plain).
#[arg(
    long,
    value_enum,
    default_value_t = InputFormat::Auto,
    value_name = "MODE",
)]
pub(crate) input_format: InputFormat,
```
→ Added to `DecodeArgs` (cli.rs:264-268). Naming: `InputFormat` distinct from output `Format` (Pitfall 10). `InputFormat::Ansi` and `InputFormat::Plain` dispatch to the same `decode_from_text` internal (Question 5 recommendation — keep distinct values for symmetry).

**No `::ALL` constant needed** — only `Format::ALL` / `LensMode::ALL` are iterated by matrix tests; no test iterates `InputFormat` variants.

**Caller update** (lib.rs `run_decode`): thread `d.input_format` into `decode_stream(reader, &mut writer, d.input_format)`.

---

### `crates/base60-cli/src/dump.rs` (EDIT — Plan 04-01 REF-04)

**Analog:** self. `dump_all` (dump.rs:115-131) owns the loop; planner appends a post-loop trailer before `out.flush()`.

**Exact diff target** (dump.rs:121-131):
```rust
pub(crate) fn dump_all<W: Write>(
    data: &[u8],
    base_offset: u64,
    w: W,
    palette: &Palette,
    lens: Option<&dyn Lens>,
) -> io::Result<()> {
    let mut out = BufWriter::new(w);
    for (idx, chunk) in data.chunks(CHUNK).enumerate() {
        let offset = base_offset.saturating_add((idx * CHUNK) as u64);
        write_line(&mut out, offset, chunk, palette, lens)?;
    }
    // NEW: length trailer (D-01, D-04). Hash prefix guarantees
    // find_digit_run (decode.rs:44-60) cannot match this line — `#` is
    // neither `[0-9]` nor `:`.
    writeln!(out, "# bytes=0x{:x}", data.len())?;
    out.flush()
}
```

**Hex-format idiom** (copy from `format.rs:97` `{offset:08x}` style):
```rust
writeln!(out, "# bytes=0x{:x}", data.len())?;  // no width, no zero-padding — {:x} is canonical
```
→ `{:x}` confirmed via Don't-Hand-Roll (research line 266).

**Inline unit-test additions** (extend existing `#[cfg(test)] mod tests` at dump.rs:222-419):
```rust
#[test]
fn dump_all_emits_length_trailer() {
    let data: Vec<u8> = (0..14).collect();  // non-8-aligned
    let mut buf = Vec::new();
    dump_all(&data, 0, &mut buf, &PALETTE_NONE, None).unwrap();
    let rendered = String::from_utf8(buf).unwrap();
    assert!(rendered.contains("# bytes=0xe\n"));  // 14 = 0xe
}
```
→ Mirrors `dump_all_emits_one_line_per_chunk` (dump.rs:284-293) structure.

---

### `crates/base60-cli/src/format.rs` (EDIT — Plan 04-01 REF-04)

**JSON-emission analog:** self. `emit_json` (format.rs:33-79) is the file's own analog.

**Exact insert point (JSON — after the chunks loop, before `out.flush()` at format.rs:78):**
```rust
    // existing: } closing the for-loop at line 77
    write!(out, "{{\"type\":\"meta\",\"bytes\":{}}}\n", data.len())?;  // NEW
    out.flush()
```
→ Hand-rolled JSON stays consistent with format.rs:31 docstring ("Hand-rolled to avoid pulling in `serde_json`").

**HTML-emission analog:** self. `emit_html` (format.rs:83-129) is the file's own analog. Insert the `<!-- bytes=0x<hex> -->` comment BEFORE `HTML_EPILOGUE` write at format.rs:127:
```rust
    // existing: closing the chunks for-loop at line 125
    write!(out, "<!-- bytes=0x{:x} -->\n", data.len())?;  // NEW
    out.write_all(HTML_EPILOGUE.as_bytes())?;
    out.flush()
```
→ The comment is between `</pre>` and `</body></html>` (HTML_EPILOGUE at format.rs:149 = `"</pre></body></html>\n"`). A `<!-- ... -->` comment inside or outside `<pre>` is valid per HTML5; choose "just before HTML_EPILOGUE" for simplicity.

**Inline unit-test additions** (mirror format.rs:234-324 `#[cfg(test)] mod tests`):
```rust
#[test]
fn json_emits_meta_line_at_end() {
    let out = json(b"hello", None);
    let last = out.lines().last().unwrap();
    assert_eq!(last, r#"{"type":"meta","bytes":5}"#);
}

#[test]
fn html_document_includes_length_comment() {
    let out = html(b"hello", None);
    assert!(out.contains("<!-- bytes=0x5 -->"));
    assert!(out.ends_with("</pre></body></html>\n"));
}
```
→ Helpers `json`/`html` already defined at format.rs:222-232.

---

### `crates/base60-cli/src/chunk.rs` (EDIT — possible, Plan 04-01)

**Analog:** self. Current `chunk.rs` is 30 lines; no emission logic, only `CHUNK`, `pad_chunk`, `be_u64`.

**Assessment:** REF-04's length-trailer emission is purely in `dump.rs` + `format.rs`. `chunk.rs` likely needs NO change. Planner should NOT introduce new public helpers here unless a cross-format trailer emitter genuinely deduplicates code. If the planner chooses to add a shared `fn emit_length_trailer(w, format, len)`, place it at the bottom of `chunk.rs` with `#[inline]` + `#[must_use]` + `#[derive(Debug)]` on any new type, matching `pad_chunk`'s attribute stack.

---

### `crates/base60-cli/tests/common/mod.rs` (EDIT — Plan 04-01 inside REF-04 commit per D-14)

**Analog:** self. Constants already declared at lines 223-234.

**Exact diff (2 lines):**
```rust
// Before (mod.rs:223-226):
pub const ROUNDTRIP_FIXTURES: &[FixtureEntry] = &[
    ("minimal_elf", fixtures::minimal_elf),
    ("zero_fill_1kib", fixtures::zero_fill_1kib),
];

// After (planner renames AND widens):
pub const ALL_FIXTURES: &[FixtureEntry] = &[
    ("minimal_elf", fixtures::minimal_elf),
    ("zero_fill_1kib", fixtures::zero_fill_1kib),
    ("hello_world", fixtures::hello_world),
    ("minimal_png", fixtures::minimal_png),
    ("minimal_zip", fixtures::minimal_zip),
];
```
```rust
// Before (mod.rs:234):
pub const ROUNDTRIP_FORMATS: &[base60::Format] = &[base60::Format::Ansi, base60::Format::Plain];

// After:
pub const ROUNDTRIP_FORMATS: &[base60::Format] = base60::Format::ALL;
// — or inline as `&[Format::Ansi, Plain, Json, Html]` if a distinct name is wanted.
```

**Consumer update in `tests/roundtrip.rs`:** lines 30-39 reference both constants by the OLD names; flip to `ALL_FIXTURES` + keep `ROUNDTRIP_FORMATS` (or rename there too, planner's call). Matrix expands 5 × 7 × 4 = 140 cells.

**Rustdoc update:** rewrite the doc comment at mod.rs:211-222 to drop the "narrowed to byte-identical subset" language (obsolete after REF-04).

---

### `crates/base60-cli/tests/cli.rs` (EDIT — Plan 04-02 + Plan 04-01)

**Analog:** self. The decoder error-pin test at lines 155-167 is the test being tightened.

**Exact diff on the existing test** (cli.rs:155-167):
```rust
// Before (loose pin, line 166):
.stderr(predicates::str::contains("99").and(predicates::str::contains("invalid")));

// After (full-message pin, D-10):
.stderr(predicates::str::contains(
    "line 1: invalid base-60 digit 99 at pair 11",
));
```

**New error-pin tests to add** (mirror the existing test exactly, change the input + assertion string):
- `decoder_invalid_digit_at_pair_1_reports_pair_1` — input `"00000000  99:00:00:00:00:00:00:00:00:00:00  |........|\n"`; assert `"at pair 1"`.
- `decoder_invalid_digit_at_pair_5_reports_pair_5` — input with 99 at pair 5; assert `"at pair 5"`.
- `decoder_ignores_non_digit_run_lines` — input `"some prefix\n# bytes=0x10\nhello world\n\n"`; assert `.success()` + `.stdout(predicates::str::is_empty())`.

**New input-format override tests** (Plan 04-01, mirror structure of `color_always_forces_ansi_even_in_pipe` at cli.rs:76-86):
```rust
#[test]
fn decode_respects_input_format_override() {
    // A dump with JSON-looking content but `--input-format=plain` forces the text path.
    base60_cmd()
        .arg("decode")
        .args(["--input-format=json"])
        .write_stdin(/* ndjson dump */)
        .assert()
        .success();
}

#[test]
fn decode_legacy_no_trailer_warns_and_continues() {
    let dump = "00000000  00:00:00:00:00:00:00:00:00:00:00  |........|\n";
    base60_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .success()
        .stderr(predicates::str::contains("no length metadata"));
}
```

---

### `crates/base60-cli/Cargo.toml` (EDIT — Plan 04-03)

**Analog:** self. `[dev-dependencies]` block at lines 30-34.

**Exact diff:**
```toml
[dev-dependencies]
assert_cmd = "2"
base60-core = { path = "../base60-core" }
predicates = "3"
serial_test = { version = "3", default-features = false }
tempfile = "3"                                            # NEW — Plan 04-03
```
→ Caret `"3"` matches existing style (`assert_cmd = "2"`, `predicates = "3"`). Verified latest 3.27.0 resolves within MSRV 1.95 (research A1).

## Shared Patterns

### 1. `#[serial(env)]` env-mutation pattern (Plan 04-04 — `tests/tui.rs` + `tests/persist.rs`)

**Source:** `crates/base60-cli/src/lib.rs:189-218`.

**Apply to:** every test function in `tests/tui.rs` and `tests/persist.rs` that reads or mutates `XDG_STATE_HOME` / `HOME`.

**Verbatim template:**
```rust
#[test]
#[serial(env)]
fn test_name() {
    // SAFETY: Rust 2024 marks `env::set_var`/`remove_var` unsafe because parallel
    // threads may observe a half-updated environment. Cargo runs each
    // `#[test]` on its own thread but within the same process, so tests
    // touching env vars must not run concurrently. `#[serial(env)]`
    // (shared key, Phase 2 D-07) enforces that invariant.
    unsafe { std::env::set_var("XDG_STATE_HOME", path) };
    // ... test body ...
    unsafe { std::env::remove_var("XDG_STATE_HOME") };
}
```

**Gate coverage:** `crates/xtask/tests/env_discipline.rs` walks `base60-core/src` + `base60-cli/src` ONLY — it does NOT scan `tests/*.rs` (Pitfall 5 / research A6). Planner MUST manually verify every env-mutation site in the new integration-test files is under `#[serial(env)]`.

### 2. Hermetic `base60_cmd()` spawn pattern (Plans 04-01/02/03/04)

**Source:** `crates/base60-cli/tests/common/mod.rs:38-54`.

**Apply to:** every subprocess invocation of `base60` in integration tests. NO raw `Command::cargo_bin` anywhere outside `common/`.

**Verbatim signature:** `pub fn base60_cmd() -> Command` with `env_clear()` + minimal `PATH`/`SystemRoot`/`USERPROFILE` restoration. The `xtask spawn_discipline` gate enforces — `common/` path is exempted (research line 837).

### 3. `pub(crate)` narrow-surface rule (all source edits)

**Source:** `.planning/phases/03-roundtrip-matrix-fixture-integration/03-CONTEXT.md` D-07; `crates/base60-cli/src/lib.rs:23, 28` (only `Format`, `LensMode` are `pub`).

**Apply to:** every new helper in `decode.rs` (`decode_from_json`, `decode_from_html`, `sniff`, `peek_first_nonempty_line`), `cli.rs` (`InputFormat`), `format.rs` (any trailer helper). All stay `pub(crate)`. Only `InputFormat` needs to cross into integration tests, but it does so via the spawned binary's `--input-format` flag — no library re-export needed.

**Anti-pattern:** adding `pub use cli::InputFormat;` to `lib.rs` — rejected per Phase 3 D-07 narrow-surface intent (research A7).

### 4. Rust 2024 `unsafe` env-mutation SAFETY comments

**Source:** `crates/base60-cli/src/lib.rs:192-198` (the canonical SAFETY comment block).

**Apply to:** every `unsafe { std::env::set_var / remove_var }` block in tests/tui.rs + tests/persist.rs. Minimum 2-line SAFETY comment citing the `#[serial(env)]` guarantee.

### 5. Workspace lint baseline (every new `pub(crate)` fn returning `io::Result`)

**Source:** `.planning/codebase/CONVENTIONS.md`; workspace `[lints]` block.

**Apply to:** every new `pub(crate) fn` in `decode.rs`. Required attrs:
- `/// Summary.` rustdoc
- `/// # Errors` section enumerating every `Err` variant
- `#[derive(Debug)]` on any new struct/enum
- `#[must_use]` on every `const fn` or pure computation returner
- NO `unwrap()`/`expect()` outside `#[cfg(test)]` blocks

**Gate:** `cargo clippy --workspace --all-targets --locked -- -D warnings` + `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` — enforced per D-17.

### 6. Inline `#[cfg(test)] mod tests` for unit tests (Plan 04-01)

**Source:** every source file in `crates/base60-cli/src/` — the crate-wide convention.

**Apply to:** new decoder unit tests (JSON roundtrip, HTML roundtrip, auto-detect) inside `decode.rs`'s existing `mod tests` (decode.rs:121-201); length-trailer unit tests inside `dump.rs`'s existing `mod tests` (dump.rs:222-419) and `format.rs`'s existing `mod tests` (format.rs:217-325).

**Anti-pattern:** creating a new unit-test file at crate root — integration tests live in `tests/*.rs`, unit tests live inline. This split is pinned by the 182-test post-Phase-3 convention (`.planning/codebase/TESTING.md`).

### 7. Error-message literal preservation (Plan 04-02)

**Source:** `crates/base60-cli/src/decode.rs:105-108` (the format string being pinned).

**Apply to:** REF-03's new `parse_run` body — the exact string `"line {line_no}: invalid base-60 digit {digit} at pair {i+1}"` and `"line {line_no}: decoded value exceeds u64::MAX"` must not drift. After the refactor these strings are pinned by `tests/cli.rs` (D-10) — any wording change fails CI.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | — | — | Every new/edited file has a strong in-repo analog. |

HTML state machine is listed as "hand-rolled" but its closest analog — `format::emit_html` — is the exact inverse and serves as the complete specification for its tag shapes. Planner does not invent the parser from scratch; they invert `emit_html`'s output format.

## Metadata

**Analog search scope:**
- `crates/base60-cli/src/` (all 13 modules)
- `crates/base60-cli/tests/` (3 files + common/mod.rs)
- `crates/base60-core/src/` (referenced for conventions only; not edited this phase)
- `crates/xtask/tests/` (env_discipline + spawn_discipline gates for applicability)

**Files scanned:** 17 source files + 4 test files + 2 xtask gates + 2 context/research documents = 25 files.

**Pattern extraction date:** 2026-04-24.
