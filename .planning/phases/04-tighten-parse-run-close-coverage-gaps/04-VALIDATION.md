---
phase: 4
slug: tighten-parse-run-close-coverage-gaps
status: populated
nyquist_compliant: true
wave_0_complete: false
created: 2026-04-24
populated: 2026-04-24
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` / `#[cfg(test)]` (workspace) + `assert_cmd 2` + `predicates 3` + `serial_test 3` + `tempfile 3` (NEW in Plan 04-03) + `ratatui::backend::TestBackend` (already present via `ratatui = "0.30.0"`) |
| **Config file** | `Cargo.toml` (workspace root) + per-crate `Cargo.toml` |
| **Quick run command** | `cargo test -p base60 --locked` |
| **Full suite command** | `cargo test --workspace --all-targets --locked` |
| **Estimated runtime** | ~45 seconds (cli crate only ~15s post-widen; full matrix ~30s after REF-04 lands) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p base60 --locked`
- **After every plan wave:** Run `cargo test --workspace --all-targets --locked`
- **Before `/gsd-verify-work`:** Full suite + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo fmt --all --check` + `cargo doc --workspace --no-deps --locked` (with `RUSTDOCFLAGS=-D warnings`) must all be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 04-01-T1 | 01 | 1 | REF-04 | T-04-01-01 | `#`-prefixed trailer cannot be misinterpreted as a digit run | unit | `cargo test -p base60 --lib --locked dump::tests::dump_all_emits_length_trailer dump::tests::dump_all_emits_trailer_for_empty_input format::tests::json_emits_meta_line_at_end format::tests::html_document_includes_length_comment` | ✅ source files exist; new tests land in existing `#[cfg(test)] mod tests` | ⬜ pending |
| 04-01-T2 | 01 | 1 | REF-04 | T-04-01-02, T-04-01-04 | JSON byte parser rejects non-numeric tokens; stderr warning stable | unit + inline | `cargo test -p base60 --locked decode::tests` | ✅ decode.rs exists; tests land in existing `mod tests` | ⬜ pending |
| 04-01-T3 | 01 | 1 | REF-04 | T-04-01-05 | 140-cell roundtrip matrix + legacy-no-trailer stderr warning pinned | integration | `cargo test -p base60 --test roundtrip --test cli --locked` | ✅ roundtrip.rs + cli.rs exist (edited); new `decode_legacy_no_trailer_warns_and_continues` / `decode_input_format_*` tests appended | ⬜ pending |
| 04-02-T1 | 02 | 2 | REF-03 | T-04-02-01, T-04-02-03 | `parse_run(&[u8; RUN_LEN], usize)` compile-time length invariant; digit-check internal; 12-pair overextension still rejected | compile-time + unit | `cargo test -p base60 --locked decode::tests && cargo check -p base60 --locked` | ✅ decode.rs exists; signature change + caller migration | ⬜ pending |
| 04-02-T2 | 02 | 2 | REF-03 | T-04-02-02 | Full-message stderr pin + 3 new position/tolerance tests lock the error contract | integration | `cargo test -p base60 --test cli --locked decoder_invalid_digit decoder_ignores_non_digit_run_lines` | ✅ cli.rs exists; tests appended | ⬜ pending |
| 04-03-T1 | 03 | 3 | TEST-05 (reader) | — | `tempfile = "3"` dev-dep added; `base60-core` stays zero-dep | build | `cargo build --workspace --all-targets --locked` | ✅ Cargo.toml exists | ⬜ pending |
| 04-03-T2 | 03 | 3 | TEST-05 (reader) | T-04-03-01 | mmap + stdin + file-open-error paths exercised black-box via `base60_cmd()`; spawn-discipline preserved | integration | `cargo test -p base60 --test reader --locked` | ❌ Wave 0 must create `crates/base60-cli/tests/reader.rs` first | ⬜ pending |
| 04-04-T1 | 04 | 3 | TEST-05 (tui) | T-04-04-03 | `run_with_terminal<B, F>` seam extracted; production path unchanged | compile + lib test | `cargo test -p base60 --lib --locked tui && cargo check -p base60 --locked` | ✅ tui.rs + lib.rs exist | ⬜ pending |
| 04-04-T2 | 04 | 3 | TEST-05 (tui) | T-04-04-02 | `drive_tui_to_quit_with_fixture` shared helper uses correct `j j j j j m a q` sequence | compile | `cargo check -p base60 --tests --locked` | ✅ common/mod.rs exists (extended) | ⬜ pending |
| 04-04-T3 | 04 | 3 | TEST-05 (tui) | T-04-04-01, T-04-04-04 | TestBackend drive asserts `cursor=40` + `bookmarks=a:40` in saved state file | integration (serial env) | `cargo test -p base60 --test tui --locked` | ❌ Wave 0 must create `crates/base60-cli/tests/tui.rs` first | ⬜ pending |
| 04-04-T4 | 04 | 3 | TEST-05 (persist) | T-04-04-01 | Three `#[serial(env)]` tests pin XDG → HOME → None ladder; `--test-threads=8` safe | integration (serial env) | `cargo test -p base60 --test persist --locked && cargo test --workspace --all-targets --locked -- --test-threads=8` | ❌ Wave 0 must create `crates/base60-cli/tests/persist.rs` first | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/base60-cli/Cargo.toml` — add `tempfile = "3"` to `[dev-dependencies]` (Plan 04-03 Task 1; reused by Plan 04-04)
- [ ] `crates/base60-cli/tests/reader.rs` — NEW file, 3 tests (mmap + stdin + file-open-error) (Plan 04-03 Task 2)
- [ ] `crates/base60-cli/tests/tui.rs` — NEW file, 1 test (TestBackend quit-with-save) (Plan 04-04 Task 3, `#[serial(env)]`)
- [ ] `crates/base60-cli/tests/persist.rs` — NEW file, 3 tests (XDG → HOME → None ladder) (Plan 04-04 Task 4, `#[serial(env)]`)
- [ ] `crates/base60-cli/tests/common/mod.rs` — add `drive_tui_to_quit_with_fixture` helper (Plan 04-04 Task 2) + flip `ROUNDTRIP_FIXTURES → ALL_FIXTURES` + `ROUNDTRIP_FORMATS → base60::Format::ALL` (Plan 04-01 Task 3)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Production TUI still launches on a real terminal after seam refactor | REF not directly tied; behaviour-preserving contract | `ratatui::run` requires a real crossterm-backed terminal; automated smoke is the pre-existing v1 CI green bar + manual `cargo run -p base60 -- -i README.md` developer check after Plan 04-04 Task 1 | `cargo run -p base60 --locked -- -i README.md`; press `j` × 5, `m a`, `q`; confirm no visual glitches; reopen the file to confirm the bookmark at `a` jumps to offset 40. |

*All other phase behaviours have automated verification. The 140-cell matrix auto-exercises REF-04/REF-03 byte-identical guarantees; the TUI seam + TestBackend covers the save path; persist env-ladder is fully black-box via drive helper + tempdir filesystem inspection.*

---

## Nyquist Dimension Coverage

Cross-reference against RESEARCH §"Validation Architecture" (all 8 dimensions covered):

| # | Dimension | Covered by | Plan(s) |
|---|-----------|-----------|---------|
| 1 | Correctness (byte-identical roundtrip) | 140-cell matrix (5 × 7 × 4) | 04-01 |
| 2 | Contract / API stability | `parse_run` array-type signature + full-message stderr pin + 3 position tests | 04-02 |
| 3 | Error-path / failure modes | pair-1, pair-5, non-digit-run-lines-ignored, legacy-no-trailer warning, file-open error, JSON non-numeric byte | 04-01, 04-02, 04-03 |
| 4 | Integration (format dispatch) | auto-detect + `--input-format` override + TUI TestBackend + reader spawn-path | 04-01, 04-03, 04-04 |
| 5 | Performance / resource | (deferred to Phase 5 bench scaffolding — documented in SUMMARY) | — |
| 6 | Regression (legacy acceptance) | no-trailer fallback + stderr warning test; all pre-existing 182 tests retained | 04-01, 04-02 |
| 7 | Observability (metadata trailer + state file) | `# bytes=0x<hex>` / NDJSON meta / HTML comment tests; state-file `cursor=` / `bookmarks=a:` assertion | 04-01, 04-04 |
| 8 | Operability (env + mmap + TUI save + XDG ladder) | reader.rs + tui.rs + persist.rs integration tests | 04-03, 04-04 |

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (every task ships with `<automated>` command)
- [x] Wave 0 covers all MISSING references (tempfile dev-dep, 3 new test files, `drive_tui_to_quit_with_fixture` helper)
- [x] No watch-mode flags (all `cargo test` invocations are single-shot)
- [x] Feedback latency < 60s (cli-crate-only run ~15s; full workspace ~45s)
- [x] `nyquist_compliant: true` set in frontmatter — every task has an automated verify command

**Approval:** populated by planner on 2026-04-24 — ready for Wave 0 execution.
