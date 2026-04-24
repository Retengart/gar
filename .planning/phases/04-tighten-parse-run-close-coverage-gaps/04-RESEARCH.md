# Phase 4: Tighten `parse_run` + Close Coverage Gaps - Research

**Researched:** 2026-04-24
**Domain:** Rust CLI hardening — decoder contract tightening, format-preserving roundtrip, test-coverage gap closure
**Confidence:** HIGH (every claim verified against in-repo source or Context7)

## Summary

Phase 4 bundles three orthogonal strands under one phase boundary:

1. **REF-04** — ship length-preserving dump/decode across ansi/plain/json/html, add JSON and HTML decoders, auto-detect input format, and widen the roundtrip matrix 28 → 140 cells. This is the biggest strand by LOC and the one that has to land first because it widens the safety net that REF-03 refactors under.
2. **REF-03** — tighten `decode::parse_run` from `&str` to `&[u8; RUN_LEN]`, promote the digit-validity check inside the function, and pin the exact error-message contract so future refactors cannot drift it silently (Pitfall 8).
3. **TEST-05** — cover three currently-untested paths: `reader::{load_file, load_stdin}` (mmap + stdin), TUI exit-with-save via `ratatui::backend::TestBackend`, and `persist::state_base_dir`'s XDG → HOME fallback ladder.

Plan granularity is 4 plans (D-13): REF-04 → REF-03 → TEST-05-reader → TEST-05-TUI/persist. Commit order per D-12 matches. The D-17 gate (full `test + clippy + fmt + doc` green) must pass between every commit. Every new env-mutating test uses `#[serial(env)]` (Phase 2 idiom); every new spawn site uses `base60_cmd()` from `tests/common/mod.rs` (Phase 3 idiom). No `base60-core` changes (zero-dep invariant preserved).

**Primary recommendation:** Plan 04-01 (REF-04) is load-bearing. Ship the length-metadata trailer (`# bytes=0x<hex>\n` / `<!-- bytes=0x<hex> -->` / `{"type":"meta","bytes":<dec>}\n`) first; slot the JSON/HTML decoders behind auto-detect + `--input-format` flag; flip the matrix constants (`ROUNDTRIP_FIXTURES → ALL_FIXTURES`, `ROUNDTRIP_FORMATS → Format::ALL`) inside the same commit per D-14. Hand-roll the JSON decoder (do NOT add `serde_json` as a direct dep — see Q3). Plans 04-02/03/04 are mechanical extensions of established patterns.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Length metadata emission (`# bytes=0x<hex>`) | CLI emitter layer (`dump.rs` + `format.rs`) | — | Format-aware; each emitter owns its own trailer syntax. Input length is known upfront from the reader, so the trailer is a post-loop write, not an accumulator. |
| Format-specific decoder dispatch | CLI decode layer (`decode.rs`) | — | `decode_stream` already owns the BufRead → byte-stream contract. New `decode_from_json` / `decode_from_html` are `pub(crate)` helpers it dispatches to after format detection. |
| Format auto-detection | CLI decode layer (`decode.rs`) | CLI arg layer (`cli.rs`) | `--input-format` flag is parsed by clap; `decode_stream` peeks the first non-empty line and routes. Both live in `base60-cli`. |
| HTML state-machine parser | CLI decode layer (submodule of `decode.rs`) | — | ~60-line hand-rolled parser tightly coupled to `format::emit_html`'s tag shapes. Documented in module comment. No external dep. |
| `parse_run` digit validation | CLI decode layer (`decode.rs`) | — | Moves from `is_digit_run` (separate sibling fn) INTO `parse_run`. The array-type parameter (`&[u8; RUN_LEN]`) makes bypass impossible. |
| Error-message contract pinning | CLI integration tests (`tests/cli.rs`) | — | Tests assert `.stderr(contains("line 1: invalid base-60 digit 99 at pair 11"))` against the spawned binary, not the in-process fn. Pins user-visible contract. |
| Reader coverage (mmap/stdin) | CLI integration tests (`tests/reader.rs` — NEW) | CLI source (`reader.rs`) | Tempfile-backed fixture for mmap; `io::Cursor<Vec<u8>>` for stdin. Reader source stays unchanged — tests exercise the existing seams. |
| TUI exit-with-save coverage | CLI integration tests (`tests/tui.rs` — NEW) | CLI source (`tui.rs` — seam-adding refactor) | `ratatui::run(...)` + `event::read()` are currently hardcoded; a new `run_with_backend_and_events<B: Backend, I: Iterator<Item=Event>>` seam is required to drive the TUI without a real terminal. |
| `persist::state_base_dir` coverage | CLI integration tests (`tests/persist.rs` — NEW) | CLI source (`persist.rs` — minor: make `state_base_dir` `pub(crate)` if not already) | Already `pub(crate) fn` at `persist.rs:72`. XDG → HOME fallback ladder exercised via `env::set_var`/`remove_var` + tempdir + `#[serial(env)]`. |

## Standard Stack

### Core (unchanged)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| anyhow | 1.0.102 | Top-level error flow in binary | Already present (`base60-cli/Cargo.toml:22`) |
| clap | 4.6.1 (derive) | Argument parsing — new `--input-format` flag hangs off `DecodeArgs` | Already present; `ValueEnum` derive pattern matches existing enums |
| ratatui | 0.30.0 | TUI rendering; `ratatui::backend::TestBackend` for new tests | Already present (runtime); `TestBackend` is crate-built-in, no dep change |

### Supporting (already present as dev-deps)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| assert_cmd | 2.2.1 | Spawn `base60` as child, capture stdout/stderr/status | Integration tests — decoder error pin, HTML/JSON roundtrip cells |
| predicates | 3.1.4 | `predicates::str::contains` / `starts_with` for assertions | Error-message pinning in `tests/cli.rs` |
| serial_test | 3.4.0 (default-features=false) | `#[serial(env)]` — Phase 2 idiom | Plan 04-04 persist + TUI tests (both mutate env) |
| base60-core | path dep | Dev-dep for `TimeScale` re-export into tests | Already used; no change |

### NEW dev-dep in this phase
| Library | Version | Purpose | Source |
|---------|---------|---------|--------|
| tempfile | 3 (latest stable: 3.27.0 per `cargo search tempfile` on 2026-04-24; use `tempfile = "3"` caret so CI resolves to latest 3.x within MSRV) | `tempfile::NamedTempFile` for mmap fixture (Plan 04-03); `tempfile::tempdir()` for `$XDG_STATE_HOME` redirect (Plan 04-04) | Deferred from Phase 3 D-22; first need is Plan 04-03 per D-16 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled JSON decoder | `serde_json` as direct dev-dep | `serde_json` 1.0.149 is ALREADY in transitive graph via ratatui/termwiz (confirmed at Cargo.lock line 1348). Adding it as a direct dev-dep would ease decoder implementation but violates the Plan 03-02 precedent of hand-rolled emitters (`format.rs:31` docstring: "Hand-rolled to avoid pulling in `serde_json` for the small, fixed schema"). **Prefer hand-rolled decoder** — the emitted shape is fully specified (`"offset":` and `"type":"meta"` prefixes only), and the per-line parser is ~30 lines. Consistency with the emitter wins. |
| `ratatui::backend::TestBackend` for TUI tests | Skip TUI integration testing entirely | TEST-05 SC3 explicitly requires TUI exit-with-save coverage (ROADMAP Phase 4 line 71). Skipping would leave the save-path untested. |
| `std::process::Command` + `Stdio::piped()` for TUI test | Drive TUI as child process | The TUI waits for crossterm events on a real terminal, which assert_cmd cannot provide. In-process `TestBackend` + programmatic event injection is the only viable approach. |
| `io::Cursor<Vec<u8>>` for stdin test | Spawn child and `write_stdin()` | Both work; in-process `Cursor` is cheaper and more direct for a pure BufRead test. Integration test IS still viable but requires exposing a seam. Recommend **in-process Cursor** for `load_stdin`, **tempfile+spawn** for `load_file` because mmap requires a real file. |

**Installation (Plan 04-03):**
```toml
# crates/base60-cli/Cargo.toml [dev-dependencies]
tempfile = "3"
```

**Version verification:** `cargo search tempfile --limit 1` → `3.27.0` on 2026-04-24. Caret `"3"` resolves to latest 3.x per semver. Phase 3 CONTEXT D-22 noted the version as `"3"`; keep the caret form for consistency with existing dev-deps (`assert_cmd = "2"`, `predicates = "3"`).

## Architecture Patterns

### System Architecture Diagram (Phase 4 data flow)

```
┌──────────── base60 dump (REF-04 emit side) ─────────────┐
│  input bytes ──► reader::load ──► dump_all / emit_*     │
│                                     │                    │
│                                     ├─ ansi/plain  ──► stdout + "# bytes=0x<hex>\n" trailer
│                                     ├─ json        ──► stdout + {"type":"meta","bytes":N}\n final NDJSON line
│                                     └─ html        ──► stdout + "<!-- bytes=0x<hex> -->" before </body>
└─────────────────────────────────────────────────────────┘

┌──────────── base60 decode (REF-04 consume side) ────────┐
│  stdin/file ──► decode_stream (peeks first line)        │
│                     │                                    │
│                     ├─ starts with "{\"offset\":" ──► decode_from_json (NEW)
│                     ├─ starts with "<!DOCTYPE"|"<html" ──► decode_from_html (NEW state machine)
│                     └─ otherwise (default)        ──► find_digit_run + parse_run (existing path; now &[u8; RUN_LEN])
│                                                          │
│                                                          └─► w.write_all(&u64.to_be_bytes())
│                                                              + optional truncation to meta.bytes
└─────────────────────────────────────────────────────────┘

┌──────────── --input-format override (D-06) ─────────────┐
│  DecodeArgs { file, input_format: InputFormat } (NEW)   │
│     ├─ Auto (default)  ──► sniff path above             │
│     ├─ Ansi | Plain    ──► find_digit_run path          │
│     ├─ Json            ──► decode_from_json             │
│     └─ Html            ──► decode_from_html             │
└─────────────────────────────────────────────────────────┘
```

### Recommended File Structure (Phase 4 additions)
```
crates/base60-cli/
├── src/
│   ├── decode.rs           # EDIT: parse_run signature (REF-03) + decode_from_json + decode_from_html + auto-detect (REF-04)
│   ├── cli.rs              # EDIT: DecodeArgs gains --input-format flag (REF-04)
│   ├── dump.rs             # EDIT: emit "# bytes=0x<hex>\n" trailer after dump_all's loop (REF-04 ansi/plain)
│   ├── format.rs           # EDIT: emit_json adds meta line; emit_html inserts comment before epilogue (REF-04)
│   └── tui.rs              # EDIT (seam): extract run_with_backend_and_events<B,I> for TestBackend drive (Plan 04-04)
├── tests/
│   ├── common/mod.rs       # EDIT: flip ROUNDTRIP_FIXTURES → ALL_FIXTURES, ROUNDTRIP_FORMATS → Format::ALL (Plan 04-01)
│   ├── cli.rs              # EDIT: expand decoder error pin to full message + add 2-3 position tests (Plan 04-02)
│   ├── reader.rs           # NEW (Plan 04-03): mmap via tempfile + stdin via Cursor + file-open error path
│   ├── tui.rs              # NEW (Plan 04-04): TestBackend 80×24 + tempdir XDG_STATE_HOME + drive `jjjjj b1 q`
│   └── persist.rs          # NEW (Plan 04-04): state_base_dir XDG → HOME ladder, #[serial(env)]
└── Cargo.toml              # EDIT: add tempfile = "3" to [dev-dependencies] (Plan 04-03)
```

### Pattern 1: Length-metadata trailer (REF-04 D-01/D-04)
**What:** Every dump format emits a trailing length-metadata line so `decode` can truncate the final 8-byte-aligned chunk back to the real input length.

**When to use:** Always (D-02 — uniform output). Applied post-loop in `dump_all` and each `emit_*`.

**Example (ansi/plain — `dump.rs::dump_all`):**
```rust
// Source: current dump.rs:115-131 — planner appends the trailer after the existing chunks loop
pub(crate) fn dump_all<W: Write>(
    data: &[u8],
    base_offset: u64,
    w: W,
    palette: &Palette,
    lens: Option<&dyn Lens>,
) -> io::Result<()> {
    let mut out = BufWriter::new(w);
    for (idx, chunk) in data.chunks(CHUNK).enumerate() {
        let offset = base_offset.saturating_add((idx * CHUNK) as u64);
        write_line(&mut out, offset, chunk, palette, lens)?;
    }
    // NEW: trailer line (REF-04 D-01, D-04). Uses `#` prefix so
    // find_digit_run's scanner won't accidentally treat it as a digit run.
    writeln!(out, "# bytes=0x{:x}", data.len())?;
    out.flush()
}
```

**Example (html — `format.rs::emit_html`):** insert `<!-- bytes=0x<hex> -->` immediately before `HTML_EPILOGUE` write.

**Example (json — `format.rs::emit_json`):** append `{"type":"meta","bytes":<dec>}\n` after the chunks loop, before `out.flush()`.

### Pattern 2: Format auto-detection + `--input-format` override (REF-04 D-06)
**What:** `decode_stream` peeks the first non-empty line to choose a decoder; `DecodeArgs.input_format` (clap enum, default `Auto`) overrides.

**Why this order:** Explicit override has priority; auto is the fallback. A user who sets `--input-format=json` should never hit the sniff path.

**Dispatch shape (pseudocode for `decode_stream`):**
```rust
pub(crate) fn decode_stream<R: BufRead, W: Write>(
    r: R,
    w: &mut W,
    input_format: InputFormat,  // NEW parameter; threaded from run_decode
) -> io::Result<()> {
    let mut reader = r;
    let first_line = peek_first_nonempty_line(&mut reader)?;
    let format = match input_format {
        InputFormat::Auto => sniff(&first_line),
        InputFormat::Ansi | InputFormat::Plain => SniffedFormat::AnsiPlain,
        InputFormat::Json => SniffedFormat::Json,
        InputFormat::Html => SniffedFormat::Html,
    };
    match format {
        SniffedFormat::AnsiPlain => decode_from_text(reader, w),  // existing loop, parse_run path
        SniffedFormat::Json => decode_from_json(reader, w),       // NEW
        SniffedFormat::Html => decode_from_html(reader, w),       // NEW
    }
}

fn sniff(first_line: &str) -> SniffedFormat {
    let t = first_line.trim_start();
    if t.starts_with("<!DOCTYPE") || t.starts_with("<html") {
        SniffedFormat::Html
    } else if t.starts_with("{\"offset\":") {
        SniffedFormat::Json
    } else {
        SniffedFormat::AnsiPlain
    }
}
```

### Pattern 3: `parse_run` signature tightening (REF-03 D-09)
**What:** `fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64>` replaces `fn parse_run(run: &str, line_no: usize) -> io::Result<u64>`. The only caller is `decode_stream` line 36.

**Current (decode.rs:94-119):**
```rust
fn parse_run(run: &str, line_no: usize) -> io::Result<u64> {
    let mut value: u128 = 0;
    for (i, pair) in run.split(':').enumerate() {
        debug_assert_eq!(pair.len(), 2);
        let bytes = pair.as_bytes();
        let hi = bytes[0] - b'0';
        let lo = bytes[1] - b'0';
        let digit = hi * 10 + lo;
        if digit >= 60 { return Err(...); }
        value = value * 60 + u128::from(digit);
    }
    u64::try_from(value).map_err(...)
}
```

**After REF-03 (array-typed, digit-validity check inside):**
```rust
fn parse_run(run: &[u8; RUN_LEN], line_no: usize) -> io::Result<u64> {
    let mut value: u128 = 0;
    // Array-type invariant replaces the old `debug_assert_eq!(pair.len(), 2)` —
    // the compiler guarantees `run.len() == RUN_LEN`. We still validate
    // digit-ASCII-ness per byte because the caller (find_digit_run) vets this
    // in Phase 3 but the type alone doesn't promise it.
    for i in 0..DIGITS {
        let pair_start = i * (PAIR + 1);  // 3 per pair: 2 digits + 1 colon (except last)
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
                format!("line {line_no}: invalid base-60 digit {digit} at pair {}", i + 1),
            ));
        }
        value = value * 60 + u128::from(digit);
    }
    u64::try_from(value).map_err(|_| io::Error::new(
        io::ErrorKind::InvalidData,
        format!("line {line_no}: decoded value exceeds u64::MAX"),
    ))
}
```

**Caller migration (single site — decode.rs:36):** change `parse_run(run, idx + 1)` to `parse_run(run.try_into().expect("RUN_LEN bytes"), idx + 1)` where `run` came from `find_digit_run` which already guarantees `RUN_LEN` bytes. Planner may also change `find_digit_run` to return `&[u8; RUN_LEN]` directly; preferred because it moves the `try_into` boundary up to where the length is proven, eliminating a runtime check.

**Digit-ASCII validation note:** The current `is_digit_run` helper (decode.rs:63-77) already validates ASCII-digit-ness at colon positions. Once `find_digit_run` proves `is_digit_run(slice)`, the inside-`parse_run` check is redundant but cheap — keep it as a belt-and-braces hedge, OR delete `is_digit_run` and let `parse_run` own the full validation. **Recommend the latter** — moves all digit-checking into one function per REF-03's spec ("promote digit-check inside the function").

### Anti-Patterns to Avoid
- **Do not emit `# bytes=` inside the chunks loop.** The trailer is a post-loop write; emitting per-chunk would confuse the scanner.
- **Do not add `serde_json` as a direct dep** — the emitter is hand-rolled for a reason (no `serde_json` in `[dependencies]` line 22 of `base60-cli/Cargo.toml`); the decoder should match. Consistency > convenience.
- **Do not `pub`-widen HTML decoder state machine types** — new internals stay `pub(crate)` per Phase 3 D-07. The `tests/common/mod.rs` file-scope `#![allow(unreachable_pub)]` trick is ONLY for integration-test helpers, not for production code.
- **Do not split `--input-format` parsing from decode dispatch across commits** — they ship together in the REF-04 commit. A commit where the flag exists but is ignored is a broken intermediate state (D-17).
- **Do not ship `parse_run_strict` alongside the old `parse_run`** — Pitfall 8's "alongside migration" advice is explicitly dropped by D-09 (only one caller; compiler catches drift).
- **Do not test TUI via `std::process::Command`** — the TUI waits for crossterm events from a real terminal; in-process `TestBackend` + programmatic event injection is the only viable path.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Temporary file for mmap fixture | Raw `std::fs::File` in `/tmp/base60-test-<pid>` | `tempfile::NamedTempFile` | Handles cleanup on panic, Windows path quirks, unique naming. Established pattern per Phase 3 D-22 / 04-CONTEXT D-16. |
| Temp directory for `XDG_STATE_HOME` | Raw `std::fs::create_dir_all` + manual cleanup | `tempfile::tempdir()` | Auto-deletes on drop even if the test panics; works identically on all 3 CI OSes. |
| Hex literal emission in `# bytes=0x<hex>` | Manual `format!("{:x}", ...)` with leading-zero padding concerns | `write!(out, "# bytes=0x{:x}", data.len())` — no padding, no prefix drama | `{:x}` is the canonical Rust hex formatter; matches `format.rs:97`'s existing `{offset:08x}` style idiom. No fixed width needed — `0x` prefix makes the boundary unambiguous. |
| JSON decoder parsing | `serde_json::Value` tree walk | Hand-rolled per-line prefix check (`.starts_with("{\"offset\":")` / `.starts_with("{\"type\":\"meta\"")`) | Emitter is hand-rolled (format.rs:31 docstring); schema is fully fixed; ~30 LOC. `serde_json` dep adds transitive weight and fights the zero-dep-adjacent posture. |
| HTML decoder | `html5ever` / `scraper` | ~60-line state machine on the emitter's exact tag shapes | The only HTML `decode` needs to consume is `emit_html`'s output. General-purpose HTML parsers are massive overkill; strict coupling to emitter is documented in a module-level comment (D-05). |
| Bookmark-slot input parsing in TUI test | Custom key-code synthesiser | Synthesise `KeyCode::Char('b')` then `KeyCode::Char('1')` via the existing `handle_key` seam OR a new `run_with_backend_and_events<B, I>` seam that pushes pre-built `Event::Key(KeyEvent { code, ... })` values | `tui.rs:428-454` (bookmark handler) already accepts `KeyCode::Char(c)` — we mirror the production code path. See Pattern 3 below for the seam. |
| FNV-1a hash precomputation for state file lookup | Roll our own hash | Call existing `persist::state_file(&tempfile_path)` helper | Already `pub(crate)`; returns the exact `.state` path the test should read. |

**Key insight:** Every "custom solution" in Phase 4 has an existing library (tempfile, ratatui-TestBackend, serial_test) or an existing in-repo helper (`state_file`, `base60_cmd`, `assert_roundtrip`, `ALL_LENS_CONFIGS`). Planner should lean on those first.

## Runtime State Inventory

> Phase 4 is primarily a feature-addition phase, NOT a rename/refactor/migration. The `parse_run` tightening (REF-03) IS a signature change but has exactly one caller, so there's no runtime state to track. No database keys, no external service config, no OS-registered state, no env var renames, no build artifacts carrying stale names.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — `persist` state files use FNV-1a of absolute canonical path, which is insensitive to REF-03/REF-04 changes. REF-04 does NOT change the on-disk state file format. | None. |
| Live service config | None — offline CLI tool. | None. |
| OS-registered state | None — no systemd units, no scheduled tasks, no pm2 entries. | None. |
| Secrets/env vars | `NO_COLOR`, `NO_UNICODE`, `TERM`, `XDG_STATE_HOME`, `HOME` — all existing, no renames. Plan 04-04's new `tests/persist.rs` / `tests/tui.rs` READ and mutate these under `#[serial(env)]`. | None — purely additive use. |
| Build artifacts / installed packages | `base60` binary in `$HOME/.cargo/bin/`: post-phase-4 it emits length trailers on every dump. Old decoders would silently ignore trailing `#`-prefixed lines (the existing scanner does); new decoders consume the trailer. Dev-dep `tempfile` gets resolved into `Cargo.lock` on first `cargo build`. | No user action — `--input-format=auto` default + D-03 fallback means old dumps still decode. |

**Nothing found in stored/live-config/OS-registered categories — verified by:** (1) `grep -rn "serde_json\|tempfile" Cargo.lock` shows `tempfile` absent today; (2) `crates/base60-cli/src/persist.rs:84-93` defines `fnv1a` as a CLI-local hash untied to any phase-4 change; (3) `.planning/codebase/INTEGRATIONS.md` documents "Not applicable" for every external-integration category.

## Common Pitfalls

### Pitfall 1: Length-metadata trailer collides with `find_digit_run`
**What goes wrong:** The scanner's extension guard (`not_extended_left` / `not_extended_right`) treats `[0-9:]` as run-chars. If the trailer were `bytes=1024` (digits only), the scanner could potentially match nested digit sequences.

**Why it happens:** `find_digit_run` walks every `[0..=len-RUN_LEN]` starting position. Without the `#` prefix, a 33-char run of digits and colons *could* land inside a future free-form metadata line.

**How to avoid:** D-04 mandates the `#` prefix (`# bytes=0x400\n`). `#` is not in `[0-9:]`, so `is_digit_run` will never start a match at a `#`-prefixed position. Verified against decode.rs:63-77 `is_digit_run` logic.

**Warning signs:** any planner proposing `bytes=0x400` without a leading `#` — would break the invariant.

### Pitfall 2: Legacy dump (no trailer) decodes to wrong length
**What goes wrong:** A user regenerates a dump with v1 (pre-REF-04) and pipes it into v2's decoder. No `# bytes=` trailer exists; decoder outputs the 8-byte-aligned padded length, silently corrupting the last up-to-7 bytes for non-aligned inputs.

**Why it happens:** Backwards compatibility. Per PROJECT.md line 114, the `decode` accept-format must stay additive and stable.

**How to avoid:** D-03 — when decoder sees EOF without having consumed a `# bytes=` trailer, emit a stderr warning ("decode: no length metadata; assuming input was 8-byte-aligned. Last chunk may contain zero-padding.") and continue with 8-byte-aligned output. Exit 0. Exact wording is Claude's discretion.

**Warning signs:** the warning message is missing, OR the decoder errors (exit 1) instead of warning. Either fails D-03.

### Pitfall 3: REF-03 silently drifts error-message contract (Pitfall 8 from PITFALLS.md)
**What goes wrong:** REF-03 refactors `parse_run`'s internal arithmetic; the format string drifts from `"line {line_no}: invalid base-60 digit {digit} at pair {i+1}"` to `"row {line_no}: digit {digit} out of range at position {i+1}"`. Consumers grepping stderr for `"pair"` break silently; existing loose assert (`contains("99").and(contains("invalid"))`) stays green.

**Why it happens:** The current test in `tests/cli.rs:156-167` pins only `"99"` + `"invalid"` — it's deliberately loose per Plan 03-03.

**How to avoid:** D-10 — expand to full-message pin: `.stderr(predicates::str::contains("line 1: invalid base-60 digit 99 at pair 11"))`. Locks line-number + pair-position + digit-value + the exact phrasing ("line"/"pair"/"invalid") into the contract. D-11 adds 2-3 position-pinning tests (pair 1, pair 5, non-digit-run-line tolerance).

**Warning signs:** a planner writing `.contains("pair 11")` alone instead of the full message — too narrow. Also, a planner asserting on `.kind()` instead of `.to_string()` — PITFALLS.md Pitfall 8 explicitly warns against this.

### Pitfall 4: TUI `TestBackend` test can't read keyboard events without a seam-adding refactor
**What goes wrong:** The current `tui.rs::run` at line 72 hardcodes `ratatui::run(|terminal| { loop { event::read()?; ... } })`. `event::read()` blocks on a real crossterm-backed stdin. A `TestBackend` doesn't drive stdin, so the test hangs.

**Why it happens:** Production TUI ties input + rendering + init/restore into one function for simplicity. Making it testable requires inversion-of-control: the test must supply its own event iterator.

**How to avoid:** Plan 04-04 adds a seam. Recommended shape:
```rust
// tui.rs — refactor pub(crate) fn run to delegate:
pub(crate) fn run(
    data: &[u8], base_offset: u64, initial_mode: LensMode,
    scale: TimeScale, purist: bool, input_file: Option<&Path>,
) -> Result<()> {
    ratatui::run(|terminal| -> Result<()> {
        run_with_terminal(terminal, data, base_offset, initial_mode, scale, purist, input_file,
            || crossterm::event::read().map(Some))
    })
}

// NEW: pub(crate) seam the test drives.
pub(crate) fn run_with_terminal<B: ratatui::backend::Backend, F>(
    terminal: &mut ratatui::Terminal<B>,
    data: &[u8], base_offset: u64, initial_mode: LensMode,
    scale: TimeScale, purist: bool, input_file: Option<&Path>,
    mut next_event: F,
) -> Result<()>
where F: FnMut() -> io::Result<Option<crossterm::event::Event>>,
{
    // Existing body from tui.rs:61-89, but call `next_event()?` instead of
    // `event::read()?`. `Option::None` signals "no more events, shut down".
}
```

The test passes a closure that pops from a pre-built `Vec<Event>` iterator. Planner sizes the Vec to the drive sequence + one `Event::Key(q)` terminator.

**Minimal-invasive alternative:** Leave `tui::run` alone; add a separate `run_with_backend_and_events<B, I>` that duplicates the event-loop body. Less DRY but zero refactor risk to the hot path. **Recommend the seam-extract form above** — production code becomes a thin wrapper, test code exercises the same loop.

**Warning signs:** a planner proposing to `Ctrl-C` a child-process TUI — impossible reliably across CI OSes. Or proposing to mock `crossterm::event` — it's a concrete module, not a trait.

### Pitfall 5: `tests/persist.rs` races with `tests/tui.rs` over `XDG_STATE_HOME`
**What goes wrong:** Both tests mutate `XDG_STATE_HOME`. Without `#[serial(env)]` on both, `cargo test --test-threads=8` (Phase 2 CI smoke step per D-15) races.

**Why it happens:** Integration tests in different files run in parallel by default. `serial_test` locks are process-local via mutex on a named key.

**How to avoid:** Every test function in `tests/persist.rs` AND `tests/tui.rs` bears `#[serial(env)]` (the Phase 2 shared key). The env-discipline xtask gate (`crates/xtask/tests/env_discipline.rs`) currently walks `base60-cli/src/` and `base60-core/src/` — it does NOT walk `base60-cli/tests/`. This means the gate won't catch a missing `#[serial(env)]` in integration tests.

**Planner action:** Planner must verify manually. If the gate should be extended to walk `tests/`, that's a Phase 2 gate-scope change (out of Phase 4 scope). **Inside Phase 4**, the planner's acceptance checklist for Plans 04-03 and 04-04 includes: "every `env::set_var` / `env::remove_var` in tests/*.rs is inside a `#[serial(env)]`-annotated test."

**Warning signs:** a `tests/persist.rs` function that reads `std::env::var_os("XDG_STATE_HOME")` but lacks `#[serial(env)]` — the read races with another test's write.

### Pitfall 6: `fnv1a` hash of canonicalized path means state-file path varies per CI machine
**What goes wrong:** `persist::state_file` (persist.rs:42) calls `fs::canonicalize(input)` — on one runner it yields `/tmp/.tmp123/fixture.bin`, on another `/private/tmp/.tmp123/fixture.bin` (macOS symlink). The FNV-1a output differs; the test's `expected state path` hardcode fails on macOS.

**How to avoid:** Plan 04-04 MUST resolve the state path by calling `persist::state_file(&tempfile_path)` itself, not by hardcoding a hash value. `persist::state_file` is `pub(crate)` at persist.rs:42 — integration test can access it via `base60::` library re-exports OR by exposing a `tests/` view into the module. **Recommend making `persist::state_file` re-exported through `lib.rs`** (only if needed for the test; otherwise the test can shell-scan `$XDG_STATE_HOME/base60/*.state` for the single matching file).

**Simpler alternative:** after driving the TUI to quit, glob `$XDG_STATE_HOME/base60/*.state` — there's exactly one (single fixture, single invocation), read it, assert contents. Avoids re-exporting private API.

**Warning signs:** test code with a hardcoded 16-char hex hash — platform-dependent fragility.

### Pitfall 7: Windows `&[u8]` → `OsStr` round-trip differs from Unix
**What goes wrong:** `persist::state_file` calls `canonical.as_os_str().as_encoded_bytes()` — on Windows the bytes are WTF-8 encoded, not identical to the Unix path bytes. The FNV-1a hash on the same logical path differs per OS. A test that works on Ubuntu breaks on Windows.

**How to avoid:** This is inherent to cross-platform persistence; it doesn't need fixing. Plan 04-04's test reads whatever state file appears — it doesn't assume a specific path pattern.

**Warning signs:** test assertions on the literal 16-char hex hash string. Use glob-or-directory-scan instead.

### Pitfall 8: `decode_stream`'s existing `BufRead` signature doesn't allow peek-then-re-read
**What goes wrong:** Format-sniff requires reading the first line to determine format. But `BufRead` exposes `lines()` as an iterator that consumes; once consumed, the rest of the stream is still available, but the first line is gone. The ansi/plain decoder needs that first line to be part of the byte stream (it's an early digit-run line).

**How to avoid:** Plan 04-01 uses `BufRead::read_line(&mut first_line)?` + a local state machine: process `first_line` as the first record of whichever format won the sniff, then continue with the remainder via `lines()` or direct reads. Alternative: wrap the reader in `io::Cursor<String>` for the first line + original `BufRead` for the rest via `Read::chain`.

**Recommended:** Buffer the first line, sniff, then construct a small helper that feeds the buffered first line followed by the remaining `BufRead`. `std::io::Chain` does this cleanly: `first_line.as_bytes().chain(remaining_reader)`.

**Warning signs:** a planner proposing to `fs::read_to_string` the whole input — kills streaming for large dump files.

### Pitfall 9: JSON per-chunk `"bytes"` field already carries length; meta line is redundant
**What goes wrong:** `emit_json` (format.rs:33-79) already writes `"bytes":[72,105,...]` with the actual chunk byte count (line 49 iterates `chunk.iter()`, not `pad_chunk`'s zero-filled form). JSON roundtrip doesn't NEED the meta line — decoder can sum `bytes[]` lengths across all lines.

**Why it matters:** D-01 requires `{"type":"meta","bytes":N}\n` anyway — for uniformity. Decoder uses it as a sanity check; mismatch is a stderr warning (D-08-equivalent tolerance), not an error.

**How to avoid:** Plan 04-01 sanity-check logic is: `if meta.bytes != accumulated_output.len() { stderr_warn; continue }`. Exit 0 regardless. Matches the general D-03 / D-08 tolerance policy.

### Pitfall 10: Adding `--input-format` to `DecodeArgs` must not accidentally change `ViewArgs.format`
**What goes wrong:** `cli.rs:119` defines `enum Format { Ansi, Plain, Json, Html }` for the OUTPUT format. A new `InputFormat` for the decoder INPUT is needed. Naming collision risk: `InputFormat::Json` vs `Format::Json`.

**How to avoid:** Distinct enums with distinct names. `InputFormat` includes `Auto` (the default); `Format` does not. Per D-06: `#[clap(value_enum, default_value_t = InputFormat::Auto)]` on `DecodeArgs.input_format`.

**Warning signs:** a planner reusing `Format` for decode input — Auto isn't an output format.

## Code Examples

Verified patterns from in-repo sources and Context7 ratatui docs.

### Example 1: `assert_cmd` full-message error pin (D-10)
```rust
// Source: expand crates/base60-cli/tests/cli.rs:155-167
#[test]
fn decoder_invalid_digit_99_error_contains_the_digit() {
    let dump = "00000000  00:00:00:00:00:00:00:00:00:00:99  |........|\n";
    base60_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "line 1: invalid base-60 digit 99 at pair 11",
        ));
}
```

### Example 2: Pair-1 error pin (D-11)
```rust
#[test]
fn decoder_invalid_digit_at_pair_1_reports_pair_1() {
    // First pair is `99` (hi=9, lo=9 → digit=99 ≥ 60). Rest doesn't matter
    // because parse_run returns on the first invalid digit.
    let dump = "00000000  99:00:00:00:00:00:00:00:00:00:00  |........|\n";
    base60_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "line 1: invalid base-60 digit 99 at pair 1",
        ));
}
```

### Example 3: Non-digit-run-lines-ignored (D-11, Pitfall 3 coverage)
```rust
#[test]
fn decoder_ignores_non_digit_run_lines() {
    // Free text that doesn't match the 11-pair run shape is skipped silently.
    // If REF-03 incorrectly collapses "no digit run found" into an error,
    // this test fails. Pins find_digit_run's tolerance (decode.rs:44-60).
    let garbage = "some prefix\n# bytes=0x10\nhello world\n\n";
    base60_cmd()
        .arg("decode")
        .write_stdin(garbage)
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}
```

### Example 4: Tempfile-backed mmap test (TEST-05, Plan 04-03)
```rust
// Source: crates/base60-cli/tests/reader.rs (NEW)
use std::io::Write;

#[test]
fn load_file_via_mmap_returns_file_contents() {
    // Source: docs.rs/tempfile 3 — NamedTempFile pattern
    let mut tmp = tempfile::NamedTempFile::new().expect("mktemp");
    tmp.write_all(b"hello world").expect("write");
    tmp.flush().expect("flush");

    // Exercise the mmap path: load() picks load_file() when path is Some.
    // reader::load is pub(crate); integration test accesses it via the
    // spawned `base60` binary rather than direct import.
    //
    // Alternative (direct): expose reader::load via `base60::` — reject
    // because it widens the library surface (Phase 3 D-07 narrow-surface).
    //
    // Chosen: spawn `base60 --color=never --format=plain FILE` and assert
    // output contains the bytes rendered. This exercises load_file in
    // its actual usage context.
    base60_cmd()
        .args(["--color=never", "--format=plain"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("|hello wo|"));
}
```

**Note for planner:** This tests `load_file` as a black-box via the CLI. A whiter-box alternative would directly invoke `reader::load(Some(path), 0, None)` — but `reader::load` is `pub(crate)` and integration tests are external crates. Two options: (a) make `reader::load` `pub` and re-export via `base60::`, widening the library surface (reject per Phase 3 D-07); (b) black-box via spawn. **Recommend option (b)** — preserves encapsulation and tests the actual user-facing behaviour.

### Example 5: `io::Cursor<Vec<u8>>` synthetic BufRead stdin test
```rust
// Source: crates/base60-cli/tests/reader.rs (NEW)
// For load_stdin coverage, we have two options:
// 1. Spawn `base60 -` with `.write_stdin(bytes)` (black-box via assert_cmd).
// 2. Construct io::Cursor<Vec<u8>> and call reader::load(None, 0, None) with
//    stdin redirected — requires mocking `std::io::stdin()`, which is
//    process-global and can't be mocked cleanly.
//
// Option 1 is the sane choice. reader::load_stdin (reader.rs:61-68) reads
// from real stdin via std::io::stdin(). In tests, assert_cmd's .write_stdin()
// is the canonical way to feed bytes to the child's stdin.

#[test]
fn load_stdin_via_write_stdin_dumps_piped_bytes() {
    base60_cmd()
        .args(["--color=never", "--format=plain"])
        .write_stdin(&b"piped!\n"[..])
        .assert()
        .success()
        .stdout(predicates::str::contains("|piped!.|"));
}
```

**Update on 04-CONTEXT Claude's Discretion:** the CONTEXT mentions "`io::Cursor<Vec<u8>>` vs writing a tiny fake `BufRead`" — both refer to the inner `read_to_end` inside `load_stdin`. In practice `assert_cmd::Command::write_stdin()` is cleaner than reaching into `reader::load_stdin` directly. Planner should prefer spawn-based coverage unless a seam-extract to `load_stdin_from<R: Read>(r: R, skip, length)` is ALSO valuable for Phase 6 streaming work — and even then, Phase 6 can add the seam when it's needed.

### Example 6: File-open-error path
```rust
#[test]
fn load_file_nonexistent_returns_error() {
    base60_cmd()
        .args(["--color=never", "--format=plain"])
        .arg("/definitely/does/not/exist/nope.bin")
        .assert()
        .failure()
        .stderr(predicates::str::contains("open").and(predicates::str::contains("nope.bin")));
}
```
The `anyhow::Context` chain at reader.rs:52 produces "open /definitely/does/not/exist/nope.bin" — the test asserts both substrings.

### Example 7: TUI TestBackend drive (Plan 04-04, sketch)
```rust
// Source: crates/base60-cli/tests/tui.rs (NEW). Requires seam-added run_with_terminal.
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, KeyEventKind, KeyEventState};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
#[serial(env)]
fn tui_quit_with_save_writes_expected_state_file() {
    let tmpdir = tempfile::tempdir().expect("tempdir");
    // SAFETY: #[serial(env)] guarantees no concurrent env access.
    unsafe { std::env::set_var("XDG_STATE_HOME", tmpdir.path()) };

    // Fixture file. Mmap-backed so persist::state_file gets a canonical path.
    let mut fixture = tempfile::NamedTempFile::new().expect("mktemp fixture");
    fixture.write_all(&vec![0_u8; 8 * 100]).expect("write");
    fixture.flush().expect("flush");

    // Driver events: j j j j j + b1 + q (close bracket/bookmark 1/quit).
    // `b` + `1` uses the BookmarkSet mode from tui.rs:361, which accepts
    // ASCII letters — but '1' is a digit, not ascii_alphabetic. So the
    // set is REJECTED (tui.rs:435-438 "bookmarks use a-z, got '1'").
    //
    // CORRECTION: the CONTEXT specifies `b1` but the TUI's
    // bookmark slots are a-z (tui.rs:358-362 — `m` + letter). Planner
    // MUST use `m` + letter, not `b` + digit. Drive sequence:
    //   j, j, j, j, j, m, a, q   (5× cursor-down, bookmark-set slot 'a',
    //                              then quit-with-save)
    let events = vec![
        key('j'), key('j'), key('j'), key('j'), key('j'),
        key('m'), key('a'),
        key('q'),
    ];
    let mut ev_iter = events.into_iter();

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    tui::run_with_terminal(
        &mut terminal,
        &fixture.as_file().metadata().unwrap(), // fake data slice
        0, LensMode::None, TimeScale::Gar, false,
        Some(fixture.path()),
        || Ok(ev_iter.next()),
    ).expect("tui run");

    // State file: persist::state_file returns $XDG_STATE_HOME/base60/<hash>.state.
    // Plan: glob the directory, read the single file, assert contents.
    let state_dir = tmpdir.path().join("base60");
    let mut entries: Vec<_> = std::fs::read_dir(&state_dir).unwrap()
        .filter_map(Result::ok).collect();
    assert_eq!(entries.len(), 1, "expected one state file");
    let contents = std::fs::read_to_string(entries.pop().unwrap().path()).unwrap();

    // Deterministic fields per persist::serialize (persist.rs:95-113):
    //   scroll=<N>\ncursor=<N>\nlens=—\nbookmarks=a:<byte>\n
    // With 5 j presses: cursor = 5 × CHUNK = 40. scroll depends on
    // view_rows (22 for 80×24 minus 2 border rows), so cursor stays
    // visible; scroll_into_view keeps scroll = 0.
    assert!(contents.contains("cursor=40"));
    assert!(contents.contains("bookmarks=a:40"));

    unsafe { std::env::remove_var("XDG_STATE_HOME") };
}

fn key(c: char) -> Event {
    Event::Key(KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}
```

**Critical correction from the CONTEXT hint:** 04-CONTEXT line 180 + "Claude's Discretion" line 82 mention `b1` (bookmark slot 1). Actual TUI bookmark mode (tui.rs:361 + 428-454) uses `m` + letter (a-z), not `b` + digit. **The hint is wrong**; planner MUST use `m<letter>`. Correct drive sequence: `j j j j j m a q`.

### Example 8: `persist::state_base_dir` ladder test (Plan 04-04)
```rust
// Source: crates/base60-cli/tests/persist.rs (NEW)
// state_base_dir is pub(crate), not pub. Integration test can't call it
// directly. Two options:
//  (a) Re-export via lib.rs — widens library surface.
//  (b) Black-box via spawning `base60 -i FILE` and observing WHERE the
//      state file lands.
// Recommend (b) — keeps persist.rs internals internal. The test IS
// exactly the XDG → HOME fallback behaviour observable by a user.

#[test]
#[serial(env)]
fn state_goes_to_xdg_when_set() {
    let tmpdir = tempfile::tempdir().unwrap();
    unsafe { std::env::set_var("XDG_STATE_HOME", tmpdir.path()) };
    unsafe { std::env::remove_var("HOME") };

    drive_tui_to_quit_with_fixture(/* ... */);

    assert!(tmpdir.path().join("base60").is_dir(),
        "state dir should be under XDG_STATE_HOME/base60/");
    unsafe { std::env::remove_var("XDG_STATE_HOME") };
}

#[test]
#[serial(env)]
fn state_falls_back_to_home_when_xdg_unset() {
    let home_tmp = tempfile::tempdir().unwrap();
    unsafe { std::env::remove_var("XDG_STATE_HOME") };
    unsafe { std::env::set_var("HOME", home_tmp.path()) };

    drive_tui_to_quit_with_fixture(/* ... */);

    assert!(home_tmp.path().join(".local/state/base60").is_dir(),
        "state dir should be under $HOME/.local/state/base60/");
    unsafe { std::env::remove_var("HOME") };
}

#[test]
#[serial(env)]
fn state_noops_when_both_unset() {
    unsafe { std::env::remove_var("XDG_STATE_HOME") };
    unsafe { std::env::remove_var("HOME") };
    // state_base_dir returns None; persist::save silently drops the write.
    // No crash; no state file anywhere. The TUI exits cleanly.
    drive_tui_to_quit_with_fixture(/* ... */);
    // Nothing to assert except successful drive-to-quit.
}
```
`drive_tui_to_quit_with_fixture` is the shared helper used by both `tests/tui.rs` and `tests/persist.rs`; planner may put it in `tests/common/mod.rs` or keep it local to each file.

**Windows caveat:** `$HOME` on Windows is usually `%USERPROFILE%`, which is NOT what persist.rs reads (line 78 reads `HOME` literally). Plan 04-04 planner must check whether this test pattern works on Windows CI — likely yes because we set `HOME` explicitly. If not, gate with `#[cfg(not(windows))]`.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `parse_run(run: &str, line_no: usize)` | `parse_run(run: &[u8; RUN_LEN], line_no: usize)` | Phase 4 REF-03 | Callers can't bypass the length invariant; digit-validity moves inside. |
| Dump emits padded chunks; decode always outputs 8 bytes | Dump emits length trailer; decode truncates to real length | Phase 4 REF-04 | Roundtrip byte-identical for non-8-aligned inputs. Matrix 28 → 140 cells. |
| `decode` accepts only ansi/plain | `decode` auto-detects ansi/plain/json/html, with `--input-format` override | Phase 4 REF-04 | JSON and HTML outputs roundtrip without a format conversion step. |
| Test decoder error with loose `contains("99").and(contains("invalid"))` | Full-message pin + 2-3 position tests | Phase 4 REF-03 (Plan 04-02) | Pitfall 8 — no silent error-semantics drift. |
| No tests for reader/TUI-save/persist | Integration tests via tempfile + TestBackend | Phase 4 TEST-05 | Closes the last untested paths (ROADMAP Phase 4 SC2-4). |
| `tempfile` deferred | Added as dev-dep in Plan 04-03 | This phase | First need is mmap fixture; reused in Plan 04-04. |

**Deprecated/outdated:**
- None. All changes are additive + signature tightening. PROJECT.md line 114 stability contract preserved (`# bytes=` trailer is additive; old dumps still decode; old CLI writes no trailer but new decoder tolerates that).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `tempfile = "3"` latest is 3.27.0 as of 2026-04-24 | Standard Stack | Low — caret `"3"` pins major; any 3.x works on MSRV 1.95. Verified via `cargo search tempfile --limit 1`. |
| A2 | CONTEXT's `b1` hint for TUI bookmark is incorrect; correct is `m<letter>` | Common Pitfalls / Example 7 | Medium — if planner follows `b1` verbatim, the TUI's bookmark handler rejects digit (tui.rs:435-438) and the test asserts nothing meaningful. **Verified by reading tui.rs:358-362 + tui.rs:428-454.** |
| A3 | `serde_json` should NOT be added as a direct dep even though it's transitively present | Alternatives Considered / Don't Hand-Roll | Low — purely stylistic/philosophical; hand-rolled decoder is ~30 LOC. Matches the emitter precedent (format.rs:31). User may override during planner checkpoint. |
| A4 | HTML decoder's state machine must be tightly coupled to `emit_html`'s exact tag shapes | D-05 / Pattern 2 / Don't Hand-Roll | Low — any general-purpose HTML parser is overkill; emitter is the only source of HTML dumps. Coupling doc is in the module comment. |
| A5 | The TUI's `ratatui::run(...)` + `event::read()` must be refactored into `run_with_terminal<B, F>` to be testable | Pitfall 4 / Example 7 | **Medium** — if planner chooses a duplicated-body approach instead, code-review may push back on DRY. Seam-extract form is cleaner; both are viable. |
| A6 | Integration tests in `tests/*.rs` are NOT walked by the xtask env_discipline gate | Pitfall 5 | Low — confirmed by reading `xtask/tests/env_discipline.rs:17` (walks `../base60-core/src` + `../base60-cli/src` only). Planner must eyeball-check `#[serial(env)]` on new test files. |
| A7 | Black-box spawn-based coverage is preferable to white-box reader::load() widening | Example 4 / Example 5 | Low — consistent with Phase 3 D-07 narrow-surface intent. Also mirrors Phase 3 Plan 03-03 edge-case tests that black-box via `base60_cmd()`. |
| A8 | `persist::state_file(&tempfile_path)` returns a canonical-path-based hash; test can resolve the state file via directory scan rather than hardcoded hash | Pitfall 6 | Low — verified by reading persist.rs:42-46 + persist.rs:84-93. Directory-scan is cross-platform safe. |
| A9 | `cargo search` and `Cargo.lock` are accurate sources for "what's available now" | Standard Stack | Low — registry data verified as-of session date. |
| A10 | `#` prefix guarantees trailer is not confused with a digit run by `find_digit_run` | Pitfall 1 / D-04 | Low — verified by reading decode.rs:63-77 (ascii-digit OR colon check; `#` fails both). |

**If this table is empty:** N/A — 10 assumptions. Most are Low risk; A2 and A5 are Medium and flagged for planner attention.

## Open Questions

1. **Should `persist::state_file` be re-exported via `lib.rs` for the TUI test to use, or should the test directory-scan?**
   - What we know: `state_file` is `pub(crate)`. Directory-scan works on one-test-one-fixture but is slightly brittle.
   - What's unclear: will Phase 5's fuzz targets need `state_file` too? (If yes, re-export now.)
   - Recommendation: **directory-scan** (Example 8). Keeps library surface at `pub use cli::{Format, LensMode};` per Phase 3 D-07. Revisit if a future phase needs `state_file` publicly.

2. **Does the TUI seam-extract (Pitfall 4 suggestion) count as "scope creep" in Plan 04-04?**
   - What we know: the seam enables the test; without it, the test is impossible.
   - What's unclear: is adding a testable seam a refactor (would need its own plan) or a prerequisite (stays inside Plan 04-04)?
   - Recommendation: **inside Plan 04-04**. The seam is the minimum viable change for TEST-05 SC3. Planner should call out the refactor in the plan description and verify the full D-24 gate stays green.

3. **What's the exact stderr-warning wording for the legacy-no-trailer fallback (D-03)?**
   - Claude's Discretion per CONTEXT. Recommended: `"decode: no length metadata; assuming input was 8-byte-aligned. Last chunk may contain zero-padding. Regenerate the dump with base60 v2+ to silence this warning."`
   - Decision deferred to planner, but test MUST assert on a substring of whatever wording ships.

4. **Should the JSON meta-line check be a warning, an error, or silent?**
   - CONTEXT says warning + continue (D-08 tolerance). Agree. Planner confirms in Plan 04-01.

5. **Is `--input-format=plain` distinct from `--input-format=ansi` for the decoder?**
   - What we know: ansi and plain share the SAME decode path (both are `NN:NN:…:NN` text; ANSI escapes are consumed by `find_digit_run`'s tolerance). Functionally identical.
   - Recommendation: keep both values in the enum for UI consistency with `Format`, but dispatch to the same `decode_from_text` internal helper. Alternative: collapse into one `Text` variant. Planner's call — **keep distinct values for symmetry** with `Format::Ansi` / `Format::Plain`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` (build, test) | All plans | ✓ | 1.95.0+ (MSRV) | — |
| `rustc` | All plans | ✓ | 1.95.0+ | — |
| `cargo install tempfile` NOT needed (crate, not tool) | Plan 04-03 | N/A — resolved by `cargo test --locked` | 3.x via `"3"` caret | — |
| Filesystem write access to `/tmp` (tempfile) | Plans 04-03/04 | ✓ on all 3 CI OSes (tempfile docs) | — | `$TMPDIR` env override auto-detected |
| Crossterm (for `ratatui::Terminal` init — not used in tests; `TestBackend` is independent) | Plan 04-04 | ✓ | 0.29.0 (already a dep) | — |
| `ratatui::backend::TestBackend` | Plan 04-04 | ✓ | 0.30.0 | — (feature is crate-built-in) |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** None — all infrastructure is either already-present or a crate-registry dep.

## Validation Architecture

> Phase 4 config enables `nyquist_validation: true`. This section is the authoritative mapping consumed by `/gsd-plan-phase` step 5.5 to generate `04-VALIDATION.md`.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `libtest` (`#[test]` / `#[cfg(test)]`) + `assert_cmd 2.2.1` + `predicates 3.1.4` + `serial_test 3.4.0` + `tempfile 3` (NEW) |
| Config file | none (workspace-level `[workspace.lints]` in root `Cargo.toml`; per-crate `[lints] workspace = true`) |
| Quick run command | `cargo test -p base60 --tests --locked` (integration tests only) |
| Full suite command | `cargo test --workspace --all-targets --locked && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REF-04 | dump emits `# bytes=0x<hex>\n` trailer (ansi/plain) | unit | `cargo test -p base60 --lib dump::tests::` | ❌ Wave 0 add: `dump_emits_length_trailer_on_ansi`, `dump_emits_length_trailer_on_plain` |
| REF-04 | `emit_html` emits `<!-- bytes=0x<hex> -->` before `</body>` | unit | `cargo test -p base60 --lib format::tests::html_document_includes_length_comment` | ❌ Wave 0 add |
| REF-04 | `emit_json` emits `{"type":"meta","bytes":N}` final line | unit | `cargo test -p base60 --lib format::tests::json_emits_meta_line_at_end` | ❌ Wave 0 add |
| REF-04 | `decode_from_json` consumes ndjson and writes bytes | unit | `cargo test -p base60 --lib decode::tests::json_roundtrip_inline` | ❌ Wave 0 add |
| REF-04 | `decode_from_html` consumes html and writes bytes | unit | `cargo test -p base60 --lib decode::tests::html_roundtrip_inline` | ❌ Wave 0 add |
| REF-04 | Auto-detect picks correct decoder from first line | unit | `cargo test -p base60 --lib decode::tests::auto_detect_*` | ❌ Wave 0 add (3 tests: ansi, json, html) |
| REF-04 | `--input-format=html` overrides sniff | integration | `cargo test -p base60 --test cli decode_respects_input_format_override` | ❌ Wave 0 add in `tests/cli.rs` |
| REF-04 | Legacy dump (no trailer) produces stderr warning, exits 0 | integration | `cargo test -p base60 --test cli decode_legacy_no_trailer_warns_and_continues` | ❌ Wave 0 add |
| REF-04 | 140-cell matrix all green | integration | `cargo test -p base60 --test roundtrip roundtrip_matrix_byte_identical` | ✓ exists (28 cells today; flips to 140 in Plan 04-01) |
| REF-03 | `parse_run(&[u8; RUN_LEN], usize)` signature | compile-time | `cargo check -p base60` | ✓ compiler catches drift |
| REF-03 | Full error-message pin: `"line 1: invalid base-60 digit 99 at pair 11"` | integration | `cargo test -p base60 --test cli decoder_invalid_digit_99_error_contains_the_digit` | ✓ exists; Plan 04-02 tightens the assert |
| REF-03 | Pair-1 error reports "at pair 1" | integration | `cargo test -p base60 --test cli decoder_invalid_digit_at_pair_1_reports_pair_1` | ❌ Wave 0 add |
| REF-03 | Pair-5 error reports "at pair 5" | integration | `cargo test -p base60 --test cli decoder_invalid_digit_at_pair_5_reports_pair_5` | ❌ Wave 0 add |
| REF-03 | Non-digit-run lines ignored (exit 0, no output) | integration | `cargo test -p base60 --test cli decoder_ignores_non_digit_run_lines` | ❌ Wave 0 add |
| TEST-05 | `load_file` mmap path exercised | integration | `cargo test -p base60 --test reader load_file_via_mmap_returns_file_contents` | ❌ Wave 0 add (new file `tests/reader.rs`) |
| TEST-05 | `load_stdin` path exercised | integration | `cargo test -p base60 --test reader load_stdin_via_write_stdin_dumps_piped_bytes` | ❌ Wave 0 add |
| TEST-05 | `load_file("/nonexistent")` returns error with context | integration | `cargo test -p base60 --test reader load_file_nonexistent_returns_error` | ❌ Wave 0 add |
| TEST-05 | TUI exit-with-save writes expected state file | integration | `cargo test -p base60 --test tui tui_quit_with_save_writes_expected_state_file` | ❌ Wave 0 add (new file `tests/tui.rs`) |
| TEST-05 | `state_base_dir` uses XDG when set | integration | `cargo test -p base60 --test persist state_goes_to_xdg_when_set` | ❌ Wave 0 add (new file `tests/persist.rs`) |
| TEST-05 | `state_base_dir` falls back to HOME when XDG unset | integration | `cargo test -p base60 --test persist state_falls_back_to_home_when_xdg_unset` | ❌ Wave 0 add |
| TEST-05 | `state_base_dir` returns None / no-ops when both unset | integration | `cargo test -p base60 --test persist state_noops_when_both_unset` | ❌ Wave 0 add |

### Sampling Rate
- **Per task commit:** `cargo test -p base60 --tests --locked` (integration tests only, ~15s)
- **Per wave merge:** `cargo test --workspace --all-targets --locked` (~2 min local)
- **Phase gate:** Full suite green: `cargo test --workspace --all-targets --locked && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo fmt --all --check && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` before `/gsd-verify-work`.

### Wave 0 Gaps
All integration test files MUST be created in Wave 0 before the implementation that satisfies them lands. Per the CONTEXT's commit order (D-12):

- [ ] `crates/base60-cli/tests/reader.rs` — covers TEST-05 reader items (Plan 04-03)
- [ ] `crates/base60-cli/tests/tui.rs` — covers TEST-05 TUI item (Plan 04-04)
- [ ] `crates/base60-cli/tests/persist.rs` — covers TEST-05 persist items (Plan 04-04)
- [ ] New unit tests inside `decode.rs` for json/html roundtrip + auto-detect (Plan 04-01) — inline `#[cfg(test)] mod tests` per the crate's convention
- [ ] New unit tests inside `dump.rs` and `format.rs` for length-trailer emission (Plan 04-01)
- [ ] Expanded `tests/cli.rs` error-pin + input-format-override cases (Plan 04-01 for override; Plan 04-02 for error pins)
- [ ] `tempfile = "3"` dev-dep (Plan 04-03 commit; Plan 04-04 uses it too but doesn't re-add)

### 8 Nyquist Validation Dimensions × Phase 4 Plan Coverage

| Dimension | What to validate | Covered by Plan | Evidence |
|-----------|------------------|-----------------|----------|
| **Correctness** | `dump | decode` byte-identical for all `LensMode × Format × Fixture` cells | 04-01 (REF-04) | 140-cell matrix in `tests/roundtrip.rs`; 7 × 4 × 5 fixtures; every cell spawns base60 twice and diffs. |
| **Contract** | Public fn signatures + error-message shapes are stable | 04-02 (REF-03) | `parse_run` array-typed at compile time (D-09); full-message stderr pin in `tests/cli.rs` (D-10); pair-position tests (D-11). Also: `base60::{Format, LensMode}` public re-exports unchanged. |
| **Error-path** | Malformed input + missing files + mismatched length metadata produce correct Err/warnings, exit codes, and stderr | 04-01 (legacy fallback, meta mismatch) + 04-02 (invalid digit, overflow) + 04-03 (nonexistent file) | `decode_legacy_no_trailer_warns_and_continues`, `decoder_invalid_digit_*`, `load_file_nonexistent_returns_error`, JSON meta-mismatch warning test in Plan 04-01. |
| **Integration** | End-to-end `base60 FILE | base60 decode` roundtrip across every format; TUI exits cleanly and persists state; reader + decoder + CLI dispatch agree | 04-01 (matrix widen) + 04-04 (TUI integration) + 04-03 (reader integration) | 140-cell matrix; `tui_quit_with_save_writes_expected_state_file`; black-box `load_file_*` via spawn. |
| **Performance** | No regression in decode/dump throughput; mmap still exercised for large inputs | implicit — no perf changes in Phase 4 | Phase 6 owns perf via PERF-01..05. This phase's new code paths MUST NOT materialise full input into Vec where the current code streams (decode.rs:30 `decode_stream<R: BufRead>` must stay streaming). Planner code-review checkpoint only. |
| **Regression** | Existing 182 inline tests + 28-cell matrix stay green between each commit (D-17) | All plans | D-24 gate per-commit: `cargo test --workspace --all-targets --locked`. Existing decoder tests (decode.rs:131-200) stay green after REF-03's signature change — they assert on `decode` helper that parses and compares bytes. |
| **Observability** | stderr warnings are detectable + stable wording | 04-01 (legacy fallback + meta mismatch) | `.stderr(predicates::str::contains("no length metadata"))` — exact wording TBD at plan time but pinned by test. |
| **Operability** | `base60 decode --help` lists the new `--input-format` flag; `base60 decode --input-format=html < file.html` works | 04-01 | `assert_cmd` test: `base60 decode --help` stdout contains `--input-format`; integration test drives `--input-format=html`. Also: the xtask spawn-discipline and env-discipline gates pass, meaning every new test file uses `base60_cmd()` + `#[serial(env)]` where applicable. |

All 8 dimensions are covered. The weakest is **Performance** — no specific regression test is added because this phase doesn't change hot-path perf; Phase 6 owns that work. Planner should include a code-review checkpoint item: "decode paths stay streaming; no new `read_to_end` calls."

## Project Constraints (from CLAUDE.md)

- **Rust edition 2024, MSRV 1.95** — every dep (including `tempfile` 3.x) must resolve within MSRV on `cargo +1.95.0 test --workspace --all-targets --locked`.
- **No runtime dependencies added to `base60-core`** — Phase 4 touches ONLY `base60-cli`. Plans 04-01/02/03/04 must not open `base60-core/Cargo.toml` or `base60-core/src/**`.
- **Workspace lints** — `clippy::pedantic + nursery + cargo` with `-D warnings`. Every new `pub(crate)` item needs a doc comment with `# Errors` / `# Panics` sections if it returns `io::Result` / can panic. Every new `#[must_use]` candidate gets the attribute. `#[derive(Debug)]` on every new `pub(crate)` struct/enum.
- **Workspace rust lints** — `unreachable_pub = warn`, `missing_debug_implementations = warn`, `unused_lifetimes = warn`, `unused_qualifications = warn`, `rust_2018_idioms = warn`.
- **`#![forbid(unsafe_op_in_unsafe_fn)]`** on both `main.rs` and `lib.rs` — applies to new test files too if they declare `unsafe fn`; they don't need to, but `unsafe { env::set_var(...) }` blocks are required around every env mutation (Rust 2024 rule), each with a `SAFETY:` comment.
- **No `unwrap()` / `expect()` outside `#[cfg(test)]` or at process-startup** — new decoder helpers return `io::Result`; tests may `.unwrap()` freely (stdlib convention).
- **No `todo!` / `unimplemented!` / `unreachable!`** in shipped code.
- **Saturating / checked arithmetic** — any new `data.len()` → offset conversion uses the `saturating_add` idiom already in use at `dump.rs:127`.
- **`# Errors`/`# Panics` rustdoc** — `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS: -D warnings` will fail on a new `pub(crate)` fn returning `io::Result<()>` without a `# Errors` section.
- **`cargo fmt --all --check` passes on every commit** — rustfmt default settings (no `rustfmt.toml`).
- **Commit granularity — 4 plans, 4 commits minimum (planner may split within a plan per D-17)** — each commit must pass the full D-24 gate before the next starts. No "WIP" or intermediate-broken states.
- **Conventional commit prefixes** — `feat(cli): …` / `refactor(cli): …` / `test(cli): …` matching the REF-04 / REF-03 / TEST-05 labels.
- **`gh` for GitHub interactions** — not relevant for Phase 4 (no PR creation in this phase's scope).

## Sources

### Primary (HIGH confidence)
- `/home/chris/Projects/utils/test-60/.planning/phases/04-tighten-parse-run-close-coverage-gaps/04-CONTEXT.md` — user decisions D-01..D-17 + canonical refs + deferred ideas
- `/home/chris/Projects/utils/test-60/.planning/phases/04-tighten-parse-run-close-coverage-gaps/04-DISCUSSION-LOG.md` — decision rationale
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/decode.rs` — current `parse_run`, `find_digit_run`, `is_digit_run`, `RUN_LEN` (=33), `PAIR` (=2)
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/dump.rs` — current `dump_all`, `write_line`, `styled_line`
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/format.rs` — current `emit_json`, `emit_html`, `HTML_EPILOGUE`, `digit_class`
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/cli.rs` — current `DecodeArgs`, `Format`, `LensMode`, `Format::ALL`
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/reader.rs` — `Bytes`, `load`, `load_file`, `load_stdin`, `clamp_range`
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/persist.rs` — `state_base_dir`, `state_file`, `serialize`, `fnv1a`
- `/home/chris/Projects/utils/test-60/crates/base60-cli/src/tui.rs:53-90` — current `pub(crate) fn run` signature + ratatui::run call
- `/home/chris/Projects/utils/test-60/crates/base60-cli/tests/common/mod.rs` — `base60_cmd()`, `ROUNDTRIP_FIXTURES`, `ROUNDTRIP_FORMATS`, `ALL_LENS_CONFIGS`, `assert_roundtrip`, `spawn_with_closed_stdout`, fixtures module
- `/home/chris/Projects/utils/test-60/crates/base60-cli/tests/cli.rs:155-167` — current loose decoder error pin
- `/home/chris/Projects/utils/test-60/crates/base60-cli/Cargo.toml` — dep versions verified: anyhow 1.0.102, clap 4.6.1, ratatui 0.30.0, assert_cmd 2, predicates 3, serial_test 3
- `/home/chris/Projects/utils/test-60/Cargo.lock:1125-1192, 1348` — ratatui 0.30.0, serde_json 1.0.149 (transitive only)
- `/home/chris/Projects/utils/test-60/crates/xtask/tests/env_discipline.rs:17` — gate walks `base60-core/src` + `base60-cli/src` only; NOT `tests/`
- `/home/chris/Projects/utils/test-60/crates/xtask/tests/spawn_discipline.rs` — gate exempts `common/` path component
- `/home/chris/Projects/utils/test-60/.planning/research/PITFALLS.md §"Pitfall 8"` — error-semantics drift remediation strategy (full-message pin)
- `/home/chris/Projects/utils/test-60/.planning/research/PITFALLS.md §"Pitfall 1"` — `#[serial(env)]` single-key requirement
- Context7 — `/websites/rs_ratatui` — `TestBackend::new(u16, u16)`, `Terminal::new(backend)`, `Terminal::draw`, `ratatui::run` signature

### Secondary (MEDIUM confidence)
- `cargo search tempfile --limit 1` — confirmed 3.27.0 is latest as of 2026-04-24 (value used in Standard Stack)
- `cargo search ratatui --limit 1` — confirmed 0.30.0 (matches Cargo.lock)
- `cargo search assert_cmd --limit 1` — confirmed 2.2.1 (matches `cargo tree` output)
- `cargo tree -p base60 --depth 1` — confirmed direct + dev-dep graph

### Tertiary (LOW confidence)
- None. Every claim is either directly cited from in-repo source, a canonical reference document in `.planning/`, or Context7-verified docs.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every version confirmed against `Cargo.lock` or `cargo search` + `cargo tree`
- Architecture patterns: HIGH — all source files read, seam requirements verified
- Pitfalls: HIGH — Pitfall 4 (TUI seam) and Pitfall 6 (fnv1a hash) verified by reading `tui.rs:53-90` and `persist.rs:42-93`
- Code examples: HIGH — all examples compile-checkable against the current source; Example 7 corrects a CONTEXT error (`b1` → `ma`) based on TUI code
- Validation architecture: HIGH — test-to-requirement map exhaustive, every REQ-ID has ≥1 covering test

**Research date:** 2026-04-24
**Valid until:** 2026-05-24 (30 days for stable Rust ecosystem; re-verify tempfile version if planning slips past this date)
