# Phase 1: Refactor Foundations - Context

**Gathered:** 2026-04-23
**Status:** Ready for planning

<domain>
## Phase Boundary

De-duplicate `be_u64` into one CLI-local module and drive every `LensMode` dispatch from a single hand-rolled `const ALL: &[LensMode]` table. No behaviour change visible to users; every v1 test stays green; `base60-core` public API is unchanged.

Requirements: **REF-01**, **REF-02**.

Not in scope: any test added beyond what's needed to prove the refactor didn't regress anything (TEST-01/03/05 are later phases); any new dependency in `base60-core`; any `LensMode` variant addition or removal.
</domain>

<decisions>
## Implementation Decisions

### Chunk module (REF-01)

- **D-01:** New module `crates/base60-cli/src/chunk.rs`. CLI-local per PROJECT.md Key Decision — `base60-core` stays zero-dep and doesn't grow chunk-decoding surface.
- **D-02:** Module is a "chunk decoding kit" (user pick), not just a single function. Contents:
  - `pub(crate) const CHUNK: usize = 8;` — canonical chunk width, re-exported from one place
  - `pub(crate) fn be_u64(bytes: &[u8; CHUNK]) -> u64` — the single source of truth
  - Room for a future `chunks(bytes: &[u8]) -> impl Iterator<Item = …>` helper when Phase 6 needs it; do not add it speculatively in this phase.
- **D-03:** Visibility is `pub(crate)`, not `pub`. The function is a CLI internal; no downstream consumer.
- **D-04:** `dump::be_u64` and `format::be_u64` are deleted; both call sites `use crate::chunk::be_u64;`. The module-level docstring at `format.rs:24` that acknowledges the duplication is removed.
- **D-05:** Any existing `const CHUNK: usize = 8;` or equivalent magic `8` in `dump.rs` / `format.rs` is replaced by `chunk::CHUNK`. Scope is the two call sites — don't sweep the whole crate.

### LensMode dispatch table (REF-02)

- **D-06:** Add `pub(crate) const ALL: &[LensMode] = &[LensMode::None, LensMode::Time, LensMode::Angle, LensMode::Tablet, LensMode::Cuneiform];` as an associated const on `impl LensMode` in `cli.rs`. Minimal-slice shape per user pick — no full-fat entry table with embedded constructors / labels.
- **D-07:** Existing `match` arms in `cycle()`, `label()`, `build_lens()`, and `persist::parse_lens()` STAY. They're idiomatic Rust and the compiler's exhaustiveness checker already forces updates on a new variant. What changes: a test guarantees `ALL` also stays complete.
- **D-08:** Exhaustiveness test lives in `cli.rs` tests module (or a sibling `lens_mode.rs` tests module if preferred):
  - Test 1 — `all_contains_every_variant`: match each `LensMode::ALL` entry against the full variant set; fail on any missing entry.
  - Test 2 — `all_methods_total_over_all`: iterate `LensMode::ALL`, call `.cycle()`, `.label()`, `build_lens(m, default, false)`, and round-trip through `persist::parse_lens(m.label())`. Any new variant added without all four sites updated will either fail a match arm at compile time OR fail this test at `cargo test`.
  - The test panics, not returns `Result` — standard idiom in this crate.
- **D-09:** `LensMode::ALL` ordering matches the current `cycle` order (None → Time → Angle → Tablet → Cuneiform). Preserves the existing behaviour for any future helper that iterates in "cycle order".

### Atomic commits

- **D-10:** Two commits, one per REQ-ID (user pick, GSD convention):
  - `refactor(cli): de-duplicate be_u64 into chunk module [REF-01]`
  - `refactor(cli): drive LensMode dispatch from const ALL table [REF-02]`
- **D-11:** REF-01 commits first (smaller, unlocks nothing that depends on REF-02). REF-02 can go in the same session but lands as a separate commit.
- **D-12:** Each commit must pass `cargo test --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings` before the next commit starts. No "WIP" state.

### Claude's Discretion

- Exact test module location (inline `#[cfg(test)] mod tests` in `cli.rs` vs. sibling file) — planner picks based on file length and crate conventions.
- Whether to reorder `LensMode` variant declaration to match `ALL` order if they already match (they do) — cosmetic, skip.
- Whether the `be_u64` module docstring references the old `dump.rs` comment — planner picks; keep it tight.
- Function signature of `be_u64` — take `&[u8; CHUNK]` (typed, panic-impossible) vs. `&[u8]` (slice, panics on wrong length). Planner picks `&[u8; CHUNK]` unless call sites can't satisfy the fixed-size type, in which case note the tradeoff.

### Folded Todos

(None — `gsd-sdk query todo.match-phase 1` returned 0 matches.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level decisions

- `.planning/PROJECT.md` — Key Decisions rows 5 (`be_u64` CLI-local) and 6 (hand-rolled `LensMode` table). Locked constraints.
- `.planning/REQUIREMENTS.md` — REF-01 and REF-02 specifications (lines 16-17).
- `.planning/ROADMAP.md` — Phase 1 Goal + 4 Success Criteria (lines 20-30). These are the acceptance bar.

### Codebase intelligence

- `.planning/codebase/ARCHITECTURE.md` — module boundaries for `base60-cli` (what `dump.rs` / `format.rs` / `cli.rs` / `persist.rs` currently do).
- `.planning/codebase/CONCERNS.md` §"Tech Debt" — first two items describe exactly REF-01 and REF-02. Read the "Fix approach" lines; they match this plan.
- `.planning/codebase/CONVENTIONS.md` — visibility patterns, `#[must_use]` idiom, `#[cfg(test)]` module style. Match these.

### v2 research outputs

- `.planning/research/ARCHITECTURE.md` — module placements, specifically the section on REF-01 / REF-02 file layout.
- `.planning/research/PITFALLS.md` — §"Refactor pitfalls", especially the `strum` in core warning (confirms our hand-rolled choice).
- `.planning/research/STACK.md` — no new crates needed for Phase 1; confirms zero-dep posture.

### Source files this phase edits

- `crates/base60-cli/src/chunk.rs` — NEW, to be created.
- `crates/base60-cli/src/cli.rs` — `LensMode::ALL` added, `cycle` / `label` / `build_lens` unchanged.
- `crates/base60-cli/src/dump.rs` — `fn be_u64` deleted (lines 35-40), `use crate::chunk::{be_u64, CHUNK};` added, references updated.
- `crates/base60-cli/src/format.rs` — `fn be_u64` and its docstring deleted (lines 24-31), `use crate::chunk::{be_u64, CHUNK};` added, references updated.
- `crates/base60-cli/src/persist.rs` — `parse_lens` (lines 139-147) covered by the new exhaustiveness test; no signature change.
- `crates/base60-cli/src/main.rs` — may need `mod chunk;` declaration; nothing else.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `LensMode` enum already has `#[must_use] const fn cycle(self)` and `#[must_use] const fn label(self)` — the table test integrates with these unchanged.
- `#[cfg(test)] mod tests` pattern is the crate-wide convention (see `main.rs:183-219`, `cuneiform.rs:150-161`, `lens.rs:321-328`). Follow this for the exhaustiveness test.
- `assert_eq!` / `assert!` with panic on failure is the standard assertion idiom — no `Result`-returning tests outside where failures are genuinely expected.

### Established Patterns

- Every numeric cast is annotated with `#[allow(clippy::cast_*)]` per `analyze.rs` evidence — irrelevant to this phase, but note if `chunk.rs` grows any cast.
- Module-level docstrings use `//!` and state purpose in one line — match the existing `dump.rs:1` / `format.rs:1` style.
- Public-within-crate symbols are `pub(crate)`; no `pub` without reason. `chunk` follows suit.

### Integration Points

- `dump.rs` uses `be_u64` at line 35 within `write_line` / `styled_line`. Single call site per function — mechanical swap.
- `format.rs` uses `be_u64` at line 26 within `emit_json` and `emit_html`. Same pattern.
- `persist.rs::parse_lens` is called from `persist::load` (deserialisation) — not touched in this phase, only referenced by the exhaustiveness test.

### Constraints from existing CI (`.github/workflows/ci.yml`)

- `cargo fmt --all --check` — new `chunk.rs` must be `rustfmt`-clean.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` with `pedantic + nursery + cargo` — the const `ALL` slice must not trigger `clippy::missing_const_for_fn` / `clippy::module_name_repetitions` warnings.
- `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS: -D warnings` — new `chunk.rs` items need doc comments on everything `pub(crate)` or above.

</code_context>

<specifics>
## Specific Ideas

- Name the module `chunk`, singular — matches `dump`/`format`/`persist`/`search` naming style (all singular).
- Prefer typed `&[u8; CHUNK]` over `&[u8]` for `be_u64` — the call sites have 8-byte chunks already; type-system-enforced length beats runtime panic. If slice-typing fights a call site, note it in the planner's rationale and escalate.
- Keep the `be_u64` body identical to the current `dump.rs:35-40` version. This is a move, not a rewrite.

</specifics>

<deferred>
## Deferred Ideas

- Chunk-iterator helper (`chunks(bytes: &[u8]) -> impl Iterator<Item = &[u8; CHUNK]>`) — useful for Phase 6 streaming. Noted as a future extension of `chunk.rs`; do not add speculatively.
- Full-fat `LensModeEntry` table (with embedded `build_fn` / persist label) — rejected for Phase 1 as too large a diff. If Phase 6 needs it for streaming dispatch, revisit then.
- Promoting `be_u64` to `base60-core` — explicitly rejected in PROJECT.md Key Decisions. Do not revisit within v2.

</deferred>

---

*Phase: 01-refactor-foundations*
*Context gathered: 2026-04-23*
