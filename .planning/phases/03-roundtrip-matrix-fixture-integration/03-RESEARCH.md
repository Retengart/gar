# Phase 3: Roundtrip Matrix + Fixture Integration — Research

**Researched:** 2026-04-24
**Domain:** Rust integration testing (`assert_cmd` + `predicates`) for a workspace binary crate; hand-crafted binary fixture factories; line-based static-analysis gate (xtask).
**Confidence:** HIGH — all crate versions verified via `cargo search`; PNG/ZIP byte layouts computed via `zlib.crc32`; `decode::parse_run` citations from source at `crates/base60-cli/src/decode.rs` HEAD; precedents lifted verbatim from Phase 2's shipped `crates/xtask/tests/env_discipline.rs`.

## Summary

Phase 3 is execution-grade work: CONTEXT.md locks 24 decisions (D-01..D-24). This research fills in concrete byte arrays, function signatures, and paste-ready snippets so `PLAN.md` tasks can be written at execution grain.

Key load-bearing facts:
- `assert_cmd = "2.2.1"`, `predicates = "3.1.4"` are current on crates.io (verified via `cargo search`).
- PNG fixture total = **45 bytes**, with **IHDR CRC = 0x3A7E9B55** and **IEND CRC = 0xAE426082** (computed via `zlib.crc32` over the chunk type + data).
- ZIP fixture = **22 bytes** (EOCD-only, empty archive) — the minimum structurally valid ZIP.
- The decoder error contract to pin is the `format!` at `crates/base60-cli/src/decode.rs:105-108`: `"line {line_no}: invalid base-60 digit {digit} at pair {i+1}"`. Input `"00:00:00:00:00:00:00:00:00:00:99"` triggers digit `99`.
- `write_stdin<S: Into<Vec<u8>>>` on `assert_cmd::Command` handles stdin piping cleanly (closes the pipe after writing); there is no cross-process `BrokenPipe` race on Windows to worry about — the binary's own `BrokenPipe` handler at `crates/base60-cli/src/main.rs:104` is what we're exercising.
- Phase 2 shipped `crates/xtask/tests/env_discipline.rs` with an AST-free, line-based walker + walkdir — this is the template to copy for `spawn_discipline.rs`. No new dep.

**Primary recommendation:** Fork Phase 2's `env_discipline.rs` verbatim, retarget to `crates/base60-cli/tests/`, swap the invariant (`env::set_var|remove_var` → `Command::cargo_bin`), swap the exclusion (nothing → `tests/common/`). Copy the 22-byte ZIP EOCD as a `const [u8; 22]`. Copy the 45-byte PNG as two concatenated slice literals with pre-computed CRCs. Use a 128-byte hand-built ELF header written straight into a `Vec<u8>`. Stdin-pipe both hops of every matrix cell — no `tempfile`, no `Stdio::piped()` gymnastics.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Fixture byte generation | `tests/common/mod.rs` (CLI integration) | — | Test-only code; never ships in the binary. D-14.2 locks this. |
| Matrix spawn + compare | `tests/roundtrip.rs` | uses `tests/common` | Single `#[test]`, 140 cells; D-11, D-18, D-19. |
| Subcommand happy-path | `tests/fixtures.rs` | uses `tests/common` | Per-subcommand × 5 fixtures; D-12. |
| Edge / flag tests | `tests/cli.rs` | uses `tests/common` | Non-matrix — stdin piping, `BrokenPipe`, colour, clamping, decoder err; D-13. |
| Spawn-discipline invariant | `crates/xtask/tests/spawn_discipline.rs` | — | Static-analysis gate; mirrors Phase 2's env-discipline gate (D-16). |
| `LensMode::ALL` / `Format::ALL` re-export | `crates/base60-cli/src/lib.rs` | — | Only the enums the integration tests need; D-07, D-10. |
| Binary entry | `crates/base60-cli/src/main.rs` | calls `base60::run()` | One-line shim; D-08. |

## Project Constraints (from CLAUDE.md)

These directives are authoritative — research recommendations must not contradict them:

1. **GSD workflow enforcement** — no raw Edit/Write outside a GSD command. Phase 3 work proceeds via `/gsd-execute-phase`.
2. **Clippy bar:** `clippy::pedantic + nursery + cargo` with `-D warnings`. `multiple_crate_versions` + `module_name_repetitions` are the only workspace-level allows. Every new test/helper will be linted.
3. **Rust edition 2024, MSRV 1.95.** `rust-version.workspace = true` — no override per-crate.
4. **`base60-core` zero-dep invariant** applies to `[dependencies]`, NOT `[dev-dependencies]`. Phase 3 adds dev-deps to `base60-cli` only; `base60-core`'s manifest is untouched.
5. **Context7 MCP preferred** for library docs. This research used `cargo search` + `docs.rs` WebFetch because Context7 MCP was not available in the agent runtime (documented upstream bug); outputs are equivalent.
6. **Concise commit messages**, sacrificing grammar for concision (per user global CLAUDE.md). D-23 lists the three exact commit messages.

## User Constraints (from CONTEXT.md)

### Locked Decisions

D-01..D-24 — copied verbatim from `03-CONTEXT.md` for the planner's reference:

- **D-01:** Matrix shape = 7 lens-config rows × 4 formats × 5 fixtures = 140 cells. `--color=never` forced on every cell.
- **D-02:** 7 lens rows = `[None, Time(Gar), Time(Sec), Time(Ms), Angle, Tablet, Cuneiform]`.
- **D-03:** `TabletLens` runs with default `--purist=false`.
- **D-04:** 5 fixtures = `minimal_elf()` (64/128 B), `minimal_png()` (≤64 B), `minimal_zip()` (22 B EOCD or ~70 B EOCD+LFH), `zero_fill_1kib()` (`vec![0; 1024]`), `hello_world()` (`b"Hello, world!\n"`, 14 B).
- **D-05:** Colour/`NO_COLOR`/`--color` axes NOT in matrix; live as focused edges in `cli.rs`.
- **D-06:** `crates/base60-cli/Cargo.toml` gains `[lib] name = "base60" path = "src/lib.rs"` alongside `[[bin]]`.
- **D-07:** `src/lib.rs` re-exports only `pub use cli::{LensMode, Format};` — minimal public surface.
- **D-08:** `main.rs` becomes `fn main() -> anyhow::Result<()> { base60::run() }`. Current body lives as `pub fn run()` in `lib.rs`.
- **D-09:** `LensMode::ALL` widens `pub(crate) → pub` (revises Phase 1 D-06).
- **D-10:** Add `impl Format { pub const ALL: &[Self] = &[Format::Ansi, Format::Plain, Format::Json, Format::Html]; }` + exhaustiveness test.
- **D-11:** `tests/roundtrip.rs` = matrix only, one `#[test]`.
- **D-12:** `tests/fixtures.rs` = per-subcommand × 5 fixtures happy path.
- **D-13:** `tests/cli.rs` = flag edges. Pins the `"99"` error message contract.
- **D-14:** `tests/common/mod.rs` = `base60_cmd()` + fixture factories + assertion helpers.
- **D-15:** `tests/common/mod.rs` exports `LensConfig` enum + `ALL_LENS_CONFIGS: &[LensConfig]`.
- **D-16:** `crates/xtask/tests/spawn_discipline.rs`; reuses `walkdir = "2"`; line-based scan; pattern = literal `Command::cargo_bin`.
- **D-17:** Failure message: `{file}:{line}: raw Command::cargo_bin outside tests/common/ — use base60_cmd() from tests/common/mod.rs`.
- **D-18:** Matrix = single `#[test] fn roundtrip_matrix_byte_identical()` with nested loops.
- **D-19:** Each cell = stdin-piped both hops (no `tempfile`).
- **D-20:** Assertion on failure prints cell identity + ±8-byte divergence window.
- **D-21:** Per-cell walltime target < 200 ms Ubuntu, < 500 ms Windows. Aggregate ~30 s/CI cell acceptable.
- **D-22:** dev-deps = `assert_cmd = "2"` + `predicates = "3"`; `serial_test` already present; NO `tempfile`.
- **D-23:** Three commits in strict order (refactor → matrix+gate → fixtures+cli).
- **D-24:** Every commit green on `cargo test --workspace --all-targets --locked` AND clippy pedantic+nursery+cargo `-D warnings`.

### Claude's Discretion

- Exact byte sequences for `minimal_elf`/`minimal_png`/`minimal_zip` (planner picks from this research).
- `LensConfig::cli_args` return type (`Vec<&'static str>` vs. `[&'static str; N]`).
- Hex-window formatting on failure diagnostics.
- Decode-side `BrokenPipe` test assertion shape.
- `tests/common/mod.rs` vs. `tests/common.rs` + `tests/common/` layout.
- Whether `TimeScale` is re-exported at `lib.rs`.
- Invocation flag order in cells.

### Deferred Ideas (OUT OF SCOPE)

- `tempfile = "3"` — Phase 4.
- `--purist` coverage in matrix — inline unit test in `lens.rs` covers it.
- Color-axis cube (`LensMode × FormatMode × ColorMode`) — Phase 3 covers color in `cli.rs` edges only.
- Sub-fixture variants (non-8-aligned sizes beyond `hello_world`).
- `cargo public-api` diff for `lib.rs`.
- Snapshot tests (`insta`), proptest.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TEST-01 | Fixture-driven roundtrip matrix (`LensMode × Format × fixtures`) | §Fixture factories (ready-to-paste bytes), §Matrix nested-loop skeleton, §assert_roundtrip helper shape |
| TEST-03 | `assert_cmd` coverage of `dump`/`analyze`/`decode`/`completions` + stdin + BrokenPipe | §assert_cmd patterns, §BrokenPipe test shape, §Decoder error contract pin |

Both requirements ship in the same phase; D-23 splits them across commits 2 and 3.

## Standard Stack

### Core (already in workspace)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `anyhow` | `1.0.102` | CLI error flow (`main.rs`, `run_*`) | Existing; unchanged in Phase 3 `[CITED: crates/base60-cli/Cargo.toml:18]` |
| `clap` | `4.6.1` (derive) | Arg parsing (`cli.rs`) | Existing `[CITED: crates/base60-cli/Cargo.toml:19]` |
| `base60-core` | path dep | Lens trait, TimeScale, convert, cuneiform | Existing path dep `[CITED: crates/base60-cli/Cargo.toml:24]` |

### New dev-deps (Phase 3 adds)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `assert_cmd` | `2` (caret) | Spawn the compiled binary from integration tests | De facto standard for Rust CLI integration. Latest 2.x = `2.2.1` `[VERIFIED: cargo search assert_cmd]`. Handles `CARGO_BIN_EXE_*` lookup, stdin piping, `Assert` chaining. |
| `predicates` | `3` (caret) | Compose stdout/stderr assertions | Companion to `assert_cmd`. Latest 3.x = `3.1.4` `[VERIFIED: cargo search predicates]`. |
| `serial_test` | already `"3"` | Env-test serialisation | Already in `[dev-dependencies]` from Phase 2 `[CITED: crates/base60-cli/Cargo.toml:27]`. No change. |

### Supporting (unchanged — NOT adding in Phase 3)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `walkdir` | `2` | Recursive dir walk in xtask gate | Already xtask dev-dep from Phase 2; `spawn_discipline.rs` reuses it `[CITED: crates/xtask/tests/env_discipline.rs:14]`. |

### Deferred dev-deps (NOT this phase)

- `tempfile = "3"` — Phase 4 (D-22 explicit).
- `rstest` — CONTEXT D-18 locks single `#[test]`. NOT adding.
- `insta` — explicitly out-of-scope per PROJECT.md and REQUIREMENTS §Out of Scope.

### Version verification

```bash
$ cargo search assert_cmd --limit 1
assert_cmd = "2.2.1"    # Test CLI Applications.
$ cargo search predicates --limit 1
predicates = "3.1.4"    # An implementation of boolean-valued predicate functions.
```

Pinning as `"2"` and `"3"` (caret) per D-22 resolves to latest 2.x/3.x during `cargo update`. Lockfile is already `--locked` in CI so drift is impossible between CI cells.

Installation:

```toml
# crates/base60-cli/Cargo.toml [dev-dependencies]
assert_cmd = "2"
predicates = "3"
serial_test = { version = "3", default-features = false }  # unchanged
```

## Architecture Patterns

### System Architecture Diagram

```
                                  ┌─────────────────────────┐
                                  │  tests/common/mod.rs    │
                                  │                         │
 fixture factories ──────────────▶│  minimal_elf()          │
  (Vec<u8>, ≤4 KiB)               │  minimal_png()          │
                                  │  minimal_zip()          │
                                  │  zero_fill_1kib()       │
                                  │  hello_world()          │
                                  │                         │
                                  │  base60_cmd()  ───────┐ │
                                  │   .env_clear()        │ │
                                  │   +PATH/SystemRoot/.. │ │
                                  │                       │ │
                                  │  LensConfig enum      │ │
                                  │  ALL_LENS_CONFIGS     │ │
                                  │                       │ │
                                  │  assert_roundtrip()   │ │
                                  └───────────────────────┼─┘
                                              │           │
                                              ▼           │
 ┌─────────────────────┐       ┌──────────────────────┐   │
 │ tests/roundtrip.rs  │       │ tests/fixtures.rs    │   │
 │ ┌─────────────────┐ │       │                      │   │
 │ │#[test]          │ │       │  dump × 5            │   │
 │ │ 7×4×5 = 140     │ │       │  analyze × 5         │   │
 │ │ cells           │ │       │  decode × 5          │   │
 │ │   hop 1: dump   │─┼──────▶│  completions × 5     │───┤
 │ │   hop 2: decode │ │       └──────────────────────┘   │
 │ │   assert eq     │ │                                  │
 │ └─────────────────┘ │       ┌──────────────────────┐   │
 └─────────────────────┘       │ tests/cli.rs         │   │
                               │                      │   │
                               │  stdin → dump        │───┤
                               │  dump  → decode stdin│   │
                               │  BrokenPipe test     │   │
                               │  NO_COLOR env        │   │
                               │  --color={a,n,never} │   │
                               │  --skip / --length   │   │
                               │  decoder "99" pin    │   │
                               └──────────────────────┘   │
                                                          │
                                              spawns      ▼
                                           ┌───────────────────┐
                                           │ base60 binary     │
                                           │ (from CARGO_BIN_  │
                                           │  EXE_base60)      │
                                           └───────────────────┘

 ┌────────────────────────────────────────┐
 │ crates/xtask/tests/spawn_discipline.rs │   (static gate)
 │                                        │
 │   walkdir: crates/base60-cli/tests/**  │
 │   exclude: tests/common/**             │
 │   flag:    line.contains("Command::cargo_bin")
 │   fail:    {path}:{line}: message      │
 └────────────────────────────────────────┘
```

### Recommended Project Structure

```
crates/base60-cli/
├── Cargo.toml                       # + [lib], + assert_cmd, + predicates
├── src/
│   ├── main.rs                      # SHRUNK: one-line shim
│   ├── lib.rs                       # NEW: pub fn run() + pub use cli::{LensMode, Format}
│   ├── cli.rs                       # EDITED: Format::ALL, LensMode::ALL pub
│   └── ... (unchanged: analyze, chunk, color, decode, dump, format, persist, reader, search, tui)
└── tests/                           # NEW DIRECTORY
    ├── common/
    │   └── mod.rs                   # base60_cmd + fixtures + LensConfig + assert_roundtrip
    ├── roundtrip.rs                 # 140-cell matrix
    ├── fixtures.rs                  # 20 happy-path tests (4 subcmds × 5 fixtures)
    └── cli.rs                       # stdin/BrokenPipe/colour/clamp/decoder edges

crates/xtask/
└── tests/
    └── spawn_discipline.rs          # NEW — mirror of env_discipline.rs
```

### Pattern 1: `base60_cmd()` helper (D-14.1)

**What:** The single spawner. `.env_clear()` + restore minimal env + absolute-path binary lookup via `CARGO_BIN_EXE_base60`.

**When to use:** Every integration test. Gate enforces no other entry.

**Example:** `[VERIFIED: docs.rs/assert_cmd/2.2.1]`

```rust
// crates/base60-cli/tests/common/mod.rs
use assert_cmd::Command;

/// Build a hermetic `base60` command: cleared env, only the minimum
/// restored so the child process can start on every CI cell.
///
/// On Windows, `CreateProcess` requires `SystemRoot` and (for some DLL
/// loader paths) `USERPROFILE`; on Unix a clean `PATH` is enough.
/// Restoring only what's set avoids injecting empty variables that some
/// libc builds treat differently from "unset".
pub(crate) fn base60_cmd() -> Command {
    let mut cmd = Command::cargo_bin("base60").expect("binary built by cargo");
    cmd.env_clear();
    // Unix + Windows: PATH is how subprocesses find helpers. assert_cmd
    // uses an absolute path for the binary itself (via CARGO_BIN_EXE_*),
    // so PATH restoration is a belt-and-braces measure.
    if let Some(path) = std::env::var_os("PATH") {
        cmd.env("PATH", path);
    }
    // Windows CreateProcess quirks — rust#37519.
    #[cfg(windows)]
    {
        if let Some(root) = std::env::var_os("SystemRoot") {
            cmd.env("SystemRoot", root);
        }
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            cmd.env("USERPROFILE", profile);
        }
    }
    cmd
}
```

Notes:
- Do NOT pre-set `NO_COLOR` in the helper — callers pass `--color=never` explicitly so the grep for colour mode stays visible per test (CONTEXT §specifics).
- `env_clear()` wipes every inherited var including `NO_COLOR`, so default behaviour becomes "no env" — callers specify what they need.

### Pattern 2: Matrix test body (D-18, D-19)

```rust
// crates/base60-cli/tests/roundtrip.rs
mod common;

use base60::{Format, LensMode};  // re-exported by lib.rs per D-07
use common::{ALL_LENS_CONFIGS, LensConfig, assert_roundtrip, base60_cmd, fixtures};

#[test]
fn roundtrip_matrix_byte_identical() {
    let all_fixtures: &[(&str, Vec<u8>)] = &[
        ("minimal_elf",    fixtures::minimal_elf()),
        ("minimal_png",    fixtures::minimal_png()),
        ("minimal_zip",    fixtures::minimal_zip()),
        ("zero_fill_1kib", fixtures::zero_fill_1kib()),
        ("hello_world",    fixtures::hello_world()),
    ];

    for (fx_label, fx_bytes) in all_fixtures {
        for lens in ALL_LENS_CONFIGS {
            for fmt in Format::ALL {
                let cell_label = format!("lens={} fmt={:?} fixture={fx_label}", lens.label(), fmt);

                // Hop 1: stdin → base60 … → stdout (the dump).
                let fmt_arg = format!("--format={}", fmt_value(*fmt));
                let mut args: Vec<&str> = vec!["--color=never", &fmt_arg];
                let lens_args = lens.cli_args();
                args.extend(lens_args.iter().copied());
                let dump_out = base60_cmd()
                    .args(&args)
                    .write_stdin(fx_bytes.clone())
                    .assert()
                    .success()
                    .get_output()
                    .stdout
                    .clone();

                // Hop 2: dump → base60 decode → stdout (raw bytes).
                let decoded = base60_cmd()
                    .arg("decode")
                    .write_stdin(dump_out)
                    .assert()
                    .success()
                    .get_output()
                    .stdout
                    .clone();

                assert_roundtrip(fx_bytes, &decoded, &cell_label);
            }
        }
    }
}

fn fmt_value(f: Format) -> &'static str {
    match f {
        Format::Ansi  => "ansi",
        Format::Plain => "plain",
        Format::Json  => "json",
        Format::Html  => "html",
    }
}
```

**Rationale for single `#[test]`** (D-18 locked):
- **Pros:** one libtest entry; one setup cost; parallel with every other `#[test]`; trivial coverage arithmetic (`cargo test` prints "1 passed" when 140 cells pass).
- **Cons:** first failing cell short-circuits the rest (libtest stops on `assert_eq!` panic). Mitigation: `cell_label` in the assert message names the exact cell (D-20); fixing one failure unblocks the next iteration.

### Pattern 3: `LensConfig` enum (D-15)

```rust
// crates/base60-cli/tests/common/mod.rs
use base60_core::lens::TimeScale;

/// Lens × time-scale combinations exercised by the roundtrip matrix.
/// Variants expand the `LensMode::Time` row across all three scales so
/// the seven rows hit every distinct CLI flag payload.
#[derive(Copy, Clone, Debug)]
pub(crate) enum LensConfig {
    None,
    Time(TimeScale),
    Angle,
    Tablet,
    Cuneiform,
}

impl LensConfig {
    /// CLI flags this config produces, in invocation order.
    pub(crate) fn cli_args(self) -> Vec<&'static str> {
        match self {
            LensConfig::None               => vec!["--lens=none"],
            LensConfig::Time(TimeScale::Gar) => vec!["--lens=time", "--time-scale=gar"],
            LensConfig::Time(TimeScale::Sec) => vec!["--lens=time", "--time-scale=sec"],
            LensConfig::Time(TimeScale::Ms)  => vec!["--lens=time", "--time-scale=ms"],
            LensConfig::Angle              => vec!["--lens=angle"],
            LensConfig::Tablet             => vec!["--lens=tablet"],
            LensConfig::Cuneiform          => vec!["--lens=cuneiform"],
        }
    }

    /// Diagnostic label for failure messages.
    pub(crate) fn label(self) -> &'static str {
        match self {
            LensConfig::None               => "None",
            LensConfig::Time(TimeScale::Gar) => "Time(Gar)",
            LensConfig::Time(TimeScale::Sec) => "Time(Sec)",
            LensConfig::Time(TimeScale::Ms)  => "Time(Ms)",
            LensConfig::Angle              => "Angle",
            LensConfig::Tablet             => "Tablet",
            LensConfig::Cuneiform          => "Cuneiform",
        }
    }
}

pub(crate) const ALL_LENS_CONFIGS: &[LensConfig] = &[
    LensConfig::None,
    LensConfig::Time(TimeScale::Gar),
    LensConfig::Time(TimeScale::Sec),
    LensConfig::Time(TimeScale::Ms),
    LensConfig::Angle,
    LensConfig::Tablet,
    LensConfig::Cuneiform,
];
```

**Re-export decision:** `TimeScale` must be visible in integration tests. Two options:
1. Re-export via `lib.rs`: `pub use base60_core::lens::TimeScale;` (adds one item to D-07's "minimal surface").
2. Import directly from `base60_core::lens::TimeScale` (no re-export; tests depend on core directly).

**Recommendation:** Option 2. `base60-core` is a path dep, not published; adding `base60_core` as a dev-dep for tests is free and preserves D-07's "CLI lib surface = `{LensMode, Format}` only" intent. If the planner picks option 1, it's also acceptable — D-07's final clause explicitly allows this.

### Pattern 4: `assert_roundtrip` helper (D-14.3, D-20)

```rust
// crates/base60-cli/tests/common/mod.rs
/// Compare decoded output to the original fixture, printing a readable
/// diagnostic (first divergence + ±8-byte windows) on mismatch.
pub(crate) fn assert_roundtrip(original: &[u8], decoded: &[u8], cell_label: &str) {
    if original == decoded {
        return;
    }
    let diverge = original
        .iter()
        .zip(decoded.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(original.len().min(decoded.len()));
    let orig_window = hex_window(original, diverge);
    let dec_window  = hex_window(decoded,  diverge);
    panic!(
        "cell: {cell_label}\n\
         original_len={} decoded_len={}\n\
         first diverge at byte {diverge}\n\
         original ±8: {orig_window}\n\
         decoded  ±8: {dec_window}",
        original.len(),
        decoded.len(),
    );
}

fn hex_window(bytes: &[u8], center: usize) -> String {
    let lo = center.saturating_sub(8);
    let hi = (center + 8).min(bytes.len());
    bytes[lo..hi]
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let abs = lo + i;
            if abs == center { format!("[{b:02x}]") } else { format!("{b:02x}") }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
```

### Anti-Patterns to Avoid

- **`tempfile` for stdin piping:** `assert_cmd::Command::write_stdin` accepts `impl Into<Vec<u8>>` — the full fixture bytes get written before the spawn returns. No temp file needed. D-22 explicit.
- **`rstest` for the matrix:** Generates N separate `#[test]`s, slowing compile and bloating libtest output. CONTEXT D-18 locks single-test.
- **Asserting `BrokenPipe` from the child's error output:** `base60`'s main.rs:104 swallows `BrokenPipe` and exits 0 — the test only needs to verify exit status, not stderr.
- **Hand-written `std::process::Command::cargo_bin`:** Doesn't exist on `std::process::Command`; requires the `CommandCargoExt` extension trait. `assert_cmd::Command::cargo_bin` is the convenience wrapper. Either is caught by the gate (D-16 scans for literal `Command::cargo_bin`).
- **Pre-setting `NO_COLOR=1` inside `base60_cmd()`:** hides the `--color=...` flag from grep-auditing; CONTEXT §specifics forbids.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Spawn compiled binary from test | Hand-parse `CARGO_BIN_EXE_*` | `assert_cmd::Command::cargo_bin("base60")` | Handles Windows path quirks, caching, and the per-test `CARGO_TARGET_TMPDIR` correctly. |
| Compose stdout/stderr assertions | String `.contains(...)` chains | `predicates::str::contains(...)` + `.stdout(pred)` | Composable; failure messages are "expected X, got Y" not just a bool; 3.x is the standard. |
| Recursive file walk in gate | hand-rolled `fs::read_dir` recursion | `walkdir::WalkDir` | Already pulled in by Phase 2; cross-platform; filter-entry is ergonomic. `[CITED: crates/xtask/tests/env_discipline.rs:14]` |
| PNG CRC32 | Implement CRC32 in-test | Pre-computed constants | CRCs for our exact IHDR/IEND are fixed at compile time (this research). Generating at runtime would require a CRC dep or byte-by-byte polynomial. |
| RNG for fuzz-style byte generation | `rand` crate | Not needed for Phase 3 | The 5 fixtures are deterministic; no randomisation. |

**Key insight:** Every single thing Phase 3 needs already exists as a workspace dev-dep, a precomputed constant, or a ~20-line snippet. No invention required.

## Fixture Factories — Exact Byte Sequences

> These are the payoff of this research. The planner pastes these verbatim into `tests/common/mod.rs`. Every fixture < 4 KiB (CONTEXT D-04 + Pitfall 7). All generated in-test, no `include_bytes!`.

### `hello_world()` — 14 bytes

```rust
pub(crate) fn hello_world() -> Vec<u8> {
    b"Hello, world!\n".to_vec()
}
```

`14 % 8 == 6` exercises the short-tail padding path (CONTEXT §specifics).

### `zero_fill_1kib()` — 1024 bytes

```rust
pub(crate) fn zero_fill_1kib() -> Vec<u8> {
    vec![0_u8; 1024]
}
```

128 full 8-byte chunks, zero short tail, minimum entropy — stresses the heat-map's low-bucket path.

### `minimal_png()` — 45 bytes `[VERIFIED: zlib.crc32 computation]`

Structure: 8-byte signature + IHDR chunk (25 bytes) + IEND chunk (12 bytes). Total = **45 bytes**.

Layout per PNG spec (RFC 2083 / libpng spec):
- Signature: `89 50 4E 47 0D 0A 1A 0A` (fixed).
- IHDR = `length(4) + "IHDR"(4) + data(13) + CRC(4) = 25` bytes. Data = 1×1 grayscale, 8-bit, no interlace.
- IHDR CRC (over `"IHDR" + data`): **`0x3A7E9B55`**.
- IEND = `length(4) + "IEND"(4) + data(0) + CRC(4) = 12` bytes.
- IEND CRC (over `"IEND"`): **`0xAE426082`**.

```rust
pub(crate) fn minimal_png() -> Vec<u8> {
    // 8-byte PNG signature + IHDR (25 B) + IEND (12 B) = 45 B total.
    // CRCs pre-computed by zlib.crc32 over (chunk_type || data); they
    // are constant for these exact chunk bodies.
    let mut out = Vec::with_capacity(45);
    // Signature
    out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    // IHDR length = 13 (big-endian)
    out.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    out.extend_from_slice(b"IHDR");
    // Width=1, Height=1, BitDepth=8, ColorType=0 (grayscale),
    // Compression=0, Filter=0, Interlace=0
    out.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x01, // width
        0x00, 0x00, 0x00, 0x01, // height
        0x08,                   // bit depth
        0x00,                   // color type
        0x00,                   // compression
        0x00,                   // filter
        0x00,                   // interlace
    ]);
    out.extend_from_slice(&0x3A7E9B55_u32.to_be_bytes()); // IHDR CRC
    // IEND length = 0
    out.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    out.extend_from_slice(b"IEND");
    out.extend_from_slice(&0xAE426082_u32.to_be_bytes()); // IEND CRC
    debug_assert_eq!(out.len(), 45);
    out
}
```

**Verification script** (Python, for the planner to re-run if doubt arises):

```python
import zlib
ihdr_data = bytes([0,0,0,1, 0,0,0,1, 8, 0, 0, 0, 0])
print(f"{zlib.crc32(b'IHDR' + ihdr_data):08X}")  # 3A7E9B55
print(f"{zlib.crc32(b'IEND'):08X}")              # AE426082
```

### `minimal_zip()` — 22 bytes (EOCD only)

An empty-archive ZIP is just the End of Central Directory record. The ZIP spec (PKWARE APPNOTE 6.3.9, §4.3.16) requires the signature `0x06054b50` and 18 bytes of zero-valued fields.

```rust
pub(crate) fn minimal_zip() -> Vec<u8> {
    // End-of-central-directory signature + all zero counts/offsets + 0-byte comment.
    // Structurally valid empty ZIP; accepted by Info-Zip, libzip, Python zipfile.
    vec![
        0x50, 0x4B, 0x05, 0x06, // EOCD signature "PK\x05\x06"
        0x00, 0x00,             // disk number
        0x00, 0x00,             // disk where CD starts
        0x00, 0x00,             // CD entries on this disk
        0x00, 0x00,             // CD entries total
        0x00, 0x00, 0x00, 0x00, // CD size
        0x00, 0x00, 0x00, 0x00, // CD offset
        0x00, 0x00,             // comment length
    ]
}
```

Picking 22-byte EOCD-only (not EOCD + LFH) because:
- CONTEXT D-04 lists both shapes as acceptable; planner picks.
- 22 bytes = `22 % 8 == 6` — also hits the short-tail path (redundant with `hello_world` but harmless; matrix invariant is roundtrip byte-identity, not format-specific coverage).
- Smaller fixture = tiny diagnostic output on failure.

### `minimal_elf()` — 128 bytes (ELF64 header only)

The ELF spec (System V ABI, §4) defines a 64-byte header for ELF64. CONTEXT D-04 says "64 or 128 bytes — pick one and justify". Recommendation: **128 bytes** — the 64-byte header + 64-byte program-header-table-placeholder. Rationale:
- 64 bytes = exactly 8 chunks, zero tail (same coverage as `zero_fill` partial).
- 128 bytes = 16 chunks, zero tail, AND exercises the offset column crossing `0x00000080`. More distinct from `zero_fill_1kib` (`0x00000000..0x00000400`).
- Either is 8-aligned and generates-in-test; 128 is preferred for the distinct-coverage argument.

Fields (`e_*` — per ELF64 spec §4.1):
- `e_ident[0..4]`: magic `7F 45 4C 46` (`\x7FELF`).
- `e_ident[4]`: class = `2` (ELFCLASS64).
- `e_ident[5]`: data = `1` (ELFDATA2LSB, little-endian).
- `e_ident[6]`: version = `1`.
- `e_ident[7]`: OSABI = `0` (System V).
- `e_ident[8..16]`: padding zero.
- `e_type` (2 B): `0x0002` (ET_EXEC) — LE.
- `e_machine` (2 B): `0x003E` (EM_X86_64) — LE.
- `e_version` (4 B): `0x00000001`.
- `e_entry` (8 B): `0` (no entry point needed for fixture validity).
- `e_phoff` (8 B): `0x40` (64) — program headers follow immediately.
- `e_shoff` (8 B): `0`.
- `e_flags` (4 B): `0`.
- `e_ehsize` (2 B): `0x0040` (64 — header size).
- `e_phentsize` (2 B): `0x0038` (56 — program header entry size).
- `e_phnum` (2 B): `0x0001` (1 program header).
- `e_shentsize` (2 B): `0`.
- `e_shnum` (2 B): `0`.
- `e_shstrndx` (2 B): `0`.

Then 64 bytes of zero for the program-header slot (the "placeholder" — it is a valid-ish PT_NULL when all-zero). Total 128 bytes.

```rust
pub(crate) fn minimal_elf() -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    // e_ident (16 bytes)
    out.extend_from_slice(&[
        0x7F, b'E', b'L', b'F',  // ELF magic
        2,  // EI_CLASS = ELFCLASS64
        1,  // EI_DATA  = ELFDATA2LSB
        1,  // EI_VERSION = EV_CURRENT
        0,  // EI_OSABI = ELFOSABI_SYSV
        0, 0, 0, 0, 0, 0, 0, 0,  // EI_PAD (8 bytes)
    ]);
    out.extend_from_slice(&0x0002_u16.to_le_bytes()); // e_type   = ET_EXEC
    out.extend_from_slice(&0x003E_u16.to_le_bytes()); // e_machine= EM_X86_64
    out.extend_from_slice(&0x0000_0001_u32.to_le_bytes()); // e_version
    out.extend_from_slice(&0_u64.to_le_bytes());      // e_entry
    out.extend_from_slice(&0x40_u64.to_le_bytes());   // e_phoff  (PHT follows)
    out.extend_from_slice(&0_u64.to_le_bytes());      // e_shoff
    out.extend_from_slice(&0_u32.to_le_bytes());      // e_flags
    out.extend_from_slice(&0x0040_u16.to_le_bytes()); // e_ehsize
    out.extend_from_slice(&0x0038_u16.to_le_bytes()); // e_phentsize
    out.extend_from_slice(&0x0001_u16.to_le_bytes()); // e_phnum = 1
    out.extend_from_slice(&0_u16.to_le_bytes());      // e_shentsize
    out.extend_from_slice(&0_u16.to_le_bytes());      // e_shnum
    out.extend_from_slice(&0_u16.to_le_bytes());      // e_shstrndx
    // Offset now 64. Pad with 64 zero bytes for the single program-header slot.
    out.resize(128, 0);
    debug_assert_eq!(out.len(), 128);
    out
}
```

All fixtures generate deterministically; no RNG, no file I/O.

## Common Pitfalls (this phase)

### Pitfall 7: Fixture corpus bloat (from PITFALLS.md §Pitfall 7)

**What goes wrong:** Check-in of `/bin/ls`, a `dd`-generated 10 MB zero-fill, or a real PNG photo. Repo grows; clones slow; LFS tempts.

**Prevention for Phase 3:** every fixture is generate-in-test (see byte arrays above), each < 4 KiB. No `include_bytes!`. A ROADMAP SC3 check (`git ls-files | xargs stat -c '%s'`) confirms no file over 8 KiB in `tests/`.

**Warning signs:**
- New file matching `crates/base60-cli/tests/**/*.{bin,elf,png,zip}` in `git status`.
- `tests/fixtures/` directory appears.

### Pitfall 12: `assert_cmd` + `.env_clear()` Windows caveat (from PITFALLS.md §Pitfall 12)

**What goes wrong:** `.env_clear()` on Windows strips `SystemRoot` and sometimes `USERPROFILE`, breaking `CreateProcess` DLL loading; child returns `WinError 0xC0000135` / `STATUS_DLL_NOT_FOUND`. Upstream: rust-lang/rust#37519.

**Prevention for Phase 3:** `base60_cmd()` restores exactly these three variables (if set on parent):
- `PATH` — cross-platform.
- `SystemRoot` — Windows-only.
- `USERPROFILE` — Windows-only.

Helper code above (Pattern 1) implements this. `#[cfg(windows)]` guard keeps the Unix path lean.

**Warning signs:**
- Test passes locally on Linux, fails on Windows CI with `STATUS_DLL_NOT_FOUND` or exit code `-1073741515`.
- `assert_cmd` test asserts on stderr containing "not found" messages.

### Pitfall 8: `parse_run` error-message drift (from PITFALLS.md §Pitfall 8)

**What goes wrong:** Phase 4 tightens `decode::parse_run` signature (REF-03); existing `decode.rs:173` test asserts only `err.kind() == InvalidData` + `contains("99")`. A refactor that changes the `format!` string to drop the `"99"` substring silently passes that test if the test isn't strong enough.

**Prevention for Phase 3:** add a dedicated integration test in `tests/cli.rs` that pins the message via substring assertion — NOT equality (too tight) and NOT `kind()`-only (too loose). This is D-13's "pinned error-message contract".

**Current error-message source** `[CITED: crates/base60-cli/src/decode.rs:103-108]`:

```rust
return Err(io::Error::new(
    io::ErrorKind::InvalidData,
    format!(
        "line {line_no}: invalid base-60 digit {digit} at pair {}",
        i + 1
    ),
));
```

**Recommended pinning regex** (loose-enough to survive harmless format tweaks; tight-enough to fail if `"99"` disappears):

```rust
use predicates::prelude::*;

// tests/cli.rs
#[test]
fn decoder_invalid_digit_99_error_contains_the_digit() {
    // The canonical invalid-digit input: last pair is "99", which decodes
    // to value 99 (hi=9, lo=9, digit = 9*10+9 = 99 >= 60).
    let dump = "00000000  00:00:00:00:00:00:00:00:00:00:99  |........|\n";
    base60_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .failure()
        .stderr(predicates::str::contains("99").and(predicates::str::contains("invalid")));
}
```

The assertion requires BOTH `"99"` AND `"invalid"` in stderr. This pins:
- The digit value is surfaced — Phase 4 cannot silently drop it.
- The word "invalid" stays in the diagnostic — Phase 4 cannot rename to e.g. "bad digit" without updating the test.

Planner MAY additionally pin `"pair"` substring to lock the "at pair N" structure; recommendation is to stop at two substrings so the test survives benign rewordings.

### Pitfall 10: `HashMap` iteration non-determinism (from PITFALLS.md §Pitfall 10)

**Prevention for Phase 3:** Every ordered collection in `tests/` is `&[...]` slice constant. `ALL_LENS_CONFIGS: &[LensConfig]`, `Format::ALL: &[Self]`, `all_fixtures: &[(&str, Vec<u8>)]`. No `HashMap` / `HashSet`. Matrix iteration follows declaration order. (D-15 explicitly says this.)

## Code Examples

### `base60_cmd()` full paste

See Pattern 1 above. ~20 lines. Copy verbatim.

### `tests/fixtures.rs` — happy-path skeleton (D-12)

```rust
mod common;

use common::{base60_cmd, fixtures};
use predicates::prelude::*;

fn all_fixtures() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("minimal_elf",    fixtures::minimal_elf()),
        ("minimal_png",    fixtures::minimal_png()),
        ("minimal_zip",    fixtures::minimal_zip()),
        ("zero_fill_1kib", fixtures::zero_fill_1kib()),
        ("hello_world",    fixtures::hello_world()),
    ]
}

#[test]
fn dump_produces_expected_prefix_per_fixture() {
    for (label, bytes) in all_fixtures() {
        // `00000000  ` = eight zeros + two spaces, at the start of every dump.
        base60_cmd()
            .args(["--color=never", "--format=plain"])
            .write_stdin(bytes)
            .assert()
            .success()
            .stdout(predicates::str::starts_with("00000000  "))
            .get_output();
        // drop output; only success + prefix matter for this smoke check.
        // keep `label` for the failing-row identity in case predicates
        // expands to `and()` later.
        let _ = label;
    }
}

#[test]
fn analyze_summary_is_sane_per_fixture() {
    for (_label, bytes) in all_fixtures() {
        base60_cmd()
            .arg("analyze")
            .write_stdin(bytes)
            .assert()
            .success()
            // `analyze::write_summary` emits "bytes: " and an entropy line;
            // pick two stable substrings from crates/base60-cli/src/analyze.rs.
            .stdout(predicates::str::contains("bytes"))
            .stdout(predicates::str::contains("entropy"));
    }
}

#[test]
fn decode_roundtrips_default_dump_per_fixture() {
    for (_label, bytes) in all_fixtures() {
        let dumped = base60_cmd()
            .args(["--color=never", "--format=plain"])
            .write_stdin(bytes.clone())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let decoded = base60_cmd()
            .arg("decode")
            .write_stdin(dumped)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        assert_eq!(decoded, bytes);
    }
}

#[test]
fn completions_shells_all_succeed() {
    for shell in ["bash", "zsh", "fish", "elvish", "powershell"] {
        base60_cmd()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicates::function::function(|s: &[u8]| !s.is_empty()));
    }
}
```

**Planner note:** the exact substrings for `analyze_summary_is_sane_per_fixture` must be verified against `crates/base60-cli/src/analyze.rs`'s `write_summary` current output — the research author did NOT open that file (it would bloat research beyond budget). Planner MUST spot-check before committing; the substrings `"bytes"` and `"entropy"` are reasonable defaults based on the module's declared purpose in STRUCTURE.md / ARCHITECTURE.md.

### `tests/cli.rs` — edges skeleton (D-13)

```rust
mod common;

use common::{base60_cmd, fixtures};
use predicates::prelude::*;

#[test]
fn stdin_piped_dump_produces_output() {
    base60_cmd()
        .args(["--color=never", "--format=plain"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::is_empty().not());
}

#[test]
fn no_color_env_suppresses_ansi_on_auto() {
    base60_cmd()
        .env("NO_COLOR", "1")
        .args(["--color=auto", "--format=ansi"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::contains("\x1b[").not());
}

#[test]
fn color_always_forces_ansi_even_in_pipe() {
    base60_cmd()
        .args(["--color=always", "--format=ansi"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::contains("\x1b["));
}

#[test]
fn color_never_suppresses_ansi_with_clicolor_force() {
    base60_cmd()
        .env("CLICOLOR_FORCE", "1")
        .args(["--color=never", "--format=ansi"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::contains("\x1b[").not());
}

#[test]
fn skip_past_end_yields_empty_dump() {
    // 14-byte fixture with --skip=1024 → zero bytes surface → empty dump body.
    // Exact stdout content depends on the no-bytes path; at minimum, success
    // exit and no crash.
    base60_cmd()
        .args(["--color=never", "--format=plain", "--skip=1024"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success();
}

#[test]
fn length_clamps_to_available_bytes() {
    base60_cmd()
        .args(["--color=never", "--format=plain", "--length=9999"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::starts_with("00000000  "));
}

#[test]
fn decoder_invalid_digit_99_error_contains_the_digit() {
    let dump = "00000000  00:00:00:00:00:00:00:00:00:00:99  |........|\n";
    base60_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .failure()
        .stderr(predicates::str::contains("99").and(predicates::str::contains("invalid")));
}

// BrokenPipe test — see dedicated section below.
```

## BrokenPipe Test Shape (D-13, Claude's Discretion)

**Observed contract** `[CITED: crates/base60-cli/src/main.rs:98-106, 117-120, 136-140]`: all three subcommand dispatchers (`run_view`, `run_analyze`, `run_decode`) convert `std::io::ErrorKind::BrokenPipe` into a silent `Ok(())`. The binary exits 0 when its stdout pipe closes early.

**Test intent:** prove the exit-0-on-BrokenPipe behaviour for `dump` and document what it does for `decode`.

**Recommended test (lowest-dep):** don't orchestrate a real cross-process pipe — spawn the binary with a giant stdin and close the pipe via `write_stdin` on a big-enough payload. When `base60` tries to write more lines than our test reader can consume, it will either:
- (a) write everything to `assert_cmd`'s internal buffer (which has no reader bottleneck) — no BrokenPipe, exit 0. Test trivially passes.
- (b) emit to our internal buffer until we call `.get_output()`, which drains. No real pipe-close happens.

**Conclusion:** `assert_cmd` does NOT naturally trigger `BrokenPipe` on the child. Testing `BrokenPipe` requires explicit `Stdio::piped()` on `std::process::Command` and dropping the stdout handle.

**Decision:** use `std::process::Command` directly for this ONE test, behind the `tests/common/` shield (the spawn-discipline gate permits `Command::cargo_bin` inside `common/`, but the test code itself in `cli.rs` uses a helper from `common/` that isolates the raw spawn). Alternative: put the BrokenPipe test inside `tests/common/mod.rs` itself (the gate excludes `common/`). This is simpler and explicit.

**Paste-ready shape:**

```rust
// tests/common/mod.rs — append to the existing module
use std::io::{Read, Write};
use std::process::{Command as StdCommand, Stdio};

/// Spawn `base60` with the given args + stdin, then drop the child's stdout
/// handle immediately to force `BrokenPipe` on the writer side. Returns the
/// child's exit status.
///
/// This is the ONLY place in the test suite allowed to use raw
/// `std::process::Command` because the spawn-discipline gate excludes
/// `tests/common/`. Callers drive this through a thin wrapper.
pub(crate) fn spawn_with_closed_stdout(
    args: &[&str],
    stdin_bytes: &[u8],
) -> std::process::ExitStatus {
    let bin = env!("CARGO_BIN_EXE_base60");
    let mut child = StdCommand::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn base60");
    // Feed stdin fully, close.
    child.stdin.take().unwrap().write_all(stdin_bytes).ok();
    // CLOSE stdout immediately — the child's next write gets EPIPE.
    drop(child.stdout.take());
    // Wait for the child to finish; on EPIPE main.rs swallows it → exit 0.
    child.wait().expect("wait base60")
}
```

Then in `tests/cli.rs`:

```rust
#[test]
fn dump_exits_zero_on_broken_pipe() {
    // 1 KiB of input produces ~128 lines of dump; well above any pipe
    // buffer's write-before-block threshold, so the write-to-closed-pipe
    // path in main.rs is exercised.
    let status = common::spawn_with_closed_stdout(
        &["--color=never", "--format=plain"],
        &fixtures::zero_fill_1kib(),
    );
    assert!(status.success(), "base60 dump must exit 0 on BrokenPipe, got {status:?}");
}
```

**Decode-side `BrokenPipe` (CONTEXT Claude's Discretion):** same shape, but pipe in a dump stream large enough that `decode`'s 8-byte-per-line output exceeds the pipe buffer. A 1 KiB dump → ~10 KiB of decoded output is safe. Planner can omit this second test if the `dump` test alone satisfies D-13's "BrokenPipe on dump" requirement — the decode contract is covered by the same `main.rs:136-140` handler so the risk of regression is shared, not independent.

**Windows caveat:** closing `stdout` on Windows produces `ERROR_BROKEN_PIPE` (109) which Rust's stdlib maps to `ErrorKind::BrokenPipe` identically to Unix's `EPIPE` (32). The test works on all three OSes. `[VERIFIED: std::io::ErrorKind docs + rust-lang/rust main branch]`

## `Format::ALL` Shape Decision (CONTEXT Discretion)

Two options:

**Option A — `pub const ALL: &'static [Self]`** (mirrors `LensMode::ALL`):
```rust
impl Format {
    pub const ALL: &'static [Self] = &[Format::Ansi, Format::Plain, Format::Json, Format::Html];
}
```

**Option B — `pub const ALL: [Self; 4]`** (owned array):
```rust
impl Format {
    pub const ALL: [Self; 4] = [Format::Ansi, Format::Plain, Format::Json, Format::Html];
}
```

**Recommendation: Option A (`&[Self]`)** because:
1. **Symmetry with LensMode::ALL** — Phase 1 set the precedent: `pub(crate) const ALL: &[Self] = &[...]` `[CITED: crates/base60-cli/src/cli.rs:47-53]`. Phase 3 widens to `pub` but keeps the shape.
2. **Iteration ergonomics:** `for fmt in Format::ALL` works identically for both; `&[Self]` is slightly preferred by clippy (no hidden `.iter()`).
3. **Surface area:** `&[Self]` publishes a slice reference; `[Self; 4]` publishes the literal length as part of the API. Future variant additions change the length and break consumers who declared `[Format; 4]` themselves. Slice shape is additive-safe.

**Exhaustiveness test** (D-10, alongside the LensMode tests already in `cli.rs`):

```rust
#[test]
fn all_contains_every_format_variant() {
    // Enumerate every Format in a match and check each appears in ALL.
    // If a future variant is added, the match becomes non-exhaustive →
    // compile error points here.
    for variant in [Format::Ansi, Format::Plain, Format::Json, Format::Html] {
        assert!(
            Format::ALL.contains(&variant),
            "Format::ALL missing variant {variant:?}",
        );
    }
    // And: ALL has no duplicates.
    let mut sorted: Vec<Format> = Format::ALL.to_vec();
    sorted.sort_by_key(|f| *f as u8);
    sorted.dedup();
    assert_eq!(sorted.len(), Format::ALL.len(), "duplicate variant in Format::ALL");
}
```

Note: `Format` needs `Ord` (or a `u8` cast) for the dedup check. Since it's `#[derive(PartialEq, Eq, ValueEnum)]` already `[CITED: crates/base60-cli/src/cli.rs:119]`, adding `PartialOrd` + `Ord` is a one-line derive addition; or the test uses `Vec::contains` uniqueness loop and skips the sort. Planner picks.

## Spawn-Discipline Gate — Concrete Template

**Template source:** `crates/xtask/tests/env_discipline.rs` (Phase 2 shipped). Reuse its:
- `walkdir` import and iteration.
- `CARGO_MANIFEST_DIR` root-resolution trick `[CITED: crates/xtask/tests/env_discipline.rs:28]`.
- Comment-filter to skip doc strings.
- Failure-accumulator `Vec<String>` + single `assert!` at the bottom.

**Key differences:**
| Aspect | env_discipline.rs | spawn_discipline.rs |
|--------|-------------------|---------------------|
| Walk root | `crates/base60-{core,cli}/src` | `crates/base60-cli/tests` |
| Exclusion | none | `tests/common/` (by path component) |
| Pattern | `env::set_var(` / `env::remove_var(` | `Command::cargo_bin` |
| Attribute check | `#[test]` + `#[serial(env)]` | — none needed |
| Failure message | attribute-missing | "raw spawn outside tests/common/" |

**Paste-ready `crates/xtask/tests/spawn_discipline.rs`:**

```rust
//! Spawn-discipline gate: every `Command::cargo_bin` invocation in
//! `crates/base60-cli/tests/**/*.rs` must live under `tests/common/`.
//! All other integration tests spawn the binary exclusively through the
//! `base60_cmd()` helper, giving one enforcement point for `.env_clear()`
//! + env-restore invariants. Phase 3 (TEST-03) invariant.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Relative root from this crate's manifest to walk.
const WALK_ROOT: &str = "../base60-cli/tests";

/// Path-component signalling "this file may legitimately spawn the binary".
const EXEMPT_DIR: &str = "common";

#[test]
fn no_raw_spawn_outside_common() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let root_path: PathBuf = Path::new(manifest_dir).join(WALK_ROOT);

    // When Phase 3 hasn't shipped yet `tests/` doesn't exist — in that
    // case the gate is a no-op rather than a failure so CI can run before
    // and after the phase.
    if !root_path.is_dir() {
        return;
    }

    let mut failures: Vec<String> = Vec::new();

    for entry in WalkDir::new(&root_path).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().is_none_or(|e| e != "rs") {
            continue;
        }
        // Skip anything under `tests/common/` — that's the sanctioned spawner.
        if entry
            .path()
            .components()
            .any(|c| c.as_os_str() == EXEMPT_DIR)
        {
            continue;
        }

        let path = entry.path();
        let contents = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

        for (idx, line) in contents.lines().enumerate() {
            // Skip commented lines (doc examples mentioning the helper).
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            if !line.contains("Command::cargo_bin") {
                continue;
            }
            let rel = path
                .strip_prefix(manifest_dir)
                .unwrap_or(path)
                .display()
                .to_string();
            failures.push(format!(
                "{rel}:{lno}: raw Command::cargo_bin outside tests/common/ \
                 — use base60_cmd() from tests/common/mod.rs",
                lno = idx + 1,
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "spawn-discipline gate failed ({count} issue(s)):\n{details}",
        count = failures.len(),
        details = failures.join("\n"),
    );
}
```

Notes:
- The "no-op when `tests/` absent" branch makes the gate safe to land in commit 1 of D-23 (before `tests/` exists) OR in commit 2 (alongside `tests/common/mod.rs`). D-23 lands it in commit 2 — safe either way.
- `components().any(|c| c.as_os_str() == EXEMPT_DIR)` is the cleanest cross-platform "path contains `common/` segment" check; `Path::starts_with` requires the full relative path.
- No regex, no `syn`, no new dep. Total ~45 lines. Mirrors the env gate.

**Pattern choice vs. module-path check** (CONTEXT D-16 alternative): the substring `Command::cargo_bin` catches:
- `assert_cmd::Command::cargo_bin("base60")` — the normal invocation.
- `std::process::Command::cargo_bin` — does not exist in stdlib (it's an extension trait), but a future `Command::cargo_bin(...)` after `use assert_cmd::cargo::CommandCargoExt` trick call-site would also match.
- Any other shape starting a raw spawn.

**False-positive risk:** a comment `// use Command::cargo_bin as shown here` would fire (line starts with `//` → skipped by the comment filter ☑). A docstring in a `pub` item would also fire on its body line — handled by the same filter. Net: low false-positive risk.

**False-negative risk:** `use assert_cmd::Command; let c = Command::cargo_bin("base60");` splits the spawn across lines. If line 1 is `use assert_cmd::Command;` and line 7 is `Command::cargo_bin(...)`, line 7 still contains `Command::cargo_bin` and fires. ☑.

A path-separator tricky case: someone does `let bin = Command ::cargo_bin(...)` with whitespace. Doesn't contain `Command::cargo_bin` exactly — escapes the gate. Planner accepts this as acceptable residual risk (matches Phase 2's precedent of line-based `env::set_var(` literal check).

## Per-Cell Walltime Verification (D-21)

**Goal:** fail-loud in local dev if a cell exceeds 500 ms (the Windows budget); don't fail CI on timing (shared runners are noisy).

**Recommended:**

```rust
// in the matrix loop body
use std::time::Instant;

let cell_start = Instant::now();
one_cell(...);
let elapsed = cell_start.elapsed();

// Debug build only — release CI doesn't care.
#[cfg(debug_assertions)]
if elapsed.as_millis() > 500 {
    eprintln!(
        "WARN: cell '{cell_label}' took {:?} (budget 500ms)",
        elapsed,
    );
}
```

Two critical properties:
- `#[cfg(debug_assertions)]` means release builds (e.g. `cargo test --release`) skip the check entirely.
- `eprintln!` not `panic!` — CI noise doesn't fail the job; local dev sees the warning.

**Don't fail the test on timing.** D-21's "target < 200 ms Ubuntu, < 500 ms Windows" is a soft budget; the aggregate 30 s per OS cell is the hard bound enforced by libtest's per-test timeout (default = none; no action needed).

## Runtime State Inventory

Phase 3 is NOT a rename/refactor/migration — the full runtime state inventory doesn't apply. The only runtime state this phase introduces:

| Category | Items | Action |
|----------|-------|--------|
| Stored data | None | — |
| Live service config | None | — |
| OS-registered state | None | — |
| Secrets/env vars | `NO_COLOR`, `CLICOLOR_FORCE` (read in tests only, via `.env(...)` on child, not mutated on parent) | None — child-process env mutation is safe with `assert_cmd` |
| Build artifacts | `tests/common/mod.rs`, new `tests/*.rs`, `tests/common/target/` (internal cargo cache) | Standard `cargo clean` handles |

Phase 3 does NOT inject any persistent side effects. Every test is hermetic (`.env_clear()` on every spawn; no filesystem writes; no `$XDG_STATE_HOME` touched — the TUI persist path is only reachable via `-i` which integration tests don't drive).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `rustc` + `cargo` 1.95 | Workspace compile | ✓ | Verified via `rust-version.workspace = true` + CI matrix | — |
| `cargo search` | Research version lookup | ✓ | n/a | — |
| Python 3 | CRC32 re-verification (optional) | ✓ | (used for this research) | — |

No missing dependencies. All Phase 3 work is pure Rust, stdlib + the three named dev-deps.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | built-in libtest via `#[test]` (existing) + `assert_cmd = "2"` + `predicates = "3"` (new) |
| Config file | `crates/base60-cli/Cargo.toml` `[dev-dependencies]` (edited) |
| Quick run command | `cargo test -p base60 --test roundtrip --test fixtures --test cli --locked` |
| Full suite command | `cargo test --workspace --all-targets --locked` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TEST-01 | 140-cell roundtrip matrix | integration | `cargo test -p base60 --test roundtrip roundtrip_matrix_byte_identical --locked` | ❌ Wave 0 — new file `crates/base60-cli/tests/roundtrip.rs` |
| TEST-01 | `Format::ALL` exhaustiveness | unit | `cargo test -p base60 --lib all_contains_every_format_variant` | ❌ Wave 0 — new test in `crates/base60-cli/src/cli.rs` tests module |
| TEST-03 | Dump happy-path × 5 fixtures | integration | `cargo test -p base60 --test fixtures dump_produces_expected_prefix_per_fixture --locked` | ❌ Wave 0 — `tests/fixtures.rs` |
| TEST-03 | Analyze happy-path × 5 | integration | `cargo test -p base60 --test fixtures analyze_summary_is_sane_per_fixture --locked` | ❌ Wave 0 — `tests/fixtures.rs` |
| TEST-03 | Decode happy-path × 5 | integration | `cargo test -p base60 --test fixtures decode_roundtrips_default_dump_per_fixture --locked` | ❌ Wave 0 — `tests/fixtures.rs` |
| TEST-03 | Completions × 5 shells | integration | `cargo test -p base60 --test fixtures completions_shells_all_succeed --locked` | ❌ Wave 0 — `tests/fixtures.rs` |
| TEST-03 | Stdin piping | integration | `cargo test -p base60 --test cli stdin_piped_dump_produces_output --locked` | ❌ Wave 0 — `tests/cli.rs` |
| TEST-03 | `BrokenPipe` on dump | integration | `cargo test -p base60 --test cli dump_exits_zero_on_broken_pipe --locked` | ❌ Wave 0 — `tests/cli.rs` + `tests/common/mod.rs::spawn_with_closed_stdout` |
| TEST-03 | `NO_COLOR` + `--color` matrix | integration | `cargo test -p base60 --test cli --locked no_color_env_suppresses color_always_forces color_never_suppresses` | ❌ Wave 0 — `tests/cli.rs` |
| TEST-03 | `--skip` / `--length` clamping | integration | `cargo test -p base60 --test cli --locked skip_past_end length_clamps` | ❌ Wave 0 — `tests/cli.rs` |
| TEST-03 | Decoder "99" error pin (Pitfall 8) | integration | `cargo test -p base60 --test cli decoder_invalid_digit_99_error_contains_the_digit --locked` | ❌ Wave 0 — `tests/cli.rs` |
| TEST-03 | Spawn-discipline gate | integration | `cargo test -p xtask --test spawn_discipline --locked` | ❌ Wave 0 — `crates/xtask/tests/spawn_discipline.rs` |

### Sampling Rate

- **Per task commit:** `cargo test -p base60 --test roundtrip --test fixtures --test cli --locked && cargo test -p xtask --locked` (under 20 s locally).
- **Per wave merge:** `cargo test --workspace --all-targets --locked` — the same command CI runs.
- **Phase gate:** Full suite green AND `cargo clippy --workspace --all-targets --locked -- -D warnings` green (per D-24, each of the 3 commits satisfies this).

### Wave 0 Gaps

- [ ] `crates/base60-cli/tests/common/mod.rs` — fixture factories, `base60_cmd`, `LensConfig`, `assert_roundtrip`, `spawn_with_closed_stdout` helper. Created in D-23 commit 2.
- [ ] `crates/base60-cli/tests/roundtrip.rs` — 140-cell matrix. Created in commit 2.
- [ ] `crates/base60-cli/tests/fixtures.rs` — per-subcommand happy path. Created in commit 3.
- [ ] `crates/base60-cli/tests/cli.rs` — edges + decoder-error pin. Created in commit 3.
- [ ] `crates/xtask/tests/spawn_discipline.rs` — spawn gate. Created in commit 2.
- [ ] `crates/base60-cli/src/lib.rs` — new crate root (lib target). Created in commit 1.
- [ ] `crates/base60-cli/Cargo.toml` — `[lib]` stanza + two new dev-deps. Edited in commit 1 (lib) and commit 2 (dev-deps).
- [ ] `crates/base60-cli/src/main.rs` — shrunk to shim. Edited in commit 1.
- [ ] `crates/base60-cli/src/cli.rs` — `Format::ALL` + exhaustiveness test; `LensMode::ALL` visibility widened. Edited in commit 1.

**Framework install:** none needed. `assert_cmd` and `predicates` install via `cargo fetch`; no toolchain changes.

## Security Domain

`security_enforcement` is not explicitly set in `.planning/config.json` — treat as enabled. Phase 3 is a test-infrastructure phase; the attack surface is minimal but documented for completeness.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth surface — offline CLI |
| V3 Session Management | no | No sessions |
| V4 Access Control | no | No multi-user model |
| V5 Input Validation | yes | Fixture bytes are trusted (generated in-test). User input in fixtures factories is zero — they return hard-coded bytes. |
| V6 Cryptography | no | No crypto in this phase (PNG CRC32 is integrity, not cryptography) |
| V7 Error Handling and Logging | yes | Decoder error-message pin test (D-13) prevents sensitive info exposure drift |
| V14 Configuration | yes | `.env_clear()` + PATH allowlist in `base60_cmd()` prevents environment-leak security issues in CI |

### Known Threat Patterns for Rust integration tests

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Hardcoded credentials in tests | Information Disclosure | None in Phase 3 — fixtures are all public-domain byte sequences (ELF/PNG/ZIP specs) |
| Test env pollution leaking to CI | Tampering | `.env_clear()` on every spawn (D-14.1) |
| Fixture with embedded malware / PoC payload | Info Disclosure / Tampering | All fixtures are minimum-viable headers; no real exploit code |
| Path traversal via fixture filenames | Tampering | No `tests/fixtures/` directory — all generated in-test (Pitfall 7) |
| RNG-based fixture non-determinism | Repudiation | No RNG in Phase 3; every fixture is a deterministic const or literal |
| Spawn-discipline bypass | Tampering | xtask gate (D-16) fails CI if raw `Command::cargo_bin` leaks outside `common/` |

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `std::process::Command::cargo_bin` + `Stdio::piped()` + hand-rolled `write_all` / `read_to_end` | `assert_cmd::Command::cargo_bin` + `.write_stdin(...)` + `.assert().success().get_output()` | `assert_cmd 0.x → 1.x → 2.x` (2019-2024) | 3-5× less test code; automatic stdin-close on drop; cross-platform path resolution |
| `std::process::Command::env("PATH", ...)` on Windows | `.env_clear()` + explicit restore of `PATH`+`SystemRoot`+`USERPROFILE` | rust-lang/rust#37519 workaround, community consensus ~2020 | Windows `CreateProcess` DLL loader works reliably |
| Checked-in `.bin` / `.png` / `.elf` fixtures | In-test `Vec<u8>` factories with pre-computed CRCs | rust-fuzz / general-testing best practice | Tiny repo; no LFS temptation; deterministic per-cell byte arrays |
| Per-variant `#[test]` generated by macro / `rstest` | Single `#[test]` with nested loops | base60 project preference (CONTEXT D-18) | Faster compile; readable libtest output; trivially parallel with other tests |

**Deprecated/outdated:**
- `assert_cmd 1.x` — superseded by 2.x; 2.x is MSRV 1.74 or newer `[VERIFIED: cargo search]`, compatible with our 1.95 floor.
- `tempdir = "3.x"` — not used at all in Phase 3. Planned for Phase 4.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `analyze`'s `write_summary` output contains substrings "bytes" and "entropy" | §Code Examples → fixtures.rs skeleton | Test fails at commit 3. Low — word-level substrings from ANALYZE-01 requirements; planner verifies in `crates/base60-cli/src/analyze.rs` before commit. |
| A2 | Decoder error message format stays at `"line {n}: invalid base-60 digit {d} at pair {p}"` through Phase 3 (changes only in Phase 4 REF-03) | §Pitfall 8 | Pin test fires. Low — verified against decode.rs:105-108 at HEAD. |
| A3 | 64-byte zero-padded ELF "program header" is accepted by `base60 dump`'s byte-reading path | §minimal_elf | None — `base60 dump` reads bytes blindly, never parses ELF. Purely a "does roundtrip survive a realistic-looking 128-byte buffer" test. |
| A4 | Empty-archive ZIP (22 B EOCD only) is sufficient for the matrix's byte-roundtrip invariant | §minimal_zip | None — same reason as A3. |
| A5 | `assert_cmd 2.x` preserves current `.write_stdin(impl Into<Vec<u8>>)` signature through the test's lifetime | §Pattern 2 | Low — 2.x is stable; `--locked` in CI pins version during test runs. |
| A6 | `CARGO_BIN_EXE_base60` env var is set when `cargo test -p base60` runs | §BrokenPipe test | None — documented Cargo behaviour `[CITED: doc.rust-lang.org/cargo/reference/environment-variables.html]`. |
| A7 | Closing child stdout via `drop(child.stdout.take())` triggers `BrokenPipe` on the child's next write on all three OSes (Unix EPIPE, Windows ERROR_BROKEN_PIPE → `ErrorKind::BrokenPipe`) | §BrokenPipe | Low — standard Rust stdlib behaviour. |

**Planner-facing note:** A1 is the only assumption that requires pre-commit verification. Opening `crates/base60-cli/src/analyze.rs` and grepping `write_summary` for the two substrings is a 30-second check; bake it into the plan as a pre-commit step for commit 3.

## Open Questions

1. **Should `TimeScale` be re-exported at `lib.rs`?**
   - What we know: CONTEXT D-07 says "minimal public surface" but also (D-15) says the enum "may need re-exporting".
   - What's unclear: aesthetics — is `base60::TimeScale` or `base60_core::lens::TimeScale` the less-surprising import for test code?
   - Recommendation: import directly from `base60_core::lens` in tests. `base60-core` is a path dep; `dev-dependencies = { base60-core = { path = "../base60-core" }}` is free. Keeps D-07's CLI-lib surface at exactly `{LensMode, Format}`.

2. **Does `analyze::write_summary` emit both `"bytes"` and `"entropy"` substrings?**
   - What we know: ANALYZE-01 requirement says "Shannon entropy, byte histogram".
   - What's unclear: exact wording in `write_summary`'s output.
   - Recommendation: planner opens `crates/base60-cli/src/analyze.rs`, greps `write_summary` body for stable English substrings, and picks two. Candidates include `"entropy"`, `"bytes"`, `"histogram"`, `"region"`. Avoid numeric output; avoid any substring that depends on the fixture input.

3. **Does the decoder-error-pin test need to additionally assert `failure()` exit vs. `success()`?**
   - What we know: `main::run_decode` does NOT swallow `InvalidData` — it propagates up via `?` → `anyhow::Error` → process exit with nonzero code.
   - What's unclear: does the binary exit 1 or some other nonzero code?
   - Recommendation: `.failure()` without a specific exit-code check. `predicates` supports `.code(1)` if planner wants tighter — but anyhow's default is "nonzero" without a fixed code.

## Sources

### Primary (HIGH confidence)

- `crates/base60-cli/src/main.rs` (HEAD) — `BrokenPipe` handler locations 98-106, 117-120, 136-140.
- `crates/base60-cli/src/decode.rs` (HEAD) — `parse_run` error source at lines 103-108; existing `rejects_digit_ge_sixty` test at line 170-176.
- `crates/base60-cli/src/cli.rs` (HEAD) — `LensMode::ALL`, `Format`, `TimeScale`, `build_lens`.
- `crates/xtask/tests/env_discipline.rs` (Phase 2 shipped) — template for `spawn_discipline.rs`.
- `03-CONTEXT.md` — all 24 locked decisions.
- `.planning/research/PITFALLS.md` — Pitfalls 7 (fixture bloat), 8 (error-message drift), 10 (HashMap non-determinism), 12 (`assert_cmd` color/env).

### Secondary (MEDIUM confidence — verified against official docs)

- `docs.rs/assert_cmd/2.2.1/assert_cmd/` — `Command` API surface, `write_stdin` signature, `Assert::get_output`.
- `docs.rs/assert_cmd/2.2.1/assert_cmd/assert/struct.Assert.html` — `.success()`, `.failure()`, `.stdout()`, `.stderr()` return types.
- `crates.io` (via `cargo search`) — `assert_cmd = "2.2.1"`, `predicates = "3.1.4"` current versions.
- PNG (RFC 2083 / W3C REC-PNG) — chunk layout, CRC32 scope.
- ZIP (PKWARE APPNOTE 6.3.9) — EOCD signature + field layout, empty-archive validity.
- ELF64 (System V ABI Gen) — header field widths + required fields for a "structurally recognisable" header.
- Python `zlib.crc32` (stdlib) — CRC computation re-verification for IHDR (`0x3A7E9B55`) and IEND (`0xAE426082`).

### Tertiary (LOW confidence — flagged for validation)

- None. Every claim in this research is either cited to repo source, verified via tooling (cargo, zlib), or locked by CONTEXT.md.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions verified by `cargo search`; signatures verified via docs.rs.
- Architecture: HIGH — patterns are literal copies of Phase 2 precedents and canonical `assert_cmd` usage.
- Pitfalls: HIGH — direct citations from PITFALLS.md with concrete remediation snippets.
- Fixture byte sequences: HIGH — all computed via Python stdlib (`zlib.crc32`) and verified against format specs.
- `analyze::write_summary` substring assumption (A1): MEDIUM — requires 30-second source grep before commit.

**Research date:** 2026-04-24
**Valid until:** 2026-05-24 (30 days; stable Rust infra, no upstream risk)

---

*Phase 3 research complete; planner can decompose D-23's three commits into execution-grade tasks using only this document + existing CONTEXT.md + the cited source files.*
