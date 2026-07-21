# gar Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add canonical core formatting, binary diff, analyze pattern search, and TUI comparison/navigation features P9-P12.

**Architecture:** Put pure formatting in gar-core, file comparison and summary search in focused gar-cli modules, and retain `tui.rs` as the integration point. Reuse existing chunk, palette, reader, and search primitives; do not create alternate parsing grammars.

**Tech Stack:** Rust 2024, clap 4.6 derive, ratatui 0.30, memchr 2.8, GitButler CLI.

## Global Constraints

- All work stays on branch `fix/all-gar-improvements`.
- Existing commands and stable output schemas remain compatible.
- Diff exit codes are 0 identical, 1 different, 2 usage/I/O error.
- Analyze match output lists at most 20 offsets and the omitted count.
- TUI keys are `<`, `>`, `v`, and `V` exactly as specified.

---

### Task 1: Export canonical `format_chunk` (P12)

**Files:**
- Modify: `crates/gar-core/src/convert.rs`
- Modify: `crates/gar-core/src/lib.rs`

**Interfaces:**
- Produces: `pub fn format_chunk(value: u64) -> String`.

- [ ] **Step 1: Add failing API tests and doctest**

```rust
#[test]
fn format_chunk_uses_eleven_zero_padded_pairs() {
    assert_eq!(format_chunk(5025), "00:00:00:00:00:00:00:00:01:23:45");
    assert_eq!(format_chunk(0).len(), 32);
}
```

- [ ] **Step 2: Verify RED**

Run: `cargo test -p gar-core format_chunk --locked`

Expected: compile failure because `format_chunk` does not exist.

- [ ] **Step 3: Implement from existing lookup tables**

Allocate `String::with_capacity(32)`, iterate `u64_to_base60(value)`, push `:`
between pairs, and push `DIGIT_PAIRS_STR[digit as usize]`. Re-export it from
`lib.rs` and document the exact 32-byte contract.

- [ ] **Step 4: Verify GREEN and docs**

Run: `cargo test -p gar-core --locked && cargo test -p gar-core --doc --locked`

- [ ] **Step 5: Commit**

Commit with `but` as `feat(core): format canonical base60 chunks`.

### Task 2: Add `gar diff OLD NEW` (P9)

**Files:**
- Create: `crates/gar-cli/src/diff.rs`
- Modify: `crates/gar-cli/src/cli.rs`
- Modify: `crates/gar-cli/src/lib.rs`
- Modify: `crates/gar-cli/src/main.rs`
- Modify: `crates/gar-cli/tests/cli.rs`

**Interfaces:**
- Produces: `DiffArgs { old: PathBuf, new: PathBuf }` and `Command::Diff(DiffArgs)`.
- Produces: `diff::write_diff<W: Write>(old: &[u8], new: &[u8], base_offset: u64, w: W, palette: &Palette) -> io::Result<bool>` where bool means files differ.
- Changes: `gar::run() -> anyhow::Result<std::process::ExitCode>`.

- [ ] **Step 1: Add failing CLI integration tests**

Use two `NamedTempFile` inputs and assert:

```rust
gar_cmd().args(["diff", old, same]).assert().code(0);
gar_cmd().args(["diff", old, changed]).assert().code(1)
    .stdout(predicate::str::contains("!"));
gar_cmd().args(["diff", old, longer]).assert().code(1)
    .stdout(predicate::str::contains(">"));
```

Also assert a missing file exits 2 and includes its path on stderr.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p gar --test cli diff_ --locked`

Expected: clap rejects the missing `diff` subcommand.

- [ ] **Step 3: Implement comparison and exit propagation**

Iterate `old.chunks(8)` and `new.chunks(8)` through the maximum chunk count.
Use markers `=` equal, `!` changed, `<` left-only, `>` right-only. Format each
present side with `gar_core::format_chunk(u64::from_be_bytes(padded))` and an
eight-character ASCII column. Use existing palette selection and reverse or
bright styling only for changed bytes in ANSI mode.

Make `main` return `ExitCode` explicitly; map normal commands to SUCCESS,
different diff to 1, and top-level errors to stderr plus 2. Handle BrokenPipe
as SUCCESS before selecting the diff status.

- [ ] **Step 4: Verify GREEN and compatibility**

Run:

```text
cargo test -p gar --test cli --locked
cargo test -p gar --test roundtrip --locked
cargo test -p gar cli::tests --locked
```

- [ ] **Step 5: Commit**

Commit the diff module, CLI wiring, main exit handling, and tests with `but` as
`feat: compare binary files in base60`.

### Task 3: Add analyze pattern reporting (P10)

**Files:**
- Modify: `crates/gar-cli/src/cli.rs`
- Modify: `crates/gar-cli/src/analyze.rs`
- Modify: `crates/gar-cli/src/lib.rs`
- Modify: `crates/gar-cli/tests/cli.rs`
- Modify: `README.md`

**Interfaces:**
- Adds: `AnalyzeArgs.pattern: Option<Pattern>` or `Option<String>` parsed once in `run_analyze`.
- Produces: `write_matches<W: Write>(matches: &[usize], base_offset: u64, w: &mut W) -> io::Result<()>`.
- Reuses: existing `AnalyzeArgs.skip` and `AnalyzeArgs.length`; no duplicate range fields.

- [ ] **Step 1: Add failing CLI tests**

Assert `gar analyze --skip 2 --length 8 --pattern hex:4142 FILE` prints
`matches       1` and the absolute offset, an invalid explicit hex pattern
fails, and more than 20 matches prints `... N more`.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p gar --test cli analyze_pattern --locked`

Expected: clap rejects `--pattern`.

- [ ] **Step 3: Implement bounded match summary**

Add `--pattern PATTERN` to `AnalyzeArgs`. Parse through `Pattern::from_str`,
call `search::find_all` on the already range-clamped bytes, then add `args.skip`
to displayed offsets. Print at most `matches.iter().take(20)` and report
`matches.len() - 20` when nonzero. Existing entropy/histogram output remains
unchanged and is always printed.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test -p gar analyze --locked && cargo test -p gar --test cli --locked`

- [ ] **Step 5: Commit**

Commit with `but` as `feat: search patterns in analyze output`.

### Task 4: Add horizontal scrolling, position ratio, and pinned split view (P11)

**Files:**
- Modify: `crates/gar-cli/src/tui.rs`
- Modify: `crates/gar-cli/tests/tui.rs`
- Modify: `README.md`

**Interfaces:**
- Adds: `ViewState.horizontal: u16`.
- Adds: `ViewState.pinned: Option<PinnedView>` containing row and scroll origin.
- Uses: cached Paragraph `.scroll((0, horizontal))` and vertical `Layout` split.

- [ ] **Step 1: Add failing state and rendering tests**

Tests must prove:

```rust
// > advances by four, < saturates at zero
// v creates a pin, a second v removes it, V replaces its row
// status contains total byte count and a percentage
// a terminal too short for two bordered panes renders one pane and a message
```

Add a `TestBackend` assertion that upper and lower pane titles contain their
distinct absolute offsets.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p gar tui::tests::horizontal --locked`

Expected: tests fail because key bindings and state do not exist.

- [ ] **Step 3: Implement geometry and key behavior**

Handle `KeyCode::Char('>')` and `('<')` with four-column saturating movement.
Handle `v` as toggle and `V` as set/replace. In draw, use one body when no pin;
otherwise split the body vertically with equal `Constraint::Ratio(1, 2)` panes.
If total body height is below six rows, render only the active pane and set the
transient message. Apply the same lens and horizontal offset to both cached
paragraphs, with independent visible row origins.

Append status fields in the form `size N   position P%`, using 0% for empty
input and `(cursor + 1) * 100 / data_len` with saturating arithmetic otherwise.

- [ ] **Step 4: Verify GREEN and cache behavior**

Run:

```text
cargo test -p gar tui::tests --locked
cargo test -p gar --test tui --locked
cargo test -p gar --test persist --locked
```

- [ ] **Step 5: Commit**

Commit with `but` as `feat: compare regions in tui`.

