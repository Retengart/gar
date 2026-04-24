# Phase 3: Roundtrip Matrix + Fixture Integration - Context

**Gathered:** 2026-04-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Create the first integration-test surface under `crates/base60-cli/tests/` and assert the Core Value (`base60 FILE | base60 decode` is byte-identical) exhaustively across every `LensMode × Format` cell for five fixed fixtures. Add `assert_cmd`-driven coverage of each subcommand (`dump` / `analyze` / `decode` / `completions`) including stdin piping and `BrokenPipe` behaviour. Ship the spawn-discipline gate that forbids any direct `Command::cargo_bin` outside `tests/common/`.

Requirements: **TEST-01**, **TEST-03**.

**In scope:**
- Thin `[lib]` target added to `base60-cli` so `tests/` can iterate dispatch tables (side-effect of enabling the matrix; scope-creep-adjacent but necessary).
- `Format::ALL` dispatch table + exhaustiveness unit test (mirrors REF-02's `LensMode::ALL` pattern).
- `crates/base60-cli/tests/roundtrip.rs`, `tests/fixtures.rs`, `tests/cli.rs`, `tests/common/mod.rs`.
- `crates/xtask/tests/spawn_discipline.rs` extending the Phase 2 gate crate.
- `assert_cmd = "2"` + `predicates = "3"` as CLI dev-deps.

**Not in scope:**
- `decode::parse_run` contract tightening → Phase 4 (REF-03), gated by this phase's matrix.
- `reader::load_file` / `reader::load_stdin` / TUI `TestBackend` / `persist::state_base_dir` tests → Phase 4 (TEST-05).
- Fuzz targets → Phase 5 (TEST-02).
- Criterion benches → Phase 5 (PERF-06).
- `tempfile` dev-dep → Phase 4 (needed there for mmap + `XDG_STATE_HOME` tests).

</domain>

<decisions>
## Implementation Decisions

### Matrix scope (TEST-01 SC1)

- **D-01:** Matrix shape = **7 lens-config rows × 4 formats × 5 fixtures = 140 cells**. Every cell forces `--color=never` so color is not a matrix axis.
- **D-02:** The 7 lens-config rows expand `LensMode::Time` across `TimeScale ∈ {Gar, Sec, Ms}`: `[None, Time(Gar), Time(Sec), Time(Ms), Angle, Tablet, Cuneiform]`. All other lenses use their default parameters.
- **D-03:** `TabletLens` runs with default `--purist=false`; `--purist` stays covered by the existing inline unit test in `lens.rs`. (Rationale: the flag affects lens text framing, not digit runs — roundtrip is invariant.)
- **D-04:** Fixture corpus is exactly the ROADMAP SC1 set, each generated in-test, each ≤ 4 KiB:
  - `minimal_elf()` — 64- or 128-byte hand-crafted ELF header (no full segments).
  - `minimal_png()` — magic + IHDR + IEND; ≤ 64 bytes.
  - `minimal_zip()` — 22-byte EOCD + one local file header + one 0-byte "file" entry.
  - `zero_fill_1kib()` — `vec![0u8; 1024]`.
  - `hello_world()` — `b"Hello, world!\n"` (14 bytes — exercises short-tail padding because 14 % 8 ≠ 0).
- **D-05:** Color auto-detect, `NO_COLOR` env, `--color={auto,always,never}`, and ANSI-interspersed decode are **not** matrix axes. They live as focused edge tests in `cli.rs` (see D-17).

### Library target + dispatch-table access (enables TEST-01 SC1)

- **D-06:** `crates/base60-cli/Cargo.toml` gains `[lib] name = "base60" path = "src/lib.rs"` alongside the existing `[[bin]]`. Keeps a single-install-target story — `cargo install` still produces only the `base60` binary because libraries aren't installed.
- **D-07:** `src/lib.rs` declares every current module as `mod X;` and re-exports a **minimal** public surface: `pub use cli::{LensMode, Format};` only (the two enums tests need). All other CLI internals stay `pub(crate)`. `cargo public-api` verification is an acceptable planner spot-check (optional).
- **D-08:** `src/main.rs` becomes a thin shim:
  ```rust
  fn main() -> anyhow::Result<()> { base60::run() }
  ```
  The current `main()` body lives as `pub fn run() -> anyhow::Result<()>` inside `lib.rs`. All existing `mod X;` declarations and `#[cfg(test)]` unit-test modules move with the code — in-source unit tests keep working unchanged.
- **D-09:** `LensMode::ALL` widens `pub(crate) → pub`. This is a deliberate revision of Phase 1 CONTEXT D-06 — the `pub(crate)` choice was correct when `base60-cli` was binary-only; adding a `[lib]` target makes `pub(crate)` too narrow for external tests. The exhaustiveness tests from Phase 1 (`all_contains_every_variant_in_cycle_order`, `all_methods_total_over_all`) continue to work as-is.
- **D-10:** Add `impl Format { pub const ALL: &[Self] = &[Format::Ansi, Format::Plain, Format::Json, Format::Html]; }` in `cli.rs`. Add one exhaustiveness test (`all_contains_every_format_variant`) alongside the LensMode tests. No `cycle`/`label` helpers on `Format` — iteration is its only use site (unlike `LensMode` which the TUI cycles through live).

### Test file layout (TEST-01, TEST-03 SC2–SC4)

- **D-11:** `tests/roundtrip.rs` contains **only** the 140-cell matrix (`roundtrip_matrix_byte_identical`). No subcommand happy path, no flag edges. Single responsibility.
- **D-12:** `tests/fixtures.rs` = per-subcommand happy path against each of the 5 fixtures:
  - `dump_produces_expected_prefix_per_fixture` — basic `base60 FIXTURE` invocation (spot-check); not a full matrix.
  - `analyze_summary_is_sane_per_fixture` — assert exit 0 + summary contains expected tokens (entropy value, byte count, region summary).
  - `decode_roundtrips_default_dump_per_fixture` — 5 sanity roundtrips with default flags (subset of matrix, but uses `base60_cmd()` path-flag rather than stdin path to cover the `--file` code path).
  - `completions_shells_all_succeed` — for each shell in `[bash, zsh, fish, elvish, powershell]`, assert exit 0 + non-empty stdout. Smoke only; don't parse the script.
- **D-13:** `tests/cli.rs` = non-fixture edges:
  - Stdin piping in (dump) and out (decode) with `.write_stdin(...)`.
  - `BrokenPipe` behaviour on `dump` by piping into a forced short reader. `dump` must exit 0 (current `main.rs:97-105` contract). Decode-side `BrokenPipe` is Claude's Discretion — planner picks based on observed current behaviour.
  - `NO_COLOR=1` env via `.env("NO_COLOR","1")` + `--color=auto` produces no ANSI escapes.
  - `--color=always` forces ANSI even when stdout is piped (non-TTY child).
  - `--color=never` suppresses ANSI even when `CLICOLOR_FORCE=1`.
  - `--skip` / `--length` clamping: zero-skip, skip-past-end, length-beyond-end.
  - Decoder error message: invalid digit `99` still produces an error whose message contains `"99"` — **this test pins the current error-message contract so Phase 4's REF-03 cannot drift it silently** (Pitfall 8).
- **D-14:** `tests/common/mod.rs` co-locates three concerns as a single file:
  1. **`pub fn base60_cmd() -> assert_cmd::Command`** — the only spawner. Does `.env_clear()`, restores `PATH` (and Windows `SystemRoot`, `USERPROFILE` only if set) to avoid the assert_cmd Windows caveat (Pitfall 12 / rust#37519), and does NOT pre-set `--color=never` (callers add it explicitly, keeping the grep for `--color=...` visible in every test).
  2. **Fixture factories** — `minimal_elf`/`minimal_png`/`minimal_zip`/`zero_fill_1kib`/`hello_world`. Each returns `Vec<u8>` ≤ 4 KiB. Build-time constants where feasible (e.g., `static HELLO_WORLD: &[u8] = b"Hello, world!\n";`).
  3. **Assertion helpers** — `assert_roundtrip(orig: &[u8], decoded: &[u8], cell_label: &str)` prints the cell identity + first-divergence index + ±8-byte hex window, then `assert_eq!` — shared failure shape between `roundtrip.rs` and `fixtures.rs`.
- **D-15:** `tests/common/mod.rs` also exports the matrix-iteration enum:
  ```rust
  #[derive(Copy, Clone, Debug)]
  pub enum LensConfig {
      None,
      Time(base60::cli::TimeScale),  // or whatever TimeScale needs re-exporting
      Angle,
      Tablet,
      Cuneiform,
  }
  impl LensConfig {
      pub fn cli_args(self) -> Vec<&'static str> { /* --lens=... [--time-scale=...] */ }
      pub fn label(self) -> &'static str { /* for diagnostics */ }
  }
  pub const ALL_LENS_CONFIGS: &[LensConfig] = &[/* 7 entries */];
  ```
  `TimeScale` may need re-exporting via `lib.rs` to keep D-07's minimal surface honest — planner's choice whether to re-export it too or keep the enum literal-based.

### Spawn-discipline gate (TEST-03 SC4)

- **D-16:** Extend Phase 2's `crates/xtask` crate with `tests/spawn_discipline.rs`. Reuses the existing `walkdir = "2"` dev-dep. Same line-based parser style as Phase 2's `env_discipline.rs` — no `syn`, no AST. Pattern: scan every `.rs` file under `crates/base60-cli/tests/` **excluding `tests/common/`**; fail on any line matching the regex-free literal `Command::cargo_bin` (catches both `assert_cmd::Command::cargo_bin` and raw `std::process::Command::cargo_bin` — neither is valid outside `common/`). Planner picks between a walk-based substring check and a more robust module-path check; either satisfies the invariant.
- **D-17:** On failure the gate emits `{file}:{line}: raw Command::cargo_bin outside tests/common/ — use base60_cmd() from tests/common/mod.rs`. Actionable message by convention.

### Matrix expression + diagnostics (TEST-01 SC1)

- **D-18:** Matrix is a single `#[test] fn roundtrip_matrix_byte_identical()` with nested loops:
  ```rust
  for fixture in &ALL_FIXTURES {          // 5
      for lens in ALL_LENS_CONFIGS {      // 7
          for fmt in Format::ALL {        // 4
              one_cell(fixture, *lens, *fmt);
          }
      }
  }
  ```
  One `#[test]` line in libtest output. Cell identity on failure. Compile-time cost stays minimal.
- **D-19:** Each cell spawns `base60` twice:
  1. `base60_cmd().args(["--color=never", &lens.cli_args()..., "--format=", fmt]).write_stdin(fixture_bytes).assert().success()` — capture stdout.
  2. `base60_cmd().args(["decode"]).write_stdin(dump_stdout).assert().success()` — capture stdout as decoded bytes.
  Compare `decoded_bytes == fixture_bytes`. No `tempfile` needed — stdin-piped throughout.
- **D-20:** Assertion helper (`assert_roundtrip` from D-14.3) prints on failure:
  ```
  cell: lens=Time(Sec) fmt=Html fixture=minimal_png
  original_len=57 decoded_len=64
  first diverge at byte 56
  original  ±8: ... 49 45 4e 44 ae 42 60 82 | <eof>
  decoded   ±8: ... 49 45 4e 44 ae 42 60 82 | 00 00 00 00 00 00 00
  ```
  Planner decides hex-window formatting style; above is indicative.
- **D-21:** Per-cell walltime budget target: **< 200 ms on Ubuntu, < 500 ms on Windows**. Aggregate budget: ~30 s per CI matrix cell (acceptable; existing `test` job already runs in ~2 min per matrix cell). If the aggregate crosses 60 s on Windows, the planner reduces the matrix by dropping the two minimum-entropy fixtures (`zero_fill_1kib`, `hello_world`) from the `Html` / `Json` cells — document the trim.

### Dev-deps

- **D-22:** Add to `crates/base60-cli/Cargo.toml [dev-dependencies]`:
  - `assert_cmd = "2"` (latest 2.x; pin to caret).
  - `predicates = "3"` (latest 3.x; pin to caret).
  - `serial_test = { version = "3", default-features = false }` is already present from Phase 2 — no change.
  - **No `tempfile`.** All Phase 3 tests pipe via stdin. Phase 4 adds `tempfile = "3"` when `reader::load_file` / TUI `TestBackend` / persist tests need it.

### Commit granularity

- **D-23:** Three commits, in order — matches Phase 1/2 style (one atomic unit per distinct concern):
  1. `refactor(cli): add thin lib target + Format::ALL dispatch table` — adds `lib.rs`, shrinks `main.rs` to shim, widens `LensMode::ALL` to `pub`, adds `Format::ALL` + exhaustiveness test. Touches `cli.rs`, `main.rs`, `Cargo.toml` ([lib] entry). **No new tests yet**; all v1+v2 tests still green.
  2. `test(cli): roundtrip matrix across LensMode × Format × fixtures [TEST-01]` — adds `tests/common/mod.rs`, `tests/roundtrip.rs`, and the `assert_cmd`/`predicates` dev-deps. The xtask spawn-discipline gate ships here too (prevents raw-spawn from the first moment tests/ exists).
  3. `test(cli): fixture-driven subcommand + edge coverage [TEST-03]` — adds `tests/fixtures.rs` and `tests/cli.rs`. Uses the already-shipped `common/mod.rs`.
- **D-24:** Each commit must pass `cargo test --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings` before the next starts. No "WIP" or broken-intermediate state.

### Claude's Discretion

- Exact byte sequences for `minimal_elf`/`minimal_png`/`minimal_zip` — planner picks (the 128-byte ELF header skeleton, 45-byte PNG, 22-byte ZIP EOCD shapes are cited in `PITFALLS.md §7`). Must compile in-test as a `const` or `Vec::from([...])` — do not `include_bytes!` a checked-in file (Pitfall 7).
- Whether `LensConfig::cli_args` returns `Vec<&'static str>` or `[&'static str; N]` — planner picks.
- Hex-window formatting on failure diagnostics.
- Decode-side `BrokenPipe` test assertion shape (current contract is observable from source; planner reads `decode_stream` + writer handling to pin it).
- Whether `tests/common/mod.rs` becomes `tests/common.rs` + `tests/common/` dir (Rust 2018+ style) or stays `tests/common/mod.rs` — both idiomatic.
- Whether `TimeScale` is re-exported at `lib.rs` or tests literal-embed the variant — planner's call, consistent with D-07 narrow-surface intent.
- Invocation flag order in cells — conventional `base60 --color=never --lens=... --format=... < fixture_stdin` — planner's call.

### Folded Todos

(None — `gsd-sdk query todo.match-phase 3` returned 0 matches.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level decisions

- `.planning/PROJECT.md` — Key Decisions row on integration-test crate placement (`crates/base60-cli/tests/` is the integration boundary); the `base60-core` zero-dep invariant applies to `[dependencies]`, not `[dev-dependencies]`.
- `.planning/REQUIREMENTS.md` — **TEST-01** (line 22) and **TEST-03** (line 24) specifications. Core Value statement (line 4) is the invariant this phase verifies.
- `.planning/ROADMAP.md` — Phase 3 Goal + 4 Success Criteria (lines 49-59). Phase dependency graph (line 117) — TEST-01 precedes REF-03.

### Prior-phase context (precedents adopted here)

- `.planning/phases/01-refactor-foundations/01-CONTEXT.md` — D-06..D-09: `LensMode::ALL` pattern and its exhaustiveness tests; D-09 is revised in this phase (`pub(crate) → pub`). The `Format::ALL` addition in Phase 3 (D-10 above) mirrors this precedent.
- `.planning/phases/02-env-test-serialisation/02-CONTEXT.md` — D-07..D-13: xtask gate pattern, line-based parser style, `walkdir` dev-dep, actionable diagnostic messages. Phase 3's spawn-discipline gate (D-16 above) extends this same crate.

### Pitfall remediations this phase consumes

- `.planning/research/PITFALLS.md` §"Pitfall 7" — Fixture corpus bloat. Drives D-04 (all fixtures ≤ 4 KiB, generated in-test).
- `.planning/research/PITFALLS.md` §"Pitfall 10" — `HashMap` iteration non-determinism. The matrix iterator uses `&[...]` slice constants (D-15), not `HashMap`; assertion helper iterates the fixture list in declaration order.
- `.planning/research/PITFALLS.md` §"Pitfall 12" — `assert_cmd` color detection. Drives D-14.1 (`.env_clear()` + PATH restore + explicit `--color=...` per test).
- `.planning/research/PITFALLS.md` §"Pitfall 8" — `parse_run` refactor error-semantics drift. Phase 3 is the safety net: D-13 pins the decoder error-message contract (`err.to_string().contains("99")`) so Phase 4 REF-03 cannot silently drift it.
- `.planning/research/PITFALLS.md` §"Looks Done But Isn't" — TEST-phase completion checklist. Phase 3 must satisfy rows 1, 6, 8 directly; row 2 (fuzz) is Phase 5; rows 3, 4, 7 (env-mutation grep, 9-cell CI green, reader/TUI coverage) are Phase 2 / Phase 4.

### Codebase intelligence

- `.planning/codebase/TESTING.md` — current 164-test inline-module idiom. Phase 3 is the **first** integration-test directory; it does not replace inline tests.
- `.planning/codebase/CONVENTIONS.md` — `pub(crate)` default, `#[must_use]`, doc comments on every `pub(crate)`-or-above item, clippy `pedantic + nursery + cargo -D warnings` applies to the new `tests/` module too.
- `.planning/codebase/STRUCTURE.md` — workspace layout; reference for where `[lib]` slots into `crates/base60-cli/Cargo.toml`.
- `.planning/codebase/INTEGRATIONS.md` — CI shape; the new tests land inside the existing `cargo test --workspace --all-targets --locked` step with zero CI YAML changes.

### Source files this phase edits or creates

**NEW:**
- `crates/base60-cli/src/lib.rs` — thin public surface (just `pub use cli::{LensMode, Format};`); `pub fn run()` hosts the current `main()` body.
- `crates/base60-cli/tests/common/mod.rs` — `base60_cmd()`, fixture factories, `LensConfig` enum + `ALL_LENS_CONFIGS`, assertion helpers.
- `crates/base60-cli/tests/roundtrip.rs` — single `roundtrip_matrix_byte_identical` test, 140 cells.
- `crates/base60-cli/tests/fixtures.rs` — per-subcommand happy path × 5 fixtures.
- `crates/base60-cli/tests/cli.rs` — stdin / BrokenPipe / color / --skip / --length / decoder error-message edges.
- `crates/xtask/tests/spawn_discipline.rs` — spawn-discipline gate.

**EDIT:**
- `crates/base60-cli/Cargo.toml` — add `[lib] name = "base60" path = "src/lib.rs"`; add `assert_cmd = "2"` + `predicates = "3"` under `[dev-dependencies]`.
- `crates/base60-cli/src/main.rs` — shrink to a one-line shim: `fn main() -> anyhow::Result<()> { base60::run() }`. All existing mod declarations and in-source unit tests move to `lib.rs` wholesale (no logic change).
- `crates/base60-cli/src/cli.rs` — widen `LensMode::ALL` from `pub(crate)` to `pub`; add `Format::ALL` + one exhaustiveness test.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `LensMode::ALL` already exists (Phase 1) with `cycle` / `label` / `build_lens` + `persist::parse_lens` exhaustiveness tests. Phase 3 promotes visibility and adds `Format::ALL` on the same pattern — **no new runtime behaviour**.
- `decode::decode_stream<R: BufRead, W: Write>` already accepts any `BufRead` → `Vec<u8>` flows via `.write_stdin(...)` + `assert_cmd::Command::output()`. No wrapper needed.
- `main::main` already handles `BrokenPipe` at `crates/base60-cli/src/main.rs:97-105` — Phase 3 tests this contract; does not change it.
- `#[cfg(test)] mod tests { use super::*; ... }` is the crate-wide unit-test convention. New integration tests under `tests/` are external — `use base60::{...};` instead of `use super::*;`.
- `serial_test = "3"` is already present from Phase 2 on both crates. Phase 3 does not add new env-mutating tests that require it — `.env("NO_COLOR","1")` on `assert_cmd::Command` affects the child process only, not the parent; xtask `env_discipline` gate won't flag it.

### Established Patterns

- Workspace-level `[workspace.lints]` (`pedantic + nursery + cargo`) automatically applies to the new `tests/` module via `[lints] workspace = true` in `base60-cli/Cargo.toml`. Every new public-ish item needs a doc comment; every numeric cast needs an explicit `#[allow(clippy::cast_*)]` or checked-arithmetic form.
- `#[derive(Debug)]` on every public type is contractual (`missing_debug_implementations = warn`). `LensConfig` from D-15 needs it.
- Fixture factories return owned `Vec<u8>` so tests can mutate freely without `clone` — matches the existing test-helper idiom (`persist::tests::sample`).
- Module-level docstrings use `//!` in one line — keep new files consistent.

### Integration Points

- `crates/base60-cli/Cargo.toml` currently has only `[[bin]]`. Adding `[lib]` does **not** break `cargo install --path crates/base60-cli` — only `[[bin]]` produces an install artefact.
- `.github/workflows/ci.yml` already runs `cargo test --workspace --all-targets --locked` on the 3×3 OS×rustc matrix. Phase 3's new `tests/` directory is picked up automatically — **zero CI YAML changes needed**.
- `main.rs`'s existing `#[cfg(test)] mod tests` block has 5 tests (all `#[serial(env)]` from Phase 2). These move to `lib.rs` together with the code they cover; annotations and their `SAFETY:` comments stay verbatim.
- `xtask` crate already exists from Phase 2 (`Cargo.toml`, `src/lib.rs`, `tests/env_discipline.rs`, `walkdir = "2"` dev-dep). Phase 3 adds one sibling `tests/spawn_discipline.rs`; no manifest changes needed.

### Constraints from existing CI

- `cargo fmt --all --check` — every new file must be rustfmt-clean.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` with pedantic+nursery+cargo — applies to tests too. Budget for `#[must_use]`, doc comments on `pub(crate)`+, and explicit allows on intentional casts.
- `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS: -D warnings` — `pub fn run`, `pub use cli::{LensMode, Format}`, and `Format::ALL` all need doc comments.
- Windows MSVC matrix cell: `assert_cmd` + `.env_clear()` requires preserving `SystemRoot` / `USERPROFILE` for `CreateProcess` to work. `base60_cmd()` must handle this (rust#37519).

</code_context>

<specifics>
## Specific Ideas

- `hello_world` fixture is `b"Hello, world!\n"` — 14 bytes. `14 % 8 == 6`, so it exercises the short-tail padding path that the matrix must handle correctly for every `(lens, format)` cell. If this fixture passes, the tail-handling contract is exercised uniformly.
- `base60_cmd()` sets `NO_COLOR` only when a test asks for it — the helper starts from a fully cleared env (minus `PATH`, Windows `SystemRoot`, `USERPROFILE`). Default colour behaviour follows `--color=...` which every caller provides explicitly.
- The spawn-discipline gate regex scans for the literal `Command::cargo_bin` substring. No regex engine dep; straight `line.contains("Command::cargo_bin")` is enough.
- `tests/common/mod.rs` is named `mod.rs` (not `common.rs`) — matches the `xtask/tests/` style from Phase 2 and the Rust-idiomatic test-helper convention.
- Roundtrip assertion message: `format!("cell lens={} fmt={:?} fixture={}: first diverge byte {}, orig±8={:02x?}, decoded±8={:02x?}", ...)` — on one line so `cargo test` output stays parseable.

</specifics>

<deferred>
## Deferred Ideas

- **`tempfile = "3"`** — Phase 4 (TEST-05) needs it for `reader::load_file` mmap fixture + TUI `TestBackend` `$XDG_STATE_HOME` redirect. Adding it now to Phase 3 is scope creep; all Phase 3 tests pipe via stdin.
- **`--purist` coverage in matrix** — already a pattern-extension (would add a single `TabletLens(purist=true)` row). Current inline unit test in `crates/base60-core/src/lens.rs` covers it. Revisit only if a TabletLens refactor in future phases changes digit-run behaviour.
- **Color-axis cube** — REQUIREMENTS TEST-01 literally says `LensMode × FormatMode × ColorMode`; ROADMAP SC1 narrowed to `LensMode × {ansi, plain, json, html}`. The color axis is fully covered in `cli.rs` edges (D-13). If a future regression shows a lens/color interaction matters, revisit by turning color into a matrix axis (140 → 420 cells).
- **Sub-fixture variants (non-8-aligned sizes beyond hello_world)** — 14-byte `hello_world` already covers the non-aligned short-tail path. If a future bug shows alignment-specific drift, add `{7, 9, 15, 17}`-byte fixtures.
- **`cargo public-api` diff check for lib.rs** — recommended verification by Pitfall 5 (for REF-01/core); for CLI's lib surface, it's optional Claude discretion. Planner may add a workspace-level `cargo xtask public-api-diff` in a future phase.
- **Snapshot tests (insta)** — explicitly out of scope per PROJECT.md and REQUIREMENTS §"Out of Scope". Roundtrip byte-identity is a stronger invariant than snapshot equality.
- **Proptest / property-based testing** — raised in Phase 2 discussion; remains deferred. Phase 5's fuzz targets will exercise adjacent surface on `parse_run` / `Pattern::from_str`.

</deferred>

---

*Phase: 03-roundtrip-matrix-fixture-integration*
*Context gathered: 2026-04-24*
