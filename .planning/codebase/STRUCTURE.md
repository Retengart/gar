# Codebase Structure

**Analysis Date:** 2026-04-23

## Directory Layout

```
test-60/
├── Cargo.toml                          # Workspace manifest (resolver "3", edition "2024", rust 1.95)
├── Cargo.lock                          # Committed lockfile (binary crate workspace)
├── README.md                           # User-facing documentation
├── .gitignore                          # Ignores `/target` only
├── .github/
│   └── workflows/
│       └── ci.yml                      # CI pipeline
├── docs/
│   └── plans/
│       └── 2026-04-23-sumerian-roadmap.md  # Phase roadmap (all seven phases shipped)
├── crates/
│   ├── base60-core/                    # Library crate (pure logic, no I/O)
│   │   ├── Cargo.toml                  # package name = "base60-core"
│   │   └── src/
│   │       ├── lib.rs                  # Crate root; module declarations + re-exports
│   │       ├── convert.rs              # u64 ↔ base-60 digits (`u64_to_base60`, `DIGITS`)
│   │       ├── cuneiform.rs            # Sumero-Babylonian glyph table (`glyph`, `ascii_pair`)
│   │       ├── lens.rs                 # `Lens` trait + four implementations
│   │       └── url.rs                  # URL-safe 11-char encoding/decoding
│   └── base60-cli/                     # Binary crate (published as `base60`)
│       ├── Cargo.toml                  # package name = "base60", [[bin]] name = "base60"
│       └── src/
│           ├── main.rs                 # Entry point, subcommand dispatch
│           ├── cli.rs                  # clap `Parser`/`Args`/`Subcommand` definitions
│           ├── reader.rs               # Input: mmap or stdin, with clamped slicing
│           ├── dump.rs                 # Text rendering (`dump_all`, `write_line`, `styled_line`)
│           ├── format.rs               # JSON + HTML renderers (`emit_json`, `emit_html`)
│           ├── decode.rs               # Inverse of `dump`: text → bytes
│           ├── analyze.rs              # Statistics (`analyze`, `write_summary`)
│           ├── color.rs                # ANSI palette + ratatui `Style` helpers
│           ├── search.rs               # Byte-pattern parser for TUI `/`-search
│           ├── persist.rs              # XDG-state per-file TUI persistence
│           └── tui.rs                  # Interactive ratatui viewer (largest module)
└── target/                             # Cargo build output (gitignored)
```

## Directory Purposes

**`crates/base60-core/`:**
- Purpose: Reusable base-60 primitives — numeric conversion, cuneiform glyphs, lens trait, URL-safe encoding.
- Contains: Library-only Rust sources; no binaries, no I/O, no `clap` or `ratatui` dependencies.
- Key files: `crates/base60-core/src/lib.rs` (module roots + pub use re-exports), `crates/base60-core/src/convert.rs`, `crates/base60-core/src/lens.rs`.

**`crates/base60-cli/`:**
- Purpose: The `base60` command-line binary — viewer, analyzer, decoder, completions generator, TUI.
- Contains: Binary crate with a single `[[bin]]` target named `base60`, but the package directory is `base60-cli` to disambiguate from the bin name.
- Key files: `crates/base60-cli/src/main.rs` (entry + dispatch), `crates/base60-cli/src/cli.rs` (arg schema), `crates/base60-cli/src/tui.rs` (interactive viewer, 1180 lines).

**`docs/plans/`:**
- Purpose: Historical phase roadmap for the project.
- Contains: Date-prefixed markdown plans (e.g. `2026-04-23-sumerian-roadmap.md`).

**`.github/workflows/`:**
- Purpose: CI configuration.
- Contains: `ci.yml`.

**`.planning/codebase/`:**
- Purpose: GSD-generated codebase analysis output directory (where this file lives).
- Generated: Yes (by `/gsd-map-codebase` agent).
- Committed: Yes.

**`target/`:**
- Purpose: Cargo build artifacts.
- Generated: Yes.
- Committed: No (in `.gitignore`).

## Key File Locations

**Entry Points:**
- `crates/base60-cli/src/main.rs`: Binary `main()` — clap parse + subcommand match.
- `crates/base60-core/src/lib.rs`: Library crate root with module declarations and flat `pub use` re-exports.

**Configuration:**
- `Cargo.toml` (root): Workspace manifest; `[workspace.package]` fields propagated to both crates via `*.workspace = true`; shared release profile (`lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`); workspace-wide lint overrides (`clippy::pedantic`, `nursery`, `cargo` at `warn`, plus rust `unsafe_op_in_unsafe_fn`, `missing_debug_implementations`, `unreachable_pub`, `rust_2018_idioms`).
- `crates/base60-core/Cargo.toml`: No runtime dependencies.
- `crates/base60-cli/Cargo.toml`: `anyhow`, `clap` (derive), `clap_complete`, `crossterm`, `memmap2`, `ratatui`, path dep on `base60-core`.

**Core Logic (library):**
- `crates/base60-core/src/convert.rs`: `u64_to_base60`, `DIGITS = 11`.
- `crates/base60-core/src/cuneiform.rs`: `glyph(d: u8) -> &'static str`, `ascii_pair(d: u8) -> [u8; 2]`, `ascii_fallback_forced()`, `LazyLock<[String; 60]>` glyph cache.
- `crates/base60-core/src/lens.rs`: `Lens` trait, `TimeLens`, `AngleLens`, `TabletLens`, `CuneiformLens`, `TimeScale` (`Gar`/`Sec`/`Ms`).
- `crates/base60-core/src/url.rs`: `ALPHABET`, `encode_u64`, `decode_u64`, `DecodeError`.

**Core Logic (binary):**
- `crates/base60-cli/src/cli.rs`: `Cli`, `Command`, `ViewArgs`, `AnalyzeArgs`, `DecodeArgs`, `CompletionsArgs`, `LensMode`, `Format`, `TimeScale`, `ColorChoice`, `build_lens()`.
- `crates/base60-cli/src/reader.rs`: `Bytes` enum, `load()`, `clamp_range()`.
- `crates/base60-cli/src/dump.rs`: `dump_all`, `write_line`, `styled_line`, `CHUNK = 8`.
- `crates/base60-cli/src/format.rs`: `emit_json`, `emit_html`, `HTML_PROLOGUE`/`HTML_EPILOGUE`, `digit_class`.
- `crates/base60-cli/src/decode.rs`: `decode_stream`, `find_digit_run`, `parse_run`, `RUN_LEN`.
- `crates/base60-cli/src/analyze.rs`: `Analysis`, `Region`, `RegionKind`, `analyze`, `write_summary`, `DEFAULT_WINDOW = 256`.
- `crates/base60-cli/src/color.rs`: `Palette`, `PALETTE_NONE`, `PALETTE_ANSI`, ratatui `Style` constructors.
- `crates/base60-cli/src/search.rs`: `Pattern`, `ParseError`, `find_all`.
- `crates/base60-cli/src/persist.rs`: `PersistedState`, `load`, `save`, `state_file`.
- `crates/base60-cli/src/tui.rs`: `run()`, `ViewState`, `Mode`.

**Testing:**
- Every source module has a trailing `#[cfg(test)] mod tests { … }` block. There is no separate `tests/` integration directory.

## Naming Conventions

**Files:**
- Snake_case, one concern per file (e.g. `convert.rs`, `cuneiform.rs`, `persist.rs`).
- Pluralisation is avoided — modules are named for a concept, not a set (`lens.rs`, not `lenses.rs`).
- Entry-point binary module is always `main.rs`; library root is always `lib.rs`.

**Directories:**
- `crates/<crate-name>/` for each workspace member.
- Library crate directory = package name (`base60-core`).
- Binary crate directory (`base60-cli`) differs from its published package name (`base60`) to keep the binary's name short while disambiguating the source directory.

**Modules:**
- Top-level crate modules match file names.
- Re-exports at the crate root promote the most-used items to a flat namespace: `base60_core::{Lens, TimeLens, u64_to_base60, encode_u64, …}`.

**Types:**
- `PascalCase` throughout: `Lens`, `TimeScale`, `Palette`, `Bytes`, `Analysis`, `Region`, `RegionKind`, `Pattern`, `PersistedState`, `ViewState`, `Cli`, `Command`, `ViewArgs`.
- Marker/state enums use short names (`Mode`, `Format`, `ColorChoice`, `LensMode`).
- Error enums end in `Error`: `DecodeError`, `ParseError`.

**Functions:**
- `snake_case` verbs: `load`, `analyze`, `dump_all`, `write_line`, `write_summary`, `emit_json`, `emit_html`, `find_digit_run`, `parse_run`, `decode_stream`, `build_lens`, `pick_palette`, `run_view`, `run_analyze`, `run_decode`, `run_completions`.
- Constructors use `new` / `auto` / `default` (e.g. `CuneiformLens::auto()`, `Palette` statics).

**Constants:**
- `SCREAMING_SNAKE_CASE`: `DIGITS`, `CHUNK`, `RUN_LEN`, `MIN_ASCII_RUN`, `MIN_WINDOW`, `DEFAULT_WINDOW`, `HIGH_ENTROPY`, `LOW_ENTROPY`, `ALPHABET`, `PALETTE_NONE`, `PALETTE_ANSI`, `TITLE`.

**Visibility:**
- Library: `pub` for public API, module-private otherwise.
- Binary: `pub(crate)` for items shared across binary modules (follows the workspace `unreachable_pub` lint). `main.rs` has `#![allow(clippy::redundant_pub_crate)]` to reconcile with the lint.

## Where to Add New Code

**New base-60 primitive or pure numeric helper:**
- Location: `crates/base60-core/src/`, new module or existing (`convert.rs`, `url.rs`).
- Re-export in `crates/base60-core/src/lib.rs` if it belongs to the public API.
- Tests: inline `#[cfg(test)] mod tests` at the bottom of the same file.

**New `Lens` variant (semantic overlay):**
- Implement `Lens` in `crates/base60-core/src/lens.rs`, following `TimeLens`/`AngleLens` pattern.
- Add a variant to `LensMode` in `crates/base60-cli/src/cli.rs`.
- Wire into `cli::build_lens` factory.
- Extend `LensMode::cycle` and `LensMode::label` to include the new variant in the TUI `L`-key cycle.

**New output format (joining `Ansi`/`Plain`/`Json`/`Html`):**
- Add a variant to `Format` enum in `crates/base60-cli/src/cli.rs`.
- Implement the emitter in `crates/base60-cli/src/format.rs` (signature mirroring `emit_json` / `emit_html`: `(data, base_offset, writer, Option<&dyn Lens>)`).
- Route the new variant from the `match view.format` in `main::run_view`.

**New subcommand:**
- Add a variant to `Command` in `crates/base60-cli/src/cli.rs` with a matching `…Args` struct.
- Add a new module `crates/base60-cli/src/<name>.rs` implementing the subcommand body.
- Declare the module in `crates/base60-cli/src/main.rs` and add a `run_<name>` dispatcher.
- Extend the `match &args.command` in `main::main`.

**New CLI flag on the default view:**
- Add a field to `ViewArgs` in `crates/base60-cli/src/cli.rs` with appropriate `#[arg(...)]`.
- Thread through `main::run_view` and any downstream renderer function.

**New rendering utility (e.g. new ANSI tier):**
- Extend `Palette` in `crates/base60-cli/src/color.rs`; add both ANSI and ratatui `Style` variants.
- Ensure `PALETTE_NONE` gets an empty `""` for the new field (invariant: mono path allocates zero bytes).

**New input source:**
- Add a variant to `Bytes` enum in `crates/base60-cli/src/reader.rs`.
- Update `Bytes::as_slice` match.
- Add a `load_<source>` private fn alongside `load_file` / `load_stdin`.
- Route from `reader::load`.

**New TUI mode or keybinding:**
- Extend `Mode` enum in `crates/base60-cli/src/tui.rs`.
- Handle the new key in `ViewState::handle_key`.
- Update the `TITLE` constant if the status line should advertise the binding.

**Utilities shared across binary modules:**
- Place in the most specific existing module; create a new `util.rs` only if the helper is widely reused. The codebase currently has no `util.rs` — each helper lives with its primary consumer.

**Tests:**
- Always inline under `#[cfg(test)] mod tests` at the bottom of the same file as the code under test.
- No integration test directory exists; if one is needed it would go at `crates/base60-cli/tests/` or `crates/base60-core/tests/` per Cargo convention.

## Special Directories

**`target/`:**
- Purpose: Cargo build output.
- Generated: Yes.
- Committed: No (gitignored).

**`.planning/`:**
- Purpose: GSD workflow artifacts (codebase maps, phase plans).
- Generated: Yes (by `/gsd-*` commands).
- Committed: Yes.

**`docs/plans/`:**
- Purpose: Human-authored roadmap documents.
- Generated: No.
- Committed: Yes.

**`.github/`:**
- Purpose: GitHub configuration (CI workflows).
- Generated: No.
- Committed: Yes.

---

*Structure analysis: 2026-04-23*
