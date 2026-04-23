---
phase: 01-refactor-foundations
plan: 02
type: execute
wave: 2
depends_on:
  - "01"
files_modified:
  - crates/base60-cli/src/cli.rs
  - crates/base60-cli/src/persist.rs
autonomous: true
requirements:
  - REF-02
tags:
  - refactor
  - cli
  - lens-mode
  - exhaustiveness-test

must_haves:
  truths:
    - "`LensMode::ALL` is a `pub(crate) const &[LensMode]` slice in `impl LensMode` at `cli.rs`, listing every variant in cycle order (None, Time, Angle, Tablet, Cuneiform)."
    - "An exhaustiveness test in `cli.rs::tests` (`all_contains_every_variant_in_cycle_order`) walks `cycle()` across `LensMode::ALL` and asserts that the slice both CONTAINS every variant (D-08 Test 1) AND is in the exact cycle order (D-09)."
    - "A second test (`all_methods_total_over_all`) iterates `LensMode::ALL`, calls `cycle`, `label`, `build_lens`, and `persist::parse_lens`, and round-trips every variant through its label."
    - "Adding a hypothetical fifth or sixth `LensMode` variant without updating `ALL` either fails to compile (match arms in `cycle`/`label`/`build_lens`/`parse_lens`) OR fails the exhaustiveness test at `cargo test` (D-08)."
    - "`persist::parse_lens` is `pub(crate) fn parse_lens(..)` — promoted from bare `fn` so the cli-tests module can reach it."
    - "`cargo test --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings` both pass after this plan's single commit."
  artifacts:
    - path: crates/base60-cli/src/cli.rs
      provides: "`LensMode::ALL` associated const; `#[cfg(test)] mod tests` with exhaustiveness tests (`all_contains_every_variant_in_cycle_order`, `all_methods_total_over_all`)"
      contains: "pub(crate) const ALL: &[LensMode]"
    - path: crates/base60-cli/src/persist.rs
      provides: "`pub(crate)` visibility on `parse_lens` so cross-module tests can call it"
      contains: "pub(crate) fn parse_lens"
  key_links:
    - from: crates/base60-cli/src/cli.rs
      to: crates/base60-cli/src/persist.rs
      via: "`use crate::persist;` inside the `#[cfg(test)] mod tests` block"
      pattern: "use crate::persist"
    - from: crates/base60-cli/src/cli.rs (tests)
      to: "LensMode::ALL"
      via: "iteration: `for &mode in LensMode::ALL { ... }`"
      pattern: "for &\\w+ in LensMode::ALL"
---

<objective>
Add a single source-of-truth dispatch list for `LensMode` — `pub(crate) const ALL: &[LensMode]` — as an associated const on `impl LensMode` in `cli.rs`. Existing `match` arms in `cycle()`, `label()`, `build_lens()`, and `persist::parse_lens()` stay as-is (the compiler's exhaustiveness checker already covers those). What's new: a pair of tests that iterate `LensMode::ALL` to guarantee the slice itself stays in sync with the variants AND exercise every dispatch site so a new variant that's missed at any one call site fails CI.

Test-naming note on D-08: CONTEXT D-08 specifies "Test 1 — `all_contains_every_variant`", and D-09 separately specifies that `ALL` must match cycle order. This plan merges both intents into a single stronger test named `all_contains_every_variant_in_cycle_order` — one `cycle()`-walk assertion proves both "every variant present" (D-08 Test 1) and "order matches cycle" (D-09) without duplication. The name encodes both intents so the file remains D-08-traceable by grep.

Purpose: Kills REF-02 tech-debt item (CONCERNS.md §"Tech Debt" row 2). Enables Phase 3's TEST-01 roundtrip matrix to enumerate `(LensMode × Format)` cells by iterating `LensMode::ALL` rather than hand-listing variants.

Output: Edits to `cli.rs` (new const + new tests module) and a 1-character visibility bump on `persist::parse_lens`. Zero behaviour change. One atomic commit.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/REQUIREMENTS.md
@.planning/phases/01-refactor-foundations/01-CONTEXT.md
@.planning/phases/01-refactor-foundations/01-PATTERNS.md
@.planning/codebase/CONVENTIONS.md
@.planning/codebase/CONCERNS.md

# Source files touched / used as analogs
@crates/base60-cli/src/cli.rs
@crates/base60-cli/src/persist.rs
@crates/base60-cli/src/color.rs

<interfaces>
<!-- Current `impl LensMode` block (cli.rs:40-66) — the new `const ALL` slots in BEFORE `cycle`. -->

From `crates/base60-cli/src/cli.rs:23-38` (CURRENT):
```rust
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum LensMode {
    #[default]
    None,
    Time,
    Angle,
    Tablet,
    Cuneiform,
}
```

From `crates/base60-cli/src/cli.rs:40-66` (CURRENT — block to extend):
```rust
impl LensMode {
    #[must_use]
    pub(crate) const fn cycle(self) -> Self { ... }

    #[must_use]
    pub(crate) const fn label(self) -> &'static str { ... }
}
```

From `crates/base60-cli/src/cli.rs:75-89` (CURRENT — `build_lens` free fn, unchanged in this plan):
```rust
pub(crate) fn build_lens(mode: LensMode, scale: TimeScale, purist: bool) -> Option<Box<dyn Lens>> { ... }
```

From `crates/base60-cli/src/persist.rs:139-147` (CURRENT — visibility bump needed):
```rust
fn parse_lens(val: &str) -> LensMode {
    match val {
        "time" => LensMode::Time,
        "angle" => LensMode::Angle,
        "tablet" => LensMode::Tablet,
        "cuneiform" => LensMode::Cuneiform,
        _ => LensMode::None,
    }
}
```

Note: `persist::parse_lens` returns `LensMode::None` for unknown inputs (including `"—"`, which is what `LensMode::None.label()` returns). The exhaustiveness test must handle this asymmetry — `mode.label()` → `parse_lens(label)` round-trips for every non-None variant; for `None` the round-trip also yields `None` (via the `_ =>` fallback) but by a different code path.

Target addition to `impl LensMode` (per D-06/D-09, placed BEFORE `cycle` per PATTERNS.md line 179 — "data before behaviour"):
```rust
/// Every variant in cycle order. Tests iterate this slice to prove
/// `cycle`, `label`, `build_lens`, and `persist::parse_lens` stay
/// exhaustive whenever a new variant is added.
pub(crate) const ALL: &[LensMode] = &[
    LensMode::None,
    LensMode::Time,
    LensMode::Angle,
    LensMode::Tablet,
    LensMode::Cuneiform,
];
```

No `#[must_use]` (doesn't apply to `const` declarations). Doc comment mandatory (`RUSTDOCFLAGS: -D warnings` per CONVENTIONS.md 208-209).
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Promote `persist::parse_lens` to `pub(crate)` and add `LensMode::ALL` to `cli.rs`</name>
  <files>crates/base60-cli/src/persist.rs, crates/base60-cli/src/cli.rs</files>
  <read_first>
    - crates/base60-cli/src/cli.rs (lines 1-100 — `LensMode` enum, `impl LensMode`, `build_lens`)
    - crates/base60-cli/src/persist.rs (lines 130-160 — `parse_lens` fn to promote)
    - crates/base60-cli/src/color.rs (role-match for "data before behaviour" ordering inside impl blocks)
    - .planning/phases/01-refactor-foundations/01-PATTERNS.md §"`crates/base60-cli/src/cli.rs`" and §"Shared Patterns"
    - .planning/phases/01-refactor-foundations/01-CONTEXT.md §"LensMode dispatch table (REF-02)" (D-06, D-07, D-09)
  </read_first>
  <behavior>
    - `LensMode::ALL` exists as `pub(crate) const ALL: &[LensMode]` inside `impl LensMode` at `cli.rs`, placed before the `cycle` method.
    - `ALL` contains `[LensMode::None, LensMode::Time, LensMode::Angle, LensMode::Tablet, LensMode::Cuneiform]` in exactly this order (matches `cycle()` walk order per D-09).
    - `persist::parse_lens` is `pub(crate) fn parse_lens(..)` — accessible from `cli.rs::tests`.
    - `cli.rs` still compiles; `persist.rs` still compiles; `cargo check --workspace` passes.
    - No behaviour change yet — tests come in Task 2.
  </behavior>
  <action>
    1. **`crates/base60-cli/src/persist.rs`:** change line 139 from
       ```rust
       fn parse_lens(val: &str) -> LensMode {
       ```
       to
       ```rust
       pub(crate) fn parse_lens(val: &str) -> LensMode {
       ```
       Also add a doc comment immediately above (mandatory per CONVENTIONS.md 208 — `RUSTDOCFLAGS: -D warnings` fires on missing docs for `pub(crate)` items):
       ```rust
       /// Parse a lens-mode label from persisted state back into a [`LensMode`].
       ///
       /// Unknown labels (including `LensMode::None`'s `"—"` display label)
       /// fall back to [`LensMode::None`], so state files from older binaries
       /// never break the TUI.
       pub(crate) fn parse_lens(val: &str) -> LensMode {
       ```
       Leave the match body unchanged.

    2. **`crates/base60-cli/src/cli.rs`:** inside `impl LensMode` (currently at lines 40-66), insert the `ALL` associated const BEFORE the existing `cycle` method. The new block opening to look like:
       ```rust
       impl LensMode {
           /// Every variant in cycle order. Tests iterate this slice to prove
           /// `cycle`, `label`, `build_lens`, and `persist::parse_lens` stay
           /// exhaustive whenever a new variant is added.
           pub(crate) const ALL: &[LensMode] = &[
               LensMode::None,
               LensMode::Time,
               LensMode::Angle,
               LensMode::Tablet,
               LensMode::Cuneiform,
           ];

           /// Advance through the lens cycle used by the interactive viewer's
           /// `L` key. Wraps back to [`LensMode::None`] from [`LensMode::Cuneiform`].
           #[must_use]
           pub(crate) const fn cycle(self) -> Self {
               ...
           }

           ...
       }
       ```
       Preserve the existing `cycle` and `label` methods exactly as-is. Do NOT add `#[must_use]` to the `const` — it doesn't apply to associated constants and clippy will not flag it.

    3. Do NOT run the test gate yet — the tests module arrives in Task 2. But DO verify the tree still compiles:
       ```
       cargo check --workspace --all-targets --locked
       ```
       Exit code must be 0. If clippy is re-run here, it might fire `dead_code` on `LensMode::ALL` because nothing references it yet — that's transient; Task 2 resolves it in the same commit (tests will iterate `ALL`).
  </action>
  <verify>
    <automated>grep -qE '^pub\(crate\) fn parse_lens\(val: &str\) -> LensMode' crates/base60-cli/src/persist.rs && grep -qE '^\s*pub\(crate\) const ALL: &\[LensMode\]' crates/base60-cli/src/cli.rs && cargo check --workspace --all-targets --locked</automated>
  </verify>
  <acceptance_criteria>
    - `grep -cE '^pub\(crate\) fn parse_lens\(val: &str\) -> LensMode \{' crates/base60-cli/src/persist.rs` returns `1`.
    - `grep -cE '^fn parse_lens\(' crates/base60-cli/src/persist.rs` returns `0` (no bare-`fn` definition remains; inner `tests::parse_lens_falls_back_to_none_for_unknown` is a `#[test] fn`, not a `fn parse_lens(`, so no false match).
    - `grep -cE '^\s*pub\(crate\) const ALL: &\[LensMode\] = &\[' crates/base60-cli/src/cli.rs` returns `1`.
    - The line number of `pub(crate) const ALL:` in `cli.rs` is less than the line number of `pub(crate) const fn cycle`.
    - `grep -c 'LensMode::None,' crates/base60-cli/src/cli.rs` is at least `1` (the first entry of `ALL`).
    - The five `LensMode::{None,Time,Angle,Tablet,Cuneiform}` entries appear in the exact order listed in `ALL`. Verified by: `awk '/pub\(crate\) const ALL: &\[LensMode\]/,/\];/' crates/base60-cli/src/cli.rs` emits lines in order `LensMode::None,`, `LensMode::Time,`, `LensMode::Angle,`, `LensMode::Tablet,`, `LensMode::Cuneiform,`.
    - `cargo check --workspace --all-targets --locked` exit code 0.
  </acceptance_criteria>
  <done>
    `LensMode::ALL` exists and the visibility bump on `persist::parse_lens` is in place. Compilation still succeeds. No tests yet — Task 2 adds them as part of the same commit.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Add exhaustiveness tests and commit REF-02</name>
  <files>crates/base60-cli/src/cli.rs</files>
  <read_first>
    - crates/base60-cli/src/cli.rs (AFTER Task 1 — new `LensMode::ALL` const plus the existing `impl LensMode` + `build_lens` + enum definitions)
    - crates/base60-cli/src/persist.rs (lines 163-237 — test-module idiom: bottom of file, `use super::*;`, panic-on-failure, helper fns inside module)
    - crates/base60-cli/src/search.rs (lines 121-204 — second idiom reference for bottom-of-file `#[cfg(test)] mod tests`)
    - .planning/phases/01-refactor-foundations/01-CONTEXT.md §"LensMode dispatch table (REF-02)" (D-08)
    - .planning/phases/01-refactor-foundations/01-PATTERNS.md §"Exhaustiveness test (NEW — test, transform)"
  </read_first>
  <behavior>
    - Test `all_contains_every_variant_in_cycle_order` (Shape B from PATTERNS.md — strictly stronger than Shape A, and the merged form of D-08 Test 1 + D-09): walking `cycle()` from `LensMode::None` produces the exact sequence in `LensMode::ALL`, and after `ALL.len()` steps we arrive back at `LensMode::None`. This proves (a) `ALL` contains every variant (D-08 Test 1 intent), (b) `ALL` is in cycle order (D-09 intent), (c) `cycle()` forms a closed ring covering every variant.
    - Test `all_methods_total_over_all`: for every `mode` in `LensMode::ALL`, `mode.label()` is non-empty, `mode.cycle()` returns something in `ALL`, `build_lens(mode, TimeScale::default(), false)` does not panic, and `persist::parse_lens(mode.label())` returns the same `mode` for every non-`None` variant (for `None`, returns `None` via the unknown-label fallback).
    - Tests are inline `#[cfg(test)] mod tests` at the bottom of `cli.rs`. Blank line above, `use super::*;` first line inside. Helper imports (`use crate::persist;`) immediately after.
    - Tests panic on failure (no `Result` return).
    - After Task 2, `cargo test --workspace --all-targets --locked` passes AND `cargo clippy --workspace --all-targets --locked -- -D warnings` passes AND `cargo fmt --all --check` passes.
    - A single atomic commit finalises REF-02.
  </behavior>
  <action>
    1. Append to the end of `crates/base60-cli/src/cli.rs` (blank line before, EOF after):
       ```rust

       #[cfg(test)]
       mod tests {
           use super::*;
           use crate::persist;

           #[test]
           fn all_contains_every_variant_in_cycle_order() {
               // Walking `cycle()` from `None` for `ALL.len()` steps must
               // yield the same sequence listed in `ALL`, then loop back to
               // `None`. This catches (a) a missing variant in `ALL`
               // (D-08 Test 1 intent), (b) a misordered `ALL` (D-09 intent),
               // and (c) a cycle that skips or revisits a variant — all in
               // one assertion.
               let mut walk = LensMode::None;
               for &expected in LensMode::ALL {
                   assert_eq!(walk, expected);
                   walk = walk.cycle();
               }
               assert_eq!(walk, LensMode::None);
           }

           #[test]
           fn all_methods_total_over_all() {
               for &mode in LensMode::ALL {
                   // Every variant has a non-empty label.
                   let lbl = mode.label();
                   assert!(!lbl.is_empty(), "label empty for {mode:?}");

                   // `cycle()` maps into `ALL` (no stray variant synthesised).
                   let next = mode.cycle();
                   assert!(
                       LensMode::ALL.contains(&next),
                       "cycle({mode:?}) = {next:?} is not in LensMode::ALL",
                   );

                   // `build_lens` dispatches without panicking. We do not
                   // inspect the returned trait object — only that no arm
                   // is missing from `build_lens`'s match.
                   let _lens = build_lens(mode, TimeScale::default(), false);

                   // `persist::parse_lens` round-trips the label back to
                   // the same variant for every non-None case. `None`'s
                   // label "—" is unknown to `parse_lens` and falls back
                   // to `None` — still the same variant.
                   assert_eq!(
                       persist::parse_lens(lbl),
                       mode,
                       "parse_lens({lbl:?}) did not round-trip to {mode:?}",
                   );
               }
           }
       }
       ```
       Note: the final assertion holds for `LensMode::None` too — `parse_lens("—")` hits the `_ =>` arm and returns `LensMode::None`, which matches `mode`. Unlike PATTERNS.md's sketched version (which branched on `mode != None`), this shape is symmetric and simpler.

    2. Run the full gate, in order. Each must succeed:
       ```
       cargo fmt --all --check
       cargo clippy --workspace --all-targets --locked -- -D warnings
       cargo test --workspace --all-targets --locked
       ```
       Then run the two targeted test invocations (both must pass):
       - `cargo test -p base60 all_contains_every_variant_in_cycle_order -- --exact`
       - `cargo test -p base60 all_methods_total_over_all -- --exact`

    3. If `clippy` fires on the new `tests` module:
       - `clippy::missing_docs_in_private_items` is NOT in the active profile — ignore if it appears (it shouldn't; workspace lints are `pedantic + nursery + cargo`, not `restriction`).
       - `clippy::uninlined_format_args` may fire on the `"{mode:?}"` / `"{lbl:?}"` usages — these are already inlined format args, so no lint should fire. If one does fire on an unrelated site, it's pre-existing — fix only if directly caused by this task.
       - `clippy::needless_pass_by_value` does not apply (we use `&mode` dereferences).

    4. Commit with EXACTLY this message (D-10; conventional-commit form):
       ```
       refactor(cli): drive LensMode dispatch from const ALL table [REF-02]

       - `LensMode::ALL` is the single source of truth for the variant
         list; `cycle` / `label` / `build_lens` / `persist::parse_lens`
         match arms stay as-is (compiler already catches a missing arm).
       - Two tests in `cli.rs::tests` walk `ALL` and exercise every
         dispatch site — adding a fifth variant without updating `ALL`
         OR any match arm fails at either compile time or test time.
       - `persist::parse_lens` promoted from `fn` to `pub(crate) fn` so
         the cross-module exhaustiveness test can call it.
       ```
       Staged files: `crates/base60-cli/src/cli.rs`, `crates/base60-cli/src/persist.rs`. No other files.
  </action>
  <verify>
    <automated>grep -qE '#\[cfg\(test\)\]\s*$' crates/base60-cli/src/cli.rs && grep -qE '^\s*fn all_contains_every_variant_in_cycle_order\(\)' crates/base60-cli/src/cli.rs && grep -qE '^\s*fn all_methods_total_over_all\(\)' crates/base60-cli/src/cli.rs && grep -qE '^\s*use crate::persist;' crates/base60-cli/src/cli.rs && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --all-targets --locked && cargo test -p base60 all_contains_every_variant_in_cycle_order -- --exact && cargo test -p base60 all_methods_total_over_all -- --exact</automated>
  </verify>
  <acceptance_criteria>
    - `grep -cE '#\[cfg\(test\)\]\s*$' crates/base60-cli/src/cli.rs` is at least `1` (there is a `#[cfg(test)]` annotation in `cli.rs`).
    - `grep -cE '^mod tests \{' crates/base60-cli/src/cli.rs` returns `1`.
    - `grep -cE '^\s*fn all_contains_every_variant_in_cycle_order\(\)' crates/base60-cli/src/cli.rs` returns `1`.
    - `grep -cE '^\s*fn all_methods_total_over_all\(\)' crates/base60-cli/src/cli.rs` returns `1`.
    - `grep -cE '^\s*use crate::persist;' crates/base60-cli/src/cli.rs` returns `1`.
    - `grep -cE 'for &\w+ in LensMode::ALL' crates/base60-cli/src/cli.rs` is at least `2` (one in each test).
    - `cargo fmt --all --check` exit code 0.
    - `cargo clippy --workspace --all-targets --locked -- -D warnings` exit code 0.
    - `cargo test --workspace --all-targets --locked` exit code 0.
    - `cargo test -p base60 all_contains_every_variant_in_cycle_order -- --exact` exit code 0 (test exists and passes).
    - `cargo test -p base60 all_methods_total_over_all -- --exact` exit code 0 (test exists and passes).
    - `git log -1 --pretty=%s` starts with `refactor(cli): drive LensMode dispatch from const ALL table` and ends with `[REF-02]`.
    - `git show --stat HEAD` lists exactly two files: `crates/base60-cli/src/cli.rs` and `crates/base60-cli/src/persist.rs`.
    - Zero-dep invariant preserved (passes if `[dependencies]` is absent from `crates/base60-core/Cargo.toml`, OR present but contains no non-comment non-section lines): `! grep -qE '^\[dependencies\]' crates/base60-core/Cargo.toml || ! grep -A1 '^\[dependencies\]' crates/base60-core/Cargo.toml | tail -n +2 | grep -qE '^[^#\[]'`
    - `grep -c 'fn be_u64' crates/base60-cli/src/*.rs` — combined over all `.rs` files in the CLI crate's `src/` — equals `1` (from Plan 01; unchanged by this plan).
  </acceptance_criteria>
  <done>
    Single atomic commit lands REF-02. Tree is clippy-clean, fmt-clean, and fully tested. `LensMode::ALL` is the canonical variant list with two tests guarding it (`all_contains_every_variant_in_cycle_order` covers D-08 Test 1 + D-09; `all_methods_total_over_all` covers D-08 Test 2). Phase 1 is complete — Phase 3 can now iterate `LensMode::ALL` to build the roundtrip matrix.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| (none new) | Plan is a pure internal code reorganisation. No new attack surface — same `LensMode` variants, same dispatch, same persistence format. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-01-04 | Tampering | `LensMode::ALL` drifts from actual enum variants (e.g. a future variant is added to the enum and to three of the four match arms but not to `ALL`) | mitigate | The purpose of this plan is exactly this mitigation. `all_contains_every_variant_in_cycle_order` detects a missing or misordered `ALL` at `cargo test` time; match-arm exhaustiveness (`cycle`, `label`, `build_lens`, `parse_lens` — all non-wildcard matches on `LensMode`) is compiler-enforced for three of them, and `parse_lens`'s `_ =>` fallback is covered by `all_methods_total_over_all`. |
| T-01-05 | Elevation of Privilege | `persist::parse_lens` visibility bump from `fn` to `pub(crate) fn` exposes a new API surface | accept | `pub(crate)` remains crate-internal; no public API change. `persist.rs` has no other callers outside `cli::tests` and `persist::load`. CONVENTIONS.md row 28-30 — `unreachable_pub = "warn"` — is satisfied because `tests::all_methods_total_over_all` reaches it. |
| T-01-06 | Denial of Service | Infinite loop if `cycle()` does not form a closed ring (e.g. `Time -> Time`) | mitigate | `all_contains_every_variant_in_cycle_order` walks exactly `ALL.len()` steps (hard bound of 5 on current enum). If `cycle` were buggy, the loop still terminates and the `assert_eq!(walk, expected)` inside catches the mismatch at the misbehaving step. No unbounded loop. |

Risk classification: **negligible**. No network, no filesystem, no user-input parsing changes, no auth, no new dependency, no `base60-core` change. Plan *strengthens* existing guarantees; it cannot weaken them. Existing CI (3 OS × 3 rustc × fmt + clippy `-D warnings` + full test suite) validates every combination.
</threat_model>

<verification>
End-to-end gate after Task 2:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test -p base60 all_contains_every_variant_in_cycle_order -- --exact
cargo test -p base60 all_methods_total_over_all -- --exact
```

Synthetic exhaustiveness check (human-executable, not required):
- Add a sixth variant `Xyz` to `LensMode` AND add its arms to `cycle`/`label`/`build_lens`/`parse_lens` (so the compiler is happy) BUT omit it from `ALL`.
- Run `cargo test -p base60 all_contains_every_variant_in_cycle_order -- --exact` — must FAIL with a mismatch at the `Xyz` step or at the final wrap-around assertion.
- Revert. This is the acceptance proof for ROADMAP Success Criterion 4 ("adding a fifth variant compile-errors at exactly one site (the table) — or fails a test"). Not wired into CI because the mutation itself would be the failing artefact.
</verification>

<success_criteria>
- `pub(crate) const ALL: &[LensMode]` exists in `impl LensMode` at `cli.rs`, listing every variant in cycle order (ROADMAP Success Criterion 3).
- Existing `cycle`/`label`/`build_lens`/`persist::parse_lens` match arms unchanged.
- Two exhaustiveness tests (`all_contains_every_variant_in_cycle_order`, `all_methods_total_over_all`) exist in `cli.rs::tests` and pass.
- `persist::parse_lens` is `pub(crate) fn`, not `fn` — enabling cross-module test access.
- `cargo test --workspace --all-targets --locked` green (ROADMAP Success Criterion 4).
- `base60-core/Cargo.toml` `[dependencies]` still empty or absent (ROADMAP Success Criterion 2 — unchanged by this plan).
- Single commit `refactor(cli): drive LensMode dispatch from const ALL table [REF-02]` with exactly two files staged.
</success_criteria>

<output>
After completion, create `.planning/phases/01-refactor-foundations/01-02-SUMMARY.md` recording:
- Files modified (the two above) and line counts before/after.
- Whether Test Shape B (`all_contains_every_variant_in_cycle_order` — cycle-walk) was used as recommended, or Shape A was substituted (note rationale).
- Commit SHA of the REF-02 commit.
- Gate-command exit codes (fmt / clippy / test / two targeted test runs — all must be 0).
- Optional: the manually-verified "add a sixth variant, fail the test" demonstration if executed locally (not required).
- Phase 1 complete marker: Ready for Phase 2 (`/gsd-plan-phase 2`).
</output>
