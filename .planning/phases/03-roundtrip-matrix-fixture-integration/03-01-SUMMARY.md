---
phase: 03-roundtrip-matrix-fixture-integration
plan: 01
subsystem: base60-cli
tags: [rust, cli, refactor, library-target, dispatch-table]
dependency_graph:
  requires:
    - Phase 1 (REF-02): LensMode::ALL precedent + exhaustiveness-test idiom
    - Phase 2 (TEST-04): serial_test dev-dep + env-discipline xtask gate
  provides:
    - base60::run — library entry point callable from integration tests
    - base60::LensMode — re-exported dispatch-table enum (was pub(crate))
    - base60::LensMode::ALL — re-exported slice of all variants
    - base60::Format — re-exported output-format enum (was pub(crate))
    - base60::Format::ALL — NEW symmetric dispatch-table slice
  affects:
    - 03-02 (TEST-01 matrix) — can now `use base60::{LensMode, Format}` from tests/
    - 03-03 (TEST-03 fixture tests) — can spawn via assert_cmd + iterate Format::ALL
tech_stack:
  added: []
  patterns:
    - "Binary-with-lib crate layout: single package exposes both [lib] and [[bin]] sharing name=base60"
    - "Minimal re-export surface: pub use cli::{Format, LensMode}; only the two enums tests need"
    - "Exhaustiveness test via closed-match-over-variant-list + slice-contains assertion"
key_files:
  created:
    - path: crates/base60-cli/src/lib.rs
      role: Library root; pub fn run + pub use cli::{Format, LensMode} + relocated env tests
      size: "231 lines / 7941 bytes"
  modified:
    - path: crates/base60-cli/src/main.rs
      role: Thin binary shim calling base60::run()
      size_delta: "227 → 13 lines (-214 lines)"
    - path: crates/base60-cli/Cargo.toml
      role: Added [lib] stanza with name=base60 path=src/lib.rs before [[bin]]
      size_delta: "31 → 35 lines (+4)"
    - path: crates/base60-cli/src/cli.rs
      role: Widened LensMode + Format enums (and LensMode::ALL) to pub; added Format::ALL + one exhaustiveness test
      size_delta: "+15 test lines, +6 Format::ALL lines; net +21 lines"
decisions:
  - "D-06/D-08 honoured: lib target added as sibling of [[bin]], both share name=base60 (cargo differentiates by target kind); cargo install still ships only the binary"
  - "D-07 honoured: lib public surface = exactly { run, Format, Format::ALL, LensMode, LensMode::ALL, LensMode::cycle, LensMode::label } — no other widening"
  - "D-09 applied: LensMode::ALL pub(crate) → pub (revision of Phase 1 D-06); dead_code allow + TODO removed (Phase 3 is the production consumer)"
  - "D-10 applied: Format::ALL added as pub const ALL: &[Self] = &[Ansi, Plain, Json, Html] + all_contains_every_format_variant test"
  - "D-23 coalesced: despite three Task blocks in the plan, D-24 forbids broken intermediate state, so all three edits ship in a single commit (matches the frontmatter commit_message and D-23 row 1)"
metrics:
  duration: "≈3 minutes"
  completed: "2026-04-24"
  tasks: 3
  commits: 1
  files_changed: 4
  files_created: 1
---

# Phase 03 Plan 01: Thin lib target + Format::ALL dispatch table — Summary

## One-liner

`base60-cli` now exposes a minimal library façade (`pub fn run`, `pub use cli::{Format, LensMode}`) so future integration tests in `tests/` can iterate the `LensMode × Format` dispatch tables without re-implementing them; existing binary behaviour is byte-identical.

## What shipped

### New library target

- **`crates/base60-cli/src/lib.rs`** (231 lines, new file) hosts:
  - `pub fn run() -> anyhow::Result<()>` — verbatim body of the previous `fn main`.
  - Eleven `mod X;` declarations (`analyze`, `chunk`, `cli`, `color`, `decode`, `dump`, `format`, `persist`, `reader`, `search`, `tui`).
  - `pub use cli::{Format, LensMode};` — the ONLY re-export (D-07 minimum surface).
  - Private helpers `run_view` / `run_analyze` / `run_decode` / `run_completions` / `pick_palette` — relocated verbatim from `main.rs`.
  - `#[cfg(test)] mod tests` block with **all 5 `#[serial(env)]` env-mutating tests** (`auto_with_tty_and_no_env_is_ansi`, `auto_with_no_tty_is_mono`, `auto_with_no_color_env_is_mono`, `always_forces_ansi_even_without_tty`, `never_forces_mono_even_with_tty`) and **4 SAFETY comments** moved verbatim.
  - Crate-root attributes: `#![forbid(unsafe_op_in_unsafe_fn)]` + `#![allow(clippy::redundant_pub_crate)]`.

- **`crates/base60-cli/src/main.rs`** (13 lines, from 227) is now a pure shim:
  ```rust
  #![forbid(unsafe_op_in_unsafe_fn)]
  #![allow(clippy::redundant_pub_crate)]

  //! Entry point for the `base60` binary viewer.

  fn main() -> anyhow::Result<()> {
      base60::run()
  }
  ```

- **`crates/base60-cli/Cargo.toml`** gains a `[lib]` stanza before the existing `[[bin]]`:
  ```toml
  [lib]
  name = "base60"
  path = "src/lib.rs"
  ```
  Sharing `name = "base60"` between `[lib]` and `[[bin]]` is permitted; `cargo install --path crates/base60-cli` still ships only the binary.

### Enum visibility widening (E0365 fix)

- **`pub enum LensMode`** (was `pub(crate)`) — required so `pub use cli::LensMode;` in `lib.rs` compiles. Variants inherit.
- **`pub enum Format`** (was `pub(crate)`) — same reason for `pub use cli::Format;`.
- **`pub const LensMode::ALL: &[Self]`** (was `pub(crate)`). The `#[allow(dead_code)]` attribute and the `TODO(phase-3 TEST-01)` comment that preceded it are **removed** — Phase 3 is the production consumer that the TODO anticipated.
- Other enums (`ColorChoice`, `TimeScale`, `Command`) and `build_lens`, `cycle`, `label` stay `pub(crate)` — the minimum surface contract from D-07 is preserved.

### New dispatch-table slice + exhaustiveness test

- **`Format::ALL`** mirrors `LensMode::ALL`:
  ```rust
  impl Format {
      pub const ALL: &[Self] = &[Self::Ansi, Self::Plain, Self::Json, Self::Html];
  }
  ```
  Shape is `&[Self]` (not `[Self; 4]`) by RESEARCH §Format::ALL Shape Decision option A — symmetric with `LensMode::ALL`.
- **`all_contains_every_format_variant`** — single new test inside the existing `#[cfg(test)] mod tests` block in `cli.rs`. Enumerates every variant through a closed array literal + `Format::ALL.contains(&variant)` + length check. Adding a new `Format` variant will trigger compile-time exhaustiveness on the literal, pointing the author at this test.

## Re-export collision avoidance

The plan flagged a potential name collision: the inner line `use cli::{..., Format, ...};` would compete with `pub use cli::{Format, LensMode};` at lib-crate root scope. Resolution (D-07 compliant): `Format` is **removed** from the inner `use cli::{...};` list; it enters scope solely via the top-level `pub use`, which is idiomatic and avoids the conflict. Final `use cli::{AnalyzeArgs, ColorChoice, Command, CompletionsArgs, DecodeArgs, ViewArgs};` — six items, no `Format`. No surprises during integration; Rust accepted the layout on the first build after the Task 3 widenings were in place.

## Verification (D-24 — full CI gate)

All four CI-level gates pass green after the combined commit:

| Gate | Command | Result |
|------|---------|--------|
| Build (lib + bin) | `cargo build -p base60 --all-targets --locked` | OK |
| Tests (all targets, workspace) | `cargo test --workspace --all-targets --locked` | OK — 167 tests pass, +1 vs. pre-plan baseline (`all_contains_every_format_variant`) |
| Clippy pedantic+nursery+cargo | `cargo clippy --workspace --all-targets --locked -- -D warnings` | OK — zero warnings |
| Fmt | `cargo fmt --all --check` | OK — zero diffs |
| Doc | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` | OK — zero broken links |
| xtask env-discipline | `cargo test -p xtask --locked` | OK — all 5 relocated env-tests still carry `#[serial(env)]` |
| Binary still builds | `cargo build -p base60 --bin base60 --locked` | OK |

## Commits

| Hash    | Message                                                        |
| ------- | -------------------------------------------------------------- |
| 42f4f1e | refactor(cli): add thin lib target + Format::ALL dispatch table |

Single commit (matches D-23 row 1 and the plan frontmatter `commit_message`). The plan is expressed as three logical Tasks, but D-24 ("each commit must pass all gates before the next starts") mandates atomic application: lib.rs without the Task 3 widenings would fail to build (E0365), violating D-24. Shipping all three changes in one commit is the only way to honour D-24 and D-23 simultaneously. Confirmed against plan frontmatter: `commit_message: "refactor(cli): add thin lib target + Format::ALL dispatch table"` — exactly one message for the plan.

## Line / byte deltas

| File                              | Before             | After              | Δ          |
| --------------------------------- | ------------------ | ------------------ | ---------- |
| `crates/base60-cli/src/lib.rs`    | —                  | 231 lines / 7941 B | +231 / +7941 |
| `crates/base60-cli/src/main.rs`   | 227 lines / ~7.3 KB | 13 lines / 489 B  | −214 / −6.8 KB |
| `crates/base60-cli/Cargo.toml`    | 31 lines           | 35 lines           | +4        |
| `crates/base60-cli/src/cli.rs`    | 330 lines           | 352 lines           | +22 (Format::ALL impl + new test) |

Sum of additions = 266 lines, sum of deletions = 223 lines (`git show --stat 42f4f1e`). Net +43 workspace lines (with the `lib.rs` → `main.rs` relocation factored in, the effective "new" code is the 4 Cargo.toml lines + the 22 cli.rs lines = 26 new-concept lines).

## Deviations from Plan

None — plan executed exactly as written. One structural observation (not a deviation): although the plan is organised as three `<task>` blocks each with its own `<verify>` step, strict sequential commits would leave the intermediate state in E0365 (Task 1 alone, Task 1+2 alone). The plan frontmatter, the action-text of Task 1, and D-23 row 1 all agree that the three Tasks form a single commit unit, so the verify gates were run once at the end rather than three times in sequence.

## Known Stubs

None. No placeholder data, no TODO/FIXME, no empty rendering paths. The `LensMode::ALL` TODO comment that hinted at this phase has been explicitly retired.

## Notes (RU)

- План 03-01 сдан единым коммитом — иначе промежуточные состояния ломают E0365 / D-24.
- `pub use` сужает утечку API: только `Format` + `LensMode` (оба с их `ALL`) доступны тестам под `tests/`, остальные символы остаются `pub(crate)`.
- Binary-with-lib крейт идиоматичен: `name = "base60"` на `[lib]` и `[[bin]]` не конфликтует, `cargo install` ставит только бинарник.
- Готов 03-02 (TEST-01 матрица) к `use base60::{Format, LensMode};` напрямую, без lens-дублей в тестовом helper.

## Self-Check: PASSED

Verified:
- `/home/chris/Projects/utils/test-60/.claude/worktrees/agent-a63bf77c/crates/base60-cli/src/lib.rs` — FOUND
- `/home/chris/Projects/utils/test-60/.claude/worktrees/agent-a63bf77c/crates/base60-cli/src/main.rs` (13 lines) — FOUND
- `/home/chris/Projects/utils/test-60/.claude/worktrees/agent-a63bf77c/crates/base60-cli/Cargo.toml` ([lib] stanza present) — FOUND
- `/home/chris/Projects/utils/test-60/.claude/worktrees/agent-a63bf77c/crates/base60-cli/src/cli.rs` (pub enums, Format::ALL, new test) — FOUND
- Commit `42f4f1e` in `git log` — FOUND
