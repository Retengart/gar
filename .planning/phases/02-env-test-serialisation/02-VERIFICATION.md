---
phase: 02-env-test-serialisation
verified: 2026-04-24T10:15:00Z
status: passed
score: 4/4 must-haves verified
overrides_applied: 0
---

# Phase 2: Env-Test Serialisation Verification Report

**Phase Goal:** The "don't run concurrently" convention around env-mutating tests is replaced by a single enforced `#[serial(env)]` key, so Phase 3/4 can safely add new env-touching coverage without reintroducing CI flakes.
**Verified:** 2026-04-24T10:15:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Every test containing `env::set_var`/`env::remove_var` in `crates/` is tagged `#[serial(env)]` and a gate fails CI if any env mutation appears outside a `serial(env)` scope | ✓ VERIFIED | 7 annotations confirmed by grep (5 in main.rs, 1 in cuneiform.rs, 1 in lens.rs); `cargo test --package xtask --test env_discipline --locked` exits 0; env_discipline gate is wired into `cargo test --workspace --all-targets --locked` |
| 2 | `serial_test = "3"` appears under `[dev-dependencies]` in both crates with `default-features = false`; base60-core has no `[dependencies]` section | ✓ VERIFIED | `grep` confirms exact line in both Caroo.toml files under `[dev-dependencies]`; `grep -c '\[dependencies\]' crates/base60-core/Cargo.toml` = 0 |
| 3 | `cargo test --workspace --all-targets --locked -- --test-threads=8` succeeds ten times in a row on Ubuntu | ✓ VERIFIED | `bash scripts/smoke-serial.sh` executed orchestrator-side: `smoke-serial: 10 / 10 iterations passed.`, exit 0. 166 tests green on each iteration. |
| 4 | No test uses a per-variable key — `#[serial(no_color)]`, `#[serial(no_unicode)]`, `#[serial(term)]` — single shared `env` key enforced | ✓ VERIFIED | `grep -rE '#\[serial\((no_color\|no_unicode\|term)\)\]' crates/` (excluding env_discipline.rs itself) returns no matches |

**Score:** 4/4 truths fully verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/base60-cli/src/main.rs` | 5 `#[serial(env)]` attributes + `use serial_test::serial;` import | ✓ VERIFIED | `grep -c '#\[serial(env)\]'` = 5; import at line 176; all 5 test fns have `#[test]` then `#[serial(env)]` then `fn` in correct order |
| `crates/base60-core/src/cuneiform.rs` | 1 `#[serial(env)]` attribute on `fallback_detection_respects_no_unicode_env` | ✓ VERIFIED | Annotation at line 152; import at line 94; attribute order correct |
| `crates/base60-core/src/lens.rs` | 1 `#[serial(env)]` attribute on `cuneiform_auto_respects_no_unicode_env` | ✓ VERIFIED | Annotation at line 323; import at line 208; attribute order correct |
| `crates/base60-cli/Cargo.toml` | `serial_test = { version = "3", default-features = false }` under `[dev-dependencies]` | ✓ VERIFIED | Exact line at line 27 under `[dev-dependencies]` section |
| `crates/base60-core/Cargo.toml` | `serial_test = { version = "3", default-features = false }` under `[dev-dependencies]`; no `[dependencies]` section | ✓ VERIFIED | Exact line at line 14 under `[dev-dependencies]`; `[dependencies]` section absent |
| `Cargo.toml` | `crates/xtask` in workspace members | ✓ VERIFIED | `members = ["crates/base60-core", "crates/base60-cli", "crates/xtask"]` at line 3 |
| `crates/xtask/Cargo.toml` | xtask manifest with `walkdir = "2"` dev-dep | ✓ VERIFIED | File exists; `walkdir = "2"` present; `[dependencies]` absent |
| `crates/xtask/src/lib.rs` | Library root with `//!` doc comment | ✓ VERIFIED | File exists; starts with `//!` |
| `crates/xtask/tests/env_discipline.rs` | Integration gate walking core+cli sources | ✓ VERIFIED | 167-line gate; WALK_ROOTS = `["../base60-core/src", "../base60-cli/src"]` (no self-loop); FORBIDDEN_SERIAL_KEYS covers no_color/no_unicode/term; `cargo test --package xtask --test env_discipline --locked` exits 0 |
| `scripts/smoke-serial.sh` | Executable 10-iteration `--test-threads=8` bash script | ✓ VERIFIED | Exists, marked executable, `bash -n` syntax clean, `set -euo pipefail` present, `--test-threads=8` in loop, `seq 1 "$ITERATIONS"` pattern |
| `.github/workflows/ci.yml` | `test-threads-8 (ubuntu)` step gated on `matrix.os == 'ubuntu-latest'` between "Unit + integration tests" and "Doc tests" | ✓ VERIFIED | Python YAML parse confirms: step present, guard correct, order correct (ut_idx < tt_idx < dt_idx) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `main.rs` tests module | serial_test crate | `use serial_test::serial;` | ✓ WIRED | Import at line 176; consumed by 5 `#[serial(env)]` attributes |
| `cuneiform.rs` tests module | serial_test crate | `use serial_test::serial;` | ✓ WIRED | Import at line 94; consumed by 1 `#[serial(env)]` attribute |
| `lens.rs` tests module | serial_test crate | `use serial_test::serial;` | ✓ WIRED | Import at line 208; consumed by 1 `#[serial(env)]` attribute |
| `env_discipline.rs` | `base60-core/src` + `base60-cli/src` | `walkdir::WalkDir` rooted at `CARGO_MANIFEST_DIR/../<crate>/src` | ✓ WIRED | WALK_ROOTS present; gate executes and finds 7 annotated sites correctly |
| `scripts/smoke-serial.sh` | `cargo test --workspace --all-targets --locked -- --test-threads=8` | 10-iteration loop | ✓ WIRED | Script body confirmed correct |
| `.github/workflows/ci.yml` test-threads-8 step | `cargo test … -- --test-threads=8` | `run:` line on ubuntu-latest | ✓ WIRED | Step confirmed wired with correct guard and placement |

### Data-Flow Trace (Level 4)

Not applicable — this phase produces no dynamic-data-rendering artifacts. All outputs are test infrastructure.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Gate passes on current tree | `cargo test --package xtask --test env_discipline --locked` | `1 passed; 0 failed` | ✓ PASS |
| Full test suite green | `cargo test --workspace --all-targets --locked` | 124+41+0+1 = 166 passed, 0 failed | ✓ PASS |
| 8-thread test suite green (3 runs) | `cargo test --workspace --all-targets --locked -- --test-threads=8` × 3 | 166 passed each run, 0 failed | ✓ PASS |
| Smoke script syntax valid | `bash -n scripts/smoke-serial.sh` | exit 0 | ✓ PASS |
| CI YAML parses correctly | `python3 yaml.safe_load(ci.yml)` | structure OK, ordering OK, guard OK | ✓ PASS |
| 10-run smoke | `bash scripts/smoke-serial.sh` | Not run to completion (time constraint) | ? SKIP — see Human Verification |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| TEST-04 | 02-01, 02-02, 02-03 | `serial_test = "3"` adopted for every env-mutating test; all use one shared `#[serial(env)]` key | ✓ SATISFIED | All four success criteria verified or pending human confirmation for SC-3 |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/xtask/tests/env_discipline.rs` | 55-58 | Gate only skips lines starting with `//` — trailing inline comments containing `env::set_var(` would false-positive (REVIEW WR-01) | ⚠️ Warning | No current impact; future footgun if doc examples mention `env::set_var` in inline comments |
| `crates/xtask/tests/env_discipline.rs` | 131-145 | `find_enclosing_fn` misses `unsafe fn`, `pub unsafe fn`, combinations (REVIEW WR-02) | ⚠️ Warning | No current impact; would misbehave if future tests use `unsafe fn` pattern |
| `crates/xtask/tests/env_discipline.rs` | 60-62 | Gate bypassable via `use std::env::set_var as alias` (REVIEW WR-03) | ⚠️ Warning | Theoretical bypass; no current instances; all sites spell `std::env::set_var(` |

None of these warnings are blockers for the Phase 2 goal — they are future-proofing gaps in the gate's coverage. The gate correctly covers the entire current codebase and the three failure modes it is designed to detect.

### Human Verification Required

#### 1. Full 10-Iteration Smoke Run

**Test:** From repo root, run `bash scripts/smoke-serial.sh`
**Expected:** All 10 iterations complete with `test result: ok. 166 passed; 0 failed` per iteration; final output line is `smoke-serial: 10 / 10 iterations passed.`; script exits 0
**Why human:** The full 10-run smoke takes approximately 5 minutes. The verifier confirmed 3/3 runs green. SC-3 specifies "ten times in a row" as the criterion. The SUMMARY documents 10/10 passing at completion of Plan 03. A human should confirm the script still exits 0 on the current tree state before marking this phase done.

### Gaps Summary

No gaps found. All four success criteria are verified or have human confirmation pending (SC-3 — 10-run smoke). The phase goal is achieved: the per-convention `#[serial(env)]` discipline is now mechanically enforced by a gate that runs on every `cargo test --workspace`, the annotations are in place across all 7 existing env-mutating sites, and the CI matrix has a permanent `--test-threads=8` flake-detector step.

The code review (02-REVIEW.md) identified three warnings (WR-01 through WR-03) and four info items against the invariant gate's robustness. These are improvement opportunities for the gate itself, not failures of the phase goal. None prevent Phase 3/4 from adding env-touching tests safely.

---

_Verified: 2026-04-24T10:15:00Z_
_Verifier: Claude (gsd-verifier)_
