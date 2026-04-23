# Coding Conventions

**Analysis Date:** 2026-04-23

## Rust Edition & Toolchain

**Edition:** `2024` (set at workspace level in `Cargo.toml` `[workspace.package]`).

**MSRV:** `1.95` — enforced in CI matrix alongside `stable` and `beta`.

**Resolver:** `"3"` (workspace-level).

**License:** Dual `MIT OR Apache-2.0`.

## Workspace Lints (the primary style contract)

There is **no `rustfmt.toml`** and **no `clippy.toml`** — style is enforced
entirely through `[workspace.lints]` in `Cargo.toml` (lines 20-35), inherited
by both member crates via `[lints] workspace = true`. CI treats warnings as
errors (`cargo clippy ... -- -D warnings`) and checks formatting with
`cargo fmt --all --check` (default settings).

**Rust lints (`warn`):**
- `unsafe_op_in_unsafe_fn` — every `unsafe` op must be in an explicit
  `unsafe { ... }` block, even inside an `unsafe fn`. `crates/base60-cli/src/main.rs:1`
  raises this to `forbid`.
- `missing_debug_implementations` — every public type derives or implements `Debug`.
- `unreachable_pub` — items marked `pub` that can never be reached from
  outside the crate must use `pub(crate)`. Binary crates like
  `crates/base60-cli` use `pub(crate)` pervasively (see `crates/base60-cli/src/cli.rs`).
- `rust_2018_idioms` (priority `-1`) — group-level baseline.
- `unused_lifetimes`, `unused_qualifications`.

**Clippy lints (`warn`):**
- `pedantic`, `nursery`, `cargo` — all enabled at priority `-1`.
- `multiple_crate_versions` allowed (transitive dep graph; not actionable).
- `module_name_repetitions` allowed (module-per-concern layout preferred).
- `redundant_pub_crate` allowed at the binary crate root
  (`crates/base60-cli/src/main.rs:7`) because `unreachable_pub` and
  `redundant_pub_crate` conflict; correctness wins.

## Naming Patterns

**Files:** `snake_case.rs`, one module per concern. Examples: `convert.rs`,
`cuneiform.rs`, `lens.rs`, `persist.rs`, `reader.rs`.

**Modules:** Flat within each crate's `src/`. No nested `mod.rs` hierarchies;
each sub-module is a sibling file registered from `lib.rs` or `main.rs`.

**Types:** `UpperCamelCase`. Enums frequently end in a functional suffix
(`LensMode`, `ColorChoice`, `Format`, `Command`, `Pattern`, `ParseError`,
`DecodeError`, `PersistedState`, `Bytes`, `Mode`).

**Traits:** `UpperCamelCase`, short nouns (`Lens` in `crates/base60-core/src/lens.rs:35`).

**Functions & methods:** `snake_case`. Predicates are prefixed (`is_ansi`,
`is_digit_run`, `looks_like_hex`, `not_extended_left`). Constructors use
`new` or a verb-based factory (`CuneiformLens::auto`, `TabletLens::default`).

**Constants & statics:** `SCREAMING_SNAKE_CASE` (`DIGITS`, `CHUNK`,
`RUN_LEN`, `PALETTE_NONE`, `PALETTE_ANSI`, `HIGH_ENTROPY`, `MIN_WINDOW`,
`DEFAULT_WINDOW`, `MIN_ASCII_RUN`, `TITLE`, `XDG_STATE_HOME`).

**Struct fields:** `snake_case`, publicly `pub(crate)` inside the binary
crate; `pub` in `base60-core` only for genuine library API
(`TimeLens::scale`, `TabletLens::purist`, `CuneiformLens::fallback`).

**Test functions:** `snake_case`, descriptive sentences. Examples from
`crates/base60-core/src/convert.rs`: `zero`, `fifty_nine`,
`sixty_rolls_over`, `classic_example_5025`,
`u64_max_roundtrips_in_eleven_digits`. Tests read like specifications.

## Error Handling

**Two distinct strategies, chosen by crate role.**

**Library crate (`base60-core`)** defines **bespoke typed errors** and
returns `Result<T, E>`:
- `crates/base60-core/src/url.rs:31` — `pub enum DecodeError { WrongLength,
  InvalidCharacter }` with `#[derive(Clone, Debug, Eq, PartialEq)]`.
- No `thiserror` / `anyhow` dependencies — the library stays small and
  dependency-free (`crates/base60-core/Cargo.toml` has zero runtime deps).
- Overflow paths prefer checked arithmetic and map the failure into the
  domain error (`value.checked_mul(60).and_then(...).ok_or(...)`,
  `crates/base60-core/src/url.rs:69`).

**Binary crate (`base60`)** uses **`anyhow` for top-level error flow**:
- `crates/base60-cli/Cargo.toml:18` declares `anyhow = "1.0.102"`.
- `crates/base60-cli/src/main.rs:22,31` — `fn main() -> anyhow::Result<()>`.
- `crates/base60-cli/src/reader.rs` uses `.with_context(|| format!("open
  {}", path.display()))` to attach path-level context at the syscall
  boundary.
- Module-internal errors stay typed: `crates/base60-cli/src/search.rs:27`
  defines `ParseError { Empty, InvalidHex }`; `crates/base60-cli/src/decode.rs`
  returns `io::Result<()>` with `io::Error::new(io::ErrorKind::InvalidData,
  format!(...))` for structured failures.

**Broken-pipe policy:** Downstream consumers closing a pipe early
(`head`, `grep`, etc.) is treated as a clean exit, not an error, matching
`cat`/`hexdump` behaviour. See
`crates/base60-cli/src/main.rs:102-105,117-118,136-138`:
```rust
Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
```

**Best-effort persistence:** `crates/base60-cli/src/persist.rs:60-70`
swallows I/O errors entirely — losing a cursor position is not worth
aborting a clean TUI quit.

## Panic Policy

**Panics only on programmer errors.** Runtime failures always route through
`Result`.

- `unwrap()` / `expect()` appears only inside `#[cfg(test)]` modules or
  after a proven-ASCII construction (`crates/base60-cli/src/decode.rs:56`:
  `std::str::from_utf8(slice).expect("ascii")`).
- `debug_assert!` is used to encode contracts that would be UB-equivalent
  in a release build: `crates/base60-core/src/cuneiform.rs:37,72`,
  `crates/base60-cli/src/decode.rs:64,97`,
  `crates/base60-cli/src/dump.rs:36`.
- `# Panics` sections in rustdoc document every debug-panic path
  (e.g. `crates/base60-core/src/cuneiform.rs:65-68`).
- No `todo!`, `unimplemented!`, or `unreachable!` in the shipped code.
- Saturating / `checked_*` / `try_from(..).unwrap_or(usize::MAX)`
  arithmetic is preferred over raw casts for range-clamped values
  (`crates/base60-cli/src/reader.rs:72-78`).

## Unsafe Policy

**`unsafe` is rare, localised, and always accompanied by a `SAFETY:` comment.**

- Workspace-level `unsafe_op_in_unsafe_fn = "warn"`; binary crate root
  escalates to `#![forbid(unsafe_op_in_unsafe_fn)]`
  (`crates/base60-cli/src/main.rs:1`).
- Current `unsafe` sites:
  - `crates/base60-cli/src/reader.rs:56` — `unsafe { Mmap::map(&file) }`,
    with explicit rationale covering concurrent file mutation.
  - `crates/base60-core/src/cuneiform.rs:154,156`,
    `crates/base60-core/src/lens.rs:324,327`,
    `crates/base60-cli/src/main.rs:191,198,205,208` — Rust-2024 `env::set_var`
    / `env::remove_var` calls inside tests, each carrying a `// SAFETY:`
    block explaining the single-threaded test assumption.

## Module Organisation

**Pattern:** One concern per file; flat module tree; each module starts with
a `//!` crate/module-level doc comment that explains *why* it exists.

**Layering:**
- `crates/base60-core` is a pure library: `convert` → `cuneiform` → `lens`,
  with `url` as an independent sibling. `lib.rs` re-exports the public
  surface (`crates/base60-core/src/lib.rs:26-29`).
- `crates/base60-cli` depends on `base60-core` via a path dep
  (`crates/base60-cli/Cargo.toml:24`). CLI modules (`cli`, `dump`, `format`,
  `decode`, `analyze`, `reader`, `search`, `persist`, `tui`, `color`) are
  registered from `main.rs` and all items below the crate root are
  `pub(crate)`.

**Attribute hygiene:**
- `#[inline]` on hot-path helpers returning small structs
  (`crates/base60-core/src/convert.rs:16`, `/cuneiform.rs:34,69`,
  `crates/base60-cli/src/dump.rs:34,55`).
- `#[must_use]` on every pure public function returning a computed value
  (`crates/base60-core/src/convert.rs:15`, `/url.rs:40`, `/cuneiform.rs:35,70,83`,
  `/lens.rs:166`, `crates/base60-cli/src/cli.rs:43,56,74`).
- `#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]` is the canonical
  "value type" set; `ValueEnum` for clap enums
  (`crates/base60-cli/src/cli.rs:12,24,92,105`).

## Import Organisation

Imports are split into three ordered groups, separated by blank lines (the
default `rustfmt` grouping). Example from `crates/base60-cli/src/main.rs:22-29`:

```rust
use anyhow::Result;                            // external crates
use base60_core::Lens;
use clap::CommandFactory;
use clap::Parser;
use cli::{AnalyzeArgs, ColorChoice, ...};     // current-crate modules
use color::Palette;
use std::fs::File;                             // stdlib
use std::io::{BufReader, BufWriter, IsTerminal, stdout};
```

No custom path aliases; every import uses `crate::`, `super::`, or a
fully-qualified crate name.

## Doc Comment Style

**Every file starts with `//! ...`** summarising the module's purpose,
invariants, and rationale — often several paragraphs with ASCII tables.
Examples: `crates/base60-core/src/lib.rs:1-20`,
`crates/base60-core/src/convert.rs:1-6`, `crates/base60-cli/src/analyze.rs:1-16`.

**Every public item carries `///` docs.** Conventions:
- First sentence is a complete imperative summary.
- `# Errors` sections enumerate every `Err` variant
  (`crates/base60-core/src/url.rs:51-55`, `crates/base60-cli/src/tui.rs:47-51`).
- `# Panics` sections document debug-only panics
  (`crates/base60-core/src/cuneiform.rs:65-68`).
- Executable `///` examples provide doc tests: see
  `crates/base60-core/src/url.rs:10-18` (fenced with ` ``` ` — run as part
  of `cargo test --doc` in CI).
- Inter-item links use `` [`Name`] `` syntax (`crates/base60-core/src/lib.rs`
  uses this throughout its module description).
- `RUSTDOCFLAGS: -D warnings` in CI means every broken link or malformed
  example fails the build (`.github/workflows/ci.yml:67`).

## Comment Style Inside Code Bodies

Heavy use of explanatory inline comments for:
- Non-obvious arithmetic bounds (`crates/base60-core/src/url.rs:63-68`).
- Trade-offs (`crates/base60-cli/src/main.rs:99-102`,
  `crates/base60-cli/src/persist.rs:231-236`).
- Historical/domain rationale (`crates/base60-core/src/cuneiform.rs:11-15`
  on the Babylonian zero placeholder).

Comments explain *why*, not *what*. One-line summaries sit above the line
they describe.

## Function Design

- Small, single-purpose. The longest pure function is
  `crates/base60-cli/src/decode.rs:parse_run` at ~25 lines.
- Generic over the output sink: rendering helpers take `W: Write` so they
  work equally with `Vec<u8>` in tests and `BufWriter<StdoutLock>` at
  runtime (`crates/base60-cli/src/dump.rs:56`,
  `crates/base60-cli/src/format.rs:49,99`,
  `crates/base60-cli/src/analyze.rs:209`).
- Prefer borrowed parameters (`&[u8]`, `&str`, `&Path`, `Option<&Path>`)
  over owned ones unless ownership is required.

---

*Convention analysis: 2026-04-23*
