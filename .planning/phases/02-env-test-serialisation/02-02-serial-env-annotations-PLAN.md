---
phase: 02-env-test-serialisation
plan: 02
type: execute
wave: 2
depends_on: [02-01]
files_modified:
  - crates/base60-cli/src/main.rs
  - crates/base60-core/src/cuneiform.rs
  - crates/base60-core/src/lens.rs
autonomous: true
requirements: [TEST-04]

must_haves:
  truths:
    - "Every existing env-mutating test bears a `#[serial(env)]` attribute."
    - "All 7 sites use the exact key `env` (no `no_color`, `no_unicode`, `term` variants — Pitfall 1)."
    - "Existing `SAFETY:` comments above `unsafe { env::… }` blocks are unchanged."
    - "`cargo test --workspace --all-targets --locked` continues to pass on every existing matrix cell."
  artifacts:
    - path: "crates/base60-cli/src/main.rs"
      provides: "5 `#[serial(env)]` attributes on env-touching tests in the `tests` module"
      contains: "#[serial(env)]"
    - path: "crates/base60-core/src/cuneiform.rs"
      provides: "1 `#[serial(env)]` attribute on `fallback_detection_respects_no_unicode_env`"
      contains: "#[serial(env)]"
    - path: "crates/base60-core/src/lens.rs"
      provides: "1 `#[serial(env)]` attribute on `cuneiform_auto_respects_no_unicode_env`"
      contains: "#[serial(env)]"
  key_links:
    - from: "crates/base60-cli/src/main.rs tests module"
      to: "serial_test crate"
      via: "use serial_test::serial;"
      pattern: 'use serial_test::serial'
    - from: "crates/base60-core/src/cuneiform.rs tests module"
      to: "serial_test crate"
      via: "use serial_test::serial;"
      pattern: 'use serial_test::serial'
    - from: "crates/base60-core/src/lens.rs tests module"
      to: "serial_test crate"
      via: "use serial_test::serial;"
      pattern: 'use serial_test::serial'
---

<objective>
Annotate every existing env-mutating test with `#[serial(env)]` using the exact shared-key form. This closes the race window that triggers intermittent CI flakes (Pitfall 1) and satisfies TEST-04 Success Criterion 1.

Purpose: single shared key `env` means all 7 tests mutex-serialise against each other, regardless of which env var they touch — `NO_COLOR`, `NO_UNICODE`, `TERM` races cannot interleave.

Output: 7 `#[serial(env)]` attributes across 3 source files, plus a `use serial_test::serial;` line inside each affected `#[cfg(test)] mod tests` block.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@.planning/phases/02-env-test-serialisation/02-CONTEXT.md
@.planning/research/PITFALLS.md
@.planning/codebase/CONVENTIONS.md
@.planning/phases/02-env-test-serialisation/02-01-workspace-prep-SUMMARY.md

<interfaces>
<!-- Verified exact line positions from the current source. Tasks rewrite the attribute block directly above each #[test], not the test body. -->

From crates/base60-cli/src/main.rs:
```
173: #[cfg(test)]
174: mod tests {
...   // (use statements, helpers)
184:     #[test]
185:     fn auto_with_tty_and_no_env_is_ansi() {
...
196:     #[test]
197:     fn auto_with_no_tty_is_mono() {
...
203:     #[test]
204:     fn auto_with_no_color_env_is_mono() {
...
212:     #[test]
213:     fn always_forces_ansi_even_without_tty() {
...
217:     #[test]
218:     fn never_forces_mono_even_with_tty() {
```
All 5 test fns are inside the single `#[cfg(test)] mod tests` block. Only env-touching tests get the attribute; the module-level imports section inside that block must gain `use serial_test::serial;`.

From crates/base60-core/src/cuneiform.rs:
```
150:     #[test]
151:     fn fallback_detection_respects_no_unicode_env() {
```

From crates/base60-core/src/lens.rs:
```
321:     #[test]
322:     fn cuneiform_auto_respects_no_unicode_env() {
```

`serial_test` attribute macro signature (from serial_test 3.x docs):
```rust
#[serial(env)]           // EXACT — no quotes, no string literal, single identifier key
#[test]
fn my_test() { ... }
```
The attribute accepts an ident, not a string. Do NOT write `#[serial("env")]` — that's a different form and may not compile. D-13 explicitly rejects `#[serial(no_color)]`, `#[serial(no_unicode)]`, `#[serial(term)]`.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Annotate 5 env-mutating tests in base60-cli/src/main.rs</name>
  <files>crates/base60-cli/src/main.rs</files>
  <read_first>
    - crates/base60-cli/src/main.rs lines 173-220 (the full `#[cfg(test)] mod tests` block)
    - .planning/phases/02-env-test-serialisation/02-CONTEXT.md §D-04..D-06 (per-test style, exact sites, SAFETY preservation rules)
    - .planning/research/PITFALLS.md §"Pitfall 1" (why single `env` key is mandatory)
  </read_first>
  <action>
Add `#[serial(env)]` directly above each of the 5 existing `#[test]` attributes listed in D-05, plus one `use serial_test::serial;` import inside the tests module.

**Step 1** — Add the import. Inside the `#[cfg(test)] mod tests { ... }` block (starts at line 173), after `use super::*;` (or whatever the first `use` line is inside the module — check file for exact first line after `mod tests {`), add on its own line:
```rust
    use serial_test::serial;
```
Indentation: 4 spaces (matches the existing `use super::*;` line). Place it grouped with other crate-external `use` lines if any; otherwise immediately after `use super::*;`.

**Step 2** — For each of these 5 test functions, insert `#[serial(env)]` on its own line BETWEEN the existing `#[test]` line and the `fn …` line:

| Line (current) | Function name |
|----------------|---------------|
| 184–185 | `auto_with_tty_and_no_env_is_ansi` |
| 196–197 | `auto_with_no_tty_is_mono` |
| 203–204 | `auto_with_no_color_env_is_mono` |
| 212–213 | `always_forces_ansi_even_without_tty` |
| 217–218 | `never_forces_mono_even_with_tty` |

Exact shape after edit (example for `auto_with_tty_and_no_env_is_ansi`):
```rust
    #[test]
    #[serial(env)]
    fn auto_with_tty_and_no_env_is_ansi() {
```
Order (attribute rules): `#[test]` first, `#[serial(env)]` second. (serial_test docs explicitly require `#[serial]` to be placed AFTER `#[test]` for `cargo test`; reverse order changes discovery behaviour.)

Indentation: 4 spaces (matches the existing `#[test]` lines).

**Step 3** — LEAVE UNCHANGED:
- Every `// SAFETY:` comment above `unsafe { env::set_var(...) }` and `unsafe { env::remove_var(...) }` (D-06).
- Every test body, every `unsafe { … }` block, every other test in the module.
- Any test in the module that does NOT mutate env vars (none exist here per D-05, but double-check: `grep -n 'env::set_var\|env::remove_var' crates/base60-cli/src/main.rs` should return hits only inside the 5 listed functions).
- Any existing `#[cfg(...)]` attributes on other items.

**Step 4** — Under no circumstances use a per-variable key. Do NOT write `#[serial(no_color)]`, `#[serial(no_unicode)]`, `#[serial(term)]`, or any other key. The gate in Plan 03 will reject those exact spellings (D-13). Single shared key `env` is the only accepted form (Pitfall 1, TEST-04 Success Criterion 4).
  </action>
  <verify>
    <automated>test "$(grep -c '#\[serial(env)\]' crates/base60-cli/src/main.rs)" = "5" &amp;&amp; grep -q '    use serial_test::serial;' crates/base60-cli/src/main.rs &amp;&amp; ! grep -E '#\[serial\((no_color|no_unicode|term)\)\]' crates/base60-cli/src/main.rs &amp;&amp; grep -q '// SAFETY:' crates/base60-cli/src/main.rs &amp;&amp; cargo test --locked -p base60 --lib -- auto_with_tty_and_no_env_is_ansi auto_with_no_tty_is_mono auto_with_no_color_env_is_mono always_forces_ansi_even_without_tty never_forces_mono_even_with_tty</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c '#\[serial(env)\]' crates/base60-cli/src/main.rs` returns exactly `5`.
    - `crates/base60-cli/src/main.rs` contains `use serial_test::serial;` inside the `#[cfg(test)] mod tests` block (indented 4 spaces).
    - For each of the 5 function names, `grep -B1 'fn auto_with_tty_and_no_env_is_ansi\|fn auto_with_no_tty_is_mono\|fn auto_with_no_color_env_is_mono\|fn always_forces_ansi_even_without_tty\|fn never_forces_mono_even_with_tty' crates/base60-cli/src/main.rs` shows `#[serial(env)]` immediately above the `fn …` line.
    - `grep -E '#\[serial\((no_color|no_unicode|term|state_dir)\)\]' crates/base60-cli/src/main.rs` returns no matches (exit 1; inverted check exits 0).
    - `grep -c '// SAFETY:' crates/base60-cli/src/main.rs` is ≥ 4 (preserves all existing SAFETY comments).
    - All 5 named tests pass under `cargo test --locked -p base60 --lib -- <name>`.
    - No other test regressions in `cargo test --locked -p base60 --lib`.
  </acceptance_criteria>
  <done>5 `#[serial(env)]` annotations present in main.rs; `use serial_test::serial;` imported once; SAFETY comments preserved verbatim; tests pass.</done>
</task>

<task type="auto">
  <name>Task 2: Annotate env-mutating tests in base60-core (cuneiform.rs + lens.rs)</name>
  <files>crates/base60-core/src/cuneiform.rs, crates/base60-core/src/lens.rs</files>
  <read_first>
    - crates/base60-core/src/cuneiform.rs lines 140-165 (the `#[cfg(test)] mod tests` block around `fallback_detection_respects_no_unicode_env`)
    - crates/base60-core/src/lens.rs lines 310-335 (the `#[cfg(test)] mod tests` block around `cuneiform_auto_respects_no_unicode_env`)
    - .planning/phases/02-env-test-serialisation/02-CONTEXT.md §D-04..D-06
  </read_first>
  <action>
Add `#[serial(env)]` to the 2 env-mutating tests in `base60-core`, plus a `use serial_test::serial;` import inside each affected tests module.

**Step 1 — `crates/base60-core/src/cuneiform.rs`:**

Inside the `#[cfg(test)] mod tests { ... }` block, after the existing `use super::*;` line, add:
```rust
    use serial_test::serial;
```
(4-space indent; match the existing `use super::*;` indent exactly.)

Between the existing `#[test]` on line 150 and `fn fallback_detection_respects_no_unicode_env` on line 151, insert a new line:
```rust
    #[serial(env)]
```
Result:
```rust
    #[test]
    #[serial(env)]
    fn fallback_detection_respects_no_unicode_env() {
```

**Step 2 — `crates/base60-core/src/lens.rs`:**

Inside the `#[cfg(test)] mod tests { ... }` block, after the existing `use super::*;` line (or similar), add:
```rust
    use serial_test::serial;
```
(4-space indent.)

Between the existing `#[test]` on line 321 and `fn cuneiform_auto_respects_no_unicode_env` on line 322, insert:
```rust
    #[serial(env)]
```
Result:
```rust
    #[test]
    #[serial(env)]
    fn cuneiform_auto_respects_no_unicode_env() {
```

**Step 3 — preservation rules (identical to Task 1):**
- Do NOT touch `// SAFETY:` comments above `unsafe { env::set_var(...) }` / `unsafe { env::remove_var(...) }` (D-06).
- Do NOT annotate any other test in either file. `cuneiform.rs` has 9 total tests, `lens.rs` has 15 total tests — only one test per file mutates env vars (D-05), and only that one gets the attribute.
- Do NOT introduce any per-variable key spelling (Pitfall 1; D-13).
- Do NOT reorder existing `use` statements or attributes.

**Step 4 — verification aid:** After edits, `grep -n '#\[serial' crates/base60-core/src/` must show exactly 2 lines (one per file). If it shows more or fewer, stop and review — you either duplicated the attribute, annotated a non-env test, or missed a site.
  </action>
  <verify>
    <automated>test "$(grep -c '#\[serial(env)\]' crates/base60-core/src/cuneiform.rs)" = "1" &amp;&amp; test "$(grep -c '#\[serial(env)\]' crates/base60-core/src/lens.rs)" = "1" &amp;&amp; grep -q '    use serial_test::serial;' crates/base60-core/src/cuneiform.rs &amp;&amp; grep -q '    use serial_test::serial;' crates/base60-core/src/lens.rs &amp;&amp; ! grep -rE '#\[serial\((no_color|no_unicode|term|state_dir)\)\]' crates/base60-core/src/ &amp;&amp; cargo test --locked -p base60-core --lib -- fallback_detection_respects_no_unicode_env cuneiform_auto_respects_no_unicode_env</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c '#\[serial(env)\]' crates/base60-core/src/cuneiform.rs` returns exactly `1`.
    - `grep -c '#\[serial(env)\]' crates/base60-core/src/lens.rs` returns exactly `1`.
    - Both files contain `use serial_test::serial;` inside their respective `#[cfg(test)] mod tests` block (indented 4 spaces).
    - `grep -B1 'fn fallback_detection_respects_no_unicode_env' crates/base60-core/src/cuneiform.rs` shows `#[serial(env)]` on the line above `fn`.
    - `grep -B1 'fn cuneiform_auto_respects_no_unicode_env' crates/base60-core/src/lens.rs` shows `#[serial(env)]` on the line above `fn`.
    - `grep -rE '#\[serial\((no_color|no_unicode|term|state_dir)\)\]' crates/base60-core/src/` returns no matches.
    - `grep -c '// SAFETY:' crates/base60-core/src/cuneiform.rs` is unchanged from pre-edit count (≥ 2).
    - `grep -c '// SAFETY:' crates/base60-core/src/lens.rs` is unchanged from pre-edit count (≥ 2).
    - Both named tests pass under `cargo test --locked -p base60-core --lib -- <name>`.
  </acceptance_criteria>
  <done>2 `#[serial(env)]` annotations across cuneiform.rs + lens.rs; both files import `serial_test::serial`; SAFETY comments verbatim; tests pass.</done>
</task>

<task type="auto">
  <name>Task 3: Full-workspace test + clippy sweep</name>
  <files>(verification only — no files edited)</files>
  <read_first>
    - ./CLAUDE.md (workspace lints — pedantic + nursery + cargo with -D warnings)
    - .planning/codebase/TESTING.md (run commands)
  </read_first>
  <action>
Run the full test + lint matrix locally to confirm annotations compile cleanly and haven't regressed any existing test or lint. This is the local analogue of the CI matrix — it catches MSRV / clippy issues BEFORE the gate (Plan 03) or CI sees them.

**Commands to run (in order, stop on first failure):**

1. `cargo fmt --all --check` — annotations shouldn't change formatting; if rustfmt wants to reflow attribute blocks, accept its decision (`cargo fmt --all` to apply, then re-run `--check`).
2. `cargo check --workspace --all-targets --locked` — confirms the annotated tests compile; `serial_test` proc macro expands cleanly.
3. `cargo test --workspace --all-targets --locked` — full test suite; all 164 existing tests plus the 7 annotated tests must pass.
4. `cargo test --workspace --doc --locked` — doc tests (separate target; untouched by this phase but worth confirming).
5. `cargo clippy --workspace --all-targets --locked -- -D warnings` — pedantic + nursery + cargo bar. The `serial_test` macro occasionally triggers `clippy::needless_pass_by_value` on expansion; if so, check whether the warning is inside the expanded macro (acceptable — not our code) or in user code (fix). Run with `--verbose` if a specific warning is ambiguous.

**Expected observations:**
- No formatting changes required.
- No new warnings on `main.rs`, `cuneiform.rs`, `lens.rs`.
- All 7 env-annotated tests + all 164 existing tests pass.
- The `serial_test` macro expansion may surface as `clippy::multiple_crate_versions` if `serial_test` pulls in a `thiserror` version we already have — that lint is workspace-`allow`ed (CLAUDE.md), so no action needed.

**If clippy fails:** read the exact diagnostic. Common cases:
- `unused_import` on `use serial_test::serial;` if no `#[serial(env)]` attribute in scope → you forgot to annotate at least one test in that file. Fix Task 1 / Task 2.
- `missing_docs_in_private_items` → does not apply to `#[cfg(test)] mod tests` (test-only code is exempt). If it fires, add `#[allow(missing_docs)]` on the module.
- Anything else → read the full error; fix the code, not the lint.

**Do NOT commit at this point** — the commit is one-shot (D-16) and happens after Plan 03 completes.
  </action>
  <verify>
    <automated>cargo fmt --all --check &amp;&amp; cargo check --workspace --all-targets --locked &amp;&amp; cargo test --workspace --all-targets --locked &amp;&amp; cargo test --workspace --doc --locked &amp;&amp; cargo clippy --workspace --all-targets --locked -- -D warnings</automated>
  </verify>
  <acceptance_criteria>
    - `cargo fmt --all --check` exits 0.
    - `cargo check --workspace --all-targets --locked` exits 0.
    - `cargo test --workspace --all-targets --locked` exits 0 and reports all tests passing (expect 171 total: 164 previous + 7 annotated = same tests, not new).
    - `cargo test --workspace --doc --locked` exits 0.
    - `cargo clippy --workspace --all-targets --locked -- -D warnings` exits 0.
    - No files in the git working tree have been modified by fmt (if fmt modifies files, re-run from step 1 after `git diff` review).
  </acceptance_criteria>
  <done>Workspace is fmt-clean, check-clean, test-clean, doc-clean, and clippy-clean with the 7 `#[serial(env)]` annotations in place.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| test thread → process env | Rust 2024 `env::set_var` / `env::remove_var` is `unsafe` because env mutation races with any concurrent reader. `#[serial(env)]` converts this from "hope the scheduler cooperates" to "serialised by in-process mutex". |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-02-05 | T (Tampering) | concurrent env read/write across test threads | mitigate | All 7 env-mutating tests bear `#[serial(env)]` — exact key `env`, shared across all 7 sites. Single-mutex serialisation (serial_test 3.x in-process lock) eliminates the race window. Task verification enforces the single-key rule. |
| T-02-06 | T (Tampering) | per-variable key drift (`#[serial(no_color)]`) | mitigate | Tasks 1+2 explicitly forbid alternate keys in action text. Acceptance criteria include a grep that must return 0 for `no_color|no_unicode|term|state_dir`. Plan 03's gate will enforce this automatically. |
| T-02-07 | I (Info disclosure) | SAFETY comments stripped during edit | mitigate | Tasks 1+2 call out SAFETY preservation explicitly; acceptance criteria assert SAFETY-comment count is unchanged. These comments document the Rust-2024 unsafe-env rule, which `serial_test` does not replace (D-06). |
| T-02-08 | R (Repudiation) | silent test regression | mitigate | Task 3 runs the full workspace test matrix before the one-shot commit. Any regression surfaces before the gate lands. |

No new attack surface in the shipping binary (annotations are `#[cfg(test)]`-gated). All dispositions are "mitigate" via explicit task-level verification — no "accept" threats because the mitigations are cheap and mechanical.
</threat_model>

<verification>
After Tasks 1-3:
- `grep -rc '#\[serial(env)\]' crates/base60-cli/src/ crates/base60-core/src/` returns a combined total of exactly 7 (5+1+1).
- `grep -rE '#\[serial\((no_color|no_unicode|term|state_dir)\)\]' crates/` returns no matches.
- `grep -rc 'use serial_test::serial' crates/base60-cli/src/ crates/base60-core/src/` returns ≥ 3 (one per affected file).
- `cargo test --workspace --all-targets --locked` passes.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passes.
- `grep -rn 'env::set_var\|env::remove_var' crates/base60-cli/src/ crates/base60-core/src/` returns only hits that are inside functions bearing `#[serial(env)]` (the gate in Plan 03 will verify this automatically; the human reviewer should also eyeball-check).
</verification>

<success_criteria>
- Every env-mutating test across the workspace bears `#[serial(env)]` with the exact key spelling `env`.
- Zero per-variable `#[serial(no_color)]` / `#[serial(no_unicode)]` / `#[serial(term)]` sites exist (Pitfall 1 closed).
- SAFETY comments above `unsafe { env::… }` blocks are preserved verbatim (D-06).
- Workspace passes `cargo test --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- All 7 `#[serial(env)]` attributes are placed AFTER `#[test]` (serial_test attribute-order requirement).
</success_criteria>

<output>
After completion, create `.planning/phases/02-env-test-serialisation/02-02-serial-env-annotations-SUMMARY.md` documenting:
- The 7 annotated sites (function name → file:line).
- Clippy output summary (must be clean).
- `cargo test --workspace --all-targets --locked` output (test count, pass/fail).
- Confirmation that `// SAFETY:` comment counts match pre-edit values.
- Confirmation that no per-variable key spelling appears anywhere.
</output>
