# Phase 4: Tighten `parse_run` + Close Coverage Gaps - Context

**Gathered:** 2026-04-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Three independent strands in one phase, intentionally bundled by ROADMAP to close the open contract-tightening work before Phase 5 ships fuzz/bench scaffolding:

1. **REF-03** — `decode::parse_run` signature tightens from `&str` to `&[u8; RUN_LEN]`; digit-check promoted inside the function; error-semantics contract explicitly pinned against drift.
2. **REF-04** — length-preserving `decode` + JSON/HTML decode paths (filed during Phase 3 Plan 03-02 execution). Unlocks the full 140-cell roundtrip matrix (flip `ROUNDTRIP_FIXTURES → ALL_FIXTURES`, `ROUNDTRIP_FORMATS → Format::ALL` in `tests/common/mod.rs`).
3. **TEST-05** — coverage for currently-untested paths: `reader::load_file` (mmap), `reader::load_stdin`, TUI exit-with-save via `TestBackend`, `persist::state_base_dir` env-precedence ladder.

Requirements: **REF-03**, **REF-04**, **TEST-05**.

**In scope:**
- `crates/base60-cli/src/decode.rs` — tighten `parse_run` to `&[u8; RUN_LEN]` + add JSON decoder + add HTML decoder + auto-detect input format.
- `crates/base60-cli/src/chunk.rs` + `crates/base60-cli/src/dump.rs` + `crates/base60-cli/src/format.rs` — emit trailing `# bytes=0x<hex>` length-metadata line on every dump (ansi/plain/html/json).
- `crates/base60-cli/tests/common/mod.rs` — flip `ROUNDTRIP_FIXTURES` → `ALL_FIXTURES`, `ROUNDTRIP_FORMATS` → `Format::ALL` so the matrix widens to 140 cells (ships inside the REF-04 commit per D-17).
- `crates/base60-cli/tests/cli.rs` — expand decoder-error pin to full-message contains (`"line 1: invalid base-60 digit 99 at pair 11"`) + 2-3 new error-path tests.
- `crates/base60-cli/tests/reader.rs` (NEW) — mmap coverage via tempfile + stdin coverage via synthetic BufRead.
- `crates/base60-cli/tests/tui.rs` (NEW) — `ratatui::backend::TestBackend` + `tempfile::tempdir` + `$XDG_STATE_HOME` redirect + drive TUI to `q` + assert state-file content.
- `crates/base60-cli/tests/persist.rs` (NEW) — `state_base_dir` XDG→HOME fallback ladder, `#[serial(env)]`.
- `crates/base60-cli/Cargo.toml` — add `tempfile = "3"` to `[dev-dependencies]`.

**Not in scope:**
- Additional decode-input formats beyond JSON/HTML/ansi/plain.
- `search::Pattern` property tests → Phase 5 fuzz.
- Performance work → Phase 6.
- `cargo public-api` diff tooling → Phase 7 or future.
- Criterion benches → Phase 5.
- Any change to `base60-core` — zero-dep invariant holds.

</domain>

<decisions>
## Implementation Decisions

### REF-04 length metadata format (Area 1)

- **D-01:** Metadata position — **trailing line** after the last digit-run line. `# bytes=0x<hex>\n` appended at the end of every dump (ansi/plain). HTML wraps as `<!-- bytes=0x<hex> -->` at the end of `<body>` before `</body>`. JSON emits a final NDJSON line `{"type":"meta","bytes":<decimal>}\n` (hex in JSON is non-idiomatic — decimal in JSON only; ansi/plain/html use hex).
- **D-02:** Emission rule — **always emit**, regardless of whether the input length is a multiple of 8. Uniform output simplifies the decoder and makes the length invariant observable by the user in every dump.
- **D-03:** Legacy handling — when `decode` consumes a dump file without the `bytes=` trailer, fall back to the 8-byte-aligned behaviour of the current `decode_stream`. Emit a single-line `stderr` warning: `decode: no length metadata, assuming input was 8-byte-aligned; last chunk may contain zero-padding`. No crash, no exit 1 — backwards-compatible per PROJECT.md line 114-115 (JSON schema + `decode` accept-format must stay stable and additive).
- **D-04:** Syntax — field name is **`bytes`** (not `length`). Hex value in ansi/plain/html (`# bytes=0x400`, `<!-- bytes=0x400 -->`). Decimal in JSON (`"bytes":1024` — JSON convention). The `#` prefix guarantees the existing `decode::find_digit_run` ignores the trailer (no run matches inside `# bytes=`).

### REF-04 HTML decode strategy (Area 2)

- **D-05:** Parser style — **small state-machine on tag patterns** (not naive strip-all-tags). The state machine recognises the two concrete shapes `format::emit_html` produces: `<span class="d-low|d-mid|d-hi">NN</span>` → digit pair, `<span class="sep">:</span>` → separator. ~60 lines, no regex dep, strict about the emitter's own format. If `emit_html` evolves, this parser must be updated in lockstep — document that coupling in a module-level comment.
- **D-06:** Format detection — **auto-detect first AND `--input-format` override flag**. Default behaviour: decode sniffs the first non-empty line — `<!DOCTYPE` or `<html` → HTML, `{"offset":` → JSON, otherwise ansi/plain. Override flag `--input-format={auto,ansi,plain,json,html}` (default `auto`) is scope-adjacent but cheap (~6 lines of `clap` derive + dispatch) and avoids surprise auto-misdetection for piped inputs with unusual prefixes.
- **D-07:** Shell handling — strip everything outside `<body>…</body>`, parse the body. Planner handles the `<body>` boundary detection with a simple substring scan; no HTML parser dep. Robust against our own generated shell (head/body/meta tags) but **not** a general HTML parser.
- **D-08:** Malformed-HTML policy — **skip bad lines silently**, matching the existing `decode::decode_stream` behaviour (`Lines without a recognisable digit run are skipped silently, matching the behaviour of tools like xxd -r on mixed input` — module doc). The state machine treats any tag sequence it doesn't recognise as empty output for that line. No exit 1 on malformed HTML.

### REF-03 migration strategy (Area 3)

- **D-09:** Migration approach — **in-place rewrite**. `decode::parse_run` signature changes from `fn parse_run(run: &str, line_no: usize) -> io::Result<u64>` to `fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64>`. Only caller (`decode_stream` line 36) is updated in the same commit. Pitfall 8's advice to ship `parse_run_strict` alongside and migrate callers one-by-one is dropped because there's exactly one caller — keeping two versions creates noise without mitigation value. Signature bump is caught by compiler immediately.
- **D-10:** Error-message pin — **full-message contains** (not split-across-multiple-asserts). In `tests/cli.rs`, expand `decoder_invalid_digit_99_error_contains_the_digit` to assert `.stderr(predicates::str::contains("line 1: invalid base-60 digit 99 at pair 11"))` — the full message verbatim. Locks line-number + pair-position + digit-value into the contract. If REF-03's refactor drifts any of those three, the test fails deterministically rather than silently.
- **D-11:** Additional error-path tests — **add 2-3 tests** pinning error-position semantics:
  - `decoder_invalid_digit_at_pair_1_reports_pair_1` — input where the first pair is `99`; stderr contains `at pair 1`.
  - `decoder_invalid_digit_at_pair_5_reports_pair_5` — input where the fifth pair is `99`; stderr contains `at pair 5`.
  - `decoder_ignores_non_digit_run_lines` — input with non-digit garbage; decode emits nothing and exits 0 (pins `find_digit_run` tolerance semantics). This also catches the case where REF-03 might incorrectly collapse "no digit run found" into an error.

### Scope + ordering + TEST-05 (Area 4)

- **D-12:** Commit order within phase — **REF-04 → REF-03 → TEST-05 reader → TEST-05 TUI/persist**. REF-04 widens the roundtrip matrix from 28 → 140 cells, strengthening the safety net for REF-03 (Pitfall 8: "the expanded matrix is the safety net for the refactor"). TEST-05 touches wholly disjoint files (`reader.rs`, `tui.rs`, `persist.rs`) so it's parallel-safe with the refactors but sequencing after keeps commit story simple.
- **D-13:** Plan count — **4 plans**:
  1. `04-01-PLAN.md` — `feat(cli): length-preserving decode + JSON/HTML decode paths + matrix widen [REF-04]` (metadata emit across 4 formats + decode_from_json + decode_from_html + auto-detect + `--input-format` flag + matrix widen to 140 cells + legacy 8-byte fallback + warning).
  2. `04-02-PLAN.md` — `refactor(cli): tighten parse_run contract + expand decoder error-pin [REF-03]` (parse_run signature change + caller migration + expanded error-message pin + 2-3 new error-path tests).
  3. `04-03-PLAN.md` — `test(cli): reader coverage — mmap + stdin paths [TEST-05]` (new `tests/reader.rs` + `tempfile = "3"` dev-dep).
  4. `04-04-PLAN.md` — `test(cli): TUI TestBackend + persist env-fallback coverage [TEST-05]` (new `tests/tui.rs` + new `tests/persist.rs`, both `#[serial(env)]`).
- **D-14:** Matrix widen commit location — **ship inside REF-04's commit**. REF-04 is the code change that enables the widen; flipping the two slice constants (`ROUNDTRIP_FIXTURES → ALL_FIXTURES`, `ROUNDTRIP_FORMATS → Format::ALL`) lives naturally inside the same commit as the decoder change. Keeps one atomic unit; avoids a "widen is green after REF-04 lands" timing coupling.
- **D-15:** TEST-05 coverage — **three sub-targets**: `reader::load_file` + `reader::load_stdin` (Plan 04-03), TUI exit-with-save (Plan 04-04), `persist::state_base_dir` ladder (Plan 04-04). `search::Pattern` property tests are explicitly deferred to Phase 5 fuzz.
- **D-16:** `tempfile` dev-dep — added in Plan 04-03 (first need: mmap fixture). Version: `tempfile = "3"` (matches Phase 3's deferred addition per CONTEXT Phase 3 D-22).
- **D-17:** Each commit is atomic and independently green — the D-24 gate from Phase 3 (`cargo test --workspace --all-targets --locked` + `cargo clippy … -- -D warnings` + `cargo fmt --all --check` + `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS=-D warnings`) must pass between each commit. Matches Phase 1/2/3 commit-granularity convention.

### Claude's Discretion

- Exact state-machine implementation for HTML decode (Area 2 D-05) — planner picks the ~60-line shape. Module-level doc comment MUST note the tight coupling to `format::emit_html`.
- Exact bytes of the `base60::decode` legacy warning message — something like `"decode: no length metadata; assuming input was 8-byte-aligned. Last chunk may contain zero-padding. Regenerate the dump with base60 v2+ to silence this warning."` — planner's wording.
- Format detection order (JSON sniff before HTML sniff or vice versa) — planner picks based on how `first_non_empty_line` is read.
- Whether `--input-format=auto` is the default (yes per D-06) expressed via `#[clap(default_value = "auto")]` or by omitting the flag when auto — planner picks cleaner syntax.
- TUI TestBackend canvas size + exact keystrokes to drive (D-15) — recommended: 80×24, drive `j` × 5 + `b1` (bookmark) + `q` (quit-with-save).
- Whether the `# bytes=` line is followed by a newline or EOF — recommended: always terminated by `\n` to match existing line-oriented output.
- `reader::load_stdin` synthetic BufRead: planner picks `io::Cursor<Vec<u8>>` vs writing a tiny fake `BufRead` impl.

### Folded Todos

(None — `gsd-sdk query todo.match-phase 4` not run; no pending todos expected in this project.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level decisions

- `.planning/PROJECT.md` — Line 114-115: JSON schema and `decode` accept-format are stable; any change must be additive and gated. Line 33: FMT-02 NDJSON schema (offset/bytes/digits/ascii/lens) is the v1 contract.
- `.planning/REQUIREMENTS.md` — **REF-03** (line 18), **REF-04** (line 19, filed during Phase 3), **TEST-05** (line 26). Coverage: 17/17 post-REF-04-filing.
- `.planning/ROADMAP.md` — Phase 4 section (lines 64-73). Depends on Phase 3 (matrix safety net) + Phase 2 (serial_test). "Matrix can widen 28 → 140 after REF-04 ships" noted inline in Phase 4 Goal.

### Prior-phase context (precedents adopted here)

- `.planning/phases/03-roundtrip-matrix-fixture-integration/03-CONTEXT.md` — D-13 (decoder error-message pin using `"99" + "invalid"`), D-22 (deferred `tempfile = "3"`), D-23 (commit granularity = N sequential atomic commits per plan). D-09 (`LensMode::ALL` + `Format::ALL` widened to `pub`) enables the matrix iteration from tests.
- `.planning/phases/03-roundtrip-matrix-fixture-integration/03-02-SUMMARY.md` — "Scope Deviation" block documenting Problem A (decode can't parse JSON/HTML) and Problem B (dump/decode not byte-identical for non-8-aligned inputs) with a concrete reproducible `printf "test" | base60 | base60 decode` example. The root-cause analysis REF-04 is addressing.
- `.planning/phases/02-env-test-serialisation/02-CONTEXT.md` — `#[serial(env)]` idiom + the single shared env key + xtask `env_discipline.rs` gate. Plan 04-04's persist/TUI env-mutating tests MUST use `#[serial(env)]`.
- `.planning/phases/01-refactor-foundations/01-CONTEXT.md` — `LensMode::ALL` + `be_u64` CLI-local placement. REF-04 is parallel-safe with these (disjoint file surfaces).

### Pitfall remediations this phase consumes

- `.planning/research/PITFALLS.md §"Pitfall 8"` — `parse_run` refactor silently drifts error-message semantics. Addressed by D-10 (full-message pin) + D-11 (2-3 position-pinning tests). The expanded 140-cell matrix (shipped in REF-04's commit) is the roundtrip safety net per Pitfall 8's "Prevention strategy".
- `.planning/research/PITFALLS.md §"Pitfall 5"` — `be_u64` promotion must not leak into `base60-core`. REF-04 touches `chunk.rs`/`dump.rs`/`format.rs` but NOT `base60-core`. Zero-dep invariant preserved.

### Codebase intelligence

- `.planning/codebase/TESTING.md` — current 182-test inline-module idiom (post-Phase-3). Plans 04-03 and 04-04 add `tests/reader.rs`, `tests/tui.rs`, `tests/persist.rs` (3 new integration test files) — extends the pattern established in Phase 3 `tests/`.
- `.planning/codebase/CONVENTIONS.md` — `pub(crate)` default, `#[must_use]`, doc comments on every `pub(crate)`-or-above item, clippy `pedantic + nursery + cargo -D warnings`. Applies to new `decode_from_json` / `decode_from_html` / HTML state machine.
- `.planning/codebase/STRUCTURE.md` — workspace layout; `decode.rs` lives in `base60-cli`. New HTML decoder submodule (e.g., `decode/html.rs`) is planner's option.
- `.planning/codebase/INTEGRATIONS.md` — CI shape; new tests land inside existing `cargo test --workspace --all-targets --locked` step with zero CI YAML changes.

### Source files this phase edits or creates

**NEW:**
- `crates/base60-cli/tests/reader.rs` — mmap + stdin coverage (Plan 04-03).
- `crates/base60-cli/tests/tui.rs` — TUI exit-with-save via `TestBackend` (Plan 04-04).
- `crates/base60-cli/tests/persist.rs` — `state_base_dir` env-fallback ladder (Plan 04-04).

**EDIT:**
- `crates/base60-cli/src/decode.rs` — `parse_run` signature change (REF-03) + JSON decoder + HTML decoder + auto-detect + `--input-format` handling (REF-04).
- `crates/base60-cli/src/cli.rs` — add `--input-format` flag to `decode` subcommand.
- `crates/base60-cli/src/dump.rs` + `crates/base60-cli/src/format.rs` — emit trailing `# bytes=0x<hex>` / `<!-- bytes=0x<hex> -->` / NDJSON meta line (REF-04).
- `crates/base60-cli/tests/common/mod.rs` — flip `ROUNDTRIP_FIXTURES → ALL_FIXTURES`, `ROUNDTRIP_FORMATS → Format::ALL` (inside REF-04 commit per D-14).
- `crates/base60-cli/tests/cli.rs` — expand decoder-error pin to full-message (REF-03 Plan 04-02) + 2-3 error-path tests.
- `crates/base60-cli/Cargo.toml` — add `tempfile = "3"` to `[dev-dependencies]` (Plan 04-03).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `decode::find_digit_run` (decode.rs:44-60) — the existing scanner handles "skip anything that isn't a digit run" and tolerates surrounding content including ANSI escapes. Works as-is for ansi/plain; HTML decode pre-strips tags (per D-05) so the final hand-off to `parse_run` goes through the same path.
- `format::emit_json` (format.rs:34-80) — emits `"bytes":[...]` per-chunk with real unpadded length. Implication for REF-04: JSON decode is trivial — iterate lines, skip meta line, parse `bytes` array, concatenate. Length metadata in JSON is a redundant safety check, not a correctness requirement.
- `tests/common/mod.rs::base60_cmd()` (from Phase 3) — the only spawn path; `.env_clear()` + Windows env restore is reusable for Plan 04-03/04-04 integration tests.
- `tests/common/mod.rs::ROUNDTRIP_FIXTURES` + `ROUNDTRIP_FORMATS` (from Phase 3 Plan 03-02) — already defined as separate slices from `ALL_FIXTURES` + `Format::ALL` precisely to make the widen a 2-line flip.
- `crates/xtask/tests/env_discipline.rs` + `spawn_discipline.rs` (from Phases 2/3) — gates apply to new tests in Plans 04-03/04-04 automatically. No changes needed.

### Established Patterns

- `pub(crate)` default in `base60-cli` — new decode helpers (`decode_from_json`, `decode_from_html`, HTML state machine) stay `pub(crate)`. Only `run`, `LensMode`, `Format`, `LensMode::ALL`, `Format::ALL` are `pub` per Phase 3 D-07.
- `u128` accumulator in `parse_run` (decode.rs:94-119) — preserved verbatim after signature change. Overflow semantics unchanged.
- `debug_assert!` for invariants, checked arithmetic for user input (CLAUDE.md pattern).
- `#[cfg(test)] mod tests` inline blocks — new helpers ship with inline unit tests; integration tests live in `tests/*.rs`.

### Integration Points

- `decode_stream<R: BufRead, W: Write>` (decode.rs:30) — signature unchanged. Internal dispatch on format (new: auto-detect first line or `--input-format` override) routes to one of three decoders: ansi/plain (current), JSON (new), HTML (new). All three converge on the byte-stream `w.write_all(&value.to_be_bytes())?` output path.
- `tempfile` dev-dep — first added in Plan 04-03 for mmap-path fixture. Plan 04-04 reuses it for `TestBackend` + `XDG_STATE_HOME` redirect. No core dep — stays in `[dev-dependencies]` of `base60-cli` only.
- `persist::state_base_dir` (persist.rs) — already has 7 inline tests. Plan 04-04's `tests/persist.rs` replicates the XDG→HOME fallback logic as an **integration** test with `#[serial(env)]` guards per Phase 2 D-07.

### Constraints from existing CI

- `cargo fmt --all --check` — every new test file must be rustfmt-clean.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` with pedantic+nursery+cargo — applies to HTML state machine, JSON decoder, all new test code.
- `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS: -D warnings` — `pub(crate)` HTML/JSON decoders need `# Errors` sections if they return `io::Result`.
- `xtask env_discipline` — Plan 04-04 must ensure every `env::set_var`/`env::remove_var` is under `#[serial(env)]`. Plan 04-03's `reader.rs` is env-free.
- `xtask spawn_discipline` — Plans 04-03/04-04 use `base60_cmd()` from `tests/common/mod.rs` (single spawn path). No raw `Command::cargo_bin`.

</code_context>

<specifics>
## Specific Ideas

- `decode_from_json` is essentially: `for line in lines { if line.starts_with("{\"offset\":") { parse bytes[] and w.write_all(&bytes) } else if line.starts_with("{\"type\":\"meta\"") { record total_len; skip } else { skip } }`. The final length can be used as a sanity check against accumulated output length — if they mismatch, emit a `stderr` warning but don't fail (matches D-03 / D-08 tolerance policy).
- HTML state machine recognises exactly these tag patterns: `<span class="d-low">NN</span>`, `<span class="d-mid">NN</span>`, `<span class="d-hi">NN</span>`, `<span class="sep">:</span>`, `<span class="offset">HEX</span>` (ignored), `<span class="ascii">TXT</span>` (ignored), `<!-- bytes=0x<hex> -->` (records length), and the outer `<html>`/`<head>`/`<body>` shell (ignored). Any other tag is consumed and discarded. Non-tag non-whitespace text outside `<body>` is also ignored.
- Full error-pin assertion in `tests/cli.rs` (D-10): `.stderr(predicates::str::contains("line 1: invalid base-60 digit 99 at pair 11"))`. Matches the format string in `decode.rs:105-107` verbatim. Drift-detection: if REF-03 changes "pair" → "position", "line" → "row", or alters the separator, this test fails immediately.
- Matrix widen (D-14): exactly 2 lines in `tests/common/mod.rs` change — the slice constants. After the widen, `roundtrip_matrix_byte_identical` iterates 5 × 7 × 4 = 140 cells instead of 2 × 7 × 2 = 28.
- TUI TestBackend drive sequence (Claude's Discretion hint): instantiate 80×24 TestBackend + tempdir as `$XDG_STATE_HOME` + `base60 -i FILE` → send keys `j j j` (scroll) + `b1` (bookmark slot 1) + `q` (quit). Then read `$XDG_STATE_HOME/base60/<fnv1a-hash>.state` and assert `cursor=24 scroll=0 bookmarks=1:24` (exact values depend on fixture).

</specifics>

<deferred>
## Deferred Ideas

- **`cargo public-api` diff check** — recommended by Pitfall 5 for REF-04 (HTML/JSON decode adds new `pub(crate)` items; even though they're not `pub`, a `cargo public-api` snapshot check would catch accidental `pub` leaks). Deferred to Phase 7 CI hardening.
- **`search::Pattern` property / fuzz tests** — deferred to Phase 5 (TEST-02 fuzz targets include `Pattern::from_str`).
- **Additional decode-input formats beyond the 4 we emit** — out-of-scope; the v2 hardening milestone is about closing contracts, not extending formats.
- **`--output-format` flag on `decode`** — currently decode always emits raw bytes. Variants (hex, json) would be new features, deferred to v3 theme.
- **Widening TUI coverage beyond exit-with-save** — search flow, lens cycle, analyze view, scroll navigation. Phase 4 covers the save-path; broader TUI testing can be a follow-up if a bug demands it.
- **Replacing line-based `find_digit_run` with a streaming byte-level scanner** — performance improvement deferred to Phase 6 (PERF pass).

</deferred>

---

*Phase: 04-tighten-parse-run-close-coverage-gaps*
*Context gathered: 2026-04-24*
