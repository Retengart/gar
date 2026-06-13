---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: complete
last_updated: "2026-06-13T12:00:00.000Z"
progress:
  total_phases: 7
  completed_phases: 7
  total_plans: 14
  completed_plans: 14
  percent: 100
---

# Project State — base60 v2

## Project Reference

**Core Value:** Every binary blob that `base60 FILE | base60 decode` round-trips must come out byte-identical — the visual format is opinionated, but the pipeline is lossless.

**Current Milestone:** v2 — hardening only. Three themes: integration tests + fuzz, streaming + perf, refactor consolidation. No new user-facing feature surface.

**Scope:** 16 v2 requirements across 7 phases. Standard granularity. Yolo mode, parallelisation enabled, quality model profile.

## Current Position

Phase: ALL COMPLETE
**Milestone:** v2 hardening
**Status:** All 7 phases complete, 17/17 requirements shipped
**Progress:** 7 / 7 phases complete

```
[====·====·====·====·====·====·====] 100%
 P1   P2   P3   P4   P5   P6   P7
```

## Performance Metrics

**Phases completed:** 7 / 7
**Plans executed:** 14
**Requirements shipped:** 17 / 17 v2
**CI baseline:** 3 OS × 3 rustc matrix green

## Accumulated Context

### Key Decisions (from PROJECT.md — locked before roadmap)

1. **`be_u64` is CLI-local** at `crates/base60-cli/src/chunk.rs` (NOT in `base60-core`). Protects zero-dep library surface.
2. **`LensMode` dispatch is hand-rolled** — a `const ALL: &[LensMode]` table in CLI. No `strum` in core.
3. **`parse_run` / `Pattern` stay in `base60-cli`** with a `#[cfg(fuzzing)] pub` escape hatch for fuzz targets. Core public API surface stays minimal.
4. **Criterion benches are advisory-only.** Never CI-gating — GHA runner noise floor exceeds any reasonable threshold. Baselines local; numbers pasted in PRs.
5. **Fuzz CI runs weekly** on `schedule:`, Ubuntu + pinned nightly only, 5-minute timeout, non-gating. libFuzzer is Linux-nightly-only.

### Ordering Constraints (from research — inviolable)

- TEST-01 (Phase 3) before REF-03 (Phase 4) — safety net for parse_run contract change
- PERF-06 (Phase 5) before PERF-01..05 (Phase 6) — each perf PR needs a baseline
- TEST-04 (Phase 2) before any new env-mutating test — idiom-first
- REF-02 (Phase 1) enables TEST-01 matrix enumeration (Phase 3 uses the dispatch table)

### Todos

None yet. Populated during plan execution.

### Blockers

None.

### Open Questions

From research — flagged but already resolved by PROJECT.md Key Decisions:

- ~~`be_u64` CLI-local vs. core?~~ → CLI-local (decision locked)
- ~~`strum` in core vs. hand-roll?~~ → Hand-roll (decision locked)
- ~~`parse_run` core promotion vs. cfg hatch?~~ → cfg hatch (decision locked)
- ~~Criterion CI gate?~~ → Advisory only (decision locked)
- ~~Fuzz CI cadence?~~ → Weekly schedule, Ubuntu+nightly (decision locked)

Remaining open (decide during plan execution):

- Peak-RSS measurement approach for PERF-01: `procfs` dev-dep (Linux-only) vs. `/dev/zero | head -c 10G` smoke test. Defer to Phase 6 kickoff.
- Fuzz seed corpus: commit seed inputs or start empty? Defer to Phase 5 kickoff.

## Session Continuity

**Last session:** 2026-06-13T12:00:00.000Z

**Status:** v2 milestone COMPLETE. All 7 phases, 14 plans, 17 requirements shipped.

**Phase 6 deliverables (this session):**
- PERF-01: `dump_reader` streaming stdin path — bounded memory
- PERF-02: Async TUI analysis via background thread
- PERF-03: `memchr::memmem` + `memchr_iter` for `find_all`
- PERF-04: `Lens::render_to(&mut dyn Write)` — zero-alloc overrides in Tablet+Cuneiform
- PERF-05: Single-pass `analyze` with `EntropyStats` online accumulator

**Phase 7 deliverables (this session):**
- CI-02: `.github/workflows/fuzz.yml` — weekly scheduled fuzz job
- CI-03: `zero-dep-core` + `benches-compile` steps in ci.yml

---

*State initialised: 2026-04-24*
