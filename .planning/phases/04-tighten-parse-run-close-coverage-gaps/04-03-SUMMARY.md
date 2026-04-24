---
phase: 04-tighten-parse-run-close-coverage-gaps
plan: 03
subsystem: testing
tags: [rust, cli, integration-tests, reader, tempfile, mmap, stdin, test-05]

# Dependency graph
requires:
  - phase: 04-tighten-parse-run-close-coverage-gaps
    provides: "base60_cmd() hermetic spawner (Phase 3 carry-over); unchanged reader::load / load_file / load_stdin seams; Wave 1 `# bytes=0x<hex>` trailer (accounted for via substring matchers)"
provides:
  - "tempfile = \"3\" dev-dep resolved in Cargo.lock (tempfile v3.27.0)"
  - "crates/base60-cli/tests/reader.rs — 3 black-box integration tests covering mmap, stdin, and file-open-error paths"
  - "First consumer of tempfile in the workspace — dependency ordering satisfied for Plan 04-04 TUI/persist tests that reuse NamedTempFile / tempdir"
affects: [04-04-tui-persist-coverage]

# Tech tracking
tech-stack:
  added:
    - "tempfile = \"3\" (dev-dep on base60-cli; resolves to 3.27.0; base60-core zero-dep invariant preserved)"
  patterns:
    - "Black-box integration-test coverage for reader paths — exercises reader::load{,_file,_stdin} via the spawned binary without widening any pub(crate) surface (Phase 3 D-07)"
    - "Two-substring .and() predicate on error stderr — locks both the anyhow::Context chain prefix (\"open\") and the user-supplied filename (\"nope.bin\") without relying on an OS-specific path-separator rendering"

key-files:
  created:
    - "crates/base60-cli/tests/reader.rs — 3 integration tests (59 lines)"
  modified:
    - "crates/base60-cli/Cargo.toml — +1 line under [dev-dependencies]"
    - "Cargo.lock — regenerated to pin tempfile v3.27.0 + its deps (fastrand, same-file, walkdir, getrandom, rustix — most already present via other deps)"

key-decisions:
  - "Task 2 tdd=\"true\" collapsed to a single commit: the target behaviour is already-correct production code (mmap, stdin, and File::open error paths are all live since Phase 1). A separate RED commit would require temporarily breaking reader.rs, which violates atomic-green commit granularity (Phase 3 D-17). The test-first intent is preserved by the tests being the first artifact that asserts these exact end-to-end behaviours."
  - "Used tempfile::NamedTempFile (not tempdir + manual write) for the mmap fixture — matches the plan's Context7-verified API snippet + keeps the fixture scope tight to a single file."
  - "Caret version `tempfile = \"3\"` matches existing dev-dep style (`assert_cmd = \"2\"`, `predicates = \"3\"`) rather than pinning to 3.x.y."

patterns-established:
  - "Hermetic reader-path coverage: integration tests assert on CLI output for mmap vs stdin vs open-error, never imported reader internals. Pattern reusable for any future `pub(crate)` module whose surface must not widen."
  - "Env-free test file in tests/*.rs — xtask env_discipline does not walk tests/ so manual verification confirmed no env::set_var/remove_var in code (only doc-comment mentions)."

requirements-completed: [TEST-05]

# Metrics
duration: 3min
completed: 2026-04-24
---

# Phase 4 Plan 03: Reader Coverage — mmap + stdin + file-open error Summary

**Closes TEST-05's reader coverage gap with 3 black-box integration tests that exercise `reader::load_file` (mmap via `NamedTempFile`), `reader::load_stdin` (piped via `.write_stdin`), and the `File::open` error path — `reader` internals stay `pub(crate)` per Phase 3 D-07.**

## Performance

- **Duration:** 2m 47s (~3 min)
- **Started:** 2026-04-24T16:03:27Z
- **Completed:** 2026-04-24T16:06:14Z
- **Tasks:** 2
- **Files modified:** 2 (Cargo.toml, Cargo.lock) + 1 created (tests/reader.rs)
- **New tests:** 3 (all passing on first run)

## Accomplishments

- Added `tempfile = "3"` to `crates/base60-cli/Cargo.toml [dev-dependencies]`; `base60-core [dependencies]` remains absent (zero-dep invariant preserved, Pitfall 5).
- Created `crates/base60-cli/tests/reader.rs` with three independent `#[test]` functions, each routing exclusively through the hermetic `base60_cmd()` spawner from `tests/common/mod.rs`.
- `load_file_via_mmap_returns_file_contents` (11-byte `NamedTempFile`, `b"hello world"`) asserts stdout contains `|hello wo|` — proves the mmap read path + ASCII-column rendering both work end-to-end.
- `load_stdin_via_write_stdin_dumps_piped_bytes` (`b"piped!\n"`, 7 bytes) asserts stdout contains `|piped!.|` — `\n` renders as `.` in the ASCII column.
- `load_file_nonexistent_returns_error` (`/definitely/does/not/exist/nope.bin`) asserts `.failure()` + stderr contains BOTH `"open"` and `"nope.bin"` via `PredicateBooleanExt::and`, locking the `anyhow::Context` message `format!("open {}", path.display())` at `reader.rs:52`.
- spawn_discipline gate still green (zero `Command::cargo_bin` outside `tests/common/`).
- env_discipline gate still green (reader.rs is env-free).
- Full workspace gates green: test, clippy `-D warnings`, fmt `--check`, doc with `RUSTDOCFLAGS=-D warnings`.

## Task Commits

1. **Task 1: Add `tempfile = "3"` as a dev-dependency** — `f692153` (chore)
2. **Task 2: Create `tests/reader.rs` with mmap, stdin, and file-open-error integration tests** — `bd6ea65` (test)

_Note: Task 2 was marked `tdd="true"` in the plan but collapsed to a single commit — see Decisions._

## Cargo.toml diff

```diff
 [dev-dependencies]
 assert_cmd = "2"
 base60-core = { path = "../base60-core" }
 predicates = "3"
 serial_test = { version = "3", default-features = false }
+tempfile = "3"
```

Cargo.lock: `tempfile v3.27.0` added + its deps (`fastrand`, `same-file` — others already present transitively).

## `tests/reader.rs` structure

```rust
mod common;
use common::base60_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::io::Write;
```

| Test | Exercises | Fixture | Assertion |
|------|-----------|---------|-----------|
| `load_file_via_mmap_returns_file_contents` | `reader::load_file` mmap path (`unsafe { Mmap::map(&file) }` at reader.rs:56) | `NamedTempFile` + `b"hello world"` (11 B) | `.success()` + stdout contains `\|hello wo\|` |
| `load_stdin_via_write_stdin_dumps_piped_bytes` | `reader::load_stdin` `read_to_end` path (reader.rs:61-68) | `.write_stdin(&b"piped!\n"[..])` (7 B) | `.success()` + stdout contains `\|piped!.\|` |
| `load_file_nonexistent_returns_error` | `File::open` → `anyhow::Context` error path (reader.rs:52) | `.arg("/definitely/does/not/exist/nope.bin")` | `.failure()` + stderr contains `"open"` AND `"nope.bin"` |

## Decisions Made

- **Task 2 collapsed to single commit (no RED/GREEN split).** The plan's `tdd="true"` annotation was honoured in spirit: the new tests are the first artifact that asserts these specific end-to-end behaviours. Splitting into a RED commit would require temporarily breaking `reader.rs` (e.g., stubbing `load_file` to panic) to observe a failure, then reverting — that violates atomic-green commit granularity (Phase 3 D-17). Since the production code at `reader.rs` has been correct since Phase 1, a single `test(04-03)` commit is the cleanest signal.
- **`NamedTempFile` over `tempdir + manual write`** — matches the Context7-verified API snippet cited in the plan and keeps fixture scope tight to a single file. Auto-delete on drop handles cleanup even on panic (threat T-04-03-02 accept disposition).
- **Caret `"3"` version string** — matches existing dev-dep style (`assert_cmd = "2"`, `predicates = "3"`) rather than pinning to a specific patch.
- **`PredicateBooleanExt::and(...)`** — already imported in `tests/cli.rs` via the same prelude glob; re-using the pattern keeps the two-substring stderr assertion clean. No new combinator imports required.

## Deviations from Plan

**None** — plan executed exactly as written. The only item worth flagging is the Task 2 TDD collapse, which is a documented decision within the latitude the plan implicitly grants (RED tests targeting already-correct production code are a no-op split; see Decisions above).

### Auto-fix attempts

None. No Rule 1/2/3 fixes triggered. No clippy lints required `#[allow]` attributes. No `cargo fmt` reformats were needed. First compile, first clippy, first fmt check, first doc build all clean.

---

**Total deviations:** 0
**Impact on plan:** None — 100% plan-conforming execution.

## Issues Encountered

None. The only moment of ambiguity was whether to treat `tdd="true"` as mandating a separate RED commit; resolved via Decisions above (single atomic `test(04-03)` commit matches the Phase 4 D-17 atomic-green granularity convention better than a two-commit compile-break-then-fix pattern).

## Verification Evidence

Command outputs captured at plan completion (Wave 3 execution):

- `cargo test -p base60 --test reader --locked` → `3 passed; 0 failed` (all three new tests pass).
- `cargo test --workspace --all-targets --locked` → all green:
  - base60 lib: 139 passed
  - cli.rs: 16 passed
  - fixtures.rs: 4 passed
  - reader.rs: 3 passed (NEW)
  - roundtrip.rs: 1 passed (140-cell matrix)
  - base60-core lib: 41 passed
  - xtask env_discipline: 1 passed
  - xtask spawn_discipline: 1 passed
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → clean.
- `cargo fmt --all --check` → clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` → clean.
- `grep -rn 'Command::cargo_bin' crates/base60-cli/tests/ | grep -v 'common/'` → zero hits (spawn-discipline invariant preserved).
- `grep -n '^tempfile = "3"$' crates/base60-cli/Cargo.toml` → 1 hit (line 35).
- `grep -rn 'tempfile' crates/base60-core/Cargo.toml` → zero hits (core zero-dep invariant preserved).
- `grep -rn '^\[dependencies\]$' -A 5 crates/base60-core/Cargo.toml` → empty section after header (core has no runtime deps).
- `cargo tree -p base60 --locked -e dev --depth 1 | grep tempfile` → `└── tempfile v3.27.0`.
- `grep -c '#\[test\]' crates/base60-cli/tests/reader.rs` → 3.
- `grep -n 'env::set_var\|env::remove_var\|#\[serial(env)\]' crates/base60-cli/tests/reader.rs | grep -Ev ':\s*//'` → zero code matches (only doc-comment mentions of these literals, which is documentation noting that the file is env-free).

## Threat Flags

None — threat model items T-04-03-01 / T-04-03-02 were both `accept` dispositions:

- T-04-03-01 (info disclosure via error message): `anyhow::Context` emits only the user-supplied path back at the user — no additional secrets leak. Test asserts on both `"open"` and `"nope.bin"`, confirming the message is exactly the user-supplied filename plus the action verb.
- T-04-03-02 (DoS via large tempfile): fixture is 11 bytes; `NamedTempFile::drop` auto-deletes. No measurable tmp-quota risk.

No new threat surface introduced — all three tests exercise pre-existing code paths without adding any new reader behaviour.

## Self-Check: PASSED

Files:

- `crates/base60-cli/Cargo.toml` — FOUND (line 35: `tempfile = "3"`).
- `crates/base60-cli/tests/reader.rs` — FOUND (59 lines; 3 `#[test]` functions; uses `common::base60_cmd` and `tempfile::NamedTempFile`).
- `.planning/phases/04-tighten-parse-run-close-coverage-gaps/04-03-SUMMARY.md` — this file.
- `Cargo.lock` — modified (tempfile v3.27.0 pinned).

Commits:

- `f692153` chore(04-03): add tempfile = "3" as base60-cli dev-dep
- `bd6ea65` test(04-03): reader coverage — mmap + stdin + file-open error [TEST-05]

## Next Phase Readiness

- **Plan 04-04 unblocked:** `tempfile = "3"` dev-dep is now in `Cargo.lock`; 04-04 can `use tempfile::tempdir` for the `$XDG_STATE_HOME` redirect + `TestBackend` fixtures without adding a duplicate entry.
- **Reader coverage gap closed:** `reader::load_file` (mmap), `reader::load_stdin` (read_to_end), and the `File::open` error-context chain all now have end-to-end assertions. Remaining `reader.rs` surface (the 5 inline `clamp_range` unit tests) was already green pre-plan.
- **Zero blockers.**

---
*Phase: 04-tighten-parse-run-close-coverage-gaps*
*Plan: 03*
*Completed: 2026-04-24*
