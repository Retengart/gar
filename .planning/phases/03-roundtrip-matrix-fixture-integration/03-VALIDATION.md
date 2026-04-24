---
phase: 3
slug: roundtrip-matrix-fixture-integration
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-24
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` + `assert_cmd = "2"` + `predicates = "3"` |
| **Config file** | `crates/base60-cli/Cargo.toml` `[dev-dependencies]` |
| **Quick run command** | `cargo test --workspace --all-targets --locked` |
| **Full suite command** | `cargo test --workspace --all-targets --locked && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check` |
| **Estimated runtime** | ~90 s (matrix adds ~30 s to existing ~60 s suite) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace --all-targets --locked`
- **After every plan wave:** Run the full suite (tests + clippy + fmt)
- **Before `/gsd-verify-work`:** Full suite + `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS: -D warnings`
- **Max feedback latency:** 90 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 3-01-01 | 01 | 1 | — (refactor) | — | N/A | compile | `cargo check --workspace --all-targets --locked` | ❌ W0 | ⬜ pending |
| 3-01-02 | 01 | 1 | — (refactor) | — | N/A | unit | `cargo test -p base60 --lib` | ❌ W0 | ⬜ pending |
| 3-01-03 | 01 | 1 | — (dispatch) | — | N/A | unit | `cargo test -p base60 --lib all_contains_every_format_variant` | ❌ W0 | ⬜ pending |
| 3-02-01 | 02 | 2 | TEST-01 | — | N/A | integration | `cargo test -p base60 --test roundtrip` | ❌ W0 | ⬜ pending |
| 3-02-02 | 02 | 2 | TEST-03 | — | N/A | gate | `cargo test -p xtask --test spawn_discipline` | ❌ W0 | ⬜ pending |
| 3-03-01 | 03 | 3 | TEST-03 | — | N/A | integration | `cargo test -p base60 --test fixtures` | ❌ W0 | ⬜ pending |
| 3-03-02 | 03 | 3 | TEST-03 | — | N/A | integration | `cargo test -p base60 --test cli` | ❌ W0 | ⬜ pending |
| 3-03-03 | 03 | 3 | TEST-03 | — | decoder-error-msg pin | regression | `cargo test -p base60 --test cli decode_invalid_digit_error_message` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/base60-cli/src/lib.rs` — new file hosting `pub fn run()` + `mod X;` declarations (Plan 01).
- [ ] `crates/base60-cli/tests/common/mod.rs` — `base60_cmd()` helper + fixtures + `LensConfig` enum + assertion helpers (Plan 02).
- [ ] `crates/base60-cli/tests/roundtrip.rs` — 140-cell matrix test (Plan 02).
- [ ] `crates/base60-cli/tests/fixtures.rs` — per-subcommand happy path (Plan 03).
- [ ] `crates/base60-cli/tests/cli.rs` — non-matrix edges, BrokenPipe, color, --skip/--length, decoder error pin (Plan 03).
- [ ] `crates/xtask/tests/spawn_discipline.rs` — spawn-discipline gate (Plan 02).
- [ ] `assert_cmd = "2"` + `predicates = "3"` added to `crates/base60-cli/Cargo.toml` `[dev-dependencies]` (Plan 02).

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| CI matrix green on Ubuntu/macOS/Windows × rustc 1.95/stable/beta | ROADMAP SC1 | Cross-platform runtime, not locally reproducible | After merge, watch GitHub Actions run; all 9 cells must pass `cargo test --workspace --all-targets --locked`. |
| Per-cell walltime budget (< 200 ms Ubuntu / < 500 ms Windows) | D-21 | Timing depends on runner hardware; only actionable after CI run | Inspect `cargo test -- --nocapture` debug eprintln; flag for Phase 5 if Windows aggregate > 60 s. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
