# Phase 3: Roundtrip Matrix + Fixture Integration - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-24
**Phase:** 03-roundtrip-matrix-fixture-integration
**Areas discussed:** Matrix axes & cell count; Dispatch-table access from tests; cli.rs / fixtures.rs split + common/; Test granularity & reporting

---

## Matrix axes & cell count

### Q1 — How should color show up in the roundtrip matrix?

| Option | Description | Selected |
|--------|-------------|----------|
| Single `--color=never` | Force `--color=never` on every roundtrip cell; cover color auto/NO_COLOR/always/never as focused cli.rs edge tests | ✓ |
| Color as 3rd matrix axis | 300 cells per fixture, 1500 total; matches literal REQUIREMENTS wording | |
| Color=never + color=always spot-check | 100 roundtrip cells + 5-cell ansi-color sanity | |

**Rationale:** decode's digit-run scanner ignores ANSI escapes; color is orthogonal to byte-identity.

### Q2 — TimeLens `--time-scale={Gar,Sec,Ms}`?

| Option | Description | Selected |
|--------|-------------|----------|
| Default (Gar) only | Matrix uses clap-default TimeScale::Gar | |
| All 3 scales in matrix | 5 lens rows → 7 effective; catches future TimeScale-ties-to-digit-runs regressions | ✓ |
| Gar + Sec | Middle ground | |

### Q3 — TabletLens `--purist`?

| Option | Description | Selected |
|--------|-------------|----------|
| Default (non-purist) only | Focused unit test already covers --purist inline in lens.rs | ✓ |
| Both in matrix | Add 1 row | |

### Q4 — Fixture corpus

| Option | Description | Selected |
|--------|-------------|----------|
| Keep ROADMAP 5 | minimal ELF, minimal PNG, minimal ZIP, 1 KiB zero-fill, hello-world | ✓ |
| Add 6th non-8-aligned fixture | hello-world already 14 bytes → covers short-tail | |
| Reduce to 3 (ELF + zero-fill + hello-world) | Simpler; loses container-format stories | |

**Effective matrix:** 7 lens-config rows × 4 formats × 5 fixtures = **140 cells**.

---

## Dispatch-table access from tests

### Q5 — How do tests/ reach LensMode::ALL and Format::ALL?

| Option | Description | Selected |
|--------|-------------|----------|
| Add thin `[lib]` target to base60-cli | Standard Rust CLI pattern (ripgrep, fd). main.rs shims to `base60::run()`; LensMode::ALL widens to `pub` | ✓ |
| Re-declare in tests/common + parse `--help` | Fragile on clap version bumps | |
| Mirror in tests/common + trust sync | Violates REF-02's "one source of truth" | |

### Q6 — Add Format::ALL in Phase 3?

| Option | Description | Selected |
|--------|-------------|----------|
| Add Format::ALL with exhaustiveness test | Mirrors REF-02's pattern; touches cli.rs only | ✓ |
| Hand-roll the list in tests/ only | Pattern-inconsistent with LensMode | |

### Q7 — Lib surface scope

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal: LensMode + Format + their ALLs | `pub use cli::{LensMode, Format};`; rest pub(crate) | ✓ |
| Broad re-export | Expose cli, decode, search, persist as `pub mod` | |
| Via `#[cfg(any(test, feature="testing"))]` feature flag | Zero impact on non-test; more Cargo setup | |

---

## cli.rs / fixtures.rs split + common/

### Q8 — File responsibility split

| Option | Description | Selected |
|--------|-------------|----------|
| Matrix / subcommand / edges | roundtrip.rs = matrix; fixtures.rs = per-subcommand happy path × fixture; cli.rs = stdin/BrokenPipe/flag/edge tests | ✓ |
| Matrix / everything-else | Drop cli.rs; fixtures.rs owns all subcommand + edge coverage | |
| Matrix / flag-edges / subcommand-edges | Swaps which file owns stdin/broken-pipe | |

### Q9 — common/ scope

| Option | Description | Selected |
|--------|-------------|----------|
| Helper + fixtures + assert co-located | Single `common/mod.rs` with base60_cmd + fixtures + assert_roundtrip | ✓ |
| Split into common/{helpers,fixtures,assert}.rs | Cleaner if any grows past ~200 lines | |
| Helper only; fixtures inline per test file | Violates Pitfall 7 prevention spirit | |

### Q10 — Grep gate

| Option | Description | Selected |
|--------|-------------|----------|
| Extend xtask | Phase 2's xtask gains tests/spawn_discipline.rs | ✓ |
| New one-off test in tests/cli.rs | Scatters discipline enforcement | |
| Shell script + CI step only | Doesn't fire on local `cargo test` | |

### Q11 — Dev-deps

| Option | Description | Selected |
|--------|-------------|----------|
| assert_cmd + predicates | tempfile deferred to Phase 4 | ✓ |
| assert_cmd + predicates + tempfile | Preemptive Phase 4 dep | |
| assert_cmd only | Verbose call sites without predicates | |

---

## Test granularity & reporting

### Q12 — Matrix expression

| Option | Description | Selected |
|--------|-------------|----------|
| Single `#[test]` with nested loops + `{cell}` context on failure | One libtest line, cheap compile | ✓ |
| One `#[test]` per Format (4 tests) | Middle ground: 4 libtest lines | |
| `#[test]` per cell via macro (140 lines) | Slower compile; richer CI artifacts | |

### Q13 — Matrix iteration

| Option | Description | Selected |
|--------|-------------|----------|
| Helper enum in common/ | `pub enum LensConfig { None, Time(TimeScale), ... }` + `ALL_LENS_CONFIGS` (7 entries) | ✓ |
| Flatten to `(LensMode, Option<TimeScale>)` tuples | More pattern-matching in helpers | |
| Two nested loops with skip-for-non-Time | Conditional cell count harder to reason about | |

### Q14 — Spawn count mitigation

| Option | Description | Selected |
|--------|-------------|----------|
| Accept full binary roundtrip (280 spawns/cell) | ~8-22 s added per CI matrix cell; acceptable | ✓ |
| Hybrid: spawn once, decode in-process | Halves spawns; partially defeats end-to-end | |
| Hybrid: dump in-process, spawn only decode | Most invasive; promotes pub(crate) surface | |

### Q15 — Failure diagnostics

| Option | Description | Selected |
|--------|-------------|----------|
| Cell identity + byte diff summary | Cell + first-diverge index + ±8-byte hex window | ✓ |
| Cell identity + raw `assert_eq!` | Prints thousands of bytes on big fixtures | |
| Cell identity + SHA256 hash mismatch | Tiny output; debugging requires verbose re-run | |

---

## Claude's Discretion

- Exact byte sequences for minimal_elf / minimal_png / minimal_zip fixtures.
- `LensConfig::cli_args` return type (`Vec<&'static str>` vs fixed array).
- Hex-window formatting style for failure diagnostics.
- Decode-side BrokenPipe test assertion shape (from current observable contract).
- `common/mod.rs` vs `common.rs` + `common/` dir layout.
- Whether `TimeScale` is re-exported via lib.rs or literal-embedded in tests.
- Invocation flag order in matrix cells.

## Deferred Ideas

- `tempfile` dev-dep → Phase 4 (reader mmap + TUI tests).
- `--purist` coverage in matrix → revisit if TabletLens behaviour changes.
- Color axis expansion (300 → 420 cells) → revisit if lens/color interaction bug surfaces.
- Sub-fixture variants for non-8-aligned sizes → hello_world (14 bytes) covers this already.
- `cargo public-api` diff check for CLI lib surface → optional planner addition.
- Snapshot tests (insta) → permanently out of scope per REQUIREMENTS.
- Proptest / property-based testing → Phase 5 fuzz covers adjacent surface.
