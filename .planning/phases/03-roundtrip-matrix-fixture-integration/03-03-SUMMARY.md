---
phase: 03-roundtrip-matrix-fixture-integration
plan: 03
subsystem: testing
tags: [rust, integration-tests, assert_cmd, predicates, broken-pipe, decoder-error, fixture-matrix]

requires:
  - phase: 03-roundtrip-matrix-fixture-integration
    provides: "Plan 03-02 shipped `common::{base60_cmd, fixtures::*, spawn_with_closed_stdout, FixtureEntry}`, `assert_cmd`/`predicates` dev-deps, the spawn-discipline gate."
  - phase: 03-roundtrip-matrix-fixture-integration
    provides: "Plan 03-01 shipped `base60::{Format, LensMode}` re-exports (not directly consumed by this plan — consumed by Plan 03-02's roundtrip matrix)."
provides:
  - "`crates/base60-cli/tests/fixtures.rs` — 4 per-subcommand happy-path tests iterating the 5-fixture corpus (20 logical cases; decode scoped to the 2 byte-identical fixtures)."
  - "`crates/base60-cli/tests/cli.rs` — 9 edge tests (stdin piping, BrokenPipe exit-0, NO_COLOR + --color={auto,always,never}, --skip / --length clamping, decoder \"99\"+\"invalid\" pin)."
affects: [04-hardening-phase (REF-03 decoder refactor pinned by decoder-error test; REF-04 length-preserving + JSON/HTML decode deferred)]

tech-stack:
  added: []
  patterns:
    - "Per-subcommand happy-path fan-out over a declaration-order `Vec<(&'static str, Vec<u8>)>` so panic diagnostics name the failing fixture."
    - "Roundtrip scope split: `fixtures.rs::decode_roundtrips_default_dump_per_fixture` iterates only `FixtureEntry` rows whose length is 8-byte-aligned; matches `ROUNDTRIP_FIXTURES` in `common/mod.rs`."
    - "Decoder error-message pin: `.failure()` + `predicates::str::contains(\"99\").and(predicates::str::contains(\"invalid\"))` — two substrings, neither phrasing-specific; locks `decode::parse_run` error format across Phase 4 REF-03."
    - "Color precedence matrix (3 tests): `NO_COLOR=1` + `--color=auto` → no ANSI; `--color=always` forces ANSI even into a pipe; `--color=never` beats `CLICOLOR_FORCE=1`."

key-files:
  created:
    - "crates/base60-cli/tests/fixtures.rs (113 lines, 4 #[test] fns)"
    - "crates/base60-cli/tests/cli.rs (167 lines, 9 #[test] fns)"
  modified: []

key-decisions:
  - "Ship as ONE atomic commit (D-23 row 1 verbatim): `test(cli): fixture-driven subcommand + edge coverage [TEST-03]`. fixtures.rs and cli.rs are independent tests but the plan frontmatter and <objective> both mandate one commit for the plan."
  - "Task 3 `decode_roundtrips_default_dump_per_fixture` scoped to 2 fixtures (minimal_elf, zero_fill_1kib) — consistent with `ROUNDTRIP_FIXTURES` in common/mod.rs. Other 3 fixtures would trigger Problem B (length mismatch), deferred to REF-04."
  - "Used `common::FixtureEntry` type alias instead of inline `&[(&str, fn() -> Vec<u8>)]` (Rule 3 clippy::type_complexity fix — same alias already shipped in Plan 02)."
  - "No `use predicates::prelude::*;` — tests use qualified `predicates::str::contains(...)` paths to avoid an unused-import warning (predicates 3 doesn't need the prelude for `str::` constructors). cli.rs imports only `predicates::prelude::PredicateBooleanExt` to enable `.and()` / `.not()` on predicate values."

patterns-established:
  - "When clippy workspace lint bar kicks in on new test files: the two recurring offenders are `clippy::type_complexity` (use `FixtureEntry`) and `clippy::doc_markdown` (wrap fixture names + `HashMap` in backticks). Both fixed inline per Rule 3 — no allows added."

requirements-completed: [TEST-03]

metrics:
  duration: "~8 min"
  tasks: 4
  commits: 1
  files_changed: 2
  files_created: 2
  total_test_delta: 13  # 4 fixtures + 9 cli
completed: "2026-04-24"
---

# Phase 3 Plan 03: Fixture-driven subcommand + edge coverage — Summary

## One-liner

`crates/base60-cli/tests/fixtures.rs` проверяет каждую subcommand (`dump` / `analyze` / `decode` / `completions`) против 5-fixture корпуса, а `crates/base60-cli/tests/cli.rs` пинит 9 edge-контрактов (stdin, BrokenPipe exit-0, NO_COLOR + `--color` precedence, `--skip`/`--length` clamps, decoder `"99"+"invalid"` error-message pin) — всё через единственную sanctioned spawn-точку в `tests/common/mod.rs`, spawn-discipline gate остаётся зелёным.

## What shipped

### `crates/base60-cli/tests/fixtures.rs` (new, 113 lines, 4 tests)

| Test fn                                      | Iterates              | Assertion                                              |
| -------------------------------------------- | --------------------- | ------------------------------------------------------ |
| `dump_produces_expected_prefix_per_fixture`  | all 5 fixtures        | stdout starts with `"00000000  "`                       |
| `analyze_summary_is_sane_per_fixture`        | all 5 fixtures        | stdout contains both `"bytes"` и `"entropy"`            |
| `decode_roundtrips_default_dump_per_fixture` | 2 byte-identical fxs  | `decode(dump(bytes)) == bytes`                         |
| `completions_shells_all_succeed`             | [bash zsh fish elvish powershell] | exit 0 + non-empty stdout                  |

Всего 5 + 5 + 2 + 5 = 17 child-spawns по 4-м тест-функциям.

### `crates/base60-cli/tests/cli.rs` (new, 167 lines, 9 tests)

| Test fn                                              | Pins contract                                                    |
| ---------------------------------------------------- | ---------------------------------------------------------------- |
| `stdin_piped_dump_produces_output`                   | stdin → dump wires through to renderer                           |
| `dump_exits_zero_on_broken_pipe`                     | `BrokenPipe` → exit 0 (via `common::spawn_with_closed_stdout`)   |
| `no_color_env_suppresses_ansi_on_auto`               | `NO_COLOR=1` + `--color=auto` → no `\x1b[`                       |
| `color_always_forces_ansi_even_in_pipe`              | `--color=always` overrides TTY detection, emits ANSI             |
| `color_never_suppresses_ansi_with_clicolor_force`    | `--color=never` beats `CLICOLOR_FORCE=1`                         |
| `skip_past_end_yields_empty_dump`                    | `--skip=1024` on 14 B input → exit 0, empty dump (saturation)    |
| `length_clamps_to_available_bytes`                   | `--length=9999` clamps to input size                             |
| `zero_skip_is_identity`                              | `--skip=0` pins offset-0 first line (catches off-by-one drift)   |
| `decoder_invalid_digit_99_error_contains_the_digit`  | `.failure()` + stderr contains `"99"` AND `"invalid"` (D-13)     |

Декодер-pin — главный долгосрочный контракт: pins `decode::parse_run` error format (`crates/base60-cli/src/decode.rs:103-109`) против Phase 4 REF-03 refactor. Два substring одновременно — достаточно tight, чтобы поймать drift, достаточно loose, чтобы пережить re-word "at pair N" phrasing.

## Verification (D-24 — full phase gate)

Все 7 команд D-24 зелёные после единственного atomic commit:

| Gate | Command | Result |
|------|---------|--------|
| Fixtures | `cargo test -p base60 --test fixtures --locked` | 4 passed |
| CLI edges | `cargo test -p base60 --test cli --locked` | 9 passed |
| Matrix | `cargo test -p base60 --test roundtrip --locked` | 1 passed (28 cells) |
| Spawn gate | `cargo test -p xtask --test spawn_discipline --locked` | 1 passed |
| Env gate | `cargo test -p xtask --test env_discipline --locked` | 1 passed |
| Workspace | `cargo test --workspace --all-targets --locked` | 182 passed (was 169 → +13) |
| Clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 warnings |
| Fmt | `cargo fmt --all --check` | 0 diffs |
| Doc | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` | 0 warnings |

## Commits

| Hash    | Message                                                       |
| ------- | ------------------------------------------------------------- |
| e93dee6 | test(cli): fixture-driven subcommand + edge coverage [TEST-03] |

Единственный commit — план предписывает atomic ship (`commit_message` во frontmatter + `<objective>` явно говорят "as one atomic commit"). Оба файла независимы (fixtures.rs и cli.rs не делят символы кроме `mod common;`), но оба относятся к одному requirement TEST-03 и оба делят одну verification surface (D-24).

## Test count breakdown (phase 3 cumulative)

- Plan 01 (`refactor(cli)`): +1 (`all_contains_every_format_variant` exhaustiveness).
- Plan 02 (`test(integration)` + `test(xtask)` + narrowing): +2 (1 matrix, 1 spawn-gate) + env-gate uncounted (shipped in Phase 2).
- Plan 03 (`test(cli)`): +13 (4 fixtures + 9 cli).
- **Phase-3 total: +16 tests** beyond Phase-2 baseline.

## Decoder-error pin — exact predicate & location

- **File:** `crates/base60-cli/tests/cli.rs:160-167`
- **Test fn:** `decoder_invalid_digit_99_error_contains_the_digit`
- **Input:** `"00000000  00:00:00:00:00:00:00:00:00:00:99  |........|\n"` — 11 pairs (10 × `00` + `99` tail), один `find_digit_run` boundary.
- **Assertion:**
  ```rust
  .failure()
  .stderr(predicates::str::contains("99").and(predicates::str::contains("invalid")))
  ```
- **Pinned source:** `crates/base60-cli/src/decode.rs:103-109` — `format!("line {line_no}: invalid base-60 digit {digit} at pair {}", i + 1)`.

## `analyze::write_summary` substrings — verification (RESEARCH A1 closed)

Task 1 pre-flight (grep) подтвердил:
- `"bytes"` unconditional на 3+ линиях `write_summary`: `bytes         {}` (210), `unique bytes  {unique} / 256` (221), `top bytes` (233), `regions       ascii=... high-entropy=... low-entropy=` (249, через "entropy" часть тоже содержит "bytes" через "byte" — но для "bytes" достаточно первых трёх).
- `"entropy"` unconditional на 2 линиях: `entropy       {:.3} bits/byte` (211), `regions ... high-entropy=... low-entropy=` (249).

Оба substring независимы от размера/содержания fixture — `analyze_summary_is_sane_per_fixture` safe для всех 5.

## Deviations from Plan

### Auto-fixed during D-24 gate run (Rule 3 — blocking)

Task 4 (D-24 gate) первоначально упал на `cargo clippy -- -D warnings` с 7 errors в `fixtures.rs`:

**1. [Rule 3 — Blocking] `clippy::doc_markdown` × 6 on fixtures.rs:2-3, 14**
- **Issue:** fixture names (`minimal_elf`, `minimal_png`, `minimal_zip`, `zero_fill_1kib`, `hello_world`) и `HashMap` не обёрнуты в backticks в doc-комментариях.
- **Fix:** wrap в backticks (`` `minimal_elf` `` etc, `` `HashMap` ``).
- **Files modified:** `crates/base60-cli/tests/fixtures.rs` (doc-комментарии строк 1-8 и 15).

**2. [Rule 3 — Blocking] `clippy::type_complexity` on fixtures.rs:74**
- **Issue:** inline `&[(&str, fn() -> Vec<u8>)]` для `roundtrip_fixtures`.
- **Fix:** reuse `common::FixtureEntry` type alias (shipped в Plan 02 precisely для этого кейса).
- **Files modified:** `crates/base60-cli/tests/fixtures.rs` (импорт + annotation строки 12 и 74).

**3. [Rule 1 — Bug] `unused_imports` on fixtures.rs:12 `use predicates::prelude::*;`**
- **Issue:** prelude не используется (все predicate-функции вызываются по qualified path).
- **Fix:** удалить строку, оставить только `use common::{base60_cmd, fixtures};`.
- **Files modified:** `crates/base60-cli/tests/fixtures.rs`.

Все 3 авто-фикса внутри единственного atomic commit `e93dee6` — не отдельными коммитами, т.к. D-24 запрещает broken intermediate states, а план сам предписывает atomic ship.

`cli.rs` clippy-чистый сразу (разделяет стиль с `common/mod.rs` Plan 02 — там все вопросы уже решены).

---

**Total deviations:** 3 auto-fixes (2 blocking + 1 unused import) на fixtures.rs. Никаких scope/architecture deviations. Никакого production-кода не изменено.

## Issues Encountered

- Никакие из 9 cli-тестов не оказались flaky на локальном запуске (Linux debug build). Broken-pipe тест работает за счёт того, что `zero_fill_1kib` (1024 B → ~128 dump lines) всегда заполняет pipe buffer прежде чем child успеет finish — детерминированно.
- `assert_cmd::Command::write_stdin` + `.get_output().stdout.clone()` — единственный идиоматичный способ feed-тить output одного spawn в stdin следующего без intermediate файла. Подтверждено в `decode_roundtrips_default_dump_per_fixture`.

## Known Stubs

None — все helpers wired, все fixtures генерируются в реальном времени, никаких `TODO`/`FIXME`, все тесты имеют реальные assertions (не `assert!(true)`).

## Threat Flags

None — ни новых endpoints, ни auth paths, ни trust-boundary поверхностей. `base60_cmd()` hermetic env-clear + пер-тест `.env("NO_COLOR"/"CLICOLOR_FORCE", ...)` инъекция — только child; env-discipline gate подтверждает отсутствие mutation shared env-а.

## Next Phase Readiness

- Phase 3 `TEST-03` fully covered: `fixtures.rs` + `cli.rs` закрывают как per-subcommand, так и edge контракты.
- Phase 4 (`REF-03` — decoder refactor) может полагаться на `decoder_invalid_digit_99_error_contains_the_digit` как safety-net: любое изменение `decode::parse_run` error format, которое теряет substring `"99"` ИЛИ `"invalid"`, будет поймано мгновенно.
- `REF-04` (length-preserving decode + JSON/HTML decode) остаётся отложенным — `fixtures.rs::decode_roundtrips_default_dump_per_fixture` готова будет расшириться с 2 fixtures до 5, как только `REF-04` закроет Problem B, без изменений самой тестовой логики.
- Orchestrator должен запустить `/gsd-check-phase 3` для верификации SC1–SC4.

## Unresolved Questions

- Можно ли ужать декодер-pin до `.code(1)` вместо `.failure()`? anyhow не гарантирует exit code — но эксперимент показал, что `base60 decode` на ошибке всегда выходит с `1`. Сейчас `.failure()` достаточно — RESEARCH Open Q#3 recommendation сохранён.
- Надо ли покрыть `--format=json` / `--format=html` smoke-тестом в Plan 03-03 (shape-check без roundtrip)? Plan-specifics в prompt советовал это, но план-PLAN.md не включил — fixtures.rs::dump_produces_expected_prefix_per_fixture уже проверяет offset-prefix для plain/ansi, расширять до JSON/HTML имело бы смысл в составе REF-04 одновременно с decode-стороной.

## Notes (RU)

- Три clippy-фикса шорт-списком: backticks на имена fixtures, `FixtureEntry` вместо inline fn-pointer type, удалить неиспользуемый prelude-импорт. Ни одного `#[allow(...)]`.
- `common/mod.rs` уже хорошо подготовлен Plan 02 — весь surface (`FixtureEntry`, fixture-фабрики, `spawn_with_closed_stdout`) достался нам бесплатно. Plan 03 — чистый консьюмер.
- Atomic commit: TEST-03 как один requirement → один коммит (D-23 row 1 verbatim), даже несмотря на два независимых test-файла.
- После Phase 3 полностью готов к `/gsd-check-phase 3` и затем к Phase 4 REF-03/REF-04 refactor work.

## Self-Check: PASSED

- **Files created (verified `[ -f ... ]`):**
  - `crates/base60-cli/tests/fixtures.rs` — FOUND
  - `crates/base60-cli/tests/cli.rs` — FOUND
- **Commit (verified `git log --oneline | grep e93dee6`):**
  - `e93dee6 test(cli): fixture-driven subcommand + edge coverage [TEST-03]` — FOUND
- **D-24 full gate (all green):**
  - `cargo test --workspace --all-targets --locked` — 182 passed, 0 failed
  - `cargo clippy --workspace --all-targets --locked -- -D warnings` — exit 0
  - `cargo fmt --all --check` — exit 0
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` — exit 0
  - `cargo test -p xtask --test spawn_discipline --locked` — 1 passed
  - `cargo test -p xtask --test env_discipline --locked` — 1 passed

---
*Phase: 03-roundtrip-matrix-fixture-integration*
*Completed: 2026-04-24*
