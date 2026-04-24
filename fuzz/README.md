# base60 fuzz crate

This crate hosts libFuzzer targets for `base60-cli` internals surfaced via
the `#[cfg(fuzzing)] pub mod __fuzz` hatch in `crates/base60-cli/src/lib.rs`
(Phase 5 TEST-02, PROJECT.md Key Decision row 7).

## Platform

Ubuntu + pinned nightly only. libFuzzer requires LLVM sanitizer support
(x86_64/aarch64, Unix-like, nightly-only). The main workspace CI matrix
remains Ubuntu/macOS/Windows × stable/beta/1.95 because `fuzz/` is
workspace-excluded (`exclude = ["fuzz"]` in root `Cargo.toml`,
`--fuzzing-workspace=true` in this crate).

See `.planning/research/PITFALLS.md` §Pitfall 11 — cargo-fuzz silently
falls back on macOS/Windows CI; treat any non-Linux run as unsupported.

## Running locally

Prerequisites:

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
```

Smoke (30 s, no-crash gate — TEST-02 SC2):

```bash
cd fuzz
cargo +nightly fuzz run parse_run -- -max_total_time=30
cargo +nightly fuzz run pattern_from_str -- -max_total_time=30
```

Longer runs (reproduce a reported crash, extend coverage):

```bash
cd fuzz
cargo +nightly fuzz run parse_run -- -max_total_time=240
```

Build-only (used by Phase 7 CI `benches-compile`-style smoke):

```bash
cd fuzz
cargo +nightly fuzz build
```

## Targets

| Target              | Surface under test                            | Input guard                |
|---------------------|-----------------------------------------------|----------------------------|
| `parse_run`         | `base60::__fuzz::parse_run` (Phase 4 D-09)    | length-gate + try_from     |
| `pattern_from_str`  | `base60::__fuzz::Pattern::from_str`           | `std::str::from_utf8`      |

Both targets follow the `let _ = ...` pattern: `Err` returns are the
happy path; only panics are bugs (PITFALLS Pitfall 3).

## Seed corpus

Empty on commit (CONTEXT D-09). libFuzzer's coverage-guided mutation
bootstraps quickly on small input shapes. Reassess after two Phase 7
weekly runs — if corpus growth stalls, add `fuzz/seeds/{parse_run,pattern_from_str}/*`.

## CI integration

Not run by this phase. Phase 7 CI-02 adds a weekly `schedule:` workflow
under `.github/workflows/fuzz.yml` that invokes
`cargo +nightly fuzz run <target> -- -max_total_time=240` on
`ubuntu-latest` only, non-gating, 5-minute timeout.
