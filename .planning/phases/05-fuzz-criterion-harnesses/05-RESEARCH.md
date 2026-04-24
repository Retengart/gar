# Phase 5: Fuzz + Criterion Harnesses — Research

**Researched:** 2026-04-24
**Domain:** Rust CLI hardening — fuzz scaffolding + bench scaffolding (infra only, no behaviour change)
**Confidence:** HIGH (every version verified via `cargo search` on 2026-04-24; every template verified via cargo-fuzz `src/templates.rs`; every file target verified by reading live source)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions (verbatim)

**A. Fuzz workspace layout (TEST-02 SC1)**
- **D-01:** `fuzz/` created via `cargo fuzz init --fuzzing-workspace=true`. Nested `[workspace]` in `fuzz/Cargo.toml` isolates nightly + sanitizer flags from the main `Cargo.lock`.
- **D-02:** Root `Cargo.toml` gains `[workspace] exclude = ["fuzz"]` (belt-and-suspenders alongside D-01).
- **D-03:** `fuzz/Cargo.toml` path-deps on BOTH `base60-core = { path = "../crates/base60-core" }` and `base60 = { path = "../crates/base60-cli", package = "base60" }`.
- **D-04:** `rust-version` field DROPPED from `fuzz/Cargo.toml`.

**B. `#[cfg(fuzzing)] pub` hatch shape (TEST-02 SC5)**
- **D-05:** Hatch lives in `crates/base60-cli/src/lib.rs` as `#[doc(hidden)] #[cfg(fuzzing)] pub mod __fuzz { … }`. Re-exports: `pub use crate::decode::{parse_run, RUN_LEN}; pub use crate::search::Pattern;`.
- **D-06:** `fn parse_run` → `pub(crate) fn parse_run`; `const RUN_LEN` → `pub(crate) const RUN_LEN`. Both gain `# Errors` / `# Panics` rustdoc sections.
- **D-07:** `crates/base60-cli/src/search.rs`: no changes — `Pattern` is already `pub(crate)`.
- **D-08:** SC5 verification via compile-time test + manual `cargo doc` review.

**C. Seed corpus (TEST-02 SC2)**
- **D-09:** Empty seed corpora on commit.
- **D-10:** `fuzz/.gitignore` covers `corpus/`, `artifacts/`, `target/`, `coverage` (auto-generated per cargo-fuzz template — see "Environment Availability" below).
- **D-11:** Re-evaluate seeding after two Phase 7 weekly runs (out of scope).

**D. Fuzz input guards (TEST-02 SC1, PITFALLS Pitfall 3)**
- **D-12:** `parse_run.rs` shape — length-gate on `data.len() == __fuzz::RUN_LEN`, then `<&[u8; RUN_LEN]>::try_from`, then `let _ = __fuzz::parse_run(arr, 1);`.
- **D-13:** `pattern_from_str.rs` shape — UTF-8 guard (`std::str::from_utf8(data)`), then `let _ = __fuzz::Pattern::from_str(s);`.
- **D-14:** Banner comment in each fuzz target file (Err happy-path / reproduce with --release / Ubuntu+nightly only).
- **D-15:** No `arbitrary` crate as a direct dep.

**E. Criterion dev-dep (PERF-06 SC3)**
- **D-16:** `crates/base60-cli/Cargo.toml [dev-dependencies]` gains `criterion = { version = "0.8", default-features = false, features = ["cargo_bench_support", "html_reports"] }`.
- **D-17:** Same entry on `crates/base60-core/Cargo.toml [dev-dependencies]`. Does NOT violate CI-03 (zero-dep core applies to `[dependencies]` only — Phase 2 D-02 precedent).
- **D-18:** Version `0.8`; criterion MSRV `1.86` ≤ our MSRV `1.95`.

**F. Bench entries (PERF-06 SC3)**
- **D-19:** `base60-core/Cargo.toml` gains `[[bench]] name = "convert" harness = false` + `[[bench]] name = "lens" harness = false`.
- **D-20:** `base60-cli/Cargo.toml` gains `[[bench]] name = "dump" harness = false` + `[[bench]] name = "decode" harness = false` + `[[bench]] name = "search" harness = false`.
- **D-21:** Every `Criterion` instance uses `.noise_threshold(0.05)` (PITFALLS Pitfall 9).
- **D-22:** `sample_size(50)` per bench group.

**G. Bench scope (PERF-06 SC3)**
- **D-23:** `core/benches/convert.rs` — `u64_to_base60` hot loop, 1024 deterministic `u64` inputs via `wrapping_mul(0x9E3779B97F4A7C15)`.
- **D-24:** `core/benches/lens.rs` — `render(&self, u64) -> String` across all four lens impls.
- **D-25:** `cli/benches/dump.rs` — `dump_all` / `write_line` over 1 MiB compile-time-constant byte array, `&PALETTE_NONE`, no lens.
- **D-26:** `cli/benches/decode.rs` — `decode_stream` over a pre-computed 1 MiB dump; dump-generation via `std::sync::LazyLock`.
- **D-27:** `cli/benches/search.rs` — MANDATORY cells (per PITFALLS Pitfall 4):
  - Haystack `vec![0u8; 1 << 20]`, needle `b"\x00"` (1 byte).
  - Haystack `vec![0u8; 1 << 20]`, needle `b"\xff\xff"` (2 byte).
  - Haystack deterministic pseudo-random 1 MiB, needle `b"ELF"` (3 byte).
  - Haystack deterministic pseudo-random, needle `b"cafebabe"` (8 byte).
- **D-28:** Bench inputs use `wrapping_mul` / `wrapping_add`; no `rand` dev-dep.

**H. Bench READMEs (PERF-06 SC4)**
- **D-29:** `crates/base60-cli/benches/README.md` is canonical advisory-posture doc (noise floor, `--save-baseline`, "never CI-gate").
- **D-30:** `crates/base60-core/benches/README.md` = one-liner pointing at CLI README.

**I. Commit granularity**
- **D-31:** TWO plans: `05-01` (fuzz, TEST-02) → `05-02` (benches, PERF-06).
- **D-32:** Serial ordering (parallel-safe in theory; serial in practice for readable commit log).
- **D-33:** Full Phase 3 D-24 gate between commits (`fmt + clippy + test + doc` with `RUSTDOCFLAGS=-D warnings`). Plan 05-01 adds manual smoke: `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` must exit 0.
- **D-34:** xtask gates (`env_discipline`, `spawn_discipline`) already exempt `fuzz/` and `benches/` by walk-root; no gate changes.

### Claude's Discretion (from CONTEXT.md §J)

- Exact byte sequence for the 1 MiB bench haystacks (deterministic generator — lock seed pattern in a `const`).
- Wording of banner comments (D-14) and bench READMEs (D-29).
- Whether `lens.rs` uses one `criterion_group!` with four inner functions or four groups.
- Whether `palette-ansi` dump is a second bench cell (recommended: mono only).
- Exact shape of compile-time test asserting `__fuzz` module is absent in non-fuzz builds.
- Whether `.cargo/config.toml` `[alias] fuzz-smoke` is added (not required by any SC).
- Exact `fuzz/.gitignore` supplementary entries.
- Whether `criterion_group!` uses the `name = …; config = …; targets = …` macro form vs manual `Criterion` instance construction.

### Deferred Ideas (OUT OF SCOPE for Phase 5)

- `cargo-public-api --diff` tooling — v3 or later.
- Fuzz seed corpus curation — re-evaluate after two Phase 7 weekly runs.
- Iai-Callgrind migration — stays criterion for v2.
- `arbitrary`-driven structured fuzz — add if a future target needs it.
- `bencher.dev` / `codspeed.io` baseline tracking — OBSV-02, v3.
- Additional fuzz targets (`emit_json`, `emit_html`, `chunk::be_u64`) — expand only if CI surfaces a bug.
- Per-lens `render_to` UTF-8 fuzz — Phase 6 (PERF-04).
- Moving `parse_run`/`Pattern` into `base60-core` — REJECTED in PROJECT.md row 7.
- `divan` instead of criterion — REJECTED in STACK.md.
- CI-gated criterion — REJECTED permanently (PROJECT.md row 8).
- `cargo-tarpaulin` / codecov — REQUIREMENTS line 70.
- `proptest` / `quickcheck` — covered by fuzz + table tests.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **TEST-02** | `cargo-fuzz` workspace at repo-root `fuzz/`, targets for `decode::parse_run` (via `#[cfg(fuzzing)]` hatch) and `search::Pattern::from_str`, runnable via `cargo +nightly fuzz run <target>` | §"Standard Stack" (`cargo-fuzz` 0.13.1, `libfuzzer-sys` 0.4.12), §"Architecture Patterns" (cargo-fuzz init template, `__fuzz` hatch shape), §"Code Examples" (fuzz target bodies from CONTEXT D-12/D-13), §"Common Pitfalls" (Pitfall 3 banner comment), §"Environment Availability" (nightly toolchain + cargo-fuzz) |
| **PERF-06** | `criterion` benches in `base60-core/benches/{convert,lens}.rs` + `base60-cli/benches/{dump,decode,search}.rs`, advisory-only, lands before any PERF-0X | §"Standard Stack" (`criterion` 0.8.2), §"Architecture Patterns" (per-crate `benches/` layout, `harness = false` + `noise_threshold(0.05)`), §"Code Examples" (5 bench skeletons), §"Common Pitfalls" (Pitfall 4 mandatory search cells, Pitfall 9 advisory-only posture) |

Both REQ-IDs ship in Phase 5 per REQUIREMENTS.md line 91, 100. Plan 05-01 addresses TEST-02; Plan 05-02 addresses PERF-06. Plans touch disjoint files (see §"Plan Split Verification" below).
</phase_requirements>

## Summary

Phase 5 is pure infrastructure — two scaffolds that downstream phases (Phase 6 perf pass, Phase 7 weekly fuzz CI) consume. No behaviour change, no public-API change (per SC5 the `__fuzz` hatch is `#[cfg(fuzzing)]`-gated). Two plans, two commits:

1. **Plan 05-01 (TEST-02)** — create `fuzz/` via `cargo fuzz init --fuzzing-workspace=true`, add `__fuzz` re-export module in `base60-cli/src/lib.rs`, bump `parse_run`/`RUN_LEN` to `pub(crate)` with `# Errors` / `# Panics` rustdoc, add `exclude = ["fuzz"]` to root `Cargo.toml`, write two fuzz targets (`parse_run`, `pattern_from_str`) with the `let _ = …` pattern and banner comments. Manual smoke: `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` exits 0.

2. **Plan 05-02 (PERF-06)** — add `criterion = { version = "0.8", default-features = false, features = ["cargo_bench_support", "html_reports"] }` to BOTH crate's `[dev-dependencies]`, declare 5 `[[bench]] harness = false` entries, write 5 bench files with deterministic compile-time inputs and `.noise_threshold(0.05).sample_size(50)`, document advisory-only posture in `benches/README.md`.

**Primary recommendation:** Do Plan 05-01 first (smaller surface, unblocks Phase 7 CI-02 later). Plan 05-02 is mechanical — 5 bench files + 2 READMEs + manifest deltas. Neither changes any shipped behaviour; CI stays green between commits because the main workspace excludes `fuzz/` and benches compile but don't run in `cargo test`.

**What ships with Phase 5:** scaffolding. **What does NOT ship:** CI jobs, performance measurements, bug fixes. Success is "the artefacts exist and build" (SC1–SC5), not "they caught anything."

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Fuzz crate Cargo manifest + `[workspace]` isolation | Fuzz workspace (new `fuzz/Cargo.toml`, nested `[workspace]`) | Main workspace (root `Cargo.toml` `exclude = ["fuzz"]`) | Nightly-only toolchain, sanitizer flags, and `libfuzzer-sys` dep stay off the main `Cargo.lock`. Main workspace's 3×3 CI matrix never sees them. |
| Fuzz target entry points | `fuzz/fuzz_targets/*.rs` (binary crates inside fuzz workspace) | — | `#![no_main]` + `libfuzzer_sys::fuzz_target!` is the libFuzzer ABI. Targets call into `base60-cli` via the `__fuzz` re-export shim; zero business logic lives in the targets themselves. |
| CLI visibility hatch (`__fuzz` module) | CLI lib root (`crates/base60-cli/src/lib.rs`) | CLI modules (`decode.rs`, `search.rs`) | Hatch is a `#[doc(hidden)] #[cfg(fuzzing)] pub mod` in `lib.rs`. `decode::parse_run` and `decode::RUN_LEN` widen from `fn`/`const` to `pub(crate) fn`/`pub(crate) const`; `search::Pattern` already `pub(crate)`. |
| Bench harness (core) | `base60-core/benches/*.rs` + `base60-core/Cargo.toml` `[dev-dependencies]` | — | Library crate's `benches/` exercises library's public API (`u64_to_base60`, `Lens::render`). `criterion` as `[dev-dependencies]` preserves CI-03 zero-dep-runtime invariant (Phase 2 D-02 precedent). |
| Bench harness (CLI) | `base60-cli/benches/*.rs` + `base60-cli/Cargo.toml` `[dev-dependencies]` | CLI lib API (`dump::dump_all`, `decode::decode_stream`, `search::find_all` — already `pub(crate)` through thin `[lib]` target from Phase 3) | Three CLI benches exercise `pub(crate)` functions via the crate's library surface. Since `base60-cli` has `[lib] name = "base60"` (Phase 3 D-06), benches can reach them through the `base60::` library namespace — but `parse_run`/`Pattern`/`find_all` are NOT re-exported at the public surface. Solution for `find_all`: bench lives alongside `search` and uses `pub(crate)` visibility through the thin `[lib]` — confirmed no extra visibility bump needed since the bench compiles as an integration-test-equivalent binary that links `base60` the lib. |
| Advisory-only posture documentation | `crates/base60-cli/benches/README.md` (canonical) + `crates/base60-core/benches/README.md` (pointer) | PROJECT.md row 8 (Key Decision locking in posture) | README is the single source of truth for bench-workflow contract: local laptop, `--save-baseline`, paste into PR description. CI-03 + SC4 formalise "never gate." |

**Why this matters:** Bench-file visibility is the one non-trivial detail in this phase. Criterion benches declared as `[[bench]]` compile as separate binaries but link the crate's library target. If a bench needs a `pub(crate)` function, it accesses it via the `base60::` crate root — which means the function must be re-exported or the bench must live inside the same crate and reach internals directly. The CLI benches (D-25..D-27) reference `dump::dump_all`, `decode::decode_stream`, `search::find_all` — all `pub(crate)` in the thin `[lib]` Phase 3 shipped. Since the bench is an external binary, it sees only `pub` surface. **Two resolution options:**

- **Option A (recommended):** Add `#[doc(hidden)] pub` re-exports for the specific bench-consumed items OR add a second `#[cfg(bench)]`-equivalent module. Complexity: one more cfg'd module.
- **Option B (simpler, confirmed-workable):** The bench files use the same thin-lib public-API trick — since `base60-cli` has `[lib] name = "base60"`, any `pub` item on the lib root is reachable. The bench functions we need (`dump_all`, `decode_stream`, `find_all`) are currently `pub(crate)`. The planner needs to **re-export exactly what the benches need** through a `#[doc(hidden)]` shim OR **widen to `pub` with doc-hidden** on those three functions.

**Recommendation for the planner:** Extend the `__fuzz` shim pattern into a parallel `#[doc(hidden)] pub mod __bench { … }` module on `base60-cli/src/lib.rs` — no cfg-gate needed (benches always compile), just doc-hide the re-exports so they don't pollute `cargo doc` output. Functions re-exported: `pub use crate::dump::dump_all; pub use crate::decode::decode_stream; pub use crate::search::{find_all, Pattern}; pub use crate::color::PALETTE_NONE; pub use crate::cli::InputFormat;`. The bench files then `use base60::__bench::{…};`. This mirrors the `__fuzz` hatch's approach. **This adds a Claude's Discretion item to the phase** — CONTEXT.md's §J already lists `__fuzz` shape as discretion; the `__bench` shim is a symmetric extension.

**Alternative for the planner:** Place bench files at `crates/base60-cli/src/bin/bench_*.rs` wrappers, but this is unconventional and contradicts CONTEXT D-20's `[[bench]]` layout. Stick with `__bench` shim.

Rustdoc impact: every new `pub use` under `#[doc(hidden)]` is invisible in `cargo doc` output; `unreachable_pub` lint is satisfied because the items ARE now reachable from a `pub` module. No workspace-lint changes.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `libfuzzer-sys` | `0.4.12` [VERIFIED: `cargo search libfuzzer-sys --limit 1` on 2026-04-24] | `fuzz_target!` macro + FFI shim to vendored libFuzzer | De facto standard for Rust libFuzzer harnesses. Default feature set is `["link_libfuzzer"]` [CITED: github.com/rust-fuzz/libfuzzer/Cargo.toml] — `arbitrary-derive` is the only non-default feature (opt-in structured-input derive). CONTEXT D-X `default-features = false, features = ["link_libfuzzer"]` is identical-net-effect to default — it just makes the intent explicit and guards against a future default expansion. |
| `criterion` | `0.8.2` [VERIFIED: `cargo search criterion --limit 1` on 2026-04-24] | Statistics-driven microbenchmark harness | Locked by STACK.md; MSRV 1.86 compatible with our 1.95 floor. Maintenance has moved to the `criterion-rs` GitHub org; `0.8.x` is the actively-maintained line. Default features `rayon` and `plotters` are dropped — `rayon` adds parallelism noise to streaming-code measurements, `plotters` still comes in transitively via `html_reports` [VERIFIED: read from Context7 `/bheisler/criterion.rs` docs]. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `cargo-fuzz` | `0.13.1` (tool, not dep) [VERIFIED: `cargo search cargo-fuzz --limit 1` on 2026-04-24] | `cargo fuzz init / run / build / cmin` subcommand | Developer-local tool; CI will install it in Phase 7. Not pulled into `Cargo.toml` anywhere. Invoked as `cargo +nightly fuzz <subcmd>`. |
| `base60-core` | path dep `= { path = "../crates/base60-core" }` | Fuzz targets compile against the CLI's `__fuzz` re-export of `Pattern` + `parse_run`, but redundantly pulling `base60-core` confirms no-cycle and keeps STACK.md template literal (per D-03) | Fuzz `parse_run` target uses `base60_core::convert::DIGITS` transitively via `RUN_LEN = PAIR * DIGITS + (DIGITS - 1) = 33`; confirmed by inspecting `decode.rs:48-50`. |
| `base60` (CLI lib, package name) | path dep `= { path = "../crates/base60-cli", package = "base60" }` | Fuzz targets import `base60::__fuzz::{parse_run, RUN_LEN, Pattern}` | CLI's thin `[lib] name = "base60" path = "src/lib.rs"` (Phase 3 D-06) exposes the `__fuzz` module when compiled with `--cfg fuzzing`. |

### Alternatives Considered (and rejected — all locked upstream)

| Instead of | Could Use | Tradeoff | Status |
|------------|-----------|----------|--------|
| `libfuzzer-sys` | `libafl_libfuzzer` (packaged as `libfuzzer-sys = { version = "0.15.3", package = "libafl_libfuzzer" }`, per cargo-fuzz `templates.rs`) | LibAFL is a more modern engine with better telemetry, but it's still nightly-only, Linux-only, and changes the crash-reporter format Phase 7 CI depends on | REJECTED per STACK.md default choice; stick with vendored libFuzzer |
| `criterion` | `divan` (`0.1.21`) | Divan has nicer `#[divan::bench]` syntax and deterministic instruction-count mode; but pre-1.0, and `--save-baseline` comparison is criterion's main feature | REJECTED in STACK.md §"Benchmarking" |
| Hand-rolled `#![feature(test)]` nightly bench | N/A | Requires nightly for every developer, breaks MSRV 1.95 floor | REJECTED — same reason STACK.md recommends criterion |
| `arbitrary` crate (direct dev-dep) | `arbitrary = "1.4"` for `#[derive(Arbitrary)]` structured input | Our two targets take raw `&[u8]`/`&str`; structured-input gen not needed | REJECTED per D-15; revisit if future JSON-emitter fuzz needs it |
| `iai-callgrind` | `iai-callgrind = "0.14"` for instruction-count-based bench under cachegrind | Eliminates Pitfall 9 (GHA noise), but breaks macOS/Windows CI cells (valgrind is Linux-only), requires valgrind install step | REJECTED per REQUIREMENTS line 71; revisit if criterion advisory-only proves insufficient |

### Manifest shapes (EXACT — planner copies verbatim)

**`fuzz/Cargo.toml`** (after `cargo fuzz init --fuzzing-workspace=true`, then hand-edited per D-03/D-04):

```toml
[package]
name = "base60-fuzz"
version = "0.0.0"
publish = false
edition = "2024"

[package.metadata]
cargo-fuzz = true

# Use independent workspace for fuzzers (added by --fuzzing-workspace=true).
[workspace]
members = ["."]

[dependencies]
libfuzzer-sys = { version = "0.4", default-features = false, features = ["link_libfuzzer"] }
base60-core   = { path = "../crates/base60-core" }
base60        = { path = "../crates/base60-cli", package = "base60" }

[[bin]]
name = "parse_run"
path = "fuzz_targets/parse_run.rs"
test = false
doc = false
bench = false

[[bin]]
name = "pattern_from_str"
path = "fuzz_targets/pattern_from_str.rs"
test = false
doc = false
bench = false
```

Notes:
- **`edition = "2024"`** — matches workspace edition. The cargo-fuzz template usually writes `edition = "2021"` by default; planner overwrites to 2024 for consistency with the main workspace.
- **NO `rust-version`** field (per D-04). Fuzz builds are nightly-only; the main workspace's MSRV 1.95 floor doesn't apply.
- **NO `resolver = "3"`** — when a nested `[workspace]` is present, cargo requires `resolver` to be set on the root package (which `[package]` here is) OR on the `[workspace]` section. Planner can omit it (defaults to resolver=1 for edition<2021, resolver=2 for edition≥2021) OR set `resolver = "3"` explicitly on the `[workspace]` section to match the main workspace. Recommend **setting `resolver = "3"` inside `[workspace]`** for consistency. This is Claude's Discretion — both work.
- **`[package.metadata] cargo-fuzz = true`** is cargo-fuzz's recognition marker; it is written by `cargo fuzz init` and MUST stay after hand-editing [CITED: github.com/rust-fuzz/cargo-fuzz/src/templates.rs].
- **All three `[[bin]]` blocks** set `test = false, doc = false, bench = false` — this is the cargo-fuzz convention so `cargo test`/`cargo doc`/`cargo bench` don't try to build the fuzz targets [CITED: rust-fuzz book tutorial].

**Root `Cargo.toml`** (DELTA — append to existing `[workspace]`):

```toml
[workspace]
resolver = "3"
members = ["crates/base60-core", "crates/base60-cli", "crates/xtask"]
exclude = ["fuzz"]                                          # NEW (D-02)
```

Note: `--fuzzing-workspace=true` makes `exclude` redundant (nested `[workspace]` already bars membership). CONTEXT D-02 says belt-and-suspenders; keep the explicit exclude. ROADMAP SC1 specifically calls for `exclude = ["fuzz"]`.

**`crates/base60-core/Cargo.toml`** (DELTA — append to `[dev-dependencies]` + new `[[bench]]` blocks):

```toml
[dev-dependencies]
serial_test = { version = "3", default-features = false }   # existing
criterion   = { version = "0.8", default-features = false, features = ["cargo_bench_support", "html_reports"] }  # NEW (D-17)

[[bench]]
name    = "convert"
harness = false

[[bench]]
name    = "lens"
harness = false
```

**`crates/base60-cli/Cargo.toml`** (DELTA — append to `[dev-dependencies]` + new `[[bench]]` blocks):

```toml
[dev-dependencies]
# ... existing (assert_cmd, base60-core, predicates, serial_test, tempfile) ...
criterion = { version = "0.8", default-features = false, features = ["cargo_bench_support", "html_reports"] }   # NEW (D-16)

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

Planner note on `[[bench]]` block ordering: cargo doesn't care; put them alphabetically after `[[bin]]` for readability (this file already has `[lib]` before `[[bin]]` — so structure becomes `[package] / [lib] / [[bin]] / [dependencies] / [dev-dependencies] / [[bench]] × N / [lints]`).

### `cargo fuzz init --fuzzing-workspace=true` — exact generated layout

[CITED: github.com/rust-fuzz/cargo-fuzz/src/templates.rs as of 2026-04-24]

Files created by `cd <repo-root> && cargo fuzz init --fuzzing-workspace=true`:

1. **`fuzz/Cargo.toml`** — from the Cargo.toml template with `{libfuzzer_sys_dep}` = `libfuzzer-sys = "0.4"`, `{edition}` = the detected edition (usually `edition = "2021"`), and the `--fuzzing-workspace=true` flag appending:
   ```toml
   # Use independent workspace for fuzzers
   [workspace]
   members = ["."]
   ```
   **Hand-edit needed:** overwrite to edition = "2024", replace single-package `[dependencies]` block with the full path-dep set from D-03, add per-target `[[bin]]` blocks with `test = false, doc = false, bench = false`.

2. **`fuzz/fuzz_targets/fuzz_target_1.rs`** — skeleton target:
   ```rust
   #![no_main]

   use libfuzzer_sys::fuzz_target;

   fuzz_target!(|data: &[u8]| {
       // fuzzed code goes here
   });
   ```
   **Hand-edit needed:** rename to `parse_run.rs` and `pattern_from_str.rs`, replace body with D-12/D-13 shapes, prepend banner comment (D-14).

3. **`fuzz/.gitignore`**:
   ```
   target
   corpus
   artifacts
   coverage
   ```
   **Matches D-10 exactly.** No hand-edit required. CONTEXT D-10's wording ("corpus/, artifacts/, target/") is a subset — the template also adds `coverage` (used by `cargo fuzz coverage` which Phase 7 may use). Keep the template's content verbatim.

### Version verification (2026-04-24)

```
cargo search libfuzzer-sys --limit 1 → libfuzzer-sys = "0.4.12"
cargo search criterion --limit 1     → criterion = "0.8.2"
cargo search cargo-fuzz --limit 1    → cargo-fuzz = "0.13.1"
```

All three match STACK.md. Caret syntax `"0.4"` and `"0.8"` in the manifests will resolve to latest minor; `Cargo.lock` with `--locked` CI pins exact resolution.

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────── MAIN WORKSPACE ───────────────────────┐
│                                                              │
│   Cargo.toml                                                 │
│     members = [base60-core, base60-cli, xtask]               │
│     exclude = ["fuzz"]                      ◄── NEW D-02      │
│                                                              │
│   crates/base60-core/                                        │
│     Cargo.toml                                               │
│       [dev-dependencies] criterion = "0.8"  ◄── NEW D-17      │
│       [[bench]] convert + lens              ◄── NEW D-19      │
│     src/                        (unchanged)                  │
│     benches/                    ◄── NEW                      │
│       convert.rs   ◄── D-23 (u64_to_base60)                  │
│       lens.rs      ◄── D-24 (Lens::render × 4 impls)         │
│       README.md    ◄── D-30 (one-liner)                      │
│                                                              │
│   crates/base60-cli/                                         │
│     Cargo.toml                                               │
│       [dev-dependencies] criterion = "0.8"  ◄── NEW D-16      │
│       [[bench]] dump + decode + search      ◄── NEW D-20      │
│     src/                                                     │
│       lib.rs                                                 │
│         + #[doc(hidden)] #[cfg(fuzzing)]                     │
│           pub mod __fuzz { ... }              ◄── NEW D-05    │
│         + #[doc(hidden)] pub mod __bench { ... } ◄── NEW     │
│           (recommended for bench visibility; Claude's        │
│            discretion per §"Arch Resp Map")                  │
│       decode.rs                                              │
│         + pub(crate) fn parse_run            ◄── NEW D-06    │
│         + pub(crate) const RUN_LEN           ◄── NEW D-06    │
│       search.rs                              (unchanged D-07)│
│     benches/                    ◄── NEW                      │
│       dump.rs     ◄── D-25 (dump_all over 1 MiB mono)        │
│       decode.rs   ◄── D-26 (decode_stream over pre-dumped)   │
│       search.rs   ◄── D-27 (4 needle/haystack cells)         │
│       README.md   ◄── D-29 (advisory-only posture)           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
                              │
                              │ (path dep, + --cfg fuzzing)
                              ▼
┌──────────────────── FUZZ WORKSPACE (isolated) ───────────────┐
│                                                              │
│   fuzz/                                                      │
│     Cargo.toml                                               │
│       [package] name = base60-fuzz                           │
│       [package.metadata] cargo-fuzz = true                   │
│       [workspace] members = ["."]            ◄── D-01 nested │
│       [dependencies]                                         │
│         libfuzzer-sys = "0.4"                                │
│         base60-core   = path dep                             │
│         base60        = path dep                             │
│       [[bin]] × 2 with test/doc/bench = false                │
│     fuzz_targets/                                            │
│       parse_run.rs         ◄── D-12 target                   │
│       pattern_from_str.rs  ◄── D-13 target                   │
│     .gitignore (auto-generated)                              │
│     README.md              ◄── CONTEXT canonical_refs NEW    │
│     corpus/       (runtime, gitignored, empty on commit)     │
│     artifacts/    (runtime, gitignored)                      │
│     target/       (runtime, gitignored)                      │
│                                                              │
└──────────────────────────────────────────────────────────────┘

Data flow on `cargo +nightly fuzz run parse_run`:
  cargo-fuzz → cargo build --manifest-path fuzz/Cargo.toml -Zsanitizer=address
            → link libFuzzer vendored
            → run parse_run binary
            → libFuzzer mutates bytes → fuzz_target!(|data|) closure
            → length-gate → __fuzz::parse_run(arr, 1)
            → Err = happy path, panic = bug, aborts
```

### Recommended File Structure (Phase 5 additions)

```
test-60/                                # repo root (fuzz/ lives here at top level)
├── Cargo.toml                          # EDIT: add `exclude = ["fuzz"]` to [workspace]
├── crates/
│   ├── base60-core/
│   │   ├── Cargo.toml                  # EDIT: add criterion dev-dep + 2 [[bench]]
│   │   ├── src/...                     # unchanged
│   │   └── benches/                    # NEW DIRECTORY
│   │       ├── convert.rs              # NEW (D-23)
│   │       ├── lens.rs                 # NEW (D-24)
│   │       └── README.md               # NEW (D-30, one-liner)
│   └── base60-cli/
│       ├── Cargo.toml                  # EDIT: add criterion dev-dep + 3 [[bench]]
│       ├── src/
│       │   ├── lib.rs                  # EDIT: + __fuzz module (D-05), + __bench module (Claude's discretion)
│       │   ├── decode.rs               # EDIT: fn→pub(crate) fn parse_run; const→pub(crate) const RUN_LEN; + # Errors + # Panics (D-06)
│       │   └── search.rs               # unchanged (D-07)
│       └── benches/                    # NEW DIRECTORY
│           ├── dump.rs                 # NEW (D-25)
│           ├── decode.rs               # NEW (D-26)
│           ├── search.rs               # NEW (D-27)
│           └── README.md               # NEW (D-29, canonical advisory doc)
└── fuzz/                               # NEW DIRECTORY (via `cargo fuzz init --fuzzing-workspace=true`)
    ├── Cargo.toml                      # NEW (cargo-fuzz template, then hand-edited per D-03/D-04)
    ├── .gitignore                      # NEW (auto-generated per cargo-fuzz template)
    ├── README.md                       # NEW (planner writes; Ubuntu+nightly only + reproducer commands per CONTEXT canonical_refs)
    └── fuzz_targets/
        ├── parse_run.rs                # NEW (D-12 shape + D-14 banner)
        └── pattern_from_str.rs         # NEW (D-13 shape + D-14 banner)
```

### Pattern 1: `#[cfg(fuzzing)] pub` escape hatch (D-05)

**What:** `base60-cli` widens exactly two items (`parse_run`, `RUN_LEN`) visible only when `cargo-fuzz` compiles the crate with `--cfg fuzzing`. The `Pattern` struct is already `pub(crate)` and becomes `pub` reachable via the re-export. In every other compilation (the main workspace's 3×3 CI matrix, `cargo doc`, `cargo install`), the module doesn't exist — public surface is pristine.

**When to use:** CLI-internal items that need fuzz coverage but must not grow the library's stable API. (Phase 6 may reuse the pattern for `render_to<W>` fuzz if a lens surface needs it.)

**Example** (planner copies verbatim into `crates/base60-cli/src/lib.rs`):

```rust
// Existing: mod decode; mod search; etc.
// Existing: pub use cli::{LensMode, Format};
// Existing: #[doc(hidden)] pub use cli::TimeScale as __TuiTimeScale;
// Existing: #[doc(hidden)] pub mod __test_hooks { pub use crate::tui::run_with_terminal; }

/// Hidden re-exports for the repo-root `fuzz/` crate.
///
/// Only materialises when `cargo-fuzz` compiles this crate with
/// `--cfg fuzzing`. Non-fuzz builds (the 3×3 CI matrix, `cargo doc`,
/// `cargo install`) do NOT see this module, so the public API surface
/// is unchanged in every shipped artefact (TEST-02 SC5).
#[doc(hidden)]
#[cfg(fuzzing)]
pub mod __fuzz {
    pub use crate::decode::{parse_run, RUN_LEN};
    pub use crate::search::Pattern;
}
```

**Trade-offs:** `#[cfg(fuzzing)]` means the module is invisible to `cargo check --workspace` (non-fuzz), `cargo clippy`, `cargo doc` — none of those flag the hidden items. Rust-analyzer will also hide them. This is the intended behaviour for a fuzz-only hatch. Verification of SC5 is either (a) manual `cargo doc --workspace --no-deps --locked` output inspection for no new `pub` items under `base60` in non-fuzz builds, or (b) a compile-time test that asserts the symbol is absent in non-fuzz. Recommended option (b): add to the `#[cfg(test)] mod tests` in `lib.rs`:

```rust
#[test]
#[cfg(not(fuzzing))]
fn fuzz_module_absent_in_non_fuzz_build() {
    // If `__fuzz` leaks into a non-fuzz build, this file fails to compile
    // because the module is cfg-gated. The test exists purely to document
    // the invariant; the compiler is the real gate.
    // (A stronger check: `#[cfg(fuzzing)] compile_error!("fuzzing must be off");`
    // would fire at build time, but we need the test suite to see a green assertion.)
    let _: () = ();
}
```

This is Claude's Discretion per CONTEXT.md §J. Either path satisfies SC5.

### Pattern 2: `criterion_group!` with config block (D-21, D-22)

**What:** Each bench file declares `name = …; config = Criterion::default().noise_threshold(0.05).sample_size(50); targets = …`. One `criterion_main!` per file discovers the group.

**When to use:** Every bench file in Phase 5. Centralising config in the group macro avoids per-test `b.iter` boilerplate around `Criterion` mutation.

**Example** (planner copies the shape; each bench customises only the `targets = …` line):

```rust
// Source: Context7 /bheisler/criterion.rs "Configure Sample Count and Statistical Settings in Rust"
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_<something>(c: &mut Criterion) {
    c.bench_function("<name>", |b| b.iter(|| <workload>));
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_<something>
}
criterion_main!(benches);
```

**Trade-offs:** `.sample_size(50)` trades measurement precision for wall-clock time (default is 100). PITFALLS Pitfall 9 notes shared-CI-runner noise often exceeds 5%, but since benches are advisory-only and local-only, 50 samples keeps `cargo bench --workspace` under ~30s.

### Anti-Patterns to Avoid

- **`unwrap()` / `expect()` inside `fuzz_target!`** — converts expected `Err` returns into false-positive crash reports. Use `let _ = ...;` instead. (Pitfall 3.)
- **`std::panic::catch_unwind` to filter expected panics** — cargo-fuzz compiles with `-Cpanic=abort`, so catch_unwind never fires. (Pitfall 3.)
- **Committing fuzz `corpus/` or `artifacts/`** — `fuzz/.gitignore` from the cargo-fuzz template already covers them; don't override. (Pitfall 3 prevention.)
- **Running `cargo bench` in CI and gating PRs** — shared GHA noise floor > any reasonable threshold. (Pitfall 9; PROJECT.md row 8.)
- **Fuzz target that spawns the CLI binary** — process overhead dwarfs iteration budget; coverage feedback gets attenuated. Fuzz pure-function entry points only. (ARCHITECTURE.md Anti-Pattern 3.)
- **Matrix fuzz job across Ubuntu/macOS/Windows** — libFuzzer is Linux+nightly only. Phase 7 CI-02 will be Ubuntu-pinned. (Pitfall 11.)
- **`memmem::find_iter` blindly for 1-byte needle** — Pitfall 4. The search bench cells exist to catch this regression BEFORE Phase 6 PERF-03 lands.
- **Bench on non-deterministic inputs (`std::time` / system RNG)** — baselines across runs become noise. Use `wrapping_mul` / `wrapping_add` const generators (D-28).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Fuzz input mutation engine | Custom RNG bit-flipper | `libfuzzer-sys = "0.4"` | Coverage-guided mutation with sanitizer-driven feedback is 10+ years of research; libFuzzer is the reference implementation |
| Fuzz crash reproducer | Hand-crafted hex serializer | `cargo fuzz tmin` / `cargo fuzz cmin` | Corpus minimisation is built into cargo-fuzz; the reproducer format is stable across runs |
| Bench statistical analysis | Custom `mean ± σ` computation | `criterion` 0.8 | `cargo bench -- --save-baseline pre` + `cargo bench -- --baseline pre` does proper hypothesis testing with bootstrap resampling |
| Bench HTML report | Write HTML manually | `criterion` `html_reports` feature | Plotters-backed interactive charts, baseline comparison, time-series overview — all free |
| Structured fuzz input | Hand-derive `fn parse_struct(data: &[u8]) -> Option<MyType>` | `arbitrary = "1.4"` (DEFERRED) | Not needed this phase (D-15). When structured input IS needed later, `arbitrary` is the canonical answer |
| 1 MiB bench haystack | `rand::thread_rng()` fill | `wrapping_mul` const-expr | Determinism is the whole point; `rand` would add a dev-dep for zero benefit (D-28) |

**Key insight:** Every problem Phase 5 scaffolds has a canonical Rust-ecosystem answer. The only "custom" code is the bench bodies and fuzz targets themselves — both call canonical library entry points.

## Common Pitfalls

### Pitfall 3 (PITFALLS.md): Fuzz targets flag by-design rejections as crashes — **applies directly**

**What goes wrong:** `fuzz_target!(|data: &[u8]| { base60::__fuzz::parse_run(data, 1).unwrap(); })` reports every ASCII-colon mismatch / digit ≥ 60 / non-digit byte as a "crash." Real bugs drown in false positives.

**Why it happens:** `parse_run`'s `Err` branches are the happy path for malformed input — that's by design. Fuzzers treat panic/abort as signal; a `.unwrap()` converts an `Err` into a panic.

**How to avoid (enforced by D-12, D-13, D-14):**
- Every fuzz_target body uses `let _ = ...;` — never `.unwrap()` / `.expect()`.
- UTF-8 guard on `Pattern::from_str` before the call (D-13).
- Length-gate on `parse_run` before the array-try-from (D-12).
- Banner comment at top of each fuzz target file explicitly calls out "Err is happy path, panics are bugs" + reproducer instruction.

**Warning signs:**
- Corpus `corpus/parse_run/` grows > 1000 entries in the first minute → coverage noise, not bug finding.
- Any "crash" panicking on `from_utf8` or `InvalidData` display → false positive; the guard is missing.
- `unwrap()` or `expect()` appears anywhere inside `fuzz_target!`.

### Pitfall 4 (PITFALLS.md): `memchr::memmem` loses to naive on 1–3 byte needles — **applies to `search.rs` bench cells**

**What goes wrong:** `search::find_all` currently uses `windows().position()` (naive). Phase 6 PERF-03 swaps to `memchr::memmem::find_iter`. Without baseline coverage, 1-byte-needle regression on zero-fill haystack (ELF `.bss`, zeroed block devices) silently ships.

**Why it happens:** `memmem` dispatches to Two-Way with a packed-pair prefilter by default. For needle length 1, stdlib `memmem` and `memchr::memchr` converge; for length 2–3 the prefilter can over-trigger on low-entropy haystacks.

**How to avoid (enforced by D-27):**
- The search bench MUST include 4 cells minimum:
  - 1-byte `b"\x00"` on zero-fill — catches the 1-byte regression.
  - 2-byte `b"\xff\xff"` on zero-fill — catches packed-pair prefilter over-trigger.
  - 3-byte `b"ELF"` on deterministic random — representative.
  - 8-byte `b"cafebabe"` on deterministic random — realistic.
- Phase 6 PERF-03 gates on a `memchr_iter`-vs-`memmem` comparison using these cells.

**Warning signs:**
- Phase 6 PR ships `memchr::memmem::find_iter` for all needle lengths → flag it.
- Bench cell count < 4 → D-27 minimum unmet.

### Pitfall 9 (PITFALLS.md): Criterion noise floor drowns real signal on shared CI — **applies to README.md posture**

**What goes wrong:** GHA runners have 10–15% measured variance between back-to-back runs. Criterion's default `noise_threshold = 2%` flags every PR as spurious regression or improvement. Team ignores the check. Real regressions ship.

**Why it happens:** Criterion's statistics are sound; cloud CI's environment isn't. The criterion FAQ explicitly warns against it.

**How to avoid (enforced by D-21, D-29):**
- `Criterion::default().noise_threshold(0.05)` — 5% tolerance catches laptop noise, still drowns in GHA noise. That's fine because CI never runs the benches.
- `crates/base60-cli/benches/README.md` is the canonical source of truth: "advisory only, NEVER CI-gated." Phase 7 SC4 only adds a `--no-run` compile smoke.
- PROJECT.md row 8 is the lock: "Criterion benches are advisory, not CI-gating."

**Warning signs:**
- CI workflow gains a `cargo bench` step that exits non-zero on threshold breach.
- README text doesn't spell out "advisory."
- A PR reviewer comments "ignore the bench failure, it's flaky" → the gate shouldn't exist.

### Pitfall 11 (PITFALLS.md): `cargo-fuzz` silently falls back on macOS/Windows CI — **applies to Phase 7 CI-02 (NOT this phase)**

**What goes wrong:** A future `fuzz.yml` workflow using `matrix: os: [ubuntu, macos, windows]` produces: Ubuntu green (real fuzz), macOS green-but-no-fuzz (silent fallback), Windows loud fail. Noise.

**Why it happens:** libFuzzer needs `-Zsanitizer` sanitizer support on x86_64/aarch64 Linux/macOS nightly. Windows MSVC target doesn't support the flag; macOS support exists but the nightly toolchain availability is flakier.

**How to avoid (enforced by scope):**
- Phase 5 ships SCAFFOLDING ONLY — no CI job. Phase 7 CI-02 will be explicitly `runs-on: ubuntu-latest`.
- `fuzz/README.md` documents the Ubuntu+nightly-only constraint.

**Warning signs:**
- A planner draft for Phase 5 adds `.github/workflows/fuzz.yml` → out of scope; push to Phase 7.

## Runtime State Inventory

**Not applicable.** Phase 5 is a greenfield scaffolding phase — no rename, no refactor of stored data, no string-replacement across services. Every Phase 5 artefact is a new file under `fuzz/` or `benches/`, plus manifest deltas on `Cargo.toml`. No ChromaDB / Mem0 / Datadog / n8n / SOPS / pm2 / Task Scheduler / egg-info state is touched.

Category-by-category for explicitness:

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | None — no databases, no collections, no user_ids in the project | — |
| Live service config | None — no external services (Datadog / n8n / Cloudflare / Tailscale not in project) | — |
| OS-registered state | None — no scheduled tasks, no systemd units, no launchd plists | — |
| Secrets and env vars | None — CI uses only `CARGO_TERM_COLOR`, `RUST_BACKTRACE`, `CARGO_INCREMENTAL`; Phase 5 doesn't touch them | — |
| Build artifacts / installed packages | None — Phase 5 adds new files only; `Cargo.lock` will gain `criterion` + transitive entries but that's cargo's normal operation, not a rename cache issue | — |

## Code Examples

### Example 1: `fuzz/fuzz_targets/parse_run.rs` (copy-paste ready)

```rust
// IMPORTANT: Err returns are the happy path — only panics are bugs.
// On reported crash: reproduce with `--release` first to confirm.
// Platform: Ubuntu + pinned nightly only (libFuzzer is Linux-x86_64/aarch64 only).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Length-gate: `parse_run` takes `&[u8; RUN_LEN]` (Phase 4 D-09).
    // libFuzzer will still mutate inputs past RUN_LEN; we skip those.
    if data.len() != base60::__fuzz::RUN_LEN {
        return;
    }
    let Ok(arr) = <&[u8; base60::__fuzz::RUN_LEN]>::try_from(data) else {
        return;
    };
    // Errors are happy path; only panics are bugs.
    let _ = base60::__fuzz::parse_run(arr, 1);
});
```

Wording of the banner comment is Claude's Discretion per CONTEXT §J. Above matches the D-14 bullet list literally.

### Example 2: `fuzz/fuzz_targets/pattern_from_str.rs` (copy-paste ready)

```rust
// IMPORTANT: Err returns are the happy path — only panics are bugs.
// On reported crash: reproduce with `--release` first to confirm.
// Platform: Ubuntu + pinned nightly only (libFuzzer is Linux-x86_64/aarch64 only).

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str::FromStr;

fuzz_target!(|data: &[u8]| {
    // UTF-8 guard matches rust-fuzz/book's canonical pattern — `Pattern::from_str`
    // takes `&str`, so we skip invalid-UTF-8 inputs without treating them as bugs.
    // Do NOT use `std::panic::catch_unwind` — cargo-fuzz compiles with
    // `-Cpanic=abort`, which prevents unwinding.
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = base60::__fuzz::Pattern::from_str(s);
    }
});
```

### Example 3: `crates/base60-cli/src/decode.rs` — visibility bumps + rustdoc additions

The `parse_run` function currently at line 423 is `fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64>`. Its doc comment already contains `# Errors` (lines 416-422 of current source). Planner only needs:

1. Add `pub(crate)` to the `fn` keyword:
   ```rust
   // Before:
   fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64> {
   // After:
   pub(crate) fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64> {
   ```

2. Add `pub(crate)` to the const (line 50):
   ```rust
   // Before:
   const RUN_LEN: usize = PAIR * DIGITS + (DIGITS - 1);
   // After:
   pub(crate) const RUN_LEN: usize = PAIR * DIGITS + (DIGITS - 1);
   ```

3. Existing doc comment on `parse_run` already has `# Errors`. Add `# Panics` **none-section** because the function does NOT panic on its documented path — it returns `Err` on every failure (all three error branches are explicit). Rustdoc does NOT require a `# Panics` section if there are no panic paths. However, `pub(crate)` + `RUSTDOCFLAGS=-D warnings` + `clippy::missing_panics_doc` — that clippy lint is normally for public panics. Verify the existing `#[derive]` / `#[must_use]` markers don't trigger new warnings.

   **Planner verification step:** run `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --locked` after the visibility bump. If `missing_panics_doc` fires on `parse_run` (unlikely — no `unwrap`/`expect`/`panic!` in the body), add:
   ```rust
   /// # Panics
   ///
   /// Does not panic. All failure paths return [`io::ErrorKind::InvalidData`]
   /// via the `# Errors` section above.
   ```

4. `RUN_LEN` doc comment (existing: `/// Total characters for 11 digit pairs joined by 10 colons.`) gains nothing — the const has no error/panic paths. Widening visibility does NOT require new rustdoc sections for a `const`. **One caveat**: `clippy::pub_underscore_fields`/`clippy::missing_docs_in_private_items` could fire. The existing doc comment on `RUN_LEN` (`decode.rs:50`) is one line — keep it; `pub(crate)` requires documentation per the workspace convention and the comment is sufficient.

**Rustdoc snippets the planner may need (copy-paste ready if `missing_panics_doc` fires):**

```rust
/// Decode a validated 11-pair run into its `u64` value.
///
/// [... existing body comment unchanged ...]
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] with one of three messages:
/// * `"line {N}: non-digit byte at pair {P}"` — byte outside `b'0'..=b'9'`.
/// * `"line {N}: invalid base-60 digit {D} at pair {P}"` — digit `>= 60`.
/// * `"line {N}: decoded value exceeds u64::MAX"` — overflow on the final
///   `u128 → u64` conversion.
///
/// # Panics
///
/// Does not panic. Every failure path returns an [`io::Error`].
pub(crate) fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64> {
    // ... body unchanged ...
}
```

```rust
/// Total characters for 11 digit pairs joined by 10 colons.
///
/// Fixed at [`PAIR`] `*` [`DIGITS`] `+ (DIGITS - 1) = 33`. Used as the
/// compile-time array-size for every `&[u8; RUN_LEN]` parameter in this
/// module and as the length-gate in the `fuzz/parse_run` target.
pub(crate) const RUN_LEN: usize = PAIR * DIGITS + (DIGITS - 1);
```

(Adding the `fuzz/parse_run` reference to the doc comment is Claude's Discretion — flags to reviewers why the `pub(crate)` widening happened. Matches the convention in `crates/base60-cli/src/lib.rs:30-38` `__TuiTimeScale` doc.)

### Example 4: `crates/base60-core/benches/convert.rs` (D-23)

```rust
//! Benchmark for `base60_core::convert::u64_to_base60` — the hot-path
//! conversion called once per dump line.
//!
//! Input: 1024 deterministic `u64` values generated via `wrapping_mul`.
//! Run: `cargo bench -p base60-core --bench convert`. Advisory only —
//! see `../benches/README.md`.

use base60_core::convert::u64_to_base60;
use criterion::{Criterion, criterion_group, criterion_main};

/// Deterministic 1024-element input; seed pattern locked so re-runs are
/// bit-identical. `0x9E3779B97F4A7C15` is `2^64 / φ` (the Fibonacci hash
/// multiplier) — well-distributed bit patterns across 64 bits.
const FIB_MUL: u64 = 0x9E37_79B9_7F4A_7C15;
const INPUTS: [u64; 1024] = {
    let mut arr = [0_u64; 1024];
    let mut i = 0_u64;
    while i < 1024 {
        arr[i as usize] = i.wrapping_mul(FIB_MUL);
        i += 1;
    }
    arr
};

fn bench_u64_to_base60(c: &mut Criterion) {
    c.bench_function("u64_to_base60/1024_fib_inputs", |b| {
        b.iter(|| {
            let mut total: u64 = 0;
            for &n in &INPUTS {
                let digits = u64_to_base60(std::hint::black_box(n));
                total = total.wrapping_add(u64::from(digits[0]));
            }
            total
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_u64_to_base60
}
criterion_main!(benches);
```

Notes:
- `const` array with a `while`-loop initializer: compiles on stable since 1.79; our MSRV is 1.95. Verified by reading `crates/base60-cli/src/chunk.rs` which uses similar `const fn` idioms.
- `std::hint::black_box` prevents the optimiser from constant-folding the whole loop.
- `total.wrapping_add(u64::from(digits[0]))` prevents dead-code elimination of the function result.

### Example 5: `crates/base60-core/benches/lens.rs` (D-24)

```rust
//! Benchmark `Lens::render(&self, u64) -> String` for all four implementations.
//!
//! Phase 6 PERF-04 adds `render_to<W: Write>`; this bench gets extended
//! there. For Phase 5 we measure only the current `render` surface.
//! Run: `cargo bench -p base60-core --bench lens`.

use base60_core::{AngleLens, CuneiformLens, Lens, TabletLens, TimeLens};
use criterion::{Criterion, criterion_group, criterion_main};

const FIB_MUL: u64 = 0x9E37_79B9_7F4A_7C15;
const INPUTS: [u64; 1024] = {
    let mut arr = [0_u64; 1024];
    let mut i = 0_u64;
    while i < 1024 {
        arr[i as usize] = i.wrapping_mul(FIB_MUL);
        i += 1;
    }
    arr
};

fn bench_lens_render(c: &mut Criterion) {
    let mut g = c.benchmark_group("lens/render");

    let time = TimeLens::default();
    g.bench_function("time", |b| {
        b.iter(|| {
            for &n in &INPUTS {
                let s = time.render(std::hint::black_box(n));
                std::hint::black_box(s);
            }
        });
    });

    let angle = AngleLens;
    g.bench_function("angle", |b| {
        b.iter(|| {
            for &n in &INPUTS {
                std::hint::black_box(angle.render(std::hint::black_box(n)));
            }
        });
    });

    let tablet = TabletLens { purist: false };
    g.bench_function("tablet", |b| {
        b.iter(|| {
            for &n in &INPUTS {
                std::hint::black_box(tablet.render(std::hint::black_box(n)));
            }
        });
    });

    // Use `fallback: true` (ASCII path) for deterministic CI results —
    // `CuneiformLens::auto()` reads env vars which violates bench
    // reproducibility.
    let cuneiform = CuneiformLens { fallback: true };
    g.bench_function("cuneiform_ascii", |b| {
        b.iter(|| {
            for &n in &INPUTS {
                std::hint::black_box(cuneiform.render(std::hint::black_box(n)));
            }
        });
    });

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_lens_render
}
criterion_main!(benches);
```

Notes:
- `CuneiformLens::auto()` reads env vars (`NO_UNICODE`, `TERM`) — using `fallback: true` directly makes the bench deterministic.
- `benchmark_group` form lets all four lens variants appear as children of one `lens/render` group in criterion's HTML report.
- If Phase 6 PERF-04 ships `render_to<W>`, it extends this file with a sibling `lens/render_to` group.

### Example 6: `crates/base60-cli/benches/dump.rs` (D-25)

⚠️ **Visibility constraint:** `dump::dump_all` is `pub(crate)` today. A bench is an external binary that links only the `pub` surface of `base60::`. Solutions (Claude's Discretion per §"Arch Resp Map"):

- **Option A (recommended — `__bench` shim):** Add `#[doc(hidden)] pub mod __bench { pub use crate::dump::dump_all; pub use crate::color::PALETTE_NONE; ... }` to `lib.rs`; bench imports `base60::__bench::{dump_all, PALETTE_NONE};`.
- **Option B:** Widen `dump::dump_all`, `color::PALETTE_NONE`, `decode::decode_stream`, `search::find_all` to `#[doc(hidden)] pub`. More scattered.

Bench body (assuming Option A):

```rust
//! Benchmark `dump::dump_all` throughput on a 1 MiB compile-time-constant
//! byte array with the monochrome palette and no lens.
//!
//! Phase 6 PERF-01 may extend this with a streaming-path comparison.
//! Run: `cargo bench -p base60 --bench dump`. Advisory only — see README.md.

use base60::__bench::{PALETTE_NONE, dump_all};
use criterion::{Criterion, criterion_group, criterion_main};
use std::io::sink;

const SIZE: usize = 1 << 20; // 1 MiB
const INPUT: [u8; SIZE] = {
    let mut arr = [0_u8; SIZE];
    let mut i: usize = 0;
    // Deterministic pseudo-random fill via wrapping u8 arithmetic (D-28).
    // No `rand` dep; same bytes every run.
    while i < SIZE {
        // Mix with a linear-congruential-ish stepper so no sub-byte bias.
        let b = (i.wrapping_mul(13).wrapping_add(7)) as u8;
        arr[i] = b;
        i += 1;
    }
    arr
};

fn bench_dump_all_mono(c: &mut Criterion) {
    c.bench_function("dump_all/1mib_mono_no_lens", |b| {
        b.iter(|| {
            // `sink()` drains the writer — no real I/O, no allocation per line.
            let _ = dump_all(
                std::hint::black_box(&INPUT),
                0,
                sink(),
                &PALETTE_NONE,
                None,
            );
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_dump_all_mono
}
criterion_main!(benches);
```

Notes:
- Using `std::io::sink` means I/O overhead is zero — bench measures `dump_all`'s formatting throughput alone.
- 1 MiB input × ~131k lines = reasonable sample size; criterion's `sample_size(50)` gives good variance.
- `PALETTE_ANSI` cell is Claude's Discretion per CONTEXT §J (recommendation: skip for Phase 5; Phase 6 PERF-04 can add it). Keep bench focused.

### Example 7: `crates/base60-cli/benches/decode.rs` (D-26)

```rust
//! Benchmark `decode::decode_stream` throughput over a pre-computed 1 MiB
//! dump. Dump generation runs once per bench process via `LazyLock`; only
//! `decode_stream` is inside the `b.iter(...)` block.
//!
//! Run: `cargo bench -p base60 --bench decode`. Advisory only — see README.md.

use base60::__bench::{PALETTE_NONE, decode_stream, dump_all};
use base60::InputFormat; // public from Phase 4 CLI
use criterion::{Criterion, criterion_group, criterion_main};
use std::io::sink;
use std::sync::LazyLock;

const SIZE: usize = 1 << 20; // 1 MiB raw input
const RAW: [u8; SIZE] = {
    let mut arr = [0_u8; SIZE];
    let mut i: usize = 0;
    while i < SIZE {
        arr[i] = (i.wrapping_mul(13).wrapping_add(7)) as u8;
        i += 1;
    }
    arr
};

// Render the 1 MiB raw input to plain-text dump bytes exactly once; reuse
// in every iteration. LazyLock keeps the cost out of the `b.iter` block.
static DUMPED: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut out = Vec::with_capacity(SIZE * 5); // rough upper bound
    dump_all(&RAW, 0, &mut out, &PALETTE_NONE, None).expect("dump to Vec cannot fail");
    out
});

fn bench_decode_stream(c: &mut Criterion) {
    c.bench_function("decode_stream/1mib_plain_no_lens", |b| {
        b.iter(|| {
            let dumped: &[u8] = std::hint::black_box(&DUMPED);
            // Auto-detect routes to the plain/ansi text decoder (Phase 4 D-06).
            let _ = decode_stream(dumped, &mut sink(), InputFormat::Plain);
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_decode_stream
}
criterion_main!(benches);
```

Notes:
- The `.expect("dump to Vec cannot fail")` is sound because `Vec<u8>`'s `Write` impl is infallible.
- `InputFormat::Plain` (explicit override) bypasses the sniff step — keeps the decode-only measurement clean.
- `InputFormat` must be reachable from `base60::` — currently `pub` via `cli::InputFormat` (Phase 4 D-06 adds the `--input-format` flag). Verify via `cargo doc` that it's re-exported from the CLI lib root. If not, extend `__bench` shim.

### Example 8: `crates/base60-cli/benches/search.rs` (D-27, mandatory cells)

```rust
//! Benchmark `search::find_all` with the four mandatory cells from
//! PITFALLS Pitfall 4. This bench is the gating baseline for Phase 6
//! PERF-03 (`memchr::memmem` swap).
//!
//! Run: `cargo bench -p base60 --bench search`. Advisory only — see README.md.

use base60::__bench::find_all;
use criterion::{Criterion, criterion_group, criterion_main};

const HAY_SIZE: usize = 1 << 20; // 1 MiB haystack

const ZERO_FILL: [u8; HAY_SIZE] = [0_u8; HAY_SIZE];

const RANDOM_FILL: [u8; HAY_SIZE] = {
    let mut arr = [0_u8; HAY_SIZE];
    let mut i: usize = 0;
    while i < HAY_SIZE {
        arr[i] = (i.wrapping_mul(13).wrapping_add(7)) as u8;
        i += 1;
    }
    arr
};

fn bench_find_all(c: &mut Criterion) {
    let mut g = c.benchmark_group("find_all");

    // Cell 1: 1-byte needle on zero-fill haystack (1-byte dispatch).
    g.bench_function("zero_fill/1byte_null", |b| {
        b.iter(|| find_all(std::hint::black_box(&ZERO_FILL), std::hint::black_box(b"\x00")));
    });

    // Cell 2: 2-byte needle on zero-fill haystack (packed-pair prefilter).
    g.bench_function("zero_fill/2byte_ffff", |b| {
        b.iter(|| find_all(std::hint::black_box(&ZERO_FILL), std::hint::black_box(b"\xff\xff")));
    });

    // Cell 3: 3-byte needle on random haystack.
    g.bench_function("random/3byte_elf", |b| {
        b.iter(|| find_all(std::hint::black_box(&RANDOM_FILL), std::hint::black_box(b"ELF")));
    });

    // Cell 4: 8-byte needle on random haystack.
    g.bench_function("random/8byte_cafebabe", |b| {
        b.iter(|| {
            find_all(
                std::hint::black_box(&RANDOM_FILL),
                std::hint::black_box(b"cafebabe"),
            )
        });
    });

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_find_all
}
criterion_main!(benches);
```

Notes:
- 2-byte `\xff\xff` will find ZERO matches on zero-fill — exactly the adversarial case that trips the packed-pair prefilter.
- 3-byte `b"ELF"` on random input is realistic (ELF magic is 0x7F45 4C46 → the ASCII bytes "ELF" match conceptually).
- All `black_box` calls ensure the optimiser doesn't cache results across iterations.

### Example 9: `crates/base60-cli/benches/README.md` (D-29, canonical)

```markdown
# Benchmarks — advisory only, NEVER CI-gated

These `criterion` benches are a local-only baseline-tracking tool, not a
CI gate. Shared GitHub Actions runners have a 10–15% noise floor that
exceeds any reasonable threshold (PROJECT.md Key Decision row 8;
PITFALLS.md Pitfall 9). CI will **never** run `cargo bench`; Phase 7 SC4
only adds a `cargo bench --workspace --no-run --locked` compile smoke.

## Running locally

```bash
# Capture a baseline on the current commit:
cargo bench -p base60 --bench <name> -- --save-baseline pre

# Apply your change, then compare:
cargo bench -p base60 --bench <name> -- --baseline pre
```

Or for all benches across the workspace:

```bash
cargo bench --workspace -- --save-baseline pre
# ... make changes ...
cargo bench --workspace -- --baseline pre
```

Paste the before/after numbers into the PR description. Reviewers look
at the delta, not a CI checkmark.

## Per-bench scope

| Bench file | Target | Why it exists |
|-----------|--------|---------------|
| `base60-core/benches/convert.rs` | `u64_to_base60` hot loop | Every dump line calls this; regression gate for future `convert` work |
| `base60-core/benches/lens.rs` | `Lens::render` × 4 impls | Baseline for Phase 6 PERF-04 `render_to<W>` migration |
| `base60-cli/benches/dump.rs` | `dump_all` over 1 MiB mono | Baseline for Phase 6 PERF-01 streaming path |
| `base60-cli/benches/decode.rs` | `decode_stream` over 1 MiB dump | Protects roundtrip perf; no REQ-IDs currently depend on it but cheap to track |
| `base60-cli/benches/search.rs` | `find_all` × 4 cells | **Gates Phase 6 PERF-03** `memchr::memmem` swap (PITFALLS Pitfall 4). Every cell must not regress when the swap lands. |

## Noise threshold

Every `Criterion::default()` instance in this project uses
`noise_threshold(0.05)` — 5% tolerance. That's comfortable for a quiet
laptop; shared CI runners would need 10–15%, which is why CI never runs
these.

## Determinism

Bench inputs are compile-time `const` arrays filled via `wrapping_mul` /
`wrapping_add` — no `rand` dep, no system clock, no env vars. Re-runs on
the same machine produce bit-identical input bytes.
```

Wording of the README is Claude's Discretion per CONTEXT §J. Above is a concrete shape matching D-29 requirements (advisory posture, reproducer commands, noise-floor caveat, determinism note).

### Example 10: `crates/base60-core/benches/README.md` (D-30, one-liner)

```markdown
# Benchmarks

See [`../../base60-cli/benches/README.md`](../../base60-cli/benches/README.md)
for the project-wide advisory-only bench posture. Both crates' benches
follow the same workflow.
```

### Example 11: `fuzz/README.md` (planner writes — CONTEXT canonical_refs NEW)

```markdown
# base60 fuzz targets

Ubuntu + pinned nightly only. libFuzzer requires LLVM sanitizer support
(x86_64 / aarch64, Unix-like, nightly-only). The main workspace CI
matrix remains Ubuntu/macOS/Windows × stable/beta/1.95 because `fuzz/`
is workspace-excluded (root `Cargo.toml` `exclude = ["fuzz"]` +
`fuzz/Cargo.toml` nested `[workspace]` per `--fuzzing-workspace=true`).

## Running locally

```bash
cargo install cargo-fuzz
rustup toolchain install nightly

cd fuzz
cargo +nightly fuzz run parse_run           # default timeout (runs until Ctrl-C)
cargo +nightly fuzz run parse_run -- -max_total_time=30   # 30-second smoke
cargo +nightly fuzz run pattern_from_str -- -max_total_time=30
```

Parallel across cores:

```bash
cargo +nightly fuzz run --jobs 8 parse_run
```

Reproduce a crash artifact:

```bash
cargo +nightly fuzz run parse_run fuzz/artifacts/parse_run/crash-<hash>
# Verify the crash survives release-mode optimisation:
cargo +nightly fuzz run --release parse_run fuzz/artifacts/parse_run/crash-<hash>
```

Corpus minimisation (once corpora have grown):

```bash
cargo +nightly fuzz cmin parse_run
```

## Targets

| Target | Drives | REQ |
|--------|--------|-----|
| `parse_run` | `base60::__fuzz::parse_run(&[u8; 33], line_no)` — the hot path of `decode::decode_stream` (Phase 4 D-09). Length-gated inside the target so libFuzzer's mutator can grow inputs without false positives. | TEST-02 |
| `pattern_from_str` | `base60::__fuzz::Pattern::from_str(&str)` — parses user `/search` input in the TUI. UTF-8-guarded because the fn takes `&str`. | TEST-02 |

Both targets use `let _ = ...;` to ignore `Result` — `Err` returns are the
happy path; only panics are bugs (PITFALLS.md Pitfall 3).

## Platform

- **Ubuntu + nightly:** fully supported. Target platform for Phase 7
  CI-02's weekly job.
- **macOS:** works in principle on aarch64 nightly, but sanitizer
  toolchain is less predictable. Not CI-tested.
- **Windows:** unsupported — libFuzzer needs Unix-like sanitizer
  support.

## CI integration

None in Phase 5 (this phase ships scaffolding only). Phase 7 CI-02 adds
a weekly `schedule:` workflow: `cargo +nightly fuzz run <target> --
-max_total_time=240` on `ubuntu-latest`, `timeout-minutes: 5`,
non-gating.

## Seed corpus

Empty on commit (Phase 5 D-09). Directories `corpus/`, `artifacts/`, and
`target/` are gitignored. If weekly CI corpus growth stalls under ~1000
entries after Phase 7 has run for a few weeks, add hand-crafted seeds to
`fuzz/seeds/<target>/*` and document here (deferred idea — not now).
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `#![feature(test)]` nightly benches | `criterion` 0.8 with `cargo_bench_support` | criterion 0.8.x line; moved to `criterion-rs` GitHub org ~2025 | Stable-channel benches; no MSRV violation. Save-and-compare baselines for free. |
| `arbitrary` crate as a structured-input mandate | Raw `&[u8]` + length/UTF-8 guards | rust-fuzz book recommends "start without it" | No derive dep; targets work on primitive inputs; structured input added later if needed (D-15). |
| `fuzz/` as a workspace member | `fuzz/` with its own `[workspace]` via `--fuzzing-workspace=true` | cargo-fuzz introduced `--fuzzing-workspace` ~2024 | Keeps nightly/sanitizer flags out of the main `Cargo.lock`; main 3×3 CI matrix stays green. |
| `libfuzzer-sys` defaults include `arbitrary` feature | Defaults are only `["link_libfuzzer"]`; `arbitrary-derive` is opt-in | libfuzzer-sys 0.4.x line | CONTEXT's `default-features = false, features = ["link_libfuzzer"]` is net-identical to default; explicit for intent-documentation. |

**Deprecated / outdated:**

- `lazy_static` → stdlib `std::sync::LazyLock` (stable since Rust 1.80). Bench examples use `LazyLock` — matches `base60-core/src/cuneiform.rs`'s existing use.
- `once_cell` → same, subsumed by stdlib.
- `#![feature(test)]` + `#[bench]` — requires nightly for every developer; criterion removes that floor.
- `divan` (0.1.x) — pre-1.0; STACK.md rejects it for Phase 5.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `[doc(hidden)] pub mod __bench` shim is the cleanest way to expose `pub(crate)` items to `benches/*.rs` | §"Architectural Responsibility Map" | [ASSUMED] — alternative is to widen individual items to `#[doc(hidden)] pub` in their home modules. Both work; shim keeps one place to audit. If planner prefers per-item widening, benches still work; small organisation difference only. |
| A2 | `const` array initialiser with `while` loop compiles on MSRV 1.95 | §"Code Examples" (bench skeletons) | [VERIFIED: MSRV 1.95 > 1.79 when `const` body `while` was stabilised; existing codebase uses similar `const fn` idioms in `base60-cli/src/chunk.rs`]. Low risk. |
| A3 | `InputFormat` is publicly reachable from the CLI lib root (`base60::InputFormat`) after Phase 4 | §"Code Examples" Example 7 | [VERIFIED: read `lib.rs` — `pub use cli::{LensMode, Format}` and `#[doc(hidden)] pub use cli::TimeScale as __TuiTimeScale`. `InputFormat` is NOT re-exported. The `__bench` shim must include it: `pub use crate::cli::InputFormat;`.] Confirmed assumption — must be in the shim. |
| A4 | `cargo fuzz init --fuzzing-workspace=true` generates `.gitignore` with `target`, `corpus`, `artifacts`, `coverage` | §"Standard Stack" | [VERIFIED via github.com/rust-fuzz/cargo-fuzz/src/templates.rs — exact content shown above.] HIGH confidence. |
| A5 | Setting `resolver = "3"` inside the nested `[workspace]` of `fuzz/Cargo.toml` is the safe default | §"Standard Stack" | [ASSUMED] — cargo will default to `resolver = "2"` for edition≥2021; explicit `resolver = "3"` matches root workspace. If planner picks neither, cargo MAY emit a warning on nested-workspace; planner should run `cargo +nightly build --manifest-path fuzz/Cargo.toml` after scaffolding and respond to any resolver-version warning. Low risk. |
| A6 | `CuneiformLens::auto()` reads env vars at construction, making it unsuitable for deterministic benches | §"Code Examples" Example 5 | [VERIFIED: read `crates/base60-core/src/lens.rs:166-171` — `auto()` calls `cuneiform::ascii_fallback_forced()` which reads `NO_UNICODE` / `TERM`.] HIGH confidence. |
| A7 | `criterion` 0.8.2's `features = ["cargo_bench_support", "html_reports"]` is a minimal, non-rayon feature set | §"Standard Stack" | [CITED: STACK.md §"Benchmarking" + Context7 `/bheisler/criterion.rs` confirmed `cargo_bench_support` is required without nightly feature(test); `html_reports` keeps plotters-backed HTML. Dropping `rayon` avoids parallelism noise on streaming measurements.] HIGH confidence. |
| A8 | `fuzz/README.md` is required per CONTEXT canonical_refs "NEW (Plan 05-01 — fuzz)" list | §"Code Examples" Example 11 | [VERIFIED: CONTEXT.md line 239 explicitly lists `fuzz/README.md`.] HIGH confidence. |

**If any of A1, A3, A5 prove wrong at plan-execution time, the planner should pivot in-place rather than escalate** — they're implementation-detail choices with equivalent-correct alternatives.

## Open Questions (RESOLVED)

> All four questions below have concrete recommendations; two are VERIFIED/CITED.
> Items 1 and 4 are non-blocking (post-execution smoke paths); 2 and 3 cite external evidence.

1. **RESOLVED — Does `cargo test --workspace --all-targets --locked` try to build the benches in test mode?**
   - What we know: criterion's `harness = false` tells cargo "don't run libtest's default harness for this target." But `--all-targets` normally includes `bench` targets.
   - What's unclear: does `harness = false` + `cargo test` translate to a 30-second bench run in test mode? CONTEXT.md line 295 flags this as a concern with mitigation ("if the aggregate adds more than 5s per CI cell, switch to `#[cfg(not(test))]`-gated `criterion_main!`").
   - Recommendation: planner runs `cargo test --workspace --all-targets --locked` locally AFTER Plan 05-02 lands; if wall-clock increases > 5s per cell, open a planner note to add `#[cfg(not(test))]` gating. criterion's docs say `cargo test --bench <name>` runs as a smoke test (each bench function runs one iteration). The aggregate should be small. [VERIFIED per Context7 `/bheisler/criterion.rs` `cargo_bench_support` feature docs — the harness runs a "smoke" iteration under `cargo test`.]

2. **RESOLVED — Does `base60::__fuzz::RUN_LEN` interact with `#[cfg(fuzzing)]` correctly when referenced from a fuzz_target body as `base60::__fuzz::RUN_LEN`?**
   - What we know: `#[cfg(fuzzing)]` is ONLY set during `cargo fuzz build/run` compilations. The fuzz target crate (`fuzz/` workspace) compiles its binary targets with `--cfg fuzzing` propagated through path-deps to `base60` crate. So in the fuzz target compilation, `base60::__fuzz` IS present.
   - What's unclear: does cargo-fuzz's `--cfg fuzzing` propagate to the path-dep `base60` automatically, or does the fuzz manifest need `[target.'cfg(fuzzing)']` indirection?
   - Recommendation: [CITED: STACK.md line 239 "`cargo-fuzz` sets `--cfg fuzzing` on every compilation unit in the graph, so conditional exposure is clean."] Planner smoke-tests `cd fuzz && cargo +nightly fuzz build` after implementing D-05 — if `__fuzz::parse_run` is unresolved, it means cargo-fuzz changed its cfg propagation and we need a different gate (e.g., a fuzz-specific feature flag on `base60` crate).

3. **RESOLVED — Should `benches-compile` sanity check be a new xtask or left for Phase 7?**
   - CONTEXT excludes CI changes from Phase 5 scope. Phase 7 SC4 adds `cargo bench --workspace --no-run --locked`.
   - Recommendation: Leave for Phase 7 per scope. Manual check in Plan 05-02: developer runs `cargo bench --workspace --no-run --locked` before committing; planner adds this to the plan's validation checklist.

4. **RESOLVED — Does `pub(crate) const RUN_LEN` trigger `clippy::missing_panics_doc` or `missing_docs_in_private_items`?**
   - What we know: the existing const has a one-line doc comment (`decode.rs:50`).
   - What's unclear: widening to `pub(crate)` — does clippy's `missing_docs_in_private_items` demand more?
   - Recommendation: the existing 1-line doc comment is likely sufficient; if clippy fires, planner expands it (pattern shown in Example 3). Non-blocking.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| `cargo` (stable/1.95) | Main workspace compile, all `cargo test/doc/clippy/fmt` | ✓ | 1.95+ (workspace MSRV floor) | — |
| `cargo` (nightly toolchain) | `cargo fuzz build/run` (Plan 05-01 smoke test) | To verify locally: `rustup toolchain install nightly` | nightly (any recent) | Developer installs via `rustup`; CI will install in Phase 7 |
| `cargo-fuzz` subcommand | `cargo fuzz init`, `cargo fuzz run` | To verify locally: `cargo install cargo-fuzz` | `0.13.1` [VERIFIED 2026-04-24] | Developer installs via `cargo install`; CI in Phase 7 |
| libFuzzer runtime | `libfuzzer-sys = "0.4"` with `link_libfuzzer` feature | ✓ (vendored via `libfuzzer-sys`; no system install needed) | 0.4.12 | — |
| LLVM sanitizer support (`-Zsanitizer=address`) | Nightly compile of `fuzz/` crate | ✓ on Ubuntu x86_64/aarch64 with nightly | Built into nightly rustc | — (Ubuntu+nightly only; macOS/Windows out of scope per Pitfall 11) |
| Linux kernel userfaultfd/asan features | libFuzzer runtime instrumentation | ✓ on `ubuntu-latest` GHA runners | Kernel-agnostic | — |
| `cargo bench` (stable) | `cargo bench --workspace --no-run --locked` smoke (Plan 05-02 manual check) | ✓ | Part of cargo | — |

**Missing dependencies with no fallback:** None for Phase 5 file-creation work. `cargo-fuzz` and nightly toolchain are only required for the Plan 05-01 post-landing manual smoke test (D-33); the scaffolding itself compiles fine without them as long as the main workspace's `cargo check` succeeds.

**Missing dependencies with fallback:** None.

**Developer-prerequisite check** (gsd-executor should run before Plan 05-01):

```bash
command -v cargo && cargo --version
rustup toolchain list | grep -q nightly || echo "Install nightly: rustup toolchain install nightly"
command -v cargo-fuzz || echo "Install: cargo install cargo-fuzz"
```

If nightly or cargo-fuzz are missing, install them before the smoke test step. If they can't be installed (sandbox / CI restrictions), the scaffolding commit still lands; only the manual smoke (D-33) is blocked.

## Validation Architecture

> Phase 5 ships **scaffolding only** — no behaviour change, no new runtime code paths. The minimum validation surface reflects this. Most SC1–SC5 verification is manual (developer laptop) because Phase 7 owns the CI integration.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `cargo test` (existing); criterion 0.8 for bench compile-smoke |
| Config file | None (rustfmt, clippy, doc config inherited from workspace `[workspace.lints]` and workspace `Cargo.toml`) |
| Quick run command | `cargo test --workspace --all-targets --locked` (main workspace; excludes `fuzz/` by root-manifest `exclude`) |
| Full suite command | `cargo test --workspace --all-targets --locked` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --locked` + `cargo fmt --all --check` (Phase 3 D-24 gate, reused) |
| Fuzz smoke | `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` (D-33; manual, Plan 05-01 only) |
| Bench compile smoke | `cargo bench --workspace --no-run --locked` (D-33; manual, Plan 05-02 only) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| **TEST-02 SC1** (fuzz init + exclude + two targets with `let _ = ...`) | `fuzz/` exists, compiles, two targets present, `let _ = ...` pattern used | Manual-only (scaffolding presence) | `ls fuzz/ && cd fuzz && cargo +nightly fuzz build` | ❌ Wave 0 (planner creates during Plan 05-01) |
| **TEST-02 SC2** (30-second no-crash run) | `cargo +nightly fuzz run parse_run -- -max_total_time=30` exits 0 | Manual-only | Same command | ❌ Plan 05-01 smoke |
| **TEST-02 SC5** (`#[cfg(fuzzing)] pub` hatch doesn't leak) | Non-fuzz `cargo check --workspace` + `cargo doc --workspace --no-deps --locked` shows no new `pub` items under `base60` outside the hatch | Manual + inline test | Manual: `cargo doc` diff. Inline: `#[test] #[cfg(not(fuzzing))] fn fuzz_module_absent()` in lib.rs tests | ❌ Plan 05-01 — add inline test |
| **PERF-06 SC3** (5 bench files, `harness = false`, `noise_threshold(0.05)`) | All 5 benches compile and configure the 5% threshold | Automated compile smoke | `cargo bench --workspace --no-run --locked` | ❌ Wave 0 (planner creates during Plan 05-02) |
| **PERF-06 SC3** (compile on all 3 OSes) | Benches compile on Ubuntu/macOS/Windows | CI (inherits existing `cargo test --all-targets` which includes `[[bench]]` targets; criterion compiles even under `cargo test`) | `cargo test --workspace --all-targets --locked` on all 3 OS × 3 rustc cells | ✓ (existing CI picks up new benches automatically) |
| **PERF-06 SC4** (`benches/README.md` advisory posture) | `crates/base60-cli/benches/README.md` exists + mentions "advisory only, never CI-gated" | File presence + grep assertion | Manual grep; OR add xtask test if rigour wanted: `grep -q "advisory only" crates/base60-cli/benches/README.md` | ❌ Plan 05-02 creates the file |

### Sampling Rate

- **Per task commit:** full Phase 3 D-24 gate (`cargo fmt --all --check` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo test --workspace --all-targets --locked` + `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --locked`). Approximately 90 seconds on a modern dev laptop.
- **Per wave merge:** same — Phase 5 has exactly two commits (waves), and the gate runs between both.
- **Phase gate** (`/gsd-verify-work` or manual before transition): full suite green on developer machine + `cd fuzz && cargo +nightly fuzz build` succeeds + `cargo bench --workspace --no-run --locked` succeeds + `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` exits 0.

### Wave 0 Gaps (files the planner creates before any "implementation" happens)

The entire Phase 5 artefact surface IS Wave 0. There is no pre-existing scaffolding to reuse beyond the thin `[lib]` target (Phase 3 shipped) and the Phase 4 `decode.rs` signature. Specifically:

- [ ] `fuzz/` directory (entire subtree) — Plan 05-01 scaffolds via `cargo fuzz init --fuzzing-workspace=true`.
- [ ] `fuzz/Cargo.toml` — hand-edited after init per D-03/D-04.
- [ ] `fuzz/fuzz_targets/parse_run.rs` — covers TEST-02 SC1 (Plan 05-01).
- [ ] `fuzz/fuzz_targets/pattern_from_str.rs` — covers TEST-02 SC1 (Plan 05-01).
- [ ] `fuzz/README.md` — covers TEST-02 platform-constraint documentation (Plan 05-01).
- [ ] `fuzz/.gitignore` — auto-generated; planner verifies content matches D-10.
- [ ] `crates/base60-cli/src/lib.rs` — add `__fuzz` module (Plan 05-01) + `__bench` module (Plan 05-02) + optional `#[test] #[cfg(not(fuzzing))] fn fuzz_module_absent()` (Plan 05-01, SC5 belt-and-braces).
- [ ] `crates/base60-cli/src/decode.rs` — visibility bump + rustdoc additions (Plan 05-01).
- [ ] `crates/base60-core/benches/convert.rs` — covers PERF-06 SC3 (Plan 05-02).
- [ ] `crates/base60-core/benches/lens.rs` — covers PERF-06 SC3 (Plan 05-02).
- [ ] `crates/base60-core/benches/README.md` — covers PERF-06 SC4 (Plan 05-02).
- [ ] `crates/base60-cli/benches/dump.rs` — covers PERF-06 SC3 + gates Phase 6 PERF-01 (Plan 05-02).
- [ ] `crates/base60-cli/benches/decode.rs` — covers PERF-06 SC3 (Plan 05-02).
- [ ] `crates/base60-cli/benches/search.rs` — covers PERF-06 SC3 + gates Phase 6 PERF-03 (Plan 05-02, Pitfall 4 mandatory cells).
- [ ] `crates/base60-cli/benches/README.md` — covers PERF-06 SC4 (Plan 05-02).
- [ ] Root `Cargo.toml` — add `exclude = ["fuzz"]` (Plan 05-01).
- [ ] `crates/base60-core/Cargo.toml` — add criterion dev-dep + 2 `[[bench]]` blocks (Plan 05-02).
- [ ] `crates/base60-cli/Cargo.toml` — add criterion dev-dep + 3 `[[bench]]` blocks (Plan 05-02).
- [ ] Framework install (developer/CI-side): `cargo install cargo-fuzz` + `rustup toolchain install nightly` — developer responsibility for manual D-33 smoke; Phase 7 CI-02 installs in the workflow.

### How Phase 5 avoids false-CI-coverage claims

- **Nothing Phase 5 ships runs in CI beyond the existing `cargo test --workspace --all-targets --locked` path.** The bench `[[bench]]` targets compile under that command (cargo's default `--all-targets` scope), so CI catches bench code-rot for free. The fuzz crate is workspace-excluded (D-02) + nested-workspace-isolated (D-01), so CI never tries to build it.
- **SC1 (fuzz build) and SC2 (30s no-crash)** are validated manually in Plan 05-01's commit message (developer runs the command, captures a one-line result, commits). Phase 7 CI-02 turns this into an actual CI step.
- **SC3 (`noise_threshold(0.05)`, all-OS compile)** is validated by CI's `cargo test --workspace --all-targets --locked` on the existing 3 OS × 3 rustc matrix. If the benches fail to compile on any cell, existing CI turns red — no new YAML needed.
- **SC4 (advisory-only README)** is validated by file presence + manual review. Could add an xtask grep gate later, but not needed for Phase 5 scope.
- **SC5 (hatch doesn't leak)** is validated by (a) the `#[cfg(fuzzing)]` gate itself (non-fuzz builds don't compile the module — the compiler is the gate), (b) an optional inline `#[test] #[cfg(not(fuzzing))]` sanity test, and (c) manual `cargo doc --workspace --no-deps --locked` diff review at commit time.

**Planner MUST NOT claim CI coverage for SC1, SC2, or the fuzz smoke test.** Those are manual checks until Phase 7 CI-02 lands.

## Security Domain

> Required per `security_enforcement` default (absent = enabled). Phase 5's attack surface is minimal: two fuzz targets that explicitly guard against false-positive crashes, and bench harnesses that run local-only. No secret material, no network, no untrusted input flowing through the scaffolding.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | — (CLI; no authentication surface) |
| V3 Session Management | no | — (no sessions) |
| V4 Access Control | no | — (single-process binary) |
| **V5 Input Validation** | **yes** | Fuzz targets deliberately validate malformed inputs. Both targets use `let _ = ...` + guard clauses (length-gate on `parse_run`, UTF-8 check on `Pattern::from_str`) to prevent false-positive crash reports from expected `Err` returns. This is the mirror-image of ASVS V5 — instead of rejecting bad input at runtime, we exercise the REJECT code path under fuzz. PITFALLS Pitfall 3 is the canonical concern. |
| V6 Cryptography | no | — (no cryptography; the pseudo-random `wrapping_mul` generator is NOT used for security, only determinism) |
| V7 Data Protection | no | — (no sensitive data) |
| V8 Error Handling | partial | Fuzz targets must not convert `Err` into panic via `.unwrap()` — PITFALLS Pitfall 3 mitigation. Bench harness uses `.expect("dump to Vec cannot fail")` ONLY where the failure is impossible (Vec's infallible Write impl); never on user input. |
| V9 Communications | no | — (no network) |
| V10 Malicious Code | low | Scaffolding introduces no new runtime code; dev-deps come from `crates.io` via `Cargo.lock` which CI verifies with `--locked`. `libfuzzer-sys` vendors libFuzzer so no system library is trusted. |
| V11 Business Logic | no | — (no business logic introduced) |
| V12 Files & Resources | low | Fuzz corpus directories (`corpus/`, `artifacts/`) are gitignored (D-10) so accidental commit of a crash-trigger input containing sensitive bytes (unlikely — bytes are mutated from the RNG, but still best-practice) is structurally prevented. |

### Known Threat Patterns for Fuzz + Bench Scaffolding

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Fuzz corpus accidentally committed containing sensitive bytes (PII / keys) | Information Disclosure | `fuzz/.gitignore` auto-generated with `corpus` + `artifacts` entries (D-10) — cargo-fuzz template. |
| False-positive fuzz "crashes" drown real bugs | Repudiation / Observability | `let _ = ...;` + UTF-8/length guards (D-12, D-13); banner comment on happy-path Err returns (D-14); no `panic::catch_unwind` (cargo-fuzz is `-Cpanic=abort`). PITFALLS Pitfall 3. |
| Bench input via untrusted source (system RNG, env, clock) → non-deterministic baselines → regressions hidden | Tampering / Observability | Deterministic `const` arrays via `wrapping_mul` / `wrapping_add` (D-28); no `rand` dep; no `std::time` usage in bench bodies. |
| Visibility hatch leaks into shipped binary → downstream depends on internal API | Repudiation / ABI drift | `#[cfg(fuzzing)]` gate on `__fuzz` module (D-05); non-fuzz builds physically cannot reference the items. `__bench` shim uses `#[doc(hidden)]` only (always compiled) — less strict, so planner verifies `cargo doc --workspace --no-deps --locked` output shows no new visible items. |
| Malicious dev-dep pulls in crates outside the Cargo.lock baseline | Supply Chain | `--locked` flag on every CI command (already enforced); Phase 7 CI-03 adds a grep-based zero-dep-runtime check on `base60-core`. Phase 5's `criterion` + transitive graph will land in `Cargo.lock` at first build — reviewer should eyeball the diff in Plan 05-02's commit. |
| Fuzz target runs on untrusted runner → sanitizer bypass or AV issue | Elevation of Privilege | Phase 5 doesn't run fuzz in CI. Phase 7 CI-02 runs on GHA `ubuntu-latest` (GitHub's managed trust zone). No self-hosted runners. |

**Defensive notes for the planner:**

- When the `Cargo.lock` diff lands (first time `criterion` is resolved), inspect for unexpected transitive additions. Known-expected additions (from STACK.md): `plotters`, `plotters-backend`, `plotters-svg`, `criterion`, `criterion-plot`, `oorandom` (criterion's internal RNG, NOT a security-sensitive one), `anes` (possibly, from criterion), `is-terminal` (terminal-detection), `ciborium` (criterion report serialisation). Each is `dev-dependencies`-transitive — it affects only bench compile, not shipped binary.
- `libfuzzer-sys` adds a `cc = "^1.0.83"` build-dep to compile the vendored libFuzzer C code. Expected.
- No new `[dependencies]` on `base60-core` — CI-03 invariant preserved. ✅ (Verified against the D-17 declaration: entry goes in `[dev-dependencies]`.)

## Plan Split Verification

CONTEXT.md expects **2 plans** (D-31). Verification of disjoint-file claim (D-32):

| File | Plan 05-01 (fuzz) | Plan 05-02 (benches) | Overlap? |
|------|--------------------|----------------------|----------|
| `fuzz/Cargo.toml` | CREATE | — | No |
| `fuzz/fuzz_targets/parse_run.rs` | CREATE | — | No |
| `fuzz/fuzz_targets/pattern_from_str.rs` | CREATE | — | No |
| `fuzz/.gitignore` | CREATE (auto) | — | No |
| `fuzz/README.md` | CREATE | — | No |
| `Cargo.toml` (root) | EDIT (add `exclude = ["fuzz"]`) | — | No |
| `crates/base60-cli/src/lib.rs` | EDIT (add `__fuzz` module) | **EDIT** (add `__bench` module if the planner chooses Option A in §"Arch Resp Map") | **POTENTIAL OVERLAP** |
| `crates/base60-cli/src/decode.rs` | EDIT (visibility bump + rustdoc) | — | No |
| `crates/base60-cli/src/search.rs` | — (no change per D-07) | — | No |
| `crates/base60-core/Cargo.toml` | — | EDIT (add criterion dev-dep + 2 `[[bench]]` blocks) | No |
| `crates/base60-cli/Cargo.toml` | — | EDIT (add criterion dev-dep + 3 `[[bench]]` blocks) | No |
| `crates/base60-core/benches/*.rs` | — | CREATE | No |
| `crates/base60-core/benches/README.md` | — | CREATE | No |
| `crates/base60-cli/benches/*.rs` | — | CREATE | No |
| `crates/base60-cli/benches/README.md` | — | CREATE | No |

**Overlap analysis:**

- **`crates/base60-cli/src/lib.rs`**: Plan 05-01 adds `#[doc(hidden)] #[cfg(fuzzing)] pub mod __fuzz { ... }`. If the planner adopts Option A (`__bench` shim) from §"Arch Resp Map", Plan 05-02 adds `#[doc(hidden)] pub mod __bench { ... }` to the same file. This IS an overlap at the file level, but NOT at the line level — the two modules are independent sibling inserts at crate root.
- **Serial ordering (D-32) already mandates sequence** 05-01 → 05-02, so the overlap is serialised automatically. No merge conflict possible under serial execution.
- **Parallel-safe claim in CONTEXT D-32** is slightly overstated given the lib.rs touch from Plan 05-02. Since the plans are ordered serially anyway, this is not a real problem — note it as a "both touch lib.rs but at different logical sites; serial ordering mandatory."

**Alternative (eliminates overlap):** Planner adopts Option B from §"Arch Resp Map" — widen the needed items (`dump_all`, `decode_stream`, `find_all`, `PALETTE_NONE`, `InputFormat`) to `#[doc(hidden)] pub` in their home modules. Then `lib.rs` stays untouched by Plan 05-02. Trade-off: more files edited in Plan 05-02 (5 source files instead of 1 lib.rs), but truly disjoint from Plan 05-01.

**Recommendation:** Accept the lib.rs overlap and enforce serial ordering (D-32). The planner's commit log is cleaner when one "fuzz scaffolding" commit + one "bench scaffolding" commit each have one lib.rs hunk. The alternative sprays `#[doc(hidden)] pub` across 5 files which is harder to audit.

**Under the accepted recommendation: Plan 05-02 MUST run AFTER Plan 05-01 merges (serial). This matches CONTEXT.md D-32.**

## Project Constraints (from CLAUDE.md)

- **Rust edition 2024, MSRV 1.95** — criterion 0.8 MSRV 1.86 ≤ 1.95 ✅; libfuzzer-sys 0.4 has no declared MSRV but builds on any recent nightly ✅.
- **`base60-core` zero-dep runtime invariant** — Phase 5 adds criterion to `[dev-dependencies]` ONLY (D-17). `[dependencies]` stays empty. ✅ (Phase 2 D-02 precedent.)
- **Workspace lints** (`clippy::pedantic + nursery + cargo` + `-D warnings`) apply to new bench code + `__fuzz` module + visibility-bumped `parse_run` / `RUN_LEN`. Fuzz crate escapes via its own `[workspace]` — no lint propagation there. Benches pay the full lint bar.
- **Workspace rust lints** (`missing_debug_implementations`, `unreachable_pub`, `unsafe_op_in_unsafe_fn`, `rust_2018_idioms`, `unused_lifetimes`, `unused_qualifications`) apply to bench files. Criterion's own types already derive `Debug`; bench-local helpers must too.
- **`#![forbid(unsafe_op_in_unsafe_fn)]`** on `main.rs` and `lib.rs` — Phase 5 adds no new `unsafe` anywhere.
- **No `unwrap()` / `expect()` outside `#[cfg(test)]`** — bench files are NOT test files but they DO compile under `cargo test --all-targets`. `.expect()` in bench `LazyLock` initialisers is allowed by convention (the failure is structurally impossible; the string is the documentation). If clippy fires, use `unwrap_or_else(|_| unreachable!())` — Claude's Discretion.
- **No `todo!` / `unimplemented!` / `unreachable!`** in shipped code. Bench code is NOT shipped to end users but IS compiled by CI; apply the same discipline. `unreachable!` in a `LazyLock` dump-gen failure path is acceptable per stdlib idiom.
- **Saturating / checked arithmetic** — bench inputs use `wrapping_*` which is the explicit-overflow variant; the intent is "generate deterministic pseudo-random bytes," not "arithmetic on user data." Flag this in a bench-body comment so reviewers don't mistake it for a missing check.
- **`# Errors` / `# Panics` rustdoc** — `pub(crate) fn parse_run` already has `# Errors`. Adding `# Panics: Does not panic.` is Claude's Discretion; include it if `missing_panics_doc` clippy-nursery lint fires. `pub(crate) const RUN_LEN` has a one-line doc comment; no `# Errors`/`# Panics` needed on a const.
- **`cargo fmt --all --check` passes on every commit** — rustfmt applies to new bench files + fuzz target files. `fuzz/` has its own workspace, so `cargo fmt --all` at repo root does NOT walk it; planner runs `cargo fmt --manifest-path fuzz/Cargo.toml --all --check` separately OR matches conventions manually. **Claude's Discretion.**
- **Commit granularity — 2 plans, 2 commits minimum** (D-31). Each commit passes the full D-24/D-33 gate. No "WIP" state.
- **Conventional commit prefixes** — `test(cli): …` for Plan 05-01 ([TEST-02]), `test(core,cli): …` for Plan 05-02 ([PERF-06]). See D-31 exact wording.
- **`gh` for GitHub interactions** — not relevant this phase (no PR creation in Phase 5 scope; CI changes are Phase 7).
- **User's private global instruction: "be extremely concise; sacrifice grammar for concision"** applies to commit messages. CONTEXT D-31 already gives literal short-form messages — planner uses those verbatim.

## Sources

### Primary (HIGH confidence)

- `/home/chris/Projects/utils/test-60/.planning/phases/05-fuzz-criterion-harnesses/05-CONTEXT.md` — canonical decision log D-01..D-34 + canonical_refs + deferred ideas
- `/home/chris/Projects/utils/test-60/.planning/phases/05-fuzz-criterion-harnesses/05-DISCUSSION-LOG.md` — decision rationale (if present)
- `/home/chris/Projects/utils/test-60/.planning/PROJECT.md` — Key Decisions rows 7, 8, 9 (locked fuzz/bench posture)
- `/home/chris/Projects/utils/test-60/.planning/REQUIREMENTS.md` — TEST-02 (line 24), PERF-06 (line 36), traceability (lines 91, 100)
- `/home/chris/Projects/utils/test-60/.planning/ROADMAP.md` — Phase 5 Goal + SC1..SC5 (lines 80-91)
- `/home/chris/Projects/utils/test-60/.planning/STATE.md` — Open question on seed corpus (line 85) → locked to empty (D-09)
- `/home/chris/Projects/utils/test-60/.planning/research/STACK.md` — §"Fuzzing" + §"Benchmarking" + version verification
- `/home/chris/Projects/utils/test-60/.planning/research/ARCHITECTURE.md` — §"Integration Boundaries: Fuzz Crate ↔ Target Crates" (manifest template) + §"Bench Crate Layout"
- `/home/chris/Projects/utils/test-60/.planning/research/PITFALLS.md` — Pitfall 3 (fuzz false positives), Pitfall 4 (memmem), Pitfall 9 (criterion noise), Pitfall 11 (cargo-fuzz platform)
- `/home/chris/Projects/utils/test-60/.planning/phases/01-refactor-foundations/01-CONTEXT.md` — `chunk::CHUNK` + `chunk::be_u64` precedent
- `/home/chris/Projects/utils/test-60/.planning/phases/03-roundtrip-matrix-fixture-integration/03-CONTEXT.md` — thin `[lib]` precedent (D-06), minimal pub surface (D-07, D-09, D-10)
- `/home/chris/Projects/utils/test-60/.planning/phases/04-tighten-parse-run-close-coverage-gaps/04-CONTEXT.md` — `parse_run(run: &[u8; RUN_LEN], line_no) → io::Result<u64>` signature (D-09)
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/lib.rs` — live public surface: `pub use cli::{LensMode, Format}`, `#[doc(hidden)] pub use cli::TimeScale`, `#[doc(hidden)] pub mod __test_hooks`, `pub fn run()`
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/decode.rs` — `fn parse_run` at line 423, `const RUN_LEN` at line 50, `PAIR = 2`, `CHUNK_BYTES = 8`
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/search.rs` — `pub(crate) struct Pattern`, `impl FromStr for Pattern`, `pub(crate) fn find_all`
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/chunk.rs` — `pub(crate) const CHUNK: usize = 8`, `pub(crate) const fn be_u64([u8; CHUNK]) → u64`, `pub(crate) fn pad_chunk`
- `/home/chris/Projects/utils/test-60/crates/base60-core/src/lens.rs` — `pub trait Lens`, `TimeLens`, `AngleLens`, `TabletLens`, `CuneiformLens`, `TimeScale`
- `/home/chris/Projects/utils/test-60/crates/base60-core/Cargo.toml` — empty `[dependencies]`, minimal `[dev-dependencies]` (serial_test)
- `/home/chris/Projects/utils/test-60/crates/base60-cli/Cargo.toml` — `[lib] name = "base60"`, existing dev-deps (assert_cmd, base60-core, predicates, serial_test, tempfile)
- `/home/chris/Projects/utils/test-60/Cargo.toml` — workspace members, resolver = "3", MSRV 1.95, edition 2024
- `/home/chris/Projects/utils/test-60/.github/workflows/ci.yml` — 3×3 OS × rustc matrix; `cargo test --all-targets --locked` picks up new benches automatically
- `cargo search libfuzzer-sys --limit 1` — `libfuzzer-sys = "0.4.12"` on 2026-04-24
- `cargo search criterion --limit 1` — `criterion = "0.8.2"` on 2026-04-24
- `cargo search cargo-fuzz --limit 1` — `cargo-fuzz = "0.13.1"` on 2026-04-24
- Context7 `/bheisler/criterion.rs` — `criterion_group!` macro forms, `noise_threshold` / `sample_size` configuration, `harness = false` declaration
- Context7 `/rust-fuzz/cargo-fuzz` — install, invocation, basic scaffolding commands
- `github.com/rust-fuzz/cargo-fuzz/src/templates.rs` (WebFetch) — EXACT content of generated `Cargo.toml` template, `.gitignore` template, fuzz_target_1.rs template, `--fuzzing-workspace=true` nested-workspace stanza
- `github.com/rust-fuzz/libfuzzer/Cargo.toml` (WebFetch) — `[features]` default = `["link_libfuzzer"]`, `arbitrary-derive` opt-in

### Secondary (MEDIUM confidence)

- rust-fuzz book `https://rust-fuzz.github.io/book/cargo-fuzz/tutorial.html` — WebFetch confirmed the `fuzz_target!` + `std::str::from_utf8` pattern matches our D-13 shape
- Bench architecture precedent — none in this repo; first Phase 5 shipment

### Tertiary (LOW confidence)

- None. All version claims verified via `cargo search`; all code locations verified by reading files directly; all templates verified via cargo-fuzz source.

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — three cargo search results on the research date; Context7 + cargo-fuzz `templates.rs` for manifest shapes
- Architecture patterns: HIGH — `__fuzz` shape matches CONTEXT D-05 verbatim; `__bench` shape extrapolated from the same pattern (Assumptions A1 flagged)
- Pitfalls: HIGH — all four relevant pitfalls (3, 4, 9, 11) already documented in `.planning/research/PITFALLS.md` and flagged in CONTEXT canonical_refs
- Code examples: HIGH — every snippet copies CONTEXT decisions literally; bench bodies compile against the verified current-source signatures
- Validation architecture: HIGH — manual/automated split explicitly acknowledged; no false-CI-coverage claims
- Security: MEDIUM — low attack surface (no user data, no network); ASVS mapping is deliberately coarse because the phase doesn't introduce any new authentication/crypto/session concerns
- Plan split: MEDIUM-HIGH — lib.rs overlap documented with two resolution options; the accepted resolution keeps serial ordering (which CONTEXT D-32 already requires)

**Research date:** 2026-04-24
**Valid until:** 2026-05-24 (30 days — stable Rust toolchain; re-verify crate versions if planning slips past this date)
