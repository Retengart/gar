# Project Research Summary

**Project:** base60 v2 — hardening milestone
**Domain:** Rust CLI hardening (integration tests + fuzz, streaming perf, refactor consolidation)
**Researched:** 2026-04-24
**Confidence:** HIGH

## Executive Summary

base60 v1 ships a complete binary-viewer CLI (11-digit sexagesimal dump, four lenses, entropy `analyze`, ratatui TUI, JSON/HTML emitters, roundtrip `decode`). v2 is strictly a HARDENING milestone — no new user-facing features. Three themes from `PROJECT.md`: (T) integration tests + fuzz, (P) streaming/perf pass, (R) refactor consolidation. The research converges on a low-novelty toolchain (assert_cmd + cargo-fuzz + criterion + memchr::memmem + a LensMode single-table dispatch) against a well-understood v1 codebase with clearly catalogued debt.

The opinionated conclusion: do **refactors first** (REF-01/02/03) so downstream work stands on stable contracts, then land the **test/bench safety nets** (TEST-01..05 + PERF-06) in parallel, and only then ship the **perf changes** (PERF-01..05), each gated by its matching criterion bench. Two CONSTRAINT-driven course-corrections override earlier convenience picks: (1) keep `be_u64` **CLI-local**, not in `base60-core`, to protect the zero-dep-core contract; (2) **hand-roll** the `LensMode` dispatch table instead of adding `strum` to core. Both follow from PROJECT.md line 110: *"base60-core must keep zero external dependencies — its selling point."*

Key risks are well-understood and each has a named mitigation: streaming stdin re-introducing OOM via `read_to_end` (guard with a >100 MB piped integration test), criterion noise on shared GHA runners (run benches **advisory-only, not CI-gating**), fuzz false-positives from unwrap-on-error (`let _ = ...` pattern + release-mode verification), and silent `parse_run` error-semantics drift (order TEST-01 before REF-03). With these guards in place, v2 is a sequencing problem, not a research problem.

## Key Findings

### Recommended Stack

Eight new dev-dependencies, **zero new runtime dependencies**, and `base60-core` stays zero-dep. Three of the "new" crates (`memchr`, `strum`, `strum_macros`) are already in `Cargo.lock` transitively — promoting to direct deps costs zero extra compile time. Full rationale and version pins in `.planning/research/STACK.md`.

**Core technologies:**
- `assert_cmd` 2.2 + `predicates` 3.1 + `tempfile` 3.27: CLI integration tests — de facto standard, matches `hexyl`/`bat` conventions; auto-discovers the `base60` binary across workspace members
- `serial_test` 3.4: `#[serial(env)]` on env-touching tests — only mature option; replaces current "don't run concurrently" convention
- `cargo-fuzz` 0.13 + `libfuzzer-sys` 0.4: nightly fuzz harness in a workspace-excluded `fuzz/` crate — Linux+nightly only, never matrixed
- `criterion` 0.8: microbenchmarks with save/compare baselines — chosen over `divan` for stable 1.0+ release and baseline UX
- `memchr` 2.8: SIMD `memmem` for `search::find_all` — already transitive; promote to direct dep
- `strum` 0.27 (CLI only, **not core**): if adopted; preferred: hand-roll the dispatch table to preserve zero-dep-core
- Edition 2024, MSRV 1.95 — locked by v1; every new crate verified compatible

**Deliberate non-adoptions:** `proptest` (table tests already cover), `insta` (schema is byte-stable), `divan` (pre-1.0), `rstest` (conflicts with `for n in [...]` idiom), `cargo-tarpaulin` / `cargo-audit` / `iai-callgrind` (scoped out in FEATURES.md AF-01..09).

### Expected Features

v2 requirements are already enumerated in `PROJECT.md` Active. FEATURES.md confirms each maps to a peer-CLI table-stakes item or a differentiator, never a speculative addition.

**Must have (P1 — required for v2 to deserve the name):**
- TS-01 `tests/` integration crate (TEST-03 scaffold)
- TS-02 dump↔decode roundtrip matrix across `{lens} × {format}` (TEST-01)
- TS-03 real-binary fixture corpus (TEST-03)
- TS-04 `cargo-fuzz` harness for `parse_run` + `Pattern::from_str` (TEST-02)
- TS-05 `serial_test` on env-mutating tests (TEST-04)
- TS-06 broken-pipe integration test
- TS-07 streaming stdin in non-TUI dump (PERF-01)
- TS-08 `memchr::memmem` in `search::find_all` (PERF-03)
- TS-09 `be_u64` consolidation (REF-01)
- DF-01 `criterion` benches gating perf PRs (PERF-06)

**Should have (P2 — stretch, raises bar):**
- DF-02 `proptest` roundtrip for `u64_to_base60` / `encode_u64` *(optional; table tests already cover)*
- DF-03 streaming `Lens::render_to<W>` default (PERF-04)
- DF-05 online streaming entropy (PERF-05) — **keep in v2** per context
- DF-07 single-table `LensMode` dispatch (REF-02)
- DF-08 tighten `parse_run` contract (REF-03)
- DF-10 `reader.rs` mmap/stdin/file-open coverage (TEST-05)
- DF-11 TUI exit-with-save coverage via `TestBackend` (TEST-05)

**Defer to v3+:**
- DF-04 `insta` snapshots — existing `contains`/`starts_with` asserts suffice
- DF-06 async analyze TUI — largest change in set; ship only if DF-05 doesn't already fix TUI-launch latency
- Man pages, `cargo-audit` gate, coverage theatre, mutation testing, reproducible builds — all explicitly out of scope per PROJECT.md + FEATURES.md AF-01..11

### Architecture Approach

Post-v2 keeps the two-crate workspace (`base60-core` library + `base60-cli` binary), adds `fuzz/` as a workspace-excluded sibling, and gains `tests/` + `benches/` per-crate following Cargo conventions. Full import graph, per-wave build order, and module-by-module responsibility table in `.planning/research/ARCHITECTURE.md`.

**Major components (post-v2 deltas only):**
1. **`base60-cli::chunk`** (CLI-local, NEW) — single source of truth for `be_u64`; keeps `base60-core` zero-dep (CONSTRAINT override — see Implications §Decisions below)
2. **`base60-core::lens`** (updated) — gains `render_to<W: Write>` default method so `CuneiformLens` / `TabletLens` can stream without per-line `String` alloc; `LensMode` dispatch stays CLI-side driven by a hand-rolled `const` table
3. **`base60-cli::reader`** (updated) — adds `stream_to<W>(...)` callback-driven chunk walker for non-TUI stdin dump; `load()` retained for TUI/analyze random-access paths
4. **`base60-cli::tests/`** (NEW) — per-concern integration binaries (`roundtrip.rs`, `fixtures.rs`, `env.rs`, `reader.rs`, `tui.rs`) + `common/mod.rs` helpers + in-test generated fixtures (all ≤4 KB)
5. **`fuzz/`** (NEW, workspace-excluded) — two targets against pure-function entry points; Linux+nightly only, advisory CI smoke
6. **`crates/base60-{core,cli}/benches/`** (NEW) — criterion benches co-located with the code they exercise; advisory local-run, never CI-gating

**Key architectural patterns:** dispatch-table over enum variants (REF-02), `include_bytes!` / in-test generation for fixtures (TEST-03), online streaming accumulator for entropy (PERF-05), `Arc<Mutex<Option<Analysis>>>` for background TUI analyze (PERF-02).

### Critical Pitfalls

Top pitfalls from `.planning/research/PITFALLS.md` (full 14-item catalogue in that file). All have named mitigations already wired into the build plan.

1. **Streaming stdin OOM regression (PERF-01)** — `read_to_end` or `bytes().collect()` slipped into the streaming path. Mitigation: accept `R: BufRead`, fixed-size buffer, pre-landing integration test piping >100 MB.
2. **`be_u64` API surface creep in core (REF-01)** — promoting to `pub fn` in `base60-core` dilutes the zero-dep library's selling point. **Override PROJECT.md line 129 decision:** keep `be_u64` CLI-local in a new `crates/base60-cli/src/chunk.rs` `pub(super) fn` (pending user confirmation). Delivers the same dedup win without growing library API.
3. **`strum` in zero-dep core (REF-02)** — adding `strum` to `base60-core` violates the zero-dep contract. Mitigation: hand-roll `LensMode` dispatch as a `const` table in CLI; if strum is used at all it stays CLI-only (pending user confirmation).
4. **Fuzz false-positives (TEST-02)** — `unwrap()` inside `fuzz_target!` flags legitimate `Err` returns as crashes. Mitigation: `let _ = ...` pattern + `--release` verification; Ubuntu+nightly-only CI; pinned nightly.
5. **Criterion noise on shared CI (PERF-06)** — GHA runner variance (10-15%) exceeds any reasonable `noise_threshold`, making gating counterproductive. Mitigation: benches are **advisory-only**, run locally with `--save-baseline`, paste numbers in PR descriptions; `noise_threshold(0.05)`. Never gate CI.
6. **`serial_test` mis-keyed (TEST-04)** — `#[serial(no_color)]` and `#[serial(no_unicode)]` run in parallel (different keys). Mitigation: **one shared `env` key** for every env-mutating test; CI grep gate.
7. **`parse_run` silent error-semantics drift (REF-03)** — tightening signature changes error messages/positions in ways existing `kind()`-only tests don't catch. Mitigation: **TEST-01 before REF-03** in roadmap order.
8. **Fixture corpus bloat (TEST-03)** — 10 MB zero-fill or `/bin/ls` checked in. Mitigation: generate fixtures in-test (`minimal_elf() -> Vec<u8>` helpers); git-size CI gate.

## Implications for Roadmap

Research supports **7 phases** in three waves plus a final polish pass. Each phase ships behind a named completion check from PITFALLS.md §"Looks Done But Isn't."

### Phase 1: Refactor foundations

**Rationale:** REF-01 and REF-03 tighten the contracts that every downstream test and bench will depend on. Ships first because it's risk-free (no behaviour change) and unblocks fuzz targets + roundtrip matrix generation.

**Delivers:**
- `be_u64` deduplicated into `crates/base60-cli/src/chunk.rs` as `pub(super) fn` (REF-01, CLI-local per zero-dep-core protection)
- `decode::parse_run` signature tightens to `&[u8; RUN_LEN]`; digit-check moves inside (REF-03)
- `parse_run` + `search::Pattern` parsing re-homed as pure functions positioned for fuzz (move into CLI module that fuzz can import with `#[cfg(fuzzing)]` escape hatch, OR accept surface growth by moving into core — decision deferred to this phase)

**Addresses:** TS-09, DF-08
**Avoids:** Pitfall 5 (API surface creep), Pitfall 8 (error-semantics drift — note: TEST-01 must still land before REF-03 enters final form)

### Phase 2: LensMode consolidation

**Rationale:** REF-02 is bigger than REF-01/03 (touches `cli.rs`, `persist.rs`, `tui.rs`, `lens.rs`) and benefits from being isolated. Also lets Phase 4's roundtrip matrix iterate variants automatically.

**Delivers:**
- `LensMode` dispatch driven by a single hand-rolled `const LENS_MODES: &[(LensMode, &str, fn() -> Box<dyn Lens>)]` table in CLI
- Exhaustiveness-guard test iterating the table ensuring every variant has label + cycle + build entries
- Four parallel switch statements (`cli.rs:44-89`, `persist.rs:139-147`, plus TUI `L`-cycle + label) collapsed to one source of truth

**Addresses:** DF-07
**Avoids:** Pitfall 6 (zero-dep-core violation — hand-roll, no strum in core), CONCERNS.md "adding a lens forgets at least one site"

### Phase 3: Test infrastructure scaffolding

**Rationale:** TS-01 + TS-05 must exist before any other test work — the `tests/` crate and `serial_test` idiom are infrastructure that downstream tests adopt. `serial_test` adoption specifically precedes any new env-touching test. Scaffold in one phase; add specific test bodies in Phase 4.

**Delivers:**
- `crates/base60-cli/tests/common/mod.rs` with `base60_cmd()` helper (`.env_clear()` + explicit `--color`)
- `tests/fixtures.rs` module with fixture-generator helpers (`minimal_elf()`, `minimal_png()`, `minimal_zip()`, zero-fill, hello-world) returning `Vec<u8>` — no binary fixtures checked in
- `assert_cmd` / `predicates` / `tempfile` / `serial_test` dev-deps wired up
- `#[serial(env)]` applied to existing `cuneiform.rs:150`, `main.rs:183-219`, `lens.rs:321` env tests; CI grep gate ensuring no `env::set_var` outside `#[serial(env)]`
- One smoke test per fixture helper to validate scaffold

**Addresses:** TS-01, TS-03, TS-05
**Avoids:** Pitfall 1 (mis-keyed serialisation), Pitfall 7 (fixture bloat), Pitfall 12 (assert_cmd color auto-detect)

### Phase 4: Roundtrip matrix + coverage gap tests

**Rationale:** TS-02 (roundtrip matrix) is the Core Value guarantee and gates REF-03's final merge per Pitfall 8. Coverage-gap tests (TS-06 broken-pipe, DF-10 reader paths, DF-11 TUI exit-with-save) land alongside since they share the scaffolding from Phase 3.

**Delivers:**
- `tests/roundtrip.rs` iterating the `LensMode × {ansi, plain, json, html}` product, asserting byte-identical `base60 FILE | base60 decode` output for every cell (uses Phase 2's dispatch table to enumerate variants)
- `tests/fixtures.rs` exercising each generated fixture through `assert_cmd`
- Broken-pipe test for every format (TS-06)
- `reader.rs` mmap / stdin / file-open / `--skip`/`--length` coverage (DF-10)
- TUI exit-with-save path via `ratatui::backend::TestBackend` + `$XDG_STATE_HOME` redirection (DF-11)

**Addresses:** TS-02, TS-06, DF-10, DF-11
**Avoids:** Pitfall 8 (TEST-01 safety net for REF-03), Pitfall 10 (HashMap non-determinism)

### Phase 5: Fuzz + criterion harness

**Rationale:** TEST-02 (fuzz) and PERF-06 (criterion) are both infrastructure that Wave 3 (perf changes) depends on. They're independent of each other but share the "dev-infrastructure, not gating" treatment. Bundled for momentum.

**Delivers:**
- `fuzz/` workspace-excluded crate via `cargo fuzz init --fuzzing-workspace=true`
- Two fuzz targets (`parse_run`, `pattern_from_str`) with `let _ = ...` pattern; skeleton comments reminding "results are not bugs, panics are bugs"
- Ubuntu+pinned-nightly CI smoke job with `timeout-minutes: 5`, `-max_total_time=240`
- `crates/base60-{core,cli}/benches/*.rs` criterion benches per ownership (convert + lens in core; dump + decode + search in CLI); `noise_threshold(0.05)`
- `benches/README.md` documenting **advisory-only** posture; benches never run in PR CI

**Addresses:** TS-04, DF-01, PERF-06
**Avoids:** Pitfall 3 (fuzz false-positives), Pitfall 9 (criterion CI noise), Pitfall 11 (cross-platform fuzz)

### Phase 6: Perf pass (bench-gated)

**Rationale:** Every PERF-0X change ships with before/after numbers from the Phase 5 benches, pasted into the PR. PERF-03 (memmem) and PERF-04 (render_to) are independent and parallelisable; PERF-01 depends on PERF-04 (streaming path needs `render_to` or it allocates per chunk); PERF-05 is order-sensitive vs PERF-02. Ordering: 03 + 04 parallel → 01 → 05 → 02.

**Delivers:**
- PERF-03: `search::find_all` dispatches 1-byte → `memchr::memchr_iter`, 2+ byte → `memchr::memmem::Finder` (avoids Pitfall 4)
- PERF-04: `Lens::render_to<W>` default method + overrides for Cuneiform/Tablet with per-lens UTF-8 validity test
- PERF-01: `reader::stream_to` callback-driven streaming stdin dump with >100 MB piped integration test
- PERF-05: online streaming entropy sparkline (analyze splits into pure-regions + sparkline-iterator)
- PERF-02: background-thread TUI analyze via `Arc<Mutex<Option<Analysis>>>`; "analysing…" status-line state

**Addresses:** TS-07, TS-08, DF-03, DF-05, DF-06 (TUI) portion
**Avoids:** Pitfall 2 (streaming OOM), Pitfall 4 (memmem 1-byte regression), Pitfall 13 (render_to UTF-8)

### Phase 7: Documentation + CI polish

**Rationale:** Brings internal docs in sync with the post-v2 shape; verifies nothing in the existing CI matrix regressed.

**Delivers:**
- `.planning/codebase/ARCHITECTURE.md` updated to reflect post-v2 module layout
- CI additions: `cargo bench --no-run` sanity check; `cd fuzz && cargo +nightly fuzz build` smoke; grep gates for `env::set_var`, `HashMap` in deterministic-output paths, core zero-dep metadata check
- `crates/base60-cli/benches/README.md` advisory-posture doc
- Verify `cargo test --workspace --all-targets --locked` still green on 3×3 matrix

**Addresses:** CI-01 regression guard
**Avoids:** Pitfall 14 (doc-test drift — already caught by existing doc CI)

### Phase Ordering Rationale

- **Refactors before tests/benches before perf** — each wave's output is a hard input to the next. Refactors stabilise contracts → tests/benches build the safety net → perf changes ship gated.
- **REF-01/REF-03 before REF-02** — REF-01 and REF-03 are tiny and disjoint; REF-02 touches more files and wants isolation.
- **TEST-01 before REF-03 final merge** — Pitfall 8 safety ordering; the roundtrip matrix is the contract-pinning net for parse_run's error semantics.
- **Phase 5 (fuzz + criterion) before Phase 6 (perf)** — every PERF-0X PR requires matching bench numbers; fuzz is decoder-robustness infrastructure reused by REF-03's tighter contract.
- **PERF-04 before PERF-01** — streaming path calls `lens.render_to(..., w)`; without PERF-04 it still allocates per chunk and the peak-RSS win collapses.

### Research Flags

All phases have **well-documented patterns** — no phase requires deeper research. The research already covers every API, version pin, and idiom in detail. Flag items are decisions, not unknowns:

- **Phase 1 (REF-01):** Decision pending — `be_u64` CLI-local (recommended per zero-dep-core) vs core `pub fn` (PROJECT.md default). Recommend CLI-local; confirm with user.
- **Phase 1 (REF-03):** Decision pending — move `parse_run` / `search::Pattern` to `base60-core` to fuzz them, or keep in CLI behind `#[cfg(fuzzing)]` `pub` escape hatch. Recommend `#[cfg(fuzzing)]` hatch (protects core surface).
- **Phase 2 (REF-02):** Decision pending — hand-rolled dispatch table (recommended) vs `strum::EnumIter` CLI-only. Recommend hand-roll; pending user confirmation.
- **Phase 5 (PERF-06):** Affirm advisory-only posture; never gate CI on criterion.
- **Phase 5 (TEST-02):** Affirm weekly-scheduled non-gating fuzz CI cadence.
- **Phase 6 (PERF-05):** Affirm inclusion in v2 (already in PROJECT.md Active); Architecture research flagged as "defer?" but recommend KEEP — online accumulator is a clean refactor and ships with PERF-01's streaming shape.
- **Benches layout:** Affirm per-crate benches split by ownership (core benches → core; CLI benches → CLI).

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Every version verified via crates.io API + context7 + official docs; three of the new crates are already in `Cargo.lock` |
| Features | HIGH | Every feature traces to `PROJECT.md` Active or `CONCERNS.md` debt; peer CLIs (hexyl, bat, ripgrep) cross-referenced |
| Architecture | HIGH | Module boundaries and import graph verified against actual v1 source; cargo-fuzz / criterion / assert_cmd conventions verified via Context7 + upstream READMEs |
| Pitfalls | HIGH | Every pitfall has an authoritative source (serial_test docs, Criterion FAQ, rust-fuzz book, memchr issue tracker); project-local pitfalls cross-referenced with CONCERNS.md |

**Overall confidence:** HIGH

### Gaps to Address

- **`be_u64` placement decision** — PROJECT.md line 129 says `base60-core::chunk::pub fn`; Pitfalls §5 argues for CLI-local. Resolve in Phase 1 kick-off before writing code. **Recommendation: CLI-local; flag "pending user confirmation" in roadmap.**
- **`strum` in core vs hand-roll** — Architecture research treats `strum` as acceptable; Pitfalls §6 flags it as zero-dep-core violation. Resolve in Phase 2 kick-off. **Recommendation: hand-roll; flag "pending user confirmation" in roadmap.**
- **`parse_run`/`Pattern` core promotion vs `#[cfg(fuzzing)]` hatch** — Architecture recommends core promotion for natural fuzzability; Pitfalls §5 suggests the escape hatch protects surface. Either is workable; decide in Phase 1.
- **Peak-RSS measurement for PERF-01 integration test** — no stdlib primitive; options are `procfs` dev-dep (Linux-only) or a "doesn't OOM on `/dev/zero | head -c 10G`" smoke test. Decide in Phase 6 kick-off; the smoke is probably sufficient.
- **Bench CI visibility** — Phase 5 lands benches as advisory-only. If perf regressions start shipping in practice, revisit in v3 with Iai-Callgrind or self-hosted runners (FEATURES.md AF-08).

## Sources

### Primary (HIGH confidence)
- crates.io API (2026-04-24) — `max_stable_version` + MSRV + features for every crate in stack
- Context7: `/rust-fuzz/cargo-fuzz`, `/bheisler/criterion.rs`, `/peternator7/strum`, `/assert-rs/predicates-rs`, `/stebalien/tempfile`, `/burntsushi/memchr`, `/nvzqz/divan` (for comparison), `/proptest-rs/proptest` (for rejection rationale), `/mitsuhiko/insta` (for rejection rationale)
- docs.rs: `serial_test`, `assert_cmd`, `memchr::memmem`
- Rust Fuzz Book — cargo-fuzz platform constraints, nightly requirement, workspace-isolation flag
- Criterion.rs FAQ — explicit cloud-CI benchmarking warning
- `/home/chris/Projects/utils/test-60/Cargo.lock` — verified `memchr 2.8.0`, `strum 0.27.2`, `strum_macros 0.27.2` already present
- `/home/chris/Projects/utils/test-60/.planning/PROJECT.md` — v2 scope, constraints, key decisions
- `/home/chris/Projects/utils/test-60/.planning/codebase/{ARCHITECTURE,CONCERNS,CONVENTIONS,STACK,STRUCTURE,TESTING}.md` — authoritative v1 map

### Secondary (MEDIUM confidence)
- BurntSushi/memchr README + issue #139 — `memmem` prefilter regression on short needles
- cargo-fuzz issue #173 — `-Cpanic=abort` precludes `catch_unwind` filtering
- GitHub Actions 2026 pricing changelog — self-hosted runner platform fee
- rust-users forum: MSRV + dev-deps separation pattern
- tevps.net: serial_test design rationale
- Stephan Brumme: Practical String Searching (short-needle scan analysis)

### Tertiary (LOW confidence)
- None required; all pitfalls and decisions are grounded in primary or secondary sources.

---
*Research completed: 2026-04-24*
*Ready for roadmap: yes*
