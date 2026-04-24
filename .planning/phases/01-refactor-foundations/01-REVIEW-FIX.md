---
phase: 01-refactor-foundations
fixed_at: 2026-04-24T00:00:00Z
review_path: .planning/phases/01-refactor-foundations/01-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 01: Code Review Fix Report

**Fixed at:** 2026-04-24
**Source review:** `.planning/phases/01-refactor-foundations/01-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 3
- Fixed: 3
- Skipped: 0

## Fixed Issues

### IN-01: `pad_chunk` precondition is stricter than its callers'

**Files modified:** `crates/base60-cli/src/chunk.rs`
**Commit:** `b9d1e95`
**Applied fix:** Expanded the `pad_chunk` doc comment to note that callers
slicing out of `data.chunks(CHUNK)` are always safe and that a zero-length
slice at this boundary indicates a caller bug, not an input-data bug. The
`debug_assert!(!bytes.is_empty() && bytes.len() <= CHUNK)` was left
unchanged — the reviewer's preferred option was the doc note, not
tightening caller-side assertions.

### IN-02: `LensMode::ALL` allowance could assert its eventual removal

**Files modified:** `crates/base60-cli/src/cli.rs`
**Commit:** `ed884b5`
**Applied fix:** Replaced the descriptive comment above
`#[allow(dead_code)]` on `LensMode::ALL` with a standard
`TODO(phase-3 TEST-01)` marker pointing to `01-02-SUMMARY.md`. The
allowance now surfaces under `rg TODO` during Phase 3 kick-off so the
debt is discoverable from the source alone.

### IN-03: `all_methods_total_over_all` tolerates `None`'s label round-tripping by accident

**Files modified:** `crates/base60-cli/src/cli.rs`
**Commit:** `951968c`
**Applied fix:** Split the round-trip assertion in
`all_methods_total_over_all` so the `LensMode::None` case now tests the
unknown-label fallback explicitly (`parse_lens("—") == LensMode::None`)
while the non-`None` variants continue to test strict equality. This
preserves the exhaustiveness guarantee while making the
None-is-fallback-not-round-trip semantics explicit; a future variant
whose label happens to land in the same fallback bucket will no longer
silently pass the test.

---

_Fixed: 2026-04-24_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
