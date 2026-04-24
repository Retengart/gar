---
phase: 01-refactor-foundations
reviewed: 2026-04-24T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - crates/base60-cli/src/chunk.rs
  - crates/base60-cli/src/cli.rs
  - crates/base60-cli/src/dump.rs
  - crates/base60-cli/src/format.rs
  - crates/base60-cli/src/main.rs
  - crates/base60-cli/src/persist.rs
  - crates/base60-cli/src/tui.rs
findings:
  critical: 0
  warning: 0
  info: 3
  total: 3
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-04-24
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found (3 info only — no blockers)

## Summary

Phase 01 refactor is clean. Plan 01 (REF-01) extracts `CHUNK`, `pad_chunk`,
and `be_u64` into `crates/base60-cli/src/chunk.rs` as the single source of
truth; `dump.rs`, `format.rs`, and `tui.rs` all route through it. Plan 02
(REF-02) adds `LensMode::ALL` plus two exhaustiveness tests that walk
`cycle`, `label`, `build_lens`, and `persist::parse_lens`, promotes
`persist::parse_lens` to `pub(crate)` with a doc comment, and applies
`Self::` inside `impl LensMode` (clippy::use_self compliant).

Behaviour-preserving. The previous inlined `be_u64(bytes: &[u8])` folded
padding and big-endian decode together; the new `be_u64(pad_chunk(bytes))`
pipeline splits those at the call site. End state is byte-identical:
`u64::from_be_bytes` over a zero-padded `[u8; 8]`. The by-value
`[u8; CHUNK]` signature is the correct one for an 8-byte array (clippy
`trivially_copy_pass_by_ref` would reject `&[u8; 8]`, as the executor
discovered — see `01-01-SUMMARY.md`).

Workspace invariants preserved:

- No new `unsafe` blocks, no new dependencies, no API surface changes.
- Every public fn keeps `#[must_use]`; `be_u64` is `const fn`; `pad_chunk`
  correctly is not (`copy_from_slice` isn't const-stable).
- `base60-core` zero-dep invariant untouched.
- Doc coverage intact — `parse_lens` gained the doc its new `pub(crate)`
  visibility requires under `RUSTDOCFLAGS: -D warnings`.

Three info-level observations follow; none block merge.

## Info

### IN-01: `pad_chunk` precondition is stricter than its callers'

**File:** `crates/base60-cli/src/chunk.rs:15`
**Issue:** `pad_chunk` debug-asserts `!bytes.is_empty() && bytes.len() <= CHUNK`,
but its callers (`dump::write_line:48`, `dump::styled_line:146`,
`format::emit_json`, `format::emit_html`) only assert `bytes.len() <= CHUNK`.
A zero-length slice is reachable in principle through the `pub(crate)`
signatures of `write_line` / `styled_line`; in practice, both are only
driven by `data.chunks(CHUNK)` which never yields an empty chunk.

This is not a regression — the pre-refactor `dump::be_u64(bytes: &[u8])`
had the identical precondition at the identical location. The observation
is that moving the check into `chunk.rs` makes the asymmetry slightly
easier to miss, because the renderer-level `debug_assert` no longer
mentions non-emptiness.

**Fix (optional):** Tighten the caller-side assertions to match, or add a
one-line note to `chunk.rs`:

```rust
/// Right-pad a short byte slice to a full [`CHUNK`]-wide array with zero bytes.
///
/// `bytes.len()` must be in `1..=CHUNK`; longer slices are a programmer
/// error. Callers that slice out of `data.chunks(CHUNK)` are always safe;
/// a zero-length slice at this boundary indicates a bug in the caller,
/// not in the input data.
```

No code change required.

### IN-02: `LensMode::ALL` allowance could assert its eventual removal

**File:** `crates/base60-cli/src/cli.rs:44-47`
**Issue:** `#[allow(dead_code)]` is applied to `ALL` with a comment promising
Phase 3 will add a production reference. If Phase 3 slips or is reshuffled,
the allowance silently persists as dead weight. The plan summary
(`01-02-SUMMARY.md` "Note for Phase 3") already flags this, so the
intent is captured; a `TODO(REF-02/phase-3)` marker would make the
debt discoverable from the source alone.

**Fix (optional):** Convert the comment to a standard `TODO` so
`rg TODO` surfaces it during Phase 3 kick-off:

```rust
// TODO(phase-3 TEST-01): iterate LensMode::ALL in production code
// (see 01-02-SUMMARY.md), then drop the dead_code allow below.
#[allow(dead_code)]
pub(crate) const ALL: &[Self] = &[ ... ];
```

No code change required for correctness.

### IN-03: `all_methods_total_over_all` tolerates `None`'s label round-tripping by accident

**File:** `crates/base60-cli/src/cli.rs:316-320`
**Issue:** The test asserts `parse_lens(mode.label()) == mode` for every
variant. For `LensMode::None` this works only because `parse_lens`
falls back to `None` on **any** unrecognised string — including `"—"`.
The test comment documents this, which is good, but the invariant is
subtly fragile: if a future variant's label ever becomes `"—"` (or any
string that falls into the same fallback bucket), the test will still
pass while `parse_lens` silently misroutes it.

**Fix (optional):** Split the round-trip assertion so the `None` case
tests the fallback explicitly, and the non-`None` cases test strict
equality:

```rust
if mode == LensMode::None {
    // "—" is intentionally not a valid persisted label; it maps to
    // None via the unknown-label fallback.
    assert_eq!(persist::parse_lens(lbl), LensMode::None);
} else {
    assert_eq!(persist::parse_lens(lbl), mode,
        "parse_lens({lbl:?}) did not round-trip to {mode:?}");
}
```

This preserves the exhaustiveness guarantee while making the
None-is-fallback-not-round-trip semantics explicit.

---

_Reviewed: 2026-04-24_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
