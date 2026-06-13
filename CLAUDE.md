# gar — Sumero-Babylonian binary viewer

`gar` is a hex-dump alternative that renders every 8 bytes of input as eleven
sexagesimal (base-60) digit pairs. Ships a coloured TTY dump, four optional
lenses (including actual cuneiform), entropy-based statistical analysis,
roundtrip decoding, and a ratatui-based interactive TUI.

**Core invariant:** `gar FILE | gar decode` round-trips byte-identical.

## Quick start

```sh
cargo install --path crates/gar-cli
gar some-binary-file          # coloured base-60 dump
gar -i some-binary-file       # interactive TUI
gar --lens=cuneiform FILE     # cuneiform overlay
gar --format=json FILE        # JSON output
gar FILE | gar decode         # roundtrip back to bytes
```

## Workspace layout

| Crate | Path | Purpose |
|-------|------|---------|
| `gar-core` | `crates/gar-core` | Pure library: `u64_to_base60`, lenses, cuneiform glyphs, URL-safe encoding. Zero external deps. |
| `gar` | `crates/gar-cli` | CLI binary: dump, decode, analyze, TUI, completions. |
| `xtask` | `crates/xtask` | CI/dev automation (not published). |

## Constraints

- **Rust edition 2024**, MSRV `1.95`. Single statically-linked binary via `cargo install`.
- **`gar-core` has zero external dependencies** — its selling point.
- **JSON schema and `decode` accept-format are stable.** Any change must be additive.
- **Output determinism:** `NO_COLOR`, `NO_UNICODE`, `TERM=dumb` are contractual.
- **CI matrix:** Ubuntu/macOS/Windows × rustc 1.95/stable/beta.
- **Lint bar:** `clippy::pedantic + nursery + cargo` with `-D warnings`.
  Only `multiple_crate_versions` and `module_name_repetitions` are allowed.

## Architecture

```
gar-core (lib)                   gar (bin)
┌─────────────────────┐          ┌──────────────────────────┐
│ convert.rs          │          │ cli.rs      (clap parser)│
│   u64_to_base60     │◄────────│ reader.rs   (mmap/stdin) │
│ cuneiform.rs        │          │ dump.rs     (ANSI/plain) │
│   glyph table       │          │ format.rs   (JSON/HTML)  │
│ lens.rs             │          │ decode.rs   (text→bytes) │
│   Time/Angle/       │          │ analyze.rs  (entropy)    │
│   Tablet/Cuneiform  │          │ tui.rs      (ratatui)    │
│ url.rs              │          │ search.rs   (byte patt.) │
│   encode/decode_u64 │          │ color.rs    (heatmap)    │
└─────────────────────┘          │ persist.rs  (xdev state) │
                                 └──────────────────────────┘
```

### Key design decisions

- **Lens trait** (`Send + Sync`): `fn render(&self, chunk: u64) -> String`.
  Single factory `cli::build_lens()` used by both CLI flags and TUI `L`-key cycle.
- **Four output formats** (`Ansi`, `Plain`, `Json`, `Html`) share chunking + `u64_to_base60`.
- **Palette pattern:** struct of `&'static str` ANSI escapes; mono path uses zero-length
  strings — same code, no branch.
- **Chunk = 8 bytes** → one big-endian `u64` → 11 base-60 digits.
- **`BrokenPipe`** → clean `Ok(())` on every stdout writer.
- **Decode scanner** ignores surrounding text (offsets, ANSI, ASCII columns) and uses
  `u128` accumulator to detect overflow before truncation.

## Module guide

### gar-core

| Module | Purpose |
|--------|---------|
| `convert` | `u64_to_base60`, `DIGITS`, `ascii_pair`, `ascii_fallback_forced` |
| `cuneiform` | `glyph()` — 60 Sumero-Babylonian Unicode characters via `LazyLock` |
| `lens` | `Lens` trait, `TimeLens`, `AngleLens`, `TabletLens`, `CuneiformLens`, `TimeScale` |
| `url` | URL-safe 11-char `u64` encoding; `ALPHABET = 0-9A-Za-x`; `DecodeError` |

### gar (CLI)

| Module | Purpose |
|--------|---------|
| `cli` | clap `Parser`, `Command` enum, `ViewArgs`/`AnalyzeArgs`/`DecodeArgs`, `build_lens` |
| `reader` | `Bytes` enum (mmap/owned), `load()`, `clamp_range()` |
| `dump` | `write_line`, `styled_line`, `dump_all` — streaming ANSI/plain renderer |
| `format` | `emit_json`, `emit_html` — structured output |
| `decode` | `decode_stream`, `find_digit_run`, `parse_run` — reverse of dump |
| `analyze` | `Analysis`, `Region`, `RegionKind`, entropy calculation |
| `tui` | Full-screen ratatui: hjkl motion, lens cycling, `/`-search, bookmarks, semantic jumps |
| `search` | `Pattern` newtype, `FromStr` with `hex:`/`str:`/auto-detect, `find_all` |
| `persist` | Per-file state at `$XDG_STATE_HOME/gar/<fnv1a>.state` |
| `color` | `Palette` struct, `PALETTE_NONE`/`PALETTE_ANSI`, ratatui `Style` constructors |

## Error handling

- **Library:** `Result` + typed `url::DecodeError`; infallible hot paths use `#[must_use]` + `debug_assert!`.
- **Binary:** `anyhow::Result`; low-level modules return `io::Result`.
- **BrokenPipe** → clean exit (matches `cat`/`grep` semantics).
- **Persistence write failures** silently swallowed — losing cursor pos is not worth a TUI crash.

## Panic & unsafe policy

- `unwrap()`/`expect()` only in `#[cfg(test)]`.
- `debug_assert!` for contracts that would be UB-equivalent in release.
- No `todo!`, `unimplemented!`, `unreachable!` in shipped code.
- Single `unsafe` site: `mmap` in `reader.rs` (documented: stale bytes only failure mode).
- `unsafe_op_in_unsafe_fn = "warn"` enforced workspace-wide.

## Environment variables

| Variable | Effect |
|----------|--------|
| `NO_COLOR` | Monochrome output ([no-color.org](https://no-color.org)) |
| `NO_UNICODE` | Cuneiform ASCII fallback |
| `TERM=dumb` | Cuneiform ASCII fallback |
| `XDG_STATE_HOME` | TUI per-file state location |
| `HOME` | Fallback state path `$HOME/.local/state/gar/…` |

## Conventions

- `#[inline]` on hot-path helpers returning small structs.
- `#[must_use]` on every pure public function.
- Canonical derive: `Copy, Clone, Debug, Default, Eq, PartialEq`.
- Doc comments: first sentence is complete imperative summary; `# Errors` / `# Panics` sections.
- All tests co-located in `#[cfg(test)] mod tests { … }` at bottom of each source file.
- No external deps in `gar-core`. Binary crate uses `anyhow`, `clap`, `crossterm`, `ratatui`, `memmap2`, `memchr`.

## Building & testing

```sh
cargo test --workspace                    # all tests
cargo clippy --workspace --all-targets    # lints
cargo doc --workspace --no-deps           # docs
cargo build --release                     # optimised binary (thin LTO)
```

## Publishing

```sh
cargo publish -p gar-core   # publish library first
cargo publish -p gar        # then the binary
```
