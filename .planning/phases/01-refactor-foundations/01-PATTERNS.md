# Phase 1: Refactor Foundations - Pattern Map

**Mapped:** 2026-04-23
**Files analyzed:** 5 (1 NEW + 4 EDIT) + 1 test site
**Analogs found:** 5 / 5 (every file has at least a role-match analog)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/base60-cli/src/chunk.rs` (NEW) | utility (CLI-internal) | transform (`&[u8;8]` → `u64`) | `crates/base60-cli/src/color.rs` (single-purpose, pub(crate)-only, const-heavy) | role-match (closest "small one-purpose CLI module") |
| `crates/base60-cli/src/cli.rs` (EDIT) | config / arg-parsing | request-response (clap) | self — `impl LensMode { const fn cycle / label }` already at lines 40-66 | exact (extending an existing impl block) |
| `crates/base60-cli/src/dump.rs` (EDIT) | renderer (text) | streaming | self — current `be_u64` at lines 35-40 is the canonical body to MOVE (not rewrite) | exact (move-only refactor) |
| `crates/base60-cli/src/format.rs` (EDIT) | renderer (json/html) | streaming | `crates/base60-cli/src/dump.rs` (already imports `CHUNK` via `use crate::dump::CHUNK;` at line 17 — same import pattern, different symbol) | exact |
| `crates/base60-cli/src/main.rs` (EDIT) | binary entry | request-response | self — `mod` declarations at lines 11-20 are the ordering template | exact |
| Exhaustiveness test (location: inline `#[cfg(test)] mod tests` in `cli.rs`) | test | transform | `crates/base60-cli/src/persist.rs:163-237` and `crates/base60-cli/src/search.rs:121-204` (both: bottom-of-file, `use super::*;`, `assert_eq!`/`assert!`, panic-on-failure) | exact |

**Recommendation for test location:** inline `#[cfg(test)] mod tests` at the bottom of `cli.rs`. `cli.rs` is currently 257 lines with no test module; adding one matches `color.rs` (170 lines, has tests), `search.rs` (205 lines, has tests), `persist.rs` (238 lines, has tests). A sibling `lens_mode.rs` is unwarranted — the new const + 2 tests don't justify a new module.

---

## Pattern Assignments

### `crates/base60-cli/src/chunk.rs` (NEW — utility, transform)

**Analog:** `crates/base60-cli/src/color.rs` (small, single-purpose, all-const, `pub(crate)`-only).

**Module-level docstring pattern** (from `dump.rs:1-8`, `color.rs:1-12`, `persist.rs:1-17`, `search.rs:1-17`): start with one-line `//!` purpose, optionally followed by a blank `//!` and supporting paragraphs. The new file's docstring should be one tight line per CONTEXT.md "keep it tight" guidance — model it after `dump.rs:1` and `cli.rs:1`:

From `crates/base60-cli/src/dump.rs:1-8`:
```rust
//! Hex-dump-style line renderer: `offset  base-60 digits  |ASCII|`.
//!
//! Two rendering paths share the same heat-map palette:
//! ...
```

From `crates/base60-cli/src/cli.rs:1`:
```rust
//! Command-line interface definition.
```

**Recommended `chunk.rs` docstring** (one line, matches `cli.rs:1` brevity):
```rust
//! 8-byte chunk decoding primitives shared by every renderer.
```

---

**Use-statement ordering** (from `crates/base60-cli/src/dump.rs:10-18` — current-crate first, then `base60_core`, then externals, then `std`; alphabetised within each group; `use` items split per-import not grouped with `{ }` unless from the same path):

```rust
use crate::color::{
    self, Palette, delim_style, digit_style, dot_style, lens_style, offset_style, printable_style,
    sep_style,
};
use base60_core::convert::{DIGITS, u64_to_base60};
use base60_core::lens::Lens;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use std::io::{self, BufWriter, Write};
```

**For `chunk.rs`:** no imports needed at all (pure `u64::from_be_bytes`, no traits, no externals). Skip the `use` block entirely.

---

**Visibility / `const` declaration pattern** (from `crates/base60-cli/src/dump.rs:20-30`):

```rust
/// Number of bytes consumed per output line.
///
/// One line ≡ one big-endian [`u64`] ≡ one base-60 number of up to
/// [`base60_core::convert::DIGITS`] digits.
pub(crate) const CHUNK: usize = 8;

/// Width of the zero-padded hex offset column.
const OFFSET_WIDTH: usize = 8;

/// ASCII representation of a non-printable byte.
const DOT: u8 = b'.';
```

Doc comment first, then `pub(crate) const NAME: TYPE = …;`. Inter-item links use `[`Name`]` syntax (note the backticks-in-brackets).

**For `chunk.rs::CHUNK`:** copy the doc comment verbatim from `dump.rs:20-23` since the meaning is identical and downstream callers will see the same explanation.

---

**`be_u64` body — copy verbatim from `crates/base60-cli/src/dump.rs:32-40`:**

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

**Three deltas required by CONTEXT D-03 / D-04 / specifics:**
1. Visibility: `fn` → `pub(crate) fn` (CLI-internal but cross-module).
2. Signature: `&[u8]` → `&[u8; CHUNK]` (typed, panic-impossible, per CONTEXT specifics line 128). The `debug_assert!` and the runtime padding loop go away because the type guarantees the length.
3. Doc comment: tighten to one line (no need to mention right-padding once the slice type is fixed-size).

**Recommended final shape:**
```rust
/// Decode an 8-byte big-endian chunk as a [`u64`].
#[inline]
#[must_use]
pub(crate) fn be_u64(bytes: &[u8; CHUNK]) -> u64 {
    u64::from_be_bytes(*bytes)
}
```

`#[must_use]` is mandatory per CONVENTIONS.md "every pure public function returning a computed value" (lines 164-166) — analogs at `crates/base60-core/src/convert.rs:15`, `crates/base60-cli/src/cli.rs:43,56,74` (all the `LensMode::cycle/label`, `build_lens` already follow this).

**Clippy note:** the body is a single `u64::from_be_bytes` call — `clippy::missing_const_for_fn` (nursery) WILL fire because the function is trivially const-eligible. Either add `const` (preferred — matches `color.rs::Palette::digit` and `digit_class` in `format.rs`) or `#[allow(clippy::missing_const_for_fn)]`. Prefer `const`:

```rust
#[inline]
#[must_use]
pub(crate) const fn be_u64(bytes: &[u8; CHUNK]) -> u64 {
    u64::from_be_bytes(*bytes)
}
```

`u64::from_be_bytes` is `const fn` since Rust 1.44. The `*bytes` deref of `&[u8; N]` is `const`-eligible as of Rust 1.83 (well within MSRV 1.95).

---

**Tradeoff if `&[u8; CHUNK]` fights call sites:** if either `dump::write_line` (line 64: `let chunk_be = be_u64(bytes);` where `bytes: &[u8]` after a `chunks(CHUNK)` iterator) or `format::emit_json/html` can't easily produce a `&[u8; CHUNK]`, fall back to keeping the `&[u8]` signature with the existing pad-and-debug-assert body. Per CONTEXT line 128, escalate if that's the case. **Verified:** both call sites currently pass `bytes: &[u8]` from `data.chunks(CHUNK)` — short final chunks (length 1-7) need padding before `from_be_bytes`. The fixed-size signature requires the caller to pad first. Two options for the planner:

- **Option A (recommended):** Add a small `pad_chunk(bytes: &[u8]) -> [u8; CHUNK]` helper in `chunk.rs` for the short-chunk case, and let `be_u64` take `&[u8; CHUNK]`. Cleaner separation.
- **Option B:** Keep `be_u64(&[u8]) -> u64` (slice signature, debug_assert + pad inside). Identical to current code, just relocated. Smaller diff. CONTEXT specifics prefer Option A but explicitly allow Option B with a noted rationale.

---

### `crates/base60-cli/src/cli.rs` (EDIT — config, request-response)

**Analog:** self. `impl LensMode { … }` already exists at lines 40-66 with two `#[must_use] pub(crate) const fn` methods. Add `pub(crate) const ALL` as a third item in the same impl block.

**Existing block to extend** (`crates/base60-cli/src/cli.rs:40-66`):

```rust
impl LensMode {
    /// Advance through the lens cycle used by the interactive viewer's
    /// `L` key. Wraps back to [`LensMode::None`] from [`LensMode::Cuneiform`].
    #[must_use]
    pub(crate) const fn cycle(self) -> Self {
        match self {
            Self::None => Self::Time,
            // ...
        }
    }

    /// Short label suitable for status bars: `"time"`, `"angle"`, …
    /// `"—"` for [`LensMode::None`] so the indicator never vanishes.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            // ...
        }
    }
}
```

**Const slice precedent:** the only existing `pub const NAME: &[T; N]` slice in the workspace is `crates/base60-core/src/url.rs:28`:

```rust
pub const ALPHABET: &[u8; 60] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwx";
```

Note: `ALPHABET` uses an explicit `&[T; N]` array reference. `LensMode::ALL` per CONTEXT D-06 uses `&[LensMode]` (slice, not fixed-size array reference) — this is fine and is the more common Rust idiom for a "list of variants" where length is informational, not load-bearing. No clippy lint fires on either form.

**Recommended insertion point:** between `cycle` (ends line 52) and `label` (starts line 56), OR at the top of the impl block before `cycle`. Convention from `color.rs` puts data (the `PALETTE_NONE`/`PALETTE_ANSI` statics) before behaviour (the `impl Palette { fn digit }`); for consistency, place `ALL` BEFORE `cycle` inside the impl:

```rust
impl LensMode {
    /// Every variant in cycle order. Used by tests to prove dispatch
    /// remains exhaustive across `cycle`, `label`, `build_lens`, and
    /// `persist::parse_lens` whenever a new variant is added.
    pub(crate) const ALL: &[LensMode] = &[
        LensMode::None,
        LensMode::Time,
        LensMode::Angle,
        LensMode::Tablet,
        LensMode::Cuneiform,
    ];

    /// Advance through the lens cycle ...
    #[must_use]
    pub(crate) const fn cycle(self) -> Self { ... }
```

**Doc comment is mandatory** — workspace CI runs `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS: -D warnings` (CONTEXT line 120 / CONVENTIONS lines 208-210). Every `pub(crate)` item needs `///`.

**No `#[must_use]` on associated `const`** — `#[must_use]` applies to functions and types, not constants. Skip it here.

---

### `crates/base60-cli/src/dump.rs` (EDIT — renderer, streaming)

**Analog:** self. The deletion is mechanical.

**Delete** lines 32-40 (the local `be_u64` definition shown above).

**Add to existing import block** (currently lines 10-18). New `use` line goes in the "current-crate modules" group, alphabetically before `crate::color`:

```rust
use crate::chunk::{CHUNK, be_u64};
use crate::color::{
    self, Palette, delim_style, digit_style, dot_style, lens_style, offset_style, printable_style,
    sep_style,
};
// ... rest unchanged
```

**Also delete** the existing `pub(crate) const CHUNK: usize = 8;` at line 24 along with its doc comment (lines 20-24) — it's now imported from `chunk.rs`. Note: this re-exposes a question — does any other module import `crate::dump::CHUNK`? Yes:
- `crates/base60-cli/src/format.rs:17` — `use crate::dump::CHUNK;` — must be updated to `use crate::chunk::CHUNK;` (handled in the format.rs section below).
- `crates/base60-cli/src/tui.rs` — confirm via grep (planner: run `rg "dump::CHUNK" crates/base60-cli/src/`). If the TUI imports `CHUNK` from `dump`, swap it too. CONTEXT D-05 says "scope is the two call sites — don't sweep the whole crate" but a `use` redirect is part of the move, not a sweep.

**Module docstring at `dump.rs:1-8` stays untouched.** Per CONTEXT D-04, only the docstring at `format.rs:24` (the one acknowledging duplication) is removed; `dump.rs` has no such acknowledgement.

**Call sites in `dump.rs`** (line 64 in `write_line`, line 162 in `styled_line`) call `be_u64(bytes)` where `bytes: &[u8]`. If Option A from §chunk.rs is taken (`be_u64` takes `&[u8; CHUNK]`), these sites need to materialise a `[u8; CHUNK]` first — either via `chunk::pad_chunk(bytes)` or inline. If Option B (slice signature retained), no call-site change.

---

### `crates/base60-cli/src/format.rs` (EDIT — renderer, streaming)

**Analog:** self + `dump.rs` for the import pattern.

**Delete** lines 22-31:

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

The "Duplicated from `dump::be_u64`…" docstring (lines 24-25) is the one CONTEXT D-04 explicitly calls out for removal.

**Update** the existing import at line 17 from `use crate::dump::CHUNK;` to:

```rust
use crate::chunk::{CHUNK, be_u64};
```

This replaces both the deleted `fn be_u64` and the redirected `CHUNK` import in a single line. Slot it alphabetically with the other `crate::*` imports (it's currently the only `crate::` import, so position is unconstrained — but keep `chunk` before `dump` if a `dump::*` import is later added).

**Module docstring at `format.rs:1-15` stays.** Only the function-level docstring (lines 22-25) goes.

**Call sites** at lines 53 (`emit_json`) and 105 (`emit_html`) call `be_u64(chunk)` where `chunk: &[u8]`. Same Option A vs B decision applies as in `dump.rs`.

---

### `crates/base60-cli/src/main.rs` (EDIT — binary entry, request-response)

**Analog:** self. `mod` declarations at lines 11-20 are alphabetised with no blank lines between them:

```rust
mod analyze;
mod cli;
mod color;
mod decode;
mod dump;
mod format;
mod persist;
mod reader;
mod search;
mod tui;
```

**Add `mod chunk;` at the alphabetically correct position** — between `cli` (line 12) and `color` (line 13):

```rust
mod analyze;
mod chunk;
mod cli;
mod color;
mod decode;
mod dump;
mod format;
mod persist;
mod reader;
mod search;
mod tui;
```

No `pub(crate)` prefix (matches every other `mod` declaration in this file). No re-export from main needed; consumers (`dump`, `format`, future tests) reach in via `crate::chunk::…`.

---

### Exhaustiveness test (NEW — test, transform)

**Location decision:** inline `#[cfg(test)] mod tests` at the bottom of `crates/base60-cli/src/cli.rs`. Currently has no tests module; adding one fits crate convention (every other CLI module has a bottom-of-file tests block).

**Analog for module placement and shape:** `crates/base60-cli/src/persist.rs:163-237` (closer than `search.rs` because it tests both data structures and pure functions, like the new test will).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PersistedState { ... }

    #[test]
    fn roundtrip_full_state() {
        let s = sample();
        let text = serialize(&s);
        let back = parse(&text).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn parse_rejects_file_without_cursor_key() {
        assert!(parse("scroll=5\n").is_none());
    }
    // ...
}
```

**Pattern points to copy:**
1. `#[cfg(test)] mod tests {` at column 0, blank line above, end-of-file.
2. First line inside: `use super::*;` (no blank line before).
3. Each test fn: `#[test]` then `fn snake_case_descriptive_sentence() { … }`.
4. Assertions: `assert!`, `assert_eq!`, `assert_ne!` — panic on failure. NO `Result`-returning tests.
5. Helper functions defined inside the tests module (like `sample()` above, or `line_mono` / `line_ansi` in `dump.rs:243-259`).

**Test 1 — `all_contains_every_variant` (per CONTEXT D-08):**

Two viable shapes — pick whichever the planner finds clearer.

*Shape A — match-arm exhaustiveness (compile-time check + runtime check combined):*
```rust
#[test]
fn all_contains_every_variant() {
    // The match below fails to compile if a variant is added without
    // updating both arms; the assert fails if `ALL` is missing the entry.
    for &m in LensMode::ALL {
        match m {
            LensMode::None
            | LensMode::Time
            | LensMode::Angle
            | LensMode::Tablet
            | LensMode::Cuneiform => {}
        }
    }
    assert_eq!(LensMode::ALL.len(), 5);
}
```

*Shape B — round-trip via `cycle()` (proves ordering matches cycle):*
```rust
#[test]
fn all_matches_cycle_order() {
    let mut walk = LensMode::None;
    for &expected in LensMode::ALL {
        assert_eq!(walk, expected);
        walk = walk.cycle();
    }
    // After walking len() steps, we must be back at None.
    assert_eq!(walk, LensMode::None);
}
```

Shape B is stronger — it proves both "ALL is complete" and "ALL is in cycle order" in one test, which is what CONTEXT D-09 calls out. **Recommend Shape B for the first test.**

**Test 2 — `all_methods_total_over_all` (per CONTEXT D-08):**

```rust
#[test]
fn all_methods_total_over_all() {
    for &mode in LensMode::ALL {
        // Every variant has a non-empty label.
        let lbl = mode.label();
        assert!(!lbl.is_empty(), "label empty for {mode:?}");

        // cycle() returns a known variant.
        let next = mode.cycle();
        assert!(LensMode::ALL.contains(&next));

        // build_lens dispatches without panicking. We don't care about
        // the resulting trait object — only that no arm is missing.
        let _lens = build_lens(mode, TimeScale::default(), false);

        // persist::parse_lens round-trips the label back to the same
        // variant for every non-None case (None has the "—" label which
        // doesn't round-trip — it's the fallback for unknown labels).
        if mode != LensMode::None {
            assert_eq!(persist::parse_lens(lbl), mode, "round-trip failed for {mode:?}");
        } else {
            // None's label "—" is unknown to parse_lens, which returns None.
            assert_eq!(persist::parse_lens(lbl), LensMode::None);
        }
    }
}
```

**Visibility issue to flag:** `persist::parse_lens` is currently `fn` (no `pub(crate)`) at `persist.rs:139`. The new test in `cli.rs` cannot reach it. Two fixes:
- **Recommended:** promote it to `pub(crate) fn parse_lens(val: &str) -> LensMode` in `persist.rs:139`. Matches the pattern of every other cross-module helper in the crate.
- Alternative: put the test in `persist.rs::tests` instead, where `parse_lens` is already in scope. CONTEXT D-08 explicitly says "test lives in `cli.rs` tests module (or a sibling `lens_mode.rs`)" — so promote `parse_lens`.

**Import for the test:** `use super::*;` brings `LensMode`, `TimeScale`, `build_lens` into scope. `persist::parse_lens` needs an explicit `use crate::persist;` inside the tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist;
    // ...
}
```

**Tests panic on failure (no `Result` return):** matches every test in the crate per CONVENTIONS.md and CONTEXT D-08 ("test panics, not returns `Result` — standard idiom in this crate").

---

## Shared Patterns

### Visibility
**Source:** workspace lint `unreachable_pub = "warn"`, `crates/base60-cli/src/main.rs:7` allows `clippy::redundant_pub_crate`, every CLI module uses `pub(crate)`.
**Apply to:** every new item in `chunk.rs` and `LensMode::ALL`.
**Rule:** No bare `pub` in the binary crate. Every cross-module item is `pub(crate)`. Single-module-private items are unprefixed `fn`/`const`.

### `#[must_use]` annotation
**Source:** `crates/base60-cli/src/cli.rs:43,56,74` (`cycle`, `label`, `build_lens`).
**Apply to:** new `pub(crate) const fn be_u64`.
**Rule:** Every pure function returning a computed value gets `#[must_use]`. Skip on `const` declarations and on functions returning `()`.

### `#[inline]` on hot-path helpers
**Source:** `crates/base60-cli/src/dump.rs:34` (`be_u64`), `:55` (`write_line`).
**Apply to:** new `chunk::be_u64`.
**Rule:** Single-call-per-chunk helpers in the streaming path get `#[inline]`. Tests/CI don't enforce it but the existing `be_u64` already has it; preserve.

### `#[cfg(test)] mod tests` placement
**Source:** every CLI module — `color.rs:137`, `dump.rs:237`, `format.rs:228`, `persist.rs:163`, `search.rs:121`. Library: `cuneiform.rs:150`, `lens.rs:321`.
**Apply to:** new tests in `cli.rs`.
**Rule:** Bottom of file, blank line before, `use super::*;` as first line inside, no module-level docs, helper fns interspersed with `#[test]` fns.

### Module docstring style (`//!` one-liner where possible)
**Source:** `crates/base60-cli/src/cli.rs:1` (`//! Command-line interface definition.`).
**Apply to:** new `chunk.rs`.
**Rule:** Single-line where the module's purpose is obvious; multi-paragraph with ASCII tables only when justified (`color.rs`, `search.rs`, `format.rs` all have tables). `chunk.rs` should be a single line per CONTEXT discretion line 53 ("keep it tight").

### `use`-statement ordering
**Source:** `crates/base60-cli/src/main.rs:22-29`, `crates/base60-cli/src/dump.rs:10-18`.
**Apply to:** updated import blocks in `dump.rs` and `format.rs`.
**Rule:** Three groups, blank-line separated, alphabetised within each group:
1. `crate::*` (current-crate modules)
2. `base60_core::*` then external crates (`anyhow`, `clap`, `ratatui`, etc.) — these are merged into one alphabetic group in practice
3. `std::*`

This is enforced by `cargo fmt --all --check` (default rustfmt grouping with no `rustfmt.toml`).

### Clippy-clean `const fn` posture
**Source:** `crates/base60-cli/src/persist.rs:84` (`fnv1a` is `const fn`), `format.rs:162` (`digit_class` is `const fn`), `color.rs:72,85,…` (every style helper is `const fn`).
**Apply to:** new `chunk::be_u64`.
**Rule:** With `clippy::nursery` enabled, `clippy::missing_const_for_fn` will flag any function whose body is const-eligible. Either mark `const` or `#[allow]`. Prefer `const` — it's the dominant pattern.

### Atomic-commit policy (CONTEXT D-10/D-11/D-12)
**Source:** CONTEXT.md only — no codebase analog because this is a process rule, not a code rule.
**Apply to:** the planner's commit instructions for both REF-01 and REF-02.
**Rule:** Two commits, REF-01 first. Each commit must pass `cargo test --workspace --all-targets --locked` AND `cargo clippy --workspace --all-targets --locked -- -D warnings` standalone. Conventional-commit form: `refactor(cli): <subject> [REF-NN]`.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (none) | — | — | Every file in scope has at least a role-match analog in the existing crate. The new `chunk.rs` is novel as a "decoding kit" but its content (a `const`, a `const fn`) is structurally identical to existing patterns in `dump.rs` and `color.rs`. |

---

## Metadata

**Analog search scope:**
- `crates/base60-cli/src/*.rs` — full read of `cli.rs`, `dump.rs`, `format.rs`, `main.rs`, `color.rs`, `search.rs`, `persist.rs`
- `crates/base60-core/src/cuneiform.rs:145-162`, `lens.rs:315-329` — test-pattern reference points cited in CONTEXT
- `crates/base60-core/src/url.rs:28` — only existing `pub const NAME: &[T; N]` slice in the workspace
- Grep across both crates for `const.*: &\[`, `impl.*ValueEnum`, `#[cfg(test)]`

**Files scanned:** 9 source files + 2 codebase intelligence docs (`ARCHITECTURE.md`, `CONVENTIONS.md`)

**Pattern extraction date:** 2026-04-23
