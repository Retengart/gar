# base60 — Sumero-Babylonian binary viewer

## What This Is

`base60` is a hex-dump alternative that renders every 8 bytes of input as eleven
sexagesimal (base-60) digit pairs — a single statically-linked Rust CLI that
ships a coloured TTY dump, four optional lenses (including actual cuneiform),
entropy-based statistical analysis, roundtrip decoding, and a ratatui-based
interactive TUI. Target user: anyone who reaches for `xxd` / `hexdump` and
wants a denser, more visually legible view of binary files.

## Core Value

Every binary blob that `base60 FILE | base60 decode` round-trips must come out
byte-identical — the visual format is opinionated, but the pipeline is
lossless.

## Requirements

### Validated

<!-- Shipped in v1 (seven-phase Sumerian roadmap, 2026-04-23). -->

- ✓ **CORE-01**: `u64` → 11 base-60 digits conversion (core invariant: `60¹¹ > u64::MAX`) — v1
- ✓ **CORE-02**: URL-safe `encode_u64` / `decode_u64` using `0-9A-Za-x` alphabet — v1
- ✓ **DUMP-01**: 8-byte-chunk coloured hex-dump view with offset column and ASCII gutter — v1
- ✓ **DUMP-02**: Magnitude-based heatmap palette (dark gray / green / yellow / red) honouring `NO_COLOR` — v1
- ✓ **LENS-01**: `time` lens — Sumerian day · beru · uš · gar with `--time-scale={gar,sec,ms}` — v1
- ✓ **LENS-02**: `angle` lens — sexagesimal deg°arcmin′arcsec.mas″ — v1
- ✓ **LENS-03**: `tablet` lens with `--purist` no-zero scribal framing — v1
- ✓ **LENS-04**: `cuneiform` lens with `NO_UNICODE` / `TERM=dumb` decimal fallback — v1
- ✓ **FMT-01**: `--format={ansi,plain,json,html}` output modes — v1
- ✓ **FMT-02**: Newline-delimited JSON schema (`offset`/`bytes`/`digits`/`ascii`/`lens`) — v1
- ✓ **FMT-03**: Self-contained HTML output with inline CSS heatmap — v1
- ✓ **TUI-01**: Interactive viewer (`-i`) with vim-style keybinds (`hjkl`, `g`/`G`, `Ctrl-d`/`Ctrl-u`) — v1
- ✓ **TUI-02**: Search (`hex:`, `str:`, quoted, auto-detect) with `n`/`N` navigation — v1
- ✓ **TUI-03**: 26-slot bookmarks (`m<letter>` / `'<letter>`) — v1
- ✓ **TUI-04**: Semantic jumps — printable runs, zero-runs, entropy spikes (`]p`/`]z`/`]e` + reverse) — v1
- ✓ **TUI-05**: Per-file state persistence under `$XDG_STATE_HOME/base60/` (cursor, scroll, lens, bookmarks) — v1
- ✓ **ANALYZE-01**: `base60 analyze` — Shannon entropy, byte histogram, top-bytes, region detection — v1
- ✓ **ANALYZE-02**: `--window N` tuning for Shannon window size — v1
- ✓ **DECODE-01**: `base60 decode` roundtrip — scans ANSI-interspersed dumps, recovers bytes — v1
- ✓ **CMPL-01**: `base60 completions {bash,zsh,fish,elvish,powershell}` — v1
- ✓ **WS-01**: Workspace split — `base60-core` (zero-dep library) + `base60-cli` (binary) — v1
- ✓ **CI-01**: GitHub Actions — fmt / clippy (pedantic+nursery+cargo) / doc / test across Ubuntu/macOS/Windows × rustc 1.95/stable/beta — v1

### Active

<!-- v2 hardening milestone — no new user-facing features; focus on correctness, performance, and maintainability. -->

- [ ] **TEST-01**: End-to-end dump↔decode roundtrip tests across every `--lens` × `--format` combination
- [ ] **TEST-02**: `cargo-fuzz` targets for `decode::parse_run` and `search::Pattern::from_str`
- [ ] **TEST-03**: Fixture-driven integration tests against real binaries (ELF / PNG / ZIP / zero-fill) via `assert_cmd`
- [ ] **TEST-04**: Serialise env-mutating tests with `serial_test` to eliminate CI flakes
- [ ] **TEST-05**: Cover the mmap/stdin/file-open paths in `reader.rs` and the TUI exit-with-save path
- [ ] **PERF-01**: Stream stdin in non-TUI dump path — no OOM on `base60 < /dev/sda`
- [ ] **PERF-02**: Async / lazy `analyze` in TUI — don't block first frame on big files
- [ ] **PERF-03**: `memchr::memmem` for `search::find_all` — strictly faster, zero new deps
- [ ] **PERF-04**: Streaming `Lens::render_to<W>` default method — skip per-line `String` allocation
- [ ] **PERF-05**: Streaming entropy-window sparkline — online accumulation, no `Vec<f32>` materialisation
- [ ] **PERF-06**: `criterion` benchmarks gating each perf change (guardrail, not user feature)
- [ ] **REF-01**: Promote `be_u64` into `base60-core` — one source of truth for chunk decoding
- [ ] **REF-02**: Drive `LensMode` dispatch from a single table (`strum::EnumIter` or equivalent)
- [ ] **REF-03**: Tighten `decode::parse_run` contract — take `&[u8; RUN_LEN]`, promote digit-check inside

### Out of Scope

<!-- Explicit exclusions for the v2 hardening milestone. -->

- **New lens modes** — current four cover demonstrated demand; broader visual surface area is v3 territory
- **`--endian=little` flag** — would require header-marker protocol in `decode`; user feedback hasn't asked for it
- **Streaming hash/CRC column** — adjacent to `analyze` but grows the per-line rendering budget; defer
- **Bookmark notes/labels** — current 26-slot model is minimal and works; user-strings raise persistence-security surface
- **Man-page generation** — shell completions already cover discoverability; man pages duplicate `--help`
- **Publishing to crates.io** — workspace is `publish = false` and consumed via `cargo install --path` / git
- **Unsafe-block elimination** — the two surviving `unsafe` blocks (mmap, env-var tests) are acknowledged and gated

## Context

**Codebase status:** Brownfield. All seven phases of the original Sumerian
roadmap (`docs/plans/2026-04-23-sumerian-roadmap.md`) are implemented and
released as v1. Codebase map exists under `.planning/codebase/`
(ARCHITECTURE, CONCERNS, CONVENTIONS, INTEGRATIONS, STACK, STRUCTURE,
TESTING). CI is green across three OSes and three rustc channels.

**Discipline baseline:** Zero `TODO`/`FIXME`/`HACK` markers in the codebase.
Every numeric cast is annotated; every potential overflow uses `checked_*` /
`saturating_*`. Workspace-level clippy profile is `pedantic + nursery +
cargo` with `-D warnings` on every CI target. `#![forbid(unsafe_op_in_unsafe_fn)]`
on the binary.

**Known pain points driving v2:**
- `be_u64` is duplicated between `dump.rs` and `format.rs` with a module-level
  comment acknowledging the copy. Divergence would silently break JSON/HTML.
- Lens dispatch is spread across four parallel switch statements
  (`cli.rs:44-89`, `persist.rs:139-147`). Adding a lens forgets at least one.
- Env-touching tests call `unsafe { std::env::set_var }` and rely on
  "don't run concurrently" by convention — Cargo default is multi-threaded.
- `stdin().read_to_end` and whole-file TUI `analyze` both materialise the
  entire input in RAM before returning.
- No integration tests. No fuzz targets. `tests/` directory doesn't exist.
- Dump→decode roundtrip is only tested with hand-crafted ASCII lines, never
  with the output of `dump_all` under an active lens.

## Constraints

- **Tech stack**: Rust edition 2024, MSRV `1.95`, single binary via
  `cargo install`. No runtime dependencies, no service, no daemon.
- **Library API**: `base60-core` must keep zero external dependencies —
  its selling point.
- **Backwards compatibility**: JSON schema and `decode` accept-format are
  stable. Any change must be additive and gated.
- **Output determinism**: `NO_COLOR`, `NO_UNICODE`, `TERM=dumb` behaviours
  are contractual. State-file byte ordering is deterministic by explicit sort.
- **Platform**: CI matrix of Ubuntu/macOS/Windows × rustc 1.95/stable/beta
  is the correctness floor. Nothing may regress any of these.
- **Lint bar**: `clippy::pedantic + nursery + cargo` with `-D warnings` stays
  enforced. `multiple_crate_versions` and `module_name_repetitions` are the
  only documented allows.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| v2 is hardening-only — no new user-facing features | CONCERNS.md lists real risks (duplication, untested paths, unbounded stdin); v1 feature surface is already rich | — Pending |
| Integration-test crate goes under `crates/base60-cli/tests/` | CLI-side roundtrip is the integration boundary; `base60-core` stays zero-dep and unit-tested | — Pending |
| Fuzz crate at repo-root `fuzz/` as excluded workspace member | Standard `cargo-fuzz` layout; nightly tooling and ASAN stay off the main-workspace MSRV floor | — Pending |
| Streaming stdin path applies to non-TUI dump only | TUI genuinely needs random-access slice for bookmarks/search; preserve mmap path unchanged | — Pending |
| `be_u64` moves to `crates/base60-cli/src/chunk.rs` (CLI-local, not core) | Protects `base60-core` public API surface from internal CLI concerns; single source of truth within CLI; core stays zero-dep without chunk-decoding responsibility | — Pending |
| `LensMode` dispatch driven by hand-rolled `const ALL: &[LensMode]` table, not `strum` in core | `base60-core` zero-dep constraint beats compile-time convenience; four variants don't justify a proc-macro dep | — Pending |
| `parse_run` / `Pattern` stay in `base60-cli` with `#[cfg(fuzzing)] pub` hatch for fuzz targets | Keeps public library API surface minimal; fuzz crate imports via cfg-gated visibility | — Pending |
| Criterion benches are advisory, not CI-gating | Shared GitHub Actions runners have noise floor > typical signal; baselines stay local, numbers pasted in PR descriptions | — Pending |
| Fuzz CI runs on a weekly `schedule:` workflow, Ubuntu + dated nightly only | `libFuzzer` is Linux-nightly-only; per-PR runs blow the time budget; weekly covers drift | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-23 after initialization*
