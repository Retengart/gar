---
phase: 03-roundtrip-matrix-fixture-integration
reviewed: 2026-04-24T00:00:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - Cargo.lock
  - crates/base60-cli/Cargo.toml
  - crates/base60-cli/src/cli.rs
  - crates/base60-cli/src/lib.rs
  - crates/base60-cli/src/main.rs
  - crates/base60-cli/tests/cli.rs
  - crates/base60-cli/tests/common/mod.rs
  - crates/base60-cli/tests/fixtures.rs
  - crates/base60-cli/tests/roundtrip.rs
  - crates/xtask/tests/spawn_discipline.rs
findings:
  critical: 0
  warning: 1
  info: 5
  total: 6
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-04-24
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

Обзор затронул test-infrastructure фазу 3: тонкий `lib.rs` shim, `Format::ALL`, фикстурные фабрики, матричный roundtrip (28 ячеек), CLI-edge тесты и spawn-discipline gate. Код идиоматичен, lints workspace-уровня удовлетворяются, `unsafe` блоки не добавлены, инъекций и shell-escape проблем нет (все аргументы — литералы). Dev-dep прирост в `Cargo.lock` (`assert_cmd`, `predicates`, `base60-core` как dev) оправдан тремя манифестными добавлениями.

Проблем критичного уровня не найдено. Одна warning-фикс — дублирование `fmt_value` в `roundtrip.rs`, хотя `clap::ValueEnum` уже даёт канонический string-маппинг через `to_possible_value().unwrap().get_name()`. Остальные пять замечаний — информационные (документация, неиспользуемые helper'ы, мелкая очистка).

## Warnings

### WR-01: Дубликат маппинга `Format → CLI string` в `roundtrip.rs`

**File:** `crates/base60-cli/tests/roundtrip.rs:94-101`
**Issue:** Функция `fmt_value` повторяет четвёртый (после `clap::ValueEnum`-derive в `cli.rs:119`, `Format::ALL` в `cli.rs:138`, и документации в `cli.rs:232-240`) раз маппинг вариантов `Format` в их CLI-написание. При добавлении нового варианта появится тихое расхождение: `Format::ALL` и `clap`-derive обновятся через exhaustive match в `cli.rs`, но const-функция здесь скомпилируется только после ручной правки — и тест может начать генерировать неверный `--format=…` (clap отклонит). Есть и идиоматичная альтернатива: `f.to_possible_value().unwrap().get_name()`, которую `ValueEnum`-derive уже предоставляет.

**Fix:**
```rust
// Заменить const fn fmt_value на:
fn fmt_value(f: Format) -> &'static str {
    use clap::ValueEnum;
    f.to_possible_value()
        .expect("Format is not skipped in clap derive")
        .get_name()
}
// Либо пометить Format::ALL как `pub const` + derive строк от clap —
// и удалить fmt_value целиком, вытащив имена через `.get_name()`
// прямо в форматтере.
```
Даёт одну точку истины для CLI-написаний и превращает любое будущее расхождение в compile-time ошибку.

## Info

### IN-01: `pub use cli::{LensMode, Format}` не имеет doc-комментария на уровне re-export

**File:** `crates/base60-cli/src/lib.rs:28`
**Issue:** Re-export `pub use cli::{Format, LensMode};` виден через публичный API библиотеки (`base60::LensMode`, `base60::Format`), но без doc-комментария. Оригинальные определения в `cli.rs` документированы, и rustdoc подтянет их описание через `pub use`, так что `RUSTDOCFLAGS=-D warnings` не ругается. Но короткий комментарий "re-exported for integration tests that iterate the matrix" сэкономит будущим читателям обратный поиск, зачем бинарный crate раскрывает эти типы.

**Fix:**
```rust
/// Re-exported for integration tests (and any downstream consumer) that
/// need to iterate the matrix without hard-coding variant lists.
pub use cli::{Format, LensMode};
```

### IN-02: `Format::ALL` не имеет `#[must_use]` на уровне константы (N/A сейчас, но стоит держать в уме)

**File:** `crates/base60-cli/src/cli.rs:138`
**Issue:** `Format::ALL` — `pub const` slice, `#[must_use]` к нему неприменимо (атрибут работает на функциях и типах). Это корректно. Отмечаю как info потому, что аналогичный `LensMode::ALL` (`cli.rs:46`) тоже без `#[must_use]` — симметрия соблюдена, но если когда-нибудь появится функция-геттер, добавьте `#[must_use]`. Ничего исправлять сейчас не надо.

**Fix:** Никакого действия, просто якорь на будущее.

### IN-03: `tests/common/mod.rs` содержит `pub fn minimal_png` / `minimal_zip` / `hello_world` / `minimal_elf` + `ALL_LENS_CONFIGS` — часть используется только из одного тест-крейта

**File:** `crates/base60-cli/tests/common/mod.rs:66-149, 196-204`
**Issue:** `fixtures::minimal_png` / `minimal_zip` вызываются только из `tests/fixtures.rs`; `ALL_LENS_CONFIGS` / `ROUNDTRIP_FIXTURES` / `ROUNDTRIP_FORMATS` / `assert_roundtrip` / `LensConfig` — только из `tests/roundtrip.rs`; `spawn_with_closed_stdout` — только из `tests/cli.rs`. Это по дизайну: integration-тесты в Rust компилируются как отдельные крейты, и `#![allow(dead_code)]` на file scope (`mod.rs:24`) документирует именно такой pattern ("each test file pulls only a subset of helpers"). Upside: одна точка истины для фикстур. Downside: любой будущий не-используемый helper скроется под `dead_code`-allow. Mitigation (не обязательная сейчас): добавьте в каждый тест-файл assert'ы или грош-комментарий "uses: A, B, C from common::*", чтобы trivially grep-able'ся проверять покрытие.

**Fix:** Не требуется. Документировано в `mod.rs:17-24`. Упомянуто как hazard на будущее.

### IN-04: `let _ = cell_start` в `roundtrip.rs` — формальная заглушка

**File:** `crates/base60-cli/tests/roundtrip.rs:91`
**Issue:** Строка `let _ = cell_start;` после `#[cfg(debug_assertions)]`-блока нужна чтобы «заглушить unused-when-not-debug». Но `Instant::now()` — side-effect-free для timing, и `clippy::unused_variables` обычно игнорирует `let _ = …`. Альтернатива чище: оберните создание `cell_start` в `#[cfg(debug_assertions)]` тоже.

**Fix:**
```rust
#[cfg(debug_assertions)]
let cell_start = std::time::Instant::now();

// ... existing code ...

#[cfg(debug_assertions)]
{
    let elapsed = cell_start.elapsed();
    if elapsed.as_millis() > 500 {
        eprintln!("WARN: cell '{cell_label}' took {elapsed:?} (budget 500ms)");
    }
}
// Строку `let _ = cell_start;` удалить.
```
Убирает `Instant::now()`-вызов в release-тестах и финальный no-op.

### IN-05: `all_fixtures()` в `fixtures.rs` — аллокация `Vec<(&str, Vec<u8>)>` в каждом тесте

**File:** `crates/base60-cli/tests/fixtures.rs:16-24`
**Issue:** `all_fixtures()` возвращает `Vec<(&str, Vec<u8>)>` и вызывается дважды (в `dump_produces_expected_prefix_per_fixture` и `analyze_summary_is_sane_per_fixture`). Каждая фикстура аллоцируется по два раза при запуске тест-бинарника. Pattern в `common::ROUNDTRIP_FIXTURES` — `&[(&str, fn() -> Vec<u8>)]` — чище: отложенная инициализация, никакого общего state. Не критично (тесты всё равно спавнят `base60` subprocess на каждый кейс, стоимость `Vec::from` пренебрежима), но рассинхронизация стилей между двумя тест-файлами.

**Fix:**
```rust
fn all_fixtures() -> &'static [(&'static str, fn() -> Vec<u8>)] {
    &[
        ("minimal_elf", fixtures::minimal_elf),
        ("minimal_png", fixtures::minimal_png),
        ("minimal_zip", fixtures::minimal_zip),
        ("zero_fill_1kib", fixtures::zero_fill_1kib),
        ("hello_world", fixtures::hello_world),
    ]
}

// в тесте:
for (label, factory) in all_fixtures() {
    let bytes = factory();
    // …
}
```
Также снимает `#[allow(dead_code)]`-давление на `label` (`fixtures.rs:41` `let _ = label`).

---

## Observations (non-issue)

Кратко, чтобы зафиксировать контекст ревью:

- **`tests/common/mod.rs::spawn_with_closed_stdout`** использует raw `std::process::Command` намеренно, gate `crates/xtask/tests/spawn_discipline.rs:19-20` исключает `common/` — pattern корректен и задокументирован.
- **`env_clear()` + восстановление `PATH`/`SystemRoot`/`USERPROFILE`** в `base60_cmd()` (`common/mod.rs:38-54`) — canonical Windows-safe env-leak mitigation, комментарий (`mod.rs:32-37`) ясно объясняет зачем.
- **`unsafe { std::env::remove_var }`** в `lib.rs:198/206/214/217` не новый — перенесён из `main.rs`, pattern документирован SAFETY-комментарием и `#[serial(env)]`.
- **`base60-core` dev-dep** (`Cargo.toml:32`) используется в `common/mod.rs:27` (`base60_core::lens::TimeScale`) — не bloat.
- **`assert_cmd` 2.2.1 + `predicates` 3.1.4** пришли с transitive-хвостом (`bstr`, `difflib`, `float-cmp`, `normalize-line-endings`, `predicates-core`, `predicates-tree`, `termtree`, `wait-timeout`) — ~8 новых dev-deps на ~94 строки в `Cargo.lock`, разумный trade-off для matрицы из 28 cases × 4+ spawn'ов на тест.
- **`base60-core` zero-dep invariant** не нарушен: все новые зависимости ушли в `[dev-dependencies]` бинарного crate.
- `cli.rs` тесты (`all_contains_every_variant_in_cycle_order`, `all_methods_total_over_all`, `all_contains_every_format_variant`) закрывают D-08/D-09 — тройная проверка синхронизации `ALL` ↔ `cycle` ↔ `build_lens` ↔ `persist::parse_lens`.

---

_Reviewed: 2026-04-24_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
