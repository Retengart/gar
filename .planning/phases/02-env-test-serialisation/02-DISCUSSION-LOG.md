# Phase 2: Env-Test Serialisation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-24
**Phase:** 02-env-test-serialisation
**Areas discussed:** Core env-test placement, Gate enforcement mechanism, Annotation granularity, 10× smoke test enforcement, Gate home / name / walker / scope, Commit granularity, Gate read-flagging policy, `persist.rs` deferred-comment refresh

---

## Gray area selection (multiSelect)

| Option | Description | Selected |
|--------|-------------|----------|
| Core env-test placement | `base60-core` has 2 env tests; keep in-tree (dev-dep) vs. relocate. | ✓ |
| Gate enforcement mechanism | Rust test vs. CI shell vs. both. | ✓ |
| Annotation granularity | Per-test vs. module-wrap macro. | ✓ |
| 10× smoke test enforcement | Script + CI step vs. loop in CI vs. convention only. | ✓ |

**User note:** "adding proptest" — flagged as out-of-scope; captured in Deferred Ideas.

---

## Core env-test placement

| Option | Description | Selected |
|--------|-------------|----------|
| Keep in-tree, add dev-dep | Add `serial_test` to `base60-core/Cargo.toml [dev-dependencies]`. Core's zero-runtime-dep posture is preserved (CI-03 checks `[dependencies]` only). | ✓ |
| Relocate to `base60-cli/tests/` | Move the 2 env tests to an integration test in the binary crate to keep `base60-core` pristine. | |

**Rationale:** Simpler; dev-deps don't affect consumer builds or the zero-dep selling point.

---

## Gate enforcement mechanism

| Option | Description | Selected |
|--------|-------------|----------|
| In-repo Rust test | Integration test that walks sources and asserts the invariant; runs via `cargo test`. | ✓ |
| CI shell grep step | 3-line CI-only check; cheap but brittle. | |
| Both | Defense-in-depth; two places to maintain. | |

**Rationale:** Local + CI coverage in one mechanism; Rust-aware parsing dodges comment/string false positives.

---

## Annotation granularity

| Option | Description | Selected |
|--------|-------------|----------|
| Per-test `#[serial(env)]` | Explicit, greppable, library-idiomatic for `serial_test` 3.x. ~7 annotations total. | ✓ |
| Sub-module wrap via helper macro | Single application point; overkill for current scale. | |

---

## 10× smoke test enforcement

| Option | Description | Selected |
|--------|-------------|----------|
| Helper script + 1× CI step | Local 10-loop + single `--test-threads=8` step in CI. | ✓ |
| CI matrix cell runs 10-loop | Strongest, permanent; ~10× wall-clock cost per PR. | |
| Convention only | Reviewer vigilance — exactly what this phase removes. | |

---

## Gate home (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| `base60-cli/tests/env_discipline.rs` (Recommended) | Binary crate; already has a `[dev-dependencies]` slot. | |
| Workspace-root new crate | Neutral home outside either shipped crate; adds a workspace member. | ✓ |

**Note:** User chose the non-recommended option. Reflected back and confirmed; planner treats this as locked.

---

## Commit granularity (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| One commit per REQ-ID | Per Phase 1 precedent. Dep + annotations + gate in one atomic change. | ✓ |
| Split into three commits | Dep, annotations, gate separately. More granular but with unenforced intermediate states. | |

---

## Gate read-flagging policy (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| Mutations only | `env::set_var`, `env::remove_var`. Reads are race-free without serial. | ✓ |
| Reads too | Over-inclusive; false positives on `TERM` read at `cuneiform.rs:158`. | |

---

## `persist.rs` deferred-comment refresh (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| Leave for Phase 4 | Phase 4 deletes the comment and adds the real test. Clean separation. | ✓ |
| Refresh to point at `#[serial(env)]` | One-line bleed of Phase 4 context into Phase 2. | |

---

## Gate crate name (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| `crates/xtask/` | Standard Rust convention for workspace tooling. Room to grow for future checks. | ✓ |
| `crates/env-gate/` | Purpose-named; narrower scope. | |

---

## File-walker approach (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| std only (Recommended) | No deps; hand-rolled `fs::read_dir` recursion for ~a dozen files. | |
| walkdir dev-dep | Ergonomic iteration; small transitive footprint confined to xtask. | ✓ |

**Note:** User chose the non-recommended option. `walkdir = "2"` added to `crates/xtask/Cargo.toml [dev-dependencies]`.

---

## Gate walk scope (follow-up)

| Option | Description | Selected |
|--------|-------------|----------|
| `crates/*/src/**/*.rs` | Both crates' source trees. | ✓ |
| Only `#[test]`-bearing files | Pre-filter heuristic; misses legitimate raw `mod tests` blocks. | |

---

## Claude's Discretion

- Exact `smoke-serial.sh` shape (flag set, output format).
- Gate parser implementation (line-window walk vs. minimal tokenizer).
- Gate failure diagnostic format.
- Whether the CI step is named `test-threads-8` or `test (threads=8)`.
- Version-string style: `"3"` vs. `"3.x"`.

## Deferred Ideas

- Proptest / property-based testing — not a scope fit for Phase 2; revisit for Phase 5 or a later dedicated phase.
- Expanding gate to env reads — revisit only if a concrete race pattern emerges.
- Refreshing the `persist.rs:236-241` deferred comment — Phase 4 replaces it with the real test.
- Macro / module-level annotation wrapper — revisit if env-test count grows beyond ~10.
