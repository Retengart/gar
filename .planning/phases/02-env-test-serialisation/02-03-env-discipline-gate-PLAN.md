---
phase: 02-env-test-serialisation
plan: 03
type: execute
wave: 3
depends_on: [02-01, 02-02]
files_modified:
  - crates/xtask/tests/env_discipline.rs
  - scripts/smoke-serial.sh
  - .github/workflows/ci.yml
autonomous: true
requirements: [TEST-04]

must_haves:
  truths:
    - "Running `cargo test --workspace --all-targets --locked` executes an env-discipline gate that PASSES on the current tree."
    - "If a future developer adds `env::set_var` / `env::remove_var` to a test missing `#[serial(env)]`, the gate FAILS the test with a precise file:line diagnostic."
    - "If a future developer uses `#[serial(no_color)]` / `#[serial(no_unicode)]` / `#[serial(term)]`, the gate FAILS with a diagnostic naming the offending key."
    - "If a future developer introduces `env::set_var` / `env::remove_var` in a non-test function (production code), the gate FAILS."
    - "`scripts/smoke-serial.sh` runs 10 iterations of `cargo test --workspace --all-targets --locked -- --test-threads=8` and exits non-zero on the first failure."
    - "CI gains one step (`test-threads-8 (ubuntu)`) that runs the full workspace test suite with `--test-threads=8` on `ubuntu-latest` only."
  artifacts:
    - path: "crates/xtask/tests/env_discipline.rs"
      provides: "Integration-test-driven invariant gate; walks base60-cli + base60-core sources via walkdir."
      contains: "env::set_var"
    - path: "scripts/smoke-serial.sh"
      provides: "10-iteration local smoke runner for --test-threads=8"
      contains: "test-threads=8"
    - path: ".github/workflows/ci.yml"
      provides: "CI matrix gains 1 step running --test-threads=8 once on ubuntu-latest"
      contains: "test-threads=8"
  key_links:
    - from: "crates/xtask/tests/env_discipline.rs"
      to: "crates/base60-core/src + crates/base60-cli/src"
      via: "walkdir rooted at env!(CARGO_MANIFEST_DIR)/../<crate>/src"
      pattern: 'walkdir::WalkDir'
    - from: "scripts/smoke-serial.sh"
      to: "cargo test --workspace --all-targets --locked -- --test-threads=8"
      via: "10× loop"
      pattern: '--test-threads=8'
    - from: ".github/workflows/ci.yml (test-threads-8 step)"
      to: "cargo test --workspace --all-targets --locked -- --test-threads=8"
      via: "run: line on ubuntu-latest cell"
      pattern: 'test-threads=8'
---

<objective>
Close Phase 2 by landing (1) the invariant gate that refuses future regressions of the `#[serial(env)]` idiom, (2) the 10× local smoke helper, and (3) the CI step that pins `--test-threads=8` on Ubuntu.

Purpose: the gate is the phase's permanent contribution — once this plan ships, no future PR can silently reintroduce the Phase 1 flake without a CI failure on every matrix cell. The smoke script is a one-time local sanity gate the implementer runs before committing; the CI step is the running sanity gate every PR touches forever.

Output: `crates/xtask/tests/env_discipline.rs` as a Rust integration test; `scripts/smoke-serial.sh` as an executable bash helper; `.github/workflows/ci.yml` with one new step under the existing `test` job.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@.planning/phases/02-env-test-serialisation/02-CONTEXT.md
@.planning/phases/02-env-test-serialisation/02-01-workspace-prep-SUMMARY.md
@.planning/phases/02-env-test-serialisation/02-02-serial-env-annotations-SUMMARY.md
@.planning/research/PITFALLS.md
@.github/workflows/ci.yml
@crates/base60-cli/Cargo.toml
@crates/base60-core/Cargo.toml
@Cargo.toml

<interfaces>
<!-- walkdir 2.x key API (Context7 / docs.rs): the only bits the gate needs. -->

```rust
// From walkdir = "2"
use walkdir::WalkDir;

for entry in WalkDir::new(path)
    .into_iter()
    .filter_map(Result::ok)
{
    if entry.file_type().is_file()
        && entry.path().extension().is_some_and(|e| e == "rs")
    {
        // ...
    }
}
```

<!-- Current .github/workflows/ci.yml `test` job (for insertion context): -->

```yaml
  test:
    name: test (${{ matrix.os }} / ${{ matrix.rust }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: ['1.95.0', stable, beta]
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.os }}-${{ matrix.rust }}
      - name: Unit + integration tests
        run: cargo test --workspace --all-targets --locked
      - name: Doc tests
        run: cargo test --workspace --doc --locked
```
The new step inserts AFTER `- name: Unit + integration tests` (so the canonical green signal always runs first) and BEFORE `- name: Doc tests`, GATED on `matrix.os == 'ubuntu-latest'` per D-15.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Write the env-discipline gate integration test</name>
  <files>crates/xtask/tests/env_discipline.rs</files>
  <read_first>
    - crates/xtask/Cargo.toml (confirms walkdir dev-dep is present from Plan 01)
    - crates/xtask/src/lib.rs (confirms empty lib-only skeleton from Plan 01)
    - .planning/phases/02-env-test-serialisation/02-CONTEXT.md §D-07..D-13 (gate shape, walk roots, parser contract, rejected keys)
    - crates/base60-cli/src/main.rs lines 173-220 (reference shape the walker must accept)
    - crates/base60-core/src/cuneiform.rs lines 140-165 (reference shape the walker must accept)
    - crates/base60-core/src/lens.rs lines 310-335 (reference shape the walker must accept)
  </read_first>
  <action>
Create `crates/xtask/tests/env_discipline.rs` as a Rust integration test. Runs under `cargo test --workspace --all-targets --locked`. Per D-07, D-10, D-11, D-12, D-13.

**Full file content (write exactly this — the walker is kept deliberately simple; line-based per D-12):**

```rust
//! Env-discipline gate: every `env::set_var` / `env::remove_var` call in
//! `base60-core/src/**/*.rs` and `base60-cli/src/**/*.rs` must live inside a
//! test function bearing `#[serial(env)]` — no alternate keys, no production
//! code exceptions. Phase 2 (TEST-04) invariant.
//!
//! Walks both crate sources via `walkdir`. Line-based parser: for each
//! `env::set_var` / `env::remove_var` occurrence, walks upward to find the
//! enclosing `fn`, then confirms the preceding attribute block contains
//! exactly `#[serial(env)]` (no `#[serial(no_color)]` etc.) AND the function
//! also bears `#[test]`. Any deviation fails the test with a precise
//! file:line diagnostic.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Relative roots from this crate's manifest to walk.
const WALK_ROOTS: &[&str] = &[
    "../base60-core/src",
    "../base60-cli/src",
];

/// Attribute key shapes that are explicitly rejected (Phase 2 D-13).
const FORBIDDEN_SERIAL_KEYS: &[&str] = &[
    "#[serial(no_color)]",
    "#[serial(no_unicode)]",
    "#[serial(term)]",
];

#[test]
fn every_env_mutation_is_serialised() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut failures: Vec<String> = Vec::new();

    for root in WALK_ROOTS {
        let root_path: PathBuf = Path::new(manifest_dir).join(root);
        assert!(
            root_path.is_dir(),
            "walk root does not exist: {}",
            root_path.display()
        );

        for entry in WalkDir::new(&root_path)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().is_none_or(|e| e != "rs") {
                continue;
            }

            let path = entry.path();
            let contents = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let lines: Vec<&str> = contents.lines().collect();

            for (idx, line) in lines.iter().enumerate() {
                // Skip commented lines to avoid false positives on `SAFETY:`
                // comments that mention `env::set_var` for documentation.
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }

                let mentions_mutation = line.contains("env::set_var(")
                    || line.contains("env::remove_var(");
                if !mentions_mutation {
                    continue;
                }

                let line_no = idx + 1;
                let rel = path
                    .strip_prefix(manifest_dir)
                    .unwrap_or(path)
                    .display()
                    .to_string();

                // Walk upward to the enclosing `fn` declaration.
                let Some(fn_idx) = find_enclosing_fn(&lines, idx) else {
                    failures.push(format!(
                        "{rel}:{line_no}: env mutation has no enclosing `fn` — \
                         env-discipline requires this to be inside a \
                         `#[serial(env)]` test"
                    ));
                    continue;
                };

                // Scan attributes immediately above the fn declaration.
                let attrs = collect_attributes_above(&lines, fn_idx);

                let has_test = attrs.iter().any(|a| a.trim() == "#[test]");
                let has_serial_env =
                    attrs.iter().any(|a| a.trim() == "#[serial(env)]");
                let forbidden: Vec<&str> = attrs
                    .iter()
                    .map(|s| s.trim())
                    .filter(|a| {
                        FORBIDDEN_SERIAL_KEYS
                            .iter()
                            .any(|forbidden| a == forbidden)
                    })
                    .collect();

                if !has_test {
                    failures.push(format!(
                        "{rel}:{line_no}: env mutation in non-`#[test]` \
                         function — env-discipline forbids env mutation \
                         outside tests"
                    ));
                }
                if !forbidden.is_empty() {
                    failures.push(format!(
                        "{rel}:{line_no}: found forbidden serial_test key \
                         {:?}; use only `#[serial(env)]` (shared key)",
                        forbidden
                    ));
                }
                if !has_serial_env {
                    failures.push(format!(
                        "{rel}:{line_no}: env mutation missing \
                         `#[serial(env)]` attribute — add \
                         `#[serial(env)]` above the enclosing `fn`"
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "env-discipline gate failed ({count} issue(s)):\n{details}",
        count = failures.len(),
        details = failures.join("\n"),
    );
}

/// Walks backwards from `line_idx` to find the first line whose trimmed
/// prefix begins with `fn `, `pub fn `, `pub(crate) fn `, `pub(super) fn `,
/// `async fn ` or `const fn `. Returns the 0-based line index of that `fn`,
/// or `None` if no such line exists above.
fn find_enclosing_fn(lines: &[&str], line_idx: usize) -> Option<usize> {
    for i in (0..=line_idx).rev() {
        let t = lines[i].trim_start();
        if t.starts_with("fn ")
            || t.starts_with("pub fn ")
            || t.starts_with("pub(crate) fn ")
            || t.starts_with("pub(super) fn ")
            || t.starts_with("async fn ")
            || t.starts_with("const fn ")
        {
            return Some(i);
        }
    }
    None
}

/// Collects the contiguous block of attribute lines (`#[...]`) immediately
/// preceding `fn_idx`. Stops at the first non-attribute, non-blank line.
fn collect_attributes_above(lines: &[&str], fn_idx: usize) -> Vec<String> {
    let mut out = Vec::new();
    if fn_idx == 0 {
        return out;
    }
    for i in (0..fn_idx).rev() {
        let t = lines[i].trim_start();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("#[") {
            out.push(lines[i].to_string());
            continue;
        }
        // First non-attribute, non-blank line: stop.
        break;
    }
    out
}
```

**Implementation notes (enforced by acceptance criteria):**

1. File MUST be at `crates/xtask/tests/env_discipline.rs` (NOT `crates/xtask/src/`; it's an integration test, per D-08).
2. File MUST start with a `//!` module doc comment (RUSTDOCFLAGS `-D warnings` enforcement).
3. `WALK_ROOTS` MUST list exactly `"../base60-core/src"` and `"../base60-cli/src"`. NOT `"../xtask/src"` — the gate must not walk itself (D-10).
4. `FORBIDDEN_SERIAL_KEYS` MUST list exactly the three forms in D-13. Do NOT add `#[serial(state_dir)]` to the list — that's a legitimate future scope key if `persist::state_base_dir` test lands in Phase 4 and uses a distinct scope. The gate only rejects the three known-wrong per-variable spellings.
5. The walker MUST skip commented lines (`//`-prefixed) so that `// SAFETY: ... env::set_var ...` documentation comments don't false-positive.
6. The walker MUST flag three distinct failure modes:
   a. `env::set_var` / `env::remove_var` in a non-`#[test]` function (production code).
   b. Use of a forbidden per-variable key (Pitfall 1).
   c. Missing `#[serial(env)]` (the headline invariant).
7. Diagnostic messages MUST include the relative file path + line number of the offending call site (D-12). Do NOT output only the function name — line number is critical for triage.
8. The test function MUST be named `every_env_mutation_is_serialised` (single test function; the gate is atomic per D-07).
9. Clippy-pedantic-clean: the code uses `if let Some(fn_idx) = … else { … }` (let-else) and `is_none_or` / `is_some_and` — both stable in Rust 1.95 per MSRV. Do NOT add `#[allow(clippy::…)]` unless a specific lint fires; the code is written to avoid them.
10. RUSTDOCFLAGS: `-D warnings` applies — every `fn` you add (only `find_enclosing_fn`, `collect_attributes_above`) MUST have a `///` doc comment. The code above already has them.
  </action>
  <verify>
    <automated>cargo test --package xtask --test env_discipline --locked &amp;&amp; cargo clippy --package xtask --all-targets --locked -- -D warnings &amp;&amp; cargo doc --package xtask --no-deps --locked</automated>
  </verify>
  <acceptance_criteria>
    - `crates/xtask/tests/env_discipline.rs` exists.
    - File starts with a `//!` doc comment (first line).
    - `grep -c 'env::set_var' crates/xtask/tests/env_discipline.rs` returns ≥ 1 (the walker references the pattern).
    - `grep -c 'env::remove_var' crates/xtask/tests/env_discipline.rs` returns ≥ 1.
    - `grep -c '#\[serial(env)\]' crates/xtask/tests/env_discipline.rs` returns ≥ 1 (the string appears in the `has_serial_env` check).
    - `grep -q 'WALK_ROOTS' crates/xtask/tests/env_discipline.rs` succeeds AND the array lists `"../base60-core/src"` and `"../base60-cli/src"` (NOT `"../xtask/src"`).
    - `grep -q 'FORBIDDEN_SERIAL_KEYS' crates/xtask/tests/env_discipline.rs` succeeds AND includes `no_color`, `no_unicode`, `term`.
    - `cargo test --package xtask --test env_discipline --locked` exits 0 on the current tree (the gate passes because Plans 01 + 02 are in).
    - `cargo clippy --package xtask --all-targets --locked -- -D warnings` exits 0.
    - `RUSTDOCFLAGS='-D warnings' cargo doc --package xtask --no-deps --locked` exits 0.
    - **Negative check — mutation test:** Temporarily add a stub `#[test] fn bad_test() { unsafe { std::env::set_var("X", "1"); } }` to `crates/base60-cli/src/main.rs` (no `#[serial(env)]`), re-run `cargo test --package xtask --test env_discipline --locked`, and confirm it exits non-zero with the file path + line number of `bad_test` in the diagnostic. Then revert the stub. This verification proves the gate actually fires, not just that it exists. (Document this mutation test in the plan SUMMARY; do NOT leave the stub in the tree.)
  </acceptance_criteria>
  <done>Gate test passes on the current tree; fails when annotations are missing; clippy + doc clean; diagnostics include file:line.</done>
</task>

<task type="auto">
  <name>Task 2: Author scripts/smoke-serial.sh local helper</name>
  <files>scripts/smoke-serial.sh</files>
  <read_first>
    - .planning/phases/02-env-test-serialisation/02-CONTEXT.md §D-14 (10-iteration loop, non-zero on first failure, `set -euo pipefail` header)
    - .planning/phases/02-env-test-serialisation/02-CONTEXT.md §"Claude's Discretion" (exact script shape is discretionary — any `for i in {1..10}; do … || exit 1; done` variant is acceptable)
  </read_first>
  <action>
Create `scripts/` directory + `scripts/smoke-serial.sh` as an executable bash helper. Implementer runs it once locally before the final commit; it is NOT wired into CI (the CI step in Task 3 runs the command once, not 10×).

**Step 1** — Create the `scripts/` directory if it doesn't exist:
```bash
mkdir -p scripts
```

**Step 2** — Write `scripts/smoke-serial.sh` with EXACTLY this content:

```bash
#!/usr/bin/env bash
# smoke-serial.sh — Run the full workspace test matrix 10 times with
# --test-threads=8 to catch residual races in env-touching tests.
#
# Expected usage: run once locally before landing a Phase 2 commit.
# Does not replace the CI --test-threads=8 step (.github/workflows/ci.yml);
# this is the phase-handoff gate from TEST-04 Success Criterion 3.
#
# Exit 0 on 10/10 success. Exit non-zero with the iteration number on
# the first failure.

set -euo pipefail

ITERATIONS=10

for i in $(seq 1 "$ITERATIONS"); do
    echo "=== smoke-serial iteration $i / $ITERATIONS ==="
    if ! cargo test --workspace --all-targets --locked -- --test-threads=8; then
        echo "smoke-serial: iteration $i failed" >&2
        exit 1
    fi
done

echo "smoke-serial: $ITERATIONS / $ITERATIONS iterations passed."
```

**Step 3** — Mark the file executable:
```bash
chmod +x scripts/smoke-serial.sh
```
(In git, this records the executable bit; on a fresh clone the file will be executable.)

**Notes:**
- `#!/usr/bin/env bash` (NOT `#!/bin/bash`) for portability — works on macOS where `/bin/bash` is old-bash 3.2.
- `set -euo pipefail` is mandatory per D-14 (and per the "Specific Ideas" section of 02-CONTEXT.md).
- `$(seq 1 "$ITERATIONS")` instead of `{1..10}` — brace expansion with a variable doesn't work in bash (`{1..$VAR}` is literal). Using `seq` is the portable idiom.
- Redirect failure message to stderr (`>&2`) so CI / human can tell success log from failure log.
- Do NOT use `run_in_background`; this script is a foreground task.
  </action>
  <verify>
    <automated>test -x scripts/smoke-serial.sh &amp;&amp; head -1 scripts/smoke-serial.sh | grep -q '^#!/usr/bin/env bash$' &amp;&amp; grep -q 'set -euo pipefail' scripts/smoke-serial.sh &amp;&amp; grep -q '\-\-test-threads=8' scripts/smoke-serial.sh &amp;&amp; grep -q 'seq 1 ' scripts/smoke-serial.sh &amp;&amp; bash -n scripts/smoke-serial.sh</automated>
  </verify>
  <acceptance_criteria>
    - `scripts/smoke-serial.sh` exists and is marked executable (`test -x` passes).
    - First line is exactly `#!/usr/bin/env bash`.
    - Contains `set -euo pipefail` (exact line).
    - Contains `--test-threads=8` (for the cargo test invocation).
    - Loop iterates 10 times (`seq 1 10` or a clear literal).
    - `bash -n scripts/smoke-serial.sh` (syntax check) exits 0.
    - Comment block explains the script's purpose and phase-handoff role.
    - Script exits non-zero on any iteration failure (explicit `exit 1` inside the failure branch).
    - The script is NOT added to `.github/workflows/ci.yml` (Task 3 adds a one-shot CI step; this 10-loop helper is local only per D-14 / D-15 split).
  </acceptance_criteria>
  <done>`scripts/smoke-serial.sh` is an executable bash helper with `set -euo pipefail`, a 10-iteration `--test-threads=8` loop, and clean `bash -n` syntax.</done>
</task>

<task type="auto">
  <name>Task 3: Add CI --test-threads=8 step to ubuntu-latest matrix cell</name>
  <files>.github/workflows/ci.yml</files>
  <read_first>
    - .github/workflows/ci.yml (the full existing `test` job — especially lines 19-40)
    - .planning/phases/02-env-test-serialisation/02-CONTEXT.md §D-15 (Ubuntu matrix cell only, step name, positioning)
  </read_first>
  <action>
Add ONE new step to the existing `test` job. Step runs `cargo test --workspace --all-targets --locked -- --test-threads=8` and is gated on `matrix.os == 'ubuntu-latest'` per D-15.

**Step 1** — Locate the `test` job's `steps:` list (begins around line 28 of current `ci.yml`). Insert the new step BETWEEN the existing `- name: Unit + integration tests` step and the existing `- name: Doc tests` step. This ordering is intentional (per 02-CONTEXT.md "Specific Ideas"): the default-threads run always lands first as the canonical green signal.

**Step 2** — Exact YAML to insert (match the existing 6-space indentation of sibling steps):

```yaml
      - name: test-threads-8 (ubuntu)
        if: matrix.os == 'ubuntu-latest'
        run: cargo test --workspace --all-targets --locked -- --test-threads=8
```

**Step 3** — Resulting `test` job `steps:` block (for reference — verify your edit matches this shape exactly):

```yaml
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.os }}-${{ matrix.rust }}
      - name: Unit + integration tests
        run: cargo test --workspace --all-targets --locked
      - name: test-threads-8 (ubuntu)
        if: matrix.os == 'ubuntu-latest'
        run: cargo test --workspace --all-targets --locked -- --test-threads=8
      - name: Doc tests
        run: cargo test --workspace --doc --locked
```

**Notes:**
- Step name is exactly `test-threads-8 (ubuntu)` per D-15. Do NOT rename to `test (threads=8)` or `test-8-threads` — the name is the CI artifact identifier and should stay stable.
- `if: matrix.os == 'ubuntu-latest'` is the only correct gating shape. The `matrix.os` and `matrix.rust` variables are available in `if:` expressions (already used elsewhere in the same file). This step runs on all three Rust channels (1.95 / stable / beta) on Ubuntu only — that's 3 runs per PR, ~2 minutes total, which is the intended cost per D-15.
- Do NOT add this step to a new job; the existing `test` job is the right home (shares cache + setup).
- Do NOT touch any other job (clippy, fmt, doc, release-build). Do NOT touch any env-var or trigger. Only insert the one step.
- YAML is whitespace-sensitive: use 6 spaces for step dashes (matching the existing `- name: Doc tests` step).
  </action>
  <verify>
    <automated>grep -q 'name: test-threads-8 (ubuntu)' .github/workflows/ci.yml &amp;&amp; grep -q "if: matrix.os == 'ubuntu-latest'" .github/workflows/ci.yml &amp;&amp; grep -q 'cargo test --workspace --all-targets --locked -- --test-threads=8' .github/workflows/ci.yml &amp;&amp; python3 -c 'import yaml,sys;d=yaml.safe_load(open(".github/workflows/ci.yml"));steps=d["jobs"]["test"]["steps"];names=[s.get("name","") for s in steps];assert "test-threads-8 (ubuntu)" in names,"new step missing";tt_idx=names.index("test-threads-8 (ubuntu)");ut_idx=names.index("Unit + integration tests");dt_idx=names.index("Doc tests");assert ut_idx < tt_idx < dt_idx,"step order wrong";assert steps[tt_idx].get("if")=="matrix.os == 'ubuntu-latest'","if-guard missing";print("ci.yml structure OK")'</automated>
  </verify>
  <acceptance_criteria>
    - `.github/workflows/ci.yml` contains exactly one step named `test-threads-8 (ubuntu)`.
    - That step has `if: matrix.os == 'ubuntu-latest'`.
    - That step's `run:` line is exactly `cargo test --workspace --all-targets --locked -- --test-threads=8`.
    - The step is positioned AFTER `- name: Unit + integration tests` and BEFORE `- name: Doc tests` inside the `test` job's `steps:` list.
    - YAML parses cleanly (validated by Python `yaml.safe_load`).
    - No other job / step / trigger is modified — `git diff .github/workflows/ci.yml` shows only the one insertion block.
    - The existing `Unit + integration tests` step and `Doc tests` step remain unchanged.
    - No new `env:` vars, no new trigger branches, no matrix changes.
  </acceptance_criteria>
  <done>`.github/workflows/ci.yml` has one new `test-threads-8 (ubuntu)` step correctly gated and correctly positioned; YAML parses.</done>
</task>

<task type="auto">
  <name>Task 4: Run smoke script + final workspace verification</name>
  <files>(verification only — no files edited)</files>
  <read_first>
    - scripts/smoke-serial.sh (the script from Task 2)
    - .planning/phases/02-env-test-serialisation/02-CONTEXT.md §D-14 (smoke is a one-time phase-handoff gate)
  </read_first>
  <action>
Run the 10-iteration smoke gate locally + the full workspace test/lint/doc matrix. This is the last pre-commit verification for Phase 2.

**Commands (run in order, stop on first failure):**

1. `bash scripts/smoke-serial.sh` — 10× `cargo test --workspace --all-targets --locked -- --test-threads=8`. Must complete with `"smoke-serial: 10 / 10 iterations passed."`. If any iteration fails, the whole script exits non-zero; investigate the failure before proceeding.
2. `cargo fmt --all --check` — no format drift.
3. `cargo test --workspace --all-targets --locked` — including the new `env_discipline` gate integration test.
4. `cargo test --workspace --doc --locked` — doc tests unchanged.
5. `cargo clippy --workspace --all-targets --locked -- -D warnings` — pedantic bar.
6. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` — doc-build bar.

**Mutation probe (one-shot, non-persistent):**

7. Temporarily add `unsafe { std::env::set_var("PHASE2_PROBE", "1"); }` inside `auto_with_tty_and_no_env_is_ansi` in `crates/base60-cli/src/main.rs`, then REMOVE the `#[serial(env)]` attribute from that test. Run `cargo test --package xtask --test env_discipline --locked`. Confirm it fails with a diagnostic naming `crates/base60-cli/src/main.rs` and the line number of the probe. Then revert the probe (re-add `#[serial(env)]`, delete the extra `set_var` line). Re-run step 3 to confirm the tree is green again. Document the probe output in the plan SUMMARY — this is the positive evidence that the gate fires correctly.

**Do NOT commit at this point.** D-16 specifies a SINGLE commit for the entire phase:
```
test(cli,core): adopt #[serial(env)] for env-touching tests [TEST-04]
```
The commit should span:
- `Cargo.toml` (workspace members)
- `crates/base60-cli/Cargo.toml` (dev-dep)
- `crates/base60-core/Cargo.toml` (dev-dep)
- `crates/base60-cli/src/main.rs` (5 annotations + `use`)
- `crates/base60-core/src/cuneiform.rs` (1 annotation + `use`)
- `crates/base60-core/src/lens.rs` (1 annotation + `use`)
- `crates/xtask/Cargo.toml`
- `crates/xtask/src/lib.rs`
- `crates/xtask/tests/env_discipline.rs`
- `scripts/smoke-serial.sh`
- `.github/workflows/ci.yml`
- `Cargo.lock` (auto-updated by cargo when serial_test and walkdir land)

The orchestrator / execute-phase runner handles staging + committing all of these together. Do NOT commit from inside this task.
  </action>
  <verify>
    <automated>bash scripts/smoke-serial.sh &amp;&amp; cargo fmt --all --check &amp;&amp; cargo test --workspace --all-targets --locked &amp;&amp; cargo test --workspace --doc --locked &amp;&amp; cargo clippy --workspace --all-targets --locked -- -D warnings &amp;&amp; RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked    </automated>
  </verify>
  <acceptance_criteria>
    - `bash scripts/smoke-serial.sh` exits 0 and logs `smoke-serial: 10 / 10 iterations passed.`
    - `cargo fmt --all --check` exits 0.
    - `cargo test --workspace --all-targets --locked` exits 0; the `every_env_mutation_is_serialised` test inside the `xtask` package reports a single pass.
    - `cargo test --workspace --doc --locked` exits 0.
    - `cargo clippy --workspace --all-targets --locked -- -D warnings` exits 0.
    - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` exits 0.
    - Mutation probe (step 7) produces a failing `env_discipline` test whose diagnostic includes `crates/base60-cli/src/main.rs` and a line number; probe is reverted; tree is green again.
    - Working tree is ready for a single-commit landing per D-16 (`test(cli,core): adopt #[serial(env)] for env-touching tests [TEST-04]`); no intermediate commits exist for Phase 2.
  </acceptance_criteria>
  <done>Smoke passes 10/10; full workspace matrix is green; gate fires correctly under a mutation probe and passes on the reverted tree.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| future-developer → workspace | Source of invariant violations: a PR author who forgets `#[serial(env)]` or picks a per-variable key. |
| CI ubuntu-latest runner → workspace | The one-shot `--test-threads=8` step is the forever-running flake detector on every PR. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-02-09 | T (Tampering) | future PR reintroduces un-annotated env mutation | mitigate | `crates/xtask/tests/env_discipline.rs` fires on every `cargo test --workspace --all-targets --locked` run (every CI cell on every PR). Diagnostic names file + line. Negative-mutation probe in Task 4 proves the gate actually fires. |
| T-02-10 | T (Tampering) | future PR uses per-variable key (`#[serial(no_color)]`) | mitigate | `FORBIDDEN_SERIAL_KEYS` in the gate flags all three spellings per D-13. Test fails with the offending key named in the diagnostic. |
| T-02-11 | T (Tampering) | future PR adds `env::set_var` in production code | mitigate | Gate flags any env mutation whose enclosing fn lacks `#[test]`. Even legitimate production env mutations (there are none today) must be explicitly weighed and either lifted out of that code path or the gate's allowlist extended with a reviewed exception. |
| T-02-12 | R (Repudiation) | intermittent `NO_COLOR` / `NO_UNICODE` / `TERM` race flake | mitigate | `test-threads-8 (ubuntu)` CI step exercises the `#[serial(env)]` lock under pressure on every PR; 10× local smoke (`scripts/smoke-serial.sh`) is the phase-handoff gate. |
| T-02-13 | I (Info disclosure) | gate walks `crates/xtask/` and self-loops | mitigate | `WALK_ROOTS` explicitly lists only `../base60-core/src` and `../base60-cli/src`; `crates/xtask/tests/` is NOT a walk root (D-10). Acceptance criteria require this. |
| T-02-14 | D (Denial of service) | CI wall-clock blow-up from repeated `--test-threads=8` runs | accept | D-15 pins the CI step to `ubuntu-latest` (not the full matrix) and runs `--test-threads=8` ONCE per PR per Rust channel. Total added CI cost: ~2 minutes × 3 channels = 6 minutes. 10× loop is local only. |
| T-02-15 | S (Spoofing) | false-positive gate hit on `// SAFETY: mentions env::set_var` comments | mitigate | Walker's line-based parser skips any line whose trimmed form begins with `//`. D-06's SAFETY comments are preserved; they don't trip the gate. |

Phase closes TEST-04's full attack surface: the in-process env race is serialised, future regressions are gated, and the flake detector runs every PR.
</threat_model>

<verification>
After Tasks 1-4 complete:
- `crates/xtask/tests/env_discipline.rs` exists and is lint-clean (clippy, doc, fmt).
- `cargo test --package xtask --test env_discipline --locked` passes on the tree produced by Plans 01 + 02.
- `bash scripts/smoke-serial.sh` passes 10 / 10 iterations.
- `grep -q 'test-threads-8 (ubuntu)' .github/workflows/ci.yml` succeeds; YAML parses; step positioned correctly.
- Negative mutation probe (Task 4 step 7) demonstrates the gate fires with file:line diagnostics, and the tree returns to green after the probe is reverted.
- Working tree is staged for the single phase commit; no intermediate commits are expected.
</verification>

<success_criteria>
- Gate integration test `every_env_mutation_is_serialised` passes on the current tree and is wired into `cargo test --workspace --all-targets --locked`.
- Gate fails with a precise file:line diagnostic on three distinct violation shapes: missing attribute, forbidden key, env mutation in non-test function.
- `scripts/smoke-serial.sh` runs 10 iterations of `--test-threads=8`; exits 0 on success, exits non-zero on first failure.
- `.github/workflows/ci.yml` has exactly one new step `test-threads-8 (ubuntu)`, gated on `matrix.os == 'ubuntu-latest'`, placed after the existing `Unit + integration tests` step and before the `Doc tests` step; no other changes.
- Full local matrix (fmt / check / test / doc / clippy) passes.
- Phase 2 artifacts are ready for the single D-16 commit.
</success_criteria>

<output>
After completion, create `.planning/phases/02-env-test-serialisation/02-03-env-discipline-gate-SUMMARY.md` documenting:
- The final shape of `crates/xtask/tests/env_discipline.rs` (file size + line count; summary of walker shape).
- `scripts/smoke-serial.sh` content + executable-bit confirmation.
- Exact diff of `.github/workflows/ci.yml` (the one inserted step).
- Output of `bash scripts/smoke-serial.sh` (10/10 iterations).
- Output of the mutation probe run (the failing diagnostic from the intentionally-broken tree) + confirmation that the probe was reverted.
- Final `cargo test --workspace --all-targets --locked` summary (test count, all green).
- Total source line count of files added in this plan (for SUMMARY digest).
</output>
