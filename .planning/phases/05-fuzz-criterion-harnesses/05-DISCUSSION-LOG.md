# Phase 5: Fuzz + Criterion Harnesses - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-24
**Phase:** 05-fuzz-criterion-harnesses
**Mode:** `--auto` — Claude auto-selected the recommended option for every gray area without user interaction per skill contract; user pre-confirmed by flag.
**Areas discussed:** Fuzz workspace layout, Fuzz hatch shape, Seed corpus, Fuzz input guards, Criterion dev-dep, Bench scope, Bench readmes, Commit granularity

---

## Fuzz workspace layout

| Option | Description | Selected |
|--------|-------------|----------|
| `--fuzzing-workspace=true` + root `exclude = ["fuzz"]` | Nested `[workspace]` in `fuzz/Cargo.toml` + explicit root exclude (belt + suspenders) | ✓ |
| `--fuzzing-workspace=true` only | Nested workspace, no root exclude (redundant but cleaner) | |
| Root `exclude = ["fuzz"]` only (no nested) | Fuzz crate in same resolver; nightly flags leak | |
| `fuzz/` as workspace MEMBER | Cargo check coverage, but contaminates lockfile with nightly-profile state | |

**Claude's choice:** Option 1 (nested workspace + root exclude).
**Rationale:** ROADMAP SC1 literally enumerates both mechanisms ("declared via `cargo fuzz init --fuzzing-workspace=true`, listed under root `Cargo.toml` `[workspace] exclude = ["fuzz"]`"), and they serve different audiences — one is cargo behaviour, the other is documentation-by-manifest for future maintainers.
**Notes:** STACK.md §Fuzzing confirms this is the canonical pattern. Matches PROJECT.md Key Decision row 7.

---

## Fuzz hatch shape

| Option | Description | Selected |
|--------|-------------|----------|
| `__fuzz` re-export module in `lib.rs` | `#[doc(hidden)] #[cfg(fuzzing)] pub mod __fuzz { pub use ... }` — widens all fuzz targets through one gated module | ✓ |
| Per-item `#[cfg_attr(fuzzing, visibility::make(pub))]` | Adds `visibility` crate dep; minimal single-site edits; still needs the trait hack | |
| Hand-written `#[cfg(fuzzing)] pub fn` wrapper | Duplicates the function body; maintenance nightmare | |
| Move `parse_run`/`Pattern` to `base60-core` | Rejected by PROJECT.md Key Decision row 7 — would leak CLI concerns into zero-dep library | |

**Claude's choice:** Option 1 (`__fuzz` module re-export).
**Rationale:** Matches PROJECT.md Key Decision row 7 literally ("`parse_run`/`Pattern` stay in `base60-cli` with `#[cfg(fuzzing)] pub` hatch"). One cfg-gated module is trivially auditable (`git grep '#\[cfg(fuzzing)\]' crates/base60-cli/src/lib.rs` returns 1 hit); zero new deps; existing visibility bumps (`pub(crate)` on `parse_run`/`RUN_LEN`) are additive and don't affect non-fuzz `cargo doc` output.
**Notes:** Inside `decode.rs`: `fn parse_run` → `pub(crate) fn parse_run`; `const RUN_LEN` → `pub(crate) const RUN_LEN`. Both need `/// # Errors` / `/// # Panics` rustdoc additions (CI `RUSTDOCFLAGS=-D warnings`).

---

## Seed corpus

| Option | Description | Selected |
|--------|-------------|----------|
| Empty on commit | No seeds checked in; libFuzzer bootstraps | ✓ |
| Seeded with roundtrip fixtures | Reuse `tests/common/` fixtures as seeds | |
| Hand-crafted targeted seeds | `00:00:…:00`, `29:29:…:29`, `99:00:…:00`, malformed variants | |
| Cargo `fuzz-corpus-dir` pointed at `.planning/fuzz-seeds/` | Track seeds in planning tree, not `fuzz/` | |

**Claude's choice:** Option 1 (empty).
**Rationale:** ROADMAP SC2 asserts 30 s no-crash smoke — doesn't require coverage floor. libFuzzer's coverage-guided mutation bootstraps quickly on 23-byte inputs (`parse_run` RUN_LEN = 23) and string-parsing targets. Pre-seeding biases coverage toward already-tested paths — worth avoiding for a new fuzz campaign.
**Notes:** Re-evaluate after two Phase 7 CI-02 weekly runs. If coverage plateaus, seed with targeted inputs then. Documented in CONTEXT Deferred Ideas §2.

---

## Fuzz input guards

| Option | Description | Selected |
|--------|-------------|----------|
| Raw `&[u8]` + length/UTF-8 guards | `if data.len() != RUN_LEN { return; }` / `if let Ok(s) = std::str::from_utf8(data)` — `let _ = ...` for results | ✓ |
| `arbitrary::Arbitrary` derive | Structured input via `arbitrary = "1.4"` dep — more expressive but unnecessary | |
| `std::panic::catch_unwind` in target | Invalid — `cargo-fuzz` compiles `-Cpanic=abort`, catch_unwind is a no-op | |

**Claude's choice:** Option 1 (raw bytes + guards).
**Rationale:** Both entry points take `&[u8]`/`&str` — no structured input to generate. STACK.md explicitly says "start without [arbitrary]". PITFALLS Pitfall 3 mandates `let _ = ...` + UTF-8 guard shape.
**Notes:** Each fuzz target file opens with a `// IMPORTANT: Err returns are happy path; only panics are bugs.` banner to prevent future maintainers from adding `.unwrap()`.

---

## Criterion dev-dep + feature flags

| Option | Description | Selected |
|--------|-------------|----------|
| `criterion 0.8` with `default-features = false, features = ["cargo_bench_support", "html_reports"]` on both crates | Drops `rayon` (bench-noise on streaming code); keeps baseline UX; required `cargo_bench_support` for non-nightly | ✓ |
| `criterion` with default features | Adds `rayon` parallelism — noise on single-threaded streaming measurements | |
| `divan 0.1` | Pre-1.0; nicer API; lacks save-and-compare | |
| `iai-callgrind` | Instruction-count determinism; breaks macOS/Windows CI | |

**Claude's choice:** Option 1 (criterion 0.8 minimal features, both crates).
**Rationale:** STACK.md §Benchmarking explicitly recommends criterion over divan for the save-and-compare (`cargo bench -- --save-baseline`) feature — that's the one knob that makes advisory-only benches useful. `rayon` off because the streaming code is single-threaded and `rayon`-driven sampling adds variance. Dev-dep on `base60-core` doesn't violate CI-03 zero-dep invariant (Phase 2 D-02 precedent).
**Notes:** `noise_threshold(0.05)` per PITFALLS Pitfall 9 — 5% tolerance; above shared GHA runner noise but below typical regression.

---

## Bench scope + structure (PERF-06 SC3)

| Option | Description | Selected |
|--------|-------------|----------|
| 5 files (2 core + 3 CLI) per ROADMAP SC3 | `convert`, `lens` in core; `dump`, `decode`, `search` in CLI — 1 MiB deterministic corpus each | ✓ |
| 3 files (CLI only) | Skip core benches; Phase 6 `render_to` belongs near its call site anyway | |
| Single consolidated `base60-bench` crate | One crate, 5 modules; centralised but breaks per-crate ownership | |
| Minimal scaffolding — only `search.rs` | Only gates PERF-03; others land in Phase 6 | |

**Claude's choice:** Option 1 (5 files per ROADMAP).
**Rationale:** ROADMAP SC3 literally enumerates the 5 files. PERF-03 gating (PITFALLS Pitfall 4) specifically needs 1-byte and 2-byte zero-fill needle cells in `search.rs`. Advisory-only means no CI wallclock concern; scaffolding is cheap.
**Notes:** Each bench uses `sample_size(50)` to keep `cargo bench --workspace` under ~30 s (Phase 7 SC4 `--no-run` compile step won't run them anyway). Bench inputs are compile-time-constant bytes or `wrapping_mul` fillers — no `rand` dep.

---

## Bench README posture (PERF-06 SC4)

| Option | Description | Selected |
|--------|-------------|----------|
| `crates/base60-cli/benches/README.md` canonical + one-liner `core/benches/README.md` pointer | Single source of truth, mirror pointer satisfies SC4's "crate-level" reading | ✓ |
| Two full READMEs (one per crate) | DRY violation; drift risk | |
| Workspace-root `BENCHMARKS.md` | Visibility but breaks SC4 locality | |

**Claude's choice:** Option 1.
**Rationale:** ROADMAP SC4 refers specifically to `crates/base60-cli/benches/README.md`. Core benches are a small subset (2 of 5) that share posture with the CLI ones — pointer-mirror is honest about that.

---

## Commit granularity + plan count

| Option | Description | Selected |
|--------|-------------|----------|
| 2 plans (TEST-02 + PERF-06, one per REQ) | Matches Phase 1-4 convention; each plan = 1 atomic commit | ✓ |
| 4 plans (fuzz init / fuzz targets / bench scaffolding / bench contents) | Finer atomicity but more workflow overhead for a parallel-safe phase | |
| 1 combined plan | Bundles unrelated surfaces (fuzz ≠ benches); breaks REQ-per-commit | |

**Claude's choice:** Option 1 (2 plans).
**Rationale:** Phase 1, 2, 3, 4 all used plan-per-REQ-ID (or close to it). REQ-per-plan keeps bisect-ability high. Files are fully disjoint (`fuzz/` vs `crates/*/benches/`) so internal parallel-safety is preserved.
**Notes:** Commit order 05-01 → 05-02 is arbitrary — both plans are parallel-safe. `/gsd-plan-phase 5` orchestrator will refine this.

---

## Claude's Discretion

- Exact bench input generator (deterministic seed pattern).
- Bench function names + group organisation.
- Fuzz target banner comment wording.
- Whether `benches/common/mod.rs` helper file exists or generators duplicate inline.
- Whether PALETTE_ANSI dump variant is a second bench cell in `dump.rs`.
- Exact `fuzz/.gitignore` supplementation.
- Whether `criterion_group!` uses macro form or manual `Criterion::default()` instance.

## Deferred Ideas

- `cargo-public-api --diff` tooling (v3).
- Seed corpus curation (after Phase 7 weekly data).
- Iai-Callgrind migration (if criterion proves insufficient in Phase 6).
- `arbitrary`-driven structured fuzz (when a target needs it).
- `bencher.dev` / `codspeed.io` baseline tracking (v3 OBSV-02).
- Additional fuzz targets (emit_json/emit_html, chunk::be_u64).
- Per-lens `render_to` UTF-8 fuzz (Phase 6 PERF-04).

## Out of Scope (rejected, not deferred)

- Move `parse_run`/`Pattern` to `base60-core` — PROJECT.md row 7 rejects.
- `divan` — STACK.md rejects for save-and-compare gap.
- CI-gated criterion — PROJECT.md row 8 rejects permanently.
- `cargo-tarpaulin` / codecov — REQUIREMENTS line 70 rejects.
- `proptest` / `quickcheck` — Phase 2 discussion rejected; re-confirmed here.
