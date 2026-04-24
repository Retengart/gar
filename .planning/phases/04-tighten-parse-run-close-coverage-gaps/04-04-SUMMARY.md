---
phase: 04-tighten-parse-run-close-coverage-gaps
plan: 04
subsystem: testing
tags: [rust, cli, integration-tests, tui, persist, ratatui, testbackend, serial-test, test-05]

# Dependency graph
requires:
  - phase: 04-tighten-parse-run-close-coverage-gaps
    provides: "tempfile dev-dep pinned by 04-03; base60_cmd hermetic spawner; serial_test shared-key idiom; xtask env_discipline + spawn_discipline gates"
provides:
  - "crates/base60-cli/src/tui.rs::run_with_terminal<B: Backend, F: FnMut() -> io::Result<Option<Event>>> seam — pub #[doc(hidden)] so integration tests can drive the TUI against ratatui::backend::TestBackend; production `run` is a 6-line wrapper"
  - "crates/base60-cli/src/lib.rs::__test_hooks::run_with_terminal — hidden re-export gated by __ prefix; no stability guarantee"
  - "crates/base60-cli/src/lib.rs::__TuiTimeScale — hidden alias for cli::TimeScale so test drivers can pass a scale value without widening the public API"
  - "crates/base60-cli/tests/common/mod.rs::drive_tui_to_quit_with_fixture — shared helper driving j*5 + m + a + q through an 80x24 TestBackend"
  - "crates/base60-cli/tests/tui.rs — 1 passing #[serial(env)] integration test asserting cursor=40 + bookmarks=a:40 in the persisted state file"
  - "crates/base60-cli/tests/persist.rs — 3 passing #[serial(env)] tests pinning the XDG_STATE_HOME -> HOME -> None state-dir fallback ladder"
  - "TEST-05 fully closed — all three sub-targets (reader mmap/stdin/file-open-error, TUI exit-with-save, persist env ladder) now have end-to-end coverage"
affects: [phase-5-fuzz-bench-scaffolding]

# Tech tracking
tech-stack:
  added:
    - "ratatui::backend::TestBackend — first consumer in the workspace; used via the 80x24 canvas in tests/common::drive_tui_to_quit_with_fixture"
  patterns:
    - "run_with_terminal<B: Backend, F: FnMut() -> io::Result<Option<Event>>> seam — production path wraps with ratatui::run + crossterm::event::read; tests inject TestBackend + Vec<Event> iterator"
    - "#[doc(hidden)] pub re-exports under an __ prefix / __test_hooks module — expose a narrow surface for integration tests without widening the public API"
    - "Env-ladder coverage via snapshot-restore: prev_xdg / prev_home std::env::var_os captures + conditional set_var/remove_var on teardown keeps the test harness hermetic even when Unix CI has HOME preset"
    - "Directory-scan for state file rather than recomputing FNV-1a — Pitfall 6 (canonicalisation path differs on macOS /private/tmp symlinks); rely on '.state' file count in the tempdir"

key-files:
  created:
    - "crates/base60-cli/tests/tui.rs — 1 integration test, 68 lines"
    - "crates/base60-cli/tests/persist.rs — 3 integration tests, 130 lines"
    - ".planning/phases/04-tighten-parse-run-close-coverage-gaps/04-04-SUMMARY.md — this file"
  modified:
    - "crates/base60-cli/src/tui.rs — extracted run_with_terminal<B, F> seam; run() becomes a thin ratatui::run wrapper"
    - "crates/base60-cli/src/lib.rs — added __TuiTimeScale alias + __test_hooks module"
    - "crates/base60-cli/src/cli.rs — widened TimeScale from pub(crate) to pub (needed so lib.rs can pub-use-alias it across the re-export boundary)"
    - "crates/base60-cli/tests/common/mod.rs — added drive_tui_to_quit_with_fixture helper + const fn key(c: char) -> Event"

key-decisions:
  - "Widened run_with_terminal to pub (with #[doc(hidden)]) rather than keeping pub(crate). Rustc rejects `pub use crate::tui::run_with_terminal` when the target is pub(crate) (E0364). The plan's acceptance-criteria grep `pub(crate) fn run_with_terminal` has been relaxed to `pub fn run_with_terminal` — same seam shape, strict test-only surface contract preserved by the __ prefix + #[doc(hidden)]."
  - "Widened cli::TimeScale from pub(crate) to pub for the same reason — the __TuiTimeScale re-export requires it. Added a doc comment explaining the test-only motivation."
  - "#[allow(clippy::too_many_arguments)] on run_with_terminal. 8 args maps 1:1 to fn run's 6 args + &mut Terminal + event closure — splitting would just move the tuple one indirection deeper without reducing total argument count at call sites."
  - "Used the top-level form `#[doc(hidden)] pub use cli::TimeScale as __TuiTimeScale` rather than the alternative `__test_hooks::TimeScale` nested form. Fewer surface paths; matches the plan's Part B default."
  - "No #[cfg(not(windows))] gates applied. persist::state_base_dir reads HOME literally on every platform, so explicitly setting it via env::set_var makes the tests OS-agnostic. Should Windows CI ever fail, the plan documents the escalation path (gate + comment)."
  - "Each task landed as a single atomic commit rather than RED/GREEN split. Task 1 is a mechanical refactor (no behavior change); Tasks 2-4 are test additions against already-correct production code. Matches Phase 4 D-17 atomic-green granularity + the precedent set by 04-03-SUMMARY."
  - "Manual smoke check (Part C) could not open an interactive terminal in the autonomous executor; production-path equivalence verified instead via cargo run -p base60 -- --color=never --length=16 README.md (non-TUI path produces identical dump including the `# bytes=0x10` trailer). The TUI delegation is a pure structural wrapper compiled+clippy+doc-checked against the same types and closures."

patterns-established:
  - "Test-only visibility widening via `#[doc(hidden)] pub` + `__` prefix — alternative to an entire `test-internals` feature flag. Zero runtime cost; no additional Cargo.toml surface."
  - "In-process TUI drive via ratatui::backend::TestBackend + Vec<Event> iterator closure — reusable shape for any future TUI integration test (search, bookmark jumps, semantic navigation)."
  - "XDG_STATE_HOME redirect via tempfile::tempdir + #[serial(env)] — hermetic, cross-platform, auto-cleanup state-dir testing pattern."

requirements-completed: [TEST-05]

# Metrics
duration: 19min
completed: 2026-04-24
---

# Phase 4 Plan 04: TUI TestBackend + Persist Env-Fallback Coverage Summary

**Closes the final TEST-05 coverage strand: extracts a generic `run_with_terminal<B, F>` seam from `tui::run`, exposes it via hidden `__test_hooks`, then drives the TUI through `j j j j j m a q` against an 80x24 `TestBackend` to assert the persisted state file contains `cursor=40` + `bookmarks=a:40`. Three `#[serial(env)]` tests pin `persist::state_base_dir`'s XDG -> HOME -> None ladder. All 4 new tests pass; workspace gate green at `--test-threads=8`.**

## Performance

- **Duration:** ~19 min
- **Started:** 2026-04-24T15:59:03Z (approx, worktree base commit)
- **Completed:** 2026-04-24T16:18:26Z
- **Tasks:** 4 (1 refactor + 3 test additions)
- **Files modified:** 4 (src/tui.rs, src/lib.rs, src/cli.rs, tests/common/mod.rs)
- **Files created:** 3 (tests/tui.rs, tests/persist.rs, 04-04-SUMMARY.md)
- **New tests:** 4 (1 in `tests/tui.rs`, 3 in `tests/persist.rs`) — all passing first run
- **Task commits:** 4

## Accomplishments

- Extracted `run_with_terminal<B: Backend, F: FnMut() -> io::Result<Option<Event>>>(...)` seam in `src/tui.rs`; production `fn run` becomes a 13-line wrapper delegating to it with `|| crossterm::event::read().map(Some)` as the event source.
- `Ok(None)` from `next_event` triggers graceful save+exit — production path never hits it (real `crossterm::event::read` always blocks-or-succeeds), but tests can exhaust an iterator without a final `q` press and still get a clean shutdown.
- Added `#[doc(hidden)] pub use cli::TimeScale as __TuiTimeScale` + `#[doc(hidden)] pub mod __test_hooks { pub use crate::tui::run_with_terminal; }` in `src/lib.rs`. Widened `cli::TimeScale` from `pub(crate)` to `pub` with a module-local doc note explaining the test-only motivation.
- Added `drive_tui_to_quit_with_fixture(fixture_bytes, fixture_path)` helper to `tests/common/mod.rs`. In-process 80x24 `TestBackend` + pre-built `Vec<Event>` drives the canonical sequence `j j j j j m a q`. Helper is env-free (callers own XDG_STATE_HOME/HOME mutation under `#[serial(env)]`).
- New `tests/tui.rs`: 1 `#[serial(env)]` integration test asserting `cursor=40` and `bookmarks=a:40` in the persisted state file after the drive.
- New `tests/persist.rs`: 3 `#[serial(env)]` tests covering the XDG -> HOME -> None fallback ladder with snapshot/restore of both env vars around each test body.
- Full workspace gate green: `cargo test --workspace --all-targets --locked` + `--test-threads=8` variant + `cargo clippy --workspace --all-targets --locked -- -D warnings` + `cargo fmt --all --check` + `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`.
- spawn_discipline gate still green (no raw `Command::cargo_bin` outside `tests/common/`).
- env_discipline gate still green for crate sources (workspace lib.rs env mutation stays under `#[serial(env)]`); env mutation inside `tests/*.rs` is eyeball-verified per plan (Pitfall 5, `xtask env_discipline` does not walk tests/).

## Task Commits

| # | Task                                                            | Hash      | Type     |
|---|-----------------------------------------------------------------|-----------|----------|
| 1 | Extract run_with_terminal seam + __test_hooks re-exports         | `f479058` | refactor |
| 2 | Add drive_tui_to_quit_with_fixture helper to tests/common        | `30b2d46` | test     |
| 3 | TestBackend integration test asserting state file contents       | `864b172` | test     |
| 4 | XDG -> HOME -> None ladder tests in tests/persist.rs [TEST-05]   | `2aeb79f` | test     |

## `src/tui.rs` seam extraction — exact diff

Before (tui.rs:53-90, 38 lines):

```rust
pub(crate) fn run(
    data: &[u8],
    base_offset: u64,
    initial_mode: LensMode,
    scale: TimeScale,
    purist: bool,
    input_file: Option<&Path>,
) -> Result<()> {
    let mut state = ViewState::new(data, initial_mode, scale, purist);

    let persist_path: Option<PathBuf> = input_file.map(Path::to_path_buf);
    if let Some(path) = &persist_path
        && let Some(saved) = persist::load(path)
    {
        state.apply_persisted(saved);
    }

    ratatui::run(|terminal| -> Result<()> {
        loop {
            terminal.draw(|frame| state.draw(frame, data, base_offset))?;

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if state.handle_key(key.code, key.modifiers, data).is_break() {
                if let Some(path) = &persist_path {
                    persist::save(path, &state.snapshot());
                }
                break Ok(());
            }
        }
    })
}
```

After (tui.rs:56-75 + 77-142, 13-line wrapper + 55-line seam):

```rust
pub(crate) fn run(
    data: &[u8],
    base_offset: u64,
    initial_mode: LensMode,
    scale: TimeScale,
    purist: bool,
    input_file: Option<&Path>,
) -> Result<()> {
    ratatui::run(|terminal| -> Result<()> {
        run_with_terminal(
            terminal,
            data,
            base_offset,
            initial_mode,
            scale,
            purist,
            input_file,
            || crossterm::event::read().map(Some),
        )
    })
}

// 8 args maps 1:1 to `run`'s 6 args plus the `&mut Terminal` + event
// closure injected for tests; splitting would just move the tuple one
// indirection deeper without improving the call sites.
#[allow(clippy::too_many_arguments)]
#[doc(hidden)]
pub fn run_with_terminal<B, F>(
    terminal: &mut Terminal<B>,
    data: &[u8],
    base_offset: u64,
    initial_mode: LensMode,
    scale: TimeScale,
    purist: bool,
    input_file: Option<&Path>,
    mut next_event: F,
) -> Result<()>
where
    B: Backend,
    B::Error: std::error::Error + Send + Sync + 'static,
    F: FnMut() -> io::Result<Option<Event>>,
{
    // ... state setup, persist::load, event loop with `next_event()?` ...
    // On Ok(None): save + return Ok(()).
    // On Event::Key with handle_key().is_break(): save + return Ok(()).
}
```

The `B::Error: std::error::Error + Send + Sync + 'static` bound was added during execution (Rule 3 — blocking compile error): `?` through `terminal.draw(...)` into `anyhow::Result<()>` needs `B::Error` to implement `From<_> for anyhow::Error`, which requires Send + Sync + 'static + Error. Without the bound, `CrosstermBackend<Stdout>` works (production path) but the generic definition fails to compile.

## `__test_hooks` final form

Chose the top-level `__TuiTimeScale` alias + nested `__test_hooks::run_with_terminal` form (Part B default), not the alternative `__test_hooks::TimeScale` nested form documented in Task 2's action. The default was cleaner at call sites (`base60::__TuiTimeScale::Gar` vs `base60::__test_hooks::TimeScale::Gar`) and matched the plan's primary suggestion.

```rust
// src/lib.rs (final):
pub use cli::{Format, LensMode};

#[doc(hidden)]
pub use cli::TimeScale as __TuiTimeScale;

#[doc(hidden)]
pub mod __test_hooks {
    pub use crate::tui::run_with_terminal;
}
```

## State-file contents (observed)

Per `persist::serialize` (persist.rs:95-113), the file under `$XDG_STATE_HOME/base60/<fnv1a>.state` after the `j j j j j m a q` drive contains:

```text
scroll=0
cursor=40
lens=—
bookmarks=a:40
```

Math confirmed: 5 j-presses * CHUNK=8 bytes = cursor offset 40; bookmark slot `a` captures the cursor at slot-set time (the `m` -> `BookmarkSet` mode, then `a` dispatches to `handle_bookmark_key` which stores `('a', 40)`). `scroll=0` because the 1024-byte fixture at 80x24 keeps offset 40 on-screen without viewport scrolling. `lens=—` because `LensMode::None`'s label is `—` (em-dash).

## Drive-sequence correction (CONTEXT.md D-15 vs reality)

The CONTEXT.md D-15 hint recommended `j j j j j b 1 q` which is INCORRECT — the TUI bookmark handler at `tui.rs:428-454` only accepts `is_ascii_alphabetic()` letters; digit `'1'` is rejected with `bookmarks use a-z, got '1'`. Per RESEARCH Example 7 + PATTERNS tui.rs:361 trace, the correct sequence is `j j j j j m a q`:
- `m` enters `Mode::BookmarkSet`.
- `a` dispatches to `handle_bookmark_key` which stores `('a', cursor_byte)`.
- `q` breaks out of the loop; state saves.

## Windows `#[cfg(not(windows))]` gates applied

**None.** `persist::state_base_dir` reads `HOME` literally on every platform (persist.rs:78), and the tests SET `HOME` explicitly via `env::set_var`, so the behaviour is platform-independent. The plan documented the escalation path (gate + comment) should Windows CI ever fail, but no such failure occurred. A trial `cargo test -p base60 --test persist --locked` pass confirmed on Linux; cross-platform CI remains the final arbiter.

## Final test count

| File                                            | New tests | Drive sequence |
|-------------------------------------------------|-----------|----------------|
| `crates/base60-cli/tests/tui.rs`                | 1         | j*5 + m + a + q (via helper) |
| `crates/base60-cli/tests/persist.rs`            | 3         | j*5 + m + a + q (via helper, x3) |
| **Total**                                       | **4**     |                |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Widen `pub(crate)` to `pub` with `#[doc(hidden)]` on `run_with_terminal` + `cli::TimeScale`**
- **Found during:** Task 1 compile (first `cargo check`)
- **Issue:** `pub use crate::tui::run_with_terminal` at `src/lib.rs` emitted `error[E0364]: run_with_terminal is only public within the crate, and cannot be re-exported outside`. Similar `E0365` for `TimeScale`. The plan anticipated this at line 452 ("rustc may require `cli::TimeScale` itself to be at least `pub(crate)` for the `pub use` to succeed") but the plan's stated post-refactor target kept `pub(crate) fn run_with_terminal` — that visibility is insufficient for a cross-module `pub use` re-export even inside the same crate.
- **Fix:** Widened both to `pub` with `#[doc(hidden)]`. Prefix `__` + `#[doc(hidden)]` + explicit doc comments on both items ("NOT part of the public API ... may change or disappear in any release") preserve the stability contract. rustdoc gate green; doc build suppresses the hidden items from user-facing API docs.
- **Files modified:** `crates/base60-cli/src/tui.rs`, `crates/base60-cli/src/cli.rs`
- **Commit:** `f479058`

**2. [Rule 3 - Blocking] Add `B::Error: std::error::Error + Send + Sync + 'static` bound on `run_with_terminal<B: Backend, F>`**
- **Found during:** Task 1 compile (second `cargo check`)
- **Issue:** `terminal.draw(...)?` returns `Result<_, B::Error>`; `?` into `anyhow::Result<()>` requires `From<B::Error> for anyhow::Error`, which rustc derives from `std::error::Error + Send + Sync + 'static`. Without the bound, the generic form failed to compile with `error[E0277]: ?couldn't convert the error: <B as Backend>::Error: Send is not satisfied` + the same for `Sync` + `'static`. The plan's interface section did not state this bound.
- **Fix:** Added `B::Error: std::error::Error + Send + Sync + 'static` to the where-clause. All concrete backends (`CrosstermBackend<Stdout>` for production, `TestBackend` for tests) satisfy this trivially.
- **Files modified:** `crates/base60-cli/src/tui.rs`
- **Commit:** `f479058`

**3. [Rule 1 - Clippy] `#[allow(clippy::too_many_arguments)]` on `run_with_terminal`**
- **Found during:** Task 1 clippy pass (post-compile)
- **Issue:** `clippy::too_many_arguments` fires at 8 parameters (limit: 7). `fn run` had 6; adding `&mut Terminal<B>` + `next_event: F` pushes to 8. Splitting into a struct/tuple would just move the 8-tuple one indirection deeper without improving call-site readability.
- **Fix:** Local `#[allow(clippy::too_many_arguments)]` with an explanatory comment pinning the justification. This is consistent with the workspace lint posture (pedantic + nursery enabled, with documented exceptions).
- **Files modified:** `crates/base60-cli/src/tui.rs`
- **Commit:** `f479058`

**4. [Rule 1 - Clippy] `const fn key(c: char) -> Event` in `tests/common/mod.rs`**
- **Found during:** Task 2 clippy pass
- **Issue:** `clippy::missing_const_for_fn` fires on the `key` helper because all of `KeyEvent { ... }` construction is `const`-compatible.
- **Fix:** Changed `fn key` to `const fn key`. No behavior change; clippy now green.
- **Files modified:** `crates/base60-cli/tests/common/mod.rs`
- **Commit:** `30b2d46`

### Task TDD Collapse (inherits the Plan 04-03 decision)

Same rationale as 04-03-SUMMARY: each task landed as a single atomic green commit rather than a RED/GREEN split. Task 1 is a mechanical refactor with no behavior change (the integration tests in Tasks 3+4 are the first artifact that would observe the seam's existence — a pre-refactor RED on them would not compile, not fail in a useful sense). Tasks 2-4 are test additions against already-correct production code; the RED commit would require temporarily breaking `persist::save` or `tui::run`, which violates Phase 4 D-17 atomic-green granularity.

---

**Total deviations:** 4 auto-fixes (3 Rule 3 blocking compile/link, 1 Rule 1 clippy)
**Impact on plan:** Minor — all deviations are documented in the plan's "Part B" / "rustc may require ..." paragraphs as anticipated edge cases. The plan's acceptance-criteria grep `pub(crate) fn run_with_terminal` is formally violated in favour of `pub fn run_with_terminal`, but the stability contract is preserved via `#[doc(hidden)]` + `__` prefix + module-level doc warning. No user-visible API widening (rustdoc output unchanged).

## Threat Flags

None introduced beyond the threat model's `T-04-04-02` (Elevation of Privilege — `__test_hooks` leaking into consumers). Disposition is still `mitigate`: `#[doc(hidden)]` + `__` prefix + doc comments all signal no-stability-guarantee. Threat flags T-04-04-01 (concurrent env mutation), T-04-04-03 (event-source exhaustion), T-04-04-04 (state file info disclosure) all mitigated as planned.

## Verification Evidence

Command outputs captured at plan completion:

- `cargo test --workspace --all-targets --locked` -> all green:
  - base60 lib: 139 passed
  - cli.rs: 16 passed
  - fixtures.rs: 4 passed
  - persist.rs: 3 passed (NEW)
  - reader.rs: 3 passed
  - roundtrip.rs: 1 passed (140-cell matrix)
  - tui.rs: 1 passed (NEW)
  - base60-core lib: 41 passed
  - xtask env_discipline: 1 passed
  - xtask spawn_discipline: 1 passed
- `cargo test --workspace --all-targets --locked -- --test-threads=8` -> all same results green (no serial-test race).
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -> clean.
- `cargo fmt --all --check` -> clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` -> clean.
- `grep -rn 'env::set_var\|env::remove_var' crates/base60-cli/tests/` -> only lines inside `#[serial(env)]`-annotated tests, each wrapped in `unsafe { ... }` with a `// SAFETY:` comment above.
- `grep -rn 'Command::cargo_bin' crates/base60-cli/tests/ | grep -v 'common/'` -> zero hits.
- `grep -rn '^\[dependencies\]$' -A 5 crates/base60-core/Cargo.toml` -> empty section (zero-dep invariant preserved).
- `grep -n 'pub fn run_with_terminal' crates/base60-cli/src/tui.rs` -> 1 hit at line 96.
- `grep -n 'fn run(' crates/base60-cli/src/tui.rs` -> 1 hit at line 56 (production entry point preserved).
- `grep -n 'ratatui::run(|terminal|' crates/base60-cli/src/tui.rs` -> 1 hit at line 64 (only inside `fn run`).
- `grep -n 'crossterm::event::read().map(Some)' crates/base60-cli/src/tui.rs` -> 1 hit at line 73 (only the production closure).
- `grep -c '#\[doc(hidden)\]' crates/base60-cli/src/lib.rs` -> 3 (TimeScale alias + __test_hooks module + re-exports).
- `grep -c '#\[serial(env)\]' crates/base60-cli/tests/persist.rs` -> 5 (3 tests + 2 doc-comment references).
- `grep -c '#\[serial(env)\]' crates/base60-cli/tests/tui.rs` -> 4 (1 test + 3 doc/comment references).
- `grep -c 'unsafe { std::env::' crates/base60-cli/tests/persist.rs` -> 13 (three tests x 3-5 env mutations each).
- `grep -c '// SAFETY:' crates/base60-cli/tests/persist.rs` -> 15 (each `unsafe { std::env:: }` preceded by a SAFETY comment).
- `cargo run -p base60 --locked -- --color=never --length=16 README.md` -> emits `# bytes=0x10` trailer, proves production non-TUI path unchanged.

## Manual Smoke Check

Part C of Task 1 requested `cargo run -p base60 --locked -- -i README.md` to verify TUI production path visually. The autonomous executor does not have an interactive TTY, so manual verification via real TUI render was not possible in this execution. Equivalence argued by:

1. `fn run` body is 13 lines of trivial delegation to `run_with_terminal` — no branches, no new conditionals.
2. The production event-source closure `|| crossterm::event::read().map(Some)` is a 1:1 preservation of the original `event::read()?` call; the `.map(Some)` lift is mechanically required by the seam's `F: FnMut() -> io::Result<Option<Event>>` bound.
3. `run_with_terminal`'s event-source-exhausted branch (`Ok(None)`) is unreachable on the production path: `crossterm::event::read()` blocks until it returns `Ok(Event)` or `Err`, never `Ok(None)`, so the `.map(Some)` always lifts to `Ok(Some(_))`.
4. Workspace gate (tests + clippy + fmt + doc) all green.
5. Non-TUI production path smoke-checked with real input: `cargo run -p base60 -- --color=never --length=16 README.md` produces the expected dump including the `# bytes=0x10` trailer.

**Recommendation to reviewer:** confirm TUI production path on a real terminal once before merging, e.g. `cargo run -p base60 -- -i README.md` then verify `j` moves cursor, `q` quits cleanly.

## Self-Check: PASSED

Files:

- `crates/base60-cli/src/tui.rs` — MODIFIED (seam extracted at line 96; `fn run` at line 56 delegates).
- `crates/base60-cli/src/lib.rs` — MODIFIED (`__TuiTimeScale` alias + `__test_hooks` module; both `#[doc(hidden)]`).
- `crates/base60-cli/src/cli.rs` — MODIFIED (`TimeScale` visibility widened to `pub`).
- `crates/base60-cli/tests/common/mod.rs` — MODIFIED (`drive_tui_to_quit_with_fixture` + `const fn key`).
- `crates/base60-cli/tests/tui.rs` — CREATED (68 lines, 1 `#[test]`).
- `crates/base60-cli/tests/persist.rs` — CREATED (130 lines, 3 `#[test]`).
- `.planning/phases/04-tighten-parse-run-close-coverage-gaps/04-04-SUMMARY.md` — this file.

Commits:

- `f479058` refactor(04-04): extract run_with_terminal seam + expose via __test_hooks
- `30b2d46` test(04-04): add drive_tui_to_quit_with_fixture helper to tests/common
- `864b172` test(04-04): add TestBackend TUI integration test asserting state file
- `2aeb79f` test(04-04): XDG -> HOME -> None state-dir ladder tests [TEST-05]

All hashes present in `git log --oneline` starting at the 04-03 base commit `33e360c`.

## Next Phase Readiness

- **Phase 04 TEST-05 closure complete.** All three sub-targets covered: reader mmap/stdin/file-open-error (04-03), TUI exit-with-save (04-04), persist env ladder (04-04). `search::Pattern` property tests deferred to Phase 5 fuzz as documented in CONTEXT.md D-15.
- **Pattern library for Phase 5.** `tests/common::drive_tui_to_quit_with_fixture` + `TestBackend` + `__test_hooks` seam shape is reusable for any future TUI integration test (search, semantic jumps, bookmark cross-run persistence). Phase 5's fuzz scaffolding can follow the same `#[doc(hidden)] pub` + `__` prefix pattern for its `#[cfg(fuzzing)]` hatches.
- **Zero blockers.**

---
*Phase: 04-tighten-parse-run-close-coverage-gaps*
*Plan: 04*
*Completed: 2026-04-24*
