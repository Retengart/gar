# Stack Research — v2 Hardening Milestone

**Domain:** Rust workspace CLI hardening (integration tests, fuzzing, benchmarks, refactors)
**Researched:** 2026-04-24
**Confidence:** HIGH (every version verified via crates.io API + context7 + official docs)
**Scope note:** Incremental additions for `base60` v2 only. Existing runtime deps (`clap` 4.6, `ratatui` 0.30, `crossterm` 0.29, `anyhow` 1.0, `memmap2` 0.9) are locked and out of scope.

## TL;DR Recommendation

Add eight dev-dependencies, zero runtime dependencies. `base60-core` stays zero-dep. All new crates are MSRV 1.95-compatible, most are already transitively in `Cargo.lock`.

| Need | Crate | Version | Goes in | Transitive? |
|------|-------|---------|---------|-------------|
| CLI integration tests | `assert_cmd` | `2.2` | `base60-cli` dev-deps | No |
| Predicate assertions | `predicates` | `3.1` | `base60-cli` dev-deps | No (but auto-pulled by `assert_cmd`) |
| Tempdirs for fixtures | `tempfile` | `3.27` | `base60-cli` dev-deps | No (but auto-pulled by `assert_cmd`) |
| Env-var test serialisation | `serial_test` | `3.4` | `base60-cli` dev-deps | No |
| Enum-driven lens dispatch | `strum` + `strum_macros` | `0.27` | **workspace** dep (both crates) | **Yes — 0.27.2 already** |
| Byte search | `memchr` | `2.8` | `base60-cli` runtime dep | **Yes — 2.8.0 already** |
| Benchmarks | `criterion` | `0.8` | `base60-cli` dev-deps | No |
| Fuzzing | `cargo-fuzz` (tool) + `libfuzzer-sys` + `arbitrary` | `0.13` / `0.4` / `1.4` | Separate `fuzz/` project, NOT workspace members | No |

## Core Technologies

### Integration Testing Trio

| Crate | Version | Purpose | Why |
|-------|---------|---------|-----|
| `assert_cmd` | `2.2.1` | Spawn `base60` binary, assert on exit code / stdout / stderr | De facto standard for Rust CLI integration tests; MSRV 1.85; maintained by `assert-rs` org; 59M downloads |
| `predicates` | `3.1.4` | Composable assertions for `assert_cmd` (`predicate::str::contains`, `predicate::str::is_match`) | Auto-depended by `assert_cmd`; MSRV 1.74; trim default features to shed `regex`/`float-cmp`/`difflib` we don't need |
| `tempfile` | `3.27.0` | Per-test scratch dirs for fixture binaries (ELF/PNG/ZIP) and `XDG_STATE_HOME` redirection | Auto-depended by `assert_cmd`; MSRV 1.63; `TempDir::path()` returns owned `PathBuf`, auto-cleans on drop |

**Feature flags (rationale):**

```toml
# crates/base60-cli/Cargo.toml
[dev-dependencies]
assert_cmd = { version = "2.2", default-features = false }          # drop color/anstream
predicates = { version = "3.1", default-features = false, features = ["diff"] }  # keep diff, drop regex/float-cmp/color
tempfile   = { version = "3.27", default-features = false, features = ["getrandom"] }  # default minus nightly
```

`diff` keeps human-readable failure output; `regex`/`float-cmp` are dead weight for byte-oriented assertions. `color` is irrelevant in test runners. `getrandom` is the default `tempfile` feature and what the `rand_v` path suffix uses.

### Env-var Test Serialisation

| Crate | Version | Purpose | Why |
|-------|---------|---------|-----|
| `serial_test` | `3.4.0` | Replace the `SAFETY: don't run concurrently` convention in `cuneiform.rs:151-161` and `main.rs:183-219` with `#[serial]` / `#[serial(env)]` | Only mature option in this niche; MSRV 1.68; 107M downloads; keyed serialisation lets env tests run in parallel with non-env tests |

**API:** `#[serial]` alone = global lock; `#[serial(key)]` = lock scoped to `key`. `#[file_serial]` uses file locks and works across processes (relevant for `cargo test --doc`). `#[parallel]` marks tests explicitly safe to run concurrently even with `#[serial]` siblings — no in-scope need but useful to know.

**Feature flags:**

```toml
serial_test = { version = "3.4", default-features = false }
```

Default pulls in `logging` (+ `log`) and `async` (+ `futures-executor`, `futures-util`). We need neither — env-touching tests are synchronous and we don't log from tests. Stripping defaults drops ~5 transitive crates.

**No-std note:** `serial_test` requires `std` (uses `parking_lot`, `scc`). Irrelevant here — this is a CLI-side dev-dep; `base60-core` stays untouched.

### Enum-driven Dispatch

| Crate | Version | Purpose | Why |
|-------|---------|---------|-----|
| `strum` | `0.27.2` (already in `Cargo.lock`) | Trait crate — provides `IntoEnumIterator`, `VariantNames`, `EnumCount` | Transitively present via clap/ratatui chain; MSRV 1.71; zero cost to use the copy we already compile |
| `strum_macros` | `0.27.2` (already in `Cargo.lock`) | Derive macros — `EnumIter`, `VariantNames`, `Display`, `EnumString` | Same — already transitively compiled |

**CRITICAL:** `strum` 0.27 is already in the dep graph (`grep -A1 '^name = "strum"' Cargo.lock` → `version = "0.27.2"`). Pinning to `0.27` avoids a second major version being resolved. `0.28.0` exists (released 2026-02-22) but adopting it would force transitive upgrades or invite `clippy::multiple_crate_versions` noise on top of the ones we already allow.

**Recommendation:** Declare at workspace level so both `base60-core` (if it ever needs it — currently doesn't) and `base60-cli` share the pin.

```toml
# Cargo.toml (workspace root) — NEW section
[workspace.dependencies]
strum        = "0.27"
strum_macros = "0.27"

# crates/base60-cli/Cargo.toml
[dependencies]
strum        = { workspace = true, features = ["derive"] }
strum_macros = { workspace = true }
```

The `derive` feature on `strum` re-exports `strum_macros` so consumers only need `use strum::EnumIter`. Idiomatic for edition 2024.

**Pattern for REF-02 (drive `LensMode` from a table):**

```rust
use strum::{EnumIter, IntoEnumIterator, VariantNames};

#[derive(Debug, Clone, Copy, EnumIter, VariantNames, strum::Display, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum LensMode { None, Time, Angle, Tablet, Cuneiform }

// cli.rs: LensMode::VARIANTS  → &["none", "time", "angle", "tablet", "cuneiform"]
// persist.rs: LensMode::from_str(s) → parse; l.to_string() → serialise
// Anywhere needing all-variant dispatch: LensMode::iter()
```

This collapses all four parallel switch statements called out in `PROJECT.md:96-97` into one derive site.

### Byte Search

| Crate | Version | Purpose | Why |
|-------|---------|---------|-----|
| `memchr` | `2.8.0` (already in `Cargo.lock`) | SIMD-accelerated substring search replacing naive `windows().position()` in `search::find_all` | Already transitively compiled via `regex`/`ratatui` chain; MSRV 1.61; de facto standard |

**Current module/API (verified via docs.rs):** `memchr::memmem::find_iter(haystack, needle)` returns `FindIter<'_, '_>` — non-overlapping forward matches. For repeated searches with the same needle across different haystacks, build a `memchr::memmem::Finder::new(needle)` once and call `finder.find_iter(haystack)`. The search target in `base60-cli/src/search.rs` (lookup panel, `n`/`N` navigation) falls squarely in the "reuse finder" bucket.

**Declaration:**

```toml
# crates/base60-cli/Cargo.toml — promote from transitive to direct
[dependencies]
memchr = { version = "2.8", default-features = false, features = ["std"] }
```

No new compilation cost — exact version already in `Cargo.lock`. Explicit dependency fails CI (`--locked`) if upstream drops it.

### Benchmarking

**Recommendation: `criterion` 0.8, not `divan`.**

| Crate | Version | Purpose | Why |
|-------|---------|---------|-----|
| `criterion` | `0.8.2` | Micro-benchmarks gating PERF-01 through PERF-05 (stdin streaming, `memchr::memmem`, streaming lens render, streaming entropy sparkline) | Mature, statistics-driven, rich HTML reports; `cargo_bench_support` flag means it works without nightly; MSRV 1.86 (compatible with our 1.95 floor) |

**Why not `divan` (0.1.21):**

- Divan's API is genuinely nicer (generic/const-parameter benchmarks, tree output). CodSpeed recommends it going forward.
- But our bench surface is narrow (~6 benches gating specific perf deltas), statistics-driven regression detection is exactly what we want (PERF-06 is a "guardrail, not user feature"), and criterion's save-and-compare (`cargo bench -- --save-baseline main`) is the single feature that makes bench-in-CI actually useful.
- Divan is pre-1.0 (0.1.21, last release 2025-04) vs criterion's 0.8.2 with an active fork under `criterion-rs` org taking over from the original maintainer — ecosystem momentum currently favours the stable choice.
- Ratatui/clap chain already pulls `plotters`/`rayon` adjacents — criterion doesn't import entirely new leaves.

**Feature flags:**

```toml
[dev-dependencies]
criterion = { version = "0.8", default-features = false, features = ["cargo_bench_support", "html_reports"] }

[[bench]]
name    = "dump"
harness = false

[[bench]]
name    = "decode"
harness = false

[[bench]]
name    = "search"
harness = false
```

Default features include `rayon` and `plotters`. Dropping `rayon` avoids parallelism noise in measurements of single-threaded streaming code (stdin dump, search). `html_reports` kept because baseline comparison UX lives there. `cargo_bench_support` is required for `cargo bench` to find benches without `#![feature(test)]`.

**Location:** `crates/base60-cli/benches/*.rs`. Criterion benches go in the CLI crate (where the perf-sensitive code lives — `dump.rs`, `search.rs`, `decode.rs`, `analyze.rs`). `base60-core` benches could live later under `crates/base60-core/benches/` if convert/lens hot paths need tracking — not needed for v2.

### Fuzzing

**Recommendation: `cargo-fuzz` tool + `libfuzzer-sys` harness + `arbitrary` for structured inputs.**

| Component | Version | Purpose | Why |
|-----------|---------|---------|-----|
| `cargo-fuzz` (installed tool) | `0.13.1` | `cargo fuzz init`, `cargo fuzz run`, corpus management | De facto standard; libFuzzer is still the default engine; nightly required because libFuzzer needs the `-Zsanitizer` unstable flag |
| `libfuzzer-sys` | `0.4.12` | The `fuzz_target!` macro; FFI shim to libFuzzer | Default engine; default `link_libfuzzer` feature links the vendored libFuzzer — no system libFuzzer needed |
| `arbitrary` | `1.4.2` | `Arbitrary` derive for structured input generation if `parse_run`'s raw `&[u8]` input isn't expressive enough | Opt-in: the CONCERNS-driven targets (`decode::parse_run`, `search::Pattern::from_str`) both take `&[u8]`/`&str`, so `arbitrary` may be unnecessary — start without it |

**Nightly status (verified):** Yes, still nightly-only as of 2026-04. libFuzzer relies on LLVM sanitizer support exposed via `-Zsanitizer=address`, which remains unstable. Works on x86-64 and aarch64 Linux/macOS; Windows is NOT supported for libFuzzer-backed fuzzing. This means the fuzz CI job must be Linux-only (our main CI matrix stays untouched).

**Workspace layout (two-crate workspace with fuzzing):**

```
test-60/
├── Cargo.toml                     # workspace (unchanged)
├── crates/
│   ├── base60-core/               # library (zero-dep, fuzz targets point at its public API)
│   └── base60-cli/                # binary
└── fuzz/                          # NEW — created by `cd crates/base60-cli && cargo fuzz init`
    ├── Cargo.toml                 # fuzz crate, NOT a workspace member
    ├── .gitignore                 # auto-generated
    ├── fuzz_targets/
    │   ├── parse_run.rs           # TEST-02: decode::parse_run
    │   └── pattern_from_str.rs    # TEST-02: search::Pattern::from_str
    ├── corpus/                    # persisted inputs
    └── artifacts/                 # crash reproducers
```

**Critical workspace-isolation decision:** Run `cargo fuzz init --fuzzing-workspace=true`. This makes `fuzz/Cargo.toml` declare its own `[workspace]` (not a member of ours), which:

1. Prevents `cargo-fuzz`-specific profile settings (debug assertions, coverage instrumentation) from leaking into the main workspace's `Cargo.lock`.
2. Keeps the main workspace's MSRV 1.95 floor and stable CI matrix unaffected — the `fuzz/` crate can freely require nightly.
3. Satisfies the Key Decision in `PROJECT.md:127`: "Fuzz targets gated by `cargo-fuzz`, not pulled into default workspace."

**Adding to the main workspace's `exclude` list:**

```toml
# Cargo.toml (root) — NEW
[workspace]
resolver = "3"
members  = ["crates/base60-core", "crates/base60-cli"]
exclude  = ["fuzz"]                                          # NEW
```

**Fuzz crate dependencies (`fuzz/Cargo.toml`, hand-edited after init):**

```toml
[package]
name         = "base60-fuzz"
version      = "0.0.0"
edition      = "2024"
publish      = false
rust-version = "1.95"                                        # or drop — fuzz builds use nightly anyway

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = { version = "0.4", default-features = false, features = ["link_libfuzzer"] }
base60-core   = { path = "../crates/base60-core" }
base60        = { path = "../crates/base60-cli", package = "base60" }

[[bin]]
name    = "parse_run"
path    = "fuzz_targets/parse_run.rs"
test    = false
doc     = false
bench   = false

[[bin]]
name    = "pattern_from_str"
path    = "fuzz_targets/pattern_from_str.rs"
test    = false
doc     = false
bench   = false
```

Note: fuzzing `base60-cli` internals requires those internals to be `pub` (or `pub(crate)` lifted to `pub` behind `#[cfg(fuzzing)]` — `cargo-fuzz` sets `--cfg fuzzing` on every compilation unit in the graph, so conditional exposure is clean).

**Invocation:**

```bash
cargo +nightly fuzz run parse_run           # single process, default timeout
cargo +nightly fuzz run parse_run -- -max_total_time=60
cargo +nightly fuzz run --jobs 8 parse_run  # parallel fuzzing across cores
cargo +nightly fuzz cmin parse_run          # corpus minimization
```

## Supporting Libraries

### Actively Considered, Recommended Against

| Crate | Version | Why NOT | What to do instead |
|-------|---------|---------|--------------------|
| `proptest` | `1.11.0` | MSRV 1.85 fine, but table-driven property tests (`convert.rs:79-96`, `url.rs:117`) already cover the `u64 → digits → u64` invariant with curated inputs. Adding a random-input framework here would test the same property with more infrastructure overhead | Keep hand-rolled table tests; let `cargo-fuzz` cover randomised input validation where actual bug-finding pays for itself (decoder, search parser) |
| `quickcheck` | — | Same rationale as proptest; also less actively maintained (proptest is the modern choice anyway) | Skip |
| `insta` | `1.47.2` | Snapshot testing is powerful for CLI output comparison, BUT: (1) `NO_COLOR` / `NO_UNICODE` / `TERM=dumb` output matrix is already contract-tested via explicit `assert_eq!` strings; (2) JSON schema is frozen and additive-only — snapshot would make legitimate schema additions noisy; (3) adds `insta` CLI review workflow that the team hasn't invested in | `assert_cmd` + `predicates::str::contains` for integration tests. If TEST-01 roundtrip failures produce unreadable diffs, revisit — but expectation is byte-identical roundtrip, so plain `assert_eq!` is fine |
| `rexpect` / `expectrl` | — | TUI needs a PTY-driving integration test eventually; both crates provide that. Scope call: TUI unit tests (45 `#[test]`s in `tui.rs` via `handle_key`) cover behaviour without a PTY, and no v2 requirement demands end-to-end TUI I/O | Defer to v3 if/when the TUI grows enough surface to warrant PTY-level tests |
| `lazy_static` | — | Obsolete — stdlib `LazyLock` (stable since 1.80) covers the same need | Already using `std::sync::LazyLock` in `base60-core/src/lib.rs` per STACK.md line 54. Keep doing that |
| `once_cell` | — | Also obsolete for the same reason (`LazyLock` + `OnceLock`) | Same — stdlib |
| `pretty_assertions` | — | Marginal — `predicates::str::diff` already renders readable diffs inside `assert_cmd` | Pass on it; revisit only if diff output is actually unreadable in practice |
| `rstest` | — | Parametrised tests are nice but the table-driven `for n in [...]` pattern is already established in the codebase (`convert.rs:79-96`, `url.rs:117`). Introducing an attribute-based parametriser conflicts with that convention | Keep table-driven; every existing contributor understands it |

## Workspace-level Additions

### Root `Cargo.toml` changes

```toml
[workspace]
resolver = "3"
members  = ["crates/base60-core", "crates/base60-cli"]
exclude  = ["fuzz"]                                          # isolate fuzz/ from main resolve

[workspace.dependencies]                                     # NEW — shared pins
strum        = "0.27"
strum_macros = "0.27"
memchr       = { version = "2.8", default-features = false, features = ["std"] }

[workspace.lints.rust]
unsafe_op_in_unsafe_fn       = "warn"
missing_debug_implementations = "warn"
unreachable_pub               = "warn"
rust_2018_idioms              = { level = "warn", priority = -1 }
unused_lifetimes              = "warn"
unused_qualifications         = "warn"

[workspace.lints.clippy]
pedantic                = { level = "warn", priority = -1 }
nursery                 = { level = "warn", priority = -1 }
cargo                   = { level = "warn", priority = -1 }
multiple_crate_versions = "allow"
module_name_repetitions = "allow"

[profile.release]
lto           = "thin"
codegen-units = 1
strip         = "symbols"
```

### `crates/base60-cli/Cargo.toml` changes

```toml
[dependencies]
# ... existing: anyhow, clap, clap_complete, crossterm, memmap2, ratatui, base60-core ...
memchr       = { workspace = true }                          # NEW — direct, already transitive
strum        = { workspace = true, features = ["derive"] }   # NEW
strum_macros = { workspace = true }                          # NEW (transitive but direct import)

[dev-dependencies]                                           # NEW section entirely
assert_cmd  = { version = "2.2",  default-features = false }
predicates  = { version = "3.1",  default-features = false, features = ["diff"] }
tempfile    = { version = "3.27", default-features = false, features = ["getrandom"] }
serial_test = { version = "3.4",  default-features = false }
criterion   = { version = "0.8",  default-features = false, features = ["cargo_bench_support", "html_reports"] }

[[bench]]
name    = "dump"
harness = false

[[bench]]
name    = "decode"
harness = false

[[bench]]
name    = "search"
harness = false
```

### `crates/base60-core/Cargo.toml` changes

**None.** Core crate stays zero-runtime-dep. If strum is ever needed in core (it isn't — the `LensMode` enum lives in core but its CLI-facing dispatch is in cli; see REF-02), the `workspace.dependencies` pin is ready.

## Development Tools

| Tool | Install | Notes |
|------|---------|-------|
| `cargo-fuzz` | `cargo install cargo-fuzz` | Install in CI job only; requires nightly toolchain at invocation time |
| Nightly rustc for fuzzing | `rustup toolchain install nightly` | Fuzz CI job only. Main test/clippy/fmt/doc matrix stays on 1.95/stable/beta |
| `cargo-nextest` (optional) | `cargo install cargo-nextest` | Not required. Would speed up integration tests but adds install surface to CI. Keep built-in `cargo test` — `serial_test` works with either |

## Version Compatibility Matrix

| Crate | Declared MSRV | 1.95 compatible? | Notes |
|-------|---------------|------------------|-------|
| `assert_cmd` 2.2.1 | 1.85 | Yes | |
| `predicates` 3.1.4 | 1.74 | Yes | |
| `tempfile` 3.27.0 | 1.63 | Yes | |
| `serial_test` 3.4.0 | 1.68 | Yes | |
| `strum` 0.27.2 | 1.71 | Yes | Already pinned transitively |
| `strum_macros` 0.27.2 | 1.71 | Yes | Already pinned transitively |
| `memchr` 2.8.0 | 1.61 | Yes | Already pinned transitively |
| `criterion` 0.8.2 | 1.86 | Yes | Highest MSRV of the set; still under our 1.95 floor |
| `libfuzzer-sys` 0.4.12 | (none) | N/A | Fuzz-only; runs under nightly |
| `arbitrary` 1.4.2 | 1.63 | Yes (if adopted) | Only if structured-input fuzzing is later needed |

## What's Already Free in `Cargo.lock` (Don't Double-Depend)

Verified via `grep -A1 '^name = "X"' Cargo.lock`:

| Crate | Version in lock | Action |
|-------|-----------------|--------|
| `memchr` | `2.8.0` | Promote to direct dep at the same version — zero new compile cost |
| `strum` | `0.27.2` | Promote to direct dep at the same version — zero new compile cost |
| `strum_macros` | `0.27.2` | Promote to direct dep at the same version — zero new compile cost |

This is important for `clippy::multiple_crate_versions`: if we pinned `strum = "0.28"` we'd force a second major version into the graph. Pinning to `0.27` reuses the transitive copy.

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `lazy_static` | Obsolete — stdlib has `LazyLock` (stable 1.80+) | `std::sync::LazyLock` (already in use) |
| `once_cell` | Same — stdlib has `LazyLock` / `OnceLock` (stable 1.80+) | stdlib |
| `proptest` | Property-test infra for an invariant that already has curated coverage | Keep table tests; let fuzz targets do the random-input work |
| `insta` | Snapshot churn on byte-stable formats (JSON schema, decode roundtrip) | `predicates::str::*` composed inside `assert_cmd` |
| `divan` | Pre-1.0; loses to criterion on save-and-compare baselines for CI regression gating | `criterion` 0.8 |
| `rstest` | Parametrised tests conflict with the codebase's established `for n in [...]` idiom | Keep table-driven loops |
| `pretty_assertions` | Redundant with `predicates::str::diff` | Skip |
| `clap` 5+ (if it exists) | Not a concern in 2026-04; 4.6 is current. Do not speculatively upgrade | Stay on 4.6 |

## Stack Patterns by Scope

**If a test mutates `std::env`:**
- Tag with `#[serial_test::serial(env)]` (keyed, not global) — lets non-env tests still run in parallel.
- Remove the `SAFETY: don't run concurrently` comments in `cuneiform.rs:151-161` and `main.rs:183-219`.

**If a test spawns `base60`:**
- `Command::cargo_bin("base60")` from `assert_cmd`, never raw `std::process::Command`.
- Use `tempfile::TempDir` for `XDG_STATE_HOME` redirection so the test doesn't pollute `$HOME`.
- Assert with `predicate::str::contains(...)` / `predicate::eq(bytes)` for bytes.

**If a function searches bytes:**
- Use `memchr::memmem::Finder::new(needle)` when the needle is reused across multiple calls (search navigation — `n`/`N`).
- Use `memchr::memmem::find_iter(haystack, needle)` for one-off `find_all`.

**If an enum gains a variant:**
- Derive `EnumIter` + `VariantNames` + `Display` + `EnumString` from `strum`; drive clap `possible_values`, persist serialise/parse, and dispatch from the same source.

**If a perf requirement is staged:**
- Land a `crates/base60-cli/benches/<name>.rs` first, baseline it with `cargo bench -- --save-baseline pre-perf`, then implement. CI can later gate via `cargo bench -- --baseline pre-perf` comparing percentiles.

## Sources

### Verified via crates.io API (2026-04-24)
- `https://crates.io/api/v1/crates/<name>` — max_stable_version + MSRV + features for every crate listed above. HIGH confidence.

### Context7 library IDs (via `ctx7` CLI fallback)
- `/stebalien/tempfile` — `NamedTempFile::persist`, `TempDir` patterns. HIGH confidence.
- `/assert-rs/predicates-rs` — declaration patterns, version confirmation. HIGH confidence.
- `/peternator7/strum` — `EnumIter` / `VariantNames` / `EnumCount` derive patterns. HIGH confidence.
- `/bheisler/criterion.rs` — `harness = false`, dev-dep declaration, feature set. HIGH confidence.
- `/nvzqz/divan` — divan setup (for comparison before recommending against). HIGH confidence.
- `/rust-fuzz/cargo-fuzz` — install / invocation. HIGH confidence on basics.

### docs.rs direct fetch
- `https://docs.rs/serial_test/latest/serial_test/` — `#[serial]` / `#[serial(key)]` / `#[file_serial]` / `#[parallel]` semantics. HIGH confidence.
- `https://docs.rs/assert_cmd/latest/assert_cmd/` — `Command::cargo_bin` + `.assert()` chain. HIGH confidence.
- `https://docs.rs/memchr/latest/memchr/memmem/` — `find_iter` + `Finder` distinction. HIGH confidence.

### Rust Fuzz Book
- `https://rust-fuzz.github.io/book/cargo-fuzz.html` (+ `setup.html`, `guide.html`) — nightly requirement, libFuzzer default engine, workspace handling. HIGH confidence on nightly status; MEDIUM on workspace layout (confirmed via cross-reference with WebSearch).

### Web search
- `"cargo-fuzz workspace multiple crates setup 2026 nightly"` — confirmed `--fuzzing-workspace=true` flag behaviour, confirmed Windows is not supported for libFuzzer. MEDIUM confidence (cross-referenced with official Rust Fuzz Book).
- `"divan vs criterion rust benchmarking 2025 2026 comparison"` — confirmed criterion 0.8 release + maintenance move to `criterion-rs` org, confirmed divan remains 0.1.x. MEDIUM confidence.

### Local repository inspection
- `/home/chris/Projects/utils/test-60/Cargo.lock` — confirmed `memchr 2.8.0`, `strum 0.27.2`, `strum_macros 0.27.2` already in graph. HIGH confidence.
- `/home/chris/Projects/utils/test-60/.planning/PROJECT.md` — v2 Active requirements scope. HIGH confidence.
- `/home/chris/Projects/utils/test-60/.planning/codebase/STACK.md` + `TESTING.md` — existing dep inventory + test conventions. HIGH confidence.

---
*Stack research for: base60 v2 hardening milestone*
*Researched: 2026-04-24*
