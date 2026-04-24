# Phase 2: Env-Test Serialisation - Context

**Gathered:** 2026-04-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace the ad-hoc "don't run concurrently" convention for env-mutating tests with a single enforced `#[serial(env)]` key via `serial_test = "3"`. Mechanical annotation pass + compile-time-adjacent gate that refuses un-annotated env mutations.

Requirement: **TEST-04**.

**In scope:**
- Add `serial_test = { version = "3", default-features = false }` as a dev-dep to both workspace crates.
- Annotate every existing env-mutating `#[test]` with `#[serial(env)]` (7 sites — 5 in `base60-cli/src/main.rs`, 1 in `base60-core/src/cuneiform.rs`, 1 in `base60-core/src/lens.rs`).
- Build the gate as a workspace-root `xtask` crate with an integration test that walks both crates' sources and asserts the invariant.
- Ship a `scripts/smoke-serial.sh` local helper + one `--test-threads=8` CI step.

**Not in scope:**
- The `persist::state_base_dir` test (Phase 4 / TEST-05).
- Any new env-mutating test beyond the existing 7 — Phase 3/4 adds those.
- Proptest / property-based testing (flagged during discussion — see Deferred Ideas).
- Expanding the gate to `env::var` / `env::var_os` reads.

</domain>

<decisions>
## Implementation Decisions

### Dev-dep placement (TEST-04 Success Criterion 2)

- **D-01:** `serial_test = { version = "3", default-features = false }` added to `crates/base60-cli/Cargo.toml [dev-dependencies]`.
- **D-02:** Same dep added to `crates/base60-core/Cargo.toml [dev-dependencies]`. Core env tests stay in-tree (`cuneiform.rs`, `lens.rs`). CI-03 (zero-dep invariant) checks `[dependencies]` only — dev-deps are untouched by that rule, so core's runtime zero-dep posture is preserved.
- **D-03:** `default-features = false` on both sites — `serial_test` default-enables `async`, `file_locks`, `logging`; we need none of those for synchronous `#[serial(env)]`. Minimises transitive footprint.

### Annotation style

- **D-04:** Per-test `#[serial(env)]` attribute directly above each `#[test]`. No macro wrapper, no module-level regrouping. Library-idiomatic for `serial_test` 3.x.
- **D-05:** Exact 7 sites to annotate (no others currently exist):
  - `crates/base60-cli/src/main.rs` → `auto_with_tty_and_no_env_is_ansi`, `auto_with_no_tty_is_mono`, `auto_with_no_color_env_is_mono`, `always_forces_ansi_even_without_tty`, `never_forces_mono_even_with_tty`.
  - `crates/base60-core/src/cuneiform.rs` → `fallback_detection_respects_no_unicode_env`.
  - `crates/base60-core/src/lens.rs` → `cuneiform_auto_respects_no_unicode_env`.
- **D-06:** The `SAFETY:` comments above `unsafe { env::set_var … }` / `unsafe { env::remove_var … }` stay unchanged — they document the Rust 2024 unsafe-env rule, which is orthogonal to `serial_test`'s purpose.

### Gate enforcement (TEST-04 Success Criteria 1 & 4)

- **D-07:** Gate ships as a Rust integration test, not a CI shell step. Runs under `cargo test --workspace --all-targets --locked` — fires in local + CI.
- **D-08:** Gate lives in a new workspace member: `crates/xtask/`. Bare test-only crate — empty `src/lib.rs` + `tests/env_discipline.rs` + a minimal `Cargo.toml`. Name follows the widespread `xtask` convention so future workspace-level checks can grow into it.
- **D-09:** `crates/xtask/Cargo.toml` adds `walkdir = "2"` as a dev-dep. Workspace member entry is added to root `Cargo.toml` `members = [..., "crates/xtask"]`. Inherits `publish = false` from workspace.
- **D-10:** Gate walks `crates/base60-core/src/**/*.rs` and `crates/base60-cli/src/**/*.rs`. Does **not** walk `crates/xtask/` itself (self-loop), nor `target/`, nor any generated paths. Implementation: resolve via `env!("CARGO_MANIFEST_DIR")` relative to xtask, then `../base60-core/src` and `../base60-cli/src`.
- **D-11:** Gate pattern: flag `env::set_var` and `env::remove_var` only. Reads (`env::var`, `env::var_os`) are NOT flagged — reads don't mutate, so concurrent reads are race-free. `TERM` read at `cuneiform.rs:158` stays as-is.
- **D-12:** Gate assertion shape (per-hit): for each `env::set_var`/`env::remove_var` call site, walk upward in the same source file looking for the enclosing function; assert that function's attribute block contains `#[serial(env)]`. If the enclosing function is not a `#[test]` at all, also fail — no env mutation in production code is sanctioned. Keep the parser line-based and minimal (no syn); the invariant is simple enough to express without a full AST walk.
- **D-13:** Gate rejects per-variable keys: any `#[serial(no_color)]`, `#[serial(no_unicode)]`, or `#[serial(term)]` is a fail. Only the bare `#[serial(env)]` (with that exact key spelling) is accepted.

### 10× smoke (TEST-04 Success Criterion 3)

- **D-14:** `scripts/smoke-serial.sh` — a short bash helper that runs `cargo test --workspace --all-targets --locked -- --test-threads=8` in a 10-iteration loop and exits non-zero on the first failure. Documented in contributor docs (CONTRIBUTING.md if present; otherwise PR description handoff). Implementer runs this once locally before the Phase-2 commit lands — it's the phase-handoff gate, not a permanent CI cost.
- **D-15:** CI gains one step (Ubuntu matrix cell only) that runs the command ONCE with `--test-threads=8`. Enough to catch regressions on every PR without multiplying CI wall-clock by 10. Step name: `test-threads-8 (ubuntu)`.

### Commit granularity

- **D-16:** Single commit: `test(cli,core): adopt #[serial(env)] for env-touching tests [TEST-04]`. Per Phase 1 convention (one commit per REQ-ID). Avoids intermediate state where the dep is present but the gate isn't, or annotations exist but the gate isn't enforced.
- **D-17:** The commit spans: `Cargo.toml` (workspace members), `crates/base60-cli/Cargo.toml`, `crates/base60-core/Cargo.toml`, the 3 source files with annotations (`main.rs`, `cuneiform.rs`, `lens.rs`), the new `crates/xtask/` crate, `scripts/smoke-serial.sh`, and `.github/workflows/ci.yml` (new step).

### Claude's Discretion

- Exact lines of the `smoke-serial.sh` helper — any `for i in {1..10}; do cargo test … || exit 1; done` variant is fine; `set -euo pipefail` header expected.
- Whether the gate test's line-based parser uses `grep`-like line counting or a lightweight attribute-window walk; both satisfy D-12.
- Exact diagnostic message format on gate failure — planner picks something actionable (file path + line number + the offending call + hint).
- Whether the CI step name is `test-threads-8` or `test (threads=8)` — cosmetic.
- Whether `serial_test` is pinned as `"3"` or `"3.x"` caret-style — both resolve; `"3"` matches the roadmap wording.

### Folded Todos

(None — `gsd-sdk query todo.match-phase 2` returned 0 matches.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level decisions

- `.planning/PROJECT.md` — Key Decisions; the zero-dep posture of `base60-core` applies to `[dependencies]` (enforced by CI-03 in Phase 7), not `[dev-dependencies]`. This phase's addition of `serial_test` to core's dev-deps is explicitly permitted by that scope split.
- `.planning/REQUIREMENTS.md` — TEST-04 specification (line 25). Lists both `NO_COLOR` and `NO_UNICODE` as the trigger vars and names `serial_test = "3"` with a single shared key.
- `.planning/ROADMAP.md` — Phase 2 Goal + 4 Success Criteria (lines 34-44). Acceptance bar.

### Codebase intelligence

- `.planning/codebase/CONVENTIONS.md` — `#[cfg(test)] mod tests` inline-module style, `unsafe { env::… }` with `SAFETY:` comment pattern. Both must be preserved.
- `.planning/codebase/TESTING.md` — current test-organisation shape; informs where the gate's `tests/env_discipline.rs` fits into the mental model.
- `.planning/codebase/STRUCTURE.md` — workspace layout; references for adding a new `crates/xtask` member.

### v2 research outputs

- `.planning/research/PITFALLS.md` §"Pitfall 1" — this phase is the direct remediation. Read the Fix-approach line before planning.
- `.planning/research/STACK.md` — `serial_test = "3"` is the named dep. No other crates added by this phase.

### Source files this phase edits

- `Cargo.toml` — workspace root; `members` gains `"crates/xtask"`.
- `crates/base60-cli/Cargo.toml` — add `[dev-dependencies] serial_test = { version = "3", default-features = false }`.
- `crates/base60-core/Cargo.toml` — same dev-dep entry.
- `crates/base60-cli/src/main.rs` — 5 `#[serial(env)]` annotations on the `#[cfg(test)] mod tests` block's env-touching functions.
- `crates/base60-core/src/cuneiform.rs` — 1 `#[serial(env)]` annotation on `fallback_detection_respects_no_unicode_env`.
- `crates/base60-core/src/lens.rs` — 1 `#[serial(env)]` annotation on `cuneiform_auto_respects_no_unicode_env`.
- `crates/xtask/` — NEW: `Cargo.toml`, `src/lib.rs` (empty), `tests/env_discipline.rs` (the gate).
- `scripts/smoke-serial.sh` — NEW: 10-loop local helper.
- `.github/workflows/ci.yml` — one new step under the `test` job (Ubuntu matrix cell).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- The 7 env-touching tests already follow a uniform shape: `unsafe { env::set_var(...) } ... unsafe { env::remove_var(...) }` with a `SAFETY:` comment. Annotation is purely additive; no test-body restructuring.
- `#[cfg(test)] mod tests` inline-module convention is crate-wide — gate walker can rely on test functions living inside `mod tests` blocks without scanning integration tests (we have none yet in-repo).
- Workspace-level `[workspace.lints]` (pedantic + nursery + cargo) applies to the new `xtask` crate automatically via `[lints] workspace = true` — new crate gets the same bar.

### Established Patterns

- `unsafe { env::… }` block with `SAFETY:` doc-comment is the crate-wide idiom for Rust 2024's unsafe-env rule. Leave these comments verbatim — they explain something `serial_test` does not replace.
- `#[must_use]`, `#[derive(Debug)]`, and `rustfmt`-clean modules are the universal style — xtask's `src/lib.rs` (even if empty) gets a one-line module docstring.
- Integration tests elsewhere in the workspace: none yet. Phase 3 will add `crates/base60-cli/tests/`. The `xtask` gate pre-dates Phase 3's fixture layout; keep the two structurally independent.

### Integration Points

- Root `Cargo.toml` `members` currently lists `["crates/base60-core", "crates/base60-cli"]`. One-line append: `"crates/xtask"`.
- `.github/workflows/ci.yml` `test` job already runs `cargo test --workspace --all-targets --locked`. The new step can be a sibling run: `cargo test --workspace --all-targets --locked -- --test-threads=8`. Keeping it a separate step makes flaky-vs-deterministic failure easy to triage.
- No existing `scripts/` directory — creating one is a small structural precedent for future helper scripts.

### Constraints from existing CI (`.github/workflows/ci.yml`)

- Workspace matrix is `ubuntu-latest / macos-latest / windows-latest` × `1.95.0 / stable / beta`. The new gate test runs on every matrix cell (it's part of `cargo test --workspace`). walkdir is cross-platform so this is fine.
- Clippy job (`pedantic + nursery + cargo -D warnings`) will lint the xtask crate and the new test file. Budget for: `pub(crate)` items, `#[must_use]` on non-unit-returning helpers, doc comments on every `pub(crate)` symbol.
- `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS: -D warnings` covers xtask too — any `pub(crate)` item in the new crate needs a doc comment.

</code_context>

<specifics>
## Specific Ideas

- Name the gate file `tests/env_discipline.rs` — "discipline" names the invariant this test enforces (env-touching tests must discipline themselves with `#[serial(env)]`), rather than describing the mechanism.
- Use `walkdir = "2"` in xtask dev-deps. User-picked; cleaner `.into_iter().filter_entry(...)` chain than hand-rolled `fs::read_dir` recursion for ~a dozen files + future growth.
- The new `scripts/smoke-serial.sh` starts with `#!/usr/bin/env bash` + `set -euo pipefail`. Ten-iteration loop; exit 1 on first failure; echo the iteration number. Chmod +x in the commit.
- CI step for `--test-threads=8` runs AFTER the existing `Unit + integration tests` step, so the default-threads run always lands first as the canonical green signal.

</specifics>

<deferred>
## Deferred Ideas

- **Proptest / property-based testing.** Raised during discussion. Not a scope fit for Phase 2 (mechanical serialisation is deterministic; nothing to property-check). Candidates for a future phase: Phase 5 (`parse_run` fuzz target already covers a related surface) or a dedicated "proptest-adopt" phase if a clear property emerges. Note in backlog.
- **Expanding gate to env reads.** Considered and rejected in D-11. If a future test case reads an env var while a concurrent test mutates the same var, revisit. Not expected before Phase 6.
- **`persist::state_base_dir` deferred comment refresh** — the `// state_base_dir reads process-wide env vars …` block at `persist.rs:236-241` stays verbatim. Phase 4 deletes it and replaces with the real test. Touching it in Phase 2 would bleed scope.
- **Module-level or macro-driven annotation wrapper** — rejected in D-04. If Phase 3/4 adds >10 more env-touching tests, revisit; per-test is currently optimal.

</deferred>

---

*Phase: 02-env-test-serialisation*
*Context gathered: 2026-04-24*
