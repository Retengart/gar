# gar Internals and Tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove duplicated formatting and avoidable allocations, cache TUI rows, and close property/fuzz coverage gaps P1-P8.

**Architecture:** Keep input traversal separate for slices and streams, but route both through shared per-chunk writers. Add a bounded cached `Paragraph<'static>` for the active TUI viewport and draw cursor emphasis directly into the rendered buffer. Preserve all public output bytes and parsing semantics.

**Tech Stack:** Rust 2024, ratatui 0.30, proptest 1.11, cargo-fuzz, GitHub Actions, GitButler CLI.

## Global Constraints

- All work stays on GitButler branch `fix/all-gar-improvements`.
- `gar-core` keeps zero runtime dependencies.
- JSON, HTML, decode, color-environment, and roundtrip contracts stay compatible.
- Production code adds no `unwrap`, `expect`, panic, speculative abstraction, or unrelated cleanup.
- Rust checks use `cargo check --message-format=json`.

---

### Task 1: Share structured chunk writers and HTML protocol tokens (P1, P2)

**Files:**
- Create: `crates/gar-cli/src/html.rs`
- Modify: `crates/gar-cli/src/lib.rs`
- Modify: `crates/gar-cli/src/format.rs`
- Modify: `crates/gar-cli/src/decode.rs`

**Interfaces:**
- Produces: `html::{DIGIT_CLASSES, SPAN_OPEN, SPAN_CLOSE, SEPARATOR, LENGTH_PREFIX, LENGTH_SUFFIX, digit_class}`.
- Produces: private `write_json_chunk<W: Write>` and `write_html_chunk<W: Write>` in `format.rs`.
- Preserves: existing `emit_json*`, `emit_html*`, and `decode_stream` signatures.

- [ ] **Step 1: Add failing tests for the missing shared interfaces**

Add tests that call the not-yet-defined chunk writers and shared HTML tokens:

```rust
#[test]
fn json_chunk_writer_emits_one_schema_row() {
    let mut out = Vec::new();
    write_json_chunk(&mut out, 8, &[0, 0, 0, 0, 0, 0, 0x13, 0xa1], None).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"offset\":8,\"bytes\":[0,0,0,0,0,0,19,161],\"digits\":[0,0,0,0,0,0,0,0,1,23,45],\"ascii\":\"........\"}\n"
    );
}

#[test]
fn html_protocol_digit_class_is_shared() {
    assert_eq!(crate::html::digit_class(0), "d-zero");
    assert_eq!(crate::html::digit_class(59), "d-high");
}
```

- [ ] **Step 2: Verify RED**

Run: `cargo test -p gar format::tests::json_chunk_writer_emits_one_schema_row --locked`

Expected: compile failure because `write_json_chunk` and `crate::html` do not exist.

- [ ] **Step 3: Implement the shared HTML protocol and per-chunk writers**

Create `html.rs` with exact crate-private tokens:

```rust
pub(crate) const SPAN_OPEN: &str = "<span class=\"";
pub(crate) const SPAN_CLOSE: &str = "</span>";
pub(crate) const SEPARATOR: &str = "<span class=\"sep\">:</span>";
pub(crate) const LENGTH_PREFIX: &str = "<!-- bytes=0x";
pub(crate) const LENGTH_SUFFIX: &str = " -->";
pub(crate) const DIGIT_CLASSES: [&str; 4] = ["d-zero", "d-low", "d-mid", "d-high"];

pub(crate) const fn digit_class(digit: u8) -> &'static str {
    match digit {
        0 => "d-zero",
        1..20 => "d-low",
        20..40 => "d-mid",
        _ => "d-high",
    }
}
```

Move one JSON row and one HTML row body into helpers taking writer, offset,
chunk, and optional lens. Replace both buffered and streaming duplicated
bodies with calls to those helpers. Make `decode_from_html` consume the shared
prefix, suffix, separator, digit-class names, and trailer markers.

- [ ] **Step 4: Verify GREEN and parity**

Run:

```text
cargo test -p gar format::tests --locked
cargo test -p gar decode::tests --locked
cargo test -p gar --test roundtrip --locked
```

Expected: all tests pass; the 140-cell integration matrix remains byte-identical.

- [ ] **Step 5: Commit**

Run `but diff`, then commit only `html.rs`, `lib.rs`, `format.rs`, and
`decode.rs` to `fix/all-gar-improvements` with message
`refactor: share structured output writers`.

### Task 2: Parse auto-detected hex in one pass and remove dead profile (P4, P5)

**Files:**
- Modify: `crates/gar-cli/src/search.rs`
- Modify: `crates/gar-cli/Cargo.toml`

**Interfaces:**
- Produces: private `parse_auto_hex(&str) -> Option<Vec<u8>>`.
- Preserves: `Pattern::from_str` grammar and `ParseError` values.

- [ ] **Step 1: Add failing allocation-independent parser tests**

```rust
#[test]
fn auto_hex_parser_returns_none_without_partial_output() {
    assert_eq!(parse_auto_hex("de ad nope"), None);
}

#[test]
fn auto_hex_parser_decodes_whitespace_in_one_pass() {
    assert_eq!(parse_auto_hex("de ad be ef"), Some(vec![0xde, 0xad, 0xbe, 0xef]));
}
```

- [ ] **Step 2: Verify RED**

Run: `cargo test -p gar search::tests::auto_hex_parser --locked`

Expected: compile failure because `parse_auto_hex` does not exist.

- [ ] **Step 3: Implement a single scan**

```rust
fn parse_auto_hex(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() / 2);
    let mut high = None;
    for byte in input.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        let digit = hex_digit(byte)?;
        if let Some(hi) = high.take() {
            out.push(hi * 16 + digit);
        } else {
            high = Some(digit);
        }
    }
    (!out.is_empty() && high.is_none()).then_some(out)
}
```

Use it only for auto-detection. Keep explicit `hex:` parsing error-producing.
Delete `looks_like_hex`. Remove only the ignored `[profile.release]` table from
`crates/gar-cli/Cargo.toml`.

- [ ] **Step 4: Verify GREEN and warning removal**

Run:

```text
cargo test -p gar search::tests --locked
cargo check --workspace --all-targets --locked --message-format=json
```

Expected: search tests pass and Cargo no longer prints the non-root profile warning.

- [ ] **Step 5: Commit**

Commit the two files with `but` as `perf: parse search hex in one pass`.

### Task 3: Cache visible TUI rows (P3)

**Files:**
- Modify: `crates/gar-cli/src/dump.rs`
- Modify: `crates/gar-cli/src/tui.rs`
- Test: `crates/gar-cli/src/tui.rs`

**Interfaces:**
- Produces: `RowCache { key: Option<RowCacheKey>, paragraph: Paragraph<'static>, rebuilds: usize }`.
- Produces: `RowCacheKey` containing visible start/end, lens mode, base offset, body width/height, and horizontal offset.
- Consumes: `dump::styled_line(..., cursor_in_line = None)` for stable cached lines.

- [ ] **Step 1: Add failing cache-reuse and invalidation tests**

Drive two `TestBackend` draws without state changes and assert the cache rebuild
count remains one. Then change lens and viewport and assert it increments:

```rust
#[test]
fn unchanged_frames_reuse_cached_rows() {
    let data = vec![0_u8; 80];
    let mut state = ViewState::new(&data, LensMode::None, TimeScale::Gar, false);
    draw_test_frame(&mut state, &data, 80, 24);
    draw_test_frame(&mut state, &data, 80, 24);
    assert_eq!(state.row_cache.rebuilds, 1);
}
```

- [ ] **Step 2: Verify RED**

Run: `cargo test -p gar tui::tests::unchanged_frames_reuse_cached_rows --locked`

Expected: compile failure because `row_cache` does not exist.

- [ ] **Step 3: Implement the bounded cached paragraph**

Build owned `Line<'static>` values only when `RowCacheKey` changes, wrap them
in `Paragraph<'static>`, and render the paragraph by reference:

```rust
frame.render_widget(&self.row_cache.paragraph, body_area);
```

After rendering, calculate the active cursor's screen cell in the ASCII column
and apply `Modifier::REVERSED` through `frame.buffer_mut()` so cursor movement
does not mutate or clone the cached text. Do not cache rows outside the visible
window. Keep `rebuilds` under `#[cfg(test)]` if production code does not need it.

- [ ] **Step 4: Verify GREEN and rendering equivalence**

Run:

```text
cargo test -p gar tui::tests --locked
cargo test -p gar dump::tests --locked
cargo test -p gar --test tui --locked
```

Expected: cache tests and existing cursor/render tests pass.

- [ ] **Step 5: Commit**

Commit `dump.rs` and `tui.rs` with `but` as `perf: cache visible tui rows`.

### Task 4: Add property tests and execute every fuzz target (P6-P8)

**Files:**
- Modify: `crates/gar-core/Cargo.toml`
- Modify: `crates/gar-core/src/convert.rs`
- Modify: `crates/gar-core/src/url.rs`
- Modify: `.github/workflows/fuzz.yml`
- Verify: `crates/gar-cli/tests/roundtrip.rs`

**Interfaces:**
- Adds dev-only `proptest = "1.11"`.
- Preserves zero runtime dependencies.

- [ ] **Step 1: Add the property tests before the dependency**

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn arbitrary_u64_roundtrips_through_base60(value in any::<u64>()) {
        let digits = u64_to_base60(value);
        prop_assert!(digits.iter().all(|digit| *digit < 60));
        prop_assert_eq!(recompose(&digits), value);
    }
}
```

In `url.rs`, add the analogous `encode_u64`/`decode_u64` roundtrip and
eleven-character property.

- [ ] **Step 2: Verify RED**

Run: `cargo test -p gar-core arbitrary_u64 --locked`

Expected: compile failure because `proptest` is not declared.

- [ ] **Step 3: Add the dev dependency and fuzz matrix**

Add `proptest = "1.11"` under gar-core dev dependencies. Replace sequential
fuzz steps with:

```yaml
strategy:
  fail-fast: false
  matrix:
    target: [parse_run, pattern_from_str, decode_stream]
steps:
  - uses: actions/checkout@v4
  - uses: dtolnay/rust-toolchain@master
    with:
      toolchain: nightly
      components: cargo-fuzz
  - name: Fuzz ${{ matrix.target }}
    run: cargo +nightly fuzz run ${{ matrix.target }} -- -max_total_time=240
  - name: Upload fuzz artifacts
    if: failure()
    uses: actions/upload-artifact@v4
    with:
      name: fuzz-artifacts-${{ matrix.target }}
      path: fuzz/artifacts/${{ matrix.target }}/
      if-no-files-found: ignore
```

Name artifacts with `${{ matrix.target }}` to avoid collisions.

- [ ] **Step 4: Verify GREEN and P8 evidence**

Run:

```text
cargo test -p gar-core --locked
cargo test -p gar --test roundtrip --locked
cargo metadata --locked --format-version=1
```

Expected: properties pass; the existing 140-cell real-process roundtrip passes;
metadata shows no gar-core normal dependency.

- [ ] **Step 5: Commit**

Commit the property/fuzz files and resulting `Cargo.lock` with `but` as
`test: add property and complete fuzz coverage`.
