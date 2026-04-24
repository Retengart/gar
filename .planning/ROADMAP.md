# Roadmap: base60 v2 (Hardening Milestone)

**Created:** 2026-04-24
**Milestone:** v2 — hardening only (no user-facing feature surface)
**Granularity:** standard (7 phases)
**Coverage:** 16 / 16 v2 requirements mapped

## Phases

- [ ] **Phase 1: Refactor Foundations** — De-duplicate `be_u64` (CLI-local) and drive `LensMode` dispatch from one table.
- [ ] **Phase 2: Env-Test Serialisation** — Adopt `serial_test` with a single `env` key before any new env-mutating test lands.
- [ ] **Phase 3: Roundtrip Matrix + Fixture Integration** — Assert byte-identical dump↔decode across every `LensMode × Format` combination; fixture-driven `assert_cmd` coverage.
- [ ] **Phase 4: Tighten `parse_run` + Close Coverage Gaps** — REF-03 ships behind Phase 3's safety net; cover mmap/stdin/TUI-save paths.
- [ ] **Phase 5: Fuzz + Criterion Harnesses** — Weekly-runnable fuzz targets and advisory-only bench scaffolding — Phase 6's prerequisite.
- [ ] **Phase 6: Streaming + Performance Pass** — Each PERF-0X change ships with before/after numbers from Phase 5 benches.
- [ ] **Phase 7: CI Hardening** — Weekly scheduled fuzz job + enforced zero-dep invariant on `base60-core`.

## Phase Details

### Phase 1: Refactor Foundations
**Goal**: The contracts every downstream test, fuzz target, and bench will stand on are stabilised — one source of truth for chunk decoding, one table for lens dispatch.
**Depends on**: Nothing (foundation).
**Requirements**: REF-01, REF-02
**Success Criteria** (what must be TRUE):
  1. `grep -n 'fn be_u64' crates/base60-cli/src/*.rs` returns exactly one hit (at `crates/base60-cli/src/chunk.rs`); `dump.rs` and `format.rs` call it via `use super::chunk::be_u64`.
  2. `crates/base60-core/Cargo.toml` `[dependencies]` section remains empty — no new workspace dep leaked into the zero-dep library.
  3. A single `const ALL: &[LensMode]` (or equivalent hand-rolled table) in `cli.rs` drives every lens dispatch site; `build_lens` / `cycle` / `label` / `persist::parse_lens` all read from it, verified by a compile-time exhaustiveness test that iterates the table and panics on missing variants.
  4. Adding a hypothetical fifth `LensMode` variant compile-errors at exactly one site (the table); `cargo test --workspace --all-targets --locked` stays green.
**Plans**: 2 plans
- [ ] 01-dedupe-be-u64-PLAN.md — de-duplicate `be_u64` into CLI-local `chunk.rs`; `dump.rs`/`format.rs` import from it [REF-01]
- [ ] 02-lens-mode-dispatch-table-PLAN.md — add `LensMode::ALL` + exhaustiveness tests; promote `persist::parse_lens` to `pub(crate)` [REF-02]
**Parallel-safe with**: none (ships before all other phases).

### Phase 2: Env-Test Serialisation
**Goal**: The "don't run concurrently" convention around env-mutating tests is replaced by a single enforced `#[serial(env)]` key, so Phase 3/4 can safely add new env-touching coverage without reintroducing CI flakes.
**Depends on**: Nothing (parallel-safe with Phase 1).
**Requirements**: TEST-04
**Success Criteria** (what must be TRUE):
  1. Every test containing `env::set_var` / `env::remove_var` in `crates/` is tagged `#[serial(env)]` — verified by a repo-root grep-gate that fails CI if any env mutation appears outside a `serial(env)` scope.
  2. `serial_test = "3"` appears under `[dev-dependencies]` in `crates/base60-cli/Cargo.toml` with `default-features = false`; `base60-core/Cargo.toml` dev-deps add `serial_test` only if core's env tests remain in-tree.
  3. Running `cargo test --workspace --all-targets --locked -- --test-threads=8` succeeds ten times in a row on Ubuntu (no flakes from `NO_COLOR`/`NO_UNICODE`/`TERM` races).
  4. No test uses a per-variable key (`#[serial(no_color)]`, `#[serial(no_unicode)]`) — grep verifies the single shared key convention.
**Plans**: 3 plans
- [x] 02-01-workspace-prep-PLAN.md — add xtask workspace member + serial_test dev-dep on both crates [TEST-04]
- [x] 02-02-serial-env-annotations-PLAN.md — annotate 7 env-mutating tests with #[serial(env)] across main.rs/cuneiform.rs/lens.rs [TEST-04]
- [x] 02-03-env-discipline-gate-PLAN.md — xtask gate integration test + smoke-serial.sh + CI --test-threads=8 step [TEST-04]
**Parallel-safe with**: Phase 1 (disjoint files; REF-01/REF-02 touch `chunk.rs`/`cli.rs`, TEST-04 is a mechanical annotation pass).

### Phase 3: Roundtrip Matrix + Fixture Integration
**Goal**: The Core Value guarantee — every `base60 FILE | base60 decode` round-trips byte-identically — is asserted exhaustively across the `LensMode × FormatMode` product before any contract-tightening refactor runs. Fixture-driven `assert_cmd` tests cover dump / analyze / decode / completions entry points.
**Depends on**: Phase 1 (needs the `LensMode` dispatch table to enumerate variants), Phase 2 (any env-touching integration test uses `#[serial(env)]`).
**Requirements**: TEST-01, TEST-03
**Success Criteria** (what must be TRUE):
  1. `crates/base60-cli/tests/roundtrip.rs` iterates every `(LensMode × {ansi, plain, json, html})` cell — driven by Phase 1's dispatch table — and asserts `base60 FILE | base60 decode` is byte-identical to the original for each of: minimal ELF, minimal PNG, minimal ZIP, 1 KiB zero-fill, short hello-world. Every cell passes on Ubuntu/macOS/Windows × rustc 1.95/stable/beta.
  2. `crates/base60-cli/tests/fixtures.rs` and `crates/base60-cli/tests/cli.rs` exercise each subcommand (`dump`, `analyze`, `decode`, `completions`) including stdin piping and `BrokenPipe` short-reader behaviour via `assert_cmd` + `predicates`.
  3. All fixtures are generated in-test (`fn minimal_elf() -> Vec<u8>`, etc.) — `git ls-files | xargs stat -c '%s'` shows no tracked file over 8 KiB inside `tests/`.
  4. A shared `crates/base60-cli/tests/common/mod.rs` helper (`base60_cmd()` with `.env_clear()` + explicit `--color`) is the only way tests spawn the binary; grep verifies no raw `Command::cargo_bin` invocations outside `common/`.
**Plans**: 3 plans
- [x] 03-01-PLAN.md — thin `[lib]` target + `Format::ALL` dispatch table (prereq for matrix; widens `LensMode::ALL` to `pub`)
- [ ] 03-02-PLAN.md — 140-cell roundtrip matrix + `tests/common/mod.rs` + xtask spawn-discipline gate [TEST-01, TEST-03]
- [ ] 03-03-PLAN.md — per-subcommand fixtures + CLI edges (stdin/BrokenPipe/color/clamps/decoder pin) [TEST-03]
**Parallel-safe with**: Phase 5's bench scaffolding (disjoint new files under `benches/`).

### Phase 4: Tighten `parse_run` + Close Coverage Gaps
**Goal**: `decode::parse_run` accepts `&[u8; RUN_LEN]`, promotes its digit-check inside, and ships only after Phase 3's roundtrip matrix guarantees no silent error-semantics drift. Previously-untested paths (`reader::load_file` mmap, `reader::load_stdin`, TUI exit-with-save, `persist::state_base_dir`) gain direct coverage.
**Depends on**: Phase 3 (REF-03's safety net), Phase 2 (TEST-05's `state_base_dir` test is env-mutating — uses the `serial(env)` idiom).
**Requirements**: REF-03, TEST-05
**Success Criteria** (what must be TRUE):
  1. `decode::parse_run` signature reads `fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64>`; the digit-validity check lives inside the function; no caller constructs `parse_run` input without the length-type invariant. Roundtrip matrix from Phase 3 still passes byte-for-byte.
  2. New tests in `crates/base60-cli/tests/reader.rs` cover the mmap path (tempfile fixture), the stdin path (synthetic `BufRead`), and the file-open error path (nonexistent path returns the expected `io::ErrorKind`).
  3. `crates/base60-cli/tests/tui.rs` uses `ratatui::backend::TestBackend` + `tempfile::tempdir()` to redirect `$XDG_STATE_HOME`, drives the TUI to quit (`q`), and asserts the state file appears at the expected path with the expected `scroll` / `cursor` / `lens_mode` / `bookmarks` content.
  4. A `persist::state_base_dir` unit / integration test covers the `XDG_STATE_HOME` → `HOME` fallback ladder — tagged `#[serial(env)]` per Phase 2.
**Plans**: TBD
**Parallel-safe with**: Phase 5's fuzz + bench scaffolding (disjoint files; REF-03's new contract is exactly what TEST-02's fuzz target will consume, so Phase 5 can reference Phase 4's signature while scaffolding in parallel).

### Phase 5: Fuzz + Criterion Harnesses
**Goal**: Infrastructure only — a workspace-excluded `fuzz/` crate with two targets, and per-crate `benches/` directories with criterion scaffolding. Neither gates CI; both exist to make Phase 6's perf pass measurable and Phase 7's weekly fuzz job runnable.
**Depends on**: Phase 1 (fuzz target for `parse_run` uses REF-01's CLI-local `chunk::be_u64`; targets reach `parse_run`/`Pattern` via `#[cfg(fuzzing)] pub` hatch). Phase 4's tightened `parse_run` signature is ideal for fuzz consumption.
**Requirements**: TEST-02, PERF-06
**Success Criteria** (what must be TRUE):
  1. `fuzz/` exists at repo root, declared via `cargo fuzz init --fuzzing-workspace=true`, listed under root `Cargo.toml` `[workspace] exclude = ["fuzz"]`. Two fuzz targets compile and run: `parse_run` and `pattern_from_str`. Each uses the `let _ = ...` pattern (no `unwrap()` inside `fuzz_target!`).
  2. `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` completes without crash on Ubuntu + pinned nightly; corpus and artifacts directories are listed in `fuzz/.gitignore`.
  3. `crates/base60-core/benches/{convert,lens}.rs` and `crates/base60-cli/benches/{dump,decode,search}.rs` exist, each declared `harness = false`, configured with `Criterion::default().noise_threshold(0.05)`. `cargo bench --workspace --no-run --locked` compiles every bench on all three OSes.
  4. `crates/base60-cli/benches/README.md` documents advisory-only posture: benches run locally with `--save-baseline`, numbers pasted into PR descriptions, CI never gates on them.
  5. `#[cfg(fuzzing)] pub` hatch for `decode::parse_run` and `search::Pattern` is verified: non-fuzzing `cargo check --workspace` leaves the public API surface unchanged (`cargo public-api` or equivalent manual diff confirms no new `pub` item in `base60-core` or `base60-cli` outside the fuzz-gated hatch).
**Plans**: TBD
**Parallel-safe with**: Phase 3 (disjoint files — Phase 3 touches `tests/`, Phase 5 touches `fuzz/` and `benches/`). Phase 5 can also be parallelised with Phase 4 once Phase 1 + Phase 3 have merged.

### Phase 6: Streaming + Performance Pass
**Goal**: Each of PERF-01 through PERF-05 ships with before/after numbers from the matching Phase 5 bench, pasted into the PR description. Streaming stdin path never OOMs; TUI first frame draws within one tick regardless of input size; no performance-oriented allocation regression on the hot paths.
**Depends on**: Phase 5 (every perf PR compares against a baseline); PERF-04 (`render_to<W>`) ships before PERF-01 (streaming path consumes `render_to`); PERF-02 (async TUI analyze) prefers PERF-05 (online entropy) to land first so the background worker isn't doing unnecessary work.
**Requirements**: PERF-01, PERF-02, PERF-03, PERF-04, PERF-05
**Success Criteria** (what must be TRUE):
  1. `base60 < /dev/zero | head -c 1G > /dev/null` completes on Linux with bounded peak RSS (documented smoke test + pipe-driven integration test that feeds >100 MB through a synthetic `BufRead` without materialising a full `Vec<u8>`).
  2. `search::find_all` dispatches 1-byte needles to `memchr::memchr_iter` and ≥2-byte needles to `memchr::memmem::Finder`; the `search` criterion bench shows no regression on the `(zero-fill haystack, 1-byte needle)` cell vs. the pre-change baseline (avoids Pitfall 4).
  3. `Lens::render_to<W: Write>` default method exists on the `base60-core::lens::Lens` trait; `CuneiformLens` and `TabletLens` override it; the `dump` and `lens` criterion benches show alloc-reduction wins pasted in the PR. Per-lens UTF-8-validity unit tests pass for 1000 random `u64` inputs.
  4. TUI analyze runs off the render thread via `Arc<Mutex<Option<Analysis>>>`; first frame draws before analysis completes; semantic-jump keys show an "analysing…" status when `analysis` is still `None`. A `TestBackend` integration test asserts the first-frame render happens without blocking on a synthetic 100 MiB fixture.
  5. `analyze::entropy_windows` no longer materialises a `Vec<f32>` for the sparkline — an online min/max/mean accumulator ships, verified by inspection + the `analyze` bench showing bounded memory use on a 1 GiB synthetic input.
**Plans**: TBD
**Parallel-safe within phase**: PERF-03 ↔ PERF-04 (disjoint files: `search.rs` vs. `lens.rs` + render call sites). Serial dependencies: PERF-01 after PERF-04; PERF-05 before PERF-02.

### Phase 7: CI Hardening
**Goal**: The `base60-core` zero-dep selling point and the fuzz-drift window are both enforced by CI — not by convention, not by reviewer vigilance.
**Depends on**: Phase 5 (fuzz crate must exist before CI can schedule it), Phase 1 (`base60-core` zero-dep check must run after REF-01 proves the CLI-local `chunk` placement, not while core is in flux).
**Requirements**: CI-02, CI-03
**Success Criteria** (what must be TRUE):
  1. A `.github/workflows/fuzz.yml` job on a weekly `schedule:` runs `cargo +nightly fuzz run parse_run -- -max_total_time=240` (and the same for `pattern_from_str`) on `ubuntu-latest` only, with `timeout-minutes: 5`, pinned nightly, `actions/upload-artifact@v4` on failure for crashes. Job is non-gating.
  2. The existing `ci.yml` gains a `zero-dep-core` step that fails if `cargo metadata --manifest-path crates/base60-core/Cargo.toml --no-deps` reports any `[dependencies]` entry, or if `grep -P '^\[dependencies\]' crates/base60-core/Cargo.toml` is followed by any non-empty line before the next section header.
  3. `cargo test --workspace --all-targets --locked` remains green on the 3 OS × 3 rustc matrix; `cargo doc --workspace --no-deps --locked` stays green; no regression on any existing CI cell.
  4. A `benches-compile` CI step runs `cargo bench --workspace --no-run --locked` on Ubuntu to catch bench-code rot, without actually running the benches.
**Plans**: TBD
**Parallel-safe with**: nothing (final phase; depends on everything prior).

## Phase Dependency Graph

```
Phase 1 ──┬──> Phase 3 ──> Phase 4 ──┐
          │                          ├──> Phase 5 ──> Phase 6 ──> Phase 7
          └──> Phase 5 ──────────────┘
Phase 2 ──> Phase 3 / Phase 4 (provides #[serial(env)] idiom)
```

**Critical ordering constraints** (from research):
- **TEST-01 before REF-03**: Phase 3 before Phase 4. Phase 3's roundtrip matrix is the safety net for Phase 4's `parse_run` contract change (Pitfall 8).
- **PERF-06 before PERF-01..05**: Phase 5 before Phase 6. Every perf PR in Phase 6 compares against a Phase 5 bench baseline.
- **TEST-04 before new env-mutating tests**: Phase 2 lands the idiom; Phase 3 / Phase 4's new env-touching tests adopt it immediately.
- **REF-02 enables TEST-01 matrix enumeration**: Phase 1's dispatch table lets Phase 3 iterate every variant automatically.

**Parallel-safe pairs** (disjoint modules, no edge):
- Phase 1 ↔ Phase 2: REF-01/REF-02 edit `chunk.rs`/`cli.rs`/`lens.rs`; TEST-04 annotates existing env tests. No file overlap.
- Phase 3 ↔ Phase 5: Phase 3 touches `tests/`; Phase 5 touches `fuzz/` and `benches/`. No file overlap.
- Within Phase 6: PERF-03 (`search.rs`) ↔ PERF-04 (`lens.rs`) ↔ PERF-02 (`tui.rs`) can ship as three independent PRs.

## Coverage

| Requirement | Phase | Notes |
|-------------|-------|-------|
| REF-01 | Phase 1 | CLI-local `crates/base60-cli/src/chunk.rs` — PROJECT.md Key Decision row 5 |
| REF-02 | Phase 1 | Hand-rolled dispatch table in CLI — PROJECT.md Key Decision row 6 |
| REF-03 | Phase 4 | Gated by Phase 3 roundtrip matrix (Pitfall 8) |
| TEST-01 | Phase 3 | Uses Phase 1's dispatch table to enumerate variants |
| TEST-02 | Phase 5 | `#[cfg(fuzzing)] pub` hatch — PROJECT.md Key Decision row 7 |
| TEST-03 | Phase 3 | In-test fixture generation (Pitfall 7) |
| TEST-04 | Phase 2 | Precedes every new env-touching test (Pitfall 1) |
| TEST-05 | Phase 4 | `persist::state_base_dir` test adopts `#[serial(env)]` from Phase 2 |
| PERF-01 | Phase 6 | Depends on PERF-04 (`render_to<W>`) for the streaming alloc win |
| PERF-02 | Phase 6 | Ships after PERF-05 to reduce background work |
| PERF-03 | Phase 6 | 1-byte needle dispatched to `memchr_iter` (Pitfall 4) |
| PERF-04 | Phase 6 | UTF-8-validity test per lens (Pitfall 13) |
| PERF-05 | Phase 6 | Online accumulator; splits `analyze::analyze` |
| PERF-06 | Phase 5 | Advisory-only posture — PROJECT.md Key Decision row 8 |
| CI-02 | Phase 7 | Weekly schedule, Ubuntu+nightly only — PROJECT.md Key Decision row 9 |
| CI-03 | Phase 7 | `cargo metadata` zero-dep check |

**Coverage:** 16 / 16 v2 requirements mapped ✓ · No orphans · No duplicates

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Refactor Foundations | 0 / TBD | Not started | — |
| 2. Env-Test Serialisation | 0 / TBD | Not started | — |
| 3. Roundtrip Matrix + Fixture Integration | 0 / TBD | Not started | — |
| 4. Tighten parse_run + Close Coverage Gaps | 0 / TBD | Not started | — |
| 5. Fuzz + Criterion Harnesses | 0 / TBD | Not started | — |
| 6. Streaming + Performance Pass | 0 / TBD | Not started | — |
| 7. CI Hardening | 0 / TBD | Not started | — |

---

*Roadmap created: 2026-04-24*
*Research source: `.planning/research/{SUMMARY,STACK,ARCHITECTURE,PITFALLS,FEATURES}.md`*
*Next: `/gsd-plan-phase 1`*
