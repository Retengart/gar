---
phase: 05-fuzz-criterion-harnesses
plan: 02
subsystem: benchmarking
tags: [rust, core, cli, bench, perf-06, criterion, scaffolding]

# Dependency graph
requires:
  - phase: 05-fuzz-criterion-harnesses
    plan: 01
    provides: "__fuzz shim precedent (pub + #[allow(unreachable_pub)] widening pattern); unexpected_cfgs workspace lint"
  - phase: 03-roundtrip-matrix-fixture-integration
    provides: "[lib] name = base60 + pub fn run() thin surface; __test_hooks #[doc(hidden)] pattern precedent"
  - phase: 02-env-test-serialisation
    provides: "[dev-dependencies] precedent (serial_test) — dev-deps don't violate CI-03 zero-dep invariant"
provides:
  - "criterion 0.8 dev-dep in both base60-core and base60-cli"
  - "5 [[bench]] harness=false entries (convert, lens in core; dump, decode, search in cli)"
  - "#[doc(hidden)] pub mod __bench re-export shim in base60-cli/src/lib.rs"
  - "pub-widened dump::dump_all, decode::decode_stream, search::find_all, color::{Palette, PALETTE_NONE}, cli::InputFormat under #[allow(unreachable_pub)]"
  - "4 PITFALLS-Pitfall-4-mandatory search cells gating Phase 6 PERF-03 (memchr::memmem swap)"
  - "Advisory-only README at crates/base60-cli/benches/README.md with reproducer commands"
  - "One-liner pointer README at crates/base60-core/benches/README.md"
affects: [06-perf-passes, 07-ci-hardening]

# Tech tracking
tech-stack:
  added:
    - "criterion 0.8 (dev-dep in both crates; default-features=false, features=[cargo_bench_support, html_reports])"
    - "criterion transitive: anes, ciborium, ciborium-io, ciborium-ll, criterion-plot, half, is-terminal, itertools, oorandom, plotters, plotters-backend, plotters-svg, tinytemplate (via Cargo.lock)"
  patterns:
    - "__bench re-export shim mirrors __fuzz/__test_hooks — #[doc(hidden)] pub mod, always compiled (no cfg gate)"
    - "Item visibility: pub + #[allow(unreachable_pub)] on source item when it is only reachable externally through a #[doc(hidden)] pub mod __bench re-export"
    - "Large-input bench pattern: static LazyLock<Vec<u8>> instead of const [u8; 1<<20] — avoids long_running_const_eval (rust) and large_stack_frames/large_const_arrays (clippy) on 1 MiB inputs"
    - "Bench file-level allow: #![allow(clippy::missing_panics_doc, clippy::missing_errors_doc, clippy::cast_possible_truncation, clippy::large_stack_arrays)] — benches are not public API"

key-files:
  created:
    - "crates/base60-core/benches/convert.rs"
    - "crates/base60-core/benches/lens.rs"
    - "crates/base60-core/benches/README.md"
    - "crates/base60-cli/benches/dump.rs"
    - "crates/base60-cli/benches/decode.rs"
    - "crates/base60-cli/benches/search.rs"
    - "crates/base60-cli/benches/README.md"
  modified:
    - "Cargo.lock (criterion + transitive deps resolved)"
    - "crates/base60-core/Cargo.toml (criterion dev-dep + 2 [[bench]] blocks)"
    - "crates/base60-cli/Cargo.toml (criterion dev-dep + 3 [[bench]] blocks)"
    - "crates/base60-cli/src/lib.rs (__bench module added after __fuzz)"
    - "crates/base60-cli/src/cli.rs (InputFormat pub + #[allow(unreachable_pub)])"
    - "crates/base60-cli/src/color.rs (Palette struct pub + #[derive(Debug)]; PALETTE_NONE pub)"
    - "crates/base60-cli/src/decode.rs (decode_stream pub + #[allow(unreachable_pub)] + # Errors rustdoc)"
    - "crates/base60-cli/src/dump.rs (dump_all pub + #[allow(unreachable_pub)] + # Errors rustdoc)"
    - "crates/base60-cli/src/search.rs (find_all pub + #[allow(unreachable_pub)])"

key-decisions:
  - "Bench inputs switched from const to static LazyLock for 1 MiB arrays — rust's long_running_const_eval and clippy's large_stack_frames/large_const_arrays forbid 1 MiB const-initialized arrays under -D warnings; LazyLock preserves determinism (same wrapping_mul/wrapping_add pattern) without hitting either lint"
  - "Palette struct widened to pub (not just PALETTE_NONE) — private_interfaces warning fires because dump_all takes &Palette in its public signature; needed #[derive(Debug)] to satisfy missing_debug_implementations"
  - "All 5 bench-reachable items widened to pub + #[allow(unreachable_pub)] via same 05-01 pattern — mod cli/color/decode/dump/search all stay private at crate root, so non-bench public API is pristine (verified via cargo doc output — only Format/LensMode/run visible under base60::)"

patterns-established:
  - "Pattern-P5-3: 1 MiB bench input via static LazyLock<Vec<u8>> with (0..SIZE).map(|i| (i.wrapping_mul(13).wrapping_add(7)) as u8).collect() — deterministic, bit-identical across machines, compiles cleanly under -D warnings at any input size"
  - "Pattern-P5-4: __bench shim parallel to __fuzz — always-on (no #[cfg] gate) because cargo bench --no-run is part of any --all-targets compile"

requirements-completed: [PERF-06]

# Metrics
duration: ~35min
completed: 2026-04-24
---

# Phase 5 Plan 02: Criterion Bench Scaffolding Summary

**5 criterion benches (convert/lens in core; dump/decode/search in cli) wired through a `#[doc(hidden)] pub mod __bench` re-export shim; every bench uses `noise_threshold(0.05)` + `sample_size(50)`; the 4 PITFALLS-Pitfall-4-mandatory search cells gate Phase 6 PERF-03.**

## Performance

- **Duration:** ~35 min (including criterion first-resolve which pulled ~14 transitive deps)
- **Started:** 2026-04-24 (executor session)
- **Completed:** 2026-04-24
- **Tasks:** 13 / 13
- **Files modified:** 16 (7 new bench artifacts + 9 source/manifest edits)

## Accomplishments

- `#[doc(hidden)] pub mod __bench` added to `crates/base60-cli/src/lib.rs` immediately after `__fuzz` — re-exports `InputFormat`, `PALETTE_NONE`, `decode_stream`, `dump_all`, `find_all`. No `#[cfg]` gate (benches compile on every `--all-targets` pass).
- Both `crates/base60-core/Cargo.toml` and `crates/base60-cli/Cargo.toml` gained `criterion = { version = "0.8", default-features = false, features = ["cargo_bench_support", "html_reports"] }` in `[dev-dependencies]`. `[dependencies]` section stays untouched on both — CI-03 zero-dep invariant preserved.
- 5 `[[bench]] harness = false` entries: `convert` + `lens` in core; `dump` + `decode` + `search` in cli.
- 5 bench files with deterministic `wrapping_mul`/`wrapping_add` inputs, `noise_threshold(0.05)`, `sample_size(50)`, file-level clippy allow block for the benches-aren't-public-API lints.
- `crates/base60-cli/benches/search.rs` contains ALL 4 mandatory cells per D-27/Pitfall 4: `zero_fill/1byte_null`, `zero_fill/2byte_ffff`, `random/3byte_elf`, `random/8byte_cafebabe`.
- `crates/base60-cli/benches/README.md` — canonical advisory-only posture with `--save-baseline`/`--baseline` reproducer workflow, 5-row per-bench scope table, citations to PROJECT.md row 8 and PITFALLS.md Pitfall 9.
- `crates/base60-core/benches/README.md` — one-liner pointer.
- Phase 3 D-24 gate green: `cargo fmt --all --check` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo test --workspace --all-targets --locked` + `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`.
- `cargo bench --workspace --no-run --locked` produces 5 bench binaries (`convert-*`, `lens-*`, `dump-*`, `decode-*`, `search-*`) — PERF-06 SC3 satisfied.
- `cargo doc` output confirms no new public items under `base60::` or `base60_core::` beyond the Phase 3 surface (`Format`, `LensMode`, `run`, `cli` module).

## Task Commits

Single atomic commit per CONTEXT D-31:

1. **Tasks 01–13 (Plan 05-02 atomic)** — `f603f63` (test) — `test(core,cli): criterion bench scaffolding [PERF-06]`

Worktree-level `--no-verify` used per parallel-executor convention (orchestrator validates hooks once after merge).

## Files Created/Modified

- `crates/base60-core/benches/convert.rs` — `u64_to_base60` hot loop over 1024 FIB_MUL-seeded inputs (D-23).
- `crates/base60-core/benches/lens.rs` — `Lens::render` across 4 impls inside one `benchmark_group("lens/render")` — `time`/`angle`/`tablet`/`cuneiform_ascii`. `CuneiformLens { fallback: true }` literal avoids env-reading `auto()` (Assumption A6).
- `crates/base60-core/benches/README.md` — one-liner pointer per D-30.
- `crates/base60-cli/benches/dump.rs` — `dump_all` over 1 MiB `static LazyLock<Vec<u8>>` deterministic input, mono palette, no lens.
- `crates/base60-cli/benches/decode.rs` — pre-computed 1 MiB dump via `LazyLock<Vec<u8>>`; only `decode_stream` is inside `b.iter`. `InputFormat::Plain` forces the text decoder path.
- `crates/base60-cli/benches/search.rs` — 4 mandatory cells (Pitfall 4). `ZERO_FILL` and `RANDOM_FILL` both `static LazyLock<Vec<u8>>`.
- `crates/base60-cli/benches/README.md` — canonical advisory-only doc.
- `Cargo.lock` — criterion + 13 transitive deps resolved (anes, ciborium + ciborium-io + ciborium-ll, criterion-plot, half, is-terminal, itertools, oorandom, plotters + plotters-backend + plotters-svg, tinytemplate). Verified against STACK.md expected set; no surprises.
- `crates/base60-core/Cargo.toml` — `[dev-dependencies]` gains `criterion` (between existing `serial_test`, alphabetical); `[[bench]]` blocks for `convert` and `lens`.
- `crates/base60-cli/Cargo.toml` — `[dev-dependencies]` gains `criterion` (slotted between `base60-core` and `predicates`, alphabetical); `[[bench]]` blocks for `dump`, `decode`, `search`.
- `crates/base60-cli/src/lib.rs` — `#[doc(hidden)] pub mod __bench` after `__fuzz` with 5 re-exports.
- `crates/base60-cli/src/cli.rs` — `InputFormat` widened to `pub` + `#[allow(unreachable_pub)]` with rationale doc.
- `crates/base60-cli/src/color.rs` — `Palette` struct widened to `pub` + `#[derive(Debug)]`; `PALETTE_NONE` widened to `pub`; both with `#[allow(unreachable_pub)]` + rationale doc.
- `crates/base60-cli/src/decode.rs` — `decode_stream` widened to `pub` + `#[allow(unreachable_pub)]` + `# Errors` rustdoc block.
- `crates/base60-cli/src/dump.rs` — `dump_all` widened to `pub` + `#[allow(unreachable_pub)]` + `# Errors` rustdoc block.
- `crates/base60-cli/src/search.rs` — `find_all` widened to `pub` + `#[allow(unreachable_pub)]`.

## Decisions Made

- **1 MiB input via `static LazyLock<Vec<u8>>` instead of `const [u8; 1<<20]`:** The original RESEARCH Example 6/7/8 shapes specified `const` arrays initialized with `while` loops. Under `-D warnings` on Rust 1.95+, this triggers three distinct failures: (a) `long_running_const_eval` (rustc) on the 1 MiB `while` loop, (b) `clippy::large_const_arrays` on the resulting const, (c) `clippy::large_stack_frames` on any closure that captures an `&[u8; 1<<20]` reference. `static LazyLock<Vec<u8>>` produces bit-identical bytes (same `i.wrapping_mul(13).wrapping_add(7) as u8` mixer), is heap-allocated (no stack-frame warning), and first-access lazy (no const-eval cost). Determinism per D-28 is preserved.
- **Widen Palette struct (not just PALETTE_NONE):** `dump_all` takes `palette: &Palette`. Making `PALETTE_NONE: Palette` public without widening `Palette` triggers `private_interfaces` warning — a public static with a private type. Widening the struct required `#[derive(Debug)]` to satisfy `missing_debug_implementations`. All struct fields stay `pub(crate)` — the opaque-struct invariant at crate boundaries is preserved.
- **All 5 bench-reachable items widened to `pub + #[allow(unreachable_pub)]`:** Same shape as Plan 05-01 deviation. `pub use crate::x::y` inside `pub mod __bench` requires `y` to be `pub` at definition site (Rust re-export E0364/E0365). The enclosing `mod cli/color/decode/dump/search` all stay private at crate root, so no public API leak — verified via `cargo doc` (only `Format`, `LensMode`, `run`, and `cli` module visible under `base60::`).
- **File-level clippy allow in every bench:** `#![allow(clippy::missing_panics_doc, clippy::missing_errors_doc, clippy::cast_possible_truncation, clippy::large_stack_arrays)]` — benches are not public API; `as u8` truncation in the pseudo-random mixer is intentional; `missing_panics_doc` on `bench_*` functions is noise.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Widened 5 bench-reachable items + Palette struct to `pub + #[allow(unreachable_pub)]`**
- **Found during:** Task 05-02-01 verification (`cargo check -p base60 --locked --all-targets`).
- **Issue:** The 5 items re-exported by `pub mod __bench` are `pub(crate)` at definition sites. Rust's re-export rules (E0364/E0365) require `pub` source items when re-exporting publicly through a `pub mod`. Additionally, making `PALETTE_NONE` public (it's typed `Palette`) triggered `private_interfaces` warning because `Palette` itself was `pub(crate)`; widening `Palette` to `pub` then triggered `missing_debug_implementations`.
- **Fix:** Widened `PALETTE_NONE`, `dump_all`, `decode_stream`, `find_all`, `InputFormat`, and the `Palette` struct to `pub` with `#[allow(unreachable_pub)]` + rationale doc blocks citing "enclosing `mod X` is private at crate root" (same shape as Plan 05-01 deviation 1-2). Added `#[derive(Debug)]` to `Palette` to satisfy `missing_debug_implementations`. Added `# Errors` rustdoc to `dump_all` and `decode_stream`.
- **Files modified:** `crates/base60-cli/src/{cli,color,decode,dump,search}.rs`.
- **Verification:** `cargo clippy --workspace --all-targets --locked -- -D warnings` passes; `cargo doc --workspace --no-deps --locked` shows no new public items under `base60::` (only `Format`, `LensMode`, `run`, and `cli` module — the Phase 3 surface).
- **Committed in:** `f603f63`.

**2. [Rule 1 - Bug] Switched 1 MiB `const` arrays to `static LazyLock<Vec<u8>>` in dump/decode/search benches**
- **Found during:** Task 05-02-11 (`cargo bench --workspace --no-run`).
- **Issue:** RESEARCH Examples 6/7/8 specified `const INPUT: [u8; 1<<20] = { let mut arr = [0u8; SIZE]; let mut i = 0; while i < SIZE { arr[i] = (i.wrapping_mul(13).wrapping_add(7)) as u8; i += 1; } arr };`. On Rust 1.95+ this fires three different warnings under `-D warnings`: `#[deny(long_running_const_eval)]` (rustc, the `while` loop evaluates too slowly at compile time), `clippy::large_const_arrays` (>512KB const), and `clippy::large_stack_frames` (>512KB on-stack reference capture in bench closure).
- **Fix:** Replaced all four 1 MiB const arrays (`INPUT` in dump.rs, `RAW` in decode.rs, `ZERO_FILL` + `RANDOM_FILL` in search.rs) with `static LazyLock<Vec<u8>>`. Same deterministic `wrapping_mul(13).wrapping_add(7) as u8` mixer via `(0..SIZE).map(...).collect()` — byte-identical output. `ZERO_FILL` uses `vec![0u8; SIZE]`. Added `use std::sync::LazyLock;` imports to dump.rs and search.rs (decode.rs already had it for `DUMPED`).
- **Files modified:** `crates/base60-cli/benches/{dump,decode,search}.rs`.
- **Verification:** `cargo bench --workspace --no-run --locked` produces all 5 bench binaries; `cargo clippy --workspace --all-targets --locked -- -D warnings` passes; determinism preserved (same mixer, same seed pattern).
- **Committed in:** `f603f63`.
- **Impact on plan:** Small deviation from RESEARCH Examples 6-8 exact shape, but the `LazyLock<Vec<u8>>` pattern is idiomatic for benches with multi-MB inputs (D-26's `DUMPED` already uses it). Compile-time `const` determinism wasn't actually load-bearing — `LazyLock` is equally bit-identical across machines (same deterministic closure, first-access computation).

**3. [Rule 1 - Bug] Added `#[derive(Debug)]` to `Palette` struct**
- **Found during:** Clippy check after widening Palette to `pub`.
- **Issue:** Workspace lint `missing_debug_implementations = warn` fires on the newly-`pub` `Palette` struct (previously `pub(crate)`, lint didn't apply).
- **Fix:** Added `#[derive(Debug)]`. All fields are `&'static str`, so Debug is trivially derivable.
- **Files modified:** `crates/base60-cli/src/color.rs`.
- **Verification:** `cargo clippy --workspace --all-targets --locked -- -D warnings` passes.
- **Committed in:** `f603f63`.

---

**Total deviations:** 3 auto-fixed (1 Rule 3 blocking + 2 Rule 1 bugs uncovered by the bench compile step and `-D warnings`).
**Impact on plan:** All three deviations are mechanical fixes preserving plan intent (benches compile, non-bench public API pristine, clippy `-D warnings` green). `LazyLock` substitution in deviation 2 is strictly more robust than the `const [u8; 1<<20]` form specified in RESEARCH — it sidesteps three separate lints under `-D warnings` without touching determinism.

## Issues Encountered

- The `criterion` first-time resolve added 13 transitive deps to the workspace `Cargo.lock` (~183-line diff), all expected per RESEARCH §Security Domain Defensive notes: `anes`, `ciborium{,-io,-ll}`, `criterion-plot`, `half`, `is-terminal`, `itertools` (existing version bump), `oorandom`, `plotters{,-backend,-svg}`, `tinytemplate`. No surprises.
- RESEARCH Examples 6-8 specified `const [u8; 1<<20]` arrays with `while`-loop initializers. All three fired `long_running_const_eval` under Rust 1.95's default lint level — the examples may predate that lint tightening. Switched to `LazyLock<Vec<u8>>` as noted in deviation 2.
- No other issues. `cargo bench --workspace --no-run --locked` succeeds on the developer laptop; bench targets also compile as test variants under `cargo test --workspace --all-targets --locked` (criterion 0.8 handles `harness = false` test-mode via a smoke stub — "Testing <name>, Success" output).

## User Setup Required

None. Benches are developer-laptop-only — no CI changes in this phase. For developers who want to run benches locally:

- No extra toolchain needed (benches compile on every `cargo bench` or `cargo test --all-targets`).
- `cargo bench -p base60 --bench search -- --save-baseline pre` captures a baseline; `cargo bench -p base60 --bench search -- --baseline pre` compares.
- HTML reports land in `target/criterion/` (gitignored by the existing repo-root `.gitignore`).

## Known Stubs

None — this is a scaffolding-only plan with no runtime behaviour change. Every bench exercises a real code path; none contain TODOs, placeholders, or mocked-out values.

## Threat Flags

None. No new trust boundaries, auth paths, or schema changes introduced beyond the threat register already in the plan's `<threat_model>` — every threat (T-05-07 through T-05-12) is addressed by an acceptance-criteria-verified fix in this commit.

## Next Phase Readiness

- Phase 6 PERF-01..05 can now consume the bench baselines. Every perf PR paste before/after numbers from `cargo bench --bench <name> -- --save-baseline pre` / `-- --baseline pre` per CLI README workflow.
- Phase 6 PERF-03 (`memchr::memmem` swap) is specifically gated by the 4 mandatory search cells — any regression on `zero_fill/1byte_null`, `zero_fill/2byte_ffff`, `random/3byte_elf`, or `random/8byte_cafebabe` blocks the swap.
- Phase 6 PERF-04 (`render_to<W>` migration) has a baseline in `base60-core/benches/lens.rs` — Phase 6 extends this file with a sibling `lens/render_to` benchmark group.
- Phase 7 CI-02 (weekly fuzz) and CI-03 (zero-dep grep) both reuse Phase 5's scaffolding unchanged.
- Phase 7 SC4 (`cargo bench --workspace --no-run --locked` compile smoke) is trivially added — the invocation already works on the developer laptop and will work on the 3×3 CI matrix's existing `cargo test --all-targets` cells.

---
*Phase: 05-fuzz-criterion-harnesses*
*Completed: 2026-04-24*

## Self-Check: PASSED

- crates/base60-core/benches/convert.rs exists
- crates/base60-core/benches/lens.rs exists
- crates/base60-core/benches/README.md exists
- crates/base60-cli/benches/dump.rs exists
- crates/base60-cli/benches/decode.rs exists
- crates/base60-cli/benches/search.rs exists
- crates/base60-cli/benches/README.md exists
- Commit f603f63 present in git log
- `criterion = { version = "0.8"` present in both Cargo.toml files
- `harness = false` appears 2x in base60-core/Cargo.toml, 3x in base60-cli/Cargo.toml
- `pub mod __bench` present in crates/base60-cli/src/lib.rs
- 4 mandatory cell names present in crates/base60-cli/benches/search.rs (zero_fill/1byte_null, zero_fill/2byte_ffff, random/3byte_elf, random/8byte_cafebabe)
- `advisory` + `save-baseline` + PROJECT.md + PITFALLS + 5% all present in benches/README.md
- `base60-cli/benches/README.md` pointer present in crates/base60-core/benches/README.md
- [dependencies] section of base60-core/Cargo.toml does NOT contain criterion (zero-dep invariant)
