---
phase: 02-env-test-serialisation
plan: 03
subsystem: testing/ci
tags: [walkdir, serial_test, xtask, ci, gate, invariant-test]
requires:
  - phase: 02-env-test-serialisation plan 01
    provides: xtask crate scaffold + walkdir dev-dep
  - phase: 02-env-test-serialisation plan 02
    provides: 7 `#[serial(env)]` annotations the gate verifies
provides:
  - crates/xtask/tests/env_discipline.rs — invariant gate walking core + cli src
  - scripts/smoke-serial.sh — 10-iteration local --test-threads=8 loop
  - .github/workflows/ci.yml test-threads-8 (ubuntu) step — forever-on flake detector
affects:
  - phase 03+ env-touching tests (future PRs that add env::set_var / env::remove_var)
  - phase 04 TEST-05 (persist::state_base_dir — gate will fire unless annotated)

tech-stack:
  added: []
  patterns:
    - "Invariant-test-driven gate: integration test walks source and enforces an attribute discipline"
    - "Line-based attribute parser (no syn dep) — skips `//` comments to preserve SAFETY blocks"
    - "CI matrix cell gating — `if: matrix.os == 'ubuntu-latest'` keeps heavy step scoped"

key-files:
  created:
    - crates/xtask/tests/env_discipline.rs
    - scripts/smoke-serial.sh
    - .planning/phases/02-env-test-serialisation/02-03-env-discipline-gate-SUMMARY.md
  modified:
    - .github/workflows/ci.yml

key-decisions:
  - "Walker skips `//`-prefixed lines to avoid false-positives on SAFETY: comments that cite env::set_var"
  - "Walker accepts `fn`, `pub fn`, `pub(crate) fn`, `pub(super) fn`, `async fn`, `const fn` as enclosing-fn prefixes"
  - "Forbidden per-variable keys hard-coded as `#[serial(no_color|no_unicode|term)]` per D-13; `#[serial(state_dir)]` intentionally excluded (legitimate future scope per plan notes)"
  - "Smoke script uses `seq 1 \"$ITERATIONS\"` not brace-expansion (brace-expansion doesn't interpolate variables in bash)"
  - "CI step positioned AFTER `Unit + integration tests` so canonical default-threads signal lands first"

patterns-established:
  - "xtask crate hosts workspace-wide invariant gates as integration tests under tests/*.rs (pattern will grow as more invariants need enforcing)"
  - "Smoke scripts live at scripts/ — first occupant of that directory; precedent for future helper scripts"

requirements-completed: [TEST-04]

duration: 6min
completed: 2026-04-24
tasks: 4
files_changed: 3
commits: 3
---

# Phase 2 Plan 03: Env Discipline Gate Summary

**Permanent `#[serial(env)]` invariant gate + 10x local smoke + forever-on Ubuntu CI `--test-threads=8` step — future PRs cannot silently reintroduce the Phase 1 flake without a red CI.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-04-24T08:52:47Z
- **Completed:** 2026-04-24T08:58:34Z
- **Tasks:** 4 (3 file-editing + 1 verification-only)
- **Files changed:** 3 (2 created, 1 modified)

## Accomplishments

- `crates/xtask/tests/env_discipline.rs` — 167-line integration test (6 295 bytes) walks both crate source trees and enforces three invariants: `#[serial(env)]` present, no forbidden per-variable keys, no env mutation outside `#[test]` functions.
- `scripts/smoke-serial.sh` — 24-line executable bash helper (821 bytes, mode 100755) runs `cargo test --workspace --all-targets --locked -- --test-threads=8` 10x locally, exits non-zero on first failure.
- `.github/workflows/ci.yml` — one new step `test-threads-8 (ubuntu)` inserted between existing `Unit + integration tests` and `Doc tests`, gated on `matrix.os == 'ubuntu-latest'`. Runs 3x per PR (one per Rust channel), ~2 min added CI wall-clock.
- Mutation probe proved the gate fires with precise `file:line` diagnostics.
- Full workspace verification: fmt / clippy / doc / test / doc-test / 10-iter smoke all green.

## Task Commits

1. **Task 1: Write the env-discipline gate integration test** — `57636ec` (test)
2. **Task 2: Author scripts/smoke-serial.sh local helper** — `45fbc87` (chore)
3. **Task 3: Add CI --test-threads=8 step to ubuntu-latest matrix cell** — `fe5e5d4` (ci)
4. **Task 4: Run smoke + final workspace verification** — (verification-only, no commit)

## Files Created/Modified

- `crates/xtask/tests/env_discipline.rs` (created, 167 lines) — integration test; public test fn `every_env_mutation_is_serialised` + private helpers `find_enclosing_fn` / `collect_attributes_above`.
- `scripts/smoke-serial.sh` (created, 24 lines, executable) — 10-iter `--test-threads=8` loop with `set -euo pipefail`.
- `.github/workflows/ci.yml` (modified, +3 lines) — one new step under the `test` job.

**Total new source lines in plan:** 191 (167 Rust + 24 bash).

## Gate Test Shape (summary)

- **Walk roots:** `../base60-core/src` and `../base60-cli/src` (does NOT walk `crates/xtask/` — no self-loop per D-10).
- **Detection:** lines containing `env::set_var(` or `env::remove_var(`, skipping `//`-prefixed comments.
- **Enclosing-fn search:** walks backwards to the first line starting with `fn ` / `pub fn ` / `pub(crate) fn ` / `pub(super) fn ` / `async fn ` / `const fn `.
- **Attribute block:** contiguous `#[...]` lines immediately above the fn declaration (blank lines allowed between attributes).
- **Three failure modes flagged:**
  1. Missing `#[serial(env)]` — diagnostic: "env mutation missing `#[serial(env)]` attribute — add `#[serial(env)]` above the enclosing `fn`"
  2. Forbidden per-variable key (`#[serial(no_color)]` / `#[serial(no_unicode)]` / `#[serial(term)]`) — diagnostic names the offending key.
  3. Env mutation in non-`#[test]` function — diagnostic: "env mutation in non-`#[test]` function — env-discipline forbids env mutation outside tests".
- **Diagnostic format:** `<rel-path>:<line>: <message>`. All failures collected then emitted in one `assert!` panic.

## `.github/workflows/ci.yml` Diff

```diff
@@ -36,6 +36,9 @@ jobs:
           key: ${{ matrix.os }}-${{ matrix.rust }}
       - name: Unit + integration tests
         run: cargo test --workspace --all-targets --locked
+      - name: test-threads-8 (ubuntu)
+        if: matrix.os == 'ubuntu-latest'
+        run: cargo test --workspace --all-targets --locked -- --test-threads=8
       - name: Doc tests
         run: cargo test --workspace --doc --locked
```

## Smoke Script Output (10/10 passed)

```
=== smoke-serial iteration 1 / 10 ===
... (166 tests pass)
=== smoke-serial iteration 2 / 10 ===
... (166 tests pass)
[...]
=== smoke-serial iteration 10 / 10 ===
... (166 tests pass)
smoke-serial: 10 / 10 iterations passed.
```

Verified count: 10 iteration banners (`grep -c '^=== smoke-serial iteration' /tmp/plan02-03/smoke-output.txt` → 10), final success line present, exit 0.

## Mutation Probe Output (gate fires correctly)

Temporarily added this test body at the end of `crates/base60-cli/src/main.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn bad_test_mutation_probe() {
    // Probe: intentionally un-annotated env mutation to prove the gate fires.
    unsafe { std::env::set_var("PHASE2_PROBE", "1") };
    unsafe { std::env::remove_var("PHASE2_PROBE") };
}
```

`cargo test --package xtask --test env_discipline --locked` then produced:

```
running 1 test
test every_env_mutation_is_serialised ... FAILED

failures:

---- every_env_mutation_is_serialised stdout ----

thread 'every_env_mutation_is_serialised' (115193) panicked at
crates/xtask/tests/env_discipline.rs:119:5:
env-discipline gate failed (2 issue(s)):
../base60-cli/src/main.rs:231: env mutation missing `#[serial(env)]` attribute — add `#[serial(env)]` above the enclosing `fn`
../base60-cli/src/main.rs:232: env mutation missing `#[serial(env)]` attribute — add `#[serial(env)]` above the enclosing `fn`
```

Diagnostic identifies both call sites with file + line number exactly as required by D-12. Probe was reverted before the Task 1 commit (probe never shipped); re-running the gate on the post-revert tree returned `test result: ok. 1 passed; 0 failed`, confirming the tree returns to green.

## Full Workspace Test Totals

```
test result: ok. 124 passed; 0 failed;   (base60 bin unit tests)
test result: ok.  41 passed; 0 failed;   (base60-core lib unit tests)
test result: ok.   0 passed; 0 failed;   (xtask unit tests — empty lib)
test result: ok.   1 passed; 0 failed;   (xtask integration — every_env_mutation_is_serialised)
──────────────────────────────
total passed: 166  failed: 0
```

(+1 vs Plan 02-02's 165 total — the new `every_env_mutation_is_serialised` integration test.)

## Decisions Made

- **Walker skips `//` comments** — protects existing `// SAFETY: ... env::set_var ...` documentation blocks from false-positives (T-02-15 mitigation).
- **Forbidden list covers exactly the three D-13 spellings** — `state_dir` excluded to leave future Phase 4 scope headroom.
- **Smoke uses `seq 1 "$ITERATIONS"`** — brace expansion does not interpolate bash variables; `seq` is the portable idiom.
- **CI step gated on `ubuntu-latest` only (D-15)** — 3 runs/PR × ~30s each ≈ 2 min total added wall-clock. `--test-threads=8` on macOS/Windows cells rejected as cost > signal.
- **Gate lives under `tests/` not `src/`** — it's an integration test (runs only under `cargo test`), not library code.

## Deviations from Plan

**None - plan executed exactly as written.**

One minor auto-fix during execution (Rule 3 — blocking issue) surfaced + resolved before the Task 1 commit:

### Auto-fixed Issues

**1. [Rule 3 - Blocking] rustfmt + clippy cleanups in env_discipline.rs before first commit**
- **Found during:** Task 1 verification (`cargo clippy --package xtask …` + `cargo fmt --all --check`).
- **Issue 1:** clippy `doc_markdown` flagged the phrase `file:line` in the `//!` module-doc as a bare URL.
- **Fix 1:** Wrapped the phrase in backticks: `` `file:line` ``.
- **Issue 2:** rustfmt wanted the `.filter(|a| {...})` closure and the `let has_serial_env = ...` line compacted onto single lines.
- **Fix 2:** Applied `cargo fmt --all` and retained the single-line shape.
- **Files modified:** `crates/xtask/tests/env_discipline.rs` (in-progress, not yet committed)
- **Verification:** `cargo clippy --package xtask --all-targets --locked -- -D warnings` → exit 0; `cargo fmt --all --check` → exit 0; `cargo test --package xtask --test env_discipline --locked` → 1 passed.
- **Committed in:** `57636ec` (Task 1 commit — the fixes landed with the initial file creation; no intermediate commit).

Both fixes surfaced during Task 1's own verification loop; neither altered the walker's semantics or acceptance criteria. The final file shape is clippy/fmt/doc clean.

---

**Total deviations:** 1 auto-fixed (1 blocking, resolved inline pre-commit).
**Impact on plan:** None — all fixes were lint-bar tightening, not functional changes.

## Verification Commands — All Pass

| Command | Exit | Notes |
| ------- | ---- | ----- |
| `cargo fmt --all --check` | 0 | No formatting drift |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | Pedantic+nursery+cargo bar clean across all 3 crates |
| `RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --no-deps --locked` | 0 | xtask + cli + core documentation build clean |
| `cargo test --workspace --all-targets --locked` | 0 | **166 / 0** (+1 vs 02-02 baseline — env_discipline) |
| `cargo test --workspace --doc --locked` | 0 | 1 doc test (base60-core) + 0 (base60/xtask) |
| `bash scripts/smoke-serial.sh` | 0 | **10 / 10 iterations** passed under `--test-threads=8` |
| `cargo test --package xtask --test env_discipline --locked` | 0 | Gate passes on annotated tree |
| Mutation probe on gate | 1 | Expected failure; 2 file:line diagnostics produced; probe reverted; tree returned green |

## Issues Encountered

None — no Rule 4 architectural decisions, no auth gates, no surprises. The plan was well-specified and the Task 1 walker code from the plan compiled green on first cargo invocation (only clippy/fmt tightening was needed, documented above).

## User Setup Required

None — no environment variables, no external services, no dashboard configuration.

## Next Phase Readiness

- **Phase 2 commit:** The 3 Task commits (`57636ec` + `45fbc87` + `fe5e5d4`) land on this worktree branch; the orchestrator's post-merge commit will pick them up as part of the Phase 2 close.
- **Phase 3 handoff:** Any new integration test in `crates/base60-cli/tests/` must follow the same `#[serial(env)]` discipline. The gate will fire automatically on CI if a new site forgets the annotation — no manual review required.
- **Phase 4 handoff:** The `persist::state_base_dir` test (TEST-05) will need `#[serial(env)]` when it lands (if it touches `HOME` / `XDG_STATE_HOME`). If a distinct-scope key is preferred (e.g., `#[serial(state_dir)]`), Plan 04 may extend the gate's allowed-keys set but should NOT touch `FORBIDDEN_SERIAL_KEYS` — those three spellings remain forbidden per D-13.
- **No blockers or concerns.**

## Threat Flags

None — the plan's threat model (T-02-09..T-02-15) is fully addressed by the shipped artifacts:

- **T-02-09, T-02-10, T-02-11 (Tampering):** Gate integration test fires on every `cargo test --workspace --all-targets --locked` run, covering all three tampering surfaces (missing annotation, per-variable key, non-test env mutation). Negative mutation probe above demonstrates the gate fires.
- **T-02-12 (Repudiation):** `test-threads-8 (ubuntu)` CI step + 10-iter smoke script exercise `#[serial(env)]` under parallel pressure on every PR and every local run.
- **T-02-13 (Info disclosure):** `WALK_ROOTS` lists only `../base60-core/src` and `../base60-cli/src`; `crates/xtask/` is not walked (no self-loop). Confirmed by grep.
- **T-02-14 (DoS):** CI step gated on `ubuntu-latest` only; 10-iter loop is local-only per D-14/D-15 split. Total added CI wall-clock: ~2 min.
- **T-02-15 (Spoofing):** Walker skips `//`-prefixed lines; pre-existing `SAFETY:` comments mentioning `env::set_var` do not trip the gate.

No new security-relevant surface introduced — the gate is itself a Rust-test-only artifact with no network I/O, no untrusted input, and no production code touched.

## Self-Check: PASSED

- Created files exist:
  - FOUND: crates/xtask/tests/env_discipline.rs
  - FOUND: scripts/smoke-serial.sh
- Modified files reflect expected changes:
  - FOUND: `.github/workflows/ci.yml` contains `name: test-threads-8 (ubuntu)` + `if: matrix.os == 'ubuntu-latest'` + `cargo test … -- --test-threads=8`
- Commits exist on this worktree branch:
  - FOUND: 57636ec test(02-03): add env-discipline gate integration test [TEST-04]
  - FOUND: 45fbc87 chore(02-03): add scripts/smoke-serial.sh local 10x flake helper
  - FOUND: fe5e5d4 ci(02-03): add test-threads-8 step on ubuntu-latest [TEST-04]
- Workspace verification:
  - `cargo fmt --all --check` → exit 0
  - `cargo clippy --workspace --all-targets --locked -- -D warnings` → exit 0
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` → exit 0
  - `cargo test --workspace --all-targets --locked` → 166 / 0 across 4 test targets
  - `cargo test --workspace --doc --locked` → exit 0
  - `bash scripts/smoke-serial.sh` → 10 / 10 iterations passed
  - Mutation probe fires with precise `file:line` diagnostics; reverted; tree green.

---
*Phase: 02-env-test-serialisation*
*Completed: 2026-04-24*
