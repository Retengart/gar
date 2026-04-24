---
phase: 5
slug: fuzz-criterion-harnesses
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-24
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> **Phase 5 ships scaffolding only — no runtime behaviour change.** Manual fuzz+bench smokes sit alongside the existing `cargo test --workspace --all-targets --locked` CI baseline. Do NOT claim CI coverage for fuzz runs — that ships in Phase 7 CI-02.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` + `cargo test` (existing); criterion 0.8 for bench compile-smoke; libFuzzer via `cargo-fuzz` 0.13 (manual-only this phase) |
| **Config file** | None — rustfmt / clippy / doc config inherited from workspace `Cargo.toml` `[workspace.lints]` |
| **Quick run command** | `cargo test --workspace --all-targets --locked` |
| **Full suite command** | `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --all-targets --locked && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` (Phase 3 D-24 gate — reused verbatim) |
| **Estimated runtime** | ~90 s full suite on a modern dev laptop; `cargo test` alone ~20 s |

Manual-only auxiliary commands (not for CI until Phase 7):

- Fuzz compile smoke (Plan 05-01): `cd fuzz && cargo +nightly fuzz build`
- Fuzz no-crash smoke (Plan 05-01, SC2): `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` (expect exit 0)
- Bench compile smoke (Plan 05-02): `cargo bench --workspace --no-run --locked`

---

## Sampling Rate

- **After every task commit:** `cargo test --workspace --all-targets --locked` (quick — ~20 s). Includes `[[bench]]` target compile because `--all-targets` covers bench targets.
- **After every plan wave:** Full suite gate (fmt + clippy + test + doc). ~90 s.
- **Before `/gsd-verify-work` / phase transition:** Full suite green PLUS `cargo bench --workspace --no-run --locked` (Plan 05-02) PLUS `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` (Plan 05-01).
- **Max feedback latency:** 90 s (full suite).

---

## Per-Task Verification Map

> Task IDs are provisional — refined in PLAN.md files. Wave 0 = "entire Phase 5 surface" per RESEARCH.md (nothing exists pre-phase; planner creates it).

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 01 | 1 | TEST-02 SC1 | — | Fuzz crate compiles with `--cfg fuzzing` propagation | compile (manual) | `cd fuzz && cargo +nightly fuzz build` | ❌ W0 | ⬜ pending |
| 05-01-02 | 01 | 1 | TEST-02 SC1 | V5 / Pitfall 3 | `let _ = ...` + length guard on `parse_run` target — no `unwrap()` | grep + compile | `grep -q 'let _ =' fuzz/fuzz_targets/parse_run.rs && ! grep -q 'unwrap()' fuzz/fuzz_targets/parse_run.rs` | ❌ W0 | ⬜ pending |
| 05-01-03 | 01 | 1 | TEST-02 SC1 | V5 / Pitfall 3 | `let _ = ...` + UTF-8 guard on `pattern_from_str` target | grep + compile | `grep -q 'std::str::from_utf8' fuzz/fuzz_targets/pattern_from_str.rs && ! grep -q 'unwrap()' fuzz/fuzz_targets/pattern_from_str.rs` | ❌ W0 | ⬜ pending |
| 05-01-04 | 01 | 1 | TEST-02 SC2 | — | 30 s fuzz run completes without crash | manual smoke | `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` | ❌ W0 | ⬜ pending |
| 05-01-05 | 01 | 1 | TEST-02 SC5 | — | `__fuzz` module absent in non-fuzz builds | unit test + `cargo doc` diff | Inline `#[test] #[cfg(not(fuzzing))] fn fuzz_module_absent()`; manual `cargo doc` diff | ❌ W0 | ⬜ pending |
| 05-01-06 | 01 | 1 | TEST-02 SC1 | — | `exclude = ["fuzz"]` in root Cargo.toml; `fuzz/.gitignore` contains `corpus`, `artifacts`, `target` | grep | `grep -q 'exclude = \[.*fuzz' Cargo.toml && grep -q '^corpus$' fuzz/.gitignore && grep -q '^artifacts$' fuzz/.gitignore` | ❌ W0 | ⬜ pending |
| 05-01-07 | 01 | 1 | TEST-02 SC1 | — | `parse_run` bumped to `pub(crate) fn`; `RUN_LEN` bumped to `pub(crate) const`; rustdoc `# Errors` present; passes existing CI doc gate | grep + `cargo doc` | `grep -q 'pub(crate) fn parse_run' crates/base60-cli/src/decode.rs && grep -q 'pub(crate) const RUN_LEN' crates/base60-cli/src/decode.rs` + `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --locked` | ❌ W0 | ⬜ pending |
| 05-02-01 | 02 | 2 | PERF-06 SC3 | — | `criterion = { version = "0.8", default-features = false, features = ["cargo_bench_support", "html_reports"] }` in `[dev-dependencies]` of both crates; zero-dep runtime invariant preserved | grep | `grep -q 'criterion = { version = "0.8"' crates/base60-core/Cargo.toml crates/base60-cli/Cargo.toml && ! grep -A3 '^\[dependencies\]' crates/base60-core/Cargo.toml \| grep -q 'criterion'` | ❌ W0 | ⬜ pending |
| 05-02-02 | 02 | 2 | PERF-06 SC3 | — | 5 `[[bench]] name = "…" harness = false` entries (2 in core, 3 in cli); all compile | `cargo bench --no-run` | `cargo bench --workspace --no-run --locked` | ❌ W0 | ⬜ pending |
| 05-02-03 | 02 | 2 | PERF-06 SC3 | Pitfall 9 | Every bench calls `Criterion::default().noise_threshold(0.05)` | grep | `grep -rq 'noise_threshold(0.05)' crates/base60-core/benches/ crates/base60-cli/benches/` (expect ≥5 hits) | ❌ W0 | ⬜ pending |
| 05-02-04 | 02 | 2 | PERF-06 SC3 | Pitfall 4 | `search.rs` bench contains the 4 mandatory cells: 1-byte/zero-fill, 2-byte/zero-fill, 3-byte/random, 8-byte/random | grep | `grep -c 'zero_fill\|ELF\|cafebabe' crates/base60-cli/benches/search.rs` (expect ≥4) | ❌ W0 | ⬜ pending |
| 05-02-05 | 02 | 2 | PERF-06 SC4 | — | `crates/base60-cli/benches/README.md` documents advisory-only posture with the exact phrase "advisory only" AND "never CI-gated" (or equivalent) AND reproducer commands | grep | `grep -q 'advisory' crates/base60-cli/benches/README.md && grep -q 'baseline' crates/base60-cli/benches/README.md` | ❌ W0 | ⬜ pending |
| 05-02-06 | 02 | 2 | PERF-06 SC4 | — | `crates/base60-core/benches/README.md` points at the CLI README | grep | `grep -q 'base60-cli/benches/README.md' crates/base60-core/benches/README.md` | ❌ W0 | ⬜ pending |
| 05-02-07 | 02 | 2 | PERF-06 SC3 | — | Bench compile check on all 3 OS × 3 rustc CI cells (inherits from existing `cargo test --all-targets`) | CI | Existing `cargo test --workspace --all-targets --locked` on Ubuntu/macOS/Windows × 1.95/stable/beta | ✓ (existing) | ⬜ pending |
| 05-02-08 | 02 | 2 | TEST-02 SC5 | — | (If planner chooses `__bench` re-export hatch) `#[doc(hidden)] pub mod __bench` in lib.rs — verify no new visible items in `cargo doc` output | `cargo doc` review | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --locked` — no new `pub` items beyond Phase 3 surface | ❌ W0 (conditional) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

All files below do NOT exist pre-phase; the planner creates them during Wave 0 of their respective plan.

**Plan 05-01 creates:**

- [ ] `fuzz/Cargo.toml` — post-`cargo fuzz init` hand-edit (nested `[workspace]`, two `[[bin]]` blocks, `package.metadata.cargo-fuzz = true`, path-deps on both crates, `libfuzzer-sys = "0.4"` with `default-features = false, features = ["link_libfuzzer"]`)
- [ ] `fuzz/fuzz_targets/parse_run.rs` — length-guarded target per D-12
- [ ] `fuzz/fuzz_targets/pattern_from_str.rs` — UTF-8-guarded target per D-13
- [ ] `fuzz/.gitignore` — auto-generated, verify `target`, `corpus`, `artifacts`
- [ ] `fuzz/README.md` — Ubuntu+nightly constraint, reproducer commands
- [ ] Root `Cargo.toml` — add `exclude = ["fuzz"]`
- [ ] `crates/base60-cli/src/lib.rs` — add `#[doc(hidden)] #[cfg(fuzzing)] pub mod __fuzz { … }` + optional `#[test] #[cfg(not(fuzzing))] fn fuzz_module_absent()`
- [ ] `crates/base60-cli/src/decode.rs` — `fn parse_run` → `pub(crate) fn parse_run` + `const RUN_LEN` → `pub(crate) const RUN_LEN` + rustdoc `# Errors` / `# Panics` sections
- [ ] Developer/CI environment: `cargo install cargo-fuzz` + `rustup toolchain install nightly` (for manual smoke; Phase 7 CI-02 installs it automatically)

**Plan 05-02 creates:**

- [ ] `crates/base60-core/Cargo.toml` — `criterion` `[dev-dependencies]` entry + 2 `[[bench]] harness = false` blocks
- [ ] `crates/base60-cli/Cargo.toml` — same `criterion` dev-dep + 3 `[[bench]] harness = false` blocks
- [ ] `crates/base60-core/benches/convert.rs` — `u64_to_base60` hot loop, 1024 deterministic `u64` inputs, `noise_threshold(0.05)`, `sample_size(50)`
- [ ] `crates/base60-core/benches/lens.rs` — `render(&self, u64) -> String` per lens (4 lenses), `CuneiformLens { fallback: true }` (avoid env-reading `auto()`)
- [ ] `crates/base60-core/benches/README.md` — one-line pointer at CLI README
- [ ] `crates/base60-cli/benches/dump.rs` — `dump_all` over 1 MiB deterministic array, `PALETTE_NONE` (mono path)
- [ ] `crates/base60-cli/benches/decode.rs` — pre-computed 1 MiB dump via `LazyLock`; bench only measures `decode_stream`
- [ ] `crates/base60-cli/benches/search.rs` — 4 cells (Pitfall 4 mandatory): 1-byte/zero-fill, 2-byte/zero-fill, 3-byte `ELF` / random, 8-byte `cafebabe` / random
- [ ] `crates/base60-cli/benches/README.md` — advisory-only posture + `--save-baseline` / `--baseline` reproducer commands
- [ ] (Conditional) `crates/base60-cli/src/lib.rs` — add `#[doc(hidden)] pub mod __bench { … }` if planner chooses the re-export shim over per-item `pub` widening

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `cd fuzz && cargo +nightly fuzz build` compiles the fuzz crate | TEST-02 SC1 | Requires nightly toolchain + `cargo-fuzz` install. Phase 5 does NOT add CI; Phase 7 CI-02 does. | Developer runs once locally before committing Plan 05-01. Paste exit code into commit body. |
| `cd fuzz && cargo +nightly fuzz run parse_run -- -max_total_time=30` exits 0 | TEST-02 SC2 | Same as above — nightly + `cargo-fuzz` only. 30-second smoke; Phase 7 CI-02 extends to 240 s weekly. | Developer runs once. Paste "exit 0" + corpus-growth count into commit body. |
| `cd fuzz && cargo +nightly fuzz run pattern_from_str -- -max_total_time=30` exits 0 | TEST-02 SC2 | Same. | Developer runs once. Same reporting. |
| `cargo bench --workspace --no-run --locked` compiles all 5 benches | PERF-06 SC3 | Long-form `cargo bench` CAN run in CI, but Phase 5 scope is "compile only" and the existing `cargo test --all-targets` already catches bench compile. Full run is a human decision. | Developer runs locally at Plan 05-02 commit time to confirm bench code compiles on their machine. Ubuntu/macOS/Windows verify via existing CI matrix's `--all-targets`. |
| Criterion output sanity check | PERF-06 SC4 | Local-only advisory run to confirm the HTML reports + baseline save-restore actually work end-to-end. Not a pass/fail gate; if broken in a cell, fix then. | `cd crates/base60-cli && cargo bench --bench search -- --save-baseline pre` then inspect `target/criterion/`. One-off after Plan 05-02. |
| `Cargo.lock` diff review after criterion lands | V10 Supply Chain | First time criterion resolves, a handful of transitive deps land (plotters, oorandom, ciborium, cc build-dep). Reviewer eyeballs the diff; no automated tool. | Reviewer reads the Plan 05-02 commit diff for `Cargo.lock` additions; flags anything outside the expected set (STACK.md / RESEARCH.md §Security Domain Defensive notes). |
| `cargo doc --workspace --no-deps --locked` shows no new public items | TEST-02 SC5 | `cargo-public-api` deferred to v3; manual doc diff is acceptable for a single-phase-sized change. | Before committing Plan 05-01, run `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --locked`, open the HTML output, confirm no new `pub` items in `base60::` or `base60_core::` beyond Phase 3 surface. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — ✅ 14 tasks, each with a grep/compile/manual check
- [x] Sampling continuity: no 3 consecutive tasks without automated verify — ✅ Every task has a grep or compile check; 3 manual-smokes are interleaved with grep-verifiable tasks
- [x] Wave 0 covers all MISSING references — ✅ Entire Phase 5 surface is Wave 0; all files enumerated above
- [x] No watch-mode flags — ✅ All commands are one-shot
- [x] Feedback latency < 90s — ✅ Full suite gate (~90 s); quick test ~20 s
- [x] `nyquist_compliant: true` set in frontmatter — pending planner sign-off after PLAN.md files exist

**Approval:** pending (planner sets `nyquist_compliant: true` after Plan 05-01 and Plan 05-02 pass `gsd-plan-checker`)
