---
phase: 04-tighten-parse-run-close-coverage-gaps
plan: 01
subsystem: cli-decode
tags: [rust, cli, decode, format, roundtrip, ref-04, length-metadata, ndjson, html, auto-detect]

# Dependency graph
requires:
  - phase: 03-roundtrip-matrix-fixture-integration
    provides: "ROUNDTRIP_FIXTURES/ROUNDTRIP_FORMATS slices + LensMode::ALL/Format::ALL public consts + base60_cmd()/assert_roundtrip test helpers (D-14 widen target)"
provides:
  - "Length-metadata trailer emitted by every dump format (ansi/plain/html/json)"
  - "`base60 decode` auto-detects HTML / JSON / ansi-plain from first non-empty line"
  - "`--input-format={auto,ansi,plain,json,html}` override flag on the decode subcommand"
  - "NDJSON decoder (hand-rolled, zero-dep) consuming `{\"offset\":...,\"bytes\":[...]}` + `{\"type\":\"meta\",\"bytes\":N}` records"
  - "HTML decoder (hand-rolled state machine) consuming `<span class=\"d-zero|d-low|d-mid|d-high\">NN</span>` + `<!-- bytes=0x<hex> -->` shapes"
  - "Legacy-dump stderr warning (`no length metadata`) + 8-byte-aligned fallback (D-03)"
  - "140-cell roundtrip matrix (5 fixtures × 7 lens configs × 4 formats) — byte-identical for every cell"
affects: [04-02-plan-parse-run, 04-03-reader-coverage, 05-fuzz-bench, 06-perf, 07-public-api-diff]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Hand-rolled NDJSON / HTML decoders — zero-dep parity with the existing hand-rolled emitters (format.rs:31 docstring precedent)"
    - "First-line sniff + `Cursor::new(first).chain(r)` replay — avoids double-reading stdin"
    - "Streaming last-chunk buffer for trailer truncation — no full-stream allocation needed"

key-files:
  created: []
  modified:
    - "crates/base60-cli/src/cli.rs — new `InputFormat` enum; `DecodeArgs.input_format` flag"
    - "crates/base60-cli/src/decode.rs — full rewrite around dispatch; +5 helpers; module doc rewritten to note HTML coupling"
    - "crates/base60-cli/src/dump.rs — `dump_all` appends `# bytes=0x<hex>` trailer"
    - "crates/base60-cli/src/format.rs — `emit_json` + `emit_html` append length trailers"
    - "crates/base60-cli/src/lib.rs — `run_decode` threads `args.input_format` into `decode_stream`"
    - "crates/base60-cli/tests/cli.rs — 4 new tests (legacy warning, help advertising, json/html overrides)"
    - "crates/base60-cli/tests/common/mod.rs — `ROUNDTRIP_FIXTURES` → `ALL_FIXTURES` (5 entries); `ROUNDTRIP_FORMATS` → `Format::ALL`"
    - "crates/base60-cli/tests/roundtrip.rs — doc rewrite (140 cells); import flip"

key-decisions:
  - "Trailer emission is unconditional (D-02) — empty input emits `# bytes=0x0\\n` so the decoder invariant is symmetric"
  - "Legacy dumps without trailer decode with single stderr warning, exit 0 (D-03 — backwards-compatible per PROJECT.md L114)"
  - "HTML decoder hand-rolled per D-05 — ~60 LOC state machine, module-level doc pins the coupling to emit_html"
  - "`parse_run` signature left UNCHANGED — Plan 04-02 (REF-03) owns the tightening per D-09"
  - "Trailer position in HTML is just before `HTML_EPILOGUE` so it sits between `</pre>` and `</body></html>` (valid HTML5)"

patterns-established:
  - "Emitter/decoder inverse-spec contract: every dump format has a matching module-documented decoder; the docstring names the emitter whose shape it parses"
  - "`parse_trailer_hex` as a small pure function — easy to unit-test, zero side effects, catches leading whitespace + partial hex gracefully"
  - "`#` prefix on text trailer is the decoder's disambiguation handle (neither ASCII digit nor colon, so `find_digit_run` cannot misalign)"

requirements-completed: [REF-04]

# Metrics
duration: 14min
completed: 2026-04-24
---

# Phase 4 Plan 1: Length-preserving decode + JSON/HTML decode paths + 140-cell matrix widen Summary

**REF-04 landed as 3 atomic commits: every dump format carries a length trailer, `base60 decode` auto-detects HTML/JSON/ansi-plain with a `--input-format` override, and the 140-cell roundtrip matrix flips from 28 → 140 — byte-identical across all 5 fixtures × 7 lens configs × 4 formats.**

## Performance

- **Duration:** ~14 min
- **Started:** 2026-04-24T15:36:00Z (approx; worktree base was set)
- **Completed:** 2026-04-24T15:49:52Z
- **Tasks:** 3
- **Files modified:** 8
- **New tests:** 18 (8 inline decode unit tests + 5 inline dump/format trailer tests + 4 integration cli tests + 1 trailer_hex parser test; the 140-cell matrix stays in its single `#[test]` entry)

## Accomplishments

- Every dump format now emits a length trailer: ansi/plain `# bytes=0x<hex>\n`, HTML `<!-- bytes=0x<hex> -->\n`, JSON `{"type":"meta","bytes":<decimal>}\n`. Always emitted (including for empty input).
- `base60 decode` dispatches on format via auto-detection (first non-empty line) or `--input-format={auto,ansi,plain,json,html}` override. Default is `auto`.
- NDJSON decoder handles `{"offset":...,"bytes":[...]}` chunk lines and `{"type":"meta","bytes":N}` pin — hand-rolled, no `serde_json` dep.
- HTML decoder walks `<span class="d-zero|d-low|d-mid|d-high">NN</span>` + `<span class="sep">:</span>` pairs, strips everything outside `<body>...</body>`, consumes `<!-- bytes=0x<hex> -->` — ~60-line state machine, hand-rolled.
- Legacy dumps (pre-trailer) decode with a single stderr warning containing `no length metadata` and exit 0. Fallback alignment is 8 bytes.
- Roundtrip matrix expanded from 28 → 140 cells; `base60 FILE | base60 decode` is byte-identical for `hello_world` 14 B, `minimal_png` 45 B, `minimal_zip` 22 B (all short-tail fixtures) across every format.
- `base60-core` `[dependencies]` section remains absent — zero-dep invariant preserved (Pitfall 5).

## Task Commits

1. **Task 1: Emit length-metadata trailer across all four formats + inline unit tests** — `ab4bc8b` (feat)
2. **Task 2: `InputFormat` clap enum + auto-detect + `--input-format` + JSON/HTML/legacy-warning decoders** — `37ef37b` (feat)
3. **Task 3: Widen roundtrip matrix to 140 cells + tests/cli.rs overrides + legacy-warning test** — `cf08016` (test)

## Files Created/Modified

- `crates/base60-cli/src/dump.rs` — `dump_all` appends `# bytes=0x<hex>\n` before `out.flush()`; 3 new inline tests (empty, plain, ansi); 1 updated test (chunk count).
- `crates/base60-cli/src/format.rs` — `emit_json` appends meta NDJSON line; `emit_html` inserts length comment before `HTML_EPILOGUE`; 2 new tests (meta-at-end, html-comment); 2 updated tests (empty output, one-line-per-chunk count).
- `crates/base60-cli/src/cli.rs` — new `InputFormat` clap `ValueEnum` (Auto default); `DecodeArgs.input_format` field with `#[arg(long, value_enum, default_value_t = InputFormat::Auto)]`.
- `crates/base60-cli/src/lib.rs` — `run_decode` threads `args.input_format` into `decode_stream` for both file and stdin paths.
- `crates/base60-cli/src/decode.rs` — module rewritten: new dispatch `decode_stream`, helpers `sniff`, `decode_from_text`, `decode_from_json`, `decode_from_html`, `parse_trailer_hex`; internal enum `SniffedFormat`; module-level doc explains HTML coupling (D-05) and length-metadata contract. 8 new inline unit tests; existing 9 tests preserved (updated to pass `InputFormat::Auto`).
- `crates/base60-cli/tests/common/mod.rs` — `ROUNDTRIP_FIXTURES` renamed to `ALL_FIXTURES` and widened to 5 entries; `ROUNDTRIP_FORMATS` rewritten to `base60::Format::ALL`; docstrings rewritten to reflect REF-04.
- `crates/base60-cli/tests/roundtrip.rs` — doc block rewritten to describe 140-cell matrix; imports flipped.
- `crates/base60-cli/tests/cli.rs` — 4 new tests covering `--input-format` help advertising, json override, html override, legacy-no-trailer warning.

## Decisions Made

- **Exact stderr warning wording (planner's discretion per CONTEXT):** `"decode: no length metadata; assuming input was 8-byte-aligned. Last chunk may contain zero-padding. Regenerate the dump with base60 v2+ to silence this warning."` — matches plan's example text; the fixed substring `"no length metadata"` is what integration tests pin.
- **Trailer detection / write logic streaming:** buffered only the LAST 8-byte chunk (not the entire stream), so truncation uses O(1) extra memory regardless of input size.
- **Sniff retry cap:** 16 blank-line retries before giving up and treating the stream as ansi/plain — bounds latency when stdin is slow.
- **`parse_run` unchanged:** left `fn parse_run(run: &str, ...)` as-is. The HTML decoder synthesises an 11-pair ASCII string and passes through `std::str::from_utf8(&run).expect("ascii by construction")`. Plan 04-02 (REF-03) owns the signature tightening per D-09.
- **`InputFormat` stays `pub(crate)`:** narrow-surface rule (Phase 3 D-07). Integration tests exercise it via the spawned binary's `--input-format` flag — no library re-export.

## Deviations from Plan

**None material** — plan executed exactly as written, with two minor planner-discretion choices both within the latitude the plan explicitly grants:

### Planner-discretion choices (pre-authorised)

1. **HTML state-machine implementation.** Exact shape of the ~60-line parser left to the planner per CONTEXT Claude's Discretion. Implementation: scan for `<span class="..."` opens, read class up to `"`, read `>NN</span>` suffix, recognise four `d-*` classes and two-ASCII-digit contents; drop partial trailing rows.
2. **Sniff order + trailer insertion placement.** Plan flagged these as planner's option; used `<!DOCTYPE|<!doctype|<html` → HTML, then `{"offset":` → JSON, else AnsiPlain. Trailer in HTML sits between `</pre>` and `</body></html>` (just before `HTML_EPILOGUE` write).

### Minor auto-fix (Rule 1 — bug in own draft, caught at first compile)

**1. [Rule 1 — Bug] `parse_run` call site passed `&[u8; RUN_LEN]` but the function still expects `&str`.**
- **Found during:** Task 2 (first `cargo build`).
- **Issue:** My first draft anticipated Plan 04-02's tighter signature and passed `run.as_bytes().try_into().expect(...)`, but that tightening is owned by the next plan.
- **Fix:** Pass `run: &str` at both new call sites (text + html decoders). HTML decoder synthesises its run as `[u8; RUN_LEN]` then goes through `std::str::from_utf8(&run).expect("ascii by construction")`.
- **Files modified:** `crates/base60-cli/src/decode.rs`
- **Verification:** `cargo test --workspace --all-targets --locked` green post-fix.
- **Committed in:** `37ef37b` (Task 2 commit — discovered + fixed before commit).

**2. [Rule 1 — Bug] Unused-variable warnings in final trailer-handling branch.**
- **Found during:** Task 2 (first `cargo clippy`).
- **Issue:** `written += tail` / `written += CHUNK_BYTES` at the end of the function were dead-assignments after the final flush.
- **Fix:** Dropped the dead writes (the running total was already informational); kept the branches structurally identical.
- **Committed in:** `37ef37b` (Task 2 commit).

**3. [Rule 1 — Bug] `clippy::useless_let_if_seq` on HTML trailer-parse.**
- **Found during:** Task 2 (`cargo clippy -- -D warnings`).
- **Issue:** `let mut trailer = None; if let Some(idx) = ... { trailer = ...; }` tripped `useless_let_if_seq`.
- **Fix:** Rewrote as `let trailer = slice.find(...).and_then(|idx| { ... usize::from_str_radix(...).ok() })`.
- **Committed in:** `37ef37b` (Task 2 commit).

**4. [Rule 1 — fmt] `cargo fmt --all` applied once after Task 2.**
- **Found during:** Task 2 `cargo fmt --all --check`.
- **Issue:** Cosmetic wrap drift around `body_start = raw.find("<body>").map_or(...)`.
- **Fix:** `cargo fmt --all`; file content now matches rustfmt expectations.
- **Committed in:** `37ef37b` (Task 2 commit).

None of the above altered plan semantics; they were draft-stage bugs caught inside the same task before commit.

---

**Total deviations:** 4 internal auto-fixes (all in Task 2, pre-commit) + 2 pre-authorised planner-discretion choices
**Impact on plan:** No scope creep; all behaviour matches the plan's `<behavior>` blocks + acceptance criteria.

## Issues Encountered

- None. The biggest consideration was ensuring Task 1 (trailer emission) remained compatible with the still-unchanged decoder from Phase 3 so the commit was independently green — verified by the fact that the pre-widen `roundtrip_matrix_byte_identical` test stayed green between Task 1 and Task 2 commits (trailer lines are skipped by `find_digit_run` because `#` is neither ASCII digit nor colon).

## Verification Evidence

Command outputs captured at plan completion:

- `cargo test --workspace --all-targets --locked` → all green (137 lib / 13 cli / 4 fixtures / 1 roundtrip (= 140 cells) / 41 core / 2 xtask gates).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → clean.
- `cargo fmt --all --check` → clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` → clean.
- `grep -rn 'ROUNDTRIP_FIXTURES' crates/base60-cli/tests/` → zero hits.
- `grep -n 'pub const ALL_FIXTURES' crates/base60-cli/tests/common/mod.rs` → 1 hit.
- `grep -n '^\[dependencies\]' crates/base60-core/Cargo.toml` → zero hits (zero-dep invariant preserved).
- `cargo run -p base60 --locked -- decode --help | grep -c -- '--input-format'` → `1`.
- Shell roundtrip on 14-byte input (`printf 'Hello, world!\n'`) across `--format=plain`, `--format=json`, `--format=html` piped into `base60 decode` → byte-identical output confirmed via `od -c` (file ends `d ! \n`, 14 bytes).

## Threat Flags

None — threat model `<threat_model>` items T-04-01-01 … T-04-01-05 all mapped to code mitigations that were implemented:
- T-04-01-01: `#` prefix on trailer line + `parse_trailer_hex` precedence in `decode_from_text` (verified by inline test).
- T-04-01-02: `u8::from_str` explicit with `io::Error::new(InvalidData, ...)` on bad byte tokens (no silent wraparound).
- T-04-01-03: `read_to_string` acceptance noted in module doc; bounded by CLI trust model.
- T-04-01-04: stderr warning contains only fixed text; no env or user-secret leak path.
- T-04-01-05: legacy-dump warning + 8-byte-aligned fallback pinned by integration test `decode_legacy_no_trailer_warns_and_continues`.

## Self-Check: PASSED

Files / commits verified:

- `crates/base60-cli/src/dump.rs` — FOUND (contains `# bytes=0x`)
- `crates/base60-cli/src/format.rs` — FOUND (contains `"type":"meta"` and `<!-- bytes=0x`)
- `crates/base60-cli/src/decode.rs` — FOUND (contains `decode_from_json`, `decode_from_html`, `decode_from_text`, `sniff`, `no length metadata`)
- `crates/base60-cli/src/cli.rs` — FOUND (contains `enum InputFormat`, `pub(crate) input_format: InputFormat`)
- `crates/base60-cli/src/lib.rs` — FOUND (contains `args.input_format`)
- `crates/base60-cli/tests/common/mod.rs` — FOUND (contains `pub const ALL_FIXTURES`, `base60::Format::ALL`)
- `crates/base60-cli/tests/roundtrip.rs` — FOUND (contains `ALL_FIXTURES`, 140-cell doc)
- `crates/base60-cli/tests/cli.rs` — FOUND (contains `decode_legacy_no_trailer_warns_and_continues`, `decode_input_format_override_forces_json`, `decode_input_format_override_forces_html`, `decode_input_format_flag_is_advertised_in_help`)
- Commit `ab4bc8b` — FOUND
- Commit `37ef37b` — FOUND
- Commit `cf08016` — FOUND

## Next Phase Readiness

- **Plan 04-02 (REF-03) unblocked:** the 140-cell matrix is in place as the Pitfall 8 safety net; tightening `parse_run` from `&str` → `&[u8; RUN_LEN]` will compile-fail at both call sites (text + html decoders) immediately — the planner's migration plan is now directly applicable.
- **Plan 04-03 / 04-04 unaffected:** disjoint file surfaces (`reader.rs`, `tui.rs`, `persist.rs`).
- **Downstream (Phase 5 fuzz / Phase 6 perf) inherit two fuzz-worthy hand-rolled parsers:** `decode_from_json` and `decode_from_html` become prime fuzz targets under TEST-02. Worth noting in the next phase's CONTEXT.
- **Zero blockers.**

---
*Phase: 04-tighten-parse-run-close-coverage-gaps*
*Completed: 2026-04-24*
