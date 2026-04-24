---
phase: 02-env-test-serialisation
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - crates/base60-cli/Cargo.toml
  - crates/base60-core/Cargo.toml
  - crates/xtask/Cargo.toml
  - crates/xtask/src/lib.rs
autonomous: true
requirements: [TEST-04]

must_haves:
  truths:
    - "Workspace compiles (`cargo check --workspace --locked`) with xtask as a member."
    - "`serial_test` is available as a dev-dep in both base60-cli and base60-core."
    - "`walkdir` is available as a dev-dep in xtask."
    - "base60-core `[dependencies]` section remains empty (zero-dep invariant preserved; CI-03 contract intact)."
  artifacts:
    - path: "Cargo.toml"
      provides: "workspace members list including crates/xtask"
      contains: '"crates/xtask"'
    - path: "crates/base60-cli/Cargo.toml"
      provides: "serial_test dev-dep"
      contains: 'serial_test = { version = "3"'
    - path: "crates/base60-core/Cargo.toml"
      provides: "serial_test dev-dep"
      contains: 'serial_test = { version = "3"'
    - path: "crates/xtask/Cargo.toml"
      provides: "xtask package manifest + walkdir dev-dep"
      contains: 'name = "xtask"'
    - path: "crates/xtask/src/lib.rs"
      provides: "library root with crate-level doc comment"
      contains: "//!"
  key_links:
    - from: "Cargo.toml"
      to: "crates/xtask/Cargo.toml"
      via: "workspace members entry"
      pattern: 'crates/xtask'
    - from: "crates/xtask/Cargo.toml"
      to: "walkdir"
      via: "dev-dep"
      pattern: 'walkdir = "2"'
---

<objective>
Lay the dependency + workspace scaffolding that every downstream task in Phase 2 stands on.

Purpose: `#[serial(env)]` annotations (Plan 02) will not compile without the `serial_test` dev-dep; the gate test (Plan 03) will not compile without a `crates/xtask/` workspace member + `walkdir` dev-dep. Land all three wiring changes together so Plan 02 / Plan 03 start from a green workspace.

Output: `crates/xtask/` scaffold exists as a workspace member; `serial_test = { version = "3", default-features = false }` is a dev-dep on both `base60-cli` and `base60-core`; `cargo check --workspace --locked` passes.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@.planning/phases/02-env-test-serialisation/02-CONTEXT.md
@.planning/REQUIREMENTS.md
@.planning/ROADMAP.md

<interfaces>
<!-- Existing workspace manifest shape — new xtask entry must integrate cleanly. -->

From Cargo.toml (workspace root):
```toml
[workspace]
resolver = "3"
members = ["crates/base60-core", "crates/base60-cli"]
```
Target after Plan 01:
```toml
[workspace]
resolver = "3"
members = ["crates/base60-core", "crates/base60-cli", "crates/xtask"]
```

From crates/base60-cli/Cargo.toml (current):
```toml
[dependencies]
anyhow = "1.0.102"
clap = { version = "4.6.1", features = ["derive"] }
clap_complete = "4.6.1"
crossterm = "0.29.0"
memmap2 = "0.9.10"
ratatui = "0.30.0"
base60-core = { path = "../base60-core" }

[lints]
workspace = true
```
Note: no `[dev-dependencies]` section exists yet; must be added.

From crates/base60-core/Cargo.toml (current):
```toml
[lints]
workspace = true
```
Note: no `[dependencies]` and no `[dev-dependencies]` section — preserve zero-dep runtime posture (CI-03).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Scaffold xtask workspace member</name>
  <files>Cargo.toml, crates/xtask/Cargo.toml, crates/xtask/src/lib.rs</files>
  <read_first>
    - Cargo.toml (workspace root — to preserve field ordering)
    - crates/base60-core/Cargo.toml (reference for workspace-inherited fields)
    - ./CLAUDE.md (workspace lints, RUSTDOCFLAGS = -D warnings on cargo doc)
  </read_first>
  <action>
Create the new `crates/xtask/` crate as a bare test-only workspace member (per D-08, D-09).

**Step 1** — Create `crates/xtask/Cargo.toml` with this exact content:
```toml
[package]
name = "xtask"
description = "Workspace-level automation tasks (test gates, lints, housekeeping)."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish.workspace = true

[dev-dependencies]
walkdir = "2"

[lints]
workspace = true
```
Notes:
- `publish.workspace = true` inherits `publish = false` from workspace (D-09).
- No `[dependencies]` — the gate is entirely a test-only walker; `walkdir` lives in `[dev-dependencies]`.
- No `[[bin]]` — this is a library crate; the gate lives in `tests/env_discipline.rs` in Plan 03.
- `[lints] workspace = true` opts the new crate into pedantic + nursery + cargo + `-D warnings` (CLAUDE.md constraint).

**Step 2** — Create `crates/xtask/src/lib.rs` with this exact content:
```rust
//! Workspace-level automation helpers for base60.
//!
//! This crate hosts repo-wide invariant checks that run as integration
//! tests under `cargo test --workspace --all-targets --locked`. It has
//! no runtime code; all behaviour is in `tests/*.rs`.
```
Notes:
- Must start with `//!` (CONVENTIONS.md "Every file starts with `//!`" + `RUSTDOCFLAGS: -D warnings` enforcement).
- Empty code body: no `pub` items yet — Plan 03 adds the gate as an integration test, not as library code (D-08: "bare test-only crate — empty `src/lib.rs` + `tests/env_discipline.rs`").
- Do NOT add `pub fn` / `pub(crate) fn` items here; they would trigger `missing_docs_in_private_items` / `must_use_candidate` lints and aren't needed.

**Step 3** — Edit workspace root `Cargo.toml`: change the `members` line from
```toml
members = ["crates/base60-core", "crates/base60-cli"]
```
to
```toml
members = ["crates/base60-core", "crates/base60-cli", "crates/xtask"]
```
Do not reorder the existing entries. Preserve every other line in `Cargo.toml` verbatim (profile, workspace.package, workspace.lints).
  </action>
  <verify>
    <automated>cargo check --workspace --locked 2>&amp;1 | tee /tmp/p02-01-check.log; grep -q 'error' /tmp/p02-01-check.log &amp;&amp; exit 1 || true; cargo metadata --format-version 1 --manifest-path Cargo.toml --no-deps | grep -q '"name":"xtask"' &amp;&amp; cargo metadata --format-version 1 --manifest-path crates/base60-core/Cargo.toml --no-deps | python3 -c 'import json,sys;d=json.load(sys.stdin);assert d["packages"][0]["dependencies"]==[] or all(x.get("kind")=="dev" for x in d["packages"][0]["dependencies"]),"base60-core has runtime deps"'</automated>
  </verify>
  <acceptance_criteria>
    - `cargo check --workspace --locked` exits 0 (all three crates compile).
    - `cargo metadata --format-version 1 --no-deps` lists `"name":"xtask"` in `packages[]`.
    - `crates/xtask/Cargo.toml` exists and contains `name = "xtask"`, `publish.workspace = true`, `[dev-dependencies] walkdir = "2"`, `[lints] workspace = true`.
    - `crates/xtask/src/lib.rs` exists and its first line starts with `//!`.
    - `Cargo.toml` `members` line contains exactly `"crates/base60-core", "crates/base60-cli", "crates/xtask"` in that order.
    - `grep -c '\[dependencies\]' crates/base60-core/Cargo.toml` returns `0` (zero-dep runtime posture intact).
  </acceptance_criteria>
  <done>Workspace compiles with three members; xtask crate exists as a bare library-only workspace member; no runtime deps added to base60-core.</done>
</task>

<task type="auto">
  <name>Task 2: Add serial_test dev-dep to cli and core</name>
  <files>crates/base60-cli/Cargo.toml, crates/base60-core/Cargo.toml</files>
  <read_first>
    - crates/base60-cli/Cargo.toml (current content — to preserve existing sections)
    - crates/base60-core/Cargo.toml (current content — to preserve the zero-`[dependencies]` shape)
    - .planning/phases/02-env-test-serialisation/02-CONTEXT.md §D-01..D-03 (exact dep line wording, `default-features = false` rationale)
  </read_first>
  <action>
Add `serial_test = { version = "3", default-features = false }` as a dev-dep to BOTH crates (per D-01, D-02, D-03).

**Step 1** — Edit `crates/base60-cli/Cargo.toml`: append a new section after the existing `[dependencies]` block and before `[lints]`:
```toml
[dev-dependencies]
serial_test = { version = "3", default-features = false }
```
Exact line. `default-features = false` is required (D-03): disables `async`, `file_locks`, `logging` features we don't use. Do NOT add any other dev-deps.

**Step 2** — Edit `crates/base60-core/Cargo.toml`: insert a new section between `publish.workspace = true` and `[lints]`:
```toml
[dev-dependencies]
serial_test = { version = "3", default-features = false }
```
Same exact line. D-02 confirms dev-deps are permitted in core (CI-03 checks `[dependencies]` only). Do NOT add anything to `[dependencies]` — that section must stay absent.

**Step 3** — Verify `Cargo.lock` updates cleanly: run `cargo check --workspace --locked` once; if the lock is out-of-sync, run `cargo update -p serial_test --precise <version>` is NOT needed — the initial lockfile update happens via `cargo fetch`. If `--locked` fails because the lock is stale, run `cargo check --workspace` (without `--locked`) once to update `Cargo.lock`, then re-run with `--locked` to confirm.
  </action>
  <verify>
    <automated>grep -q 'serial_test = { version = "3", default-features = false }' crates/base60-cli/Cargo.toml &amp;&amp; grep -q 'serial_test = { version = "3", default-features = false }' crates/base60-core/Cargo.toml &amp;&amp; ! grep -A5 '^\[dependencies\]' crates/base60-core/Cargo.toml | grep -v '^\[dependencies\]$' | grep -q '[a-z]' ; cargo check --workspace --locked</automated>
  </verify>
  <acceptance_criteria>
    - `crates/base60-cli/Cargo.toml` contains exact line `serial_test = { version = "3", default-features = false }` under a `[dev-dependencies]` section.
    - `crates/base60-core/Cargo.toml` contains the same exact line under a `[dev-dependencies]` section.
    - Neither `Cargo.toml` contains `serial_test` under `[dependencies]`.
    - `crates/base60-core/Cargo.toml` has NO non-empty `[dependencies]` section (CI-03 zero-dep invariant).
    - `cargo check --workspace --locked` exits 0.
    - `cargo metadata --format-version 1 --manifest-path crates/base60-core/Cargo.toml` shows at least one entry with `"kind":"dev"` and `"name":"serial_test"`, and zero entries with `"kind":null` (null = runtime dep).
  </acceptance_criteria>
  <done>`serial_test = "3"` is a dev-dep on both crates with `default-features = false`; Cargo.lock is consistent; base60-core runtime dep count stays zero.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| crates.io → workspace | Dev-dep pulls in `serial_test` + `walkdir` transitives into the test-build dependency graph |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-02-01 | T (Tampering) | `serial_test = "3"` transitive supply chain | accept | Dev-dep only (never linked into the shipped binary); workspace has no `cargo-deny` / `cargo-audit` gate yet (v2 decision, see REQUIREMENTS.md Out of Scope); acceptable at current project size. |
| T-02-02 | T (Tampering) | `walkdir = "2"` transitive supply chain | accept | Dev-dep only; widely-audited crate; same rationale as T-02-01. |
| T-02-03 | I (Info disclosure) | `base60-core` zero-dep posture | mitigate | Both new deps land under `[dev-dependencies]`, not `[dependencies]`; verification step greps `[dependencies]` section in core for emptiness. CI-03 (Phase 7) will automate this later. |
| T-02-04 | D (Denial of service) | xtask scaffold compile time | accept | xtask crate has no runtime code and one dev-dep (`walkdir`); compile-time impact < 2s on a warm cache — negligible. |

Phase attack surface is minimal: no network I/O, no untrusted input, no new public API. Dispositions are mostly "accept" because the added code is test-only and the added supply-chain surface is dev-build-only.
</threat_model>

<verification>
After Task 1 + Task 2:
- `cargo check --workspace --locked` exits 0.
- `cargo test --workspace --all-targets --locked` still passes (no behaviour changed yet; annotations + gate come in Plans 02/03).
- `cargo metadata --format-version 1 --manifest-path crates/base60-core/Cargo.toml --no-deps` shows zero non-dev dependencies.
- `grep -c '\[dependencies\]' crates/base60-core/Cargo.toml` returns 0.
- `grep -c 'crates/xtask' Cargo.toml` returns at least 1.
</verification>

<success_criteria>
- Workspace is a 3-member project (base60-core, base60-cli, xtask) compiling cleanly under `--locked`.
- `serial_test` is importable from tests in both `base60-cli` and `base60-core` (verified by the next plan using `use serial_test::serial;`).
- base60-core runtime dep count: **0** (CI-03 anticipated invariant preserved).
</success_criteria>

<output>
After completion, create `.planning/phases/02-env-test-serialisation/02-01-workspace-prep-SUMMARY.md` documenting:
- Exact `[dev-dependencies]` sections added (both crates) + `crates/xtask/Cargo.toml` content.
- `cargo check --workspace --locked` output (pass).
- `Cargo.lock` diff summary (new serial_test + walkdir entries).
- Zero-dep posture verification result for base60-core.
</output>
