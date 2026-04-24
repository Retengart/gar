---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
last_updated: "2026-04-24T08:41:28.157Z"
progress:
  total_phases: 7
  completed_phases: 1
  total_plans: 5
  completed_plans: 2
  percent: 40
---

# Project State — base60 v2

## Project Reference

**Core Value:** Every binary blob that `base60 FILE | base60 decode` round-trips must come out byte-identical — the visual format is opinionated, but the pipeline is lossless.

**Current Milestone:** v2 — hardening only. Three themes: integration tests + fuzz, streaming + perf, refactor consolidation. No new user-facing feature surface.

**Scope:** 16 v2 requirements across 7 phases. Standard granularity. Yolo mode, parallelisation enabled, quality model profile.

## Current Position

Phase: 02 (env-test-serialisation) — EXECUTING
Plan: 1 of 3
**Milestone:** v2 hardening
**Phase:** 2
**Plan:** Not started
**Status:** Executing Phase 02
**Progress:** 0 / 7 phases complete

```
[----·----·----·----·----·----·----]   0%
 P1   P2   P3   P4   P5   P6   P7
```

## Performance Metrics

**Phases completed:** 0 / 7
**Plans executed:** 0
**Requirements shipped:** 0 / 16 v2
**CI baseline:** 3 OS × 3 rustc matrix green on v1 (last verified at roadmap creation)

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

**Last session:** 2026-04-24T08:17:15.731Z

**Next session:** `/gsd-plan-phase 1` — decompose Phase 1 (Refactor Foundations) into executable plans for REF-01 (CLI-local `chunk::be_u64`) and REF-02 (LensMode dispatch table).

**Artifacts produced this session:**

- `.planning/ROADMAP.md` (7 phases, 16 / 16 requirements mapped)
- `.planning/STATE.md` (this file)
- `.planning/REQUIREMENTS.md` (traceability section updated in place)

**Artifacts consumed this session:**

- `.planning/PROJECT.md` (9 Key Decisions — all resolved before roadmapping)
- `.planning/REQUIREMENTS.md` (16 v2 requirements)
- `.planning/research/SUMMARY.md` (7-phase wave recommendation)
- `.planning/research/STACK.md` (crate versions + feature flags)
- `.planning/research/ARCHITECTURE.md` (module placements, import graph)
- `.planning/research/PITFALLS.md` (ordering constraints, 14 named pitfalls)
- `.planning/research/FEATURES.md` (MVP + dependency graph)
- `.planning/codebase/ARCHITECTURE.md` (v1 module boundaries baseline)
- `.planning/config.json` (standard granularity, quality model)

---

*State initialised: 2026-04-24*
