# gar improvement program design

Date: 2026-07-21

## Goal

Address every item P1-P16 in `research/gar-improvements/REPORT.md` on one
branch while preserving gar's existing output contracts, zero-runtime-
dependency core, and roundtrip invariant.

Work is complete when every applicable item has an implementation or, for an
already-satisfied finding, fresh evidence; all behavior changes have automated
tests; and the full project verification suite passes without skipped checks.

## Verified starting point

The report is directionally useful but contains stale details that affect the
implementation:

- P1 remains valid for JSON and HTML chunk emission. `dump_all` and
  `dump_reader` already share `write_line`, so no second dump formatter will be
  introduced.
- P3 remains valid, although the current line has roughly 37 spans rather than
  the reported 57. `draw` still allocates and formats every visible row on
  every frame.
- P8 is already covered by CLI integration tests, including real child-process
  dump-to-decode roundtrips. It needs verification and focused strengthening,
  not a duplicate test harness.
- The declared MSRV is Rust 1.95, while the CI matrix currently starts at
  1.96.0. P14 will align the toolchain file, CI, and developer documentation on
  1.95.

The untracked `research/` tree is input owned by the user and is not part of
the implementation commits.

## Delivery structure

All work stays on one GitButler branch and is divided into reviewable commits:

1. rendering and parsing internals (P1-P5);
2. property, integration, and fuzz coverage (P6-P8);
3. binary diff command (P9);
4. analyze filters (P10);
5. TUI navigation and comparison (P11);
6. public core formatting API (P12);
7. contributor tooling and CI (P13-P16);
8. documentation alignment and final verification fixes.

This ordering puts shared primitives and tests before their consumers and
keeps unrelated failures attributable to one small change set.

## P1-P5: rendering, coupling, and allocation

### Shared structured-output writers

`format.rs` will have one JSON-chunk writer and one HTML-chunk writer. Buffered
slice emitters and streaming reader emitters will retain their existing input
loops, length accounting, prologue/trailer handling, and `BrokenPipe`
semantics, but both paths will call the same per-chunk writer. This removes the
duplicated schemas without adding a generic callback framework.

The dump paths will continue to share `dump::write_line`; their distinct slice
and reader traversal is small and encodes different length accounting.

### Shared HTML grammar tokens

Emitter and decoder will import the same crate-private constants for digit
classes, span prefix/suffix, separator markup, and length trailer markers.
The decoder remains tolerant of irrelevant surrounding HTML, but changes to
gar's emitted tag names can no longer silently update only one side.

### Bounded TUI row cache

`ViewState` will own a cache for the current visible row window. Cached rows
contain the stable offset, digit, ASCII, and lens spans. Cache keys include the
row range, lens mode, base offset, and viewport geometry. The cache is rebuilt
when scrolling changes the visible window, the lens changes, the terminal
resizes, or split-view geometry changes.

Cursor movement must not rebuild all rows. Only the previously selected and
newly selected cached rows are updated with cursor styling. The cache is
bounded to visible rows, avoiding memory growth proportional to file size.
Tests will use an internal rebuild counter or equivalent observable seam to
prove that two unchanged frames reuse rows and that invalidation occurs for
each relevant state change.

### Single-pass search parsing

Auto-detected hex will be scanned once into its output buffer. Invalid hex or
an odd digit count falls back to the original UTF-8 bytes exactly as today;
explicit `hex:` input returns `InvalidHex`. Whitespace handling and quoted and
`str:` forms remain unchanged. No intermediate whitespace-stripped `String`
is allocated.

### Release profile

The ignored `[profile.release]` section will be removed from
`crates/gar-cli/Cargo.toml`; the workspace root remains the sole profile
definition.

## P6-P8: test and fuzz coverage

`proptest` will be a dev dependency only, preserving gar-core's zero runtime
dependencies. Properties will cover:

- every generated `u64` recomposes from `u64_to_base60` to the same value;
- every generated `u64` survives `encode_u64` then `decode_u64`;
- generated values always produce eleven base-60 digits below 60 and an
  eleven-character URL encoding.

The fuzz workflow will execute `parse_run`, `pattern_from_str`, and
`decode_stream` as a matrix so each target has an independent time budget and
failure artifact. The existing real-process roundtrip suite will receive a
focused regression case only if it does not already cover short trailing
chunks and arbitrary binary bytes; otherwise P8 is closed by the existing
test plus fresh execution evidence.

## P9: binary diff command

The CLI will add:

```text
gar diff OLD NEW
```

It compares both files in eight-byte chunks and emits a base-60 side-by-side
view. Each logical row contains an absolute offset, a marker, the left digit
run and ASCII view, and the right digit run and ASCII view. Markers distinguish
equal, changed, left-only, and right-only chunks. Changed bytes receive ANSI
emphasis when color is active; `NO_COLOR`, `TERM=dumb`, and non-TTY output are
deterministic plain text.

All rows are shown. This mirrors the normal viewer and avoids adding an
unrequested context/filter language. Exit status is 0 for identical files, 1
for differences, and 2 for usage or I/O failure. Broken output pipes still
exit cleanly. Tests cover identical input, changed bytes, unequal lengths,
plain determinism, offsets, and exit codes.

Diff remains in gar-cli. No comparison concepts leak into gar-core.

## P10: analyze range and pattern filters

`gar analyze` gains the following options:

```text
--skip BYTES
--length BYTES
--pattern PATTERN
```

`--skip` and `--length` select the analysis slice using the same saturating
range rules as the viewer. Reported region and match offsets remain absolute
file offsets. `--pattern` uses the TUI search grammar (`hex:`, `str:`, quoted,
or auto-detected) and adds a match count plus match offsets to the report; it
does not silently discard histogram or entropy data. This interpretation
provides search within analysis without changing the meaning of existing
summary fields. The summary lists the first 20 offsets and reports how many
additional matches were omitted, keeping output bounded on repetitive input.

Empty patterns are errors. A range beyond EOF analyzes an empty slice and
reports zero matches. Unit tests cover range selection and offset rebasing;
CLI tests cover option parsing, pattern forms, and combined filters.

## P11: TUI navigation and comparison

### Horizontal viewport

`<` and `>` scroll the rendered body horizontally by four terminal columns;
Home/End keep their existing byte-cursor meaning. Horizontal position is
clamped to rendered content width and reset only when explicitly scrolled back
or when the lens change makes the old position invalid.

### Position status

The normal status line adds total file bytes and the current cursor percentage
with defined empty-file behavior. Existing line range, byte range, cursor,
lens, and modal messages remain.

### Pinned comparison pane

Pressing `v` pins the current row and viewport as a read-only comparison pane.
The active pane continues to navigate normally while the pinned pane remains
fixed, allowing two regions of one file to be compared. Pressing `v` again
closes it; pressing `V` replaces the pin with the current row. Both panes use
the same lens and horizontal offset. The body is split into an upper active
pane and lower pinned pane, each with its own border and absolute row label.
Small terminal sizes fall back to the single pane with a transient status
message rather than panicking or creating zero-sized layouts.

Tests cover key dispatch, clamping, pin/replace/close transitions, split
geometry, cache invalidation, and status contents through ratatui's
`TestBackend`.

## P12: core formatting API

gar-core will export:

```rust
pub fn format_chunk(value: u64) -> String
```

It returns the canonical 32-byte textual digit run
`00:00:00:00:00:00:00:00:00:00:00`, using the existing precomputed digit-pair
table. The function has no external dependencies, is documented with an
example, and is re-exported at the crate root. Tests cover zero, 5025, and
`u64::MAX`; a property asserts the output has eleven parseable pairs matching
`u64_to_base60`.

## P13-P16: contributor workflow and CI

`CONTRIBUTING.md` will describe prerequisites, the zero-dependency invariant,
test-first workflow, required commands, output compatibility rules, and pull
request expectations. It will point to `CLAUDE.md` for architecture rather
than duplicate it.

`rust-toolchain.toml` will pin channel 1.95 with `rustfmt` and `clippy`.
The CI MSRV matrix entry and `CLAUDE.md` matrix description will also use
1.95. Stable and beta coverage remains.

CI will add a dedicated cargo-deny job using the maintained cargo-deny GitHub
Action and the existing `deny.toml`. It will run independently of compile jobs.

A minimal `justfile` will expose `check`, `test`, `lint`, `doc`, `deny`,
`fuzz`, and `verify`. Recipes will delegate to the same explicit Cargo commands
used in CI; they will not duplicate logic in shell scripts or replace `xtask`.
`verify` will fail immediately if any required command fails.

## Compatibility and error handling

- Existing view, JSON, HTML, decode, and analyze output remains compatible;
  analyze only appends fields when a pattern is requested.
- gar-core retains zero runtime dependencies.
- All range arithmetic is saturating and offsets are absolute.
- I/O errors include the relevant path. Invalid patterns are user-facing
  errors. Diff uses its documented three-way exit status.
- No panic, `unwrap`, or `expect` is added to production paths.
- No unrelated cleanup or formatting churn is included.

## Verification

Every behavioral item follows red-green-refactor. Configuration-only changes
are verified by the tool they configure. Before completion, run fresh:

```text
cargo check --workspace --all-targets --locked --message-format=json
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo deny check
```

Also run targeted real CLI scenarios for dump/decode, diff exit codes, analyze
filters, and TUI rendering tests. Fuzz target compilation and workflow syntax
are verified locally; bounded fuzz campaigns are run when nightly cargo-fuzz
is available. Any unavailable tool or skipped check is reported explicitly.

Completion is assessed against P1-P16 one item at a time, not inferred only
from a green aggregate test command.
