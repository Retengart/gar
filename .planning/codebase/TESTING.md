# Testing Patterns

**Analysis Date:** 2026-04-23

## Test Framework

**Runner:** Built-in `libtest` via `#[test]` / `#[cfg(test)]`. No external
test crates (no `proptest`, no `criterion`, no `rstest`, no `mockall`, no
`insta`). Neither workspace nor member `Cargo.toml` defines a
`[dev-dependencies]` section.

**Assertion macros:** stdlib only — `assert!`, `assert_eq!`, `assert_ne!`,
with trailing context strings used to disambiguate loop failures:
```rust
// crates/base60-core/src/convert.rs:94
assert_eq!(recompose(&d), u128::from(n), "roundtrip failed for {n}");
```

**Doc tests:** Enabled and run in CI. Example in
`crates/base60-core/src/url.rs:12-18` exercises the encode/decode
round-trip. CI runs them separately from unit + integration tests:
```yaml
# .github/workflows/ci.yml:40
- name: Doc tests
  run: cargo test --workspace --doc --locked
```

## Run Commands

```bash
cargo test --workspace --all-targets --locked   # unit + integration
cargo test --workspace --doc --locked           # doc tests
cargo test -p base60-core                       # library crate only
cargo test -p base60 persist::tests             # module-scoped
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all --check
cargo doc --workspace --no-deps --locked        # RUSTDOCFLAGS=-D warnings
```

## Test File Organisation

**All tests are inline, co-located in the same file as the code under test,
inside a `#[cfg(test)] mod tests { ... }` block at the bottom.**

There are **no `tests/` integration directories** in either crate and no
`benches/` directories.

**Files with inline test modules** (count of `#[test]` per file):

| File                                                | Tests |
|-----------------------------------------------------|-------|
| `crates/base60-cli/src/tui.rs`                      | 45    |
| `crates/base60-cli/src/dump.rs`                     | 14    |
| `crates/base60-cli/src/search.rs`                   | 14    |
| `crates/base60-cli/src/format.rs`                   | 11    |
| `crates/base60-core/src/url.rs`                     | 10    |
| `crates/base60-cli/src/decode.rs`                   |  9    |
| `crates/base60-cli/src/analyze.rs`                  |  9    |
| `crates/base60-core/src/cuneiform.rs`               |  9    |
| `crates/base60-cli/src/persist.rs`                  |  7    |
| `crates/base60-core/src/convert.rs`                 |  7    |
| `crates/base60-core/src/lens.rs`                    | 15    |
| `crates/base60-cli/src/main.rs`                     |  5    |
| `crates/base60-cli/src/reader.rs`                   |  5    |
| `crates/base60-cli/src/color.rs`                    |  3    |
| `crates/base60-cli/src/cli.rs`                      |  0    |

Total: **164 `#[test]` functions across 14 files**.

## Test Structure

**Canonical shape:**
```rust
// crates/base60-core/src/convert.rs:27
#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(n: u64) -> String { /* local helper */ }
    fn recompose(digits: &[u8; DIGITS]) -> u128 { /* local helper */ }

    #[test]
    fn zero() {
        assert_eq!(fmt(0), "00:00:00:00:00:00:00:00:00:00:00");
    }

    #[test]
    fn sixty_rolls_over() { ... }
}
```

**Conventions:**
- `mod tests` (always named `tests`) sits at the bottom of each source file.
- `use super::*;` is the first line — tests reach into private items, which
  is the primary reason inline tests are preferred over integration tests.
- Private helper `fn`s above the `#[test]` functions build fixtures or
  wrap a generic API into a test-friendly one (e.g. `line_mono` in
  `crates/base60-cli/src/dump.rs:243`, `json`/`html` helpers in
  `crates/base60-cli/src/format.rs:233,239`).
- Test names are assertions in sentence form: `zero_encodes_to_all_first_alphabet_character`,
  `decode_rejects_wrong_length`, `u64_max_roundtrips_in_eleven_digits`,
  `bookmark_set_slot_is_case_insensitive`.
- No `#[should_panic]` usage — error paths are tested via `Result::Err`
  assertions on the return value.

**Helper idioms for writer APIs** — functions that take `W: Write` are
tested against a `Vec<u8>`:
```rust
// crates/base60-cli/src/format.rs:233
fn json(data: &[u8], lens: Option<&dyn Lens>) -> String {
    let mut buf = Vec::new();
    emit_json(data, 0, &mut buf, lens).unwrap();
    String::from_utf8(buf).unwrap()
}
```

**Custom trait impls inside tests** — `crates/base60-cli/src/format.rs:295-301`
defines a one-off `TrickyLens` struct inside the test function to drive an
escape-hazard edge case through `write_json_string`.

## Mocking

**No mocking framework.** The codebase is structured so mocks are
unnecessary:

- I/O is injected via generics (`R: BufRead`, `W: Write`) so tests pass
  `&[u8]` / `Vec<u8>` directly: `crates/base60-cli/src/decode.rs:30,127`,
  `crates/base60-cli/src/analyze.rs:209,402`.
- Colour handling is parameterised over a `&'static Palette`, so tests
  swap `PALETTE_NONE` / `PALETTE_ANSI` explicitly instead of mocking a
  capability (`crates/base60-cli/src/dump.rs:245,251`).
- Lens behaviour is a `dyn Lens` trait object, allowing ad-hoc fakes
  (`TrickyLens` above).
- Environment-dependent logic is tested by directly mutating env vars
  inside a test, with a `SAFETY:` comment explaining the single-threaded
  contract (`crates/base60-core/src/cuneiform.rs:151-161`,
  `crates/base60-cli/src/main.rs:183-219`).
- File-system persistence (`crates/base60-cli/src/persist.rs`) is tested
  via `serialize` / `parse` round-trips that stay in memory; the
  `state_base_dir` env-reading helper is **explicitly not tested** because
  it races with other env-mutating tests — see the comment at
  `crates/base60-cli/src/persist.rs:231-236`.

## Fixtures and Factories

Fixtures are **inline, small, and local to each test module**. No
centralised test-support crate or `#[fixture]` macros.

Canonical factory pattern — `crates/base60-cli/src/persist.rs:167-174`:
```rust
fn sample() -> PersistedState {
    PersistedState {
        scroll: 42,
        cursor: Some(17),
        lens_mode: LensMode::Cuneiform,
        bookmarks: vec![('a', 10), ('z', 999)],
    }
}
```

TUI tests build `ViewState` directly with a synthetic byte slice
(`crates/base60-cli/src/tui.rs:1055-1064`):
```rust
let mut data = vec![0_u8; 10];
data.extend_from_slice(b"Hello, world!");
data.extend_from_slice(&[0_u8; 10]);
let mut s = ViewState::new(&data, LensMode::None, TimeScale::Gar, false);
```

## Test Categories

**Unit tests (all of them):** Every test lives in the same crate as the
code under test and exercises a single private function or narrow API.
There is no separate integration surface; the CLI has no `tests/*.rs` file
invoking the `base60` binary.

**Property-style coverage** without a property-testing framework — tests
iterate over a curated sample set and check an invariant
(`crates/base60-core/src/convert.rs:79-96` checks round-trip on ten hand-picked
`u64` values plus a sweep in `every_digit_is_valid` at line 99).

**Round-trip tests** are the dominant pattern for any encode/decode pair:
- `crates/base60-core/src/url.rs:117` — `encode_u64` ↔ `decode_u64`.
- `crates/base60-cli/src/decode.rs:143` — `dump` text ↔ big-endian bytes.
- `crates/base60-cli/src/persist.rs:177,184` — `serialize` ↔ `parse`.

**Negative tests** assert on specific error variants and on error-message
substrings:
```rust
// crates/base60-core/src/url.rs:141
assert_eq!(decode_u64(""), Err(DecodeError::WrongLength));
// crates/base60-cli/src/decode.rs:173
let err = decode_stream(line.as_bytes(), &mut out).unwrap_err();
assert_eq!(err.kind(), io::ErrorKind::InvalidData);
assert!(err.to_string().contains("99"));
```

**Boundary tests** cover the outer edges of numeric ranges: `u64::MAX`,
`u64::MAX - 1`, `0`, `59`, `60` (`crates/base60-core/src/convert.rs`,
`crates/base60-core/src/url.rs`).

## Coverage

**Not enforced.** No `tarpaulin`, `grcov`, `llvm-cov`, or Codecov
integration is present in CI or `Cargo.toml`. Coverage is emergent from
the "every new module ships with an inline `mod tests`" convention.

## CI Test Setup

**Workflow:** `.github/workflows/ci.yml`

**Triggers:** Push to `main`, PRs targeting `main`.

**Concurrency:** PR runs cancel superseded jobs; `main` runs are never
cancelled (`.github/workflows/ci.yml:10-12`).

**Environment** (`.github/workflows/ci.yml:14-17`):
```yaml
CARGO_TERM_COLOR: always
RUST_BACKTRACE: 1
CARGO_INCREMENTAL: 0
```

**Jobs (5 in total):**

1. **`test`** — cross-platform, 3×3 matrix:
   - OS: `ubuntu-latest`, `macos-latest`, `windows-latest`.
   - Rust: `1.95.0` (MSRV), `stable`, `beta`.
   - `fail-fast: false` — every combination reports independently.
   - Steps: `actions/checkout@v4` → `dtolnay/rust-toolchain@master` →
     `Swatinem/rust-cache@v2` (keyed on `os-rust`) → `cargo test --workspace
     --all-targets --locked` → `cargo test --workspace --doc --locked`.

2. **`clippy`** — Ubuntu + stable toolchain, all warnings treated as errors:
   `cargo clippy --workspace --all-targets --locked -- -D warnings`
   (`.github/workflows/ci.yml:51`).

3. **`fmt`** — Ubuntu + stable + `rustfmt` component:
   `cargo fmt --all --check` (default `rustfmt` settings — no
   `rustfmt.toml`).

4. **`doc`** — Ubuntu + stable, `RUSTDOCFLAGS: -D warnings`:
   `cargo doc --workspace --no-deps --locked` — a broken intra-doc link
   or malformed example fails CI (`.github/workflows/ci.yml:63-72`).

5. **`release-build`** — matrix across all three OSes, `cargo build
   --release --locked` to catch release-profile breakages
   (`profile.release` sets `lto = "thin"`, `codegen-units = 1`,
   `strip = "symbols"` in the root `Cargo.toml:15-18`).

**Caching:** `Swatinem/rust-cache@v2` on every job; separate cache key per
job (`${{ matrix.os }}-${{ matrix.rust }}` for test; default for clippy /
doc; `release-${{ matrix.os }}` for release-build).

**`--locked` everywhere** — CI fails if `Cargo.lock` needs updating,
guarding against silent dependency drift.

## Common Patterns

**Table-driven checks without a helper macro:**
```rust
// crates/base60-core/src/url.rs:117
for n in [0, 1, 42, 60, 60*60, 1_000_000, u64::MAX/3, u64::MAX-1, u64::MAX] {
    let s = encode_u64(n);
    assert_eq!(decode_u64(&s), Ok(n), "roundtrip failed for {n}");
}
```

**Environment-sensitive tests are gated with a positive-only assertion** to
stay resilient under CI environments that set `TERM=dumb` etc.:
```rust
// crates/base60-core/src/cuneiform.rs:156-161
unsafe { std::env::set_var("NO_UNICODE", "1") };
assert!(ascii_fallback_forced());
unsafe { std::env::remove_var("NO_UNICODE") };
if std::env::var("TERM").as_deref() != Ok("dumb") {
    assert!(!ascii_fallback_forced());
}
```

**TUI keyboard-driven tests** call `handle_key` directly on a `ViewState`
without touching crossterm or ratatui — all modal logic is pure:
```rust
// crates/base60-cli/src/tui.rs:1002-1007
let mut s = state(80);
s.cursor = Some(7);
let _ = s.handle_key(KeyCode::Char('m'), KeyModifiers::NONE, b"");
let _ = s.handle_key(KeyCode::Char('Z'), KeyModifiers::NONE, b"");
assert_eq!(s.bookmarks.get(&'z'), Some(&7));
```

**Assertions on rendered strings** use `contains` / `starts_with` /
`ends_with` for output that has incidental formatting:
```rust
// crates/base60-cli/src/dump.rs:305-307
assert!(rendered.starts_with("00000100  "));
assert!(rendered.lines().nth(1).unwrap().starts_with("00000108  "));
```

**`unwrap()` is the norm inside tests** — a failure is a test failure,
which is acceptable. Outside tests, see `CONVENTIONS.md` panic policy.

---

*Testing analysis: 2026-04-23*
