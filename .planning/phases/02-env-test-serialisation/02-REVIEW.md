---
phase: 02-env-test-serialisation
reviewed: 2026-04-24T09:04:10Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - .github/workflows/ci.yml
  - Cargo.toml
  - crates/base60-cli/Cargo.toml
  - crates/base60-cli/src/main.rs
  - crates/base60-core/Cargo.toml
  - crates/base60-core/src/cuneiform.rs
  - crates/base60-core/src/lens.rs
  - crates/xtask/Cargo.toml
  - crates/xtask/src/lib.rs
  - crates/xtask/tests/env_discipline.rs
  - scripts/smoke-serial.sh
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
status: issues_found
---

# Phase 2: Code Review Report

**Reviewed:** 2026-04-24T09:04:10Z
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Phase 2 delivers `#[serial(env)]` annotations on the seven env-mutating tests
across `base60-core` and `base60-cli`, plus a workspace-level invariant gate
(`crates/xtask/tests/env_discipline.rs`) and a local smoke script
(`scripts/smoke-serial.sh`). The gate runs under `cargo test` and currently
passes; clippy (`pedantic+nursery+cargo`, `-D warnings`) is clean on 1.95.0.

The Rust changes are small, idiomatic, and correctly annotated. Every
`env::set_var` / `env::remove_var` call site is wrapped in `unsafe { ... }`
with a SAFETY comment, sits under both `#[test]` and `#[serial(env)]`, and
uses the single shared `env` key across all three modules — matching
D-13 ("single key across all env-mutating tests") and D-12
("every env-mutating test gets `#[serial(env)]`").

Findings below are concentrated in the invariant gate itself: line-based
parsing has a handful of escape hatches that a future contributor could
trip over (attribute alias, `unsafe fn` declaration, trailing-comment
false-positive, nested-fn closure). None are blocking for Phase 2 as
written — every current call site is covered — but each weakens the gate's
"catches future regressions" guarantee and deserves either a code fix or
a documented limitation. CI and the smoke script have minor hardening
opportunities but no correctness bugs.

## Warnings

### WR-01: Gate accepts trailing-comment mentions of `env::set_var(` as real mutations

**File:** `crates/xtask/tests/env_discipline.rs:55-58`
**Issue:** The comment-skip only handles lines whose *trimmed start* begins
with `//`:

```rust
let trimmed = line.trim_start();
if trimmed.starts_with("//") {
    continue;
}
```

A perfectly innocent line like

```rust
let flag = is_set;                 // env::set_var(...) happens elsewhere
```

contains `env::set_var(` in a comment but is classified as real code. The
gate would then require an enclosing `#[test]` + `#[serial(env)]`, producing
a spurious failure. Block comments (`/* ... env::set_var( ... */`) have the
same exposure. The current tree doesn't hit this, but it's a plausible
future footgun: the SAFETY commentary in `main.rs:188-193` already
discusses "`env::remove_var`" in prose, and a reviewer adding "e.g.
`env::set_var(\"FOO\", \"bar\")`" to a future doc line would break CI.

**Fix:** Strip inline comments before the match, not just line-prefix:

```rust
// Strip anything after the first `//` outside a string. This is a
// coarse approximation (doesn't handle `//` inside a string literal),
// but it is sufficient for the single-line patterns we scan and avoids
// false positives on SAFETY/example comments.
let code = line.split_once("//").map_or(line, |(head, _)| head);
let mentions_mutation =
    code.contains("env::set_var(") || code.contains("env::remove_var(");
```

Or, if stricter handling is preferred, switch to a `syn`-based visitor (adds
one dev-dependency but eliminates the whole class of line-parser bugs).

### WR-02: `find_enclosing_fn` omits `unsafe fn`, `extern fn`, and `pub(in ...) fn`

**File:** `crates/xtask/tests/env_discipline.rs:131-145`
**Issue:** The enclosing-fn detector hardcodes six prefixes:

```rust
t.starts_with("fn ")
    || t.starts_with("pub fn ")
    || t.starts_with("pub(crate) fn ")
    || t.starts_with("pub(super) fn ")
    || t.starts_with("async fn ")
    || t.starts_with("const fn ")
```

Valid Rust fn declarations it does not match:

- `unsafe fn foo(...)` and `pub unsafe fn foo(...)`
- `extern "C" fn foo(...)` / `unsafe extern "C" fn foo(...)`
- `pub(in crate::mod) fn foo(...)`
- combinations: `pub(crate) async fn`, `pub(crate) unsafe fn`,
  `async unsafe fn`, `const unsafe fn`

If any future test is written as `unsafe fn` (plausible if someone wraps
the `unsafe { env::set_var(...) }` block into an `unsafe fn helper`), the
walker keeps scanning upward past the real enclosing fn and either returns
`None` (false-positive "no enclosing fn") or lands on the wrong fn (false
positive "missing `#[test]`"). The diagnostic would blame the wrong line.

**Fix:** Replace the prefix list with a regex or a single substring check
that accepts "contains ` fn ` preceded only by keywords":

```rust
fn is_fn_decl(line: &str) -> bool {
    // Allow any combination of `pub[(...)]?`, `async`, `unsafe`,
    // `const`, `extern "..."` keywords before `fn `.
    let t = line.trim_start();
    // Fast path: must contain `fn ` followed by an identifier.
    let Some(idx) = t.find("fn ") else { return false };
    // Everything before `fn ` must be keywords / visibility only.
    let prefix = &t[..idx];
    prefix
        .split_whitespace()
        .all(|tok| matches!(
            tok,
            "pub" | "async" | "unsafe" | "const" | "extern"
        ) || tok.starts_with("pub(") || tok.starts_with('"'))
}
```

Document the limitation either way — the "`fn` prefix list" approach is
fragile enough to warrant a comment listing what is *not* caught.

### WR-03: Gate is bypassable via `use std::env::set_var as …;`

**File:** `crates/xtask/tests/env_discipline.rs:60-62`
**Issue:** The match is literal:

```rust
let mentions_mutation =
    line.contains("env::set_var(") || line.contains("env::remove_var(");
```

Any of the following patterns silently bypasses the gate:

- `use std::env::set_var as sv; unsafe { sv("X", "y") };`
- `use std::env::{self, set_var}; unsafe { set_var(...) };`
- `use std::env::set_var; unsafe { set_var(...) };` (bare call)
- macro helpers: `env_helper!(set "X", "y")` that expand to a mutation

Phase 2 D-13 states "single key across all env-mutating tests", but the
gate can only enforce that invariant on call shapes it recognises. Today
every call site spells `std::env::set_var(`, so the bypass is theoretical —
but the whole point of the gate is to catch *future* drift.

**Fix:** Either (a) forbid the aliasing import in the same walker
(`line.contains("use std::env::set_var") || line.contains("use std::env::remove_var")`
→ always failure), or (b) switch to a `syn`-based visitor that resolves
identifiers after `use` normalisation. The cheap fix is (a); the
engineering-correct fix is (b). Document whichever is chosen.

## Info

### IN-01: Nested fn inside a `#[test]` produces a false positive

**File:** `crates/xtask/tests/env_discipline.rs:131-145`
**Issue:** `find_enclosing_fn` stops at the first `fn` walking upward.
A test that defines an inner helper would be misclassified:

```rust
#[test]
#[serial(env)]
fn outer() {
    fn inner() { unsafe { std::env::set_var("X", "y"); } }
    inner();
}
```

The walker finds `fn inner`, sees no `#[test]` attribute above it, and
fails. None of the current tests use this pattern, but it's a legitimate
Rust idiom the gate would reject.

**Fix:** Either (a) track brace depth while walking upward to find the
*outermost* fn that contains the mutation line (harder to get right with
line-based parsing), or (b) accept the limitation and document that env
mutations must be in the direct test body, not in nested helper fns. (b)
is the cheap call given serial_test itself has no way to "inherit" the
key anyway — a nested helper fn called from *two* `#[serial(env)]` tests
would be serialised correctly, but a helper called from a non-serial test
would not. The documentation is the real fix.

### IN-02: CI ubuntu matrix runs `cargo test` twice per Rust version

**File:** `.github/workflows/ci.yml:37-41`
**Issue:** The new `test-threads-8` step runs on `matrix.os == 'ubuntu-latest'`
unconditionally, which means it executes on all three Rust versions
(1.95, stable, beta). The preceding "Unit + integration tests" step has
already run the same tests with the default thread count on the same
matrix cell, so each ubuntu cell now pays ~2× test duration.

Given env-discipline is a per-process invariant (serial_test coordinates
within-process), one Rust version's confirmation would typically suffice.

**Fix (optional — not a correctness issue):**

```yaml
- name: test-threads-8 (ubuntu/stable)
  if: matrix.os == 'ubuntu-latest' && matrix.rust == 'stable'
  run: cargo test --workspace --all-targets --locked -- --test-threads=8
```

Or keep the current behaviour and document it as a deliberate belt-and-
braces check in `02-03-env-discipline-gate-SUMMARY.md`. Either is fine.

### IN-03: `smoke-serial.sh` lacks `IFS` hardening and working-directory guard

**File:** `scripts/smoke-serial.sh:12`
**Issue:** `set -euo pipefail` is present (good), but the script does not
pin `IFS` and does not anchor itself to the repo root. Running it from
an arbitrary cwd relies on `cargo`'s upward workspace discovery — which
works today but would break if invoked from, e.g., `/tmp`.

**Fix:**

```bash
set -euo pipefail
IFS=$'\n\t'

# Resolve repo root relative to this script so `cargo` picks up the
# expected workspace regardless of invocation cwd.
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}/.."
```

Nice-to-have; not blocking.

### IN-04: Gate has no negative test (mutation that *should* fail)

**File:** `crates/xtask/tests/env_discipline.rs`
**Issue:** The gate asserts "all current mutations comply" but has no
fixture demonstrating that a non-compliant file *would* fail. If the
regex or walker silently breaks (e.g., refactored into accepting the
empty string), the gate would pass on every future commit without
surfacing the regression.

**Fix:** Add a second test that synthesises an in-memory non-compliant
Rust snippet string and runs the same walker/attribute-check logic
against it, asserting the expected failure message. This is a light lift
if the walker is refactored into a pure `fn check(source: &str) ->
Vec<Failure>` — the existing `#[test]` then becomes the "walk the real
tree" wrapper. Purely defensive; not blocking.

---

_Reviewed: 2026-04-24T09:04:10Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
