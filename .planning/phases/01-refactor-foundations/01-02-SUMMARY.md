---
phase: 01-refactor-foundations
plan: 02
subsystem: cli
tags:
  - refactor
  - cli
  - lens-mode
  - exhaustiveness-test

dependency_graph:
  requires:
    - phase: 01-refactor-foundations plan 01
      provides: "chunk.rs (be_u64 single source of truth) — confirmed unchanged by this plan"
  provides:
    - "LensMode::ALL — pub(crate) const slice in impl LensMode (cli.rs) listing all 5 variants in cycle order"
    - "Two exhaustiveness tests: all_contains_every_variant_in_cycle_order (D-08+D-09), all_methods_total_over_all (D-08 Test 2)"
    - "persist::parse_lens promoted to pub(crate) for cross-module test access"
  affects:
    - "Phase 3 TEST-01 — can iterate LensMode::ALL for (LensMode x Format) roundtrip matrix"

tech-stack:
  added: []
  patterns:
    - "Data-before-behaviour ordering inside impl blocks: ALL const placed before cycle() method"
    - "Exhaustiveness-test Shape B: cycle-walk assertion proves both ALL completeness and order"
    - "#[allow(dead_code)] interim annotation on pub(crate) const used only in tests until Phase 3 wires it in production code"

key-files:
  created: []
  modified:
    - crates/base60-cli/src/cli.rs
    - crates/base60-cli/src/persist.rs

key-decisions:
  - "Use Self:: instead of LensMode:: in ALL slice entries (clippy::use_self inside impl LensMode)"
  - "#[allow(dead_code)] on ALL const to suppress transient dead_code lint until Phase 3 uses it in production — prefer this over removing the const or guarding with #[cfg(test)]"
  - "Cycle-walk Shape B used as planned: single assertion proves both D-08 Test 1 (completeness) and D-09 (cycle order) — no separate shape A fallback needed"

requirements-completed:
  - REF-02

duration: 2m52s
completed: "2026-04-24"
---

# Phase 01 Plan 02: LensMode dispatch table Summary

**`LensMode::ALL` const slice added to `cli.rs` as the single source-of-truth variant list, with two exhaustiveness tests that walk every dispatch site — adding a variant without updating `ALL` or any match arm fails at compile time or test time.**

## Performance

- **Duration:** 2m52s
- **Started:** 2026-04-24T06:46:31Z
- **Completed:** 2026-04-24T06:49:23Z
- **Tasks:** 2 (Tasks 1 and 2 combined into a single atomic commit per plan spec)
- **Files modified:** 2

## Accomplishments

- `LensMode::ALL` (`pub(crate) const ALL: &[Self]`) added inside `impl LensMode` in `cli.rs`, listing all 5 variants in cycle order (None, Time, Angle, Tablet, Cuneiform), placed before `cycle()` per data-before-behaviour convention
- `persist::parse_lens` promoted from bare `fn` to `pub(crate) fn` with mandatory doc comment (`RUSTDOCFLAGS: -D warnings`)
- Two exhaustiveness tests in `#[cfg(test)] mod tests` at bottom of `cli.rs`: `all_contains_every_variant_in_cycle_order` (D-08+D-09 merged) and `all_methods_total_over_all` (exercises label, cycle, build_lens, parse_lens for every variant)
- Full gate passes: fmt, clippy `-D warnings`, 165 tests (124 CLI + 41 core)

## Task Commits

Both plan tasks landed in a single atomic commit as specified:

1. **Tasks 1+2: Promote parse_lens + add ALL const + add exhaustiveness tests** — `8e5b0fe` (refactor)

## Files Created/Modified

| File | Before | After | Change |
|------|--------|-------|--------|
| `crates/base60-cli/src/cli.rs` | 257 lines | 324 lines | +67 lines: ALL const, #[allow(dead_code)], tests module |
| `crates/base60-cli/src/persist.rs` | ~237 lines | ~243 lines | +6 lines: pub(crate) + doc comment on parse_lens |

## Gate Command Results

| Command | Exit code |
|---------|-----------|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --all-targets --locked` | 0 (165 tests: 124 CLI + 41 core) |
| `cargo test -p base60 cli::tests::all_contains_every_variant_in_cycle_order -- --exact` | 0 |
| `cargo test -p base60 cli::tests::all_methods_total_over_all -- --exact` | 0 |

## Decisions Made

- Used `Self::` instead of `LensMode::` in the `ALL` slice entries — `clippy::use_self` fires inside `impl LensMode` when the type name is repeated explicitly
- Applied `#[allow(dead_code)]` to `ALL` with a comment explaining Phase 3 will add the production reference — avoids removing/guarding the const while keeping clippy clean
- Cycle-walk Shape B used as recommended: single `for &expected in LensMode::ALL` loop asserting `walk == expected` then advancing via `cycle()`, then checking `walk == None` at end — proves completeness and order in one pass

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `clippy::use_self` fired on `LensMode::*` in `ALL` entries inside `impl LensMode`**
- **Found during:** Task 2 gate (clippy)
- **Issue:** `LensMode::None`, `LensMode::Time`, etc. inside `impl LensMode` trigger `use_self` — clippy requires `Self::` inside the impl block
- **Fix:** Changed all 5 entries in `ALL` from `LensMode::*` to `Self::*`
- **Files modified:** `crates/base60-cli/src/cli.rs`
- **Verification:** `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0
- **Committed in:** `8e5b0fe` (same atomic commit)

**2. [Rule 1 - Bug] `dead_code` lint on `ALL` const — only referenced in `#[cfg(test)]` scope**
- **Found during:** Task 2 gate (clippy, implied by `-D warnings`)
- **Issue:** `pub(crate) const ALL` is only referenced inside `#[cfg(test)] mod tests`. The non-test binary target sees it as unused, triggering `dead_code`
- **Fix:** Added `#[allow(dead_code)]` with inline comment: "Phase 3 (TEST-01) will iterate this in production code; suppress the dead-code lint in the interim"
- **Files modified:** `crates/base60-cli/src/cli.rs`
- **Verification:** `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0
- **Committed in:** `8e5b0fe` (same atomic commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 bugs — clippy correctness)
**Impact on plan:** Both fixes required for clippy `-D warnings` compliance. No semantic change to the constant or tests. No scope creep.

## Issues Encountered

- `cargo test -p base60 all_contains_every_variant_in_cycle_order -- --exact` returns 0 tests (filtered out). The `--exact` flag requires the full module path. Correct invocation: `cargo test -p base60 cli::tests::all_contains_every_variant_in_cycle_order -- --exact`. The plan's Task 2 action step omitted the module prefix — noted here for future reference.

## Known Stubs

None.

## Threat Flags

None — pure internal code reorganisation. No new trust boundaries, no new network/filesystem/auth surface. `pub(crate)` visibility on `parse_lens` remains crate-internal.

## Zero-dep Invariant

`crates/base60-core/Cargo.toml` has no `[dependencies]` section. Invariant preserved.

## Note for Phase 3

`LensMode::ALL` is ready. Phase 3 (TEST-01) can iterate it via `for &mode in LensMode::ALL { ... }` to enumerate the (LensMode × Format) roundtrip matrix without hand-listing variants. Once Phase 3 adds a production reference to `ALL`, the `#[allow(dead_code)]` annotation should be removed.

## Self-Check: PASSED

- FOUND: `crates/base60-cli/src/cli.rs`
- FOUND: `crates/base60-cli/src/persist.rs`
- FOUND: `.planning/phases/01-refactor-foundations/01-02-SUMMARY.md`
- FOUND: commit `8e5b0fe` (2 files: cli.rs, persist.rs)
- FOUND: `ALL` const with `Self` type (`pub(crate) const ALL: &[Self]`)
- FOUND: `pub(crate) fn parse_lens` in persist.rs
