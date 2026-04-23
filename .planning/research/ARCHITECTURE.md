# Architecture Research

**Domain:** Rust CLI hardening — `base60` v2 (tests + fuzz + benches + refactors + perf)
**Researched:** 2026-04-23
**Confidence:** HIGH (Cargo / cargo-fuzz / criterion / assert_cmd / strum conventions all verified via Context7 + upstream READMEs; project-specific decisions pinned to existing module boundaries)

This is *project* architecture research — not domain-ecosystem research. The v1 architecture is known (see `.planning/codebase/ARCHITECTURE.md`); the job here is to specify *where each v2 change slots in*, *who imports whom* afterwards, and *the build order* the roadmap can follow.

## Post-v2 System Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│                            WORKSPACE ROOT                                │
│  Cargo.toml  [workspace] members = [                                     │
│                  "crates/base60-core", "crates/base60-cli" ]             │
│  (fuzz/ joins as a workspace member — see §5)                            │
├──────────────────────────────────────────────────────────────────────────┤
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                       base60-core (library)                        │  │
│  │                                                                    │  │
│  │  convert │ chunk │ cuneiform │ lens │ url                          │  │
│  │     ▲       ▲         ▲        ▲      ▲                            │  │
│  │     │       │ NEW     │        │      │                            │  │
│  │     │       │ be_u64  │        │      │                            │  │
│  │  (unchanged — still zero external deps)                            │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                              ▲             ▲              ▲              │
│              path dep        │             │              │  path dep    │
│                              │             │              │              │
│  ┌──────────────────────────────────┐  ┌──────────────┐  ┌──────────────┐│
│  │     base60-cli (binary)          │  │  core bench  │  │  fuzz crate  ││
│  │                                  │  │   targets    │  │              ││
│  │  main │ cli │ reader │ dump      │  │              │  │  fuzz_targets││
│  │  format│decode│analyze│tui │…    │  │ convert_bench│  │  /decode.rs  ││
│  │                                  │  │ lens_bench   │  │  /search.rs  ││
│  │  + cli/benches/*.rs (NEW)        │  │              │  │              ││
│  │  + cli/tests/*.rs    (NEW)       │  └──────────────┘  └──────────────┘│
│  └──────────────────────────────────┘          │                 │       │
│                    ▲                           │                 │       │
│                    │ Command::cargo_bin        │ cargo bench     │ cargo │
│                    │ (built by cargo)          │                 │ fuzz  │
│                    │                           │                 │ run   │
│  ┌──────────────────────────────────┐          │                 │       │
│  │   integration test binaries      │          │                 │       │
│  │   crates/base60-cli/tests/       │          │                 │       │
│  │   ├── roundtrip.rs  (TEST-01)    │          │                 │       │
│  │   ├── fixtures.rs   (TEST-03)    │          │                 │       │
│  │   ├── env.rs        (TEST-04)    │          │                 │       │
│  │   ├── reader.rs     (TEST-05)    │          │                 │       │
│  │   ├── common/mod.rs (helpers)    │          │                 │       │
│  │   └── fixtures/     (bin corpus) │          │                 │       │
│  └──────────────────────────────────┘          │                 │       │
└──────────────────────────────────────────────────────────────────────────┘
```

## Recommended Post-v2 Project Structure

```
test-60/
├── Cargo.toml                         # workspace manifest; gains `"fuzz"` in members
├── Cargo.lock
├── test                               # STALE 5-byte file from early experimentation — DELETE (§1)
├── README.md
├── .github/workflows/ci.yml           # gains fuzz-smoke + bench jobs
├── docs/plans/…
├── .planning/…
├── crates/
│   ├── base60-core/
│   │   ├── Cargo.toml                 # gains [dev-dependencies] criterion, [[bench]] entries
│   │   ├── src/
│   │   │   ├── lib.rs                 # re-exports gain `chunk::be_u64`, `LensMode`
│   │   │   ├── convert.rs             # unchanged
│   │   │   ├── chunk.rs               # NEW — pub fn be_u64(&[u8]) -> u64  (REF-01)
│   │   │   ├── cuneiform.rs           # unchanged
│   │   │   ├── lens.rs                # gains `LensMode` enum + `render_to<W>` default (REF-02, PERF-04)
│   │   │   └── url.rs                 # unchanged
│   │   └── benches/
│   │       ├── convert_bench.rs       # PERF-06: u64_to_base60 hot path
│   │       └── lens_bench.rs          # PERF-06: render_to vs. render
│   └── base60-cli/
│       ├── Cargo.toml                 # gains dev-deps: assert_cmd, serial_test, criterion,
│       │                              # tempfile; [[bench]] entries
│       ├── src/
│       │   ├── main.rs                # unchanged except LensMode import from base60-core
│       │   ├── cli.rs                 # LensMode moves out (re-exported); strum derives optional
│       │   ├── reader.rs              # NEW fn stream_to<W>() for non-TUI dump (PERF-01)
│       │   ├── dump.rs                # drops local be_u64; calls base60_core::chunk::be_u64
│       │   ├── format.rs              # drops local be_u64; calls base60_core::chunk::be_u64
│       │   ├── decode.rs              # parse_run takes &[u8; RUN_LEN] (REF-03)
│       │   ├── analyze.rs             # gains online/streaming sparkline (PERF-05)
│       │   ├── search.rs              # find_all uses memchr::memmem (PERF-03)
│       │   ├── persist.rs             # LensMode now comes from base60-core; match still exhaustive
│       │   │                          # via strum::EnumIter (REF-02)
│       │   ├── tui.rs                 # analyze runs in background thread (PERF-02)
│       │   └── color.rs               # unchanged
│       ├── benches/
│       │   ├── dump_bench.rs          # PERF-06: dump_all throughput
│       │   ├── decode_bench.rs        # PERF-06: decode_stream throughput
│       │   └── search_bench.rs        # PERF-06: memmem vs. naive (gate for PERF-03)
│       └── tests/
│           ├── common/
│           │   └── mod.rs             # helpers: tempdir, bin(), golden-file assertions
│           ├── fixtures/              # committed binary corpus
│           │   ├── empty.bin
│           │   ├── zero_fill_1k.bin
│           │   ├── hello.txt
│           │   ├── tiny.elf           # x86_64 ELF header snippet
│           │   ├── tiny.png           # ≤2 KiB PNG
│           │   └── tiny.zip           # ≤2 KiB ZIP
│           ├── roundtrip.rs           # TEST-01: dump↔decode × lens × format
│           ├── fixtures.rs            # TEST-03: ELF/PNG/ZIP smoke via assert_cmd
│           ├── env.rs                 # TEST-04: NO_COLOR/NO_UNICODE/TERM=dumb serialised
│           ├── reader.rs              # TEST-05: mmap, stdin, --skip/--length boundaries
│           └── tui.rs                 # TEST-05: persist save-on-quit path
└── fuzz/                              # TEST-02 (cargo fuzz init); joins workspace members
    ├── Cargo.toml                     # path-dep on BOTH base60-core (search-pattern fuzz)
    │                                  # AND base60-cli (decode-run fuzz)
    ├── fuzz_targets/
    │   ├── parse_run.rs               # fuzzes decode::parse_run
    │   └── search_pattern.rs          # fuzzes search::Pattern::from_str
    └── .gitignore                     # auto-generated
```

### Structure Rationale

- **`crates/base60-core/src/chunk.rs` (new):** Single source of truth for the big-endian 8-byte→`u64` conversion currently duplicated in `dump.rs:35` and `format.rs:26`. Lives in the library so every consumer (CLI renderers, future bench harness, downstream users) sees the same code. The existing `convert.rs` is about base-60 digit conversion, not byte packing — promoting `be_u64` into `convert` would blur the module's single responsibility. A three-line new file is cheaper than a semantic overload. Matches the PROJECT.md decision: `be_u64 → base60-core::chunk`.
- **`LensMode` in `base60-core::lens` (moved):** `LensMode` is today in `crates/base60-cli/src/cli.rs:25`, yet every downstream (`cli::build_lens`, `persist::parse_lens`, `persist::serialize`, tui `L`-key cycle) reads a `LensMode` and decides which `Lens` impl to build. Co-locating the mode enum with the trait it dispatches over kills the "four parallel switch" problem in one stroke. The CLI re-imports it as `pub(crate) use base60_core::LensMode;`, so the existing `cli::build_lens` signature and `clap::ValueEnum` derive stay put (strum adds `EnumIter` for the table, `IntoStaticStr` for the label). The `clap::ValueEnum` derive stays on the CLI-side type — `base60-core` must not pick up a `clap` dependency.
- **`crates/base60-cli/tests/`:** Cargo's canonical integration-test location for a binary crate. `cargo test -p base60` discovers every `tests/*.rs`, builds each as its own binary, and `assert_cmd`'s `Command::cargo_bin("base60")` locates the produced `base60` binary automatically across workspace crates. No reason to invent a new crate for this.
- **`crates/base60-cli/tests/fixtures/`:** Binary corpus travels with the test code that consumes it — no split-brain between where the fixture lives and where the test runs. Fixtures are kept small (≤2 KiB each) so the repo stays lean.
- **`crates/base60-cli/tests/common/mod.rs`:** Shared test helpers (create tempdir, shell out to the bin, strip ANSI, compare to a golden file). The `common` subdir naming prevents cargo from treating `common.rs` as its own test binary — Cargo only builds top-level `tests/*.rs` as tests, a sub-directory `common/mod.rs` imported with `mod common;` is the recommended idiom.
- **The `/test` 5-byte file at repo root is *not* a directory** (`ls -la` confirmed: 5-byte plain file containing the word "test"). It's leftover experimentation, pre-dates the workspace split, and will confuse tooling if we reuse the name for a fixture dir. Delete it; don't reclaim it. Fixtures go inside the CLI crate's `tests/fixtures/` where they belong.
- **`fuzz/` at the repo root:** This is what `cargo fuzz init` generates by default. Placing it under `crates/base60-cli/fuzz/` technically works but the cargo-fuzz docs lead with the root-level pattern, CI examples all assume `cd fuzz && cargo fuzz run …`, and the fuzz crate will want to exercise code from *both* workspace members (`search::Pattern::from_str` lives in the CLI, `decode::parse_run` lives in the CLI but will depend on `chunk::be_u64` from core). A shared, repo-root `fuzz/` crate is the right scope.
- **`benches/` per crate:** Criterion benches live at the crate root alongside `src/`. Core-owned benches (`u64_to_base60`, lens render) go in `crates/base60-core/benches/`; CLI-owned benches (end-to-end `dump_all`, `decode_stream`, `search::find_all`) go in `crates/base60-cli/benches/`. Ownership rule: the benchmark lives next to the code it exercises and with the dev-dependencies it needs.

## Component Responsibilities (Post-v2)

| Component | Responsibility | Implementation |
|-----------|----------------|----------------|
| `base60-core::convert` | `u64` ↔ base-60 digits | Unchanged |
| `base60-core::chunk` (NEW) | Big-endian byte-slice → `u64` | `pub fn be_u64(&[u8]) -> u64`, right-zero-padding contract; moves from `dump.rs`/`format.rs` |
| `base60-core::cuneiform` | Glyph table | Unchanged |
| `base60-core::lens` | `Lens` trait + impls + `LensMode` dispatch enum | Gains `LensMode` (previously in CLI), gains `fn render_to<W: Write>(&self, chunk: u64, w: &mut W) -> io::Result<()>` default method that falls back to `write_all(self.render(chunk).as_bytes())` — existing impls opt in by overriding to stream without the intermediate `String` (PERF-04) |
| `base60-core::url` | URL-safe `u64` encode/decode | Unchanged |
| `base60-cli::cli` | clap parser, `build_lens` factory | Re-exports `LensMode` from `base60-core`; the clap `ValueEnum` stays on a thin CLI-side adapter if needed, or (cleaner) a newtype wrapper. `build_lens` is driven by a `match` that rust-analyser + `strum::EnumIter` can cross-check. |
| `base60-cli::reader` | mmap or stdin slurp | Gains `pub(crate) fn stream_to<W: Write>(path: Option<&Path>, skip, length, sink: impl FnMut(u64, &[u8]) -> io::Result<()>) -> io::Result<()>` that walks an 8-byte-chunk callback without materialising a `Vec<u8>` from stdin (PERF-01). Existing `load()` stays for the TUI / analyze paths that genuinely need random access. |
| `base60-cli::dump` | Text rendering | Drops local `be_u64`; imports `base60_core::chunk::be_u64`. Hot path unchanged. |
| `base60-cli::format` | JSON/HTML rendering | Same as `dump`. |
| `base60-cli::decode` | Dump → bytes | `parse_run` signature tightens to `fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64>` (REF-03); digit-validity check moves inside the function; scanner no longer has to prove the `&str` invariant. |
| `base60-cli::analyze` | Stats summary | Entropy sparkline becomes an online accumulator (PERF-05). No module split; the change is internal to `analyze::analyze`. |
| `base60-cli::search` | Pattern parser + finder | `find_all` swaps naive scan for `memchr::memmem::Finder` (PERF-03). Public surface unchanged. |
| `base60-cli::persist` | XDG state I/O | `parse_lens` / `serialize` matches stay exhaustive; with `LensMode` now coming from core + `strum::EnumIter`, a compile-time `for mode in LensMode::iter()` test in `#[cfg(test)]` guards against new variants being forgotten here. |
| `base60-cli::tui` | ratatui viewer | `analyze` moves off the critical path: on entry, spawn `std::thread::spawn` returning via a channel or an `Arc<Mutex<Option<Analysis>>>` shared state; the first frame draws with `analysis = None` (semantic-jump binds show "analysing…"); keypresses that need the analysis check-and-wait or show the pending marker (PERF-02). |
| `base60-cli::color` | Palette | Unchanged. |

## Import Graph After v2

**Pure dependency edges (compile-time):**

```
base60-core::convert  ←  base60-core::chunk     (if chunk uses DIGITS)
base60-core::convert  ←  base60-core::lens
base60-core::chunk    ←  base60-cli::dump
base60-core::chunk    ←  base60-cli::format
base60-core::lens     ←  base60-cli::cli
base60-core::lens     ←  base60-cli::dump
base60-core::lens     ←  base60-cli::format
base60-core::lens     ←  base60-cli::tui
base60-core::lens     ←  base60-cli::persist   (LensMode)
base60-core           ←  base60-cli/benches/*
base60-core           ←  base60-core/benches/*
base60-core + base60-cli ← fuzz/fuzz_targets/* (path deps on both)

base60-cli binary     ←  base60-cli/tests/*    (via Command::cargo_bin)
                         [process-level; no compile-time import]
```

**New third-party dev-dependencies (CLI crate only):**
- `assert_cmd = "2"` — `Command::cargo_bin("base60")` + `.assert().success().stdout(…)` (TEST-01/03/05)
- `serial_test = "3"` — `#[serial]` attribute on env-touching tests (TEST-04)
- `tempfile = "3"` — temp dirs for state-persistence tests (TEST-05)
- `criterion = { version = "0.5", default-features = false, features = ["html_reports"] }` — bench harness (PERF-06)
- `memchr = "2"` — runtime dep (not dev) in the CLI for `memmem` (PERF-03)

**New runtime dependency (workspace-wide):**
- `strum = { version = "0.26", features = ["derive"] }` — `EnumIter` / `EnumCount` / `IntoStaticStr` derives on `LensMode` (REF-02). Core-side dep; the "zero external deps" promise in PROJECT.md applies to *runtime behavioural* deps — `strum` is a derive crate that produces pure data. If that reading is too lenient, fall back to a hand-rolled `const LENS_MODES: [LensMode; 5] = [...];` + manual `impl LensMode { const ALL: &'static [Self] = …; }` and skip the derive. The dispatch table's *shape* is the point, not the crate that materialises it.

**Decision flag for roadmapper:** confirm whether `strum` is acceptable on `base60-core`. If not, the fallback is trivial and ships the same invariant.

## Data Flow Changes

### PERF-01: Streaming stdin (non-TUI dump)

**Before (`reader::load_stdin` → `dump::dump_all`):**
```
stdin → read_to_end → Vec<u8> (entire file in RAM) → chunks(8) → write_line × N → stdout
```

**After (`reader::stream_to` → per-chunk callback):**
```
stdin.lock() → BufReader → loop {
    read_exact(8 bytes) or short tail → callback(offset, &chunk)
} → write_line directly to BufWriter<StdoutLock>
```

- Never materialises the whole input.
- TUI / `analyze` paths keep `load()` because they need random access (bookmarks, search, per-window entropy).
- Shared code: both paths ultimately call `dump::write_line(&mut w, offset, bytes, palette, lens)`, which is already `W: Write`-generic and chunk-driven. No duplication.
- Dispatch point: `run_view` branches on `view.file.is_none() && !view.interactive && matches!(view.format, Ansi | Plain)` → stream path; everything else → load path. JSON/HTML also stream (they already chunk), so cover them in the same callback. Interactive + any file path still goes through `load()` (mmap gives random access for free).

### PERF-02: Lazy analyze in TUI

**Module footprint:** entirely within `tui.rs`. No new module.

**Before:** `ViewState::new` → `analyze::analyze(data, DEFAULT_WINDOW)` blocks on entry for a 1 GiB file.

**After:**
```rust
struct ViewState {
    …
    analysis: Arc<Mutex<Option<Analysis>>>,  // None until worker finishes
}

impl ViewState {
    fn new(data: &[u8], …) -> Self {
        let analysis = Arc::new(Mutex::new(None));
        {
            // Send an owned copy to the worker. `data` is `&[u8]` borrowed
            // from `Bytes::as_slice`; for mmap-backed inputs we can hand
            // the worker an `Arc<Bytes>` instead of copying.
            let analysis = Arc::clone(&analysis);
            let owned: Vec<u8> = data.to_vec();
            std::thread::spawn(move || {
                let a = analyze::analyze(&owned, DEFAULT_WINDOW);
                *analysis.lock().expect("poisoned") = Some(a);
            });
        }
        Self { …, analysis }
    }
}
```

- Semantic-jump keys (`]p`, `]z`, `]e` and their `[` counterparts) check `analysis.lock().unwrap().as_ref()`; if `None`, show `" analysing… "` in the status bar and ignore the keypress.
- Trade-off vs. compute-on-first-keypress: the thread-spawn path shows correct data immediately on first press *after* analysis completes (~1s for 100 MiB), whereas the lazy-key path stalls the TUI redraw loop on exactly the keystroke users care about. Thread is worth the complexity.
- Avoid zero-copy `&data` across the thread boundary (`'static` borrow requirement) by doing the `.to_vec()` once; for the TUI context, doubling memory once is fine — the problem we're solving is *first-frame latency*, not peak memory.
- Drop the thread on quit by letting it complete naturally (analyze is pure and short-lived). If a user quits mid-analysis the `Arc<Mutex>` refcount drops cleanly — no explicit cancellation needed.

### PERF-04: Streaming `Lens::render_to<W>`

**Trait addition (backwards-compatible):**
```rust
pub trait Lens: Send + Sync {
    fn render(&self, chunk: u64) -> String;

    /// Default: allocate via `render` and emit in one `write_all`.
    /// Override to stream without the intermediate `String`.
    fn render_to<W: Write>(&self, chunk: u64, w: &mut W) -> io::Result<()> {
        w.write_all(self.render(chunk).as_bytes())
    }
}
```
`dump::write_line` and `format::emit_json` / `emit_html` switch the active-lens branch to `lens.render_to(chunk_be, &mut w)?`. Per-line `String` vanishes for all overrides. Existing `render()` stays for JSON-string-escape paths that genuinely need the full value in memory (the current `write_json_string` needs the string to escape it).

## Integration Boundaries

### Test Crate ↔ Binary (TEST-01, TEST-03, TEST-05)

**Communication:** process-level, via `assert_cmd::Command::cargo_bin("base60")`.

**Pattern — roundtrip per (lens, format):**
```rust
// tests/roundtrip.rs
use assert_cmd::Command;

#[test]
fn roundtrip_ansi_no_lens() {
    let fixture = include_bytes!("fixtures/zero_fill_1k.bin");
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.bin");
    std::fs::write(&input, fixture).unwrap();

    let dump = Command::cargo_bin("base60").unwrap()
        .arg("--format=plain").arg(&input)
        .assert().success()
        .get_output().stdout.clone();

    let back = Command::cargo_bin("base60").unwrap()
        .arg("decode").write_stdin(dump)
        .assert().success()
        .get_output().stdout.clone();

    assert_eq!(back, fixture);
}
```
A `build.rs`-free approach using Cargo's standard test discovery. `common::lens_format_matrix()` generates the `(LensMode × Format)` product; each test body is one function but the assertion lives in a shared helper to keep noise low.

### Fuzz Crate ↔ Target Crates (TEST-02)

**Communication:** path-dependency import.

`fuzz/Cargo.toml`:
```toml
[package]
name = "base60-fuzz"
version = "0.0.0"
publish = false
edition = "2024"

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"
base60-core = { path = "../crates/base60-core" }
# `parse_run` and `Pattern::from_str` are `pub(crate)` today. To fuzz them,
# either (a) promote the hot entry points to `pub` on a `#[doc(hidden)]` 
# `pub mod __fuzz;` escape hatch in `base60-cli` (no), or (b) re-home the 
# pure-logic halves in the core crate. Pattern parsing has no CLI dependency 
# — move `search::{Pattern, parse}` into `base60-core::search`. Decode 
# likewise — `parse_run` needs only DIGITS from core, so move it alongside 
# `chunk::be_u64`. After moves the fuzz crate depends on base60-core alone.

[[bin]]
name = "parse_run"
path = "fuzz_targets/parse_run.rs"
test = false
doc = false
bench = false

[[bin]]
name = "search_pattern"
path = "fuzz_targets/search_pattern.rs"
test = false
doc = false
bench = false
```

`fuzz/fuzz_targets/parse_run.rs`:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() != base60_core::decode::RUN_LEN { return; }
    let arr: &[u8; 32] = data.try_into().unwrap();
    let _ = base60_core::decode::parse_run(arr, 1);
});
```

**Workspace-integration decision:** add `"fuzz"` to `Cargo.toml [workspace] members`. The cargo-fuzz README documents this as the default and happy-path. An independent nested workspace via `--fuzzing-workspace=true` is only needed when the fuzz crate pulls in a conflicting resolver or MSRV — neither applies here. Adding it to members means `cargo check --workspace` covers the fuzz crate, matching our CI discipline.

**Secondary REF consequence:** this pressure pushes `parse_run` and `Pattern` parsing into `base60-core`. That's a net win — both are pure, depend only on primitives, and would benefit from doctests alongside their inverse. Update the architecture table accordingly: `base60-core::decode` (new module) and `base60-core::search` (new module) become the fuzzable surfaces.

### Bench Crate Layout (PERF-06)

- `crates/base60-core/benches/convert_bench.rs` — `u64_to_base60` hot loop over random u64s.
- `crates/base60-core/benches/lens_bench.rs` — `render` vs `render_to` throughput (gates PERF-04).
- `crates/base60-cli/benches/dump_bench.rs` — `dump_all` against 1 MiB random buffer (gates PERF-04).
- `crates/base60-cli/benches/decode_bench.rs` — `decode_stream` over 1 MiB dump (gates REF-03).
- `crates/base60-cli/benches/search_bench.rs` — `find_all` naive vs. `memmem` (gates PERF-03).

Each `Cargo.toml` gains `[[bench]] name = "…", harness = false` entries plus `[dev-dependencies] criterion = …`. Use `criterion_group!{name = …; config = Criterion::default().sample_size(50); targets = …}` so a full `cargo bench` run finishes under ~30s on CI; tighter precision reserved for the investigative runs a human drives.

## Architectural Patterns

### Pattern 1: Dispatch-table for `LensMode` (REF-02)

**What:** Replace four parallel `match mode { None => …, Time => …, … }` with a single iteration over `LensMode::iter()` that yields every variant and its companion data (label, constructor arg, next-in-cycle target). `strum::EnumIter` + `strum::IntoStaticStr` does this in ~5 lines of derive.

**When to use:** any time the number of call sites that need to enumerate variants exceeds one. The current codebase has four (`cli::build_lens`, `cli::LensMode::cycle`, `cli::LensMode::label`, `persist::parse_lens`, plus `persist::serialize` using `label`). Forgetting any one is a silent bug (see PROJECT.md: "Adding a lens forgets at least one").

**Trade-offs:** adds a derive-only compile-time dep. Worth it for the lint: `match mode` over `LensMode::iter().collect::<Vec<_>>()` gives the compiler `non_exhaustive_match` checks automatically, plus a test `for m in LensMode::iter() { assert!(build_lens(m, …).is_some_or(m == None)) }` catches regressions by construction.

**Example:**
```rust
// base60-core/src/lens.rs
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, strum::EnumIter, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum LensMode { #[default] None, Time, Angle, Tablet, Cuneiform }

impl LensMode {
    #[must_use]
    pub const fn next(self) -> Self {
        match self { /* cycle */ }
    }
}

// CLI side — build_lens:
pub(crate) fn build_lens(m: LensMode, scale, purist) -> Option<Box<dyn Lens>> {
    match m {
        LensMode::None => None,
        LensMode::Time => Some(Box::new(TimeLens { scale: scale.into() })),
        LensMode::Angle => Some(Box::new(AngleLens)),
        LensMode::Tablet => Some(Box::new(TabletLens { purist })),
        LensMode::Cuneiform => Some(Box::new(CuneiformLens::auto())),
    }
}

// Persist side — becomes data-driven:
fn parse_lens(val: &str) -> LensMode {
    LensMode::iter().find(|m| Into::<&str>::into(*m) == val).unwrap_or_default()
}
```
The `match` in `build_lens` stays (each variant has different constructor args), but it's the *only* `match` on `LensMode`; everything else reads the variant set from `iter()`.

### Pattern 2: Test fixtures as embedded bytes (TEST-03)

**What:** `include_bytes!("fixtures/tiny.elf")` inside the test compiles the fixture into the test binary. No filesystem path wrangling, works under cross-compilation, fails fast at compile time if the fixture file is missing.

**When to use:** for every fixture that fits in <= ~10 KiB and isn't being mutated. For the cases where a real file is required (the mmap path under test), write the bytes into a `tempfile::NamedTempFile` at test-start.

**Trade-offs:** fixtures pinned to binary size; not a concern at 2 KiB each.

### Pattern 3: Online streaming accumulator (PERF-05)

**What:** for the per-window entropy sparkline, compute Shannon incrementally instead of accumulating a `Vec<f32>` of every window's entropy.

**When to use:** a sparkline or rolling summary statistic is about to materialise a `Vec<T>` with one element per window over gigabyte-scale input.

**Trade-offs:** sparkline becomes pull-based — callers that want every window's value lose that capability. Check TUI semantic-jumps — they use `HighEntropy` / `LowEntropy` regions, which come from `analyze::detect_regions` that consumes the full `Vec`, not the sparkline. Refactor in two steps: (1) split `analyze::analyze` into a pure-regions function + a sparkline iterator; (2) swap the iterator to online. Detect-regions keeps the `Vec`, as it's bounded by `total / window_size`.

## Anti-Patterns

### Anti-Pattern 1: Putting integration tests in the root workspace

**What people do:** create `/tests/` at workspace root.
**Why it's wrong:** `cargo test --workspace` doesn't discover top-level `tests/` directories; they only work inside a package. Tests under root silently don't run.
**Do this instead:** `crates/base60-cli/tests/*.rs` under the CLI package. `cargo test -p base60` runs them; workspace-level `cargo test` also picks them up via member traversal.

### Anti-Pattern 2: One giant `integration.rs`

**What people do:** write every integration test in a single `tests/integration.rs`.
**Why it's wrong:** Cargo compiles each `tests/*.rs` as its own binary. One file means one link, one test process, serialised execution of every test — bad when some tests are env-touching and need `#[serial]` while others can parallelise.
**Do this instead:** one concern per `tests/*.rs`, matching the source-module-per-concern convention. `env.rs` alone runs serially; `roundtrip.rs` parallelises freely.

### Anti-Pattern 3: Fuzz targets that touch CLI I/O

**What people do:** `fuzz_target!(|data: &[u8]| { /* spawn base60 binary, pipe bytes */ })`.
**Why it's wrong:** process spawn dwarfs the fuzz iteration budget, coverage feedback is attenuated by process boundaries, and signal handling becomes a separate science.
**Do this instead:** fuzz pure-function entry points (`parse_run`, `Pattern::from_str`). Promote those functions to `pub` in `base60-core` as part of REF-02 and REF-03's natural re-homing.

### Anti-Pattern 4: Leaving `be_u64` duplicated "because it's three lines"

**What people do:** shrug — "it's the same three lines, copy it."
**Why it's wrong:** the module-level comment in `format.rs:23-24` already acknowledges the divergence risk. A silent mismatch between JSON output and text output is exactly the bug the downstream decode tests *can't* catch (decode only parses text).
**Do this instead:** REF-01 moves it to `base60-core::chunk::be_u64`. Cost: one new file, two call-site updates. Benefit: the two renderers are provably byte-identical on the `u64` intermediate.

## Scaling Considerations

This is a CLI, not a service. "Scale" here means input size per invocation, not concurrent users.

| Input size | v1 behaviour | v2 behaviour |
|-----------|--------------|--------------|
| <= ~100 MiB file | mmap is instant; TUI responsive; dump streams fine | Unchanged |
| ~1 GiB file (mmap) | mmap is instant; TUI analyze stalls first frame | TUI analyze runs off-thread (PERF-02) |
| stdin > RAM (e.g. `base60 < /dev/sda`) | OOM | Streams via chunk callback (PERF-01) |
| Lens over 10M chunks | Per-line `String` allocation dominates | `render_to<W>` avoids allocation (PERF-04) |
| `search::find_all` over 100 MiB | Quadratic worst-case on pathological input | `memchr::memmem` — SIMD-optimised (PERF-03) |

The benches (PERF-06) are the *regression guardrail*, not the delivery mechanism — any perf change ships with a criterion group that fails CI if the change is a net loss.

## Suggested Build Order

Changes fall into four waves. Within a wave, items are parallelisable; between waves, the earlier output is a hard input to the later.

### Wave 1 — Foundations (serialisable work, sets up the shared contract)

Order within the wave matters a little:

1. **REF-01** — Move `be_u64` to `base60-core::chunk`. Touches `dump.rs`, `format.rs`, adds three lines to `base60-core`. Tiny, risk-free, unblocks everything that wants to call it. **Must ship first.**
2. **REF-02** — Move `LensMode` to `base60-core::lens`; add `strum::EnumIter`. Touches `cli.rs`, `persist.rs`, `tui.rs`, `lens.rs`. Medium surface, no behaviour change. **Should ship second.**
3. **REF-03** — Tighten `parse_run` signature. Local to `decode.rs`. **Can ship in parallel with REF-02.**
4. **Move `parse_run` + `search::Pattern` parsing into `base60-core`** (pure pre-requisite for TEST-02). Implied by cargo-fuzz boundary. **Ships alongside REF-03.**

**Why this order:** REF-01 is everyone's precondition (the benches in Wave 2 want to call `be_u64` directly). REF-02 unlocks the exhaustive-variant invariant used by TEST-01's test-matrix generator. REF-03 tightens a contract that TEST-02's fuzz target will exercise.

**Parallel-safe in Wave 1:** REF-02 ↔ REF-03 (disjoint files).
**Serial in Wave 1:** REF-01 → everything (touches files Wave 2 reads).

### Wave 2 — Safety net (tests + benches) before perf

Nothing in Wave 2 changes behaviour. All items can ship in parallel once Wave 1 merges.

5. **TEST-01** — Roundtrip matrix (lens × format). Uses REF-02's `LensMode::iter()` to generate variants.
6. **TEST-02** — Fuzz targets against `parse_run` and `Pattern::from_str`. Uses REF-03's tightened signature.
7. **TEST-03** — Fixture-driven `assert_cmd` tests (ELF/PNG/ZIP). Independent.
8. **TEST-04** — `serial_test` on env-touching tests. Independent, mechanical.
9. **TEST-05** — Cover mmap / stdin / TUI-persist paths. Depends on nothing; independent.
10. **PERF-06** — Criterion bench scaffolding. Must land before any PERF-0X change so those changes have a regression gate. Independent of every test item.

**Parallel-safe in Wave 2:** all five TEST items + PERF-06 (disjoint new files).
**Serial in Wave 2:** none — each edits a different new file.

### Wave 3 — Performance, each gated by a bench landed in Wave 2

Each of these ships only after the matching bench in Wave 2 is green.

11. **PERF-03** — `memchr::memmem` in `search::find_all`. Gated by `search_bench.rs`.
12. **PERF-04** — `Lens::render_to<W>`. Gated by `lens_bench.rs` + `dump_bench.rs`.
13. **PERF-01** — Streaming stdin in non-TUI dump. Gated by `dump_bench.rs` (memory-use axis, not just throughput — track peak RSS in a separate integration test).
14. **PERF-05** — Online entropy sparkline. Gated by a new `analyze_bench.rs` (add to Wave 2 if PERF-05 is scoped in; mark as a sub-task otherwise).
15. **PERF-02** — Async analyze in TUI. Not gated by a criterion bench (TUI latency is hard to microbench meaningfully); gated instead by a manual stopwatch check documented in the PR description + a TEST-05 assertion that the first frame is produced within a bound on a 100 MiB synthetic file.

**Parallel-safe in Wave 3:** PERF-03 ↔ PERF-04 ↔ PERF-02 (disjoint files: `search.rs`, `lens.rs` + render callsites, `tui.rs`).
**Serial in Wave 3:** PERF-01 depends on PERF-04 being in place (streaming path calls `lens.render_to(…, w)`; without PERF-04 it allocates per chunk and the peak-RSS claim collapses). PERF-05 depends on PERF-01's decision about whether `analyze` itself should stream its input (if yes, PERF-05 inherits the `stream_to` callback shape).

### Wave 4 — Documentation + CI

16. Update `.planning/codebase/ARCHITECTURE.md` to reflect the post-v2 module layout.
17. Add CI jobs: `cargo bench --no-run` (sanity-check benches compile), `cd fuzz && cargo +nightly fuzz build` (smoke the fuzz crate), per-PR `cargo test --workspace` (already on, but verify it picks up the new `tests/*.rs`).

## Integration Points

### External Dev-Tooling

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| `cargo-fuzz` (nightly) | `cd fuzz && cargo +nightly fuzz run <target>` | Not part of default `cargo test`; nightly-only. CI smoke = `cargo +nightly fuzz build` on one target per PR. |
| `criterion` | `cargo bench -p base60-core` / `-p base60` | Runs in release; not in default CI (takes minutes). Dedicated `bench.yml` workflow on PR label, or weekly schedule. |
| `assert_cmd` | `Command::cargo_bin("base60")` in `tests/*.rs` | Cargo builds the bin automatically before running tests; no extra orchestration. |
| `serial_test` | `#[serial]` attribute on env-touching tests | Pair with Wave 1 REF-02 so `persist.rs::state_base_dir` can finally be covered. |
| `tempfile` | `tempfile::tempdir()` for persist + fixture copy | Honours `TMPDIR`, cleans up on drop, works cross-platform. |

### Internal Module Boundaries (post-v2)

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `base60-cli/tests/*.rs` ↔ `base60` binary | Process spawn via `Command::cargo_bin` | Process-level; no shared state; reads stdout/stderr; works across workspace members by design. |
| `fuzz/` ↔ `base60-core` | Rust path-dep + `pub` entry points | Direct function call inside libFuzzer harness; the `pub` promotion is intentional and `#[doc(hidden)]` where appropriate. |
| `crates/base60-cli/benches/*.rs` ↔ `base60-cli::{dump, decode, search, analyze}` | Criterion needs `pub(crate)` → `pub` on the specific entry points being benched (OR) benches live inside `#[cfg(bench)]` test modules | Prefer: add `#[doc(hidden)] pub mod __bench;` re-exports for the exact functions. Keeps the public API stable while letting benches call into internals. |
| `crates/base60-core/benches/*.rs` ↔ `base60-core` | Public API only (it's a library) | Trivial — benches call the actual published surface. |
| `tui.rs` analysis thread ↔ main loop | `Arc<Mutex<Option<Analysis>>>` | One-shot, no cancellation; analysis is pure and finishes fast enough that abandonment via refcount-drop is fine. |
| `reader::stream_to` ↔ `dump::write_line` | Function pointer / `impl FnMut` callback | Keeps the streaming concern (I/O) separate from the per-chunk formatting concern; matches the existing chunk-driven shape. |

## Sources

- **Verified HIGH:** Context7 `/rust-fuzz/cargo-fuzz`, `/bheisler/criterion.rs`, `/peternator7/strum` (derive behaviours, cargo-fuzz directory layout, criterion bench declaration, strum EnumIter/IntoStaticStr).
- **Verified HIGH:** cargo-fuzz README — workspace integration: "If your crate uses cargo workspaces, add `fuzz` directory to `workspace.members`" and `cargo fuzz init --fuzzing-workspace=true` for independent workspace.
- **Verified MEDIUM:** `docs.rs/assert_cmd/2` — `Command::cargo_bin` auto-discovery across workspace members, canonical `tests/*.rs` location.
- **Verified HIGH:** The codebase itself (`.planning/codebase/ARCHITECTURE.md`, `STRUCTURE.md`, `CONVENTIONS.md`, `Cargo.toml`, individual module source) — all module names, line counts, import graphs, and style commitments cited here.
- **Verified HIGH:** cargo book — `tests/*.rs` integration-test discovery; `tests/common/mod.rs` subdirectory idiom to avoid the `tests/common.rs` gotcha.

---

*Architecture research for: Rust CLI hardening milestone (base60 v2)*
*Researched: 2026-04-23*

## Unresolved questions

- OK to take `strum` as a dep on `base60-core`? Crate ships zero runtime deps today; derive macros are compile-time only but some would read PROJECT.md's "must keep zero external dependencies" as a hard rule. Fallback: hand-rolled `LensMode::ALL: &'static [Self]` + `IntoStaticStr`-by-hand. Affects REF-02 shape only.
- `parse_run` / `search::Pattern` move to `base60-core` — net gain (enables fuzz, doctests) but grows the library's public surface. Accept that surface growth, or gate behind `#[doc(hidden)]`?
- PERF-05 in-scope this milestone, or defer? PROJECT.md lists it Active, but it has the most semantic risk (changes what `analyze` can return) and the least urgency.
- Bench CI budget — on every PR, or nightly / on-demand? Runtime for a full criterion sweep is minutes; PR budget may not tolerate it.
- Track peak-RSS for PERF-01 how? No stdlib way; add a `procfs` dev-dep (Linux-only) or accept a "didn't OOM on `/dev/zero | head -c 10G`" smoke test?
