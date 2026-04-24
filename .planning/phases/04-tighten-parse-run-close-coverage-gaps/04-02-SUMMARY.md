---
phase: 04-tighten-parse-run-close-coverage-gaps
plan: 02
subsystem: cli-decode
tags: [rust, cli, decode, refactor, ref-03, error-contract, tdd]
requires:
  - 04-01 (widened 140-cell roundtrip matrix as REF-03's safety net)
  - Phase 3 TEST-03 (loose error pin inherited at tests/cli.rs:155-167)
provides:
  - compile-time array-type invariant for decode::parse_run (&[u8; RUN_LEN])
  - find_digit_run returns Option<&[u8; RUN_LEN]> (no try_into at call sites)
  - full-message stderr pin "line 1: invalid base-60 digit 99 at pair 11"
  - three position/tolerance tests locking pair-1, pair-5, non-digit-run tolerance
  - new defensive error variant "line N: non-digit byte at pair P" inside parse_run
affects:
  - crates/base60-cli/src/decode.rs (parse_run + find_digit_run signatures + 2 call sites)
  - crates/base60-cli/tests/cli.rs (existing decoder test tightened + 3 appended)
tech-stack-added: []
patterns-used:
  - array-type compile-time length invariant (`&[u8; N]` parameter)
  - TryFrom<&[u8]> for &[u8; N] via `if let Ok(..)` (avoids clippy::expect_used)
  - full-message stderr predicate (`predicates::str::contains` on exact literal)
  - TDD RED (compile-fail test) → GREEN (signature change) per-task
key-files-created: []
key-files-modified:
  - crates/base60-cli/src/decode.rs
  - crates/base60-cli/tests/cli.rs
decisions:
  - D-09 (in-place rewrite; no parse_run_strict sibling)
  - D-10 (full-message stderr pin)
  - D-11 (pair-1, pair-5, tolerance — three tests)
duration_seconds: 268
completed: 2026-04-24
---

# Phase 4 Plan 02: Tighten `parse_run` contract + expand decoder error-pin — Summary

Ships REF-03 atomically: `decode::parse_run` and its scanner partner `find_digit_run`
now speak in `&[u8; RUN_LEN]` arrays, the digit-ASCII check lives inside `parse_run`
(callers cannot bypass via a manually-built slice), and the stderr error-format is
locked to a full-message contains plus three position/tolerance pins — closing the
Pitfall 8 drift window under Plan 04-01's widened 140-cell roundtrip net.

## Tasks & commits

| Task | Name                                                                 | Commit    |
|------|----------------------------------------------------------------------|-----------|
| 1    | RED — add direct-call tests for tightened parse_run (compile-fail)   | `ae3919f` |
| 1    | GREEN — rewrite parse_run + find_digit_run + migrate both call sites | `05153fc` |
| 2    | Full-message + position-pinning decoder error tests (tests/cli.rs)   | `af4ecd7` |

Per-task TDD: Task 1 splits into a RED commit (two tests that call `parse_run` with
`&[u8; RUN_LEN]`, which cannot compile against the old `&str` signature) and a GREEN
commit (the signature rewrite that makes them compile and pass).

## Diff — `crates/base60-cli/src/decode.rs`

### Before (post-Plan-04-01)

```rust
// line 352-368 (before)
fn find_digit_run(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    if bytes.len() < RUN_LEN { return None; }
    for start in 0..=bytes.len() - RUN_LEN {
        let slice = &bytes[start..start + RUN_LEN];
        if is_digit_run(slice)
            && not_extended_left(bytes, start)
            && not_extended_right(bytes, start + RUN_LEN)
        {
            return Some(std::str::from_utf8(slice).expect("ascii"));
        }
    }
    None
}

// line 402-427 (before)
fn parse_run(run: &str, line_no: usize) -> io::Result<u64> {
    let mut value: u128 = 0;
    for (i, pair) in run.split(':').enumerate() {
        debug_assert_eq!(pair.len(), 2);
        let bytes = pair.as_bytes();
        let hi = bytes[0] - b'0';
        let lo = bytes[1] - b'0';
        let digit = hi * 10 + lo;
        if digit >= 60 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "line {line_no}: invalid base-60 digit {digit} at pair {}",
                    i + 1
                ),
            ));
        }
        value = value * 60 + u128::from(digit);
    }
    u64::try_from(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("line {line_no}: decoded value exceeds u64::MAX"),
        )
    })
}
```

### After (this plan)

```rust
// find_digit_run — returns array-typed borrow; TryFrom via if-let-Ok avoids expect_used.
fn find_digit_run(line: &str) -> Option<&[u8; RUN_LEN]> {
    let bytes = line.as_bytes();
    if bytes.len() < RUN_LEN { return None; }
    for start in 0..=bytes.len() - RUN_LEN {
        let slice = &bytes[start..start + RUN_LEN];
        if is_digit_run(slice)
            && not_extended_left(bytes, start)
            && not_extended_right(bytes, start + RUN_LEN)
        {
            if let Ok(arr) = <&[u8; RUN_LEN]>::try_from(slice) {
                return Some(arr);
            }
        }
    }
    None
}

// parse_run — array-typed input + internal ASCII-digit guard + preserved error strings.
fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64> {
    let mut value: u128 = 0;
    for i in 0..DIGITS {
        let pair_start = i * (PAIR + 1); // 3 bytes per pair: 2 digits + 1 separator.
        let hi_byte = run[pair_start];
        let lo_byte = run[pair_start + 1];
        if !hi_byte.is_ascii_digit() || !lo_byte.is_ascii_digit() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("line {line_no}: non-digit byte at pair {}", i + 1),
            ));
        }
        let hi = hi_byte - b'0';
        let lo = lo_byte - b'0';
        let digit = hi * 10 + lo;
        if digit >= 60 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "line {line_no}: invalid base-60 digit {digit} at pair {}",
                    i + 1
                ),
            ));
        }
        value = value * 60 + u128::from(digit);
    }
    u64::try_from(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("line {line_no}: decoded value exceeds u64::MAX"),
        )
    })
}
```

### Caller migrations (both Plan-04-01 call sites)

```rust
// decode_from_text — removed the bridging try_into from Plan 04-01.
let Some(run) = find_digit_run(&line) else { continue; };
let value = parse_run(run, idx + 1)?;              // run: &[u8; RUN_LEN]

// decode_from_html — removed the from_utf8 + &str handoff.
let value = parse_run(&run, 0)?;                   // run: [u8; RUN_LEN]
```

## Format-string preservation (D-10 audit)

Character-identical to the pre-refactor bodies:

| Format string                                                           | Preserved? |
|-------------------------------------------------------------------------|------------|
| `"line {line_no}: invalid base-60 digit {digit} at pair {}"` (arg `i+1`) | yes        |
| `"line {line_no}: decoded value exceeds u64::MAX"`                      | yes        |

New (additive, unpinned — defensive variant):

| Format string                                        | Status           |
|------------------------------------------------------|------------------|
| `"line {line_no}: non-digit byte at pair {}"` (`i+1`) | new; acceptable  |

`find_digit_run` still filters non-digit bytes upstream, so the new variant is
belt-and-braces. It is covered by an inline unit test (`parse_run_flags_non_digit_byte_at_pair_position`)
but intentionally not locked into `tests/cli.rs` — the public stderr contract
continues to speak only the two pre-existing messages.

## Tests added / retained

| File                                    | Test                                                    | Kind        | Delta     |
|-----------------------------------------|---------------------------------------------------------|-------------|-----------|
| crates/base60-cli/src/decode.rs         | `parse_run_flags_non_digit_byte_at_pair_position`       | inline unit | new       |
| crates/base60-cli/src/decode.rs         | `parse_run_reports_invalid_digit_at_first_pair`         | inline unit | new       |
| crates/base60-cli/tests/cli.rs          | `decoder_invalid_digit_99_error_contains_the_digit`     | integration | tightened |
| crates/base60-cli/tests/cli.rs          | `decoder_invalid_digit_at_pair_1_reports_pair_1`        | integration | new       |
| crates/base60-cli/tests/cli.rs          | `decoder_invalid_digit_at_pair_5_reports_pair_5`        | integration | new       |
| crates/base60-cli/tests/cli.rs          | `decoder_ignores_non_digit_run_lines`                   | integration | new       |

All 16 pre-existing decoder inline tests + 8 pre-existing cli.rs tests continue
to pass. Net: +5 tests (2 inline + 3 integration); 1 integration test tightened.

## Clippy / doc notes — zero `#[allow(...)]` attributes added

- `clippy::indexing_slicing` — **not triggered**. The `run[pair_start]` /
  `run[pair_start + 1]` accesses on `&[u8; RUN_LEN]` with `i < DIGITS = 11`
  yield `pair_start + 1 = 3*i + 1 <= 31 < RUN_LEN = 32` — clippy proves the
  bound at compile time and does not fire.
- `clippy::expect_used` — avoided in `find_digit_run` by replacing
  `slice.try_into().expect("RUN_LEN-sized slice")` with
  `if let Ok(arr) = <&[u8; RUN_LEN]>::try_from(slice) { return Some(arr); }`.
  The non-happy branch (slice length ≠ RUN_LEN) is genuinely unreachable by
  construction; `if let Ok(..)` drops into the next loop iteration, which is
  dead on the first iteration too.
- `# Errors` rustdoc section on `parse_run` enumerates all three variants —
  `RUSTDOCFLAGS=-D warnings` passes.

## Constant arithmetic note

The plan's `04-02-PLAN.md` comment asserted `RUN_LEN = 33`; the actual source
constant is `RUN_LEN = PAIR * DIGITS + (DIGITS - 1) = 2*11 + 10 = 32`. This is
a comment-only discrepancy in the plan — the code has been 32 since Phase 1.
No implementation adjustment was required; mentioned for transparency.

## Verification — all gates green

| Gate                                                                     | Result |
|---------------------------------------------------------------------------|--------|
| `cargo test -p base60 --locked decode::tests` (18 tests, +2 new)         | pass   |
| `cargo test -p base60 --test cli --locked decoder_` (4 tests, +3 new)    | pass   |
| `cargo test --workspace --all-targets --locked` (139+16+4+1+41 + gates)  | pass   |
| `cargo clippy --workspace --all-targets --locked -- -D warnings`         | pass   |
| `cargo fmt --all --check`                                                | pass   |
| `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --locked`      | pass   |
| `cargo check -p base60 --locked`                                         | pass   |
| Manual sanity: `echo '...99:00:...' \| base60 decode` → stderr contains `"at pair 1"` | pass |

## Deviations from plan

None — plan executed exactly as written. The only pre-existing fact worth
flagging was the plan comment's `RUN_LEN = 33` arithmetic; the source
constant was 32 all along, so no code adjustment was needed.

## Threat model — mitigations landed

| Threat ID    | Status   | Evidence                                                                                 |
|--------------|----------|------------------------------------------------------------------------------------------|
| T-04-02-01   | mitigated| `parse_run` takes `&[u8; RUN_LEN]`; digit-ASCII check internal; compile-time invariant.   |
| T-04-02-02   | mitigated| `tests/cli.rs::decoder_invalid_digit_99_error_contains_the_digit` pins full phrasing.    |
| T-04-02-03   | mitigated| `rejects_twelve_pair_overextension` inline test still green (see decode.rs:489-495).     |
| T-04-02-04   | accepted | New `"non-digit byte at pair {P}"` variant reveals only the pair index (no secret leak).|

## Decisions made

- Used `if let Ok(..)` instead of `.expect(..)` inside `find_digit_run` to
  satisfy `clippy::expect_used` without a targeted allow (D-09-support).
- Left `is_digit_run` untouched — it still gates `find_digit_run`'s scan loop
  so non-digit lines are skipped silently (required by
  `decoder_ignores_non_digit_run_lines`). Removing it would make `parse_run`
  raise on lines `find_digit_run` should quietly skip — a behaviour regression.

## Unresolved questions

None.

## Self-Check

**Files:**
- `crates/base60-cli/src/decode.rs` — modified (verified: grep for `fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64>` returns 1 hit).
- `crates/base60-cli/tests/cli.rs` — modified (verified: grep for `line 1: invalid base-60 digit 99 at pair 11` returns 1 hit).
- `.planning/phases/04-tighten-parse-run-close-coverage-gaps/04-02-SUMMARY.md` — this file.

**Commits:**
- `ae3919f` test(04-02): add failing tests for tightened parse_run
- `05153fc` refactor(04-02): tighten parse_run + find_digit_run to &[u8; RUN_LEN]
- `af4ecd7` test(04-02): pin decoder error messages to full contract (D-10/D-11)

## Self-Check: PASSED
