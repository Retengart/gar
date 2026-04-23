# Architecture

**Analysis Date:** 2026-04-23

## Pattern Overview

**Overall:** Cargo workspace with a reusable library crate + a thin binary CLI crate on top.

**Key Characteristics:**
- `base60-core` (lib) owns all pure numeric/encoding logic; no I/O, no terminal concerns, no CLI dependencies. Categorised as `no-std`-compatible encoding primitive (Cargo `categories = ["encoding", "no-std"]`) though currently uses `std` for `LazyLock` / `String`.
- `base60` (bin, crate name `base60`, package dir `crates/base60-cli`) owns all user-facing I/O: argument parsing, mmap/stdin reader, dump/format/decode writers, and the interactive TUI.
- Lens-based plugin system: `base60_core::lens::Lens` trait lets any semantic overlay (time, angle, tablet, cuneiform) render alongside the core dump without the renderers knowing the variants.
- Format-agnostic rendering: four output formats (`Ansi`, `Plain`, `Json`, `Html`) share a single byte-chunking + `u64_to_base60` pipeline; palette/format branching happens at the outermost layer of `run_view`.
- Bidirectional: `dump::dump_all` bytes→text; `decode::decode_stream` text→bytes, closing a full roundtrip loop with `base60 --color=never FILE | base60 decode`.

## Layers

**Numeric core (library):**
- Purpose: Convert `u64` ↔ base-60 digits and Sumero-Babylonian glyph table.
- Location: `crates/base60-core/src/convert.rs`, `crates/base60-core/src/cuneiform.rs`
- Contains: `u64_to_base60`, `DIGITS` const, `ascii_pair`, `glyph`, `ascii_fallback_forced`.
- Depends on: `std` only (`LazyLock` for glyph cache).
- Used by: every renderer in `base60-cli`, every lens in `base60-core::lens`.

**Encoding core (library):**
- Purpose: URL-safe 11-char `u64` encoding using unambiguous 60-symbol alphabet (`0-9A-Za-x`).
- Location: `crates/base60-core/src/url.rs`
- Contains: `ALPHABET`, `encode_u64`, `decode_u64`, `DecodeError`.
- Depends on: `convert` module.
- Used by: re-exported at crate root for downstream consumers; not used by the CLI dump/decode paths (those operate on 8-byte chunks, not single `u64`s).

**Lens layer (library):**
- Purpose: Reinterpret a raw `u64` chunk as a piece of Sumero-Babylonian semantics.
- Location: `crates/base60-core/src/lens.rs`
- Contains: `Lens` trait (`Send + Sync`, single `fn render(&self, chunk: u64) -> String`), `TimeLens`, `AngleLens`, `TabletLens`, `CuneiformLens`, `TimeScale` enum.
- Depends on: `convert`, `cuneiform`.
- Used by: `dump::write_line`, `dump::styled_line`, `format::emit_json`, `format::emit_html`, and the TUI `L`-key cycle.

**CLI argument layer (binary):**
- Purpose: Parse command line, build a `Lens` trait object, dispatch subcommand.
- Location: `crates/base60-cli/src/cli.rs`
- Contains: `Cli` (clap `Parser`), `Command` enum (`Analyze`, `Decode`, `Completions`; default = view), `ViewArgs`, `AnalyzeArgs`, `DecodeArgs`, `CompletionsArgs`, `LensMode`, `TimeScale`, `Format`, `ColorChoice`, `build_lens` factory.
- Depends on: `base60_core::lens::*`, `clap`, `clap_complete::Shell`.
- Used by: `main.rs` only.

**Input layer (binary):**
- Purpose: Unified `&[u8]` over mmap or stdin, with `--skip`/`--length` clamping.
- Location: `crates/base60-cli/src/reader.rs`
- Contains: `Bytes` enum (`Mapped { map, start, end }` or `Owned(Vec<u8>)`), `load()`, `clamp_range()`.
- Depends on: `memmap2::Mmap`, `anyhow`.
- Used by: `run_view`, `run_analyze` in `main.rs`. `run_decode` skips this layer and uses `BufRead` directly because dump files are small.

**Rendering layer (binary):**
- Purpose: Turn bytes into a formatted line. Four parallel implementations, one per output format.
- Location:
  - ANSI + plain (text): `crates/base60-cli/src/dump.rs` (`dump_all`, `write_line`)
  - JSON + HTML: `crates/base60-cli/src/format.rs` (`emit_json`, `emit_html`)
  - ratatui spans for TUI: `crates/base60-cli/src/dump.rs` (`styled_line`)
- Each accepts `data: &[u8]`, `base_offset: u64`, a writer, and `Option<&dyn Lens>`.
- Chunk size is fixed at `CHUNK = 8` bytes (one big-endian `u64` → 11 base-60 digits).

**Decode layer (binary):**
- Purpose: Reverse of `dump::dump_all`. Parse `NN:NN:…:NN` runs back into big-endian bytes.
- Location: `crates/base60-cli/src/decode.rs`
- Contains: `decode_stream`, `find_digit_run`, `parse_run`. Scanner ignores surrounding text (offset columns, ASCII columns, ANSI escapes) and uses `not_extended_left`/`not_extended_right` guards to refuse overlapping windows (e.g. 12-pair runs).
- Uses `u128` accumulator to detect overflow before truncating to `u64`.

**Palette layer (binary):**
- Purpose: Single source of truth for the four-tier heatmap colours across CLI and TUI.
- Location: `crates/base60-cli/src/color.rs`
- Contains: `Palette` struct of static string ANSI escapes, `PALETTE_NONE` (all empty), `PALETTE_ANSI`, ratatui `Style` constructors (`digit_style`, `offset_style`, `sep_style`, `delim_style`, `printable_style`, `dot_style`, `lens_style`, `title_style`, `border_style`, `status_style`).
- Tiers: digit 0 = DarkGray, 1–19 = Green, 20–39 = Yellow, 40–59 = Red.

**Analysis layer (binary):**
- Purpose: Statistical summary for `base60 analyze` (entropy, byte histogram, ASCII regions).
- Location: `crates/base60-cli/src/analyze.rs`
- Contains: `Analysis`, `Region`, `RegionKind`, `analyze()`, `write_summary()`, plus `MIN_WINDOW = 64`, `DEFAULT_WINDOW = 256`, entropy thresholds (`LOW_ENTROPY = 1.0`, `HIGH_ENTROPY = 7.5`).

**Interactive viewer (binary):**
- Purpose: Full-screen ratatui TUI with hjkl cursor motion, lens cycling, search, bookmarks, semantic jumps.
- Location: `crates/base60-cli/src/tui.rs` (1180 lines — largest file)
- Depends on: `ratatui`, `crossterm`, `persist` for cross-run state, `search::Pattern` for `/`-search, `analyze::Analysis` for semantic jumps.

**Search module (binary):**
- Purpose: Byte-pattern parser for TUI `/`-search.
- Location: `crates/base60-cli/src/search.rs`
- Contains: `Pattern` newtype (`Vec<u8>`), `FromStr` impl that auto-detects hex vs string, `find_all`.
- Prefix syntax: `hex:`, `str:`, quoted strings, or auto-detect.

**Persistence module (binary):**
- Purpose: Per-file TUI state across runs — scroll, cursor, lens mode, bookmarks.
- Location: `crates/base60-cli/src/persist.rs`
- Storage: `$XDG_STATE_HOME/base60/<fnv1a-hash>.state` (fallback `$HOME/.local/state/base60/…`), plain `key=value` text. Stdin bypasses; write failures silently no-op.

## Data Flow

**Encode (bytes → base-60 dump), default subcommand:**

1. `main::main` parses `cli::Cli`; absence of subcommand selects `run_view`.
2. `reader::load(view.file, view.skip, view.length)` returns a `Bytes` enum wrapping either a `Mmap` slice or an owned `Vec<u8>` (stdin). `clamp_range` saturates `--skip`/`--length` against the input size.
3. `cli::build_lens(view.lens, view.time_scale, view.purist)` constructs `Option<Box<dyn Lens>>`.
4. `main::pick_palette(view.color, stdout.is_terminal())` resolves `ColorChoice` against `NO_COLOR` env and TTY state, returning `&PALETTE_ANSI` or `&PALETTE_NONE`.
5. Match on `view.format`:
   - `Ansi` → `dump::dump_all` with chosen palette.
   - `Plain` → `dump::dump_all` with `PALETTE_NONE` (forces mono regardless of `--color`).
   - `Json` → `format::emit_json`.
   - `Html` → `format::emit_html` (prologue + body + epilogue).
6. Each renderer iterates `data.chunks(CHUNK=8)`, pads short chunks with zeros, calls `be_u64`, then `u64_to_base60`, and emits one output line per chunk to a `BufWriter<StdoutLock>`.
7. `BrokenPipe` errors are swallowed so piping into `head` doesn't produce spurious failures.

**Decode (dump → bytes), `base60 decode` subcommand:**

1. `main::run_decode` opens file or locks stdin, wraps in `BufReader`.
2. `decode::decode_stream` reads line-by-line with `BufRead::lines()`.
3. Each line passes through `find_digit_run`, which scans a sliding window of length `RUN_LEN = 2*DIGITS + (DIGITS-1) = 32` chars looking for `NN:NN:…:NN` with `not_extended_{left,right}` guards to reject over-length runs.
4. Matched run goes to `parse_run`: each `NN` pair → digit in `0..60` (rejects `≥60`); accumulator uses `u128 * 60 + digit`; final `u64::try_from` detects overflow.
5. Result emitted as 8 big-endian bytes via `w.write_all(&value.to_be_bytes())`.
6. Lines without a matching run are silently skipped (matches `xxd -r` behaviour on mixed input).

**Analyze, `base60 analyze` subcommand:**

1. `main::run_analyze` loads bytes identically to `run_view`.
2. `analyze::analyze` does two passes: one for global histogram + Shannon entropy, one for per-window entropies (skipping trailing partial window).
3. `detect_regions` produces disjoint `Region`s (ASCII runs ≥ 4 chars, plus window-aligned `HighEntropy`/`LowEntropy` tiers), sorted by start offset.
4. `write_summary` prints plain-text report (no ANSI) with byte totals, entropy stats, top-5 byte histogram, region tally, and first 5 ASCII previews.

**Completions, `base60 completions` subcommand:**

1. `main::run_completions` calls `cli::Cli::command()` (via clap's `CommandFactory`) to materialise the full `clap::Command` tree.
2. `clap_complete::generate(args.shell, &mut cmd, bin_name, &mut stdout.lock())` writes the shell's completion script to stdout.

**Interactive (TUI), `--interactive` / `-i` flag:**

1. `run_view` detects `view.interactive` and branches into `tui::run` before the format switch.
2. `tui::run` builds `ViewState`, restores persisted state if the input was a file, enters the ratatui draw/read loop with `event::read()`.
3. On quit (`q`), state is persisted back to disk.

**State Management:**
- CLI paths are stateless — each invocation reads input, renders, exits.
- TUI `ViewState` holds scroll, cursor, mode, bookmarks, search matches, current lens settings, and layout cache.
- Persistence (`persist.rs`) writes only on clean quit; write failures are silently dropped.

## Key Abstractions

**`Lens` trait (`crates/base60-core/src/lens.rs`):**
- Purpose: Uniform overlay contract. Implementors take a `u64` chunk, return a `String` column to append to the dump line.
- Contract: `Send + Sync` so the same instance can be used from both the sync streaming CLI path and the ratatui draw thread.
- Examples: `TimeLens`, `AngleLens`, `TabletLens`, `CuneiformLens`.
- Constructor glue: `cli::build_lens(LensMode, TimeScale, purist) -> Option<Box<dyn Lens>>` — single factory used by both the flag path (`--lens=…`) and the TUI `L`-key cycle, so the two can never disagree.

**`Palette` struct (`crates/base60-cli/src/color.rs`):**
- Purpose: ANSI colour source of truth for the text renderers.
- Pattern: a struct of `&'static str` escape codes; `PALETTE_NONE` has every field `""`. Because the writes still happen, but with zero-length slices, the mono path has no runtime branch per token — it's the same code path as ANSI.

**`Bytes` enum (`crates/base60-cli/src/reader.rs`):**
- Purpose: Single abstraction over mmap and stdin input.
- Variants: `Mapped { map: Mmap, start, end }` (keeps the mapping alive for the lifetime of any borrowed slice), `Owned(Vec<u8>)` (for stdin).
- Surface: a single `as_slice(&self) -> &[u8]`.

**`Format` enum (`crates/base60-cli/src/cli.rs`):**
- Variants: `Ansi` (default), `Plain`, `Json`, `Html`.
- Dispatch: single match in `run_view` maps each variant to a writer function (`dump_all` / `emit_json` / `emit_html`).
- Each format shares the same chunking + `u64_to_base60` call; only the per-chunk emission differs.

**`Command` enum (`crates/base60-cli/src/cli.rs`):**
- Variants: `Analyze(AnalyzeArgs)`, `Decode(DecodeArgs)`, `Completions(CompletionsArgs)`. `None` = default view.
- Dispatch: `main::main` matches once, delegates to `run_view` / `run_analyze` / `run_decode` / `run_completions`.

**`Pattern` newtype (`crates/base60-cli/src/search.rs`):**
- Purpose: Parsed byte pattern for TUI `/` search, `FromStr`-driven with `hex:` / `str:` / quoted / auto-detect syntaxes.

**`PersistedState` record (`crates/base60-cli/src/persist.rs`):**
- Purpose: Serialised TUI state. Fields: `scroll`, `cursor`, `lens_mode`, `bookmarks: Vec<(char, usize)>`.

## Entry Points

**Binary entry point:**
- Location: `crates/base60-cli/src/main.rs`
- Triggers: `cargo run -p base60 -- …`, installed `base60` binary.
- Responsibilities: parse args via `clap::Parser`, dispatch to `run_view` / `run_analyze` / `run_decode` / `run_completions`, translate `BrokenPipe` into clean exit.

**Library root:**
- Location: `crates/base60-core/src/lib.rs`
- Triggers: `use base60_core::{…}` from downstream consumers or from the `base60` binary.
- Re-exports: `DIGITS`, `u64_to_base60`, `ascii_fallback_forced`, `ascii_pair`, `glyph`, `Lens`, `AngleLens`, `CuneiformLens`, `TabletLens`, `TimeLens`, `TimeScale`, `DecodeError`, `decode_u64`, `encode_u64`.

**Subcommand handlers (all in `crates/base60-cli/src/main.rs`):**
- `run_view(&ViewArgs)` — default, dispatches on `Format` and `interactive`.
- `run_analyze(&AnalyzeArgs)` — streams summary to stdout.
- `run_decode(&DecodeArgs)` — parses dump text from file or stdin.
- `run_completions(&CompletionsArgs)` — writes shell completion script via `clap_complete::generate`.

## Error Handling

**Strategy:**
- Library (`base60-core`): fallible paths use explicit `Result` with a typed error enum (`url::DecodeError`); infallible hot paths use `#[must_use]` and `debug_assert!` to surface contract violations in tests.
- Binary: `anyhow::Result` everywhere in `main` and subcommand handlers; low-level modules (`decode.rs`, `dump.rs`, `format.rs`, `reader.rs`) return `std::io::Result` and let `?` coerce into `anyhow::Error` at the outer boundary.

**Patterns:**
- `BrokenPipe` is detected on every stdout writer and converted to a clean `Ok(())` — matches `cat`/`grep`/`hexdump` semantics when piped into `head`.
- `mmap` is declared `unsafe` because another process can mutate the backing file; the comment at `crates/base60-cli/src/reader.rs:54` documents that stale bytes on screen is the only visible failure mode for a read-only viewer.
- `clamp_range` in `reader.rs` saturates rather than panicking when `--skip`/`--length` exceed `usize::MAX` or the input length.
- `decode_stream` uses `u128` accumulator to detect overflow before it corrupts the `u64`.
- Persistence write failures in `persist::save` are silently swallowed (`let _ = fs::write(…)`); losing a cursor position is not worth failing a TUI quit.

## Cross-Cutting Concerns

**Logging:**
- None. No `log`/`tracing` crate. The TUI renders its own status bar; CLI paths emit only their intended output plus any `anyhow` error on exit.

**Validation:**
- `clap` derive macros enforce enum values (`ValueEnum` for `ColorChoice`, `LensMode`, `TimeScale`, `Format`, `Shell`).
- Base-60 digits: `debug_assert!(d < 60)` in `cuneiform::glyph` and `cuneiform::ascii_pair`; release builds rely on the array bounds check.
- Decode validates `digit < 60` at runtime, returning `io::ErrorKind::InvalidData` with line number context.

**Authentication:**
- Not applicable — offline CLI utility.

**Environment sensing:**
- `NO_COLOR` env var → monochrome output (`main::pick_palette`).
- `NO_UNICODE` env var → cuneiform ASCII fallback (`cuneiform::ascii_fallback_forced`).
- `TERM=dumb` → cuneiform ASCII fallback.
- `XDG_STATE_HOME` → TUI per-file state location (`persist::state_base_dir`).
- `HOME` → fallback state path `$HOME/.local/state/base60/…`.

**Performance:**
- Zero-copy hot path: `dump::write_line` uses `write_all` on palette `&'static str` escapes, never allocates; mono path falls through to zero-length no-ops.
- `LazyLock` cuneiform glyph table (`crates/base60-core/src/cuneiform.rs`) is built once on first `glyph()` call, returns `&'static str`.
- `mmap` avoids reading large files into memory; stdin slurps into a `Vec<u8>` (unavoidable, no seek).
- Workspace release profile: `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"` (see root `Cargo.toml`).

**Testing:**
- All test modules co-located in `#[cfg(test)] mod tests { … }` blocks at the bottom of each source file.
- Library core has deep coverage of round-trips and algebraic invariants; CLI modules test line-level rendering, ANSI presence, format-specific escaping, edge cases (empty input, broken-pipe compatibility, ASCII fallback).

---

*Architecture analysis: 2026-04-23*
