# Technology Stack

**Analysis Date:** 2026-04-23

## Languages

**Primary:**
- Rust (edition 2024) - All workspace code under `crates/base60-core/src/` and `crates/base60-cli/src/`

**Secondary:**
- Not applicable (pure Rust workspace; no embedded DSLs, build scripts, or non-Rust sources)

## Runtime

**Environment:**
- Native compiled binary via `cargo build`. No interpreter or VM.
- Platform targets exercised in CI: `ubuntu-latest`, `macos-latest`, `windows-latest` (see `.github/workflows/ci.yml`).

**Package Manager:**
- Cargo (bundled with the Rust toolchain)
- Lockfile: `Cargo.lock` present at workspace root (~47KB, 201 resolved packages). CI invokes `--locked` everywhere.

## Frameworks

**Core:**
- `clap` 4.6.1 (features: `derive`) - Argument parsing for the `base60` binary. See `crates/base60-cli/src/cli.rs`.
- `clap_complete` 4.6.1 (resolves to 4.6.2 in lock) - Shell completion script generation for the `completions` subcommand. See `crates/base60-cli/src/main.rs::run_completions`.
- `ratatui` 0.30.0 - Terminal UI framework powering the interactive TUI (`-i` flag). See `crates/base60-cli/src/tui.rs`.
- `crossterm` 0.29.0 - Backend for `ratatui`; raw-mode input, cursor, and color. See `crates/base60-cli/src/tui.rs`.
- `anyhow` 1.0.102 - Top-level error type for the binary. See `crates/base60-cli/src/main.rs`.
- `memmap2` 0.9.10 - Memory-mapped file reads for large inputs in `crates/base60-cli/src/reader.rs`.

**Testing:**
- Rust built-in `#[test]` / `#[cfg(test)]` harness (no external test framework). Inline unit tests live next to the code they exercise (e.g. `tests` module at the bottom of `crates/base60-cli/src/main.rs`). Doc tests are run separately in CI.

**Build/Dev:**
- `cargo` (build, test, doc, install) - sole build tool.
- `rustfmt` - enforced via `cargo fmt --all --check` in CI (`fmt` job).
- `clippy` - enforced via `cargo clippy --workspace --all-targets --locked -- -D warnings` in CI (`clippy` job). Workspace-level lint groups enabled: `clippy::pedantic`, `clippy::nursery`, `clippy::cargo`.
- `rustdoc` - `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS: -D warnings` in CI (`doc` job).

## Key Dependencies

**Critical (declared in `crates/base60-cli/Cargo.toml`):**
- `anyhow` = "1.0.102" - Error aggregation in `main`.
- `clap` = "4.6.1" (features: `derive`) - CLI parser.
- `clap_complete` = "4.6.1" - Shell completion generator.
- `crossterm` = "0.29.0" - Low-level terminal control.
- `memmap2` = "0.9.10" - Mmap-backed input reader.
- `ratatui` = "0.30.0" - TUI layout/widgets.
- `base60-core` = { path = "../base60-core" } - Intra-workspace dep for conversion/lens primitives.

**Critical (declared in `crates/base60-core/Cargo.toml`):**
- None. The core library has zero external dependencies; it builds from `std` alone. Categories declare `no-std` compatibility intent even though the current implementation uses `std::sync::LazyLock` and `String` (see `crates/base60-core/src/lib.rs` module doc).

**Notable transitive (resolved via `Cargo.lock`):**
- `serde` / `serde_derive` / `serde_core` 1.0.228, `serde_json` 1.0.149 - pulled in by `ratatui` / `termwiz` dependency chains; not used directly by workspace code.
- `regex` 1.12.3, `fancy-regex` 0.11.0, `regex-automata` 0.4.14, `regex-syntax` 0.8.10 - transitive (likely via `termwiz`/`ratatui-termwiz`).
- `ratatui-core` 0.1.0, `ratatui-crossterm` 0.1.0, `ratatui-macros` 0.7.0, `ratatui-termwiz` 0.1.0, `ratatui-widgets` 0.3.0 - ratatui's split crate set.
- `thiserror` 1.0.69 and 2.0.18 (both versions co-exist; `clippy::multiple_crate_versions` is explicitly allowed at workspace level).
- `wasm-bindgen` 0.2.118, `wasmparser` / `wasm-encoder` / `wit-*` 0.244.0 - transitive through `termwiz`/`ratatui` ecosystem (terminal color/blob support).
- `windows-sys` 0.61.2, `winapi` 0.3.9, `crossterm_winapi` 0.9.1 - Windows target support for `crossterm`.
- `libc` 0.2.185, `rustix` 1.1.4, `linux-raw-sys` 0.12.1 - Unix syscalls used by `crossterm` / `memmap2`.

**Infrastructure:**
- No runtime infrastructure dependencies (no HTTP client, no DB driver, no async runtime). Pure offline CLI tool.

## Configuration

**Environment:**
- `NO_COLOR` - honored at runtime for auto color detection. See `crates/base60-cli/src/main.rs::pick_palette`. Follows https://no-color.org.
- CI-only env: `CARGO_TERM_COLOR=always`, `RUST_BACKTRACE=1`, `CARGO_INCREMENTAL=0` (set in `.github/workflows/ci.yml`).

**Build:**
- `Cargo.toml` (workspace root) - defines `[workspace]`, `[workspace.package]`, shared `[profile.release]`, and `[workspace.lints.*]`.
  - `resolver = "3"` (new Rust 2024 resolver).
  - `[profile.release]`: `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"` - applied to both crates when installed.
- `crates/base60-core/Cargo.toml` - library crate manifest; inherits `version`/`edition`/`rust-version`/`license`/`repository` from workspace.
- `crates/base60-cli/Cargo.toml` - binary crate manifest; declares `[[bin]] name = "base60" path = "src/main.rs"`.
- No `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, or `.cargo/config.toml` present.

**Workspace lint configuration (`Cargo.toml`):**
- `rust`: `unsafe_op_in_unsafe_fn = warn`, `missing_debug_implementations = warn`, `unreachable_pub = warn`, `rust_2018_idioms = warn`, `unused_lifetimes = warn`, `unused_qualifications = warn`.
- `clippy`: `pedantic = warn`, `nursery = warn`, `cargo = warn`, with explicit allows for `multiple_crate_versions` and `module_name_repetitions`.

## Platform Requirements

**Development:**
- Rust toolchain with `rustc`/`cargo` 1.95.0 or newer (declared `rust-version = "1.95"` in workspace). CI matrix covers `1.95.0`, `stable`, `beta`.
- Edition 2024 support (requires recent-enough toolchain).
- Any OS supported by Rust stdlib + `crossterm` (Linux, macOS, Windows — all three exercised in CI).

**Production:**
- Single statically-linked binary (`base60`) installed via `cargo install --path crates/base60-cli`. Default location `$HOME/.cargo/bin/base60`.
- No service, no daemon. Reads files or stdin, writes to stdout or alternate screen (TUI).
- `publish = false` at workspace level — crates not published to crates.io.

---

*Stack analysis: 2026-04-23*
