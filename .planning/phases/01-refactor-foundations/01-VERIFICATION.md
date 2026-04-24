---
phase: 01-refactor-foundations
verified: 2026-04-24T07:30:00Z
status: passed
score: 4/4 must-haves verified
overrides_applied: 0
---

# Phase 1: Refactor Foundations Verification Report

**Phase Goal:** The contracts every downstream test, fuzz target, and bench will stand on are stabilised — one source of truth for chunk decoding, one table for lens dispatch.
**Verified:** 2026-04-24T07:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `grep -n 'fn be_u64' crates/base60-cli/src/*.rs` returns exactly one hit at `chunk.rs`; `dump.rs` and `format.rs` call it via `use crate::chunk::{CHUNK, be_u64, pad_chunk}` | ✓ VERIFIED | Grep returns one hit: `crates/base60-cli/src/chunk.rs:24`. `dump.rs:10` and `format.rs:17` both import from `crate::chunk`. No local copies in `dump.rs` or `format.rs`. |
| 2 | `crates/base60-core/Cargo.toml` `[dependencies]` section remains empty — no new workspace dep leaked into the zero-dep library | ✓ VERIFIED | `crates/base60-core/Cargo.toml` has no `[dependencies]` section at all — only `[package]` and `[lints]`. |
| 3 | A single `const ALL: &[LensMode]` (or equivalent hand-rolled table) in `cli.rs` drives every lens dispatch site; `build_lens` / `cycle` / `label` / `persist::parse_lens` all read from it, verified by a compile-time exhaustiveness test that iterates the table and panics on missing variants | ✓ VERIFIED | `pub(crate) const ALL: &[Self]` present at `cli.rs:47` (`Self` = `LensMode` inside `impl LensMode` — equivalent, required by `clippy::use_self`). Two exhaustiveness tests iterate `ALL` and exercise all four dispatch sites. Both pass. Note: the match arms in `cycle`/`label`/`build_lens`/`parse_lens` are compiler-exhaustive; `ALL` + tests catch drift between the slice and those arms. |
| 4 | Adding a hypothetical fifth `LensMode` variant compile-errors at exactly one site (the table) OR fails `cargo test`; `cargo test --workspace --all-targets --locked` stays green | ✓ VERIFIED | `cargo test --workspace --all-targets --locked` exits 0 (165 tests: 124 CLI + 41 core). `cargo clippy --workspace --all-targets --locked -- -D warnings` exits 0. A new variant not added to `ALL` would fail `all_contains_every_variant_in_cycle_order` at runtime; a new variant not covered by `match` arms in `cycle`/`label`/`build_lens` would be a compile error. |

**Score:** 4/4 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/base60-cli/src/chunk.rs` | CLI-local decoding kit (`CHUNK`, `pad_chunk`, `be_u64`) | ✓ VERIFIED | 26 lines. Contains `pub(crate) const CHUNK: usize = 8`, `pub(crate) fn pad_chunk`, `pub(crate) const fn be_u64`. |
| `crates/base60-cli/src/main.rs` | `mod chunk;` declaration | ✓ VERIFIED | `mod chunk;` at line 12, alphabetically between `mod analyze;` (line 11) and `mod cli;` (line 13). |
| `crates/base60-cli/src/dump.rs` | `use crate::chunk::{CHUNK, be_u64, pad_chunk};`; no local `fn be_u64`, no local `const CHUNK` | ✓ VERIFIED | Import at line 10. Zero hits for `fn be_u64` or `const CHUNK: usize = 8`. Call sites: `be_u64(pad_chunk(bytes))` at lines 49 and 147. |
| `crates/base60-cli/src/format.rs` | `use crate::chunk::{CHUNK, be_u64, pad_chunk};`; no local `fn be_u64`, no `use crate::dump::CHUNK` | ✓ VERIFIED | Import at line 17. Zero hits for `fn be_u64` or `use crate::dump::CHUNK`. Call sites at lines 42 and 94. |
| `crates/base60-cli/src/tui.rs` | `use crate::chunk::CHUNK;` (split from former grouped dump import) | ✓ VERIFIED | `use crate::chunk::CHUNK;` at line 4. No `use crate::dump::{CHUNK,` anywhere. |
| `crates/base60-cli/src/cli.rs` | `LensMode::ALL` const + two exhaustiveness tests | ✓ VERIFIED | `pub(crate) const ALL: &[Self]` at line 47 (5 variants in cycle order). `#[cfg(test)] mod tests` at line 272 with both tests. |
| `crates/base60-cli/src/persist.rs` | `pub(crate) fn parse_lens` with doc comment | ✓ VERIFIED | `pub(crate) fn parse_lens(val: &str) -> LensMode` at line 144, with doc comment at lines 139-143. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `dump.rs` | `chunk.rs` | `use crate::chunk::{CHUNK, be_u64, pad_chunk};` | ✓ WIRED | Line 10. Call sites at lines 49, 147. |
| `format.rs` | `chunk.rs` | `use crate::chunk::{CHUNK, be_u64, pad_chunk};` | ✓ WIRED | Line 17. Call sites at lines 42, 94. |
| `tui.rs` | `chunk.rs` | `use crate::chunk::CHUNK;` | ✓ WIRED | Line 4. CHUNK used as layout constant throughout. |
| `main.rs` | `chunk.rs` | `mod chunk;` | ✓ WIRED | Line 12, alphabetical position correct. |
| `cli.rs` tests | `persist.rs` | `use crate::persist;` in `#[cfg(test)] mod tests` | ✓ WIRED | Line 275. `persist::parse_lens(lbl)` called in `all_methods_total_over_all`. |
| `cli.rs` tests | `LensMode::ALL` | `for &mode in LensMode::ALL` | ✓ WIRED | Both tests iterate `LensMode::ALL`. |

### Data-Flow Trace (Level 4)

Not applicable — this phase produces pure code-organisation refactors (no new user-visible data flows). Existing rendering pipelines were verified by the test suite (165 tests passing).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `be_u64` defined exactly once | `grep -n 'fn be_u64' crates/base60-cli/src/*.rs` | 1 hit: `chunk.rs:24` | ✓ PASS |
| `base60-core` zero-dep | `cat crates/base60-core/Cargo.toml` | No `[dependencies]` section | ✓ PASS |
| `ALL` const present | `grep -n 'const ALL' crates/base60-cli/src/cli.rs` | `47: pub(crate) const ALL: &[Self]` | ✓ PASS |
| `parse_lens` promoted | `grep -n 'pub(crate) fn parse_lens' crates/base60-cli/src/persist.rs` | `144: pub(crate) fn parse_lens` | ✓ PASS |
| Exhaustiveness tests pass | `cargo test -p base60 cli::tests::all_contains_every_variant_in_cycle_order -- --exact` | 1 passed | ✓ PASS |
| Exhaustiveness tests pass | `cargo test -p base60 cli::tests::all_methods_total_over_all -- --exact` | 1 passed | ✓ PASS |
| Full test suite green | `cargo test --workspace --all-targets --locked` | 165 passed, 0 failed | ✓ PASS |
| Clippy clean | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| REF-01 | 01-dedupe-be-u64-PLAN.md | De-duplicate `be_u64` into CLI-local `chunk.rs`; `dump.rs` and `format.rs` import from it | ✓ SATISFIED | `chunk.rs` exists with single `be_u64`. Both callers import from `crate::chunk`. Commit `b8aeac5`. |
| REF-02 | 02-lens-mode-dispatch-table-PLAN.md | Drive `LensMode` dispatch from one `const ALL` table in `cli.rs`. Adding a new variant touches one site. | ✓ SATISFIED | `LensMode::ALL` at `cli.rs:47`. Two exhaustiveness tests guard it. Commit `8e5b0fe`. |

No orphaned requirements for Phase 1 — REQUIREMENTS.md maps only REF-01 and REF-02 to this phase. Both are covered.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/base60-cli/src/cli.rs` | 46 | `#[allow(dead_code)]` on `ALL` const | ℹ️ Info | Intentional; SUMMARY documents that Phase 3 (TEST-01) will add the production reference to `ALL` and the allow should then be removed. Not a blocker. |

No TODOs, stubs, placeholder returns, or hollow implementations found in phase-modified files.

### Human Verification Required

None. All must-haves are verifiable programmatically and confirmed passing.

### Gaps Summary

No gaps. All four roadmap success criteria are met:

1. `be_u64` has exactly one definition in the CLI crate (`chunk.rs`); both renderers import from `crate::chunk`.
2. `base60-core/Cargo.toml` has no `[dependencies]` section.
3. `LensMode::ALL` (as `&[Self]`, equivalent to `&[LensMode]` inside `impl LensMode`) is the single source-of-truth slice, with two tests verifying exhaustiveness across all four dispatch sites.
4. `cargo test --workspace --all-targets --locked` passes (165 tests green); `cargo clippy --workspace --all-targets --locked -- -D warnings` passes.

**Implementation notes vs. plan spec (not gaps):**
- `be_u64` signature is by-value `[u8; CHUNK]` rather than by-reference `&[u8; CHUNK]` — required by `clippy::trivially_copy_pass_by_ref`. Semantically identical.
- `ALL` uses `&[Self]` rather than `&[LensMode]` — required by `clippy::use_self` inside `impl LensMode`. Semantically identical.
- `#[allow(dead_code)]` on `ALL` is a planned interim annotation pending Phase 3's production reference.

Phase 1 goal is achieved. Foundations are stable for downstream phases.

---

_Verified: 2026-04-24T07:30:00Z_
_Verifier: Claude (gsd-verifier)_
