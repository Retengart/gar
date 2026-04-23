---
phase: 01-refactor-foundations
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/base60-cli/src/chunk.rs
  - crates/base60-cli/src/main.rs
  - crates/base60-cli/src/dump.rs
  - crates/base60-cli/src/format.rs
  - crates/base60-cli/src/tui.rs
autonomous: true
requirements:
  - REF-01
tags:
  - refactor
  - cli
  - dedup

must_haves:
  truths:
    - "`be_u64` is defined exactly once in the CLI crate, at `crates/base60-cli/src/chunk.rs`."
    - "`dump.rs` and `format.rs` call `be_u64` via `use crate::chunk::{CHUNK, be_u64}` — no local copy in either file."
    - "`CHUNK` is declared exactly once in the CLI crate, at `chunk.rs`; `dump::CHUNK` no longer exists."
    - "`tui.rs` imports `CHUNK` from `crate::chunk`, not `crate::dump`, so deleting `dump::CHUNK` does not break the TUI build."
    - "`base60-core/Cargo.toml` `[dependencies]` section stays empty (zero-dep invariant preserved)."
    - "`cargo test --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings` both pass after this plan's single commit."
  artifacts:
    - path: crates/base60-cli/src/chunk.rs
      provides: "CLI-local chunk decoding kit (`CHUNK`, `pad_chunk`, `be_u64`)"
      contains: "pub(crate) const fn be_u64"
    - path: crates/base60-cli/src/main.rs
      provides: "`mod chunk;` declaration so the new module is registered with the binary"
      contains: "mod chunk;"
    - path: crates/base60-cli/src/dump.rs
      provides: "`use crate::chunk::{CHUNK, be_u64};` import; no local `fn be_u64`, no local `const CHUNK`"
    - path: crates/base60-cli/src/format.rs
      provides: "`use crate::chunk::{CHUNK, be_u64};` import; no local `fn be_u64`, no local duplicate-acknowledgement docstring"
    - path: crates/base60-cli/src/tui.rs
      provides: "Split import: `use crate::chunk::CHUNK;` + `use crate::dump::{border_style, status_style, styled_line, title_style};` — `CHUNK` now flows from `chunk`, styling helpers from `dump`"
  key_links:
    - from: crates/base60-cli/src/dump.rs
      to: crates/base60-cli/src/chunk.rs
      via: "`use crate::chunk::{CHUNK, be_u64};`"
      pattern: "use crate::chunk::\\{"
    - from: crates/base60-cli/src/format.rs
      to: crates/base60-cli/src/chunk.rs
      via: "`use crate::chunk::{CHUNK, be_u64};` (replaces old `use crate::dump::CHUNK;`)"
      pattern: "use crate::chunk::\\{"
    - from: crates/base60-cli/src/tui.rs
      to: crates/base60-cli/src/chunk.rs
      via: "`use crate::chunk::CHUNK;` (split out from the former `use crate::dump::{CHUNK, ...};`)"
      pattern: "use crate::chunk::CHUNK;"
    - from: crates/base60-cli/src/main.rs
      to: crates/base60-cli/src/chunk.rs
      via: "`mod chunk;` declaration (alphabetically between `analyze` and `cli`)"
      pattern: "^mod chunk;"
---

<objective>
De-duplicate the `be_u64` big-endian chunk decoder into a single CLI-local module `crates/base60-cli/src/chunk.rs`. All three callers (`dump.rs`, `format.rs`, `tui.rs`) import `CHUNK` from the new module; the two local `fn be_u64` copies and the duplicate-acknowledgement docstring in `format.rs` are deleted. `CHUNK` moves with `be_u64` — `dump.rs::CHUNK` goes away, `format.rs::17` redirects its import, and `tui.rs:5` splits its grouped `use crate::dump::{CHUNK, ...};` into a chunk-side and dump-side import.

Purpose: Kills REF-01 tech-debt item (CONCERNS.md §"Tech Debt" row 1). Single source of truth for chunk decoding; removes the silent-divergence risk between the terminal renderer and the JSON/HTML emitters. Prerequisite for any Phase 6 streaming work that wants a `chunks(..)` iterator helper in the same module.

Note on `pad_chunk`: `pad_chunk` is required to satisfy the Option A typed signature `be_u64(&[u8; CHUNK])`; CONTEXT D-02's "do not add speculatively" directive targeted the future `chunks()` iterator helper, not padding that is load-bearing for the chosen typed signature. `pad_chunk` is a co-requisite of the signature choice, not speculation.

Output: New file `crates/base60-cli/src/chunk.rs` (a "decoding kit" per D-02 with `CHUNK`, a small `pad_chunk` helper, and `be_u64`), updated `main.rs` / `dump.rs` / `format.rs` / `tui.rs`, one atomic commit. Zero behaviour change — `cargo test --workspace --all-targets --locked` green before AND after.
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
@crates/base60-cli/src/dump.rs
@crates/base60-cli/src/format.rs
@crates/base60-cli/src/tui.rs
@crates/base60-cli/src/main.rs
@crates/base60-cli/src/color.rs

<interfaces>
<!-- Existing `be_u64` bodies (identical). The refactor moves ONE of these into `chunk.rs` -->
<!-- and retypes the signature to `&[u8; CHUNK]` per CONTEXT "Claude's Discretion" + Option A. -->
<!-- Call sites pass `bytes: &[u8]` from `data.chunks(CHUNK)`; short final chunks (length 1-7) -->
<!-- need padding. We add a `pad_chunk` helper so call sites read `be_u64(&pad_chunk(bytes))`. -->
<!-- `pad_chunk` is not speculative — it is the load-bearing complement to the typed signature. -->

From `crates/base60-cli/src/dump.rs:20-24` (CURRENT — will be deleted):
```rust
/// Number of bytes consumed per output line.
///
/// One line ≡ one big-endian [`u64`] ≡ one base-60 number of up to
/// [`base60_core::convert::DIGITS`] digits.
pub(crate) const CHUNK: usize = 8;
```

From `crates/base60-cli/src/dump.rs:32-40` (CURRENT — will be deleted from dump.rs):
```rust
/// Parse `bytes` (length `1..=CHUNK`, right-padded with zeros) as a
/// big-endian [`u64`].
#[inline]
fn be_u64(bytes: &[u8]) -> u64 {
    debug_assert!(!bytes.is_empty() && bytes.len() <= CHUNK);
    let mut padded = [0_u8; CHUNK];
    padded[..bytes.len()].copy_from_slice(bytes);
    u64::from_be_bytes(padded)
}
```

From `crates/base60-cli/src/format.rs:17` (CURRENT — will be redirected):
```rust
use crate::dump::CHUNK;
```

From `crates/base60-cli/src/format.rs:22-31` (CURRENT — will be deleted):
```rust
/// Parse `bytes` (right-padded with zeros to 8) as big-endian `u64`.
///
/// Duplicated from `dump::be_u64` because exposing it would blur the
/// line between private renderer internals and a public conversion.
fn be_u64(bytes: &[u8]) -> u64 {
    debug_assert!(!bytes.is_empty() && bytes.len() <= CHUNK);
    let mut padded = [0_u8; CHUNK];
    padded[..bytes.len()].copy_from_slice(bytes);
    u64::from_be_bytes(padded)
}
```

From `crates/base60-cli/src/tui.rs:5` (CURRENT — will be split):
```rust
use crate::dump::{CHUNK, border_style, status_style, styled_line, title_style};
```

Target (AFTER — two lines; `chunk` before `cli` before `dump`, alphabetical):
```rust
use crate::chunk::CHUNK;
use crate::dump::{border_style, status_style, styled_line, title_style};
```

Call sites that must swap to `be_u64(&pad_chunk(bytes))`:
- `crates/base60-cli/src/dump.rs:64`  — `write_line`, `bytes: &[u8]` from `data.chunks(CHUNK)`
- `crates/base60-cli/src/dump.rs:162` — `styled_line`, same shape
- `crates/base60-cli/src/format.rs:53`  — `emit_json`, `chunk: &[u8]` from `data.chunks(CHUNK)`
- `crates/base60-cli/src/format.rs:105` — `emit_html`, same shape

(`tui.rs` has no `be_u64` call site — it only uses `CHUNK` as a layout constant. Nothing to change beyond the import line.)

Target `chunk.rs` shape (recommended):
```rust
//! 8-byte chunk decoding primitives shared by every renderer.

/// Number of bytes consumed per output line.
///
/// One line ≡ one big-endian [`u64`] ≡ one base-60 number of up to
/// [`base60_core::convert::DIGITS`] digits.
pub(crate) const CHUNK: usize = 8;

/// Right-pad a short byte slice to a full [`CHUNK`]-wide array with zero bytes.
///
/// `bytes.len()` must be in `1..=CHUNK`; longer slices are a programmer error.
#[inline]
#[must_use]
pub(crate) fn pad_chunk(bytes: &[u8]) -> [u8; CHUNK] {
    debug_assert!(!bytes.is_empty() && bytes.len() <= CHUNK);
    let mut padded = [0_u8; CHUNK];
    padded[..bytes.len()].copy_from_slice(bytes);
    padded
}

/// Decode an 8-byte big-endian chunk as a [`u64`].
#[inline]
#[must_use]
pub(crate) const fn be_u64(bytes: &[u8; CHUNK]) -> u64 {
    u64::from_be_bytes(*bytes)
}
```

Rationale for the typed `&[u8; CHUNK]` signature (Option A from CONTEXT / PATTERNS.md):
- Type-system-enforced length beats `debug_assert!` at runtime.
- `u64::from_be_bytes` is `const fn`; `*bytes` deref is `const`-eligible on MSRV 1.95. Mark `const` to satisfy `clippy::missing_const_for_fn` (nursery).
- Short-chunk padding is a separate concern (`pad_chunk`), keeping each function single-purpose.
- `pad_chunk` is required by this signature choice; without Option A's typed signature the helper would be unnecessary. It is therefore not speculative under D-02.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="false">
  <name>Task 1: Create `chunk.rs` decoding kit and register it in `main.rs`</name>
  <files>crates/base60-cli/src/chunk.rs, crates/base60-cli/src/main.rs</files>
  <read_first>
    - crates/base60-cli/src/dump.rs (lines 1-45 — canonical `CHUNK` doc + `be_u64` body to move)
    - crates/base60-cli/src/main.rs (lines 1-30 — module-declaration block; alphabetical ordering convention)
    - crates/base60-cli/src/color.rs (module-level idiom: `pub(crate)`-only, small, const-heavy; role-match analog per PATTERNS.md)
    - .planning/phases/01-refactor-foundations/01-PATTERNS.md §"`crates/base60-cli/src/chunk.rs`"
  </read_first>
  <behavior>
    - `chunk::CHUNK` is `pub(crate) const CHUNK: usize = 8;` with the same doc comment as the current `dump::CHUNK` (copied verbatim per PATTERNS.md line 86).
    - `chunk::pad_chunk(bytes)` right-pads a `&[u8]` of length `1..=CHUNK` into `[u8; CHUNK]`.
    - `chunk::be_u64(bytes: &[u8; CHUNK]) -> u64` returns `u64::from_be_bytes(*bytes)`. Marked `pub(crate) const fn`, `#[inline]`, `#[must_use]`.
    - Module docstring is a single `//!` line: `//! 8-byte chunk decoding primitives shared by every renderer.`
    - `main.rs` declares `mod chunk;` alphabetically between `mod analyze;` (line 11) and `mod cli;` (line 12).
  </behavior>
  <action>
    1. Create `crates/base60-cli/src/chunk.rs` with EXACTLY this content (per D-01/D-02/D-03; see `<interfaces>` for rationale):
       ```rust
       //! 8-byte chunk decoding primitives shared by every renderer.

       /// Number of bytes consumed per output line.
       ///
       /// One line ≡ one big-endian [`u64`] ≡ one base-60 number of up to
       /// [`base60_core::convert::DIGITS`] digits.
       pub(crate) const CHUNK: usize = 8;

       /// Right-pad a short byte slice to a full [`CHUNK`]-wide array with zero bytes.
       ///
       /// `bytes.len()` must be in `1..=CHUNK`; longer slices are a programmer error.
       #[inline]
       #[must_use]
       pub(crate) fn pad_chunk(bytes: &[u8]) -> [u8; CHUNK] {
           debug_assert!(!bytes.is_empty() && bytes.len() <= CHUNK);
           let mut padded = [0_u8; CHUNK];
           padded[..bytes.len()].copy_from_slice(bytes);
           padded
       }

       /// Decode an 8-byte big-endian chunk as a [`u64`].
       #[inline]
       #[must_use]
       pub(crate) const fn be_u64(bytes: &[u8; CHUNK]) -> u64 {
           u64::from_be_bytes(*bytes)
       }
       ```
       No imports. No test module in this file (the exhaustiveness test in Plan 02 tests `LensMode`, not chunk — `pad_chunk` / `be_u64` are pure identity+deref and are exercised by every existing dump/format test).
    2. In `crates/base60-cli/src/main.rs`, insert `mod chunk;` between `mod analyze;` (current line 11) and `mod cli;` (current line 12). Preserve alphabetical order; no blank lines between `mod` declarations.
    3. Do NOT touch `dump.rs`, `format.rs`, or `tui.rs` yet — Task 2 handles the call-site swap and import redirects. After Task 1, `chunk.rs` will be unused and `clippy` will warn with `dead_code`. That's expected and transient; Task 2 lands in the same commit, so the tree is never committed in a warning state.
    4. Do NOT run `cargo test` or `cargo clippy` between Task 1 and Task 2 — the interim tree has an unused module. Running Task 2 immediately after Task 1 is the contract.
  </action>
  <verify>
    <automated>test -f crates/base60-cli/src/chunk.rs && grep -qE '^pub\(crate\) const fn be_u64\(bytes: &\[u8; CHUNK\]\) -> u64' crates/base60-cli/src/chunk.rs && grep -qE '^pub\(crate\) fn pad_chunk\(bytes: &\[u8\]\) -> \[u8; CHUNK\]' crates/base60-cli/src/chunk.rs && grep -qE '^pub\(crate\) const CHUNK: usize = 8;' crates/base60-cli/src/chunk.rs && grep -qE '^mod chunk;$' crates/base60-cli/src/main.rs</automated>
  </verify>
  <acceptance_criteria>
    - `test -f crates/base60-cli/src/chunk.rs` succeeds.
    - `grep -cE '^pub\(crate\) const fn be_u64' crates/base60-cli/src/chunk.rs` returns `1`.
    - `grep -cE '^pub\(crate\) fn pad_chunk' crates/base60-cli/src/chunk.rs` returns `1`.
    - `grep -cE '^pub\(crate\) const CHUNK: usize = 8;' crates/base60-cli/src/chunk.rs` returns `1`.
    - `grep -cE '^#\[must_use\]' crates/base60-cli/src/chunk.rs` returns `2` (on `pad_chunk` and `be_u64`).
    - `grep -cE '^mod chunk;$' crates/base60-cli/src/main.rs` returns `1`.
    - The line number of `mod chunk;` in `main.rs` is strictly greater than the line of `mod analyze;` and strictly less than the line of `mod cli;`.
    - `rg -n '^//!' crates/base60-cli/src/chunk.rs | head -n 1` matches `//! 8-byte chunk decoding primitives shared by every renderer.`
  </acceptance_criteria>
  <done>
    New `chunk.rs` exists with the kit (CHUNK, pad_chunk, be_u64). `main.rs` declares `mod chunk;` in alphabetical position. Tree does NOT need to compile cleanly at this point — Task 2 finishes the wiring.
  </done>
</task>

<task type="auto" tdd="false">
  <name>Task 2: Delete local `be_u64` / `CHUNK` in `dump.rs` and `format.rs`; redirect all call sites (dump, format, tui) to `chunk::*`</name>
  <files>crates/base60-cli/src/dump.rs, crates/base60-cli/src/format.rs, crates/base60-cli/src/tui.rs</files>
  <read_first>
    - crates/base60-cli/src/dump.rs (lines 1-170 — see imports at 10-18, `CHUNK` at 20-24, `be_u64` at 32-40, call sites at 64 and 162)
    - crates/base60-cli/src/format.rs (lines 1-120 — import at 17, `be_u64` docstring+fn at 22-31, call sites at 53 and 105)
    - crates/base60-cli/src/tui.rs (lines 1-20 — the grouped import at line 5 `use crate::dump::{CHUNK, border_style, status_style, styled_line, title_style};` that must be split)
    - .planning/phases/01-refactor-foundations/01-PATTERNS.md §"`crates/base60-cli/src/dump.rs`" and §"`crates/base60-cli/src/format.rs`"
    - .planning/phases/01-refactor-foundations/01-CONTEXT.md §"Chunk module (REF-01)" (D-04, D-05)
  </read_first>
  <behavior>
    - `dump.rs` has zero occurrences of `fn be_u64` and zero occurrences of `const CHUNK: usize = 8;`.
    - `format.rs` has zero occurrences of `fn be_u64` and zero occurrences of `use crate::dump::CHUNK;`.
    - `tui.rs` imports `CHUNK` from `crate::chunk` (not `crate::dump`); the remaining `crate::dump::{border_style, status_style, styled_line, title_style}` import stays intact without `CHUNK`.
    - `dump.rs` and `format.rs` import `CHUNK` and `be_u64` from `crate::chunk`.
    - The duplicate-acknowledgement docstring `"Duplicated from `dump::be_u64`..."` (currently at `format.rs:23-25`) is deleted (D-04).
    - All four `be_u64` call sites (dump:64, dump:162, format:53, format:105) call `be_u64(&pad_chunk(bytes))` — the typed signature requires `&[u8; CHUNK]`, so short slices from `data.chunks(CHUNK)` must pad first.
    - `cargo test --workspace --all-targets --locked` passes after this task (behaviour is identical — `be_u64(&pad_chunk(bytes))` produces the same `u64` as the old `be_u64(bytes)` did).
    - `cargo clippy --workspace --all-targets --locked -- -D warnings` passes after this task (no new lints; no stale `use crate::dump::CHUNK` in `tui.rs`; `clippy::missing_const_for_fn` satisfied by `const fn be_u64`).
    - `cargo fmt --all --check` passes.
  </behavior>
  <action>
    1. **`crates/base60-cli/src/dump.rs`:**
       a. Delete lines 20-24 (the doc block + `pub(crate) const CHUNK: usize = 8;` declaration).
       b. Delete lines 32-40 (the `fn be_u64` definition, doc comment included).
       c. Add to the `crate::*` import group at line 10. The current first import is `use crate::color::{...};`. Add BEFORE it (alphabetical; `chunk` < `color`):
          ```rust
          use crate::chunk::{CHUNK, be_u64, pad_chunk};
          ```
          Final import-block ordering (current-crate group):
          ```rust
          use crate::chunk::{CHUNK, be_u64, pad_chunk};
          use crate::color::{
              self, Palette, delim_style, digit_style, dot_style, lens_style, offset_style, printable_style,
              sep_style,
          };
          ```
       d. Update call site at `dump.rs:64` (inside `write_line`, currently `let chunk_be = be_u64(bytes);` where `bytes: &[u8]`):
          ```rust
          let chunk_be = be_u64(&pad_chunk(bytes));
          ```
       e. Update call site at `dump.rs:162` (inside `styled_line`, same current shape):
          ```rust
          let chunk_be = be_u64(&pad_chunk(bytes));
          ```
       f. Keep the `debug_assert!(bytes.len() <= CHUNK);` at `dump.rs:63` and `dump.rs:161` (they're independent of `be_u64` — they guard against oversize slices at the `write_line`/`styled_line` entry, not inside `be_u64`).
       g. Keep `dump.rs` module docstring at lines 1-8 untouched.

    2. **`crates/base60-cli/src/format.rs`:**
       a. Replace line 17 `use crate::dump::CHUNK;` with:
          ```rust
          use crate::chunk::{CHUNK, be_u64, pad_chunk};
          ```
          (This one line replaces both the redirected `CHUNK` import and the soon-to-be-deleted `fn be_u64`.)
       b. Delete lines 22-31 (the doc block starting with `"Parse `bytes`..."`, the `"Duplicated from `dump::be_u64`..."` paragraph, and the entire `fn be_u64` body). This is the docstring D-04 explicitly calls out for removal.
       c. Update call site at `format.rs:53` (inside `emit_json`, currently `let chunk_be = be_u64(chunk);` where `chunk: &[u8]`):
          ```rust
          let chunk_be = be_u64(&pad_chunk(chunk));
          ```
       d. Update call site at `format.rs:105` (inside `emit_html`, same current shape):
          ```rust
          let chunk_be = be_u64(&pad_chunk(chunk));
          ```
       e. Keep `format.rs` module docstring at lines 1-15 untouched — only the function-level docstring at 22-25 is removed (D-04 is explicit: "The module-level docstring at `format.rs:24` that acknowledges the duplication is removed" — note PATTERNS.md clarifies this is the function docstring at 22-25, not the file header).

    3. **`crates/base60-cli/src/tui.rs`:** split the grouped import at line 5.
       BEFORE (current line 5):
       ```rust
       use crate::dump::{CHUNK, border_style, status_style, styled_line, title_style};
       ```
       AFTER (replace with exactly these two lines, in this order — `chunk` alphabetises before `cli` which alphabetises before `dump`; the existing `use crate::analyze::...;` at line 3 and `use crate::cli::...;` at line 4 stay; insert the new `use crate::chunk::CHUNK;` between line 4 and the remaining dump import):
       ```rust
       use crate::chunk::CHUNK;
       use crate::dump::{border_style, status_style, styled_line, title_style};
       ```
       No other changes to `tui.rs`. `CHUNK` is still used as a layout constant in the body; the import name is unchanged, only the path.

    4. Run the gate commands in order, each must succeed:
       ```
       cargo fmt --all --check
       cargo clippy --workspace --all-targets --locked -- -D warnings
       cargo test --workspace --all-targets --locked
       ```
       If `cargo fmt --all --check` reports diffs, run `cargo fmt --all` to fix and re-run the check. If `clippy` fires `missing_const_for_fn` on any edited function, check that `be_u64` is declared `const fn` in `chunk.rs` (it is; if the lint still fires on a call site, it's a different function — investigate).

    5. Commit with EXACTLY this message (D-10; conventional-commit form):
       ```
       refactor(cli): de-duplicate be_u64 into chunk module [REF-01]

       - New crate-private `crates/base60-cli/src/chunk.rs` owns `CHUNK`,
         `pad_chunk`, and `be_u64` as a single decoding kit.
       - `dump.rs` and `format.rs` drop their local copies and the
         "Duplicated from ..." docstring acknowledging the issue.
       - `tui.rs` splits its grouped import so `CHUNK` comes from `chunk`
         and dump-side styling helpers stay on `dump`.
       - Typed `be_u64(&[u8; CHUNK])` signature; callers pad short final
         chunks via `pad_chunk(&[u8]) -> [u8; CHUNK]`.
       ```
       Staged files: `crates/base60-cli/src/chunk.rs`, `crates/base60-cli/src/main.rs`, `crates/base60-cli/src/dump.rs`, `crates/base60-cli/src/format.rs`, `crates/base60-cli/src/tui.rs`. No other files.
  </action>
  <verify>
    <automated>grep -c 'fn be_u64' crates/base60-cli/src/dump.rs | grep -q '^0$' && grep -c 'fn be_u64' crates/base60-cli/src/format.rs | grep -q '^0$' && grep -c 'const CHUNK: usize = 8' crates/base60-cli/src/dump.rs | grep -q '^0$' && grep -c 'use crate::dump::CHUNK' crates/base60-cli/src/format.rs | grep -q '^0$' && grep -qE '^use crate::chunk::\{CHUNK, be_u64, pad_chunk\};' crates/base60-cli/src/dump.rs && grep -qE '^use crate::chunk::\{CHUNK, be_u64, pad_chunk\};' crates/base60-cli/src/format.rs && grep -qE '^use crate::chunk::CHUNK;' crates/base60-cli/src/tui.rs && ! grep -qE 'use crate::dump::\{CHUNK' crates/base60-cli/src/tui.rs && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --all-targets --locked</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c 'fn be_u64' crates/base60-cli/src/dump.rs` returns `0`.
    - `grep -c 'fn be_u64' crates/base60-cli/src/format.rs` returns `0`.
    - `grep -cE '^pub\(crate\) fn be_u64|^pub\(crate\) const fn be_u64' crates/base60-cli/src/chunk.rs` returns `1`.
    - `grep -c 'const CHUNK: usize = 8' crates/base60-cli/src/dump.rs` returns `0`.
    - `grep -c 'use crate::dump::CHUNK' crates/base60-cli/src/format.rs` returns `0`.
    - `rg -c 'use crate::dump::CHUNK' crates/base60-cli/src/` returns no output (no matches anywhere in the CLI crate — this includes `tui.rs`, which now uses `use crate::chunk::CHUNK;`).
    - `grep -cE '^use crate::chunk::\{CHUNK, be_u64, pad_chunk\};' crates/base60-cli/src/dump.rs` returns `1`.
    - `grep -cE '^use crate::chunk::\{CHUNK, be_u64, pad_chunk\};' crates/base60-cli/src/format.rs` returns `1`.
    - `grep -cE '^use crate::chunk::CHUNK;$' crates/base60-cli/src/tui.rs` returns `1`.
    - `grep -cE '^use crate::dump::\{border_style, status_style, styled_line, title_style\};$' crates/base60-cli/src/tui.rs` returns `1`.
    - `grep -cE 'use crate::dump::\{CHUNK,' crates/base60-cli/src/tui.rs` returns `0` (the old grouped form is gone).
    - `grep -c 'Duplicated from' crates/base60-cli/src/format.rs` returns `0`.
    - `grep -c 'be_u64(&pad_chunk(' crates/base60-cli/src/dump.rs` returns `2` (call sites in `write_line` and `styled_line`).
    - `grep -c 'be_u64(&pad_chunk(' crates/base60-cli/src/format.rs` returns `2` (call sites in `emit_json` and `emit_html`).
    - `cargo fmt --all --check` exit code 0.
    - `cargo clippy --workspace --all-targets --locked -- -D warnings` exit code 0.
    - `cargo test --workspace --all-targets --locked` exit code 0.
    - `git log -1 --pretty=%s` starts with `refactor(cli): de-duplicate be_u64` and ends with `[REF-01]`.
    - `git show --stat HEAD` lists exactly five files: `crates/base60-cli/src/chunk.rs`, `crates/base60-cli/src/dump.rs`, `crates/base60-cli/src/format.rs`, `crates/base60-cli/src/main.rs`, `crates/base60-cli/src/tui.rs`.
    - Zero-dep invariant (passes if `[dependencies]` is absent from `crates/base60-core/Cargo.toml`, OR present but contains no non-comment non-section lines): `! grep -qE '^\[dependencies\]' crates/base60-core/Cargo.toml || ! grep -A1 '^\[dependencies\]' crates/base60-core/Cargo.toml | tail -n +2 | grep -qE '^[^#\[]'`
  </acceptance_criteria>
  <done>
    Single atomic commit lands REF-01. Tree is clippy-clean, fmt-clean, and fully tested. `be_u64` has exactly one definition (in `chunk.rs`); `CHUNK` has exactly one definition (in `chunk.rs`). All three `CHUNK` consumers (`dump`, `format`, `tui`) import from `chunk`. `base60-core` unchanged. Ready for Plan 02 (REF-02) to start.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| (none new) | Plan is a pure internal code reorganisation — same functions, same bytes, relocated. No new boundary introduced. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-01-01 | Tampering | `be_u64` behaviour drift (new body returns a different `u64` than the old `from_be_bytes(padded)` for identical input) | mitigate | Typed signature takes `&[u8; CHUNK]`; `pad_chunk` preserves the exact right-pad-with-zero semantics of the old inline body. Existing round-trip tests across `dump`, `format::emit_json`, `format::emit_html`, and `decode` exercise every short-chunk case and pass before/after the refactor. |
| T-01-02 | Information Disclosure | Zero-dep invariant of `base60-core` accidentally violated (e.g. `be_u64` moved to `base60-core` instead of CLI-local) | mitigate | Plan explicitly scoped to `crates/base60-cli/src/chunk.rs`; acceptance criterion greps `crates/base60-core/Cargo.toml` for any non-empty `[dependencies]` section and fails the task if one appears. PROJECT.md Key Decision row 5 is locked. |
| T-01-03 | Denial of Service | `pad_chunk` called with a slice longer than `CHUNK` panics in debug, truncates-or-panics in release | accept | Call sites all feed from `data.chunks(CHUNK)`, which by construction yields slices of length `1..=CHUNK`. The existing `debug_assert!(bytes.len() <= CHUNK)` at each call site's `write_line` / `styled_line` entry (dump.rs:63, 161) is retained as a second belt. No user-controlled path can reach `pad_chunk` with an oversize slice. |
| T-01-04 | Tampering | Stale `use crate::dump::CHUNK` somewhere in the CLI crate causes a broken build after `dump::CHUNK` is deleted | mitigate | Acceptance criterion `rg -c 'use crate::dump::CHUNK' crates/base60-cli/src/` must return no matches. Known consumers audited at plan time: `format.rs:17`, `tui.rs:5`. Both are addressed in Task 2. |

Risk classification: **negligible**. No network, no filesystem, no user-input parsing changes, no auth. Attack surface is unchanged — identical bytes flow through an identically-typed pipeline, just with one fewer copy of the transform function. Existing CI (3 OS × 3 rustc × fmt + clippy `-D warnings` + full test suite) is the mitigation.
</threat_model>

<verification>
End-to-end gate after Task 2:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

Plus the grep gates listed in Task 2's `<acceptance_criteria>` (duplication truly gone, no stale `use crate::dump::CHUNK` anywhere in the CLI crate — including `tui.rs` — call sites updated, zero-dep invariant preserved, commit message conforms).

Manual smoke (optional, not gating):
```
echo -n 'Hello, World!\n' | cargo run --release --bin base60 -- --color=never | cargo run --release --bin base60 -- decode
```
Must emit `Hello, World!\n` unchanged — roundtrip correctness is preserved.
</verification>

<success_criteria>
- Exactly one `fn be_u64` across the CLI crate, in `chunk.rs` (ROADMAP Success Criterion 1).
- `dump.rs` and `format.rs` reach `be_u64` via `use crate::chunk::{CHUNK, be_u64, pad_chunk};`; `tui.rs` reaches `CHUNK` via `use crate::chunk::CHUNK;`.
- `crates/base60-core/Cargo.toml` `[dependencies]` section empty or absent (ROADMAP Success Criterion 2).
- `cargo test --workspace --all-targets --locked` green (ROADMAP Success Criterion 4).
- Single commit `refactor(cli): de-duplicate be_u64 into chunk module [REF-01]` with exactly five files staged.
</success_criteria>

<output>
After completion, create `.planning/phases/01-refactor-foundations/01-01-SUMMARY.md` recording:
- Files modified (the five above) and line counts before/after.
- Whether Option A (typed signature + `pad_chunk`) or Option B (slice signature) was taken; any rationale if B was forced.
- Commit SHA of the REF-01 commit.
- Gate-command exit codes (fmt / clippy / test — all must be 0).
- Note for Plan 02: nothing blocks; `persist::parse_lens` still needs `pub(crate)` promotion in Plan 02.
</output>
