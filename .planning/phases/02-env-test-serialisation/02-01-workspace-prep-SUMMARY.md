---
phase: 02-env-test-serialisation
plan: 01
subsystem: build/workspace
tags: [workspace, dev-deps, xtask, serial_test, walkdir]
requires:
  - Phase 1 complete (REF-01, REF-02)
provides:
  - crates/xtask workspace member scaffold
  - serial_test = "3" dev-dep on base60-cli and base60-core
  - walkdir = "2" dev-dep on xtask
  - base-green workspace for Plans 02-02 and 02-03
affects:
  - Cargo.toml (workspace members)
  - Cargo.lock (serial_test + walkdir + transitives)
tech_stack:
  added:
    - "serial_test 3.4.0 (dev-dep, default-features = false)"
    - "walkdir 2 (dev-dep in xtask)"
  patterns:
    - "dev-dep surface only — CI-03 zero-runtime-dep posture on base60-core preserved"
    - "xtask = bare library-only test-hosting crate (D-08)"
key_files:
  created:
    - crates/xtask/Cargo.toml
    - crates/xtask/src/lib.rs
  modified:
    - Cargo.toml
    - crates/base60-cli/Cargo.toml
    - crates/base60-core/Cargo.toml
    - Cargo.lock
decisions:
  - "serial_test pinned as major-only \"3\" per roadmap wording (resolves to 3.4.0)"
  - "default-features = false — async/file_locks/logging unused"
  - "xtask ships empty lib.rs; gate test comes in Plan 02-03 as tests/env_discipline.rs"
metrics:
  duration: ~5 min
  completed: 2026-04-24
  tasks: 2
  files_changed: 5
  commits: 2
---

# Phase 2 Plan 01: Workspace Prep Summary

Dev-dep and workspace-member scaffolding for Phase 2 env-test serialisation landed cleanly — `serial_test = "3"` now dev-dep on both `base60-cli` and `base60-core`; `crates/xtask/` exists as a bare test-only workspace member with `walkdir = "2"`; `cargo check --workspace --locked` is green and `base60-core` keeps its zero-runtime-dep posture (0 entries under `[dependencies]`).

## Tasks Completed

| Task | Name                                  | Commit   | Files                                                                 |
| ---- | ------------------------------------- | -------- | --------------------------------------------------------------------- |
| 1    | Scaffold xtask workspace member       | 437eead  | Cargo.toml, Cargo.lock, crates/xtask/Cargo.toml, crates/xtask/src/lib.rs |
| 2    | Add serial_test dev-dep to cli & core | efcc388  | crates/base60-cli/Cargo.toml, crates/base60-core/Cargo.toml, Cargo.lock |

## Exact Manifest Additions

### `crates/xtask/Cargo.toml` (new)

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

### `crates/xtask/src/lib.rs` (new)

```rust
//! Workspace-level automation helpers for base60.
//!
//! This crate hosts repo-wide invariant checks that run as integration
//! tests under `cargo test --workspace --all-targets --locked`. It has
//! no runtime code; all behaviour is in `tests/*.rs`.
```

### `Cargo.toml` (workspace root, members line)

```diff
-members = ["crates/base60-core", "crates/base60-cli"]
+members = ["crates/base60-core", "crates/base60-cli", "crates/xtask"]
```

### `crates/base60-cli/Cargo.toml` (new `[dev-dependencies]` section)

```toml
[dev-dependencies]
serial_test = { version = "3", default-features = false }
```

Inserted between the existing `[dependencies]` block and `[lints]`.

### `crates/base60-core/Cargo.toml` (new `[dev-dependencies]` section)

```toml
[dev-dependencies]
serial_test = { version = "3", default-features = false }
```

Inserted between `publish.workspace = true` and `[lints]`. `[dependencies]` section remains absent (CI-03 invariant preserved).

## `cargo check --workspace --locked` Output (pass)

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s
```

First run after Task 1 reported `error: cannot update the lock file … because --locked was passed`; expected — ran `cargo check --workspace` (without `--locked`) once to refresh `Cargo.lock`, then re-ran `--locked` green. Same pattern after Task 2. Standard idiom; no unexpected behaviour.

Full test suite also verified green post-change:

```
total passed: 165 failed: 0
```

(sum of `test result: ok.` lines across all 165 workspace tests including unit, lib, and doc tests; `xtask` itself reports `0 passed` — expected, empty lib + no tests yet.)

## `Cargo.lock` Diff Summary

New packages locked into the workspace:

| Package              | Version  | Source                                      |
| -------------------- | -------- | ------------------------------------------- |
| `serial_test`        | 3.4.0    | dev-dep on cli + core                       |
| `serial_test_derive` | 3.4.0    | proc-macro transitive of `serial_test`      |
| `scc`                | 2.4.0    | transitive of `serial_test` (serialisation scc) |
| `sdd`                | 3.0.10   | transitive of `scc`                         |
| `walkdir`            | 2.x      | dev-dep on xtask                            |
| `same-file`          | —        | transitive of `walkdir`                     |
| `winapi-util`        | —        | Windows transitive of `walkdir`/`same-file` |
| `xtask`              | 0.1.0    | new workspace member                        |

Total 4 new non-workspace packages surface from `serial_test` (`default-features = false` kept `tokio`/`log`/`fslock` branches pruned); 3 from `walkdir`. All are dev-only — none link into the shipped `base60` binary.

## Zero-Dep Posture Verification (base60-core)

```
core runtime deps: []
core dev deps: ['serial_test']
CI-03 zero-dep invariant: OK
```

`cargo metadata --format-version 1 --manifest-path crates/base60-core/Cargo.toml` returned zero entries with `kind = null` (runtime) and exactly one entry with `kind = "dev"` (`serial_test`). Also `grep -c '\[dependencies\]' crates/base60-core/Cargo.toml` = 0. Runtime contract intact.

## Success Criteria

- [x] Workspace is a 3-member project (base60-core, base60-cli, xtask) compiling cleanly under `--locked`.
- [x] `serial_test` importable from tests in both cli + core (available via `[dev-dependencies]`; Plan 02-02 will exercise).
- [x] `base60-core` runtime dep count: **0** (CI-03 anticipated invariant preserved).

## Verification Commands (all pass)

- `cargo check --workspace --locked` → exit 0
- `cargo test --workspace --all-targets --locked` → 165 passed / 0 failed
- `grep -c 'crates/xtask' Cargo.toml` → 1
- `grep -c '\[dependencies\]' crates/base60-core/Cargo.toml` → 0
- `cargo metadata --manifest-path crates/base60-core/Cargo.toml` → 0 runtime, 1 dev (serial_test)

## Deviations from Plan

None — plan executed exactly as written.

## Threat Flags

None — this plan adds only dev-build supply-chain surface (explicit `accept` dispositions T-02-01, T-02-02 in plan's threat model; `mitigate` disposition T-02-03 verified via the zero-dep check above). No new runtime surface, no network I/O, no untrusted input.

## Follow-ups / Handoff to Plan 02-02 and 02-03

- **Plan 02-02 (Serial-env annotations):** Can now `use serial_test::serial;` inside `#[cfg(test)] mod tests` of `base60-cli/src/main.rs`, `base60-core/src/cuneiform.rs`, `base60-core/src/lens.rs`. No further Cargo changes required.
- **Plan 02-03 (Env discipline gate):** Can now create `crates/xtask/tests/env_discipline.rs` and `use walkdir::WalkDir;`. The `src/lib.rs` is intentionally empty per D-08 — gate goes in `tests/`, not `src/`.

## Self-Check: PASSED

- Created files exist:
  - FOUND: crates/xtask/Cargo.toml
  - FOUND: crates/xtask/src/lib.rs
- Commits exist:
  - FOUND: 437eead (Task 1: scaffold xtask)
  - FOUND: efcc388 (Task 2: serial_test dev-dep)
- Workspace compiles: `cargo check --workspace --locked` exits 0
- Tests pass: 165 / 0 failed
- CI-03 invariant: 0 runtime deps on base60-core (verified via `cargo metadata`)
