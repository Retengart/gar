# Requirements: base60 v2 (Hardening Milestone)

**Defined:** 2026-04-23
**Core Value:** Every binary that `base60 FILE | base60 decode` round-trips must come out byte-identical.

This milestone is hardening-only. No new user-facing features. Three themes: (1) integration tests + fuzz, (2) streaming + performance, (3) refactor consolidation. v1 feature surface is locked; everything here is correctness, performance, or maintainability.

## v1 (shipped) Requirements

See `.planning/PROJECT.md` → Requirements → Validated. 23 shipped capabilities (CORE-01, DUMP-01..02, LENS-01..04, FMT-01..03, TUI-01..05, ANALYZE-01..02, DECODE-01, CMPL-01, WS-01, CI-01). Locked — any change requires explicit discussion.

## v2 Requirements (Active)

### Refactor

- [ ] **REF-01**: De-duplicate `be_u64` into a single CLI-local module (`crates/base60-cli/src/chunk.rs`); `dump.rs` and `format.rs` import it. `base60-core` surface unchanged.
- [ ] **REF-02**: Drive `LensMode` dispatch from one hand-rolled `const ALL: &[LensMode]` table in `cli.rs`. `build_lens` / `cycle` / `label` / `persist::parse_lens` all read from it. Adding a new variant touches one site.
- [ ] **REF-03**: Tighten `decode::parse_run` contract — accept `&[u8; RUN_LEN]` (not `&str`), promote digit-check inside the function. No caller can bypass the digit-run gate.

### Test Infrastructure

- [ ] **TEST-01**: Fixture-driven roundtrip matrix — `base60-cli/tests/roundtrip.rs` asserts byte-identical recovery for every `LensMode × FormatMode × ColorMode` combination against a minimum corpus (ELF, PNG, ZIP, zero-fill).
- [ ] **TEST-02**: `cargo-fuzz` workspace at repo-root `fuzz/` (excluded from main workspace); targets for `decode::parse_run` (via `#[cfg(fuzzing)]` hatch) and `Pattern::from_str`. Runnable via `cargo +nightly fuzz run <target>`.
- [ ] **TEST-03**: `assert_cmd`-driven integration tests — `crates/base60-cli/tests/cli.rs` covers the dump/analyze/decode/completions entry points including stdin piping and broken-pipe behaviour.
- [ ] **TEST-04**: `serial_test = "3"` adopted for every env-mutating test (`NO_COLOR`, `NO_UNICODE`). All use one shared `#[serial(env)]` key to prevent per-variable races.
- [ ] **TEST-05**: Coverage for currently-untested paths — `reader::load_file` (mmap), `reader::load_stdin`, `tui` exit-with-save via `TestBackend`, `persist::state_base_dir` env-precedence logic.

### Performance

- [ ] **PERF-01**: Streaming stdin path for non-TUI dump — `base60 < /dev/sda` (or any >RAM input) completes without OOM. Smoke-test proves bounded peak RSS.
- [ ] **PERF-02**: Lazy / async `analyze` in the TUI — first frame draws within one render tick regardless of file size; semantic-jump keys show "analysing…" until ready.
- [ ] **PERF-03**: `memchr::memmem::find_iter` replaces the naïve `search::find_all` scan. Length-dispatch for 1-byte needles uses `memchr::memchr_iter` (avoids the short-needle regression).
- [ ] **PERF-04**: `Lens` trait gains `fn render_to<W: Write>(&self, chunk: u64, w: &mut W) -> io::Result<()>` default method. Streaming dump path uses `render_to` and skips the per-line `String` allocation. Existing `render(&self, u64) -> String` stays for the TUI path.
- [ ] **PERF-05**: Streaming `entropy_windows` — no materialised `Vec<f32>` for window sparkline; online min/max/mean accumulation.
- [ ] **PERF-06**: `criterion` benches in `crates/base60-core/benches/` (convert, lens) and `crates/base60-cli/benches/` (dump, decode, search). Advisory-only — run locally, not CI-gating. Lands before any PERF-0X change so each perf PR has a before/after baseline.

### CI / Tooling

- [ ] **CI-02**: Weekly scheduled GitHub Actions job — `cargo +nightly fuzz run parse_run -- -max_total_time=240` on `ubuntu-latest` only, non-gating, 5-minute timeout.
- [ ] **CI-03**: `base60-core` zero-dep invariant enforced in CI — grep check that the `[dependencies]` section of `crates/base60-core/Cargo.toml` stays empty.

## v3 Requirements (Deferred)

### New feature surface

- **FEAT-01**: `--endian=little` flag with header-marker protocol in `decode`
- **FEAT-02**: Streaming hash / CRC column in dump output
- **FEAT-03**: Bookmark notes / labels with per-user salted SipHasher13 keying
- **FEAT-04**: Additional lens modes (fixed-point, BCD, duration)
- **FEAT-05**: Publish `base60-core` and `base60-cli` to crates.io

### Observability

- **OBSV-01**: Deterministic peak-RSS measurement in CI (self-hosted runner or external service)
- **OBSV-02**: Criterion baseline tracking via Bencher.dev / CodSpeed integration
- **OBSV-03**: Coverage reporting (carefully — codebase has intentional `unsafe` blocks)

## Out of Scope

| Feature | Reason |
|---------|--------|
| `--endian=little` flag | v2 theme is hardening, not features; explicit in PROJECT.md Out of Scope |
| Streaming hash/CRC column | Grows per-line rendering budget; user didn't prioritise |
| Bookmark labels | Current 26-slot model is minimal and works; raises persistence-security surface |
| Man-page generation | Shell completions already cover discoverability |
| Publish to crates.io | Workspace is `publish = false`; `cargo install --path` path is stable |
| Unsafe-block elimination | Two surviving blocks (mmap, env-var tests) are explicitly acknowledged and gated |
| New lens modes | Demonstrated-demand threshold not met; v3 territory |
| `cargo-tarpaulin` / codecov coverage gate | Conflicts with intentional `#[cfg(test)] unsafe` blocks; coverage theatre |
| `iai-callgrind` deterministic benchmarks | Breaks macOS / Windows CI matrix; revisit if criterion local-only proves insufficient |
| Property testing via `proptest` / `quickcheck` | Fuzz + table tests cover the same ground |
| Snapshot testing via `insta` | Byte-stable dump/JSON/HTML formats don't benefit; churn risk |
| `cargo-audit` / `cargo-deny` as blocking CI gate | Zero-dep core + small dep graph keeps CVE surface tiny; re-evaluate at v3 |
| `cargo-tarpaulin` coverage badges | Vanity metric; doesn't improve code quality |
| `strum` derive in `base60-core` | Violates zero-dep-core invariant; four LensMode variants don't justify a proc-macro dep |
| Reproducible-builds infrastructure | Workspace is `publish = false`; no downstream attestation need |
| `nextest` as required test runner | `cargo test` works fine for this workspace size; parallel speedup marginal |

## Traceability

Each v2 REQ-ID maps to exactly one phase in ROADMAP.md. No orphans, no duplicates.

| Requirement | Phase | Status |
|-------------|-------|--------|
| REF-01 | Phase 1 — Refactor Foundations | Pending |
| REF-02 | Phase 1 — Refactor Foundations | Pending |
| REF-03 | Phase 4 — Tighten parse_run + Close Coverage Gaps | Pending |
| TEST-01 | Phase 3 — Roundtrip Matrix + Fixture Integration | Pending |
| TEST-02 | Phase 5 — Fuzz + Criterion Harnesses | Pending |
| TEST-03 | Phase 3 — Roundtrip Matrix + Fixture Integration | Pending |
| TEST-04 | Phase 2 — Env-Test Serialisation | Pending |
| TEST-05 | Phase 4 — Tighten parse_run + Close Coverage Gaps | Pending |
| PERF-01 | Phase 6 — Streaming + Performance Pass | Pending |
| PERF-02 | Phase 6 — Streaming + Performance Pass | Pending |
| PERF-03 | Phase 6 — Streaming + Performance Pass | Pending |
| PERF-04 | Phase 6 — Streaming + Performance Pass | Pending |
| PERF-05 | Phase 6 — Streaming + Performance Pass | Pending |
| PERF-06 | Phase 5 — Fuzz + Criterion Harnesses | Pending |
| CI-02 | Phase 7 — CI Hardening | Pending |
| CI-03 | Phase 7 — CI Hardening | Pending |

**Coverage:**
- v2 requirements: 16 total
- Mapped to phases: 16 ✓
- Unmapped: 0

**Phase load:**
- Phase 1: REF-01, REF-02 (2)
- Phase 2: TEST-04 (1)
- Phase 3: TEST-01, TEST-03 (2)
- Phase 4: REF-03, TEST-05 (2)
- Phase 5: TEST-02, PERF-06 (2)
- Phase 6: PERF-01, PERF-02, PERF-03, PERF-04, PERF-05 (5)
- Phase 7: CI-02, CI-03 (2)

**Critical ordering** (enforced by phase graph in ROADMAP.md):
- TEST-01 (Phase 3) precedes REF-03 (Phase 4) — roundtrip safety net
- PERF-06 (Phase 5) precedes PERF-01..05 (Phase 6) — each perf PR needs a baseline
- TEST-04 (Phase 2) precedes any new env-mutating test (Phase 3, Phase 4)

---
*Requirements defined: 2026-04-23*
*Traceability populated: 2026-04-24 by roadmapper*
