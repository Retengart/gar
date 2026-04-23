# Pitfalls Research

**Domain:** Rust CLI hardening — test infrastructure, streaming perf, refactor consolidation
**Researched:** 2026-04-23
**Confidence:** HIGH (context7 on serial_test, criterion, memchr, strum; rust-fuzz book for cargo-fuzz; plus community post-mortems)

Scoped to the base60 v2 milestone: TEST-01..05, PERF-01..06, REF-01..03. Pitfalls are listed per-theme, severity-tagged (🔥 will bite / ⚠️ might bite / 💡 good to know), and mapped to a phase.

---

## Critical Pitfalls

### Pitfall 1: `serial_test` keys that don't actually serialise the env 🔥

**What goes wrong:**
Someone annotates `NO_COLOR` tests with `#[serial(no_color)]` and `NO_UNICODE` tests with `#[serial(no_unicode)]`, assuming the attribute name describes what's locked. `serial_test` keys are opaque identifiers — different keys run in parallel. A `NO_COLOR` test and a `NO_UNICODE` test will happily run concurrently, each calling `unsafe { env::set_var }`, and the race that currently lives in `crates/base60-cli/src/main.rs:183-219` stays alive.

**Why it happens:**
Per the `serial_test` docs, `#[serial(A)]` and `#[serial(B)]` are *deliberately* independent groups. The crate's design encourages this as a feature ("different subsets of tests to be serialised with each other, but not depend on other subsets"). Intent ("this touches env vars") ≠ key identity.

**How to avoid:**
Use **one** key (`env`, not `no_color`/`no_unicode`) for every test that reads or writes *any* process-wide env var. The current set spans four files:
- `crates/base60-cli/src/main.rs:183-219` (NO_COLOR)
- `crates/base60-cli/src/main.rs:191+198,205+208` (NO_UNICODE)
- `crates/base60-core/src/cuneiform.rs:150-161` (NO_UNICODE + TERM)
- `crates/base60-core/src/lens.rs:321-328` (NO_UNICODE)

All four get `#[serial(env)]`. Add a multi-key annotation (`#[serial(env, state_dir)]`) only when a new scope is introduced (e.g., `persist::state_base_dir` wants `HOME`/`XDG_STATE_HOME`).

Also: **never mix `#[serial(env)]` with `#[file_serial(env)]`** — they use different locking mechanisms (in-process mutex vs. file lock) and do not serialise against each other.

**Warning signs:**
- More than one `serial(...)` key appears in a crate that touches env vars.
- Test fails intermittently on Ubuntu runners with `ACTIONS_RUNNER_CORES=4` but passes locally.
- `grep -rn 'set_var\|remove_var' crates/` returns hits outside `#[serial(env)]` scope.

**Prevention strategy:**
Add a repo convention (`crates/base60-cli/tests/CONVENTIONS.md` or a comment in the first serial test module): *"Every env-mutating test uses `#[serial(env)]`. No exceptions. No alternate key names."* Add a grep-based check to CI:
```yaml
- run: |
    if grep -rn --include='*.rs' 'env::set_var\|env::remove_var' crates/ \
       | grep -v '#\[serial(env)\]' | grep -v 'fn test_'; then
      echo "env-mutating code outside #[serial(env)]"; exit 1
    fi
```

**Phase:** TEST (TEST-04 specifically)
**Severity:** 🔥

---

### Pitfall 2: Streaming stdin re-introduces OOM by accident 🔥

**What goes wrong:**
`PERF-01` replaces `stdin().read_to_end` with a `BufReader`. But somewhere between read and write, the new code does one of:
- `let all: Vec<u8> = BufReader::new(stdin()).bytes().collect();` — materialises the whole stream.
- Passes the reader to a helper that calls `read_to_end` internally.
- Uses `BufReader::with_capacity(usize::MAX)`.
- Buffers into a `Vec<u8>` up to the "last partial chunk" boundary, then forgets to cap the vec.
Net effect: the OOM on `base60 < /dev/sda` stays. CI passes because no test feeds more than a few KB.

**Why it happens:**
Chunked byte iteration in Rust is awkward. The obvious shape — iterate 8-byte chunks from a `BufRead` — doesn't exist in the stdlib. Developers reach for `read_to_end` or `collect` because they work on the test inputs.

**How to avoid:**
The streaming path must:
1. Accept `R: BufRead`, not `R: Read`.
2. Read into a fixed-size buffer at most once per chunk (`let mut buf = [0u8; CHUNK];` + `read_exact` + handle the short tail).
3. Never call `read_to_end`, `fill_buf` without consuming, or `bytes().collect()`.

Verify with a test that pipes a synthetic `Read` impl yielding `1 << 30` zero bytes in 64 KB chunks and asserts peak RSS stays bounded. `assert_cmd` + `/dev/zero` truncated by `head -c` via `std::process::Command` is the integration-level form; `peak_alloc` or a custom `GlobalAlloc` watermark is the unit-level form.

**Warning signs:**
- `Vec::with_capacity(data.len())` or `data.len()` calls in the stdin path — `len()` doesn't exist on a streaming input.
- `read_to_end` anywhere downstream of `load_stdin`.
- Tests only feed `<= 4 KB` of stdin.

**Prevention strategy:**
Before PERF-01 lands, add `crates/base60-cli/tests/streaming.rs` that pipes >100 MB of synthetic bytes through the binary under a `ulimit -v 65536` wrapper (Linux) / `Job Object` memory cap (Windows, skip if unavailable) and asserts success. On macOS/Windows, fall back to checking that the process never allocates a single `Vec<u8>` of `total_bytes_read` size — enforced by a `#[cfg(debug_assertions)]` allocator shim.

**Phase:** PERF (PERF-01)
**Severity:** 🔥

---

### Pitfall 3: Fuzz targets flag by-design rejections as crashes 🔥

**What goes wrong:**
`fuzz_targets/decode.rs` calls `decode::decode_stream(data, &mut out)` on arbitrary bytes. `decode_stream` legitimately returns `Err(InvalidData)` on malformed input (`decode.rs:173` already tests this). A fuzz harness that unwraps the `Result`, or that asserts on a specific error variant, reports every nonsense input as a "crash" — corpus fills up with trivial junk, real bugs drown.

**Why it happens:**
cargo-fuzz's convention is "any panic/abort = bug." But *error returns* are not panics; a harness that `.unwrap()`s them converts a non-bug into a bug. This inverts the signal.

**How to avoid:**
Harness shape for both fuzz targets:
```rust
fuzz_target!(|data: &[u8]| {
    let mut out = Vec::with_capacity(data.len());
    // Errors are expected on malformed input; only panics are bugs.
    let _ = base60_cli::decode::decode_stream(data, &mut out);
});
```
For `search::Pattern::from_str`:
```rust
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = base60_cli::search::Pattern::from_str(s);
    }
});
```
The UTF-8 guard matches `rust-fuzz/book`'s canonical pattern. Do **not** guard with `std::panic::catch_unwind` — cargo-fuzz compiles with `-Cpanic=abort`, so it won't catch anyway.

Additionally, when a fuzzer does find a crash, verify it's a real bug before fixing: the found input may exercise a `debug_assert!` that is correct production behaviour. Run the crash case with `--release` to confirm.

**Warning signs:**
- `unwrap()` or `expect()` anywhere inside `fuzz_target!`.
- Corpus grows >1000 entries in the first minute of fuzzing (non-reproducing coverage noise).
- "Crash" reports that panic on `from_utf8` or `InvalidData` text.

**Prevention strategy:**
Add a skeleton comment in each fuzz target file:
```rust
// IMPORTANT: Results are not bugs. Panics are bugs.
// - decode_stream returning Err is the happy path for malformed input.
// - If fuzzer reports a crash: verify with --release first.
```
Add `fuzz/Cargo.toml` to `.gitignore` for `corpus/` and `artifacts/` to prevent accidental commit of massive corpora.

**Phase:** TEST (TEST-02)
**Severity:** 🔥

---

### Pitfall 4: `memchr::memmem` loses to naive scan on 1–3 byte needles ⚠️

**What goes wrong:**
`PERF-03` swaps `search::find_all`'s naive O(n·m) scan for `memchr::memmem::find_iter`. Benchmarks on the realistic needle `"ELF"` (3 bytes) look fine, but `search.rs` accepts **single-byte** needles (a user typing `str:A` or `hex:ff`). `memmem` on a 1-byte needle goes through `packedpair::Finder` and can be slower than `memchr::memchr` on some inputs. For 2-byte needles, the prefilter's false-positive rate dominates on low-entropy haystacks (e.g., `/dev/zero` or long `.bss` runs in ELF binaries — realistic input for this tool).

**Why it happens:**
Per BurntSushi's own memchr benchmarks and issue #139 on the crate: `memmem` dispatches to Two-Way with a packed-pair prefilter by default. For needle length 1, stdlib `memmem` and `memchr` converge; for length 2–3 the prefilter can over-trigger on adversarial inputs. The official memchr README benchmarks show only 1.03× geometric-mean speedup over stdlib `memmem::prebuilt` — room for pathological inputs to flip.

**How to avoid:**
Dispatch by needle length in `find_all`:
```rust
match needle {
    [] => return Vec::new(),
    &[b] => memchr::memchr_iter(b, haystack).collect(),
    _ => memchr::memmem::find_iter(haystack, needle).collect(),
}
```
Criterion benchmark must include the adversarial corpus, not just a realistic one: `zero-fill`, `all-0xFF`, and a 1-byte needle against each. PERF-06 is this benchmark — it has to land *before* PERF-03 so the swap is gated by measurement.

**Warning signs:**
- PERF-03 PR has no benchmark showing wins on a 1-byte needle.
- Post-change, `tui::jump_next_match` feels sluggish on a zero-fill segment.
- Criterion reports "no change" or a regression on `find_all/zero_fill/needle=1`.

**Prevention strategy:**
PERF-06 (criterion harness) lands *first* and gates PERF-03. Bench parametrisation:
- Haystack: `/dev/urandom` sample, `/dev/zero` sample, hand-crafted ELF fragment.
- Needle: `b"\x00"`, `b"\xff\xff"`, `b"ELF"`, `b"cafebabe"`.
Fail the PR if any cell regresses by more than noise_threshold (`2%` default — raise to `5%` on shared GHA runners since Criterion's own FAQ warns cloud CI is too noisy for meaningful comparison; see `bheisler.github.io/criterion.rs/book/faq.html`).

**Phase:** PERF (PERF-06 gates PERF-03)
**Severity:** ⚠️

---

### Pitfall 5: `be_u64` promotion leaks the CLI's internals into the library API 🔥

**What goes wrong:**
REF-01 moves `be_u64` into `base60-core`. Someone reaches for `pub fn be_u64(&[u8]) -> u64` at the top of a new `chunk.rs` module. Now the library's public surface includes:
- A byte-slice-to-u64 helper that's a thin wrapper over `u64::from_be_bytes`.
- An implicit commitment that this function stays 8-byte-centric forever, because `base60` the CLI depends on it.

The `base60-core` README's selling point — **zero dependencies, narrow public surface, u64↔11-digit conversion** — dilutes into "and also some chunk helpers." Downstream users of `base60-core` now have a second API shape to reason about; future cleanups (e.g., generic N-byte chunking) become breaking changes.

**Why it happens:**
"Moving duplicated code to a shared crate" feels like de-duplication, not API surface growth. The symmetry with the existing `u64_to_base60` makes `pub fn be_u64` look like a natural companion. The reviewer sees "it's already duplicated, consolidation is always good."

**How to avoid:**
Put `be_u64` in a **`pub(crate)` + re-export** shape, not a flat `pub`. Either:
1. **Shared internal crate (`base60-core::chunk` module, not re-exported at crate root):** Core exposes it at `base60_core::chunk::be_u64`. The CLI imports it; third-party users have to go two levels deep. Signals "this is an implementation detail we happen to share." Cleanest option.
2. **New `base60-core-internal` crate:** Overkill for one function; skip.
3. **Keep it CLI-local in a new `crates/base60-cli/src/chunk.rs`:** Matches the CONCERNS.md fix-approach exactly ("a `pub(super) fn` in a shared `crates/base60-cli/src/chunk.rs` module"). Avoids changing the library surface entirely. **Prefer this** unless there's a downstream user asking for core-level chunk helpers — there isn't.

Key decisions in `PROJECT.md:129` says "`be_u64` moves to `base60-core::chunk` as `pub fn`" — this is the pitfall risk. Push back during REF-01 review and go with option 3 unless there's a documented reason otherwise.

**Warning signs:**
- `base60-core`'s `pub` surface area grows by more than the single promotion in question.
- `cargo doc -p base60-core` lists a new module with no user-facing doctest.
- Semver dance: deciding whether renaming `be_u64` to `chunk_be` would be a breaking change.

**Prevention strategy:**
Before merging REF-01, run `cargo public-api --diff-git-checkouts main HEAD -p base60-core` (or the equivalent manual inspection of `cargo doc` output). Any new `pub` item that isn't the single dedup target rejects the PR.

**Phase:** REF (REF-01)
**Severity:** 🔥

---

### Pitfall 6: `strum::EnumIter` pulls a derive into zero-dep core ⚠️

**What goes wrong:**
REF-02 reaches for `#[derive(strum::EnumIter)]` on `LensMode`. But `LensMode` lives in `base60-core/src/lens.rs` — the *zero-dep* crate. Adding `strum = "0.27"` and `strum_macros = "0.27"` to `base60-core/Cargo.toml` violates the library's selling point (PROJECT.md line 110: *"`base60-core` must keep zero external dependencies — its selling point"*).

**Why it happens:**
`LensMode` the enum is defined in core (because `Lens` is a core trait). Dispatch tables (`cycle`, `label`, `build_lens`) live in the CLI. The obvious single-table derive wants to hang off the enum definition — i.e., the wrong crate.

**How to avoid:**
Two workable shapes:
1. **Move `LensMode` to the CLI.** Keep the `Lens` trait in core. `LensMode` is a CLI-layer dispatch concern; it doesn't need to be in core. Then `strum` is fine as a CLI-only dev-dep-equivalent (regular `[dependencies]` in the CLI crate).
2. **Hand-roll the dispatch table without `strum`.** A `const LENSES: &[(LensMode, &str, fn() -> Box<dyn Lens>)]` in `cli.rs` covers the four call sites with zero macros. Even simpler: a match on `LensMode` that returns a tuple `(label, next, build)` — one match, one source of truth, zero new deps.

**Prefer option 2.** The project bar is zero-dep in core, and the CLI already has enough crates. Custom derives are overkill for four variants.

If `strum` *is* added (CLI-only), guard against the `EnumIter`-unused warning: when the iteration is only called from tests, the derive produces a `dead_code` lint. Annotate with `#[allow(dead_code)]` on the iter-using helper, or make it `#[cfg(test)]`-gated.

**Warning signs:**
- `base60-core/Cargo.toml` gains any entry under `[dependencies]`.
- `cargo tree -p base60-core` shows any non-stdlib crate.
- Clippy reports `dead_code` on `LensMode::iter` because only tests call it.

**Prevention strategy:**
Add a CI gate:
```yaml
- name: core-zero-dep-check
  run: |
    deps=$(cargo metadata --format-version 1 --manifest-path crates/base60-core/Cargo.toml \
           --no-deps | jq '.packages[0].dependencies | length')
    if [ "$deps" -ne 0 ]; then echo "base60-core has $deps deps, expected 0"; exit 1; fi
```

**Phase:** REF (REF-02)
**Severity:** ⚠️

---

### Pitfall 7: Integration-test fixture corpus bloats the git repo 🔥

**What goes wrong:**
TEST-03 asks for "fixture-driven integration tests against real binaries (ELF / PNG / ZIP / zero-fill)". Someone drops `/bin/ls` into `test/fixtures/elf` (130 KB), a 500 KB PNG, and a zero-fill file generated with `dd if=/dev/zero of=zero bs=1M count=10` (10 MB). The repo grows 10+ MB overnight; every clone pulls binaries forever; git LFS is raised as "the fix" and introduces CI complexity and LFS quota concerns.

**Why it happens:**
"Real binaries" is ambiguous. Test authors conflate "realistic" with "literal". The right shape is "synthetic binaries that trigger each code path", not "whatever was lying around".

**How to avoid:**
**Generate fixtures at test time, not check them in.** For each format:
- **Zero-fill:** `vec![0u8; 8192]` in-test — no fixture needed.
- **ELF:** Use `object` crate's builder *or* hand-craft a 128-byte minimal ELF header in a helper. A real binary is not required — `decode` only cares about the byte sequence.
- **PNG:** Hand-craft magic bytes + IHDR + IEND: 45 bytes total. Full decoder correctness isn't being tested; byte-identity roundtrip is.
- **ZIP:** Similarly, the 22-byte EOCD + one PK header is enough to exercise "file with high-entropy regions".

Keep all fixtures < 4 KB each and generate in-test via a const `&[u8]` or `include_bytes!`. If a test genuinely needs a large input (perf regression), gate it with `#[ignore]` and a `cargo test --ignored` opt-in, and generate it from a seeded RNG rather than checking in bytes.

**Warning signs:**
- `git ls-files | xargs -I{} wc -c {} | sort -rn | head` shows any tracked file over 100 KB.
- `test/` directory gains `.gitattributes` with LFS filters.
- `.gitignore` includes `test/fixtures/*.bin`.

**Prevention strategy:**
Pre-commit hook or CI check:
```yaml
- name: no-large-binaries
  run: |
    large=$(git ls-files | xargs -I {} stat -c '%s %n' {} 2>/dev/null \
            | awk '$1 > 102400 {print}')
    if [ -n "$large" ]; then echo "large files tracked:"; echo "$large"; exit 1; fi
```
Add a `tests/fixtures.rs` module with named builder functions (`fn minimal_elf() -> Vec<u8>`, `fn minimal_png() -> Vec<u8>`) so the pattern is discoverable.

**Phase:** TEST (TEST-03)
**Severity:** 🔥

---

### Pitfall 8: `decode::parse_run` refactor changes error semantics silently ⚠️

**What goes wrong:**
REF-03 tightens `parse_run` to take `&[u8; RUN_LEN]` and promote the digit-check inside. Developer does:
```rust
fn parse_run(run: &[u8; RUN_LEN]) -> Result<u64, DecodeError> {
    // ... old arithmetic ...
    if !run.iter().all(|b| b.is_ascii_digit() || *b == b':') {
        return Err(DecodeError::InvalidDigit);
    }
    // ... parse ...
}
```
Previous shape returned `io::Error` with `ErrorKind::InvalidData` and a specific message (`decode.rs:173`: *"assert!(err.to_string().contains(\"99\"))"*). The new shape returns a different error type, or the same type with a different message, or signals an error at a different byte position. The existing `decode::tests::rejects_*` tests keep passing because they assert on `kind()`, but the **message contents** and **which byte triggered the error** both drift.

Downstream: users piping `base60 decode` into scripts that grep stderr for `"pair at offset N"` messages now get different output. No compile-time breakage; silent behavioural drift.

**Why it happens:**
"Promote the digit-check inside" is a one-line description of a refactor that has four moving parts: return type, error variant, error position, error message. The existing test suite asserts on some of these but not all.

**How to avoid:**
1. Before starting REF-03, expand the test suite to pin down the *current* contract: add tests that assert on error message substrings (`contains("99")` is already there; add `contains("offset")` if applicable), on which byte position was flagged, and on the error type (not just `kind()`).
2. Land the tighter contract as a **new function** (`parse_run_strict(&[u8; RUN_LEN]) -> Result<u64, _>`) alongside the old one. Migrate callers one at a time. Delete the old one once callers are migrated — but only after running the expanded test suite on both.
3. Add a round-trip test that specifically verifies `base60 FILE | base60 decode` produces byte-identical output across every `LensMode × FormatMode` combination — this is TEST-01, which **must land before REF-03**. Ordering matters: TEST-01 is the safety net for REF-03.

**Warning signs:**
- REF-03 PR adds no new tests.
- Tests assert only on `err.kind()`, not `err.to_string()`.
- `git log --oneline crates/base60-cli/src/decode.rs` shows REF-03 lands before TEST-01.

**Prevention strategy:**
Roadmap ordering: **TEST-01 before REF-03.** Add to the phase-transition checklist: *"No refactor of `decode::parse_run` merges until round-trip tests cover every `LensMode × FormatMode`."*

**Phase:** TEST (TEST-01 gates REF-03)
**Severity:** ⚠️

---

### Pitfall 9: Criterion noise floor drowns real perf signal on GHA runners 🔥

**What goes wrong:**
PERF-06 adds a `criterion` harness, wires it into CI as a gate on PERF-01..05, and runs it on `ubuntu-latest`. Criterion's default `noise_threshold = 2%` is tuned for dedicated hardware. On a shared GitHub Actions runner, measured variance between back-to-back runs routinely hits 10–15% (the Criterion FAQ directly warns about this: *"You probably shouldn't rely on benchmark results from Cloud-CI providers, because the virtualization ... introduces a great deal of noise"*). Result: every PR either shows a spurious "regression" or a spurious "improvement." Gating becomes noise; team ignores the check; real regressions ship.

**Why it happens:**
Criterion's statistics are sound — noise_threshold is about what "counts" as a change. The problem is that GHA's noise *exceeds* any reasonable threshold (2–5%), so even honest code is flagged. The Criterion maintainer's guidance: use Iai (instruction-count-based) on cloud CI, or self-host.

**How to avoid:**
Three options, in order of preference for this project:
1. **Keep Criterion local, don't gate CI.** Run `cargo bench` locally before and after each perf change; paste numbers into the PR description. Human eyeballs. This is the current-project-scale-appropriate answer.
2. **Switch to Iai-Callgrind.** Instruction counts are deterministic under QEMU/cachegrind; no noise floor. Installs valgrind on Ubuntu runners (2–3 extra minutes per job, acceptable). The Criterion book explicitly recommends Iai for CI.
3. **Self-hosted runner.** Overkill for base60 scale. Requires infra, 2026 pricing now adds `$0.002/minute` platform fee for private repos (public stays free, so technically viable for this project).

**Go with option 1 for this milestone.** Reassess at v3 if perf regressions ship in practice.

Regardless: raise Criterion's `noise_threshold` to 5% in the harness config, so even local runs on a laptop with a Slack background process don't cry wolf.

**Warning signs:**
- CI includes a criterion step that fails PRs.
- PR reviewers comment "ignore the bench failure, it's flaky."
- `criterion --save-baseline` is used against a baseline captured on a different runner.

**Prevention strategy:**
Document in `crates/base60-cli/benches/README.md`:
```
These benches are advisory, not gating. Run locally:
    cargo bench --bench perf -- --save-baseline pre
    # apply change
    cargo bench --bench perf -- --baseline pre
Paste the output into the PR description. CI does not run benches.
```
Set `noise_threshold(0.05)` in the harness.

**Phase:** PERF (PERF-06)
**Severity:** 🔥

---

### Pitfall 10: `HashMap` iteration non-determinism leaks into new tests ⚠️

**What goes wrong:**
New integration tests snapshot output and compare. Somewhere in the pipeline a `HashMap` (or `HashSet`) gets printed in iteration order. Ubuntu test passes. macOS and Windows tests pass *most of the time*. One in fifty CI runs, Windows fails with a diff in byte-order-unrelated output. Flake is blamed on "CI being flaky", suppressed with a retry, masked.

The existing codebase already has this exact issue handled at `crates/base60-cli/src/tui.rs:590-600` — bookmarks are sorted before serialisation explicitly because `HashMap` iteration is non-deterministic. A refactor that "simplifies" this sort away would silently regress state-file determinism.

**Why it happens:**
`std::collections::HashMap` uses a randomised hasher by default. Iteration order changes per-process, not per-platform, so local tests pass consistently; CI runs are different processes.

**How to avoid:**
1. **No `HashMap` / `HashSet` in deterministic-output paths.** Use `BTreeMap` / `BTreeSet`, or explicitly `.collect::<Vec<_>>()` then `.sort()`.
2. **Preserve the existing sort in `tui::snapshot`.** When reviewing REF-02 or any refactor near `tui.rs:590`, verify the sort is intact.
3. **Snapshot tests must normalise.** If using `insta` or similar: use stable hash orderings (`BTreeMap` for snapshots) or post-process output through `sort -k` before comparison.

**Warning signs:**
- `HashMap` iteration used in any `Display`/`Serialize` impl.
- A test flakes with the same assertion failing different lines on different runs.
- Git log shows a commit "simplify snapshot" that removes a `.sort()` call.

**Prevention strategy:**
Grep gate in CI:
```yaml
- name: no-hashmap-in-output
  run: |
    if grep -rn 'HashMap\|HashSet' crates/*/src/format.rs crates/*/src/decode.rs \
       crates/*/src/persist.rs; then
      echo "HashMap in deterministic-output module"; exit 1
    fi
```

**Phase:** TEST (TEST-01, TEST-03)
**Severity:** ⚠️

---

### Pitfall 11: `cargo-fuzz` silently fails on macOS/Windows CI 🔥

**What goes wrong:**
TEST-02 adds `fuzz/` targets. Someone adds a CI job `cargo +nightly fuzz run decode -- -max_total_time=60` in a matrix across Ubuntu/macOS/Windows. Ubuntu passes. macOS passes. Windows fails *loudly* — good, it's fixed by `if: matrix.os != 'windows-latest'`. macOS now silently runs a much shorter fuzz because ASAN instrumentation falls back or the nightly channel auto-selects a different target.

Actual cargo-fuzz support matrix (from the Rust Fuzz Book): **libFuzzer needs LLVM sanitizer support — x86-64 and Aarch64, Unix-like OS only, not Windows. Nightly compiler required.**

**Why it happens:**
"CI matrix across all three OSes" is the existing project idiom (`ci.yml:26`). Fuzz jobs inherit this idiom without knowing the platform limitations.

**How to avoid:**
Fuzz job is **Ubuntu + nightly only**. Explicitly:
```yaml
fuzz:
  name: fuzz
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@nightly
    - uses: Swatinem/rust-cache@v2
    - run: cargo install cargo-fuzz
    - name: fuzz-decode
      timeout-minutes: 5
      run: cargo fuzz run decode -- -max_total_time=240
    - name: fuzz-pattern
      timeout-minutes: 5
      run: cargo fuzz run pattern -- -max_total_time=240
```

Key details:
- `timeout-minutes: 5` as a hard GHA-level cap (default is 360, way too high).
- `-max_total_time=240` (4 min) so libFuzzer exits cleanly with 1 min slack.
- Nightly is required (documented); pin a specific nightly (e.g. `nightly-2026-04-01`) to avoid channel-drift flakes.
- Upload corpus + artifacts on failure with `actions/upload-artifact@v4 if: failure()`.

**Warning signs:**
- Fuzz job runs in matrix over multiple OSes.
- Fuzz job has no `timeout-minutes`.
- Nightly is `nightly` (floating) rather than pinned.

**Prevention strategy:**
`crates/base60-cli/fuzz/README.md` with the platform constraints documented explicitly. CI comments the Ubuntu-only-ness with a link to `https://rust-fuzz.github.io/book/cargo-fuzz.html#platform-support`.

**Phase:** TEST (TEST-02)
**Severity:** 🔥

---

### Pitfall 12: `assert_cmd` color tests are pointless without explicit forcing ⚠️

**What goes wrong:**
Integration test does:
```rust
Command::cargo_bin("base60").unwrap()
    .arg("tests/fixtures/sample.bin")
    .assert().success().stdout(predicates::str::contains("\x1b[31m"));
```
Test passes locally. On CI, `assert_cmd` spawns the binary without a TTY; `color::is_colorful()` (or whatever the auto-detect does) returns false; stdout has no ANSI codes; test fails.

Worse: test passes on developer machine because `CLICOLOR_FORCE` happens to be set in the shell env, and propagates.

**Why it happens:**
Color auto-detection libraries (anstream, termcolor, colored) check `isatty(stdout)`. Child processes of `assert_cmd` are never a tty. Plus, CI environments vary in what env vars they expose (`NO_COLOR`, `CI`, `GITHUB_ACTIONS`). The documentation pattern is: **always force the color mode explicitly in tests.**

**How to avoid:**
Every `assert_cmd` test that asserts on colour behaviour:
```rust
Command::cargo_bin("base60").unwrap()
    .env_clear()                    // start from a known env
    .env("PATH", std::env::var_os("PATH").unwrap())  // preserve PATH (see below)
    .arg("--color=always")           // or --color=never
    .arg(fixture_path)
    .assert().success();
```

Note the **Windows PATH caveat** (rust-lang/rust#37519): `.env("PATH", ...)` on Windows doesn't always work as expected because of how `CreateProcess` resolves executables. `cargo_bin` uses an absolute path so this is usually fine, but if you ever shell-out (`Command::new("sh")`), use absolute paths there too.

For stdin piping (tests that feed input to `base60 decode`):
```rust
Command::cargo_bin("base60").unwrap()
    .arg("decode")
    .write_stdin(dumped_output.as_slice())  // assert_cmd extension
    .assert().success().stdout(original_bytes);
```
Use `write_stdin`, not `Command::stdin(Stdio::piped())` + manual `write_all`; the former closes the pipe correctly on completion.

**Warning signs:**
- `assert_cmd` test has no `--color=...` flag and asserts on ANSI codes.
- `assert_cmd` test doesn't use `.env_clear()` — inherits developer environment.
- Test passes locally, fails on CI.

**Prevention strategy:**
Test helper:
```rust
fn base60_cmd() -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("base60").unwrap();
    cmd.env_clear()
       .env("PATH", std::env::var_os("PATH").unwrap_or_default())
       .env("NO_COLOR", "1");  // deterministic default; override per-test
    cmd
}
```
Every integration test uses `base60_cmd()`, not raw `Command::cargo_bin`.

**Phase:** TEST (TEST-03)
**Severity:** ⚠️

---

### Pitfall 13: `render_to<W>` default method introduces UTF-8 bugs in the fast path ⚠️

**What goes wrong:**
PERF-04 adds a streaming `Lens::render_to<W: Write>(&self, chunk: u64, w: &mut W) -> io::Result<()>` default method. To avoid the per-line `String` alloc, the `CuneiformLens` implementation writes bytes directly:
```rust
fn render_to<W: Write>(&self, chunk: u64, w: &mut W) -> io::Result<()> {
    let digits = /* ... */;
    for &d in &digits {
        let (lo, hi) = (d % 60, d / 60);
        w.write_all(WEDGE_GLYPHS[lo as usize].as_bytes())?;
        w.write_all(WEDGE_GLYPHS[hi as usize].as_bytes())?;
        w.write_all(b" ")?;  // <-- here
    }
    Ok(())
}
```
Looks fine. But `WEDGE_GLYPHS[i].as_bytes()` is correct UTF-8 only *if* the glyph table is ASCII-or-complete-codepoint-strings; and the **byte separator** (` `) is fine; but if someone extends the lens to emit a byte-level field separator like `b'\xa0'` (intended as non-breaking space U+00A0) *without* encoding it as two bytes, the output is no-longer-UTF-8. Downstream `--format=json` embedding or `emit_html` (which escapes but doesn't validate) breaks.

Alternative failure: stack-allocated `[u8; N]` formatter for `PALETTE_NONE` writes partial bytes of a multi-byte codepoint when `N` is sized to digit count rather than byte count.

**Why it happens:**
Moving from `String` (UTF-8-enforced) to `&mut impl Write` (byte-level) trades allocation for type-safety. The invariant "lens output is valid UTF-8" lives in the type system today; after the refactor, it lives only in the developer's head.

**How to avoid:**
1. Keep the `String`-returning `render` as a default method that calls `render_to` under the hood into a `Vec<u8>` and `String::from_utf8` it. Document the UTF-8 requirement on the trait:
   ```rust
   /// # Contract
   /// Implementations MUST write only valid UTF-8 bytes. This is asserted in debug builds
   /// via `String::from_utf8` in the fallback `render` default.
   fn render_to<W: Write>(&self, chunk: u64, w: &mut W) -> io::Result<()>;
   ```
2. Stack-buffer sizing for PALETTE_NONE dump:
   ```rust
   // DIGITS * 2 ASCII + (DIGITS-1) colons + '\n' — all ASCII, no multi-byte.
   let mut buf = [0u8; DIGITS * 2 + (DIGITS - 1) + 1];
   ```
   Annotate the buffer size with *why* it works (ASCII-only), so a future author adding a lens-emitted unicode separator doesn't reach for this buffer and silently truncate.
3. A fuzz-adjacent unit test: for every `LensMode`, render 1000 random `u64` values and assert the output is valid UTF-8.

**Warning signs:**
- `render_to` implementation uses `w.write_all(&[some_byte])` with non-ASCII byte values.
- Stack buffer sized in "digits" rather than "bytes".
- `PerfLens::render_to` has no test asserting UTF-8 validity.

**Prevention strategy:**
Add a `lens_output_is_utf8` test per lens (3 new tests, `None`/`Time`/`Angle` are ASCII-trivial; focus on `Tablet`/`Cuneiform`).

**Phase:** PERF (PERF-04)
**Severity:** ⚠️

---

### Pitfall 14: Doc-tests break when internal reorganisation hides types 💡

**What goes wrong:**
REF-01 moves `be_u64` and related helpers. In passing, `pub(crate)` becomes `pub(super)`, or a module is renamed, or a type is moved. `base60-core/src/url.rs:12-18` has a doc-test that references `encode_u64`. That still works. But a doc-test added during v2 that says:
```
/// use base60_core::chunk::be_u64;
/// assert_eq!(be_u64(b"\x00\x01\x02\x03\x04\x05\x06\x07"), 0x0001020304050607);
```
...breaks if `be_u64` is `pub(crate)`. CI catches this (doc job has `-D warnings`), but the fix cascades into API surface changes.

**Why it happens:**
Doc tests compile against the crate's *public* API. Internal reorganisation that stays `pub(crate)` is invisible to doc tests; reorganisation that exposes a type is not.

**How to avoid:**
- Only add doc-tests to items that are `pub` and intended to stay `pub`.
- If adding a doc-test to a newly-promoted item (REF-01's `be_u64`), make the promotion decision *before* writing the doc-test, not after.
- `cargo doc --workspace --no-deps --locked` is already in CI (`ci.yml:72`); rely on it.

**Warning signs:**
- Doc-test references a type path that changed in the same PR.
- Doc job fails with "unresolved import" in a newly-written doc-test.

**Prevention strategy:**
None needed — CI catches it. This is 💡-level for awareness.

**Phase:** REF
**Severity:** 💡

---

## Technical Debt Patterns

Shortcuts that look reasonable for the v2 hardening milestone but create real cost.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Running `cargo bench` in CI and gating PRs | "Objective regression check" | 10%+ GHA noise → every PR is flaky → checks are ignored → real regressions ship | Never on shared GHA runners; acceptable on self-hosted |
| `serial_test` with descriptive per-variable keys (`no_color`, `no_unicode`) | Reads like documentation | Tests run in parallel across keys → env races persist | Never; always use one `env` key |
| Single-byte `memchr::memmem` dispatch | "One codepath for all needle lengths" | 1-byte needles pay Two-Way setup cost; measurable on zero-fill | Only if benchmark shows no regression vs `memchr_iter` |
| `pub fn be_u64` in `base60-core` | Simplest possible promotion | Library API bound to CLI's chunk shape forever | When a third party asks for it — not yet |
| Checking fixture binaries into git | Instant reproducibility | Repo bloat; clone cost; LFS temptation | Fixtures <4KB of ASCII; binaries: generate in-test |
| Deriving `strum::EnumIter` on `LensMode` in core | "One table, three call sites gone" | Breaks zero-dep core contract | Never in core; fine in CLI or not at all |
| `cargo fuzz run` in OS matrix | Uniform CI shape | Silent fallbacks on non-Linux; false-green | Never; Ubuntu+nightly only |
| Whole-stdin `read_to_end` in streaming path | Trivial code | OOM on `base60 < /dev/sda` — the exact bug PERF-01 targets | Never after PERF-01 lands |
| `#[ignore]` on a flaky test and moving on | Green CI today | Flake shows up in production; the ignore never gets removed | Only with a tracking issue + deadline |
| `BTreeMap`→`HashMap` "optimisation" in `persist` | Marginal constant-factor | State file diffs churn; deterministic tests fail | Never in persistence paths |

---

## Integration Gotchas

External-tool interactions specific to this milestone.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `cargo-fuzz` on GitHub Actions | Matrix across OSes; default 6-hour job timeout | Ubuntu+nightly only; `timeout-minutes: 5`; pin nightly; `-max_total_time=240` |
| `serial_test` on Windows MSVC | Assuming `#[serial]` works identically to Linux | It does (in-process mutex is cross-platform), but `#[file_serial]` can have temp-dir path issues — prefer `#[serial]` for in-process env mutation |
| `criterion` on `ubuntu-latest` | `noise_threshold = 0.02` default; gating PRs | `noise_threshold(0.05)`; advisory only; run locally with `--save-baseline`, paste numbers in PR |
| `memchr::memmem` with 1-byte needles | `memmem::find_iter(haystack, needle)` blindly | Dispatch: 1-byte → `memchr::memchr_iter`; 2+ → `memmem` |
| `assert_cmd` on Windows | `Command::env("PATH", ...)` to adjust executable resolution | Don't — rust#37519; use absolute paths or `CARGO_BIN_EXE_base60` |
| `assert_cmd` colored-output tests | Relying on auto-detect (no TTY in child) | `--color=always`/`--color=never` explicitly; `.env_clear()` first |
| `strum::EnumIter` with tests-only usage | `EnumIter` triggers `dead_code` when only test paths call `::iter()` | `#[cfg(test)]`-gate the iter call site, or `#[allow(dead_code)]` on the helper |
| `cargo fuzz` release vs debug | Reproducing a crash with default `cargo run` — it's debug mode | `cargo fuzz run <target> <crash-file>` to reproduce; `--release` to verify the bug survives optimisation |

---

## Performance Traps

Scale-dependent failures specific to this tool's usage pattern (large binary files piped through stdin, TUI on multi-GB mmapped files).

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| `read_to_end(stdin)` in non-TUI dump | OOM on `base60 < /dev/sda` | PERF-01 streaming path; test with synthetic >100MB pipe | ~System RAM / 2, today |
| Whole-file `analyze` at TUI launch | 4s frozen screen on 1GB file | PERF-02 background thread; "analysing..." state | ≥500MB file |
| Per-line `String` alloc in `CuneiformLens::render` | GC-pressure-like pauses on very long files | PERF-04 `render_to<W>` default | ≥10M rows (~80GB file) |
| `Vec<f32>` for every entropy window | 1.6GB of `Vec<f32>` for 100GB input | PERF-05 online accumulation | ≥10GB file |
| Naive `find_all` O(n·m) in search | Noticeable lag on multi-MB haystack + long needle | PERF-03 `memchr::memmem` (with 1-byte dispatch per Pitfall 4) | ≥10MB haystack, ≥8-byte needle |
| `memmem` on 1-byte needle + low-entropy haystack | Search *regresses* after PERF-03 | Dispatch 1-byte to `memchr::memchr_iter` | Zero-fill regions in ELF `.bss` |
| Per-digit `write_all` in `PALETTE_NONE` dump | Virtual dispatch overhead (documented CONCERNS.md perf) | Stack buffer in `PALETTE_NONE` branch | Only measurable above ~1GB/s throughput — unlikely to hit |
| Criterion baseline captured on one laptop, compared on another | Noise indistinguishable from signal | Per-host baselines (`--save-baseline $(hostname)`) | Always; cross-host comparison is meaningless |

---

## "Looks Done But Isn't" Checks

Verifications that must pass at phase completion, not just at PR-merge time.

### TEST phase completion
- [ ] `tests/` directory exists at `crates/base60-cli/tests/` with round-trip coverage across every `LensMode × FormatMode` combination
- [ ] `fuzz/` directory exists with targets for `decode::parse_run` and `search::Pattern::from_str`
- [ ] Every env-mutating test uses `#[serial(env)]` — grep check passes
- [ ] `cargo test --workspace --all-targets --locked` passes on all 9 CI cells (3 OSes × 3 Rust channels)
- [ ] Fuzz job runs on Ubuntu+nightly for ≥4 min and reports corpus growth
- [ ] Largest checked-in test fixture ≤4 KB — git `ls-files | xargs wc -c` check
- [ ] Reader mmap/stdin/file-open paths and TUI exit-with-save path all have at least one test hitting them
- [ ] Every `assert_cmd` test calls `.env_clear()` and explicitly sets `--color=...`

### PERF phase completion
- [ ] `base60 < /dev/zero | head -c 1G > /dev/null` completes in bounded memory (Linux check; document for macOS/Windows)
- [ ] TUI launch on 1 GB file shows first frame within 200 ms (background analyze)
- [ ] `criterion` harness exists with baseline + post-change numbers for every PERF-01..05 change, pasted into the respective PR descriptions
- [ ] `memchr::memmem` dispatch handles 1-byte needles via `memchr_iter` — unit test asserts zero-fill + 1-byte case doesn't regress
- [ ] Streaming `render_to` has UTF-8 validity test per lens
- [ ] Entropy-window streaming path doesn't materialise `Vec<f32>`

### REF phase completion
- [ ] `base60-core` still has zero `[dependencies]` — CI metadata check passes
- [ ] `cargo public-api --diff` on `base60-core` shows only the intended additions (REF-01's single promotion, nothing else)
- [ ] `LensMode` dispatch is table-driven — adding a fifth variant touches exactly one file
- [ ] `decode::parse_run` takes `&[u8; RUN_LEN]` and its error messages are pinned by tests
- [ ] `TEST-01` lands before `REF-03` in commit history

---

## Security Mistakes

Concerns specific to a binary-viewer CLI whose input is adversarial by premise.

| Pitfall | Risk Level | Mitigation |
|---------|------------|------------|
| Fuzz corpus committed to git | LOW | `.gitignore` `fuzz/corpus/` and `fuzz/artifacts/`; CI archives crashes as artifacts, not in-tree |
| Fuzz-found "bug" is actually a `debug_assert` or intentional panic | LOW | Verify every reported crash with `--release` first; document in fuzz README |
| `unsafe { env::set_var }` in parallel tests | MEDIUM | `#[serial(env)]` with a single shared key (Pitfall 1) |
| Streaming path that writes before fully reading corrupts on `BrokenPipe` | LOW | Preserve existing `main.rs:97-105` broken-pipe handler; add a `--format=json | head -1` integration test |
| New mmap fallback code path in `reader.rs` tests | MEDIUM | TOCTOU acknowledged in CONCERNS.md is accepted; new tests must not introduce `MmapMut` or mutable aliasing |

---

## Pitfall-to-Phase Mapping

How the three v2 themes prevent each pitfall.

| # | Pitfall | Phase | Verification |
|---|---------|-------|--------------|
| 1 | `serial_test` mis-keyed | TEST-04 | CI grep check: no `env::set_var` outside `#[serial(env)]` |
| 2 | Streaming stdin OOM | PERF-01 | Integration test pipes >100 MB under memory limit |
| 3 | Fuzz false-positive panics | TEST-02 | Fuzz harness code review: no `unwrap()` inside `fuzz_target!` |
| 4 | `memmem` on 1-byte needle | PERF-03 (gated by PERF-06) | Criterion parametric bench; 1-byte needle case must not regress |
| 5 | `be_u64` API surface creep | REF-01 | `cargo public-api --diff` on `base60-core` |
| 6 | `strum` in zero-dep core | REF-02 | CI check: `base60-core` has 0 deps |
| 7 | Fixture corpus bloat | TEST-03 | Git size check: no tracked file >100 KB |
| 8 | `parse_run` error drift | REF-03 (gated by TEST-01) | TEST-01 round-trip covers every `LensMode × FormatMode` |
| 9 | Criterion CI noise | PERF-06 | `benches/README.md` documents advisory-only; `noise_threshold(0.05)` |
| 10 | `HashMap` iteration leak | TEST-01, TEST-03 | Grep gate: no `HashMap` in format/decode/persist |
| 11 | `cargo-fuzz` platform fallback | TEST-02 | Fuzz job explicitly `ubuntu-latest` + pinned nightly |
| 12 | `assert_cmd` color detection | TEST-03 | Test helper `base60_cmd()` with `.env_clear()` + explicit `--color` |
| 13 | `render_to` UTF-8 bugs | PERF-04 | Per-lens UTF-8 validity unit test |
| 14 | Doc-test breakage | REF | Existing `cargo doc` CI gate (`ci.yml:72`) |

**Ordering requirements implied:**
1. **TEST-01 before REF-03** — round-trip safety net.
2. **PERF-06 before PERF-01..05** — criterion harness gates the perf changes (even though not CI-enforcing).
3. **TEST-04 (`serial_test`) before any new env-mutating test** — land the idiom before new tests adopt it.
4. **TEST-02 can run independent of others** — fuzz is infrastructure, not gating.
5. **REF-01, REF-02 independent of each other** — different files, no conflict.

---

## Sources

### Primary (authoritative — HIGH confidence)
- [serial_test docs.rs](https://docs.rs/serial_test/latest/serial_test/) — key-based serialisation, different keys do not collide
- [serial_test: file_serial attribute](https://docs.rs/serial_test/latest/serial_test/attr.file_serial.html) — mixing `serial` and `file_serial` with the same key does NOT serialise (different lock mechanisms)
- [Rust Fuzz Book: cargo-fuzz tutorial](https://rust-fuzz.github.io/book/cargo-fuzz.html) — libFuzzer platform support (x86-64/aarch64, Unix-like, nightly only)
- [Criterion.rs FAQ](https://bheisler.github.io/criterion.rs/book/faq.html) — explicit warning against CI-runner benchmarks; suggests Iai
- [BurntSushi/memchr README](https://github.com/BurntSushi/memchr) — memmem prefilter dynamic heuristic; `1.03×` geomean over stdlib
- [memchr issue #139](https://github.com/BurntSushi/memchr/issues/139) — `packedpair::Finder::find_impl` regression on short-needle parser workloads
- [assert_cmd cargo_bin docs](https://docs.rs/assert_cmd/latest/assert_cmd/cargo/index.html) — PATH limitation pointer to rust-lang/rust#37519
- [Cargo Book: Dev-dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) — dev-deps not propagated; MSRV implications
- [strum::EnumIter wiki](https://github.com/peternator7/strum/wiki/Derive-EnumIter) — variant iteration contract

### Secondary (community — MEDIUM confidence)
- [cargo-fuzz issue #173: Ignoring expected panics](https://github.com/rust-fuzz/cargo-fuzz/issues/173) — `-Cpanic=abort` precludes `catch_unwind`-based filtering
- [Stephan Brumme: Practical String Searching](https://create.stephan-brumme.com/practical-string-searching/) — naive + memchr often beats Two-Way for short needles
- [GitHub Actions 2026 pricing changes](https://github.blog/changelog/2026-02-05-github-actions-self-hosted-runner-minimum-version-enforcement-extended/) — self-hosted $0.002/min platform fee March 2026 (private repos only)
- [FalseCrashReducer paper (arXiv Oct 2025)](https://arxiv.org/html/2510.02185v1) — taxonomy of fuzz false-positive crashes
- [tevps.net: Serialising Rust tests](https://tevps.net/blog/2019/01/14/serialising-rust-tests/) — serial_test design rationale
- [rust-users: Rust-version and dev-deps](https://users.rust-lang.org/t/rust-version-and-development-dependencies/112608) — MSRV + benches separation pattern

### Project-local
- `/home/chris/Projects/utils/test-60/.planning/PROJECT.md` — v2 scope, constraints, key decisions
- `/home/chris/Projects/utils/test-60/.planning/codebase/CONCERNS.md` — pre-identified risks (`be_u64` duplication, env-mutation race, unbounded stdin, missing integration tests)
- `/home/chris/Projects/utils/test-60/.planning/codebase/TESTING.md` — current 164-test inline-module idiom; no `tests/` or `benches/` dirs yet
- `/home/chris/Projects/utils/test-60/.github/workflows/ci.yml` — 3×3 matrix, `-D warnings`, `--locked`

---

*Pitfalls research for: base60 v2 hardening milestone (TEST / PERF / REF)*
*Researched: 2026-04-23*
