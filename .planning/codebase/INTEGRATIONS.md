# External Integrations

**Analysis Date:** 2026-04-23

This is an offline CLI tool with a deliberately minimal external surface. It ships a single binary that reads local files (or stdin) and writes to stdout or the terminal's alternate screen. Most integration categories below are **Not applicable**; the entries that exist are documented honestly.

## APIs & External Services

**None.** The workspace makes no network calls. There is no HTTP client, SDK, or RPC library in `Cargo.toml` or `Cargo.lock` workspace manifests. See `crates/base60-cli/Cargo.toml` and `crates/base60-core/Cargo.toml`.

## Data Storage

**Databases:**
- None. No ORM, no DB driver, no embedded SQL engine in dependencies.

**File Storage:**
- Local filesystem only. Input is read from a file path argument or stdin.
  - Input reader: `crates/base60-cli/src/reader.rs` (uses `memmap2` 0.9.10 for mmap-backed reads of large files).
  - Decode input: `crates/base60-cli/src/decode.rs` (uses `std::io::BufRead` directly).
- Persisted TUI state (cursor/scroll/bookmarks per source file): `crates/base60-cli/src/persist.rs`. Stored on local disk; location is computed from the input file path.

**Caching:**
- None.

## Authentication & Identity

**Not applicable.** No user identity, no session, no credential handling.

## Monitoring & Observability

**Error Tracking:**
- None. Errors surface via `anyhow::Error` and the process exit code.

**Logs:**
- No logging framework. The TUI and dumper write user-facing output directly to stdout/stderr via `BufWriter`.
- `BrokenPipe` errors are swallowed to match the convention of `cat`/`grep`/`hexdump` (see `crates/base60-cli/src/main.rs`).

**Metrics:**
- None.

## CI/CD & Deployment

**Hosting:**
- No hosted service. Distributed as source; users install via `cargo install --path crates/base60-cli`.
- No published crates.io release (`publish = false` in `Cargo.toml` workspace package settings).

**CI Pipeline:**
- GitHub Actions. Single workflow file: `.github/workflows/ci.yml`.
- Triggers: `push` to `main`, `pull_request` targeting `main`. Concurrency cancels superseded PR runs but not main-branch runs.
- Jobs:
  - `test`: matrix over `{ubuntu-latest, macos-latest, windows-latest} × {1.95.0, stable, beta}`. Runs `cargo test --workspace --all-targets --locked` followed by doc tests.
  - `clippy`: `cargo clippy --workspace --all-targets --locked -- -D warnings` on ubuntu.
  - `fmt`: `cargo fmt --all --check` on ubuntu.
  - `doc`: `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS: -D warnings`.
  - `release-build`: `cargo build --release --locked` across the three OSes.
- GitHub Actions used:
  - `actions/checkout@v4`
  - `dtolnay/rust-toolchain@master` / `@stable`
  - `Swatinem/rust-cache@v2`
- No release/publish job, no artifact upload, no deploy step.

## Package Registries

**Consumer-side:**
- `crates.io` via `cargo` — source for all external dependencies listed in `Cargo.lock` (e.g. `clap`, `ratatui`, `crossterm`, `memmap2`, `anyhow`, `clap_complete`).

**Publisher-side:**
- Not applicable. Both `base60` (CLI) and `base60-core` (library) have `publish = false` (inherited from workspace package config in `Cargo.toml`). Consumers of `base60-core` must currently use a `path = "../base60-core"` dependency (per `README.md`).

## Environment Configuration

**Required env vars:**
- None are required to run the tool.

**Optional env vars read at runtime:**
- `NO_COLOR` - disables ANSI color output when set to a non-empty value (`crates/base60-cli/src/main.rs::pick_palette`).

**CI-only env vars (`.github/workflows/ci.yml`):**
- `CARGO_TERM_COLOR=always`, `RUST_BACKTRACE=1`, `CARGO_INCREMENTAL=0`.

**Secrets location:**
- None. No `.env` file, no secret store, no GitHub Actions secrets referenced.

## Webhooks & Callbacks

**Incoming:**
- None.

**Outgoing:**
- None.

## Repository Metadata

- Repo URL (declared in `Cargo.toml` `[workspace.package]`): `https://github.com/retengart/test-60`.
- License: `MIT OR Apache-2.0` (dual).

---

*Integration audit: 2026-04-23*
