---
phase: 4
slug: tighten-parse-run-close-coverage-gaps
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-24
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `#[test]` / `#[cfg(test)]` (workspace) |
| **Config file** | `Cargo.toml` (workspace root) + per-crate `Cargo.toml` |
| **Quick run command** | `cargo test -p base60-cli --locked` |
| **Full suite command** | `cargo test --workspace --all-targets --locked` |
| **Estimated runtime** | ~45 seconds (cli crate only ~15s) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p base60-cli --locked`
- **After every plan wave:** Run `cargo test --workspace --all-targets --locked`
- **Before `/gsd-verify-work`:** Full suite + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo fmt --all --check` + `cargo doc --workspace --no-deps --locked` (with `RUSTDOCFLAGS=-D warnings`) must all be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

> Populated by `gsd-planner` once plan tasks are generated. Each task in PLAN.md
> maps to a row below. The plan-checker verifies coverage against this table.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 04-01-XX | 01 | 1 | REF-04 | — | N/A | unit + integration | `cargo test -p base60-cli --locked` | ❌ W0 | ⬜ pending |
| 04-02-XX | 02 | 2 | REF-03 | — | N/A | unit + cli integration | `cargo test -p base60-cli --locked` | ❌ W0 | ⬜ pending |
| 04-03-XX | 03 | 3 | TEST-05 (reader) | — | N/A | integration | `cargo test -p base60-cli --test reader --locked` | ❌ W0 | ⬜ pending |
| 04-04-XX | 04 | 3 | TEST-05 (tui/persist) | — | N/A | integration | `cargo test -p base60-cli --test tui --test persist --locked` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/base60-cli/Cargo.toml` — add `tempfile = "3"` to `[dev-dependencies]` (D-16; first consumed by Plan 04-03, reused by Plan 04-04)
- [ ] `crates/base60-cli/tests/reader.rs` — NEW file, mmap + stdin + file-open-error coverage (Plan 04-03)
- [ ] `crates/base60-cli/tests/tui.rs` — NEW file, `TestBackend` + `tempfile::tempdir()` + `XDG_STATE_HOME` redirect (Plan 04-04, `#[serial(env)]`)
- [ ] `crates/base60-cli/tests/persist.rs` — NEW file, `state_base_dir` XDG→HOME fallback ladder (Plan 04-04, `#[serial(env)]`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| *(none)* | — | All phase behaviours are automatable via cargo test + `base60_cmd()` spawn helper | — |

*All phase behaviours have automated verification. The roundtrip matrix widens from 28 → 140 cells and auto-exercises REF-04/REF-03 byte-identical guarantees.*

---

## Nyquist Dimension Coverage

Cross-reference against RESEARCH §"Validation Architecture" (all 8 dimensions covered):

| # | Dimension | Covered by | Plan(s) |
|---|-----------|-----------|---------|
| 1 | Correctness (byte-identical roundtrip) | 140-cell matrix + existing fixtures | 04-01 |
| 2 | Contract / API stability | parse_run signature test + error-message pin | 04-02 |
| 3 | Error-path / failure modes | 2-3 new error-position tests + malformed-HTML tolerance | 04-01, 04-02 |
| 4 | Integration (format dispatch) | auto-detect + `--input-format` flag tests | 04-01 |
| 5 | Performance / resource | (deferred to Phase 5 bench scaffolding) | — |
| 6 | Regression (legacy acceptance) | no-trailer fallback + stderr warning test | 04-01 |
| 7 | Observability (metadata trailer) | `# bytes=0x<hex>` / `<!-- bytes -->` / NDJSON meta tests | 04-01 |
| 8 | Operability (env + mmap + TUI save) | reader.rs + tui.rs + persist.rs integration tests | 04-03, 04-04 |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (tempfile dev-dep, 3 new test files)
- [ ] No watch-mode flags (all `cargo test` invocations are single-shot)
- [ ] Feedback latency < 60s (cli-crate-only run is ~15s)
- [ ] `nyquist_compliant: true` set in frontmatter after planner populates per-task map

**Approval:** pending
