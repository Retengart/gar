<!-- GSD:project-start source:PROJECT.md -->
## Project

**base60 — Sumero-Babylonian binary viewer**

`base60` is a hex-dump alternative that renders every 8 bytes of input as eleven
sexagesimal (base-60) digit pairs — a single statically-linked Rust CLI that
ships a coloured TTY dump, four optional lenses (including actual cuneiform),
entropy-based statistical analysis, roundtrip decoding, and a ratatui-based
interactive TUI. Target user: anyone who reaches for `xxd` / `hexdump` and
wants a denser, more visually legible view of binary files.

**Core Value:** Every binary blob that `base60 FILE | base60 decode` round-trips must come out
byte-identical — the visual format is opinionated, but the pipeline is
lossless.

### Constraints

- **Tech stack**: Rust edition 2024, MSRV `1.95`, single binary via
  `cargo install`. No runtime dependencies, no service, no daemon.
- **Library API**: `base60-core` must keep zero external dependencies —
  its selling point.
- **Backwards compatibility**: JSON schema and `decode` accept-format are
  stable. Any change must be additive and gated.
- **Output determinism**: `NO_COLOR`, `NO_UNICODE`, `TERM=dumb` behaviours
  are contractual. State-file byte ordering is deterministic by explicit sort.
- **Platform**: CI matrix of Ubuntu/macOS/Windows × rustc 1.95/stable/beta
  is the correctness floor. Nothing may regress any of these.
- **Lint bar**: `clippy::pedantic + nursery + cargo` with `-D warnings` stays
  enforced. `multiple_crate_versions` and `module_name_repetitions` are the
  only documented allows.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust (edition 2024) - All workspace code under `crates/base60-core/src/` and `crates/base60-cli/src/`
- Not applicable (pure Rust workspace; no embedded DSLs, build scripts, or non-Rust sources)
## Runtime
- Native compiled binary via `cargo build`. No interpreter or VM.
- Platform targets exercised in CI: `ubuntu-latest`, `macos-latest`, `windows-latest` (see `.github/workflows/ci.yml`).
- Cargo (bundled with the Rust toolchain)
- Lockfile: `Cargo.lock` present at workspace root (~47KB, 201 resolved packages). CI invokes `--locked` everywhere.
## Frameworks
- `clap` 4.6.1 (features: `derive`) - Argument parsing for the `base60` binary. See `crates/base60-cli/src/cli.rs`.
- `clap_complete` 4.6.1 (resolves to 4.6.2 in lock) - Shell completion script generation for the `completions` subcommand. See `crates/base60-cli/src/main.rs::run_completions`.
- `ratatui` 0.30.0 - Terminal UI framework powering the interactive TUI (`-i` flag). See `crates/base60-cli/src/tui.rs`.
- `crossterm` 0.29.0 - Backend for `ratatui`; raw-mode input, cursor, and color. See `crates/base60-cli/src/tui.rs`.
- `anyhow` 1.0.102 - Top-level error type for the binary. See `crates/base60-cli/src/main.rs`.
- `memmap2` 0.9.10 - Memory-mapped file reads for large inputs in `crates/base60-cli/src/reader.rs`.
- Rust built-in `#[test]` / `#[cfg(test)]` harness (no external test framework). Inline unit tests live next to the code they exercise (e.g. `tests` module at the bottom of `crates/base60-cli/src/main.rs`). Doc tests are run separately in CI.
- `cargo` (build, test, doc, install) - sole build tool.
- `rustfmt` - enforced via `cargo fmt --all --check` in CI (`fmt` job).
- `clippy` - enforced via `cargo clippy --workspace --all-targets --locked -- -D warnings` in CI (`clippy` job). Workspace-level lint groups enabled: `clippy::pedantic`, `clippy::nursery`, `clippy::cargo`.
- `rustdoc` - `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS: -D warnings` in CI (`doc` job).
## Key Dependencies
- `anyhow` = "1.0.102" - Error aggregation in `main`.
- `clap` = "4.6.1" (features: `derive`) - CLI parser.
- `clap_complete` = "4.6.1" - Shell completion generator.
- `crossterm` = "0.29.0" - Low-level terminal control.
- `memmap2` = "0.9.10" - Mmap-backed input reader.
- `ratatui` = "0.30.0" - TUI layout/widgets.
- `base60-core` = { path = "../base60-core" } - Intra-workspace dep for conversion/lens primitives.
- None. The core library has zero external dependencies; it builds from `std` alone. Categories declare `no-std` compatibility intent even though the current implementation uses `std::sync::LazyLock` and `String` (see `crates/base60-core/src/lib.rs` module doc).
- `serde` / `serde_derive` / `serde_core` 1.0.228, `serde_json` 1.0.149 - pulled in by `ratatui` / `termwiz` dependency chains; not used directly by workspace code.
- `regex` 1.12.3, `fancy-regex` 0.11.0, `regex-automata` 0.4.14, `regex-syntax` 0.8.10 - transitive (likely via `termwiz`/`ratatui-termwiz`).
- `ratatui-core` 0.1.0, `ratatui-crossterm` 0.1.0, `ratatui-macros` 0.7.0, `ratatui-termwiz` 0.1.0, `ratatui-widgets` 0.3.0 - ratatui's split crate set.
- `thiserror` 1.0.69 and 2.0.18 (both versions co-exist; `clippy::multiple_crate_versions` is explicitly allowed at workspace level).
- `wasm-bindgen` 0.2.118, `wasmparser` / `wasm-encoder` / `wit-*` 0.244.0 - transitive through `termwiz`/`ratatui` ecosystem (terminal color/blob support).
- `windows-sys` 0.61.2, `winapi` 0.3.9, `crossterm_winapi` 0.9.1 - Windows target support for `crossterm`.
- `libc` 0.2.185, `rustix` 1.1.4, `linux-raw-sys` 0.12.1 - Unix syscalls used by `crossterm` / `memmap2`.
- No runtime infrastructure dependencies (no HTTP client, no DB driver, no async runtime). Pure offline CLI tool.
## Configuration
- `NO_COLOR` - honored at runtime for auto color detection. See `crates/base60-cli/src/main.rs::pick_palette`. Follows https://no-color.org.
- CI-only env: `CARGO_TERM_COLOR=always`, `RUST_BACKTRACE=1`, `CARGO_INCREMENTAL=0` (set in `.github/workflows/ci.yml`).
- `Cargo.toml` (workspace root) - defines `[workspace]`, `[workspace.package]`, shared `[profile.release]`, and `[workspace.lints.*]`.
- `crates/base60-core/Cargo.toml` - library crate manifest; inherits `version`/`edition`/`rust-version`/`license`/`repository` from workspace.
- `crates/base60-cli/Cargo.toml` - binary crate manifest; declares `[[bin]] name = "base60" path = "src/main.rs"`.
- No `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, or `.cargo/config.toml` present.
- `rust`: `unsafe_op_in_unsafe_fn = warn`, `missing_debug_implementations = warn`, `unreachable_pub = warn`, `rust_2018_idioms = warn`, `unused_lifetimes = warn`, `unused_qualifications = warn`.
- `clippy`: `pedantic = warn`, `nursery = warn`, `cargo = warn`, with explicit allows for `multiple_crate_versions` and `module_name_repetitions`.
## Platform Requirements
- Rust toolchain with `rustc`/`cargo` 1.95.0 or newer (declared `rust-version = "1.95"` in workspace). CI matrix covers `1.95.0`, `stable`, `beta`.
- Edition 2024 support (requires recent-enough toolchain).
- Any OS supported by Rust stdlib + `crossterm` (Linux, macOS, Windows — all three exercised in CI).
- Single statically-linked binary (`base60`) installed via `cargo install --path crates/base60-cli`. Default location `$HOME/.cargo/bin/base60`.
- No service, no daemon. Reads files or stdin, writes to stdout or alternate screen (TUI).
- `publish = false` at workspace level — crates not published to crates.io.
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Rust Edition & Toolchain
## Workspace Lints (the primary style contract)
- `unsafe_op_in_unsafe_fn` — every `unsafe` op must be in an explicit
- `missing_debug_implementations` — every public type derives or implements `Debug`.
- `unreachable_pub` — items marked `pub` that can never be reached from
- `rust_2018_idioms` (priority `-1`) — group-level baseline.
- `unused_lifetimes`, `unused_qualifications`.
- `pedantic`, `nursery`, `cargo` — all enabled at priority `-1`.
- `multiple_crate_versions` allowed (transitive dep graph; not actionable).
- `module_name_repetitions` allowed (module-per-concern layout preferred).
- `redundant_pub_crate` allowed at the binary crate root
## Naming Patterns
## Error Handling
- `crates/base60-core/src/url.rs:31` — `pub enum DecodeError { WrongLength,
- No `thiserror` / `anyhow` dependencies — the library stays small and
- Overflow paths prefer checked arithmetic and map the failure into the
- `crates/base60-cli/Cargo.toml:18` declares `anyhow = "1.0.102"`.
- `crates/base60-cli/src/main.rs:22,31` — `fn main() -> anyhow::Result<()>`.
- `crates/base60-cli/src/reader.rs` uses `.with_context(|| format!("open
- Module-internal errors stay typed: `crates/base60-cli/src/search.rs:27`
## Panic Policy
- `unwrap()` / `expect()` appears only inside `#[cfg(test)]` modules or
- `debug_assert!` is used to encode contracts that would be UB-equivalent
- `# Panics` sections in rustdoc document every debug-panic path
- No `todo!`, `unimplemented!`, or `unreachable!` in the shipped code.
- Saturating / `checked_*` / `try_from(..).unwrap_or(usize::MAX)`
## Unsafe Policy
- Workspace-level `unsafe_op_in_unsafe_fn = "warn"`; binary crate root
- Current `unsafe` sites:
## Module Organisation
- `crates/base60-core` is a pure library: `convert` → `cuneiform` → `lens`,
- `crates/base60-cli` depends on `base60-core` via a path dep
- `#[inline]` on hot-path helpers returning small structs
- `#[must_use]` on every pure public function returning a computed value
- `#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]` is the canonical
## Import Organisation
## Doc Comment Style
- First sentence is a complete imperative summary.
- `# Errors` sections enumerate every `Err` variant
- `# Panics` sections document debug-only panics
- Executable `///` examples provide doc tests: see
- Inter-item links use `` [`Name`] `` syntax (`crates/base60-core/src/lib.rs`
- `RUSTDOCFLAGS: -D warnings` in CI means every broken link or malformed
## Comment Style Inside Code Bodies
- Non-obvious arithmetic bounds (`crates/base60-core/src/url.rs:63-68`).
- Trade-offs (`crates/base60-cli/src/main.rs:99-102`,
- Historical/domain rationale (`crates/base60-core/src/cuneiform.rs:11-15`
## Function Design
- Small, single-purpose. The longest pure function is
- Generic over the output sink: rendering helpers take `W: Write` so they
- Prefer borrowed parameters (`&[u8]`, `&str`, `&Path`, `Option<&Path>`)
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- `base60-core` (lib) owns all pure numeric/encoding logic; no I/O, no terminal concerns, no CLI dependencies. Categorised as `no-std`-compatible encoding primitive (Cargo `categories = ["encoding", "no-std"]`) though currently uses `std` for `LazyLock` / `String`.
- `base60` (bin, crate name `base60`, package dir `crates/base60-cli`) owns all user-facing I/O: argument parsing, mmap/stdin reader, dump/format/decode writers, and the interactive TUI.
- Lens-based plugin system: `base60_core::lens::Lens` trait lets any semantic overlay (time, angle, tablet, cuneiform) render alongside the core dump without the renderers knowing the variants.
- Format-agnostic rendering: four output formats (`Ansi`, `Plain`, `Json`, `Html`) share a single byte-chunking + `u64_to_base60` pipeline; palette/format branching happens at the outermost layer of `run_view`.
- Bidirectional: `dump::dump_all` bytes→text; `decode::decode_stream` text→bytes, closing a full roundtrip loop with `base60 --color=never FILE | base60 decode`.
## Layers
- Purpose: Convert `u64` ↔ base-60 digits and Sumero-Babylonian glyph table.
- Location: `crates/base60-core/src/convert.rs`, `crates/base60-core/src/cuneiform.rs`
- Contains: `u64_to_base60`, `DIGITS` const, `ascii_pair`, `glyph`, `ascii_fallback_forced`.
- Depends on: `std` only (`LazyLock` for glyph cache).
- Used by: every renderer in `base60-cli`, every lens in `base60-core::lens`.
- Purpose: URL-safe 11-char `u64` encoding using unambiguous 60-symbol alphabet (`0-9A-Za-x`).
- Location: `crates/base60-core/src/url.rs`
- Contains: `ALPHABET`, `encode_u64`, `decode_u64`, `DecodeError`.
- Depends on: `convert` module.
- Used by: re-exported at crate root for downstream consumers; not used by the CLI dump/decode paths (those operate on 8-byte chunks, not single `u64`s).
- Purpose: Reinterpret a raw `u64` chunk as a piece of Sumero-Babylonian semantics.
- Location: `crates/base60-core/src/lens.rs`
- Contains: `Lens` trait (`Send + Sync`, single `fn render(&self, chunk: u64) -> String`), `TimeLens`, `AngleLens`, `TabletLens`, `CuneiformLens`, `TimeScale` enum.
- Depends on: `convert`, `cuneiform`.
- Used by: `dump::write_line`, `dump::styled_line`, `format::emit_json`, `format::emit_html`, and the TUI `L`-key cycle.
- Purpose: Parse command line, build a `Lens` trait object, dispatch subcommand.
- Location: `crates/base60-cli/src/cli.rs`
- Contains: `Cli` (clap `Parser`), `Command` enum (`Analyze`, `Decode`, `Completions`; default = view), `ViewArgs`, `AnalyzeArgs`, `DecodeArgs`, `CompletionsArgs`, `LensMode`, `TimeScale`, `Format`, `ColorChoice`, `build_lens` factory.
- Depends on: `base60_core::lens::*`, `clap`, `clap_complete::Shell`.
- Used by: `main.rs` only.
- Purpose: Unified `&[u8]` over mmap or stdin, with `--skip`/`--length` clamping.
- Location: `crates/base60-cli/src/reader.rs`
- Contains: `Bytes` enum (`Mapped { map, start, end }` or `Owned(Vec<u8>)`), `load()`, `clamp_range()`.
- Depends on: `memmap2::Mmap`, `anyhow`.
- Used by: `run_view`, `run_analyze` in `main.rs`. `run_decode` skips this layer and uses `BufRead` directly because dump files are small.
- Purpose: Turn bytes into a formatted line. Four parallel implementations, one per output format.
- Location:
- Each accepts `data: &[u8]`, `base_offset: u64`, a writer, and `Option<&dyn Lens>`.
- Chunk size is fixed at `CHUNK = 8` bytes (one big-endian `u64` → 11 base-60 digits).
- Purpose: Reverse of `dump::dump_all`. Parse `NN:NN:…:NN` runs back into big-endian bytes.
- Location: `crates/base60-cli/src/decode.rs`
- Contains: `decode_stream`, `find_digit_run`, `parse_run`. Scanner ignores surrounding text (offset columns, ASCII columns, ANSI escapes) and uses `not_extended_left`/`not_extended_right` guards to refuse overlapping windows (e.g. 12-pair runs).
- Uses `u128` accumulator to detect overflow before truncating to `u64`.
- Purpose: Single source of truth for the four-tier heatmap colours across CLI and TUI.
- Location: `crates/base60-cli/src/color.rs`
- Contains: `Palette` struct of static string ANSI escapes, `PALETTE_NONE` (all empty), `PALETTE_ANSI`, ratatui `Style` constructors (`digit_style`, `offset_style`, `sep_style`, `delim_style`, `printable_style`, `dot_style`, `lens_style`, `title_style`, `border_style`, `status_style`).
- Tiers: digit 0 = DarkGray, 1–19 = Green, 20–39 = Yellow, 40–59 = Red.
- Purpose: Statistical summary for `base60 analyze` (entropy, byte histogram, ASCII regions).
- Location: `crates/base60-cli/src/analyze.rs`
- Contains: `Analysis`, `Region`, `RegionKind`, `analyze()`, `write_summary()`, plus `MIN_WINDOW = 64`, `DEFAULT_WINDOW = 256`, entropy thresholds (`LOW_ENTROPY = 1.0`, `HIGH_ENTROPY = 7.5`).
- Purpose: Full-screen ratatui TUI with hjkl cursor motion, lens cycling, search, bookmarks, semantic jumps.
- Location: `crates/base60-cli/src/tui.rs` (1180 lines — largest file)
- Depends on: `ratatui`, `crossterm`, `persist` for cross-run state, `search::Pattern` for `/`-search, `analyze::Analysis` for semantic jumps.
- Purpose: Byte-pattern parser for TUI `/`-search.
- Location: `crates/base60-cli/src/search.rs`
- Contains: `Pattern` newtype (`Vec<u8>`), `FromStr` impl that auto-detects hex vs string, `find_all`.
- Prefix syntax: `hex:`, `str:`, quoted strings, or auto-detect.
- Purpose: Per-file TUI state across runs — scroll, cursor, lens mode, bookmarks.
- Location: `crates/base60-cli/src/persist.rs`
- Storage: `$XDG_STATE_HOME/base60/<fnv1a-hash>.state` (fallback `$HOME/.local/state/base60/…`), plain `key=value` text. Stdin bypasses; write failures silently no-op.
## Data Flow
- CLI paths are stateless — each invocation reads input, renders, exits.
- TUI `ViewState` holds scroll, cursor, mode, bookmarks, search matches, current lens settings, and layout cache.
- Persistence (`persist.rs`) writes only on clean quit; write failures are silently dropped.
## Key Abstractions
- Purpose: Uniform overlay contract. Implementors take a `u64` chunk, return a `String` column to append to the dump line.
- Contract: `Send + Sync` so the same instance can be used from both the sync streaming CLI path and the ratatui draw thread.
- Examples: `TimeLens`, `AngleLens`, `TabletLens`, `CuneiformLens`.
- Constructor glue: `cli::build_lens(LensMode, TimeScale, purist) -> Option<Box<dyn Lens>>` — single factory used by both the flag path (`--lens=…`) and the TUI `L`-key cycle, so the two can never disagree.
- Purpose: ANSI colour source of truth for the text renderers.
- Pattern: a struct of `&'static str` escape codes; `PALETTE_NONE` has every field `""`. Because the writes still happen, but with zero-length slices, the mono path has no runtime branch per token — it's the same code path as ANSI.
- Purpose: Single abstraction over mmap and stdin input.
- Variants: `Mapped { map: Mmap, start, end }` (keeps the mapping alive for the lifetime of any borrowed slice), `Owned(Vec<u8>)` (for stdin).
- Surface: a single `as_slice(&self) -> &[u8]`.
- Variants: `Ansi` (default), `Plain`, `Json`, `Html`.
- Dispatch: single match in `run_view` maps each variant to a writer function (`dump_all` / `emit_json` / `emit_html`).
- Each format shares the same chunking + `u64_to_base60` call; only the per-chunk emission differs.
- Variants: `Analyze(AnalyzeArgs)`, `Decode(DecodeArgs)`, `Completions(CompletionsArgs)`. `None` = default view.
- Dispatch: `main::main` matches once, delegates to `run_view` / `run_analyze` / `run_decode` / `run_completions`.
- Purpose: Parsed byte pattern for TUI `/` search, `FromStr`-driven with `hex:` / `str:` / quoted / auto-detect syntaxes.
- Purpose: Serialised TUI state. Fields: `scroll`, `cursor`, `lens_mode`, `bookmarks: Vec<(char, usize)>`.
## Entry Points
- Location: `crates/base60-cli/src/main.rs`
- Triggers: `cargo run -p base60 -- …`, installed `base60` binary.
- Responsibilities: parse args via `clap::Parser`, dispatch to `run_view` / `run_analyze` / `run_decode` / `run_completions`, translate `BrokenPipe` into clean exit.
- Location: `crates/base60-core/src/lib.rs`
- Triggers: `use base60_core::{…}` from downstream consumers or from the `base60` binary.
- Re-exports: `DIGITS`, `u64_to_base60`, `ascii_fallback_forced`, `ascii_pair`, `glyph`, `Lens`, `AngleLens`, `CuneiformLens`, `TabletLens`, `TimeLens`, `TimeScale`, `DecodeError`, `decode_u64`, `encode_u64`.
- `run_view(&ViewArgs)` — default, dispatches on `Format` and `interactive`.
- `run_analyze(&AnalyzeArgs)` — streams summary to stdout.
- `run_decode(&DecodeArgs)` — parses dump text from file or stdin.
- `run_completions(&CompletionsArgs)` — writes shell completion script via `clap_complete::generate`.
## Error Handling
- Library (`base60-core`): fallible paths use explicit `Result` with a typed error enum (`url::DecodeError`); infallible hot paths use `#[must_use]` and `debug_assert!` to surface contract violations in tests.
- Binary: `anyhow::Result` everywhere in `main` and subcommand handlers; low-level modules (`decode.rs`, `dump.rs`, `format.rs`, `reader.rs`) return `std::io::Result` and let `?` coerce into `anyhow::Error` at the outer boundary.
- `BrokenPipe` is detected on every stdout writer and converted to a clean `Ok(())` — matches `cat`/`grep`/`hexdump` semantics when piped into `head`.
- `mmap` is declared `unsafe` because another process can mutate the backing file; the comment at `crates/base60-cli/src/reader.rs:54` documents that stale bytes on screen is the only visible failure mode for a read-only viewer.
- `clamp_range` in `reader.rs` saturates rather than panicking when `--skip`/`--length` exceed `usize::MAX` or the input length.
- `decode_stream` uses `u128` accumulator to detect overflow before it corrupts the `u64`.
- Persistence write failures in `persist::save` are silently swallowed (`let _ = fs::write(…)`); losing a cursor position is not worth failing a TUI quit.
## Cross-Cutting Concerns
- None. No `log`/`tracing` crate. The TUI renders its own status bar; CLI paths emit only their intended output plus any `anyhow` error on exit.
- `clap` derive macros enforce enum values (`ValueEnum` for `ColorChoice`, `LensMode`, `TimeScale`, `Format`, `Shell`).
- Base-60 digits: `debug_assert!(d < 60)` in `cuneiform::glyph` and `cuneiform::ascii_pair`; release builds rely on the array bounds check.
- Decode validates `digit < 60` at runtime, returning `io::ErrorKind::InvalidData` with line number context.
- Not applicable — offline CLI utility.
- `NO_COLOR` env var → monochrome output (`main::pick_palette`).
- `NO_UNICODE` env var → cuneiform ASCII fallback (`cuneiform::ascii_fallback_forced`).
- `TERM=dumb` → cuneiform ASCII fallback.
- `XDG_STATE_HOME` → TUI per-file state location (`persist::state_base_dir`).
- `HOME` → fallback state path `$HOME/.local/state/base60/…`.
- Zero-copy hot path: `dump::write_line` uses `write_all` on palette `&'static str` escapes, never allocates; mono path falls through to zero-length no-ops.
- `LazyLock` cuneiform glyph table (`crates/base60-core/src/cuneiform.rs`) is built once on first `glyph()` call, returns `&'static str`.
- `mmap` avoids reading large files into memory; stdin slurps into a `Vec<u8>` (unavoidable, no seek).
- Workspace release profile: `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"` (see root `Cargo.toml`).
- All test modules co-located in `#[cfg(test)] mod tests { … }` blocks at the bottom of each source file.
- Library core has deep coverage of round-trips and algebraic invariants; CLI modules test line-level rendering, ANSI presence, format-specific escaping, edge cases (empty input, broken-pipe compatibility, ASCII fallback).
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
