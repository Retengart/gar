---
phase: 04-tighten-parse-run-close-coverage-gaps
verified: 2026-04-24T17:15:00Z
status: passed
score: 7/7 must-haves verified
overrides_applied: 0
---

# Phase 4: Tighten `parse_run` + Close Coverage Gaps — Verification Report

**Phase Goal:** `decode::parse_run` accepts `&[u8; RUN_LEN]`, promotes its digit-check inside, and ships only after Phase 3's roundtrip matrix guarantees no silent error-semantics drift. Previously-untested paths (`reader::load_file` mmap, `reader::load_stdin`, TUI exit-with-save, `persist::state_base_dir`) gain direct coverage. Also closes REF-04 — length-preserving `decode` + JSON/HTML decode paths — so Phase 3's roundtrip matrix can widen from 28 cells back to the full 140.

**Verified:** 2026-04-24T17:15:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `parse_run` signature `fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64>` with digit-check internal; single call site compiles without `try_into` at boundary | ✓ VERIFIED | `decode.rs:423` exact signature; digit-check `is_ascii_digit` at lines 431-436; `find_digit_run` returns `Option<&[u8; RUN_LEN]>` at line 354; `decode_from_text` passes `run` directly (line 157) |
| 2 | `tests/reader.rs` covers mmap, stdin, file-open-error paths via `base60_cmd()` | ✓ VERIFIED | 3 `#[test]` functions: `load_file_via_mmap_returns_file_contents`, `load_stdin_via_write_stdin_dumps_piped_bytes`, `load_file_nonexistent_returns_error`; 3/3 pass; no `Command::cargo_bin` outside `common/` |
| 3 | `tests/tui.rs` drives `run_with_terminal` via `TestBackend` + `tempfile::tempdir()`, quits on `q`, asserts `cursor=40` + `bookmarks=a:40` in state file | ✓ VERIFIED | `tests/tui.rs:13-68` exists; uses `TestBackend::new(80, 24)` via helper; `#[serial(env)]` tagged; asserts both `cursor=40` and `bookmarks=a:40`; test passes in 0.03s |
| 4 | `tests/persist.rs` covers XDG → HOME → None ladder, each test `#[serial(env)]` | ✓ VERIFIED | 3 `#[test]` functions: `state_goes_to_xdg_when_set`, `state_falls_back_to_home_when_xdg_unset`, `state_noops_when_both_unset`; 5 `#[serial(env)]` matches (3 tests + 2 doc refs); 13 `unsafe { std::env::... }` blocks with `// SAFETY:` comments |
| 5 | REF-04: length-metadata trailer emitted by every format; JSON + HTML decoders; `--input-format` flag; 140-cell matrix | ✓ VERIFIED | `dump.rs:134` emits `# bytes=0x<hex>`; `format.rs:81` emits `{"type":"meta","bytes":N}`; `format.rs:133` emits `<!-- bytes=0x<hex> -->`; `decode_from_json`/`decode_from_html`/`sniff` present; `cli.rs:152` has `InputFormat` enum; `run_decode` threads `args.input_format`; `ALL_FIXTURES` has 5 entries; `ROUNDTRIP_FORMATS = Format::ALL`; roundtrip test green (0.64s) |
| 6 | Zero-dep core invariant preserved | ✓ VERIFIED | `base60-core/Cargo.toml` has NO `[dependencies]` block; only `[dev-dependencies] serial_test` present |
| 7 | Full workspace gate green (test + clippy + fmt + doc + shell roundtrip) | ✓ VERIFIED | 210 tests pass (139 lib + 16 cli + 4 fixtures + 3 persist + 3 reader + 1 roundtrip + 1 tui + 41 core + 2 xtask); `cargo clippy -- -D warnings` clean; `cargo fmt --check` clean; shell roundtrip `Hello, world!\n` byte-identical across plain/json/html |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/base60-cli/src/decode.rs` | Tight `parse_run(&[u8; RUN_LEN], usize)` + `find_digit_run` returning array borrow + REF-04 dispatch | ✓ VERIFIED | parse_run at line 423; find_digit_run at line 354; decode_from_text/json/html at 139/208/258; sniff at 122; parse_trailer_hex at 197; module doc covers HTML coupling |
| `crates/base60-cli/src/dump.rs` | `# bytes=0x<hex>` trailer in `dump_all` | ✓ VERIFIED | Line 134: `writeln!(out, "# bytes=0x{:x}", data.len())?;`; 3 inline tests pin empty/plain/ansi |
| `crates/base60-cli/src/format.rs` | NDJSON meta + HTML length comment | ✓ VERIFIED | Line 81 NDJSON meta; line 133 HTML comment before `HTML_EPILOGUE` |
| `crates/base60-cli/src/cli.rs` | `InputFormat` enum + `DecodeArgs.input_format` | ✓ VERIFIED | Line 152 enum; line 303 DecodeArgs field |
| `crates/base60-cli/src/lib.rs` | `__test_hooks` + `__TuiTimeScale` + `run_decode` threads input_format | ✓ VERIFIED | Line 37-41 `#[doc(hidden)]` re-exports; line 144 `run_decode`; lines 153/156 pass `args.input_format` |
| `crates/base60-cli/src/tui.rs` | `run_with_terminal<B,F>` seam + `fn run` delegates | ✓ VERIFIED | Line 96 seam (pub + `#[doc(hidden)]`); line 56 `fn run`; line 65 delegates with `crossterm::event::read().map(Some)` |
| `crates/base60-cli/tests/common/mod.rs` | `ALL_FIXTURES` (5), `ROUNDTRIP_FORMATS = Format::ALL`, `drive_tui_to_quit_with_fixture` | ✓ VERIFIED | Line 217 ALL_FIXTURES (5 entries); line 227 ROUNDTRIP_FORMATS; line 337 helper with `j j j j j m a q` drive |
| `crates/base60-cli/tests/reader.rs` | 3 integration tests | ✓ VERIFIED | File present (59 lines); 3 `#[test]`; no raw `Command::cargo_bin`; uses `base60_cmd()` |
| `crates/base60-cli/tests/tui.rs` | TestBackend save-path test | ✓ VERIFIED | File present (68 lines); 1 `#[serial(env)]` test; asserts `cursor=40`+`bookmarks=a:40` |
| `crates/base60-cli/tests/persist.rs` | 3 `#[serial(env)]` ladder tests | ✓ VERIFIED | File present (130 lines); 3 tests; env mutations in `unsafe { ... }` with SAFETY comments |
| `crates/base60-cli/tests/roundtrip.rs` | 140-cell matrix using ALL_FIXTURES × ALL_LENS_CONFIGS × ROUNDTRIP_FORMATS | ✓ VERIFIED | Doc states 5×7×4=140; imports flipped; test passes in 0.64s |
| `crates/base60-cli/tests/cli.rs` | Full-message decoder error pin + pair-1/pair-5 position tests + `--input-format` override tests | ✓ VERIFIED | `line 1: invalid base-60 digit 99 at pair 11` pinned; pair-1 + pair-5 + non-digit-run tolerance tests present; legacy-warning + help-advertising + json/html override tests present |
| `crates/base60-cli/Cargo.toml` | `tempfile = "3"` under `[dev-dependencies]` | ✓ VERIFIED | Line 35 matches; `Cargo.lock` pins `tempfile v3.27.0` |
| `crates/base60-core/Cargo.toml` | No `[dependencies]` block | ✓ VERIFIED | Only `[dev-dependencies] serial_test`; zero-dep invariant preserved |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `src/lib.rs::run_decode` | `src/decode.rs::decode_stream` | `decode::decode_stream(BufReader::new(file), &mut out, args.input_format)` | ✓ WIRED | Both file and stdin paths thread `args.input_format` (lib.rs:153, 156) |
| `src/decode.rs` | `src/cli.rs::InputFormat` | `use crate::cli::InputFormat;` + match dispatch | ✓ WIRED | decode.rs:43 import; decode_stream matches all 5 variants (lines 98-103) |
| `src/dump.rs::dump_all` trailer | `src/decode.rs::find_digit_run` | `#` prefix skipped by `find_digit_run` (neither digit nor colon); consumed by `parse_trailer_hex` | ✓ WIRED | `parse_trailer_hex` at decode.rs:197 strips `#` prefix; `#` fails `is_ascii_digit()` check in `is_digit_run` |
| `tests/roundtrip.rs` | `tests/common/mod.rs` | `use common::{ALL_FIXTURES, ROUNDTRIP_FORMATS, ALL_LENS_CONFIGS, ...}` | ✓ WIRED | Line 17 imports; lines 22-25 iterate 5×7×4 |
| `src/tui.rs::run` | `src/tui.rs::run_with_terminal` | `ratatui::run(|terminal| run_with_terminal(...))` with `crossterm::event::read().map(Some)` | ✓ WIRED | tui.rs:64-74; production closure exactly once |
| `tests/common/mod.rs::drive_tui_to_quit_with_fixture` | `base60::__test_hooks::run_with_terminal` | 8-arg call with TestBackend + `Vec<Event>` iterator closure | ✓ WIRED | common/mod.rs:353-363; calls via `base60::__test_hooks::run_with_terminal` and `base60::__TuiTimeScale::Gar` |
| `tests/persist.rs` + `tests/tui.rs` | `std::env::{set_var, remove_var}` | `#[serial(env)]`-gated `unsafe { ... }` with `// SAFETY:` comments | ✓ WIRED | 13 unsafe blocks in persist.rs; all under `#[serial(env)]` |
| `tests/tui.rs` | `tempfile::tempdir()` | XDG_STATE_HOME redirect + auto-cleanup | ✓ WIRED | tui.rs:16 tempdir; line 34 sets XDG_STATE_HOME to it |

### Data-Flow Trace (Level 4)

Not directly applicable — Phase 4 ships parsers, decoders, and test infrastructure (no UI rendering dynamic DB data). Data flow verified indirectly through the 140-cell roundtrip matrix: for every `(fixture, lens, format)` cell, bytes flow `fixture → dump → decode → bytes` byte-identically. Shell smoke-check confirms real data flows: `Hello, world!\n` (14 bytes) survives dump+decode across plain/json/html formats.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full test suite green | `cargo test --workspace --all-targets --locked` | 210 passed, 0 failed across 12 binaries | ✓ PASS |
| Clippy strict | `cargo clippy --workspace --all-targets --locked -- -D warnings` | Finished, no warnings | ✓ PASS |
| Fmt check | `cargo fmt --all --check` | No output (clean) | ✓ PASS |
| Shell roundtrip plain | `printf 'Hello, world!\n' \| base60 --format=plain \| base60 decode` | `H e l l o ,   w o r l d ! \n` (14 bytes) | ✓ PASS |
| Shell roundtrip json | `printf 'Hello, world!\n' \| base60 --format=json \| base60 decode` | `H e l l o ,   w o r l d ! \n` (14 bytes) | ✓ PASS |
| Shell roundtrip html | `printf 'Hello, world!\n' \| base60 --format=html \| base60 decode` | `H e l l o ,   w o r l d ! \n` (14 bytes) | ✓ PASS |
| Zero-dep core | `grep '^\[dependencies\]' crates/base60-core/Cargo.toml` | Zero hits | ✓ PASS |
| Spawn-discipline | `grep Command::cargo_bin tests/ \| grep -v common/` | Zero hits | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| REF-03 | 04-02-PLAN.md | Tighten `decode::parse_run` — accept `&[u8; RUN_LEN]`, promote digit-check inside | ✓ SATISFIED | `decode.rs:423` signature; lines 431-436 internal digit-check; zero callers construct a raw slice (find_digit_run returns array borrow); full-message stderr pin in tests/cli.rs |
| REF-04 | 04-01-PLAN.md | Length-preserving decode + JSON/HTML decode paths; restore 140-cell matrix | ✓ SATISFIED | Trailer emitted by all 4 formats; `decode_from_json`/`decode_from_html`/`sniff` present; `--input-format` flag wired end-to-end; `ALL_FIXTURES` (5) × `ALL_LENS_CONFIGS` (7) × `ROUNDTRIP_FORMATS = Format::ALL` (4) = 140 cells; roundtrip test green |
| TEST-05 | 04-03-PLAN.md, 04-04-PLAN.md | Coverage for `reader::load_file` mmap, `reader::load_stdin`, TUI exit-with-save via TestBackend, `persist::state_base_dir` env-precedence | ✓ SATISFIED | `tests/reader.rs` (3 tests: mmap/stdin/error); `tests/tui.rs` (1 test asserting cursor=40+bookmarks=a:40); `tests/persist.rs` (3 `#[serial(env)]` tests for XDG→HOME→None ladder) |

No orphan requirements. REQUIREMENTS.md maps exactly REF-03, REF-04, TEST-05 to Phase 4; all three accounted for across the four plans.

### Anti-Patterns Found

Anti-pattern scan across files modified by Phase 4 plans:

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | — | No blocker anti-patterns found | — | — |

Detail:
- No `TODO`/`FIXME`/`XXX`/`HACK` markers in phase-modified source (`decode.rs`, `dump.rs`, `format.rs`, `cli.rs`, `lib.rs`, `tui.rs`).
- `.expect(...)` calls appear only in test code (CLAUDE.md permits in `#[cfg(test)]` and test helpers).
- `unsafe { ... }` blocks in tests are all documented with `// SAFETY:` comments (13 instances in `tests/persist.rs`, 2 in `tests/tui.rs`).
- Empty returns (`return null`, `=> {}`) not present in added code.
- No hardcoded empty props / placeholder UI (not applicable — pure Rust CLI).
- `#[allow(clippy::too_many_arguments)]` on `run_with_terminal` is documented in SUMMARY and justified (8 params map 1:1 to prod path + test injection).

### Human Verification Required

None — all must-haves verified programmatically. One known caveat documented by 04-04-SUMMARY (non-blocking):

- **TUI production path visual smoke check.** The executor could not run `cargo run -p base60 -- -i README.md` interactively. Production-path equivalence argued via (a) `fn run` is 13 lines of trivial delegation, (b) `crossterm::event::read().map(Some)` is a mechanical 1:1 preservation, (c) full workspace gate green, (d) non-TUI path smoke-checked with real input. A reviewer may wish to confirm `j` moves cursor and `q` quits cleanly on a real terminal once before merging, but this does NOT block phase acceptance — the in-process `TestBackend` TUI integration test fully exercises the save path through the seam.

### Gaps Summary

No gaps. Every ROADMAP success criterion, every plan-frontmatter must-have, every declared requirement ID (REF-03, REF-04, TEST-05), and every key link is verified in the actual codebase. The full workspace gate (210 tests + clippy `-D warnings` + fmt `--check` + shell-level JSON/HTML/plain roundtrips) is green. The zero-dep core invariant is preserved. Spawn-discipline + env-discipline gates pass. Phase 4 goal — `parse_run` tightening shipped behind the REF-04 safety net + three previously-untested paths now covered + 140-cell matrix restored — is fully achieved.

---

_Verified: 2026-04-24T17:15:00Z_
_Verifier: Claude (gsd-verifier)_
