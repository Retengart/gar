---
phase: 03-roundtrip-matrix-fixture-integration
verified: 2026-04-24T00:00:00Z
status: passed
score: 9/9 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: none
  previous_score: n/a
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 3: Roundtrip Matrix + Fixture Integration — Verification Report

**Phase Goal:** The Core Value guarantee — every `base60 FILE | base60 decode` round-trips byte-identically — is asserted exhaustively across the `LensMode × FormatMode` product before any contract-tightening refactor runs. Fixture-driven `assert_cmd` tests cover dump / analyze / decode / completions entry points.
**Verified:** 2026-04-24
**Status:** passed
**Re-verification:** No — initial verification.

**Scope note (from user):** ROADMAP Phase 3 SC1 was narrowed during execution from 140 cells (5×7×4) to 28 cells (2×7×2) — JSON/HTML formats and non-8-aligned fixtures are deferred to REF-04 (Phase 4). This report verifies against the current ROADMAP wording, not the original.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | 28-cell byte-identical matrix exists, compiles, green (SC1 narrowed) | ✓ VERIFIED | `tests/roundtrip.rs:34` has exactly one `#[test] fn roundtrip_matrix_byte_identical`; iterates `ROUNDTRIP_FIXTURES (2) × ALL_LENS_CONFIGS (7) × ROUNDTRIP_FORMATS (2) = 28`; `cargo test -p base60 --test roundtrip --locked` → `1 passed` in 0.14s |
| 2 | Per-subcommand fixture coverage (SC2) | ✓ VERIFIED | `tests/fixtures.rs` contains 4 `#[test]` (dump/analyze/decode-roundtrip/completions); dump/analyze iterate 5 fixtures, decode iterates 2 (aligned with narrowed matrix), completions iterates 5 shells; `cargo test -p base60 --test fixtures --locked` → `4 passed` |
| 3 | CLI edge coverage incl. decoder-error pin (SC2) | ✓ VERIFIED | `tests/cli.rs` contains 9 `#[test]` covering stdin piping, BrokenPipe exit-0 (via `spawn_with_closed_stdout`), NO_COLOR + 3 `--color` modes + CLICOLOR_FORCE, `--skip`/`--length` clamps, `zero_skip_is_identity`, decoder `"99"+"invalid"` stderr pin at `cli.rs:156-167`; `cargo test -p base60 --test cli --locked` → `9 passed` |
| 4 | In-test fixture generation, no tracked binaries > 8 KiB under `tests/` (SC3) | ✓ VERIFIED | `git ls-files crates/base60-cli/tests/` lists 4 files: `cli.rs (6512 B)`, `common/mod.rs (13184 B)`, `fixtures.rs (4202 B)`, `roundtrip.rs (3492 B)` — all source `.rs`, none exceed 13 KB, and `common/mod.rs` being > 8 KB is irrelevant (it is `.rs`, not a binary fixture); all 5 fixtures generated in `common/mod.rs::fixtures::*` as Rust factories with `debug_assert_eq!` byte-size pins |
| 5 | Spawn-discipline invariant (SC4) | ✓ VERIFIED | `crates/xtask/tests/spawn_discipline.rs` exists (86 lines, file: 2921 B); `grep -rn "Command::cargo_bin" crates/base60-cli/tests/` returns only `common/mod.rs:5` (doc-comment) + `common/mod.rs:39` (the one sanctioned call); `cargo test -p xtask --test spawn_discipline --locked` → `1 passed`; `cargo test -p xtask --test env_discipline --locked` → `1 passed` (Phase 2 gate unaffected) |
| 6 | `base60-cli` has thin `[lib]` target (D-06..D-08) | ✓ VERIFIED | `Cargo.toml:13-15` declares `[lib] name = "base60" path = "src/lib.rs"`; `src/lib.rs` exists (7941 B, 231 lines); `src/main.rs` is 13-line shim calling `base60::run()` (489 B) |
| 7 | Minimal public surface — only `{LensMode, Format, run}` exposed | ✓ VERIFIED | `lib.rs:28` has `pub use cli::{Format, LensMode};` as the sole re-export; `lib.rs:36` has `pub fn run() -> Result<()>`; `cli.rs:25` `pub enum LensMode`, `cli.rs:46` `pub const ALL`, `cli.rs:119` `pub enum Format`, `cli.rs:138` `pub const ALL`; `cli.rs:57/70/88` keep `cycle`/`label`/`build_lens` as `pub(crate)`; `ColorChoice`/`TimeScale`/`Command`/`Cli`/`*Args` all remain `pub(crate)` |
| 8 | Requirements traceability — TEST-01 and TEST-03 delivered | ✓ VERIFIED | `03-02-PLAN.md` frontmatter declares `requirements: [TEST-01, TEST-03]`; `03-03-PLAN.md` frontmatter declares `requirements: [TEST-03]`; TEST-01 covered by `tests/roundtrip.rs` (28-cell matrix — narrowed by user decision, REF-04 filed in REQUIREMENTS.md line 19); TEST-03 covered by `tests/fixtures.rs` + `tests/cli.rs` + spawn-discipline gate; ready for `update_roadmap` to mark both complete |
| 9 | No regression in prior-phase tests (182-test workspace target) | ✓ VERIFIED | `cargo test --workspace --all-targets --locked` → 125 (base60 lib unit) + 0 (base60 main unit) + 9 (cli edges) + 4 (fixtures) + 1 (roundtrip) + 41 (base60-core) + 0 (xtask lib) + 1 (env_discipline) + 1 (spawn_discipline) = **182 passed, 0 failed**, matches Plan 03-03 SUMMARY claim |

**Score:** 9/9 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/base60-cli/src/lib.rs` | pub fn run, pub use cli::{Format, LensMode}, 11 mod decls, 5 #[serial(env)] tests | ✓ VERIFIED | 231 lines; `grep '^pub fn run'` = 1; `grep 'pub use cli::{Format, LensMode};'` = 1 match; `grep '#\[serial(env)\]'` = 5; `grep 'SAFETY:'` = 4 |
| `crates/base60-cli/src/main.rs` | ≤15-line shim calling `base60::run()` | ✓ VERIFIED | 13 lines, 489 bytes, contains `base60::run()` call on line 12 |
| `crates/base60-cli/Cargo.toml` | `[lib]` + `[[bin]]` both named `base60`; dev-deps: assert_cmd=2, predicates=3, base60-core path, serial_test=3 | ✓ VERIFIED | All 4 dev-deps present; `base60-core = { path = "../base60-core" }` appears twice (once in `[dependencies]`, once in `[dev-dependencies]`) as planned |
| `crates/base60-cli/src/cli.rs` | `pub enum LensMode`, `pub enum Format`, `pub const ALL` (both), exhaustiveness test | ✓ VERIFIED | Line 25 `pub enum LensMode`; line 46 `pub const ALL: &[Self]`; line 119 `pub enum Format`; line 138 `pub const ALL`; `all_contains_every_format_variant` test at line 339 |
| `crates/base60-cli/tests/common/mod.rs` | base60_cmd, 5 fixtures, LensConfig + ALL_LENS_CONFIGS, assert_roundtrip, spawn_with_closed_stdout, ROUNDTRIP_FIXTURES, ROUNDTRIP_FORMATS | ✓ VERIFIED | 319 lines / 13184 B; `base60_cmd` at :38; 5 fixture factories at :66-149; `LensConfig` enum :160; `ALL_LENS_CONFIGS` :196 (7 entries); `assert_roundtrip` :244; `spawn_with_closed_stdout` :299; `ROUNDTRIP_FIXTURES` :223 (2 entries); `ROUNDTRIP_FORMATS` :234 (2 entries); `FixtureEntry` type alias :209 (clippy::type_complexity fix) |
| `crates/base60-cli/tests/roundtrip.rs` | 28-cell single `#[test]` matrix | ✓ VERIFIED | 102 lines / 3492 B; exactly 1 `#[test]` (line 34 `roundtrip_matrix_byte_identical`); triple nested `for` over `ROUNDTRIP_FIXTURES × ALL_LENS_CONFIGS × ROUNDTRIP_FORMATS`; `mod common;` at :26; `use base60::Format;` at :28 |
| `crates/base60-cli/tests/fixtures.rs` | 4 #[test] (dump/analyze/decode/completions) | ✓ VERIFIED | 113 lines / 4202 B; 4 `#[test]` fns present matching plan spec; decode scoped to 2 fixtures (documented inline at :69-74 with REF-04 pointer) |
| `crates/base60-cli/tests/cli.rs` | 9+ #[test] covering edges incl. decoder pin | ✓ VERIFIED | 167 lines / 6512 B; exactly 9 `#[test]` fns; decoder pin at :156-167 asserts `predicates::str::contains("99").and(contains("invalid"))` on stderr + `.failure()` |
| `crates/xtask/tests/spawn_discipline.rs` | Static gate flagging raw Command::cargo_bin outside common/ | ✓ VERIFIED | 86 lines / 2921 B; WALK_ROOT="../base60-cli/tests"; EXEMPT_DIR="common"; SPAWN_LITERAL="Command::cargo_bin"; line-based scanner with `//` comment filter; test exits 0 against current tree |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|------|--------|---------|
| `src/main.rs` | `src/lib.rs` | `fn main() -> anyhow::Result<()> { base60::run() }` | ✓ WIRED | `main.rs:12` literal match |
| `src/lib.rs` | `src/cli.rs` | `pub use cli::{Format, LensMode};` | ✓ WIRED | `lib.rs:28` exact literal |
| `Cargo.toml` | `src/lib.rs` | `[lib] name = "base60" path = "src/lib.rs"` | ✓ WIRED | `Cargo.toml:13-15` |
| `tests/roundtrip.rs` | `tests/common/mod.rs` | `mod common; use common::{ALL_LENS_CONFIGS, LensConfig, ROUNDTRIP_FIXTURES, ROUNDTRIP_FORMATS, assert_roundtrip, base60_cmd};` | ✓ WIRED | `roundtrip.rs:26,29-32` |
| `tests/roundtrip.rs` | `base60::Format` | `use base60::Format;` | ✓ WIRED | `roundtrip.rs:28`; iterates via `ROUNDTRIP_FORMATS` (2 variants) |
| `tests/fixtures.rs` | `tests/common/mod.rs` | `use common::{FixtureEntry, base60_cmd, fixtures};` | ✓ WIRED | `fixtures.rs:12` |
| `tests/cli.rs` | `tests/common/mod.rs` | `use common::{base60_cmd, fixtures, spawn_with_closed_stdout};` | ✓ WIRED | `cli.rs:13` |
| `tests/cli.rs` | `decode.rs:103-109` (error format) | stderr substring assertion `"99"` + `"invalid"` | ✓ WIRED | `cli.rs:166`; manually reproduced via `printf '00000000  00:00:00:00:00:00:00:00:00:00:99 ...' \| base60 decode` → stderr `"Error: line 1: invalid base-60 digit 99 at pair 11"` (exit=1) |
| `xtask/tests/spawn_discipline.rs` | `crates/base60-cli/tests/` | `WalkDir::new("../base60-cli/tests")` with `common` exemption | ✓ WIRED | spawn_discipline.rs:17,39,46-52 |
| `tests/common/mod.rs` | `base60` binary | `Command::cargo_bin("base60")` | ✓ WIRED | mod.rs:39; sole sanctioned call-site |

### Data-Flow Trace (Level 4)

Not directly applicable — this phase ships test infrastructure, not a rendering pipeline. Behavioural coverage handled in Spot-Checks below.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 28-cell roundtrip matrix passes | `cargo test -p base60 --test roundtrip --locked` | `1 passed` in 0.14s | ✓ PASS |
| CLI edge tests pass | `cargo test -p base60 --test cli --locked` | `9 passed` | ✓ PASS |
| Fixture subcommand tests pass | `cargo test -p base60 --test fixtures --locked` | `4 passed` | ✓ PASS |
| Spawn-discipline gate passes | `cargo test -p xtask --test spawn_discipline --locked` | `1 passed` | ✓ PASS |
| Phase 2 env-discipline gate still green | `cargo test -p xtask --test env_discipline --locked` | `1 passed` | ✓ PASS |
| Full workspace test suite green | `cargo test --workspace --all-targets --locked` | 182 passed, 0 failed | ✓ PASS |
| Clippy pedantic+nursery+cargo clean | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0, zero warnings | ✓ PASS |
| Rustfmt check clean | `cargo fmt --all --check` | exit 0, zero diffs | ✓ PASS |
| Rustdoc clean (`-D warnings`) | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` | exit 0 | ✓ PASS |
| Binary produces a dump on stdin | `printf 'test1234' \| base60 --color=never --format=plain` | `00000000  13:52:15:26:33:06:19:29:24:08:20  \|test1234\|` | ✓ PASS |
| Decoder error format matches pin | `printf '...00:00:00:99 ...' \| base60 decode` | stderr: `Error: line 1: invalid base-60 digit 99 at pair 11`, exit=1 | ✓ PASS |
| `base60 --help` works after shim refactor | `target/debug/base60 --help` | Usage string + 3 subcommands listed | ✓ PASS |
| `git ls-files crates/base60-cli/tests/` has no binary fixtures | size audit | 4 `.rs` files, largest 13184 B (common/mod.rs — Rust source) | ✓ PASS |
| `grep -rn "Command::cargo_bin" tests/` outside common | grep | only `common/mod.rs:5` (comment) + `common/mod.rs:39` (call) | ✓ PASS |

All 14 spot-checks passed.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| TEST-01 | 03-02-PLAN.md | Fixture-driven roundtrip matrix — asserts byte-identical recovery for every `LensMode × FormatMode × ColorMode` against minimum corpus | ✓ SATISFIED (narrowed scope) | `tests/roundtrip.rs` exercises 28 cells byte-identically. Per user decision + ROADMAP SC1 update (line 54) + REQUIREMENTS REF-04 (line 19), the remaining 112 cells are deferred to Phase 4 REF-04 (JSON/HTML decode + length-preserving decode). REQUIREMENTS.md line 119 explicitly maps this ordering: "TEST-01 (Phase 3) precedes REF-03 (Phase 4) — roundtrip safety net". |
| TEST-03 | 03-02-PLAN.md, 03-03-PLAN.md | `assert_cmd`-driven integration tests covering dump/analyze/decode/completions incl. stdin piping + broken-pipe behaviour | ✓ SATISFIED | `tests/fixtures.rs` covers all 4 subcommands against 5 fixtures; `tests/cli.rs` covers stdin, BrokenPipe exit-0, color precedence, --skip/--length clamps, decoder-error pin. Spawn-discipline gate ensures hermetic env across tests. |

**Orphaned requirements:** None. REQUIREMENTS.md Phase 3 maps exactly TEST-01 + TEST-03; both covered by the plans.

### Anti-Patterns Found

Scanned `crates/base60-cli/src/{lib,main,cli}.rs`, `crates/base60-cli/tests/{common/mod,roundtrip,fixtures,cli}.rs`, `crates/xtask/tests/spawn_discipline.rs`.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `tests/fixtures.rs` | 41 | `let _ = label;` — formal unused-binding suppressor | ℹ️ Info | REVIEW.md IN-05 flagged the same; no functional impact. Harmless idiom. |
| `tests/roundtrip.rs` | 91 | `let _ = cell_start;` — silences unused-when-not-debug | ℹ️ Info | REVIEW.md IN-04 flagged a cleaner `#[cfg(debug_assertions)]` wrap; no functional impact. |
| `tests/roundtrip.rs` | 94-101 | `fmt_value(Format) -> &'static str` duplicates clap's canonical mapping | ⚠️ Warning | REVIEW.md WR-01 — advisory only. Future `Format` variant addition would require manual sync. Accepted as-is by code review. |

No TODO/FIXME/XXX/HACK/PLACEHOLDER in shipped files. No `return null`, `return {}`, `return []`, empty handlers, or hardcoded empty rendering paths. No `console.log` / stub implementations. No unimplemented!/todo!/unreachable! macros. Pre-existing `unsafe { std::env::{remove,set}_var(...) }` in `lib.rs:198/206/214/217` is documented with SAFETY comments and tagged `#[serial(env)]` — not new to this phase.

### Deferred Items

Explicitly tracked in ROADMAP.md Phase 3 SC1 (line 54) and REQUIREMENTS.md REF-04 (line 19):

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | Remaining 112 matrix cells: `hello_world`, `minimal_png`, `minimal_zip` (non-8-aligned fixtures) | Phase 4 (REF-04) | ROADMAP SC1 line 54: "deferred to REF-04 (length-preserving `decode` + JSON/HTML decode paths)"; REQUIREMENTS.md REF-04 line 19: "Once shipped, restore the full `LensMode × {ansi, plain, json, html} × 5 fixtures = 140 cells` matrix in `tests/roundtrip.rs` (flip `ROUNDTRIP_FIXTURES` to `ALL_FIXTURES`, `ROUNDTRIP_FORMATS` to `Format::ALL`)" |
| 2 | JSON/HTML decode paths | Phase 4 (REF-04) | REQUIREMENTS.md REF-04 line 19: "(b) `decode_from_json` reading `\"digits\":` arrays, (c) `decode_from_html` stripping tags" |
| 3 | Length-preserving decode | Phase 4 (REF-04) | REQUIREMENTS.md REF-04 line 19: "(a) additive `# length=N` metadata line (or equivalent) to `dump` so `decode` can truncate padding" |
| 4 | Decode-side BrokenPipe test | Not scheduled (Claude's Discretion D-13) | Plan 03-03 explicitly excluded; `run_decode` shares the BrokenPipe absorption shape with `run_view`/`run_analyze` and is already covered implicitly by the decoder pin's clean `.failure()` exit path |

### Human Verification Required

None. All automated spot-checks passed; no UX/visual/real-time behaviour requiring human judgement (this phase ships test infrastructure, not user-visible features). Cross-OS CI matrix validation (Ubuntu/macOS/Windows × rustc 1.95/stable/beta) is the responsibility of the existing `.github/workflows/ci.yml` — not re-validated here because the local Linux run already passes all six D-24 gates and Phase 2's CI matrix is unmodified.

### Gaps Summary

Ни одного gap не обнаружено. Все 9 must-haves прошли трёхуровневую проверку (exists / substantive / wired), data-flow trace для test-кода заменена behavioural spot-checks, а анти-паттерны на отгружаемых файлах ограничиваются двумя info-пометками из REVIEW.md (`let _ = …` идиомы) и одним advisory warning (WR-01: дубликат `fmt_value`, принятый код-ревью как приемлемый trade-off до появления пятого варианта `Format`). Полный workspace gate (test + clippy + fmt + doc + 2 xtask gates + 3 интеграционных test targets) зелёный после единственного atomic commit per plan (`42f4f1e`, `dece631` narrowing, `e93dee6`). Фаза закрывает TEST-01 (в суженном scope) и TEST-03 полностью; отложенные 112 ячеек матрицы явно привязаны к REF-04 в REQUIREMENTS.md и ROADMAP SC1 — планировщик мilestone-а корректно перенёс scope вперёд без потери трассируемости.

**Готов к `update_roadmap` (отметить TEST-01 и TEST-03 completed в REQUIREMENTS.md) и переходу к Phase 4 (REF-03 tightening + REF-04 length-preserving / JSON/HTML decode + TEST-05 coverage gaps).**

---

_Verified: 2026-04-24_
_Verifier: Claude (gsd-verifier)_
