---
phase: 01-refactor-foundations
plan: 01
subsystem: cli
tags:
  - refactor
  - dedup
  - cli
dependency_graph:
  requires: []
  provides:
    - crates/base60-cli/src/chunk.rs (CHUNK, pad_chunk, be_u64 — single source of truth)
  affects:
    - crates/base60-cli/src/dump.rs
    - crates/base60-cli/src/format.rs
    - crates/base60-cli/src/tui.rs
tech_stack:
  added: []
  patterns:
    - CLI-local chunk decoding kit (chunk.rs) — by-value typed `[u8; CHUNK]` signature
key_files:
  created:
    - crates/base60-cli/src/chunk.rs
  modified:
    - crates/base60-cli/src/main.rs
    - crates/base60-cli/src/dump.rs
    - crates/base60-cli/src/format.rs
    - crates/base60-cli/src/tui.rs
decisions:
  - "Option A typed signature chosen: `be_u64([u8; CHUNK])` by value (not `&[u8; CHUNK]`) — clippy::trivially_copy_pass_by_ref required by-value for an 8-byte array"
  - "pad_chunk helper is load-bearing complement to the typed signature, not speculative"
metrics:
  duration: "2m24s"
  completed: "2026-04-23T22:38:44Z"
  tasks_completed: 2
  files_changed: 5
---

# Phase 01 Plan 01: De-duplicate `be_u64` into chunk module Summary

Single CLI-local `chunk.rs` decoding kit eliminates two copies of `be_u64` and `CHUNK` from `dump.rs` and `format.rs`, wiring all three renderers through one definition.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Create `chunk.rs` and register in `main.rs` | b8aeac5 | chunk.rs (new), main.rs |
| 2 | Delete local `be_u64`/`CHUNK`; redirect all callers | b8aeac5 | dump.rs, format.rs, tui.rs |

Both tasks landed in a single atomic commit per plan spec.

## Files Modified

| File | Before | After | Change |
|------|--------|-------|--------|
| `crates/base60-cli/src/chunk.rs` | (new) | 26 lines | Created: CHUNK, pad_chunk, be_u64 |
| `crates/base60-cli/src/main.rs` | 11 mods | 12 mods | +1 line: `mod chunk;` |
| `crates/base60-cli/src/dump.rs` | +CHUNK +be_u64 | import only | -21 lines (const + fn deleted) |
| `crates/base60-cli/src/format.rs` | +CHUNK import +be_u64 | import only | -13 lines (fn + dup docstring deleted) |
| `crates/base60-cli/src/tui.rs` | grouped import | split import | +1 line (split into chunk + dump) |

Net: 35 insertions, 33 deletions across 5 files.

## Signature Decision: Option A (by value)

Plan proposed `&[u8; CHUNK]` (Option A typed reference). During execution, `clippy::trivially_copy_pass_by_ref` fired because `[u8; 8]` is 8 bytes and fits in a register. Signature updated to `be_u64(bytes: [u8; CHUNK]) -> u64` (by value). Call sites changed from `be_u64(&pad_chunk(bytes))` to `be_u64(pad_chunk(bytes))`.

This is a correctness improvement over the plan's proposed signature — the plan itself acknowledged `clippy::missing_const_for_fn` as a risk to watch for, and by-value is the correct API here.

## Gate Command Results

| Command | Exit code |
|---------|-----------|
| `cargo fmt --all --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --all-targets --locked` | 0 (163 tests: 122 CLI + 41 core) |

## Commit

`b8aeac5` — `refactor(cli): de-duplicate be_u64 into chunk module [REF-01]`

Staged exactly 5 files: chunk.rs, main.rs, dump.rs, format.rs, tui.rs.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] By-value signature instead of by-reference**
- **Found during:** Task 2 gate (clippy)
- **Issue:** `clippy::trivially_copy_pass_by_ref` — `&[u8; CHUNK]` is an 8-byte reference to an 8-byte value; clippy requires by-value
- **Fix:** Changed `be_u64(bytes: &[u8; CHUNK])` to `be_u64(bytes: [u8; CHUNK])` and updated all 4 call sites from `be_u64(&pad_chunk(x))` to `be_u64(pad_chunk(x))`
- **Files modified:** chunk.rs, dump.rs (2 sites), format.rs (2 sites)
- **Commit:** b8aeac5 (same atomic commit)

Note: Plan's acceptance criteria grep checked for `be_u64(&pad_chunk(` (2 occurrences per file). Actual code uses `be_u64(pad_chunk(` (by value). The behaviour is identical; the API is strictly better.

## Zero-dep Invariant

`crates/base60-core/Cargo.toml` has no `[dependencies]` section. Invariant preserved.

## Note for Plan 02

Nothing blocks Plan 02 (REF-02 LensMode dispatch table). `persist::parse_lens` still needs `pub(crate)` promotion in Plan 02 per STATE.md ordering constraint.

## Known Stubs

None.

## Threat Flags

None — pure internal code reorganisation. No new trust boundaries, no new network/filesystem/auth surface.

## Self-Check: PASSED

- FOUND: `crates/base60-cli/src/chunk.rs`
- FOUND: `.planning/phases/01-refactor-foundations/01-01-SUMMARY.md`
- FOUND: commit `b8aeac5` (5 files: chunk.rs, dump.rs, format.rs, main.rs, tui.rs)
