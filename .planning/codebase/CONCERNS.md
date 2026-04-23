# Codebase Concerns

**Analysis Date:** 2026-04-23

Workspace is disciplined: zero `TODO`/`FIXME`/`HACK`/`XXX`/`bug:` markers, every numeric cast is annotated, `forbid(unsafe_op_in_unsafe_fn)` on the binary, `#![deny(warnings)]`-style posture via `pedantic + nursery + cargo` clippy profile, and all numeric/encoding hot paths use `saturating_*` or `checked_*` arithmetic. The concerns below are pragmatic risks rather than obvious bugs.

## Tech Debt

**`be_u64` duplicated across two modules:**
- Issue: The big-endian chunk decoder is hand-copied between `crates/base60-cli/src/dump.rs:35` (`fn be_u64`) and `crates/base60-cli/src/format.rs:26` (`fn be_u64`). Module-level docstring at `format.rs:24` explicitly acknowledges the duplication ("Duplicated from `dump::be_u64` because exposing it would blur the line between private renderer internals and a public conversion.").
- Files: `crates/base60-cli/src/dump.rs:35-40`, `crates/base60-cli/src/format.rs:26-31`
- Impact: If `CHUNK` ever changes or the padding rule shifts (e.g. little-endian mode), two copies must move in lockstep. Silent divergence would produce different bytes for the dump column vs. JSON/HTML emitters.
- Fix approach: Move `be_u64` into `base60-core::convert` behind a `pub(crate)`-equivalent re-export, or make it a `pub(super) fn` in a shared `crates/base60-cli/src/chunk.rs` module.

**`cycle`/`label`/`build_lens` dispatch tables are parallel enumerations:**
- Issue: Adding a new [`LensMode`] requires updating three switch statements (`cli.rs:44-52`, `cli.rs:57-65`, `cli.rs:75-89`) plus the `persist::parse_lens` at `persist.rs:139-147`. None are driven from a common source.
- Files: `crates/base60-cli/src/cli.rs:44-89`, `crates/base60-cli/src/persist.rs:139-147`
- Impact: Forgetting one of the four sites will silently drop the new lens from either the TUI cycle, the status label, the CLI flag, or the persistence layer.
- Fix approach: Add a `strum::EnumIter` + method-on-variant pattern, or a compile-time table; alternatively add a test that iterates all variants of `LensMode` through every dispatch function.

**`persist::state_base_dir` has no test coverage:**
- Issue: Explicitly acknowledged at `crates/base60-cli/src/persist.rs:231-237` ("reads process-wide env vars, which races badly with the other env-sensitive tests in this crate (NO_COLOR, NO_UNICODE). Concurrent cargo test would need a shared mutex or a separate test binary. The helper's logic is trivial and covered via manual inspection").
- Files: `crates/base60-cli/src/persist.rs:72-80`
- Impact: A refactor that changes `XDG_STATE_HOME` precedence or the `HOME` fallback would go un-caught by CI.
- Fix approach: Gate with `serial_test::serial`, or factor the env-reading into an injectable `fn (impl Fn(&str) -> Option<OsString>)` so the logic can be unit-tested without touching global state.

**Env-mutation tests serialise by convention, not enforcement:**
- Issue: Multiple tests call `unsafe { std::env::set_var(...) }` / `remove_var` with inline `// SAFETY` comments that say "Cargo runs each `#[test]` on its own thread but within the same process, so tests touching env vars must not run concurrently" (`main.rs:185-209`). No serialisation primitive enforces this — Cargo's default is multi-threaded test execution.
- Files: `crates/base60-cli/src/main.rs:183-219`, `crates/base60-core/src/cuneiform.rs:150-161`, `crates/base60-core/src/lens.rs:321-328`
- Impact: Flaky test failures on high-core-count CI runners where `NO_COLOR` / `NO_UNICODE` state can leak between threads mid-assertion.
- Fix approach: Add `serial_test = "3"` and annotate the env-touching tests with `#[serial(env)]`, or move them into their own `#[cfg(test)] mod env_tests { ... }` behind `--test-threads=1` in CI.

**`Palette` fields rely on `&'static str` rather than writers:**
- Issue: `crates/base60-cli/src/color.rs:20-32` — ANSI escapes are emitted by a dozen `w.write_all(palette.offset.as_bytes())` calls per line in `dump.rs:67-120`. Each is a separate `Write` call; with `PALETTE_NONE`, the empty-slice writes still incur a virtual dispatch through `BufWriter<W>`.
- Files: `crates/base60-cli/src/color.rs:20-68`, `crates/base60-cli/src/dump.rs:56-123`
- Impact: Negligible in practice (documented at `dump.rs:53-54` as "coloured and monochrome paths share one code path without a runtime branch per token"), but complicates future 256-colour / truecolor extensions where the escape sequence is not static.
- Fix approach: If truecolor is ever added, switch `Palette` to a trait with `fn write_offset<W: Write>(&self, w: &mut W) -> io::Result<()>` so the no-op branch can monomorphise away entirely.

## Known Bugs

None identified. Every `unwrap`/`expect` in non-test code is gated by a preceding invariant check or an explicit alphabet/range guarantee; every potential overflow uses `checked_*` or `saturating_*`.

## Security Considerations

**`unsafe { Mmap::map(&file) }` — TOCTOU on backing file:**
- Risk: Another process mutating the file while the viewer holds the `Mmap` can violate Rust's aliasing rules on the borrowed `&[u8]`. Explicitly acknowledged at `crates/base60-cli/src/reader.rs:53-55` ("the worst outcome is stale bytes on screen, which is acceptable").
- Files: `crates/base60-cli/src/reader.rs:51-59`
- Current mitigation: Read-only mapping (`Mmap::map`, not `MmapMut`); file open uses `File::open`, not `OpenOptions::write(true)`.
- Recommendations: Acceptable for a viewer. If the tool ever gains a streaming hash/CRC feature whose output is signed or committed, prefer `std::fs::read` or a `--no-mmap` fallback to avoid presenting torn bytes as authoritative.

**`unsafe { std::env::set_var / remove_var }` in tests only:**
- Risk: Rust 2024 edition marks env mutation `unsafe`. The four call sites are all `#[cfg(test)]` or inside `#[test]` functions, so no production path ever mutates the environment.
- Files: `crates/base60-cli/src/main.rs:191`, `crates/base60-cli/src/main.rs:198`, `crates/base60-cli/src/main.rs:205`, `crates/base60-cli/src/main.rs:208`, `crates/base60-core/src/cuneiform.rs:154`, `crates/base60-core/src/cuneiform.rs:156`, `crates/base60-core/src/lens.rs:324`, `crates/base60-core/src/lens.rs:327`
- Current mitigation: `#![forbid(unsafe_op_in_unsafe_fn)]` at `main.rs:1`; workspace-level `unsafe_op_in_unsafe_fn = "warn"` in `Cargo.toml:21`.
- Recommendations: Combine with the test-serialisation note above (`serial_test` crate) so the `unsafe` block is both locally scoped and race-free.

**`persist::state_file` hashes a canonicalised path without collision handling:**
- Risk: FNV-1a is a 64-bit non-cryptographic hash (`crates/base60-cli/src/persist.rs:84-93`). On realistic user libraries the birthday bound (~2^32 files before a 50% collision) is effectively unreachable, but a malicious/adversarial filename could intentionally collide with another viewed file and inherit its cursor/bookmarks.
- Files: `crates/base60-cli/src/persist.rs:42-46`
- Current mitigation: State contents are not security-sensitive (a cursor offset and bookmarks), and a collision merely resumes the "wrong" file's cursor.
- Recommendations: Low priority. If bookmarks ever carry user-entered strings (notes, labels), upgrade to `SipHasher13` (keyed with a per-user salt) or store the full path alongside the hash and verify on load.

**HTML emitter escapes five chars but not byte-order marks or unicode controls:**
- Risk: `crates/base60-cli/src/format.rs:214-226` — `write_html_char` escapes `&<>"'` but passes through U+2028/U+2029 (line/paragraph separator), U+FEFF (BOM), and bidi overrides unchanged. These flow into the `<pre>` body and never close a tag, but a hostile *lens* implementation could return a string that re-opens a script context via exotic unicode.
- Files: `crates/base60-cli/src/format.rs:207-226`
- Current mitigation: All built-in lenses produce a fixed alphabet of ASCII digits, wedge glyphs, and a handful of separators — none contain `<`, `>`, `&`, or quotes. Third-party `impl Lens` is theoretical (the trait is public via `base60-core`, but CLI-side invocation is hardcoded).
- Recommendations: For hardening, treat every lens output as untrusted and emit the `<pre>` body via a `data-*` attribute with attribute-style escaping, or set `Content-Security-Policy` via a `<meta http-equiv>` in the prologue. Not actionable for the current feature set.

## Performance Bottlenecks

**Per-digit `write_all` calls in `dump::write_line`:**
- Problem: `crates/base60-cli/src/dump.rs:75-86` issues up to 11 × 5 = 55 `write_all` calls per line (two palette writes, a byte-pair write, a reset per digit, plus the colon+reset between). Each hits the `BufWriter` but goes through a monomorphised indirect call.
- Files: `crates/base60-cli/src/dump.rs:56-123`
- Cause: Token-level writes are needed for ANSI colour handling; the no-op palette makes most of them zero-byte.
- Improvement path: For the `PALETTE_NONE` path, use a stack-allocated `[u8; 33]` formatter (`11 × 2 + 10 colons + 1 NL`), then a single `write_all`. Measure first with `cargo bench` — on a realistic 1 GB file this is bound by disk/pipe throughput, not CPU.

**`CuneiformLens::render` allocates a fresh `String` per line:**
- Problem: `crates/base60-core/src/lens.rs:175-202` allocates a new `String` (capacity `DIGITS * 20`) on every 8-byte row.
- Files: `crates/base60-core/src/lens.rs:174-203`, `crates/base60-core/src/lens.rs:118-150` (`TabletLens::render` has the same shape)
- Cause: The `Lens::render` trait signature returns `String`, forcing an allocation per call even when the output is always appended to a single `Line`'s worth of output.
- Improvement path: Add a `fn render_to<W: Write>(&self, chunk: u64, w: &mut W) -> io::Result<()>` default method so the streaming writer avoids the allocation. Keep `fn render(&self, chunk: u64) -> String` for the TUI path where `ratatui::Span` needs an owned string.

**`window_entropies` allocates a fresh `[u32; 256]` per window:**
- Problem: `crates/base60-cli/src/analyze.rs:138-149` rebuilds a 1-KiB histogram on the stack per window. A 1 GB input with window 256 → 4 million histograms.
- Files: `crates/base60-cli/src/analyze.rs:138-149`
- Cause: Per-window independence. A sliding-window variant would need an outgoing/incoming byte update per step.
- Improvement path: Keep the current non-overlapping chunks; stack-allocated `[u32; 256]` is L1-resident and the cost is dominated by the byte walk itself. No change required unless profiling shows it's a hotspot.

**`analyze::write_summary` collects every nonzero byte into a `Vec` before sorting:**
- Problem: `crates/base60-cli/src/analyze.rs:225-232` builds a `Vec<(usize, u32)>` of up to 256 entries, sorts it, then takes the top 5.
- Files: `crates/base60-cli/src/analyze.rs:224-243`
- Cause: Clarity over a min-heap of size 5.
- Improvement path: Not worth fixing — 256 entries sort in microseconds and the function runs once per invocation.

**`find_all` uses a naïve O(n·m) scan:**
- Problem: `crates/base60-cli/src/search.rs:102-119` does a byte-slice equality check at every offset. For large inputs and long needles, this approaches worst-case quadratic time; the comment at `search.rs:111` acknowledges non-overlapping advancement but not the scan complexity.
- Files: `crates/base60-cli/src/search.rs:102-119`
- Cause: Needle lengths in practice are short (`"ELF"`, `"cafebabe"`); KMP/Two-Way would add dependency weight for little gain.
- Improvement path: Use `memchr::memmem::find_iter` (zero new transitive deps — `memchr` is already pulled in by `ratatui`/`clap` per `Cargo.lock`). Strictly faster on all inputs.

## Fragile Areas

**`decode::find_digit_run` scans with an O(n) windowed match on every line:**
- Problem: `crates/base60-cli/src/decode.rs:44-60` walks every byte position looking for an exact 34-char run. Any ANSI-escape-interspersed dump (the exact use case the doc advertises — `base60 FILE | base60 decode`) will rely on the left/right boundary guards at lines 81-87 to reject partial matches.
- Files: `crates/base60-cli/src/decode.rs:44-87`
- Why fragile: The layout is load-bearing — if `dump::write_line` ever changes the colon count, digit width, or adds a space inside the run, `RUN_LEN = 34` silently misdetects. `RUN_LEN` is computed from `DIGITS` (`decode.rs:23`) but the `3`-byte stride (`"NN:"`) in `is_digit_run` (`decode.rs:66`) is a magic constant.
- Safe modification: When altering the dump format, update `decode::PAIR`, `decode::RUN_LEN`, and the `i % 3 == 2` colon-position check (`decode.rs:66`) together. Add an end-to-end round-trip test `dump → decode` against a fixture with every lens variant enabled to guard against drift.
- Test coverage: `decode::tests::rejects_twelve_pair_overextension` (`decode.rs:179-184`) covers only the right-boundary path; no test asserts that a dump with an active `--lens` still round-trips.

**`parse_run` arithmetic assumes every pair is exactly 2 ASCII digits:**
- Problem: `crates/base60-cli/src/decode.rs:94-119` does `pair.as_bytes()[0] - b'0'` and `[1] - b'0'` without a second `is_ascii_digit` check. Safe because `find_digit_run` validated upstream at `decode.rs:63-77`, but the two functions are not co-located.
- Files: `crates/base60-cli/src/decode.rs:94-119`
- Why fragile: Any refactor that calls `parse_run` from a new site (e.g. a stdin short-circuit) without running `is_digit_run` first will underflow on non-digit input and produce wrong `u8` values silently.
- Safe modification: Promote the digit check into `parse_run` itself, or take `&[u8; RUN_LEN]` instead of `&str` so the type enforces the length.

**Dual f32/f64 precision juggling in `analyze`:**
- Problem: `shannon_entropy` (`crates/base60-cli/src/analyze.rs:113-135`) computes in `f64`, truncates to `f32`, then clamps to `[0.0, 8.0]`. The clamp exists because of IEEE rounding on near-boundary values; the `#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]` at `analyze.rs:120`, `analyze.rs:132`, `analyze.rs:281`, `analyze.rs:300` silences the linter.
- Files: `crates/base60-cli/src/analyze.rs:113-135`, `crates/base60-cli/src/analyze.rs:268-306`
- Why fragile: Removing the `clamp(0.0, 8.0)` looks like a no-op cleanup but would expose values slightly > 8.0 from `f64→f32` rounding on uniform inputs, failing `uniform_byte_distribution_approaches_eight_bits` stochastically.
- Safe modification: Keep the clamp. If ever inlining a sliding-window version, keep the `f64` accumulator and only convert at the final storage step.

**Endianness assumption is baked into the pipeline:**
- Problem: `dump::be_u64` (`crates/base60-cli/src/dump.rs:35-40`) and `format::be_u64` (`crates/base60-cli/src/format.rs:26-31`) always interpret the chunk as big-endian. `decode::parse_run` emits `value.to_be_bytes()` (`decode.rs:37`). This is intentional for round-trip correctness.
- Files: `crates/base60-cli/src/dump.rs:35-40`, `crates/base60-cli/src/format.rs:26-31`, `crates/base60-cli/src/decode.rs:37`
- Why fragile: A future `--endian=little` flag would need to update all three sites and the `decode` format is line-for-line identical between BE and LE, so consumers cannot tell which mode produced a given dump.
- Safe modification: If endianness becomes configurable, embed a marker in the dump header (e.g. a comment row) so `decode` can dispatch correctly; do not rely on user memory.

**`TabletLens` purist-mode zero handling depends on the final-digit guard:**
- Problem: `crates/base60-core/src/lens.rs:118-150` erases all leading zeros in purist mode *except* the final digit (the `last = i == DIGITS - 1` check at `lens.rs:130`). For `chunk == 0` all digits are zero; without the guard, output would be blank.
- Files: `crates/base60-core/src/lens.rs:125-145`
- Why fragile: The invariant "always show at least one digit" is enforced by a single `if !last` branch. Refactoring the loop to use `iter().enumerate()` + `.last()` differently could drop the guard silently.
- Safe modification: The `tablet_purist_preserves_trailing_digit_even_if_zero` test (`lens.rs:296-302`) covers exactly this; keep it.

**`HashMap` iteration non-determinism leaks into persistence:**
- Problem: `persist::snapshot` (`crates/base60-cli/src/tui.rs:590-600`) sorts bookmarks before writing — a deliberate workaround for `HashMap`'s non-deterministic iteration.
- Files: `crates/base60-cli/src/tui.rs:590-600`
- Why fragile: If someone replaces `sort_unstable_by_key` with a plain iteration, state files will diff-churn across saves even when nothing changed.
- Safe modification: Keep the sort. The comment at `tui.rs:593` documents the intent ("Deterministic order on disk so `diff`-ing state files is useful.").

## Scaling Limits

**Whole-file `analyze` runs at launch in the TUI:**
- Current capacity: `ViewState::new` (`crates/base60-cli/src/tui.rs:155`) calls `analyze::analyze(data, DEFAULT_WINDOW)` eagerly. At ~256 MB/s for the byte walk + window pass, a 1 GB file blocks the viewer for ~4 seconds.
- Limit: The TUI is unusable until `analyze` returns; `ratatui::run` has not yet drawn a frame.
- Scaling path: Move `analyze::analyze` into a `std::thread::spawn`, store an `Arc<Mutex<Option<Analysis>>>`, and render a "analysing..." state for the semantic-jump keys until it completes. Or compute lazily on first `]p`/`[z`/`]e` keypress.

**`stdin` loading is unbounded:**
- Current capacity: `reader::load_stdin` (`crates/base60-cli/src/reader.rs:61-68`) calls `stdin().read_to_end(&mut buf)` — the whole stream is materialised in RAM before any output.
- Limit: System RAM. `base60 < /dev/sda` on a 4 TB disk will OOM.
- Scaling path: For the non-interactive dump path specifically, stream `stdin` through `BufRead` in 8-byte chunks directly into `write_line`. The TUI genuinely needs the whole slice for bookmarks/search, so keep the full-read behaviour when `--interactive` is set.

**`decode` loads one line at a time (good), but `analyze` materialises a full histogram vector:**
- Current capacity: `analyze::entropy_windows` (`crates/base60-cli/src/analyze.rs:138-149`) collects one `f32` per complete window. A 100 GB file with window 256 → ~400M `f32`s → 1.6 GB of `Vec<f32>`.
- Limit: `std::alloc::handle_alloc_error` on low-memory systems.
- Scaling path: Stream the sparkline directly into `write_summary`; only min/max/mean are needed from the windows collection, which can be accumulated online.

## Dependencies at Risk

None at immediate risk. `Cargo.toml` pins versions with caret ranges (`anyhow = "1.0.102"`, `clap = "4.6.1"`, `crossterm = "0.29.0"`, `memmap2 = "0.9.10"`, `ratatui = "0.30.0"`). `ratatui` 0.30 is recent (pre-1.0) and may introduce breaking changes.

**`ratatui` is pre-1.0:**
- Risk: `ratatui = "0.30.0"` (`crates/base60-cli/Cargo.toml:23`) tracks a library whose API shifts meaningfully between minor versions.
- Impact: TUI dependents (`tui.rs`, `dump::styled_line`, every `color::*_style` function) will likely need touch-ups on each upgrade.
- Migration plan: Upgrades already land behind the `cargo clippy --all-targets -- -D warnings` CI gate (`.github/workflows/ci.yml:51`), so breakage surfaces immediately. No preemptive pinning needed.

**`clippy::multiple_crate_versions` is silenced:**
- Risk: `Cargo.toml:33` allows `multiple_crate_versions` because "Transitive dependency graph is fixed by upstream; warning is not actionable". `Cargo.lock` shows two versions of `bitflags` listed.
- Impact: Slightly larger binary, duplicate symbols in debug builds.
- Migration plan: Re-enable the lint and update transitive constraints once `clap`/`ratatui`/`crossterm` converge on a single `bitflags` major.

## Missing Critical Features

**No fuzz target for `decode::parse_run` / `search::parse`:**
- Problem: Both modules parse user-controlled text into structured values. `decode::parse_run` does byte arithmetic that could overflow on adversarial input if the `find_digit_run` gate is ever bypassed; `search::from_str` has quoted-string edge cases (`search.rs:50-56`) that are hand-tested only.
- Blocks: Confidence in fuzzer-grade hardening of the text-ingestion surface.
- Fix approach: Add `fuzz/fuzz_targets/decode.rs` using `cargo-fuzz`, running against `decode_stream` with arbitrary `BufRead` bytes; same pattern for `Pattern::from_str`.

**No integration tests against real binaries:**
- Problem: `.github/workflows/ci.yml:37-40` runs `cargo test --workspace --all-targets --locked` but no `tests/` directory exists at the workspace root nor in either crate.
- Blocks: End-to-end guarantees that `base60 FILE | base60 decode > OUT && cmp FILE OUT` holds across every `--lens`, `--format`, and `--color` combination.
- Fix approach: Add `crates/base60-cli/tests/roundtrip.rs` using `assert_cmd` + `predicates` (fixture-driven). Pair with a `test/` corpus of ELF/PNG/ZIP/zero-fill samples. The existing `test/` directory at the repo root is empty.

## Test Coverage Gaps

**`reader::load_file` (mmap path) has no test:**
- What's not tested: `crates/base60-cli/src/reader.rs:51-59`. Tests at `reader.rs:81-108` only exercise `clamp_range` — the stdin path, mmap path, and file-open error path are all uncovered.
- Files: `crates/base60-cli/src/reader.rs:51-68`
- Risk: A regression where `File::open` or `Mmap::map` silently swaps to a different module (e.g. a `cfg`-gated `memmap2` variant) would compile clean and fail only at runtime.
- Priority: Medium. The code is three lines of well-trodden API wrapping; risk is in adding feature flags later.

**Dump → decode round-trip with `--lens` active is untested:**
- What's not tested: The advertised pipeline `base60 --color=never FILE | base60 decode > FILE.roundtrip` works when `--lens=cuneiform` is set (wedge glyphs in the line must not confuse `find_digit_run`).
- Files: `crates/base60-cli/src/decode.rs` — tests at `decode.rs:121-201` only feed hand-crafted ASCII lines, never the output of `dump_all` with a lens.
- Risk: If `dump::write_line` ever emits a digit run character (ASCII `0-9` or `:`) inside the lens column, `find_digit_run` would match the wrong substring. Current lenses are safe (cuneiform uses non-ASCII, tablet uses `⌐ ... ¬`, time uses `𒁹`), but the invariant is implicit.
- Priority: Medium. Add a parametrised test over `LensMode::iter()` that dumps, pipes through `decode_stream`, and asserts byte equality.

**`format::emit_html` / `emit_json` have no broken-pipe test:**
- What's not tested: Interaction between `BufWriter::flush` and a closed pipe in the JSON/HTML output paths. `main.rs:97-105` handles `BrokenPipe` for every format, but no test exercises the error path.
- Files: `crates/base60-cli/src/main.rs:97-105`, `crates/base60-cli/src/format.rs:89` and `format.rs:139` (where flush can fail)
- Risk: A refactor that moves the flush outside the `match result` would turn `base60 FILE --format=json | head -1` from a clean exit into a stderr-noisy error.
- Priority: Low. The pattern is stock Rust CLI idiom; failure mode is cosmetic.

**`persist::save` write-failure path is untested:**
- What's not tested: `crates/base60-cli/src/persist.rs:60-70` ignores `fs::write` errors by design. No test verifies that a read-only state directory causes a clean no-op rather than a panic.
- Files: `crates/base60-cli/src/persist.rs:60-70`
- Risk: Low — the `let _ =` binding explicitly discards errors.
- Priority: Low.

**`tui` exit-with-save path has no test:**
- What's not tested: `crates/base60-cli/src/tui.rs:81-86` calls `persist::save(path, &state.snapshot())` when `handle_key` returns `Break`. All `tui::tests::*` cases invoke `handle_key` directly and never round-trip through the `ratatui::run` driver.
- Files: `crates/base60-cli/src/tui.rs:71-89`
- Risk: Medium. A regression where the `Break` arm forgets to save would be invisible to unit tests.
- Priority: Medium. Cover with an integration test that launches `ratatui::run` against a TestBackend, pushes `q`, and asserts the state file appears.

**`main.rs::run_completions` path is exercised only implicitly:**
- What's not tested: `crates/base60-cli/src/main.rs:142-150` generates shell completions. No test asserts that `base60 completions bash` emits non-empty output or that the generated script parses in each target shell.
- Files: `crates/base60-cli/src/main.rs:142-150`
- Risk: Low — `clap_complete::generate` is well-tested upstream.
- Priority: Low.

**Byte-frequency histogram overflow (`saturating_add`) is plumbed but never hit in tests:**
- What's not tested: `crates/base60-cli/src/analyze.rs:95` uses `saturating_add(1)` to prevent `u32` overflow. Triggering this requires a >4 GB input of a single byte — outside any reasonable test fixture.
- Files: `crates/base60-cli/src/analyze.rs:95`
- Risk: Very low. The saturation is defensive; a unit test could mock the loop with a pre-populated histogram at `u32::MAX - 1` to cover the saturated branch.
- Priority: Low.

---

*Concerns audit: 2026-04-23*
