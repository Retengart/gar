---
phase: 05-fuzz-criterion-harnesses
plan: 01
subsystem: testing
tags: [rust, cli, fuzz, cargo-fuzz, libfuzzer, test-02, scaffolding, pub-hatch]

# Dependency graph
requires:
  - phase: 04-tighten-parse-run-close-coverage-gaps
    provides: "parse_run(&[u8; RUN_LEN], line_no) -> io::Result<u64> signature (D-09); error-message contract (D-10/D-11)"
  - phase: 03-roundtrip-matrix-fixture-integration
    provides: "[lib] name = base60 + pub fn run() thin surface; __test_hooks pattern precedent for __fuzz"
provides:
  - "fuzz/ workspace-excluded crate with parse_run + pattern_from_str targets"
  - "#[cfg(fuzzing)] pub mod __fuzz re-export hatch in base60-cli/src/lib.rs"
  - "pub-widened decode::parse_run, decode::RUN_LEN, search::Pattern, search::ParseError under #[allow(unreachable_pub)] (non-fuzz API still pristine)"
  - "unexpected_cfgs check-cfg = [cfg(fuzzing)] workspace lint so clippy -D warnings accepts the gate"
  - "fuzz/README.md documenting Ubuntu+nightly-only constraint and reproducer commands"
affects: [05-02-criterion-benches, 07-ci-hardening, any-future-fuzz-target]

# Tech tracking
tech-stack:
  added:
    - "libfuzzer-sys 0.4 (dev-only, workspace-excluded fuzz crate)"
    - "cargo-fuzz 0.13.1 developer tooling (not a dep; installed once)"
  patterns:
    - "__fuzz re-export shim mirrors existing __test_hooks — #[doc(hidden)] pub mod with #[cfg(fuzzing)] gate"
    - "Item visibility: pub + #[allow(unreachable_pub)] on source item when it is only reachable externally through a #[cfg(fuzzing)]-gated re-export"
    - "Fuzz targets: let _ = ... + length/UTF-8 guard + D-14 banner (Err is happy path — Pitfall 3)"

key-files:
  created:
    - "fuzz/Cargo.toml"
    - "fuzz/fuzz_targets/parse_run.rs"
    - "fuzz/fuzz_targets/pattern_from_str.rs"
    - "fuzz/.gitignore"
    - "fuzz/README.md"
    - "fuzz/Cargo.lock"
  modified:
    - "Cargo.toml (root — exclude = [fuzz] + unexpected_cfgs check-cfg)"
    - "crates/base60-cli/src/lib.rs (__fuzz module)"
    - "crates/base60-cli/src/decode.rs (pub + #[allow(unreachable_pub)] on parse_run + RUN_LEN)"
    - "crates/base60-cli/src/search.rs (same widening on Pattern + ParseError)"

key-decisions:
  - "Fuzz items widened to pub + #[allow(unreachable_pub)] rather than pub(crate): required because pub use inside pub mod __fuzz needs pub source items (Rust visibility rules)"
  - "mod decode and mod search stay private at crate root, so pub-widened items are only externally reachable through the #[cfg(fuzzing)]-gated __fuzz module — non-fuzz public API remains pristine (TEST-02 SC5 verified against cargo doc)"
  - "unexpected_cfgs check-cfg added at workspace level so one declaration covers all crates — cleaner than per-crate [lints.rust] overrides"
  - "fuzz/Cargo.lock committed (fuzz crate is binary-producing; deterministic build)"

patterns-established:
  - "Pattern-P5-1: Conditional-visibility hatch for fuzz — pub item + #[allow(unreachable_pub)] + private containing module + #[cfg(fuzzing)] re-export. Future fuzz targets re-use the shape inside __fuzz."
  - "Pattern-P5-2: Fuzz target skeleton — D-14 banner → #![no_main] → use libfuzzer_sys::fuzz_target → input guard → let _ = ... . No unwrap, no catch_unwind (-Cpanic=abort)."

requirements-completed: [TEST-02]

# Metrics
duration: ~20min
completed: 2026-04-24
---

# Phase 5 Plan 01: Fuzz Crate Scaffolding Summary

**cargo-fuzz workspace-excluded crate with two libFuzzer targets (parse_run, pattern_from_str) wired through a #[cfg(fuzzing)] pub mod __fuzz re-export hatch; 30 s smokes exit 0 with no crash artefacts.**

## Performance

- **Duration:** ~20 min (including nightly/cargo-fuzz install + two 30 s smokes)
- **Started:** 2026-04-24T18:00Z
- **Completed:** 2026-04-24T18:12Z
- **Tasks:** 12 / 12
- **Files modified:** 10 (6 new under fuzz/, 4 edits in main workspace)

## Accomplishments

- `fuzz/` workspace-excluded crate scaffolded via `cargo fuzz init --fuzzing-workspace=true`, hand-edited to the D-03/D-04 shape (nested `[workspace]`, `libfuzzer-sys = "0.4" default-features=false features=["link_libfuzzer"]`, both `base60-core` and `base60` as path deps, edition 2024, no `rust-version`).
- Two fuzz targets (`parse_run`, `pattern_from_str`) with `let _ = ...` pattern + length/UTF-8 guards + D-14 banner — Pitfall 3 mitigated. No `unwrap()`, no `catch_unwind`.
- `#[doc(hidden)] #[cfg(fuzzing)] pub mod __fuzz` re-export shim in `base60-cli/src/lib.rs` — non-fuzz builds do not see the module; `cargo doc --workspace --no-deps --locked` output contains only the Phase 3 public surface (`base60::{Format, LensMode, run, cli}`).
- Phase 3 D-24 gate green: `cargo fmt --all --check` + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo test --workspace --all-targets --locked` (232 tests passing) + `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`.
- `cd fuzz && cargo +nightly fuzz build` exits 0 (TEST-02 SC1); `cargo +nightly fuzz run parse_run -- -max_total_time=30` reaches ~347 k exec/s, `pattern_from_str` ~90 k exec/s, both exit 0 with empty `fuzz/artifacts/*/` (TEST-02 SC2).

## Task Commits

Single atomic commit per CONTEXT D-31:

1. **Tasks 01–12 (Plan 05-01 atomic)** — `db93817` (test) — `test(cli): fuzz crate scaffolding with parse_run + pattern_from_str targets [TEST-02]`

Worktree-level `--no-verify` used per parallel-executor convention (orchestrator validates hooks once after merge).

## Files Created/Modified

- `fuzz/Cargo.toml` — nested-workspace fuzz manifest (D-01/D-03/D-04).
- `fuzz/fuzz_targets/parse_run.rs` — length-gated libFuzzer target (D-12, D-14 banner).
- `fuzz/fuzz_targets/pattern_from_str.rs` — UTF-8-guarded libFuzzer target (D-13, D-14 banner).
- `fuzz/.gitignore` — auto-generated (`target corpus artifacts coverage`; D-10 OK).
- `fuzz/README.md` — Ubuntu+nightly constraint, reproducer commands, Pitfall 11 citation.
- `fuzz/Cargo.lock` — deterministic build; committed (fuzz crate produces binaries).
- `Cargo.toml` (root) — `exclude = ["fuzz"]` (D-02) + `unexpected_cfgs check-cfg = ["cfg(fuzzing)"]` (deviation — see below).
- `crates/base60-cli/src/lib.rs` — `#[doc(hidden)] #[cfg(fuzzing)] pub mod __fuzz` re-exporting `{RUN_LEN, parse_run, Pattern}` (D-05).
- `crates/base60-cli/src/decode.rs` — `parse_run` and `RUN_LEN` widened from `fn`/`const` to `pub` + `#[allow(unreachable_pub)]` (D-06, widened further than plan called for — see below).
- `crates/base60-cli/src/search.rs` — `Pattern` and `ParseError` same widening (deviation from D-07 — see below).

## Decisions Made

- **Widen to `pub` + `#[allow(unreachable_pub)]` instead of `pub(crate)`:** CONTEXT D-06/D-07 specified `pub(crate)` visibility, but Rust's re-export rules forbid `pub use pub(crate)::Item` inside a `pub mod`. Two clean solutions exist: (a) duplicate declarations under `#[cfg(fuzzing)]` / `#[cfg(not(fuzzing))]`, (b) make items `pub` with `unreachable_pub` allowed on the item and rely on the enclosing `mod decode`/`mod search` being private at crate root. Chose (b): single declaration, explicit rationale comment on each item, non-fuzz public API still pristine (verified via `cargo doc` output — no new public items).
- **`unexpected_cfgs check-cfg` at workspace level (root `Cargo.toml`):** Clippy's `unexpected_cfgs` lint (enabled by `-D warnings`) rejects `#[cfg(fuzzing)]` unless `fuzzing` is declared via `check-cfg`. Declared once under `[workspace.lints.rust]` so all crates inherit. This is the idiomatic remediation per the clippy help message.
- **Commit `fuzz/Cargo.lock`:** Fuzz crate produces libFuzzer binaries; committing the lockfile gives deterministic builds for Phase 7 CI-02 weekly runs. Nested `[workspace]` means this lockfile is independent from the main workspace's.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 – Blocking] `Pattern` and `ParseError` visibility widened in `search.rs`**
- **Found during:** Task 05-01-11 (fuzz build).
- **Issue:** Plan D-07 said `Pattern` was already `pub(crate)` so "no visibility change needed". But `pub use crate::search::Pattern` inside `pub mod __fuzz` requires `Pattern` to be `pub`. Error: `crate-private type \`search::Pattern\` in public interface`. Also propagates to `ParseError` because `impl FromStr for Pattern { type Err = ParseError; }` requires associated types on a `pub` trait impl for a `pub` type to be `pub`.
- **Fix:** Widened both `Pattern` and `ParseError` to `pub` with `#[allow(unreachable_pub)]` and a doc rationale tying back to Phase 5 TEST-02 SC5. `mod search` stays private at crate root, so no public API leak (verified by `cargo doc` output).
- **Files modified:** `crates/base60-cli/src/search.rs`.
- **Verification:** `cargo clippy --workspace --all-targets --locked -- -D warnings` passes; `cargo doc` shows no new public items under `base60::`; fuzz build succeeds.
- **Committed in:** `db93817` (atomic plan commit).

**2. [Rule 3 – Blocking] `parse_run` and `RUN_LEN` visibility widened to `pub` (beyond plan's `pub(crate)`)**
- **Found during:** Task 05-01-11 (fuzz build) — same root cause as deviation 1.
- **Issue:** D-06 specified `pub(crate)`, but `pub use crate::decode::{parse_run, RUN_LEN}` inside `pub mod __fuzz` requires `pub` sources.
- **Fix:** `pub` + `#[allow(unreachable_pub)]` on both items with doc rationale.
- **Files modified:** `crates/base60-cli/src/decode.rs`.
- **Verification:** Same as deviation 1; also confirmed existing `# Errors` rustdoc block still satisfies `RUSTDOCFLAGS="-D warnings"`; `clippy::missing_panics_doc` did NOT fire (no `unwrap`/`expect`/`panic!` in body).
- **Committed in:** `db93817`.

**3. [Rule 3 – Blocking] `unexpected_cfgs check-cfg` added to workspace lints**
- **Found during:** Task 05-01-10 (Phase 3 D-24 gate, clippy step).
- **Issue:** `#[cfg(fuzzing)]` on the `__fuzz` module triggered `unexpected_cfgs` under `-D warnings`: `unexpected 'cfg' condition name: 'fuzzing'`. The plan did not anticipate this.
- **Fix:** Added `unexpected_cfgs = { level = "warn", check-cfg = ["cfg(fuzzing)"] }` to `[workspace.lints.rust]` in root `Cargo.toml`, with a rationale comment linking to the `__fuzz` gate and Phase 5 TEST-02.
- **Files modified:** `Cargo.toml` (workspace root).
- **Verification:** Clippy passes; the `#[cfg(fuzzing)]` gate compiles cleanly both under cargo-fuzz (cfg set) and the main workspace (cfg unset).
- **Committed in:** `db93817`.

---

**Total deviations:** 3 auto-fixed (all Rule 3 – blocking issues uncovered by the fuzz build step).
**Impact on plan:** All three deviations are single-file, single-line-scale fixes that preserve the plan's intent (fuzz targets compile, non-fuzz API stays pristine, clippy -D warnings stays green). No scope creep; `search.rs` was already in the plan's source-files list indirectly via D-07 (even though D-07 predicted no change). The visibility widening is strictly more conservative than the alternative (duplicate `#[cfg]`-gated declarations), and is still invisible to non-fuzz consumers (private enclosing module blocks external reach).

## Issues Encountered

- Initial `cargo fuzz init` auto-detected `base60-core` as the target package and generated a single-dep manifest pointing at `..` — overwritten in Task 05-01-03 to the two-path-dep shape. Expected per RESEARCH.md §"cargo fuzz init — exact generated layout".
- The commit includes `fuzz/Cargo.lock` (~1935 lines) — this is intentional (binary crate deterministic builds) but inflates the diff size noticeably.

## User Setup Required

None for the main workspace. For developers who want to run fuzz targets locally:

- `rustup toolchain install nightly`
- `cargo install cargo-fuzz`

Both are documented in `fuzz/README.md`. Phase 7 CI-02 will install them automatically in the weekly scheduled job.

## Known Stubs

None — this is a scaffolding-only plan with no runtime behaviour change. The empty `fuzz/corpus/` and `fuzz/artifacts/` directories are intentional (CONTEXT D-09: empty seed corpus on commit) and gitignored.

## Next Phase Readiness

- Plan 05-02 (Criterion bench scaffolding) is strictly independent — touches `crates/*/benches/*` + `crates/*/Cargo.toml [dev-dependencies]`, no overlap with Plan 05-01's file set.
- The `unexpected_cfgs check-cfg` workspace lint precedent established here also benefits Plan 05-02 if any bench uses `#[cfg(...)]` gates.
- Phase 7 CI-02 (weekly fuzz job) can now consume this scaffolding directly: `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=240` on `ubuntu-latest`.
- Phase 6 (PERF work) has the `__fuzz` hatch pattern ready to reuse if a new fuzz target is needed for `render_to<W>` or other internal surface.

---
*Phase: 05-fuzz-criterion-harnesses*
*Completed: 2026-04-24*

## Self-Check: PASSED

- fuzz/Cargo.toml exists
- fuzz/fuzz_targets/parse_run.rs exists
- fuzz/fuzz_targets/pattern_from_str.rs exists
- fuzz/.gitignore exists
- fuzz/README.md exists
- Commit db93817 exists in git log
- `pub fn parse_run` present in decode.rs
- `pub const RUN_LEN` present in decode.rs
- `#[cfg(fuzzing)]` present in lib.rs
- `exclude = ["fuzz"]` present in root Cargo.toml
