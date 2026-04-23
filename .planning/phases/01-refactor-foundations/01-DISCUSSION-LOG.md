# Phase 1: Refactor Foundations - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-23
**Phase:** 01-refactor-foundations
**Areas discussed:** Table shape, Commit split, chunk.rs scope

---

## Table shape — `LensMode` dispatch

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal ALL slice | `const ALL: &[LensMode]` of variants; match arms stay in cycle/label/build_lens/parse_lens; test iterates ALL and verifies each method returns a value. Smallest diff, preserves idioms. | ✓ |
| Full-fat entry table | `const TABLE: &[Entry]` with `{variant, cli_flag, persist_str, build_fn}`; match arms replaced by table lookup. Adding a variant = one row. | |
| You decide | Pick whichever keeps diff smallest and test easiest | |

**User's choice:** Minimal ALL slice (recommended).
**Notes:** Keeps Rust's compile-time exhaustiveness on `match`, adds one runtime test to guard the `ALL` slice against drift.

---

## Commit split — REF-01 vs REF-02

| Option | Description | Selected |
|--------|-------------|----------|
| Two commits | One per REQ-ID; easier to revert independently; clean git log. | ✓ |
| One combined commit | Both land together as single logical change. | |

**User's choice:** Two commits (recommended).
**Notes:** Matches GSD atomic-commit convention.

---

## chunk.rs scope

| Option | Description | Selected |
|--------|-------------|----------|
| Just `be_u64` | Minimal — single `pub(crate) fn be_u64`. Lowest risk. | |
| Chunk decoding kit | Move `CHUNK` constant + `be_u64`, plus room for a future chunk iterator helper. Future-proofs Phase 6 streaming. | ✓ |

**User's choice:** Chunk decoding kit.
**Notes:** User accepted broader module scope to set up for Phase 6 streaming work — but CONTEXT.md locks "no speculative additions in Phase 1"; the iterator helper is a deferred idea, not a Phase 1 deliverable.

---

## Claude's Discretion

- Exact test module location (inline `#[cfg(test)] mod tests` vs sibling file) — planner picks.
- `be_u64` signature: `&[u8; CHUNK]` preferred over `&[u8]` unless call site forces slice. Planner confirms in plan.

## Deferred Ideas

- Chunk-iterator helper — reserved for Phase 6 streaming work.
- Full-fat `LensModeEntry` table — rejected for Phase 1; revisit if Phase 6 needs it.
- Promoting `be_u64` to `base60-core` — rejected permanently in PROJECT.md Key Decisions.
