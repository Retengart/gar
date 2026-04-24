# Phase 5: Fuzz + Criterion Harnesses - Context

**Gathered:** 2026-04-24
**Status:** Ready for planning
**Mode:** `--auto` — Claude picked recommended option for each gray area. Log at bottom of each section.

<domain>
## Phase Boundary

Infrastructure only — ship the two scaffoldings Phase 6 (perf pass) and Phase 7 (weekly fuzz CI) consume:

1. **`fuzz/`** at repo root — workspace-excluded `cargo-fuzz` crate with two targets (`parse_run`, `pattern_from_str`). No CI job in this phase — Phase 7 CI-02 adds the weekly schedule.
2. **Per-crate `benches/`** — `base60-core/benches/{convert,lens}.rs` + `base60-cli/benches/{dump,decode,search}.rs`. Criterion 0.8, `harness = false`, advisory-only posture documented in a README. No CI gating — Phase 7 SC4 adds the `--no-run` compile smoke.

Requirements: **TEST-02**, **PERF-06**.

**In scope:**
- `fuzz/` via `cargo fuzz init --fuzzing-workspace=true`, listed under root `Cargo.toml` `[workspace] exclude = ["fuzz"]` (belt-and-suspenders per ROADMAP SC1 wording).
- `#[doc(hidden)] #[cfg(fuzzing)] pub mod __fuzz { … }` hatch in `crates/base60-cli/src/lib.rs` re-exporting `decode::parse_run`, `decode::RUN_LEN`, and `search::Pattern`. Non-fuzz `cargo check --workspace` public API surface unchanged (SC5).
- `parse_run` bumped `fn` → `pub(crate) fn`; `RUN_LEN` bumped `const` → `pub(crate) const`. `Pattern` and `Pattern::from_str` already accessible via `pub(crate)`.
- `libfuzzer-sys = "0.4"` in `fuzz/Cargo.toml`. Path deps on both `base60-core` and `base60` (CLI lib target). No `arbitrary` crate.
- `fuzz/.gitignore` covers `corpus/`, `artifacts/`, `target/`. Empty seed corpus on commit.
- `criterion = { version = "0.8", default-features = false, features = ["cargo_bench_support", "html_reports"] }` added to `[dev-dependencies]` on BOTH `base60-cli` and `base60-core`. Dev-deps don't violate CI-03 zero-dep invariant (Phase 2 D-02 precedent).
- Five `[[bench]] name = "…", harness = false` entries split across the two crates. Each bench uses `Criterion::default().noise_threshold(0.05)` (PITFALLS Pitfall 9).
- Bench corpus is deterministic, compile-time-constructed bytes — no `rand` dep, no checked-in binaries.
- `crates/base60-cli/benches/README.md` documenting advisory-only posture (ROADMAP SC4). `crates/base60-core/benches/README.md` is a one-liner pointing at the CLI README.

**Not in scope:**
- Weekly fuzz CI job → Phase 7 CI-02.
- `benches-compile` CI step → Phase 7 SC4.
- `--zero-dep-core` metadata check → Phase 7 CI-03.
- PERF-04 `render_to<W>` bench comparison — only `render()` measured in this phase; Phase 6 extends `lens_bench.rs` when PERF-04 lands.
- `cargo public-api --diff` tooling — manual verification via existing `cargo doc --workspace --no-deps --locked` CI job is enough. `cargo-public-api` install deferred to v3 if drift becomes an issue.
- Fuzz seed corpus curation — empty on commit. Re-evaluate if TEST-02 coverage stalls after a few weeks of Phase 7 CI runs.
- `arbitrary` crate dev-dep — raw `&[u8]` + length/UTF-8 guards suffice for both targets (STACK.md "start without it").
- Moving `parse_run`/`Pattern` into `base60-core` — explicitly rejected in PROJECT.md Key Decision row 7 (`#[cfg(fuzzing)] pub` hatch is the lighter option).

</domain>

<decisions>
## Implementation Decisions

### A. Fuzz workspace layout (TEST-02 SC1)

- **D-01:** Create `fuzz/` at repo root via `cargo fuzz init --fuzzing-workspace=true`. The flag makes `fuzz/Cargo.toml` declare its own `[workspace]` so nightly-only resolver settings, `-Zsanitizer` instrumentation, and fuzz-profile flags never contaminate the main workspace `Cargo.lock`.
- **D-02:** Root `Cargo.toml` gains `[workspace] exclude = ["fuzz"]`. Belt-and-suspenders: `--fuzzing-workspace=true` already prevents membership via the nested `[workspace]`, but the explicit `exclude` matches ROADMAP SC1 verbatim and makes the intent obvious to a reader scanning the root manifest.
- **D-03:** `fuzz/Cargo.toml` path-deps on BOTH `base60-core = { path = "../crates/base60-core" }` and `base60 = { path = "../crates/base60-cli", package = "base60" }`. Redundant (CLI transitively pulls core) but explicit — matches STACK.md fuzz-manifest template.
- **D-04:** `rust-version` field dropped from `fuzz/Cargo.toml` — fuzz builds run under nightly only; the MSRV 1.95 floor is a main-workspace concern.

*[auto] Selected recommended option: `--fuzzing-workspace=true` + root exclude. Rationale: matches ROADMAP SC1 phrasing exactly, and the two mechanisms are orthogonal (one is cargo behaviour, one is documentation-by-manifest).*

### B. `#[cfg(fuzzing)] pub` hatch shape (TEST-02 SC5)

- **D-05:** Hatch lives in `crates/base60-cli/src/lib.rs` as a `#[doc(hidden)] #[cfg(fuzzing)] pub mod __fuzz { … }` module. Re-exports: `pub use crate::decode::{parse_run, RUN_LEN}; pub use crate::search::Pattern;`. Non-fuzz compilation units never see the module (`#[cfg(fuzzing)]` is only set by `cargo-fuzz` per STACK.md §Fuzzing).
- **D-06:** Inside `crates/base60-cli/src/decode.rs`: `fn parse_run` → `pub(crate) fn parse_run`. `const RUN_LEN` → `pub(crate) const RUN_LEN`. Both gain `# Errors` / `# Panics` rustdoc sections as required by CI (`RUSTDOCFLAGS=-D warnings`). Hot path is otherwise unchanged.
- **D-07:** `crates/base60-cli/src/search.rs`: no changes needed — `Pattern` is already `pub(crate) struct Pattern(pub(crate) Vec<u8>)` and `FromStr` trait impl is automatically visible via the re-export. No extra visibility bump.
- **D-08:** SC5 verification is performed by a unit test + manual doc review: a `#[cfg(not(fuzzing))]` compile-time test asserts no `pub` symbol exists at `base60::__fuzz` in normal builds. Also: `cargo doc --workspace --no-deps --locked` already gates public-API drift per CI — Phase 7 CI-03 adds the zero-dep check. `cargo-public-api` tooling deferred (Deferred Ideas §1).

*[auto] Selected recommended option: `__fuzz` module re-export. Rationale: widens exactly two items (a fn and a const) via one `#[cfg(fuzzing)]`-gated module instead of widening each item individually; keeps the non-fuzz public surface trivially auditable (`git grep '#\[cfg(fuzzing)\] pub' crates/base60-cli/src/` returns one line — the module declaration).*

### C. Seed corpus (TEST-02 SC2)

- **D-09:** Ship with empty seed corpora. `fuzz/corpus/` is directory-only (cargo-fuzz creates it on first run). Rationale: libFuzzer's coverage-guided mutation bootstraps quickly on 23-byte inputs (`parse_run`) and string-parsing targets (`Pattern::from_str`); pre-seeding with known-good dumps would bias coverage toward already-tested paths. ROADMAP SC2 asserts a 30-second no-crash run — that's a smoke test, not a coverage floor.
- **D-10:** `fuzz/.gitignore` (generated by `cargo fuzz init`) covers `corpus/`, `artifacts/`, `target/`. If generated content is missing any entry, supplement by hand in the same commit.
- **D-11:** Reassess seeding after two weekly Phase 7 fuzz runs. If corpus growth stalls under ~1000 entries in the first 4 minutes, add a handful of hand-crafted seeds (e.g., `00:00:...:00`, `29:29:...:29`, `99:00:...:00`) as `fuzz/seeds/parse_run/*` and document in `fuzz/README.md`. Not this phase.

*[auto] Selected recommended option: empty seeds. Rationale: libFuzzer bootstraps coverage quickly on small input shapes; premature seeding biases signal toward already-covered paths; SC2 only asks for 30 s no-crash.*

### D. Fuzz input guards (TEST-02 SC1, PITFALLS Pitfall 3)

- **D-12:** `fuzz/fuzz_targets/parse_run.rs` shape:
  ```rust
  #![no_main]
  use libfuzzer_sys::fuzz_target;
  fuzz_target!(|data: &[u8]| {
      if data.len() != base60::__fuzz::RUN_LEN { return; }
      let Ok(arr) = <&[u8; base60::__fuzz::RUN_LEN]>::try_from(data) else { return; };
      // Errors are happy path; only panics are bugs.
      let _ = base60::__fuzz::parse_run(arr, 1);
  });
  ```
  The length-gate makes coverage feedback meaningful — libFuzzer's mutator can still grow seeds past 23 bytes, but the target early-returns rather than calling `parse_run` with the wrong length (which would be a type error, not a discovered bug).
- **D-13:** `fuzz/fuzz_targets/pattern_from_str.rs` shape:
  ```rust
  #![no_main]
  use libfuzzer_sys::fuzz_target;
  use std::str::FromStr;
  fuzz_target!(|data: &[u8]| {
      if let Ok(s) = std::str::from_utf8(data) {
          let _ = base60::__fuzz::Pattern::from_str(s);
      }
  });
  ```
  UTF-8 guard matches the `rust-fuzz/book` canonical pattern. Do NOT use `std::panic::catch_unwind` — `cargo-fuzz` compiles with `-Cpanic=abort`.
- **D-14:** Each fuzz target file starts with a comment block:
  ```
  // IMPORTANT: Err returns are the happy path — only panics are bugs.
  // On reported crash: reproduce with `--release` first to confirm.
  // Platform: Ubuntu + pinned nightly only (libFuzzer is Linux-x86_64/aarch64 only).
  ```
  Literal wording is Claude's Discretion.
- **D-15:** No `arbitrary` crate. Both entry points accept `&[u8]`/`&str` directly; structured-input generation isn't needed for the current surface.

*[auto] Selected recommended option: raw `&[u8]` + length/UTF-8 guards, no `arbitrary`. Rationale: STACK.md "start without it"; guards prevent false-positive panics from by-design Err returns per PITFALLS Pitfall 3.*

### E. Criterion dev-dep (PERF-06 SC3)

- **D-16:** Add to `crates/base60-cli/Cargo.toml [dev-dependencies]`:
  ```toml
  criterion = { version = "0.8", default-features = false, features = ["cargo_bench_support", "html_reports"] }
  ```
  Default features dropped: `rayon` (parallel-bench noise on streaming-code measurements), `plotters` (stays via `html_reports` feature group anyway). `cargo_bench_support` is required for `cargo bench` discovery without `#![feature(test)]`.
- **D-17:** Same entry added to `crates/base60-core/Cargo.toml [dev-dependencies]`. Does NOT violate CI-03 zero-dep invariant — that rule covers `[dependencies]` only, per Phase 2 CONTEXT D-02 precedent.
- **D-18:** Version `0.8` (latest as of STACK.md research, 2026-04-24) — compatible with MSRV 1.95 (criterion declares 1.86). No version pinning beyond the minor; workspace `--locked` CI holds the exact resolution.

### F. Bench entries (PERF-06 SC3)

- **D-19:** `crates/base60-core/Cargo.toml` gains:
  ```toml
  [[bench]] name = "convert" harness = false
  [[bench]] name = "lens"    harness = false
  ```
- **D-20:** `crates/base60-cli/Cargo.toml` gains:
  ```toml
  [[bench]] name = "dump"   harness = false
  [[bench]] name = "decode" harness = false
  [[bench]] name = "search" harness = false
  ```
- **D-21:** Every bench configures its `Criterion` instance with `.noise_threshold(0.05)` — 5% tolerance per PITFALLS Pitfall 9. Local laptop noise + shared-CI-runner noise both comfortably fit under 5%.
- **D-22:** Bench shape is `criterion_group!` + `criterion_main!` at the bottom of each file, one group per file. `sample_size(50)` (not 100) keeps `cargo bench --workspace` under ~30 s wall-clock on Ubuntu — low enough that Phase 7's `benches-compile` CI step stays cheap.

### G. Bench scope (PERF-06 SC3, Phase 6 prerequisite)

Bench content per file — concrete targets chosen to be gating baselines for Phase 6 PERF-0X:

- **D-23:** `crates/base60-core/benches/convert.rs` — `u64_to_base60` hot loop. Input: `Vec<u64>` of 1024 deterministic values (e.g., `(0u64..1024).map(|i| i.wrapping_mul(0x9E3779B97F4A7C15)).collect()`). No `rand` dep.
- **D-24:** `crates/base60-core/benches/lens.rs` — `render(&self, u64) -> String` for each of `TimeLens`/`AngleLens`/`TabletLens`/`CuneiformLens` against a 1024-element input. One `criterion_group!` with four functions (or four groups — Claude's Discretion). PERF-04's `render_to<W>` is Phase 6; Phase 6 extends this file with a second variant.
- **D-25:** `crates/base60-cli/benches/dump.rs` — `dump::dump_all` (or `dump::write_line` loop) over a 1 MiB compile-time-constant byte array. Palette: `&PALETTE_NONE` (mono path — the common hot path; PALETTE_ANSI variant is Claude's Discretion). No lens applied (lens benches live in `core/benches/lens.rs`).
- **D-26:** `crates/base60-cli/benches/decode.rs` — `decode::decode_stream` over the output of a pre-computed 1 MiB dump (plain format, no color). The dump-generation runs once per bench-process via `std::sync::LazyLock`; only the `decode_stream` call is inside the `b.iter(...)` block.
- **D-27:** `crates/base60-cli/benches/search.rs` — `search::find_all` with parametrised input. MANDATORY cells (per PITFALLS Pitfall 4, gates PERF-03):
  - Haystack `vec![0u8; 1 << 20]`, needle `b"\x00"` (1 byte, zero-fill) — catches the 1-byte regression.
  - Haystack `vec![0u8; 1 << 20]`, needle `b"\xff\xff"` (2 byte, zero-fill) — catches the packedpair prefilter over-trigger.
  - Haystack deterministic random (1 MiB, fixed seed pattern via `wrapping_mul`), needle `b"ELF"` (3 byte).
  - Haystack deterministic random, needle `b"cafebabe"` (8 byte).
  Four cells minimum; more are Claude's Discretion.
- **D-28:** Bench input generation uses zero new deps — `u8::wrapping_mul` + `wrapping_add` is fine for pseudo-random-looking 1 MiB haystacks. If a future perf work needs true random distribution, add `rand` dev-dep in Phase 6; not Phase 5.

*[auto] Selected recommended option for each bench's shape + input size + cell set.*

### H. Bench READMEs (PERF-06 SC4)

- **D-29:** `crates/base60-cli/benches/README.md` is the canonical advisory-posture doc. Content shape (exact wording is Claude's Discretion):
  ```
  # Benchmarks — advisory only, NEVER CI-gated.
  Run locally: `cargo bench -p base60 --bench <name> -- --save-baseline pre`
  Apply change, then: `cargo bench -p base60 --bench <name> -- --baseline pre`
  Paste before/after numbers into PR description. Do not add a CI gate.
  Noise floor: shared GHA runners ~10%; `noise_threshold(0.05)` is a local-laptop tolerance.
  ```
- **D-30:** `crates/base60-core/benches/README.md` is a one-liner: `See ../../base60-cli/benches/README.md for the advisory-only posture.` Mirrors PERF-06 SC4 wording without duplication.

### I. Commit granularity

- **D-31:** **Two plans, two commits** — matches Phase 1-4 REQ-ID convention (one plan per REQ-ID):
  1. `05-01-PLAN.md` — `test(cli): fuzz crate scaffolding with parse_run + pattern_from_str targets [TEST-02]` — creates `fuzz/`, adds `__fuzz` re-export hatch in `base60-cli/src/lib.rs`, bumps `parse_run`/`RUN_LEN` to `pub(crate)`, writes both fuzz target files, adds `exclude = ["fuzz"]` to root `Cargo.toml`.
  2. `05-02-PLAN.md` — `test(core,cli): criterion bench scaffolding [PERF-06]` — adds `criterion` dev-dep to both crates, adds 5 `[[bench]]` entries, creates the 5 bench files, writes the 2 README files.
- **D-32:** Order: 05-01 before 05-02. Both plans touch disjoint files (`fuzz/` vs `benches/`), so they're parallel-safe, but serial ordering keeps the commit log readable. ROADMAP marks Phase 5 parallel-safe with Phase 3 (done) and (conditionally) Phase 4 — no inter-phase coordination needed.
- **D-33:** Each commit must pass the Phase 3 D-24 gate before the next starts:
  - `cargo fmt --all --check`
  - `cargo test --workspace --all-targets --locked`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS=-D warnings`
  - Plan 05-01 additionally: smoke `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` completes without crash (manual check captured in plan; not CI).
- **D-34:** xtask gates (Phases 2/3 `env_discipline.rs` + `spawn_discipline.rs`) do NOT apply to new code — fuzz targets/benches don't mutate env nor spawn the binary. The gates' walk-roots (`crates/base60-cli/src/`, `crates/base60-cli/tests/`, etc.) don't include `fuzz/` or `benches/`, so no gate extension needed.

### J. Claude's Discretion

- Exact byte sequence for the 1 MiB bench haystacks — planner picks a deterministic generator (`(0..SIZE).map(|i| (i as u8).wrapping_mul(13).wrapping_add(7)).collect()` works); lock the seed pattern in a `const` so re-runs are bit-identical.
- Wording of the fuzz target banner comments (D-14) and bench READMEs (D-29).
- Whether `lens.rs` bench is one `criterion_group!` with four inner functions or four small groups — either renders fine with `html_reports`.
- Whether `palette-ansi` dump is included as a second bench cell in `dump.rs` (recommended: mono only for Phase 5; Phase 6 PERF-04 can extend).
- Exact shape of the compile-time test asserting `__fuzz` module is absent in non-fuzz builds — a trait-resolution trick (`struct X; impl X { #[cfg(fuzzing)] fn _has_fuzz() {} }`) or a doc-comment note; either satisfies SC5.
- Whether to land an `.cargo/config.toml` entry for `[alias] fuzz-smoke = "..."` — nice-to-have, not required by any SC.
- Exact `fuzz/.gitignore` contents — `cargo fuzz init` generates it; supplement as needed.
- Whether `criterion_group!` in each bench uses `name = my_benches; config = Criterion::default().noise_threshold(0.05).sample_size(50); targets = …` macro form vs a manual `Criterion` instance — both idiomatic.

### Folded Todos

(None — `gsd-sdk query todo.match-phase 5` returned 0 matches.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level decisions (MUST read)

- `.planning/PROJECT.md` — **Key Decisions row 7** (`parse_run`/`Pattern` stay in CLI with `#[cfg(fuzzing)] pub` hatch — do NOT move to core), **row 8** (criterion benches advisory-only, never CI-gating), **row 9** (fuzz CI weekly Ubuntu+nightly — that's Phase 7). Lines 110-123 (constraints — zero-dep core, stable JSON schema, lint bar).
- `.planning/REQUIREMENTS.md` — **TEST-02** (line 24: `cargo-fuzz` workspace at repo-root `fuzz/`, targets for `decode::parse_run` via `#[cfg(fuzzing)]` hatch and `Pattern::from_str`), **PERF-06** (line 36: `criterion` benches in both crates' `benches/` dirs, advisory-only, lands before any PERF-0X). Lines 95-100 map both to Phase 5.
- `.planning/ROADMAP.md` — **Phase 5** (lines 80-91): Goal + 5 Success Criteria. SC1 (fuzz init + exclude + two targets, `let _ = ...` pattern), SC2 (30 s no-crash run + .gitignore), SC3 (5 bench files, `harness = false`, `noise_threshold(0.05)`, `cargo bench --workspace --no-run --locked` compiles on all 3 OSes), SC4 (`benches/README.md` advisory posture), SC5 (`#[cfg(fuzzing)] pub` hatch doesn't leak public API). Lines 117-135 phase-parallelism graph.
- `.planning/STATE.md` — Lines 55, 84-85: **Open question locked here** — "Fuzz seed corpus: commit seed inputs or start empty?" → Phase 5 answer (D-09): empty.

### Prior-phase context (precedents adopted here)

- `.planning/phases/01-refactor-foundations/01-CONTEXT.md` — D-02 (`chunk::CHUNK = 8` + `chunk::be_u64` live in `crates/base60-cli/src/chunk.rs`; fuzz target for `parse_run` needs this via the `__fuzz` re-export). D-12 (atomic commits with full gate between).
- `.planning/phases/02-env-test-serialisation/02-CONTEXT.md` — **D-02 (dev-deps vs runtime deps split — CI-03 checks `[dependencies]` only)**. This precedent is load-bearing for Phase 5 D-17: `criterion` as a core dev-dep is fine.
- `.planning/phases/03-roundtrip-matrix-fixture-integration/03-CONTEXT.md` — **D-06 through D-09** (`[lib] name = "base60"`, `pub fn run()`, minimal public surface of `pub use cli::{LensMode, Format}`). Phase 5's `__fuzz` module adds a second `#[cfg(fuzzing)]`-gated `pub` surface without touching the non-fuzz exports. D-24 (between-commit gate).
- `.planning/phases/04-tighten-parse-run-close-coverage-gaps/04-CONTEXT.md` — **D-09** (`parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64>` — current signature). **D-10, D-11** (error-message contract pinned; fuzz panic discovery won't drift it). Phase 4 landed this signature; Phase 5 fuzzes it.

### Pitfall remediations this phase consumes

- `.planning/research/PITFALLS.md §"Pitfall 3"` — Fuzz targets that flag by-design rejections as crashes. Addressed by D-12, D-13, D-14 (`let _ = ...` pattern; UTF-8 guard; banner comment).
- `.planning/research/PITFALLS.md §"Pitfall 4"` — `memchr::memmem` loses to naive on 1-byte needles. Addressed by D-27 (mandatory 1-byte and 2-byte zero-fill cells in `search.rs` bench). Gates Phase 6 PERF-03.
- `.planning/research/PITFALLS.md §"Pitfall 9"` — Criterion noise floor on GHA runners. Addressed by D-21 (`noise_threshold(0.05)`) + D-29 (README documents advisory-only).
- `.planning/research/PITFALLS.md §"Pitfall 11"` — `cargo-fuzz` silent fallback on non-Linux CI. Addressed by phase scope: Phase 5 ships scaffolding only, no CI; Phase 7 CI-02 handles the Ubuntu-nightly-only job.
- `.planning/research/PITFALLS.md §"Looks Done But Isn't"` TEST section rows 2, 5 — this phase satisfies directly.

### Research outputs

- `.planning/research/STACK.md` — §"Fuzzing" (`cargo-fuzz` + `libfuzzer-sys 0.4` + workspace isolation via `--fuzzing-workspace=true`; no `arbitrary` unless needed). §"Benchmarking" (criterion 0.8 > divan for this project; `default-features = false, features = ["cargo_bench_support", "html_reports"]`; 5 bench-file shape). §"What NOT to Use" (no insta, no proptest, no iai).
- `.planning/research/ARCHITECTURE.md` — §"Integration Boundaries" Fuzz Crate ↔ Target Crates (manifest template, path-dep pattern). §"Bench Crate Layout" (file names, which crate). §"Suggested Build Order" Wave 2 item 10 (PERF-06 bench scaffolding lands before PERF-0X in Phase 6). §"Unresolved questions" (last 5 questions are already closed by PROJECT.md and STATE.md at roadmap creation time — nothing left for Phase 5 to re-ask).

### Codebase intelligence

- `.planning/codebase/CONVENTIONS.md` — `pub(crate)` default, `#[must_use]`, doc comments on every `pub(crate)`-or-above item, `#[cfg(test)] mod tests` inline style. Applies to new `__fuzz` hatch, bench harness code, and new rustdoc on the visibility-bumped `parse_run`/`RUN_LEN`.
- `.planning/codebase/STRUCTURE.md` — workspace layout; shows where `fuzz/` slots (repo root) and where `benches/` slots (inside each crate).
- `.planning/codebase/STACK.md` — current dep inventory. Confirms no pre-existing `benches/` or `fuzz/` to collide with.
- `.planning/codebase/TESTING.md` — current 182+ test inline-module idiom (post-Phase 4). Fuzz targets and benches are separate — they live outside `#[cfg(test)]` and use different harness entry points.

### Source files this phase edits or creates

**NEW (Plan 05-01 — fuzz):**
- `fuzz/Cargo.toml` — via `cargo fuzz init --fuzzing-workspace=true`, then edited to match D-03.
- `fuzz/fuzz_targets/parse_run.rs` — D-12 shape.
- `fuzz/fuzz_targets/pattern_from_str.rs` — D-13 shape.
- `fuzz/.gitignore` — auto-generated.
- `fuzz/README.md` — platform constraints, Ubuntu+nightly only, reproducer commands.

**NEW (Plan 05-02 — benches):**
- `crates/base60-core/benches/convert.rs` — D-23.
- `crates/base60-core/benches/lens.rs` — D-24.
- `crates/base60-core/benches/README.md` — D-30.
- `crates/base60-cli/benches/dump.rs` — D-25.
- `crates/base60-cli/benches/decode.rs` — D-26.
- `crates/base60-cli/benches/search.rs` — D-27.
- `crates/base60-cli/benches/README.md` — D-29.

**EDIT (Plan 05-01):**
- `Cargo.toml` — root; `[workspace]` gains `exclude = ["fuzz"]` (D-02).
- `crates/base60-cli/src/lib.rs` — add `#[doc(hidden)] #[cfg(fuzzing)] pub mod __fuzz { pub use crate::decode::{parse_run, RUN_LEN}; pub use crate::search::Pattern; }` (D-05).
- `crates/base60-cli/src/decode.rs` — `fn parse_run` → `pub(crate) fn parse_run`; `const RUN_LEN` → `pub(crate) const RUN_LEN`; rustdoc `# Errors` / `# Panics` sections added (D-06).

**EDIT (Plan 05-02):**
- `crates/base60-core/Cargo.toml` — `[dev-dependencies]` gains `criterion = { version = "0.8", default-features = false, features = ["cargo_bench_support", "html_reports"] }` (D-17); two `[[bench]] harness = false` entries (D-19).
- `crates/base60-cli/Cargo.toml` — same `criterion` dev-dep (D-16); three `[[bench]] harness = false` entries (D-20).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `crates/base60-cli/src/lib.rs` exists from Phase 3 — thin public surface (`pub use cli::{LensMode, Format};` + `pub fn run()`). Adding a `#[cfg(fuzzing)] pub mod __fuzz` sibling is a one-line addition; non-fuzz doc-gen remains pristine.
- `chunk::be_u64` (Phase 1) in `crates/base60-cli/src/chunk.rs` is `pub(crate)` — the fuzz target doesn't need it directly (`parse_run` uses it internally), but if a future fuzz target wants chunk-boundary coverage, it can re-export via the same `__fuzz` module.
- `LensMode::ALL` (Phase 1, widened to `pub` in Phase 3) + `Format::ALL` (Phase 3) — benches can iterate these for per-variant coverage if Phase 6 later extends `lens_bench.rs` / `dump_bench.rs`. Not needed in Phase 5 — the benches pick one representative case per file.
- `#[cfg(test)] mod tests` inline-module style — the `__fuzz` module does NOT live inside `#[cfg(test)]`; `#[cfg(fuzzing)]` is orthogonal (set by `cargo-fuzz` only, never by `cargo test`).
- `Pattern::from_str` (in `crates/base60-cli/src/search.rs`) — already accessible via `use std::str::FromStr; Pattern::from_str(s)`. Zero code changes needed there.
- `decode::parse_run` current signature: `fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64>` — matches the Phase 4 D-09 tightening. Exactly the shape the fuzz target will call.

### Established Patterns

- Workspace-level `[workspace.lints.clippy]` (pedantic + nursery + cargo with `-D warnings`) automatically applies to new bench code and the `__fuzz` module. Fuzz crate has its own `[workspace]` (D-01) so it escapes the main lint set — `libfuzzer-sys` doesn't play nicely with `-D warnings` and it would be theatre to enforce them there.
- `#[derive(Debug)]` / `missing_debug_implementations = warn` — bench helper types need it; `Pattern` already has it; nothing to add.
- `criterion_group!` + `criterion_main!` macros satisfy `unused_lifetimes` / `unused_qualifications` when configured with named groups (they expand to explicit item bindings).
- Doc comments (`///`) on every `pub(crate)` item — the Phase 5 bumps to `parse_run`/`RUN_LEN` need `# Errors` / `# Panics` sections per CI (`RUSTDOCFLAGS=-D warnings`).

### Integration Points

- Root `Cargo.toml` currently: `members = ["crates/base60-core", "crates/base60-cli", "crates/xtask"]`. Phase 5 appends `exclude = ["fuzz"]`. No member changes, no resolver changes.
- `crates/base60-cli/Cargo.toml` currently has `[lib] name = "base60"` + `[[bin]] name = "base60"` (Phase 3 D-06) — the fuzz crate path-deps on `base60` (package) and finds the library target. No manifest change on the CLI side beyond dev-deps.
- CI (`.github/workflows/ci.yml`) — **no changes this phase.** Phase 5 ships scaffolding. Phase 7 adds:
  - `benches-compile` step: `cargo bench --workspace --no-run --locked`.
  - `fuzz.yml` weekly schedule: `cargo +nightly fuzz run parse_run -- -max_total_time=240` + same for `pattern_from_str`.
  - `zero-dep-core` step: `cargo metadata` check against `[dependencies]` of `base60-core`.
- xtask crate (Phase 2/3) — unchanged. `env_discipline.rs` walks `src/` only (not `fuzz/` or `benches/`); `spawn_discipline.rs` walks `tests/` only. No gate-rule changes.

### Constraints from existing CI

- `cargo fmt --all --check` — every new file (fuzz targets, bench files, READMEs) must be rustfmt-clean. `fuzz/` has its own workspace; `cargo fmt` on the repo root won't walk it unless invoked with `--manifest-path`, but matching conventions is easy.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — runs on the main workspace only. Fuzz crate escapes via its `[workspace]` declaration; benches are in-crate so they DO pay the clippy bar — expect a few `#[allow(clippy::missing_panics_doc)]` or explicit `# Panics` sections in bench setup code.
- `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS=-D warnings` — applies to the `__fuzz` module when fuzzing is NOT enabled (it's `#[cfg(fuzzing)]`-gated, so it's invisible). When fuzzing IS enabled, the crate doesn't build on stable anyway. Net: no new rustdoc burden for the hatch itself, but the two `pub(crate)` bumps (`parse_run`, `RUN_LEN`) DO need `/// # Errors` / `/// # Panics` sections.
- `cargo test --workspace --all-targets --locked` — runs benches as `test` variants when declared with `harness = false` by default. `criterion` docs explicitly say `cargo test --bench <name>` works because criterion treats test-mode as a smoke test. Minor risk: `cargo test --all-targets` might run a 30-second bench once in release mode. Watch this during plan execution; if the aggregate adds more than 5 s per CI cell, switch to `#[cfg(not(test))]`-gated `criterion_main!` per file or use `[lib]`-style inner bench modules.
- 3×3 CI matrix (Ubuntu/macOS/Windows × 1.95/stable/beta) — benches must compile on all 9 cells. The `html_reports` feature pulls in `plotters` which has transitive build-script deps — risk of Windows MSVC surprise. Mitigation: verify compile during Plan 05-02 execution on a Windows CI run before commit.

### Constraints specific to Phase 5

- `fuzz/` requires nightly rustc to BUILD (not just run) because `libfuzzer-sys`'s build script emits `-Cpanic=abort` and `-Zsanitizer=address` flags. Stable/beta will reject it. Since `fuzz/` is workspace-excluded, the main matrix never tries.
- No CI-level verification of SC1-SC5 in this phase. Each SC is manually checked at plan commit time:
  - SC1: `cd fuzz && cargo +nightly fuzz build` succeeds.
  - SC2: `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` exits 0.
  - SC3: `cargo bench --workspace --no-run --locked` succeeds on Ubuntu (developer laptop); macOS/Windows verified in the first post-phase CI run (not gated in Phase 5).
  - SC4: `crates/base60-cli/benches/README.md` exists + documents advisory-only.
  - SC5: Manual `cargo doc --workspace --no-deps --locked` diff review confirms no new `pub` items under `base60` or `base60_core` outside the `#[cfg(fuzzing)]`-gated hatch.

</code_context>

<specifics>
## Specific Ideas

- Name the fuzz target binaries `parse_run` and `pattern_from_str` per ROADMAP SC1 — singular snake_case, matches cargo-fuzz convention. Avoid `decode_parse_run` / `search_pattern_from_str` — unnecessary namespacing.
- `__fuzz` module name uses double underscore to match the Python `__init__` / C `__builtin` internal-convention signal. `fuzz` alone would collide with `crate::fuzz` (which doesn't exist, but the intent is clearer with `__`).
- `fuzz/README.md` opens with: `This crate is Ubuntu + nightly only. libFuzzer requires LLVM sanitizer support (x86_64/aarch64, Unix-like, nightly-only). Main workspace CI matrix remains Ubuntu/macOS/Windows × stable/beta/1.95 because fuzz/ is workspace-excluded.` Ties it explicitly to PITFALLS Pitfall 11.
- The bench-generation helpers (e.g., deterministic 1 MiB haystack) live in a single shared module OR are duplicated inline — Claude's Discretion, but NOT in `tests/common/` (that's for integration tests). Keeping each bench file standalone (duplicated generators) is a legitimate choice for three small benches. A `crates/base60-cli/benches/common/mod.rs` (analogous to `tests/common/mod.rs`) is also idiomatic — matches the sibling layout.
- `criterion_group! { name = dump_benches; config = Criterion::default().noise_threshold(0.05).sample_size(50); targets = bench_dump_1mib_mono }` is the shape — exact function names are Claude's Discretion.
- The `libfuzzer-sys` manifest dep deliberately uses `default-features = false, features = ["link_libfuzzer"]` — drops the `arbitrary` feature which pulls in the `arbitrary` crate. We're not using `Arbitrary`-derived input types (D-15), so the default isn't needed.
- Plan 05-01's commit also adds the `# Errors` / `# Panics` doc sections on `parse_run`. Current `pub(crate)` items (Phase 3/4 work) set the precedent.
- After both plans land, `git ls-files fuzz/ crates/*/benches/` shows exactly the scaffolding: no checked-in corpus, no baselines, no compiled HTML reports.

</specifics>

<deferred>
## Deferred Ideas

### v3 or later milestones

- **`cargo-public-api --diff` tooling** — for rigorous SC5 verification and Phase 7 API-drift checks. Manual `cargo doc` review is enough for v2. Revisit if a public-surface leak escapes review.
- **Seed corpus curation** — empty on commit per D-09. Re-evaluate after two Phase 7 weekly runs; if corpus growth stalls, add `fuzz/seeds/{parse_run,pattern_from_str}/*` hand-crafted inputs.
- **Iai-Callgrind migration** — criterion stays for v2 (advisory, local-only). If perf-regression tracking becomes a real pain point in Phase 6, migrate to Iai for instruction-count-based determinism (PITFALLS Pitfall 9 option 2).
- **`arbitrary`-driven structured fuzz** — current `parse_run` and `Pattern::from_str` surface accepts raw `&[u8]`/`&str`, so random bytes with guards are ideal. If a future target (e.g., JSON-emitter fuzz) benefits from structured input, add `arbitrary = "1.4"` to `fuzz/Cargo.toml` then — not now.
- **`bencher.dev` / `codspeed.io` baseline tracking** — out of scope per REQUIREMENTS v3-deferred OBSV-02. Stay laptop-local.
- **Additional fuzz targets** — `format::emit_json` / `format::emit_html` (output side), `chunk::be_u64` (trivial — just a byte-shuffle). Current targets are the two impedance-mismatch surfaces per CONCERNS. Expand if Phase 7 CI surfaces a real bug in an adjacent module.
- **Per-lens `render_to` UTF-8 fuzz** — PITFALLS Pitfall 13 flags this for PERF-04 (Phase 6). Not Phase 5's concern.

### Out of scope by decision (not deferred, rejected)

- **Moving `parse_run` or `Pattern` to `base60-core`** — rejected in PROJECT.md Key Decision row 7. The `#[cfg(fuzzing)] pub` hatch is the chosen path.
- **`divan` instead of criterion** — rejected in STACK.md; criterion's save-and-compare is the gating feature even for local-advisory use.
- **CI-gated criterion** — rejected permanently per PROJECT.md Key Decision row 8.
- **`cargo-tarpaulin` / codecov coverage gate** — out of scope per REQUIREMENTS line 70.
- **`proptest` / `quickcheck`** — fuzz + table-driven unit tests cover the same surface; adding another framework is churn. Raised in Phase 2 discussion, re-confirmed here.

</deferred>

---

*Phase: 05-fuzz-criterion-harnesses*
*Context gathered: 2026-04-24*
*Mode: --auto (Claude selected recommended default for every gray area; inline `[auto]` log preserves the choice)*
