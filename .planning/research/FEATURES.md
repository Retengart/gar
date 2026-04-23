# Feature Research

**Domain:** Rust CLI hardening milestone (`base60` v2 — tests, perf, refactor)
**Researched:** 2026-04-23
**Confidence:** HIGH (Rust tooling surface verified via Context7: cargo-fuzz, criterion.rs, proptest, insta, memchr, cargo-deny; peer-CLI policy verified via web sources)

## Scope Reminder

v1 of `base60` ships a full user-facing feature set. v2 is HARDENING ONLY — three themes:

1. **T — Tests + fuzz** (`TEST-01..05` in `PROJECT.md`)
2. **P — Perf + streaming** (`PERF-01..06`)
3. **R — Refactor + consolidation** (`REF-01..03`)

Every feature below is tagged with one or more of these themes.

---

## Feature Landscape

### Table Stakes (Expected Of Any Mature Rust CLI)

A `hexyl`/`xxd` replacement at v2 *must* have these or maintainers will spot the gap on first review.

| # | Feature | Theme | Why Expected | Complexity | Notes |
|---|---------|-------|--------------|-----------|-------|
| TS-01 | `tests/` integration crate driven by `assert_cmd` + `predicates` | T | Every Rust CLI above toy scale (`ripgrep`, `bat`, `fd`, `hexyl`) has one. Unit tests alone cannot cover arg parsing, exit codes, or pipe behaviour. `CONCERNS.md` flags absence. | M | Lives at `crates/base60-cli/tests/*.rs`. Keeps `base60-core` zero-dep. Covers `TEST-03`. |
| TS-02 | Dump→decode roundtrip fixture matrix over `{lens} × {format} × {color}` | T | `PROJECT.md` Core Value: "every binary blob that `base60 FILE \| base60 decode` round-trips must come out byte-identical." `CONCERNS.md` §"Dump → decode round-trip with `--lens` active is untested". Covers `TEST-01`. | M | Fixture corpus: tiny ELF, PNG header, ZIP local-file-header, zero-fill, 0xFF-fill, random 4 KiB. Assert `cmp` byte-equality. |
| TS-03 | Real-binary integration fixtures (ELF / PNG / ZIP / zero-fill / random) under `tests/fixtures/` | T | `CONCERNS.md`: "No integration tests against real binaries"; the empty `test/` dir at repo root is noted. `hexyl` ships sample files; `ripgrep`'s suite is fixture-heavy. Covers `TEST-03`. | S | Keep each fixture ≤8 KiB. Commit with a `README` documenting provenance. |
| TS-04 | `cargo-fuzz` harness gated under `fuzz/` workspace exclusion | T | `CONCERNS.md` §"No fuzz target for `decode::parse_run` / `search::parse`". Any CLI that parses user-controlled byte streams should fuzz its decoder. Matches Rust Fuzz Book canonical setup. Covers `TEST-02`. | M | Two targets: `decode_stream` and `Pattern::from_str`. `workspace.exclude = ["fuzz"]` keeps nightly out of default `cargo test`. Run in a scheduled CI job, not on every PR (`-max_total_time=60`). |
| TS-05 | Serialised env-mutating tests via `serial_test = "3"` | T | `CONCERNS.md` §"Env-mutation tests serialise by convention, not enforcement" — acknowledged `SAFETY` comments rely on "don't run concurrently" while Cargo default is multi-threaded. Flakes on high-core-count runners. Covers `TEST-04`. | S | Annotate `cuneiform.rs:150`, `lens.rs:321`, `main.rs:183`, `persist.rs` env tests with `#[serial(env)]`. One new dev-dep. |
| TS-06 | Broken-pipe integration test (`base60 FILE \| head -1`) | T | Rust default panics on SIGPIPE (issue rust-lang/rust#46016); `main.rs:97-105` already handles `BrokenPipe` for every format. Regression would be invisible without a test. `CONCERNS.md` §"`format::emit_html` / `emit_json` have no broken-pipe test". | S | Use `std::process::Command` + close child stdout early; assert exit code 0 and no panic text on stderr. |
| TS-07 | Streaming stdin in non-TUI dump path | P | `CONCERNS.md` §"`stdin` loading is unbounded". `base60 < /dev/sda` currently OOMs. `xxd`, `hexyl`, `hexdump` all stream. Table stakes for the class. Covers `PERF-01`. | M | Read stdin in 8-byte chunks directly into `write_line`; preserve whole-file read for `--interactive` (TUI needs random access). Key decision already logged in `PROJECT.md`. |
| TS-08 | `memchr::memmem::find_iter` for `search::find_all` | P | `CONCERNS.md` §"`find_all` uses a naïve O(n·m) scan". `memchr` is already a transitive dep (no new crate). Strictly faster. Covers `PERF-03`. | S | Single-function swap. Keep existing tests; they already cover overlap semantics. |
| TS-09 | `be_u64` consolidated into `base60-core::chunk` as `pub fn` | R | `CONCERNS.md` §1 tech debt item — duplicated across `dump.rs:35` and `format.rs:26` with acknowledged drift risk. `PROJECT.md` Key Decisions row 5 commits to the fix. Covers `REF-01`. | S | Library already exposes the inverse (`u64_to_base60`). Keeps `base60-core` zero-dep. |
| TS-10 | MSRV verification gate already green | T | Existing CI matrix pins rustc 1.95 as MSRV floor (`PROJECT.md` Context). Table stakes *already met*. Listed for completeness — v2 must not regress it. | — | No work needed; verify cargo-msrv-prep or equivalent isn't required (matrix check subsumes it). |
| TS-11 | `cargo test --workspace --doc` gate already green | T | Existing CI runs doc tests separately (`TESTING.md:39`). Table stakes *already met*. v2 additions must keep doc tests passing. | — | No work. Flag for roadmap: every new `pub fn` in `base60-core` ships a doctest roundtrip. |

---

### Differentiators (Measurably Raises Quality Bar Over Peer CLIs)

Features that put `base60` visibly ahead of `hexyl`, `xxd`, and `hexdump` on the engineering axis. Each is optional for "mature CLI" but each buys something concrete.

| # | Feature | Theme | Value Proposition | Complexity | Notes |
|---|---------|-------|-------------------|-----------|-------|
| DF-01 | `criterion` benchmarks gating every perf PR | P | Without benches, every "speedup" is a vibe. Canonical pattern: save baseline before change (`cargo bench -- --save-baseline pre`), compare after. Peer CLIs ship benches; `xxd` does not. Covers `PERF-06`. | M | Workspace bench crate or `base60-cli/benches/`. One bench each for: `write_line` (dump path), `find_all` (search), `shannon_entropy` (analyze), `decode_stream`. Dev-only; doesn't bloat release builds. Must land BEFORE any other perf PR. |
| DF-02 | Property-style roundtrip via `proptest` for `u64_to_base60` and `encode_u64` | T | Current convert/url tests iterate hand-picked u64s (`TESTING.md` §"Property-style coverage"). `proptest` auto-generates u64s and shrinks failures to minimal repros. Library is zero-dep; tests are dev-deps only. | S | One `proptest!` block each in `convert.rs` and `url.rs`. ~10 LOC. Keeps `base60-core` lean. |
| DF-03 | Streaming `Lens::render_to<W: Write>` default method | P+R | `CONCERNS.md` perf §"`CuneiformLens::render` allocates a fresh `String` per line". Default-method pattern keeps `render(&self, u64) -> String` for TUI where `ratatui::Span` needs owned strings. Covers `PERF-04`. | M | Breaking in theory for external `impl Lens` (the trait is `pub`), but trait is public-but-not-consumed; default method preserves backward compat. |
| DF-04 | Snapshot tests for dump/json/html output via `insta` | T | `dump.rs` tests use `starts_with`/`contains` fragments (`TESTING.md` §"Assertions on rendered strings"). Snapshots catch silent reordering / spacing drift that string-fragment tests miss. Peer CLI `bat` uses snapshots. | S | Dev-dep only. Commit `*.snap` under `tests/snapshots/`. `cargo insta review` workflow is lightweight. |
| DF-05 | Online streaming entropy sparkline | P | `CONCERNS.md` scaling §"`analyze` materialises a full histogram vector"; 100 GB input → 1.6 GB `Vec<f32>`. Online min/max/mean is O(1) memory. Covers `PERF-05`. | M | Replace `entropy_windows` materialisation with an accumulator passed to `write_summary`. Histogram preview regions stay fixed-size. |
| DF-06 | Async / lazy `analyze` in TUI | P | `CONCERNS.md` scaling §"Whole-file `analyze` runs at launch in the TUI" — 1 GB file blocks ratatui first frame ~4 s. Background thread + `Arc<Mutex<Option<Analysis>>>`, semantic-jump keys show "analysing…" until ready. Covers `PERF-02`. | L | Touches `tui.rs:155` (`ViewState::new`), every `]p`/`]z`/`]e` keypress handler, and the status-line render. Test by driving `ViewState` with `state = None` and asserting correct UI string. |
| DF-07 | Single-table `LensMode` dispatch (`strum::EnumIter` + method-on-variant) | R | `CONCERNS.md` tech debt §2 — four parallel switch statements. Adding a lens forgets at least one site. Covers `REF-02`. Even without a new lens, consolidates four touch-points into one. | M | `strum = "0.26"` adds ~3 transitive deps to CLI only; `base60-core` stays zero-dep. Alternative: hand-rolled table + exhaustiveness test. |
| DF-08 | Tighten `decode::parse_run` contract (`&[u8; RUN_LEN]`, internal digit-check) | R | `CONCERNS.md` fragile areas §"`parse_run` arithmetic assumes every pair is exactly 2 ASCII digits" — safe only because of upstream `find_digit_run` call; any new caller site silently underflows. Type-system-enforced length. Covers `REF-03`. | S | Lift array-length check to the type signature; move `is_ascii_digit` into `parse_run`. Zero behaviour change for current call-site. |
| DF-09 | Platform-matrix coverage already live (3 OS × 3 rustc) | T | CI matrix is already Ubuntu/macOS/Windows × 1.95/stable/beta (`TESTING.md` §"CI Test Setup"). Differentiator *already met*. Listed so roadmap does not inadvertently weaken. | — | No work needed. All new integration tests must pass on all nine cells. |
| DF-10 | `reader.rs` mmap/stdin/file-open coverage | T | `CONCERNS.md` §"`reader::load_file` (mmap path) has no test" — stdin path, mmap path, file-open error path all uncovered. Covers `TEST-05`. | S | Feed tempfile into `load_file`; feed known bytes to a mocked `BufRead` wrapper for stdin; assert error variant on nonexistent path. |
| DF-11 | TUI exit-with-save coverage via `ratatui::backend::TestBackend` | T | `CONCERNS.md` §"`tui` exit-with-save path has no test". `Break` arm forgetting `persist::save` would be invisible to existing unit tests. Covers `TEST-05`. | M | Drive `ratatui::run` with `TestBackend`, push `q`, assert state file appears at expected `$XDG_STATE_HOME` path (use `tempfile::tempdir` + env override). Pairs with TS-05 serialisation. |

---

### Anti-Features (Commonly Suggested for Hardening Milestones, Deliberately Skipped)

Tempting hardening additions that are wrong for *this* project's shape. Keep the milestone focused.

| # | Anti-Feature | Theme | Why Tempting | Why Skip | Alternative |
|---|--------------|-------|--------------|----------|-------------|
| AF-01 | `cargo-tarpaulin` / `cargo-llvm-cov` coverage gate | T | "You can't improve what you don't measure." Coverage badges are cheap signalling. | Workspace has acknowledged `unsafe` blocks (mmap, env tests) — tarpaulin's instrumentation has known interactions with `unsafe` and `#[cfg(test)]` env mutation. `PROJECT.md` Out of Scope row 7: "Unsafe-block elimination — the two surviving `unsafe` blocks are acknowledged and gated." | Rely on "every module ships with inline `mod tests`" convention (`TESTING.md`). Audit PRs for missing test modules manually. |
| AF-02 | `cargo-mutants` mutation testing | T | Proves tests are meaningful, not just coverage lines. | High false-positive rate on byte-arithmetic modules (`convert.rs`, `decode.rs`). Each mutation run takes minutes. Value/effort low for a 164-test codebase already disciplined about invariants. | Targeted `proptest` (DF-02) buys most of the same signal with less ceremony. |
| AF-03 | `cargo-nextest` as required runner | T | 2-3× test-wall-clock speedup; nicer output. | Adds a dev-dep to the install-to-run surface. Stdlib `cargo test` works; CI cache already covers the compile cost. Nextest-specific configs drift from `cargo test`. | Optional for local dev. CI stays on `cargo test`. |
| AF-04 | `cargo-tarpaulin` + Codecov/coveralls upload | T | Public coverage percentage. | Metric becomes the goal. Adds external-service dependency; base60 is MIT/Apache and has no publishing pipeline (`PROJECT.md` Out of Scope). | Per AF-01 alternative. |
| AF-05 | Reproducible-build gate (`SOURCE_DATE_EPOCH`, `--remap-path-prefix`) | T | Ships-with-checksums crowd expects it. | `PROJECT.md` Out of Scope row 6: "Publishing to crates.io" — workspace is `publish = false`, consumed via `cargo install --path`. Reproducible builds matter for signed distros, not personal installs. | Defer until the project has a release pipeline (out of scope for v2). |
| AF-06 | `cargo-audit` as blocking CI gate | T | Obvious security win. | `base60-core` is zero-dep; `base60-cli` has 5 direct deps and none are network-facing. CVE risk is minimal. A failing audit on a transitive dep would block unrelated PRs. | Schedule `cargo audit` weekly via cron (non-blocking), open an issue on finding. |
| AF-07 | `cargo-deny` policy enforcement | T | License + bans + sources coverage; cargo-audit superset. | Workspace already silences `multiple_crate_versions` (`PROJECT.md` Lint Bar) because transitive graph is not actionable. Deny would flag the same noise as clippy did. Overlap with existing lint posture. | Revisit when ratatui ≥1.0 (`CONCERNS.md` §"`ratatui` is pre-1.0") lets the silence be lifted. |
| AF-08 | `iai-callgrind` / instruction-count benchmarks in CI | P | Noise-free, deterministic perf regression gate — proper peer to Criterion for shared CI runners. | Pulls in Valgrind, Linux-only, breaks macOS/Windows CI matrix. Criterion baselines on a dedicated runner or local dev loop are sufficient. | Criterion (DF-01) with `--baseline` compared locally before merge. Document the pattern in CONTRIBUTING. |
| AF-09 | HTML-output XSS hardening (BOM + bidi override escaping) | R | `CONCERNS.md` security §"HTML emitter escapes five chars but not U+2028/U+2029". | Hostile-lens-impl scenario is theoretical; all four built-in lenses produce a fixed alphabet. `PROJECT.md` Out of Scope row 4: "Bookmark notes/labels… raise persistence-security surface." Parallel escalation. | Note the invariant in `format.rs` comment; revisit if/when third-party `impl Lens` becomes real. |
| AF-10 | `unsafe`-block elimination | R | `unsafe = "forbid"` is the gold-plate posture. | Mmap path genuinely needs `unsafe { Mmap::map }`; env tests need `unsafe { set_var }` (Rust 2024 contract). `PROJECT.md` Out of Scope row 7 explicitly rejects this. | Keep `#![forbid(unsafe_op_in_unsafe_fn)]` + `SAFETY:` comments. Pair with TS-05 to neutralise the env-test race. |
| AF-11 | Man-page generation (`clap_mangen`) | — | Peer CLIs ship man pages. | `PROJECT.md` Out of Scope row 5: "shell completions already cover discoverability; man pages duplicate `--help`." | `base60 --help` + shell completions (already shipped in v1). |
| AF-12 | JSON schema document + validator | T | Would catch format drift. | Schema is minimal (6 keys), already documented in README `### Output formats`. `decode` roundtrip tests (TS-02) *are* the schema contract for `bytes` / `digits`. | Existing inline tests assert the key set; add one test that parses output with `serde_json::Value` and asserts the key set if drift risk feels real. |

---

## Feature Dependencies

```
DF-01 criterion benches  ─────┐
                              ├──required before──> TS-07 streaming stdin (PERF-01)
                              ├──required before──> TS-08 memmem (PERF-03)
                              ├──required before──> DF-03 Lens::render_to (PERF-04)
                              ├──required before──> DF-05 online entropy (PERF-05)
                              └──required before──> DF-06 async analyze (PERF-02)
                                                    (every perf PR compares against a baseline)

TS-05 serial_test ──required before──> TS-10 reader.rs env tests
                  └────required before──> DF-11 TUI exit-with-save (reads $XDG_STATE_HOME)

TS-09 be_u64 consolidation ──required before──> any future --endian=little work
                             (enables single-site change; not a v2 feature)

TS-04 cargo-fuzz ──enhanced-by──> DF-08 parse_run contract tightening
                                  (type-level digit check shrinks fuzz surface)

DF-07 LensMode single-table ──enhanced-by──> TS-02 roundtrip matrix
                                             (iter all variants automatically)

TS-03 real-binary fixtures ──required by──> TS-02 roundtrip matrix
                             └──required by──> TS-01 tests/ crate scaffolding

TS-01 tests/ crate ──required before──> TS-02, TS-03, TS-06, DF-04, DF-10, DF-11
                    (integration test harness has to exist first)
```

### Dependency Notes

- **DF-01 (criterion) gates every perf PR:** Without a baseline, "strictly faster" (TS-08) and "skip allocation" (DF-03) are unfalsifiable. The PROJECT.md Active list already sequences this: `PERF-06` is the guardrail.
- **TS-01 (assert_cmd harness) gates most integration tests:** Scaffold the `tests/` crate first; every other T-theme item that touches CLI behaviour lands on top of it.
- **TS-05 (serial_test) gates any test that reads `$XDG_STATE_HOME` or `NO_COLOR`:** DF-11 (TUI exit-with-save) and DF-10 (`reader.rs` env path) both need it to avoid the existing flake pattern.
- **TS-03 (fixtures) gates TS-02 (roundtrip matrix):** Matrix test needs a corpus; fixtures must be committed before the test that consumes them runs in CI.
- **DF-07 (LensMode table) enhances TS-02:** With `EnumIter`, the roundtrip matrix gets every lens free; without, the matrix is hand-typed and drifts.

### Conflicts

- **AF-01 tarpaulin vs. existing `unsafe` posture:** Coverage tooling fights `unsafe { set_var }` instrumentation; workspace has already chosen the latter.
- **AF-02 mutation testing vs. DF-02 proptest:** Both target "are tests meaningful?" — proptest is cheaper. Pick one.
- **DF-06 async analyze vs. DF-05 online entropy:** Not strictly conflicting but overlapping. DF-05 reduces the work DF-06 has to offload; consider doing DF-05 first.

---

## MVP Definition (For the v2 Milestone)

### Must Ship in v2 (P1)

Non-negotiable for a "hardening" release to deserve the name.

- [ ] **TS-01** `tests/` integration crate + `assert_cmd`/`predicates` wiring
- [ ] **TS-02** dump↔decode roundtrip matrix across `{lens} × {format}` (addresses Core Value)
- [ ] **TS-03** real-binary fixture corpus (ELF / PNG / ZIP / zero-fill / random)
- [ ] **TS-04** `cargo-fuzz` harness for `decode_stream` + `Pattern::from_str`
- [ ] **TS-05** `serial_test` on env-mutating tests (eliminates flake pattern)
- [ ] **TS-06** broken-pipe integration test
- [ ] **TS-07** streaming stdin in non-TUI dump path (addresses OOM)
- [ ] **TS-08** `memchr::memmem` in `search::find_all` (zero-dep-cost win)
- [ ] **TS-09** `be_u64` → `base60-core::chunk` (closes duplication drift risk)
- [ ] **DF-01** `criterion` benches for dump / search / analyze / decode (gates perf PRs)

### Should Ship in v2 if Time Permits (P2)

Raises bar meaningfully; can slip to v2.1 without undermining the milestone's name.

- [ ] **DF-02** `proptest` roundtrip for `u64_to_base60` / `encode_u64`
- [ ] **DF-03** streaming `Lens::render_to<W>` default method
- [ ] **DF-05** online streaming entropy-window sparkline
- [ ] **DF-07** single-table `LensMode` dispatch
- [ ] **DF-08** tighten `decode::parse_run` contract
- [ ] **DF-10** `reader.rs` mmap/stdin/file-open coverage
- [ ] **DF-11** TUI exit-with-save coverage via `TestBackend`

### Defer to v3 or Beyond (P3)

- [ ] **DF-04** `insta` snapshot tests — nice polish; existing `contains`/`starts_with` asserts work
- [ ] **DF-06** async / lazy `analyze` in TUI — largest change in the set; ship if DF-05 shows the entropy work is still expensive after online accumulation

---

## Feature Prioritisation Matrix

| # | Feature | User Value | Implementation Cost | Priority |
|---|---------|-----------|---------------------|----------|
| TS-01 | `tests/` integration crate | HIGH (maintainer confidence) | M | P1 |
| TS-02 | dump↔decode matrix | HIGH (Core Value guarantee) | M | P1 |
| TS-03 | fixture corpus | MEDIUM (enabler) | S | P1 |
| TS-04 | cargo-fuzz harness | HIGH (decoder robustness) | M | P1 |
| TS-05 | serial_test env gate | HIGH (eliminates CI flake) | S | P1 |
| TS-06 | broken-pipe test | MEDIUM (regression guard) | S | P1 |
| TS-07 | streaming stdin | HIGH (OOM fix, user-visible) | M | P1 |
| TS-08 | memchr memmem | MEDIUM (silent speedup) | S | P1 |
| TS-09 | be_u64 consolidation | MEDIUM (drift risk closure) | S | P1 |
| DF-01 | criterion benches | HIGH (gates all other perf PRs) | M | P1 |
| DF-02 | proptest roundtrip | MEDIUM (invariant strength) | S | P2 |
| DF-03 | Lens::render_to | MEDIUM (alloc reduction) | M | P2 |
| DF-05 | online entropy | MEDIUM (memory ceiling) | M | P2 |
| DF-07 | LensMode single-table | MEDIUM (future-proofing) | M | P2 |
| DF-08 | parse_run contract | MEDIUM (type-level safety) | S | P2 |
| DF-10 | reader.rs coverage | MEDIUM (gap closure) | S | P2 |
| DF-11 | TUI exit-with-save | MEDIUM (gap closure) | M | P2 |
| DF-04 | insta snapshots | LOW | S | P3 |
| DF-06 | async analyze TUI | MEDIUM (UX on big files) | L | P3 |

**Priority key:** P1 = required for v2; P2 = stretch; P3 = defer.

---

## Competitor / Peer Analysis

Cross-reference of how `xxd`, `hexyl`, `bat`, `ripgrep` handle the same hardening surface.

| Area | xxd | hexyl | bat | ripgrep | base60 v2 plan |
|------|-----|-------|-----|---------|----------------|
| Integration tests | C test harness in vim repo | `tests/integration_tests.rs` via `assert_cmd` | `tests/integration_tests.rs`, extensive; snapshot-style | `tests/regression.rs` custom `workdir.rs` helper (not `assert_cmd`) | **assert_cmd** (TS-01); matches hexyl/bat pattern |
| Fuzzing | none | none published | none published | none published | **cargo-fuzz** on decoder (TS-04) — strictly above peer CLIs |
| Benchmarks | none | none in-tree | `hyperfine` end-to-end, not criterion micros | `rebar` + hyperfine for end-to-end throughput | **criterion** micros (DF-01) for the code under change |
| Broken-pipe | handled natively (C) | handled in `main.rs` | handled | handled; issue #22 was fixed long ago | **already handled** in `main.rs:97-105`; v2 adds test (TS-06) |
| Streaming stdin | yes (C) | yes (line-buffered) | yes | yes (pager-friendly) | **not yet**; TS-07 adds it |
| MSRV in CI | n/a | yes (pinned in CI) | yes (pinned) | yes (1.74 pinned) | **already enforced** (1.95 pinned); TS-10 confirms no regression |
| `cargo-audit` gate | n/a | not observed | not observed | no (large dep graph; would be noisy) | **intentionally skipped** (AF-06); zero-dep-core justifies it |
| Snapshot tests | n/a | string equality | yes, bat has a custom harness | partial | **deferred** (AF / DF-04 P3) |
| Coverage | n/a | none | none | none | **intentionally skipped** (AF-01) |

**Takeaway:** The plan above beats peer CLIs in one axis (fuzzing) and matches them in the axes that matter (integration tests, streaming, broken-pipe, MSRV). It deliberately skips peer-CLI anti-patterns (coverage theatre, mutation testing).

---

## Unresolved Questions

Short list for the REQUIREMENTS.md phase to resolve before plan-write:

- Criterion benches: workspace `benches/` crate or per-crate `benches/` dir? (affects `cargo bench` ergonomics)
- `strum` vs. hand-rolled dispatch table for DF-07? (strum adds 3 transitive deps to CLI; hand-roll is zero-dep but more code)
- Fuzz corpus: commit seed corpus to repo or bootstrap empty? (seed corpus speeds convergence; empty keeps repo small)
- DF-05 vs. DF-06 ordering: ship DF-05 first and re-measure, or ship DF-06 unconditionally? (depends on DF-01 benchmark results on a 1 GB fixture)
- Does TS-04 run in GitHub Actions CI (`schedule:`) or stay as a local/manual target? (nightly toolchain + 60 s × 2 targets = ~3 min runtime)

---

## Sources

### Context7 (HIGH confidence — verified crate docs)
- `/rust-fuzz/cargo-fuzz` — cargo-fuzz init, target layout, `libfuzzer-sys`, corpus/artifacts
- `/bheisler/criterion.rs` — `--save-baseline` / `--baseline` comparison, `CRITERION_HOME`, noise threshold
- `/proptest-rs/proptest` — `prop_map`, roundtrip patterns, `any::<u64>()` strategies
- `/mitsuhiko/insta` — snapshot workflow
- `/burntsushi/memchr` — `memmem::find_iter` SIMD-accelerated substring search
- `/websites/embarkstudios_github_io_cargo-deny` — four-category linter (advisories, bans, licenses, sources)

### Web (MEDIUM confidence — cross-referenced)
- [Rust Fuzz Book — cargo-fuzz tutorial](https://rust-fuzz.github.io/book/cargo-fuzz/tutorial.html)
- [Rust Fuzz Book — Fuzzing in CI](https://rust-fuzz.github.io/book/cargo-fuzz/ci.html)
- [`cargo-fuzz` README](https://github.com/rust-fuzz/cargo-fuzz)
- [criterion.rs docs](https://docs.rs/criterion/latest/criterion/)
- [Bencher — track Criterion in CI](https://bencher.dev/learn/track-in-ci/rust/criterion/)
- [Rust Performance Book — Benchmarking](https://nnethercote.github.io/perf-book/benchmarking.html)
- [RustSec / cargo-audit](https://rustsec.org/)
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/)
- [cargo-machete](https://github.com/bnjbvr/cargo-machete)
- [cargo-msrv verify](http://gribnau.dev/cargo-msrv/commands/verify.html)
- [ripgrep integration-test critique (#448)](https://github.com/BurntSushi/ripgrep/issues/448)
- [ripgrep broken-pipe panic history (#22)](https://github.com/BurntSushi/ripgrep/issues/22)
- [rustc broken-pipe tracking (rust-lang/rust#46016)](https://github.com/rust-lang/rust/issues/46016)
- [Testing Handbook — cargo-fuzz](https://appsec.guide/docs/fuzzing/rust/cargo-fuzz/)

### Internal (HIGH confidence — authoritative for this project)
- `.planning/PROJECT.md` — v1 validated / v2 active / out of scope list
- `.planning/codebase/CONCERNS.md` — every gap/debt item cited above
- `.planning/codebase/TESTING.md` — existing test landscape, CI matrix
- `README.md` — user-facing feature surface the hardening milestone must preserve

---
*Feature research for: Rust CLI hardening milestone (base60 v2)*
*Researched: 2026-04-23*
