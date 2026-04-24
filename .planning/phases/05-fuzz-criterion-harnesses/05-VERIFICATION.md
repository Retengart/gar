---
phase: 05-fuzz-criterion-harnesses
verified: 2026-04-24T22:00:00Z
status: passed
score: 5/5 success criteria verified
overrides_applied: 0
---

# Phase 5: Fuzz + Criterion Harnesses — Verification Report

**Phase Goal:** Infrastructure only — workspace-excluded `fuzz/` crate with two targets + per-crate `benches/` with criterion scaffolding. Neither gates CI; both exist to make Phase 6 measurable and Phase 7 runnable.
**Verified:** 2026-04-24T22:00:00Z
**Status:** PASSED
**Re-verification:** No — initial verification.

## Goal Achievement

### Success-Criteria Verdicts

| # | SC | Verdict | Evidence |
|---|----|---------|----------|
| SC1 | fuzz/ at repo root, workspace-excluded, 2 targets with `let _` | PASS | `fuzz/Cargo.toml` exists with `edition="2024"`, nested `[workspace] resolver="3" members=["."]`, libfuzzer-sys 0.4 + both path deps; two `[[bin]]` entries `parse_run` + `pattern_from_str`. Root `Cargo.toml:4 exclude = ["fuzz"]`. Both targets use `let _ =` (no `unwrap()`, no `catch_unwind`, no `.expect(`). |
| SC2 | 30 s parse_run smoke on Linux, .gitignore excludes corpus/artifacts | PASS | `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` completed: `Done 10818084 runs in 31 second(s)`, exec/s ~349 k, no crash. `pattern_from_str` 30 s smoke also DONE with no crash (3.4M runs). `fuzz/artifacts/{parse_run,pattern_from_str}/` empty. `fuzz/.gitignore` lists `target`, `corpus`, `artifacts`, `coverage`. |
| SC3 | 5 benches with `harness=false` + `noise_threshold(0.05)`, workspace compiles | PASS | All 5 bench files exist: `base60-core/benches/{convert,lens}.rs` + `base60-cli/benches/{dump,decode,search}.rs`. `harness = false` count: 2 (core) + 3 (cli) = 5. `noise_threshold(0.05)` present in all 5 files. `cargo bench --workspace --no-run --locked` finishes clean with 5 executable bench binaries (`convert-*`, `lens-*`, `dump-*`, `decode-*`, `search-*`). |
| SC4 | CLI benches README documents advisory-only posture | PASS | `crates/base60-cli/benches/README.md` opens with "Benchmarks — advisory only, NEVER CI-gated"; documents `--save-baseline pre` / `--baseline pre` workflow, per-bench scope table, 5% noise-threshold rationale, determinism notes, citation to PROJECT.md row 8 + PITFALLS Pitfall 9. `crates/base60-core/benches/README.md` is a pointer per D-30. |
| SC5 | `#[cfg(fuzzing)] pub` hatch verified — no new public items in non-fuzz build | PASS | `target/doc/base60/` index lists only `enum.Format.html`, `enum.LensMode.html`, `fn.run.html`, `cli/` module (Phase 3 surface). No `__fuzz` (cfg-gated), no `__bench` in rustdoc (doc-hidden); no `parse_run`/`RUN_LEN`/`dump_all`/`decode_stream`/`find_all`/`Palette`/`InputFormat`/`PALETTE_NONE` present in `base60/all.html`. Manual diff vs Phase 3 surface: unchanged. |

**Score:** 5/5 success criteria verified.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `fuzz/Cargo.toml` | Nested-workspace fuzz manifest with `libfuzzer-sys 0.4` + path deps + two `[[bin]]` | PASS | edition 2024, no rust-version, `resolver="3"`, `members=["."]`, both path deps present, `cargo-fuzz = true` marker |
| `fuzz/fuzz_targets/parse_run.rs` | Length-gated libFuzzer target | PASS | D-14 banner, `#![no_main]`, length gate, try_from guard, `let _ = base60::__fuzz::parse_run(arr, 1)` |
| `fuzz/fuzz_targets/pattern_from_str.rs` | UTF-8-guarded libFuzzer target | PASS | D-14 banner, `#![no_main]`, `str::from_utf8` guard, `let _ = base60::__fuzz::Pattern::from_str(s)` |
| `fuzz/.gitignore` | Excludes target/corpus/artifacts/coverage | PASS | All 4 lines present |
| `fuzz/README.md` | Ubuntu+nightly constraint + reproducer | PASS | Cites Pitfall 11, lists install steps, smoke commands, targets table |
| `Cargo.toml` (root) | `exclude = ["fuzz"]` | PASS | Line 4; plus `unexpected_cfgs check-cfg = ["cfg(fuzzing)"]` workspace lint (deviation, documented) |
| `crates/base60-cli/src/lib.rs` | `#[doc(hidden)] #[cfg(fuzzing)] pub mod __fuzz` + `#[doc(hidden)] pub mod __bench` | PASS | Both modules present (lines 54-59 `__fuzz`, lines 69-76 `__bench`); `__bench` not cfg-gated by design (benches always compile under `--all-targets`) |
| `crates/base60-cli/src/decode.rs` | `parse_run` + `RUN_LEN` widened | PASS | Widened to `pub` + `#[allow(unreachable_pub)]` with rationale (deviation from D-06's `pub(crate)`; documented) |
| `crates/base60-cli/src/search.rs` | `Pattern` accessible for fuzz re-export | PASS | `Pattern`, `ParseError`, `find_all` all `pub` + `#[allow(unreachable_pub)]` with rationale |
| `crates/base60-cli/src/color.rs` | `Palette`, `PALETTE_NONE` accessible for bench re-export | PASS | `Palette` struct widened to `pub` + `#[derive(Debug)]` + `#[allow(unreachable_pub)]`; `PALETTE_NONE` same |
| `crates/base60-cli/src/dump.rs` | `dump_all` accessible for bench re-export | PASS | `pub fn dump_all` + `#[allow(unreachable_pub)]` + `# Errors` rustdoc |
| `crates/base60-cli/src/cli.rs` | `InputFormat` accessible for bench re-export | PASS | `pub enum InputFormat` + `#[allow(unreachable_pub)]` with rationale |
| `crates/base60-core/Cargo.toml` | criterion in dev-deps, 2 `[[bench]]` entries | PASS | `criterion = { version = "0.8", default-features = false, features = [...] }` in `[dev-dependencies]`; `[dependencies]` section absent (zero-dep invariant preserved); 2× `harness = false` |
| `crates/base60-cli/Cargo.toml` | criterion in dev-deps, 3 `[[bench]]` entries | PASS | criterion in `[dev-dependencies]`; 3× `harness = false` |
| `crates/base60-core/benches/{convert,lens}.rs` | Criterion benches with 0.05 noise threshold | PASS | Both present, `Criterion::default().noise_threshold(0.05).sample_size(50)` |
| `crates/base60-cli/benches/{dump,decode,search}.rs` | Criterion benches | PASS | All three present + same noise/sample config |
| `crates/base60-cli/benches/README.md` | Advisory-only posture | PASS | Opens "Benchmarks — advisory only, NEVER CI-gated" |
| `crates/base60-core/benches/README.md` | Pointer to CLI README | PASS | One-liner `See ../../base60-cli/benches/README.md` |

### Key-Link Verification (Wiring)

| From | To | Via | Status | Detail |
|------|----|-----|--------|--------|
| `fuzz/fuzz_targets/parse_run.rs` | `crates/base60-cli/src/decode.rs::parse_run` | `use base60::__fuzz::{parse_run, RUN_LEN}` | WIRED | `cargo +nightly fuzz build` succeeds; 30s smoke exec/s ~349k — real function reached with real mutations |
| `fuzz/fuzz_targets/pattern_from_str.rs` | `crates/base60-cli/src/search.rs::Pattern` | `use base60::__fuzz::Pattern; Pattern::from_str(s)` | WIRED | 30s smoke exec/s ~109k; recommended dictionary includes `"hex:"` prefix — parser actually exercised |
| root `Cargo.toml` `[workspace]` | `fuzz/` | `exclude = ["fuzz"]` | WIRED | Main workspace CI matrix untouched; `cargo test --workspace` does not attempt to compile fuzz crate |
| `benches/*.rs` | cli/core internals | `base60::__bench::{...}` + `base60_core::{...}` | WIRED | `cargo bench --workspace --no-run --locked` compiles 5 bench binaries; `cargo test --all-targets` runs bench stubs emitting "Testing <name>, Success" |

### Data-Flow Trace (Level 4)

N/A — this phase is pure scaffolding (no dynamic-data rendering artifacts). The "data flow" is libFuzzer mutations → fuzz target → CLI internal, which is verified by the live 30 s smoke (real bytes in, real parser exercised, non-zero coverage growth recorded).

### Behavioural Spot-Checks

| Behaviour | Command | Result | Status |
|-----------|---------|--------|--------|
| `cargo fmt --all --check` | — | exit 0, no output | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | — | exit 0 | PASS |
| `cargo test --workspace --all-targets --locked` | — | 232 tests passed, 0 failed (139 cli lib + 41 core lib + 16 cli + 4 fixtures + 3 persist + 3 reader + 1 roundtrip + 1 tui + 1 env_discipline + 1 spawn_discipline + 22 doc + bench test-mode stubs all "Success") | PASS |
| `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --locked` | — | exit 0, generated docs in `target/doc/` | PASS |
| `cargo bench --workspace --no-run --locked` | — | 5 bench binaries compiled (`convert`, `lens`, `dump`, `decode`, `search`) | PASS |
| `cd fuzz && cargo +nightly fuzz build` | — | exit 0 | PASS |
| `cargo +nightly fuzz run parse_run -- -max_total_time=30` | — | Done 10818084 runs in 31 s, exec/s ~349 k, no crash, artifacts dir empty | PASS |
| `cargo +nightly fuzz run pattern_from_str -- -max_total_time=30` | — | Done 3438107 runs in 31 s, exec/s ~109 k, no crash, artifacts dir empty | PASS |
| 4 mandatory search cells present in `benches/search.rs` | `grep -E 'b"\x00"\|b"\xff\xff"\|b"ELF"\|b"cafebabe"'` | 4 hits (`zero_fill/1byte_null`, `zero_fill/2byte_ffff`, `random/3byte_elf`, `random/8byte_cafebabe`) | PASS |
| CI-03 zero-dep-core invariant preview | `awk '/^\[dependencies\]/,/^\[/' base60-core/Cargo.toml` | Empty — `[dependencies]` section absent from `base60-core/Cargo.toml` | PASS |
| Public API unchanged | `ls target/doc/base60/` | Only `all.html`, `cli/`, `enum.Format.html`, `enum.LensMode.html`, `fn.run.html`, `index.html`, `sidebar-items.js` — identical to Phase 3 surface | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| TEST-02 | 05-01-PLAN (`requirements: [TEST-02]`) | cargo-fuzz workspace + targets for `decode::parse_run` via `#[cfg(fuzzing)]` hatch and `Pattern::from_str` | SATISFIED | Full fuzz scaffolding present; both targets compile + run 30 s smoke; `__fuzz` hatch uses `#[doc(hidden)] #[cfg(fuzzing)] pub mod`; commit `db93817` |
| PERF-06 | 05-02-PLAN (`requirements: [PERF-06]`) | criterion benches in both crates' `benches/`, advisory-only | SATISFIED | 5 bench files + criterion dev-dep (both crates) + advisory README + `__bench` hatch; commit `f603f63` |

No orphaned requirements — ROADMAP Phase 5 declares exactly TEST-02 + PERF-06, both are addressed.

### Anti-Pattern Scan

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | — | — | — | Scan across phase-modified files returned no TODO/FIXME/placeholder/stub patterns. Every change is load-bearing scaffolding. |

Bench input arrays use `static LazyLock<Vec<u8>>` intentionally (deviation documented in 05-02-SUMMARY #2) to avoid `long_running_const_eval` + `large_const_arrays` + `large_stack_frames` under `-D warnings`. Deterministic mixer preserved. Not a stub — real inputs drive real bench code.

### Deviations Recap (Documented in SUMMARYs)

All deviations from the plan were auto-fixed during execution and are internally consistent with the `__fuzz`/`__bench` shims:

1. **pub + `#[allow(unreachable_pub)]` instead of `pub(crate)`** (05-01 + 05-02) — Required by Rust re-export rules (E0364/E0365). Public API stays pristine because enclosing `mod decode/search/color/cli/dump` are all private at crate root. Verified by rustdoc diff.
2. **`unexpected_cfgs check-cfg = ["cfg(fuzzing)"]`** added to `[workspace.lints.rust]` — Required so clippy `-D warnings` accepts `#[cfg(fuzzing)]`. Idiomatic remediation.
3. **`Palette` gained `#[derive(Debug)]`** — Required by workspace lint `missing_debug_implementations` once struct was widened to `pub`. Trivially derivable (all `&'static str` fields).
4. **1 MiB bench inputs use `static LazyLock<Vec<u8>>` instead of `const [u8; 1<<20]`** — Switched to sidestep three `-D warnings` lints; deterministic mixer preserved.

All four deviations are surgical single-file fixes that preserve plan intent; the `__fuzz` and `__bench` shims correctly re-export every item they claim to (verified by successful fuzz build + bench compile).

### Human Verification Required

None. Every success criterion is verifiable by automated commands that completed successfully during this verification run. Visual/UX concerns do not apply (this is infrastructure scaffolding with no UI surface).

### Gaps

None.

---

## Final Verdict

**VERIFICATION PASSED** — 5/5 success criteria verified.

- All artifacts exist and match expected shapes.
- All wiring (fuzz targets → `__fuzz` → CLI internals; benches → `__bench` → CLI internals; root workspace `exclude`) is live and exercised.
- Phase 3 D-24 gate green: fmt + clippy `-D warnings` + 232 passing tests + rustdoc `-D warnings`.
- `cargo bench --workspace --no-run --locked` compiles all 5 benches.
- `cargo +nightly fuzz build` succeeds; 30 s smokes on both targets complete with no crash (parse_run: 10.8M runs @ 349k exec/s; pattern_from_str: 3.4M runs @ 109k exec/s).
- CI-03 zero-dep-core invariant preserved: `base60-core/Cargo.toml [dependencies]` section remains empty.
- Public API surface unchanged: rustdoc shows only Phase 3 items (`Format`, `LensMode`, `run`, `cli` module). No leakage from `__fuzz` (cfg-gated) or `__bench` (doc-hidden).
- Four deviations from plan are documented, auto-fixed, internally consistent.

Phase 5 is ready; Phase 6 (PERF-01..05) and Phase 7 (CI hardening) can consume this scaffolding directly.

---

*Verified: 2026-04-24T22:00:00Z*
*Verifier: Claude (gsd-verifier)*
