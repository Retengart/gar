# Phase 3: Roundtrip Matrix + Fixture Integration — Pattern Map

**Mapped:** 2026-04-24
**Файлов под создание/правку:** 9 (6 NEW + 3 EDIT)
**Аналогов найдено:** 8 / 9 (один файл без прямого аналога — первый `[lib]` в бинарной crate)

## File Classification

| Файл | Статус | Роль | Data Flow | Ближайший аналог | Match Quality |
|------|--------|------|-----------|------------------|---------------|
| `crates/base60-cli/src/lib.rs` | NEW | library-root / module-index | re-export | `crates/base60-core/src/lib.rs:1-29` | style-match (модуль-декларации + `pub use`) |
| `crates/base60-cli/src/main.rs` | EDIT | binary-shim (refactor) | request-response | `crates/base60-core/src/lib.rs` (re-export-only style) | style-match |
| `crates/base60-cli/src/cli.rs` | EDIT | CLI dispatch-table (refactor) | — | **Тот же файл**: `LensMode::ALL` на строках 47-53 → модель для `Format::ALL` | exact (same-file precedent) |
| `crates/base60-cli/Cargo.toml` | EDIT | dev-dep manifest | — | `crates/xtask/Cargo.toml:11-12` (walkdir dev-dep) + workspace root `[lib]` convention | role-match |
| `crates/base60-cli/tests/common/mod.rs` | NEW | test-helper module | fixture/build | отсутствует прямой; базовый style = inline `#[cfg(test)] mod tests` в `main.rs:173-227` | style-match |
| `crates/base60-cli/tests/roundtrip.rs` | NEW | integration test (matrix) | spawn-and-compare | отсутствует (первый integration test) | none — use RESEARCH.md §Pattern 2 |
| `crates/base60-cli/tests/fixtures.rs` | NEW | integration test (happy-path × subcommand) | spawn-and-assert | как выше | none — use RESEARCH.md §Code Examples/fixtures.rs |
| `crates/base60-cli/tests/cli.rs` | NEW | integration test (edges) | spawn-and-assert | как выше | none — use RESEARCH.md §Code Examples/cli.rs |
| `crates/xtask/tests/spawn_discipline.rs` | NEW | static-analysis gate | line-walker | **`crates/xtask/tests/env_discipline.rs:1-167`** (Phase 2) | EXACT — прямой fork по D-16 |

---

## Pattern Assignments

### 1. `crates/base60-cli/src/lib.rs` (NEW) — library root

**Аналог:** `crates/base60-core/src/lib.rs:1-29` — единственный файл в репо, где уже делается `mod X; ... pub use ...` в нужном стиле.

**Паттерн модуль-декларации + re-export** (скопировать структуру, подменить имена):

```rust
// crates/base60-core/src/lib.rs:1-29
//! Core building blocks shared by the `base60` CLI and any downstream
//! library consumer.
//!
//! The crate exposes three layers:
//! ...

pub mod convert;
pub mod cuneiform;
pub mod lens;
pub mod url;

pub use convert::{DIGITS, u64_to_base60};
pub use cuneiform::{ascii_fallback_forced, ascii_pair, glyph};
pub use lens::{AngleLens, CuneiformLens, Lens, TabletLens, TimeLens, TimeScale};
pub use url::{DecodeError, decode_u64, encode_u64};
```

**Адаптация для Phase 3** (узкая поверхность, D-07 минимум):
- Модуль-декларации переезжают из `main.rs:11-21` как есть (`mod analyze; mod chunk; mod cli; ...`) — остаются `pub(crate)`-level через отсутствие `pub` (это бинарник-с-lib, а не библиотека-первого-класса).
- `pub use cli::{LensMode, Format};` — единственный re-export (D-07).
- `pub fn run() -> anyhow::Result<()> { ... }` хостит текущее тело `main()` из `main.rs:32-43`.
- Модуль-docstring `//!` в одну-две строки в стиле `core/src/lib.rs:1-2`.

**Crate-атрибуты**, которые сейчас на `main.rs:1-7`, остаются там — lib.rs не нужен свой `#![forbid(...)]` (он наследуется workspace `[lints]`).

---

### 2. `crates/base60-cli/src/main.rs` (EDIT) — шим

**Аналог:** стилевой — весь репо-паттерн "binary crates вызывает lib". Ближайший структурный пример — `base60-core/src/lib.rs` как "re-export-only" (после того как `main.rs` станет shim, он тоже будет однострочно-делегирующий).

**Текущее тело `main`** (`crates/base60-cli/src/main.rs:32-43`), которое нужно ПЕРЕНЕСТИ в `lib.rs::run`:

```rust
fn main() -> Result<()> {
    let args = cli::Cli::parse();
    match &args.command {
        None => run_view(&args.view),
        Some(Command::Analyze(a)) => run_analyze(a),
        Some(Command::Decode(d)) => run_decode(d),
        Some(Command::Completions(c)) => {
            run_completions(c);
            Ok(())
        }
    }
}
```

**Результирующий `main.rs` (D-08):**

```rust
//! Entry point for the `base60` binary viewer.

fn main() -> anyhow::Result<()> {
    base60::run()
}
```

**КРИТИЧНО** при миграции (D-08 explicit):
- Все `mod X;` декларации (`main.rs:11-21`) уезжают в `lib.rs`.
- Весь `#[cfg(test)] mod tests { ... }` блок (`main.rs:173-227`) со всеми 5 `#[serial(env)]` тестами и их SAFETY-комментариями уезжает в `lib.rs` **verbatim** — аннотации не меняются.
- `use` клаузы (`main.rs:23-30`) перенести в `lib.rs`.
- `#![forbid(unsafe_op_in_unsafe_fn)]` и `#![allow(clippy::redundant_pub_crate)]` из `main.rs:1-7` — **оставить на `main.rs`** (это crate-root атрибуты бинаря; lib имеет свои crate-root атрибуты, но их предоставляет workspace).

---

### 3. `crates/base60-cli/src/cli.rs` (EDIT) — `Format::ALL` + `LensMode::ALL` pub

**Аналог: ТОТ ЖЕ ФАЙЛ**. `LensMode::ALL` в строках 47-53 — прямой шаблон для `Format::ALL`.

**Паттерн `LensMode::ALL`** (`crates/base60-cli/src/cli.rs:44-53`):

```rust
// TODO(phase-3 TEST-01): iterate LensMode::ALL in production code
// (see 01-02-SUMMARY.md), then drop the dead_code allow below.
#[allow(dead_code)]
pub(crate) const ALL: &[Self] = &[
    Self::None,
    Self::Time,
    Self::Angle,
    Self::Tablet,
    Self::Cuneiform,
];
```

**Правки для Phase 3:**

1. **Widen** `LensMode::ALL`:
   - `pub(crate) const ALL:` → `pub const ALL:` (D-09).
   - **Удалить** `#[allow(dead_code)]` и TODO-комментарий — Phase 3 и есть тот потребитель из TODO.
   - Doc-comment (строки 41-43) оставить.

2. **Add** `Format::ALL` (D-10) — вставить сразу после `enum Format { ... }` (`cli.rs:119-132`) зеркальным блоком:

```rust
impl Format {
    /// Every variant in declaration order. Tests iterate this slice to
    /// keep the 4-format × lens × fixture matrix exhaustive.
    pub const ALL: &[Self] = &[Self::Ansi, Self::Plain, Self::Json, Self::Html];
}
```

   Тип — `&[Self]` (не `[Self; 4]`), чтобы сохранить симметрию с `LensMode::ALL` (RESEARCH §Format::ALL Shape Decision, option A).

**Паттерн exhaustiveness-теста** — копировать из inline-tests того же файла (`cli.rs:277-291`):

```rust
// crates/base60-cli/src/cli.rs:277-291
#[test]
fn all_contains_every_variant_in_cycle_order() {
    let mut walk = LensMode::None;
    for &expected in LensMode::ALL {
        assert_eq!(walk, expected);
        walk = walk.cycle();
    }
    assert_eq!(walk, LensMode::None);
}
```

**Адаптация для `Format::ALL`** (единственный новый тест, без `cycle`/`label`):

```rust
#[test]
fn all_contains_every_format_variant() {
    for variant in [Format::Ansi, Format::Plain, Format::Json, Format::Html] {
        assert!(
            Format::ALL.contains(&variant),
            "Format::ALL missing variant {variant:?}",
        );
    }
    assert_eq!(Format::ALL.len(), 4, "Format::ALL length drift");
}
```

Тест добавляется в существующий `#[cfg(test)] mod tests { ... }` в `cli.rs:272-330` (тот же блок, что содержит `all_contains_every_variant_in_cycle_order` и `all_methods_total_over_all`).

---

### 4. `crates/base60-cli/Cargo.toml` (EDIT) — `[lib]` + dev-deps

**Аналог для `[lib]` entry:** workspace-root pattern (нет прямого в crate-Cargo, так как все crate были single-target). Структурно ближе всего — `crates/xtask/Cargo.toml` (где `[dev-dependencies]` идиоматично).

**Текущий manifest** (`crates/base60-cli/Cargo.toml:1-30`):

```toml
[package]
name = "base60"
# ...

[[bin]]
name = "base60"
path = "src/main.rs"

[dependencies]
anyhow = "1.0.102"
clap = { version = "4.6.1", features = ["derive"] }
# ...

[dev-dependencies]
serial_test = { version = "3", default-features = false }

[lints]
workspace = true
```

**Правки (D-06, D-22):**

```toml
# Добавить СРАЗУ перед [[bin]] (conventional order: [lib] первым, [[bin]] вторым):
[lib]
name = "base60"
path = "src/lib.rs"

# Расширить [dev-dependencies]:
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
serial_test = { version = "3", default-features = false }  # already present
```

**Паттерн для `walkdir`-style dev-dep** из `crates/xtask/Cargo.toml:11-12`:

```toml
# crates/xtask/Cargo.toml:11-12
[dev-dependencies]
walkdir = "2"
```

Caret-версии `"2"`, `"3"` — D-22 explicit, совпадает с precedent'ом `walkdir = "2"` и `serial_test = "3"`.

---

### 5. `crates/base60-cli/tests/common/mod.rs` (NEW) — helper module

**Аналог (style reference):** inline `#[cfg(test)] mod tests` в `main.rs:173-227` — задаёт тон doc-comment'ов на `fn`, SAFETY-комментариев и использования `serial_test`. Integration-тесты отличаются отсутствием `use super::*;` (вместо этого `use base60::{...}`).

**Doc-comment стиль модуля** (Phase 3 §specifics: "Module-level docstrings use `//!` in one line"):

```rust
//! Shared spawner, fixture factories, and roundtrip-assertion helper for
//! the base60-cli integration tests. Only file under `tests/` allowed to
//! call `assert_cmd::Command::cargo_bin` (enforced by spawn-discipline gate).
```

**Паттерн `pub(crate)`-visibility для helpers** — из `cli.rs:13, 25, 89, 107, 120, 138` (каждый `pub(crate)`-item имеет doc comment). Пример:

```rust
// crates/base60-cli/src/cli.rs:87-103
/// Turn a [`LensMode`] into a live trait object, or [`None`] for
/// [`LensMode::None`]. Shared by the CLI dump path and the TUI so the
/// `L` toggle and the `--lens` flag go through the same constructor.
///
/// `scale` only affects [`LensMode::Time`]; `purist` only affects
/// [`LensMode::Tablet`]. Unused combinations are silently ignored.
#[must_use]
pub(crate) fn build_lens(mode: LensMode, scale: TimeScale, purist: bool) -> Option<Box<dyn Lens>> {
    // ...
}
```

**Содержимое mod.rs** — три блока (D-14, D-15) с готовыми сниппетами в RESEARCH:
- `base60_cmd()` → RESEARCH §Pattern 1 (строки 257-277 research-документа).
- `LensConfig` + `ALL_LENS_CONFIGS` → RESEARCH §Pattern 3 (строки 362-406).
- `assert_roundtrip` + `hex_window` → RESEARCH §Pattern 4 (строки 421-455).
- Fixture factories (`minimal_elf`/`minimal_png`/`minimal_zip`/`zero_fill_1kib`/`hello_world`) → RESEARCH §Fixture Factories (строки 484-639), байты pre-computed, copy-paste-ready.
- (Optional, для BrokenPipe) `spawn_with_closed_stdout` → RESEARCH §BrokenPipe Test Shape (строки 927-945).

**`#[derive]` bar** (из `cli.rs:12, 24, 106, 119`): `#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]` — `LensConfig` (D-15) должен иметь минимум `Copy, Clone, Debug` (workspace lint `missing_debug_implementations = warn`).

---

### 6. `crates/base60-cli/tests/roundtrip.rs` (NEW) — 140-cell matrix

**Прямого аналога в репо нет.** Используем RESEARCH §Pattern 2 (строки 287-345) — готовый caller-skeleton.

**Стилевая привязка** (из `main.rs:173-227`): `#[cfg(test)]` НЕ нужен на файлах под `tests/` — это integration tests, libtest уже знает. Одна разница с inline: `use base60::{...}` вместо `use super::*;` (CONTEXT §code_context).

Планнер копирует RESEARCH-скелетон (единый `#[test] fn roundtrip_matrix_byte_identical`) + три вложенных цикла (fixture × lens × fmt = 140).

**Failure-messaging шаблон** — задан RESEARCH §Pattern 4 и D-20 (cell label + first-diverge index + ±8-byte hex window).

---

### 7. `crates/base60-cli/tests/fixtures.rs` (NEW) — per-subcommand × 5 fixtures

**Прямого аналога нет.** Готовый skeleton — RESEARCH §Code Examples/fixtures.rs (строки 727-809): 4 теста (`dump_produces_expected_prefix_per_fixture`, `analyze_summary_is_sane_per_fixture`, `decode_roundtrips_default_dump_per_fixture`, `completions_shells_all_succeed`).

**ВНИМАНИЕ планнеру** (RESEARCH line 810-811): перед коммитом проверить, что substrings `"bytes"` и `"entropy"` в `analyze_summary_is_sane_per_fixture` реально есть в выхлопе `analyze::write_summary` (файл `crates/base60-cli/src/analyze.rs` — spot-check требуется).

---

### 8. `crates/base60-cli/tests/cli.rs` (NEW) — edges

**Прямого аналога нет.** Готовый skeleton — RESEARCH §Code Examples/cli.rs (строки 815-897) + BrokenPipe-test §BrokenPipe Test Shape (строки 951-961).

**Pin-test для decoder error** (D-13, Pitfall 8) — фиксирует текущий формат ошибки из `crates/base60-cli/src/decode.rs:103-108`:

```rust
// crates/base60-cli/src/decode.rs:103-108
return Err(io::Error::new(
    io::ErrorKind::InvalidData,
    format!(
        "line {line_no}: invalid base-60 digit {digit} at pair {}",
        i + 1
    ),
));
```

Pin-assertion (из RESEARCH §Pitfall 8):

```rust
.stderr(predicates::str::contains("99").and(predicates::str::contains("invalid")));
```

---

### 9. `crates/xtask/tests/spawn_discipline.rs` (NEW) — gate

**Аналог: `crates/xtask/tests/env_discipline.rs:1-167`** — прямой fork по D-16. Copy-paste-and-swap.

#### Module-doc паттерн (copy + swap invariant)

```rust
// crates/xtask/tests/env_discipline.rs:1-11
//! Env-discipline gate: every `env::set_var` / `env::remove_var` call in
//! `base60-core/src/**/*.rs` and `base60-cli/src/**/*.rs` must live inside a
//! test function bearing `#[serial(env)]` — no alternate keys, no production
//! code exceptions. Phase 2 (TEST-04) invariant.
//!
//! Walks both crate sources via `walkdir`. Line-based parser: for each
//! `env::set_var` / `env::remove_var` occurrence, walks upward to find the
//! enclosing `fn`, then confirms the preceding attribute block contains
//! exactly `#[serial(env)]` (no `#[serial(no_color)]` etc.) AND the function
//! also bears `#[test]`. Any deviation fails the test with a precise
//! `file:line` diagnostic.
```

→ Swap to spawn-discipline language; reference Phase 3 (TEST-03).

#### Imports + `const` декларация (exact copy)

```rust
// crates/xtask/tests/env_discipline.rs:13-17
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Relative roots from this crate's manifest to walk.
const WALK_ROOTS: &[&str] = &["../base60-core/src", "../base60-cli/src"];
```

→ Заменить `WALK_ROOTS` (мн.число) на одиночный `WALK_ROOT: &str = "../base60-cli/tests"`; добавить `const EXEMPT_DIR: &str = "common";`

#### Walkdir + filter skeleton (exact copy)

```rust
// crates/xtask/tests/env_discipline.rs:26-50
#[test]
fn every_env_mutation_is_serialised() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut failures: Vec<String> = Vec::new();

    for root in WALK_ROOTS {
        let root_path: PathBuf = Path::new(manifest_dir).join(root);
        assert!(
            root_path.is_dir(),
            "walk root does not exist: {}",
            root_path.display()
        );

        for entry in WalkDir::new(&root_path).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().is_none_or(|e| e != "rs") {
                continue;
            }

            let path = entry.path();
            let contents = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let lines: Vec<&str> = contents.lines().collect();
```

**Изменения для spawn_discipline.rs (D-16):**
1. Имя теста: `fn no_raw_spawn_outside_common()`.
2. `assert!(root_path.is_dir(), ...)` → мягкий `if !root_path.is_dir() { return; }` — gate no-op когда `tests/` ещё не существует (см. RESEARCH §Spawn-Discipline, строки 1058-1060): позволяет commit 1 из D-23 прошит до commit 2.
3. Одиночный `WALK_ROOT`, не цикл по `WALK_ROOTS`.
4. После проверки extension — вставить exemption check:
   ```rust
   if entry.path().components().any(|c| c.as_os_str() == EXEMPT_DIR) {
       continue;
   }
   ```

#### Comment-filter (exact copy)

```rust
// crates/xtask/tests/env_discipline.rs:52-58
for (idx, line) in lines.iter().enumerate() {
    // Skip commented lines to avoid false positives on `SAFETY:`
    // comments that mention `env::set_var` for documentation.
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        continue;
    }
```

→ Keep verbatim, комментарий-обоснование переписать на "avoid false positives in doc examples mentioning the helper".

#### Pattern-match (swap invariant)

```rust
// crates/xtask/tests/env_discipline.rs:60-64
let mentions_mutation =
    line.contains("env::set_var(") || line.contains("env::remove_var(");
if !mentions_mutation {
    continue;
}
```

→ Swap to:
```rust
if !line.contains("Command::cargo_bin") {
    continue;
}
```

Substring `"Command::cargo_bin"` — D-16 explicit, ловит `assert_cmd::Command::cargo_bin` и гипотетический `std::process::Command::cargo_bin`.

#### Relative-path diagnostic (exact copy of шаблона)

```rust
// crates/xtask/tests/env_discipline.rs:66-71
let line_no = idx + 1;
let rel = path
    .strip_prefix(manifest_dir)
    .unwrap_or(path)
    .display()
    .to_string();
```

→ Copy verbatim.

#### Failure-message (swap content, keep shape)

```rust
// crates/xtask/tests/env_discipline.rs:95-114
if !has_test {
    failures.push(format!(
        "{rel}:{line_no}: env mutation in non-`#[test]` \
         function — env-discipline forbids env mutation \
         outside tests"
    ));
}
// ...
```

→ Single push per match, message из D-17:
```rust
failures.push(format!(
    "{rel}:{line_no}: raw Command::cargo_bin outside tests/common/ \
     — use base60_cmd() from tests/common/mod.rs",
));
```

#### Final assert! (exact copy)

```rust
// crates/xtask/tests/env_discipline.rs:119-124
assert!(
    failures.is_empty(),
    "env-discipline gate failed ({count} issue(s)):\n{details}",
    count = failures.len(),
    details = failures.join("\n"),
);
```

→ Copy verbatim, swap `env-discipline` → `spawn-discipline`.

#### Helpers `find_enclosing_fn` / `collect_attributes_above` — **НЕ нужны**

Phase 2's gate проверяет enclosing-`fn` + attribute-block. Phase 3's gate — просто "файл вне common/ содержит substring" → skip helpers; готовый код в RESEARCH §Spawn-Discipline (строки 1050-1112) — ~45 строк против 167 в env_discipline.

---

## Shared Patterns

### Workspace lint propagation

**Источник:** `crates/xtask/Cargo.toml:14-15`, `crates/base60-cli/Cargo.toml:29-30`.

```toml
[lints]
workspace = true
```

**Применимо к:** ВСЕМ crate-manifest'ам. Phase 3 не трогает эту строку; новые `tests/` файлы наследуют `pedantic + nursery + cargo -D warnings` автоматически через существующий `workspace = true`.

**Бюджет для tests/:**
- Doc-comments на каждом `pub(crate)`-и-выше item (включая helpers в `common/mod.rs`).
- `#[must_use]` на каждой чистой функции возвращающей значение.
- `#[derive(Debug)]` на каждом public-type (`missing_debug_implementations = warn`).
- Явные `#[allow(clippy::cast_*)]` на умышленных кастах (или checked-формы).

### `pub(crate)` default + doc comment on every pub-item

**Источник:** `crates/base60-cli/src/cli.rs` — каждый `pub(crate)` item имеет doc-строку. Примеры:
- `cli.rs:10-21` (ColorChoice enum + variant docs)
- `cli.rs:23-38` (LensMode enum + variant docs)
- `cli.rs:87-103` (build_lens fn + `# Errors`/`# Panics`-free body-level doc)

**Применимо к:** helper-функциям в `tests/common/mod.rs` (они `pub(crate)` или `pub` в scope test-bin).

### `#[derive]` canonical шаблон

**Источник:** `cli.rs:12, 24, 106, 119` — `#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]`.

**Применимо к:**
- `LensConfig` (D-15): минимум `Copy, Clone, Debug`; `PartialEq`/`Eq` — nice-to-have для assertions.
- Любых новых enum'ов/struct'ов в `common/mod.rs`.

### `#[allow(dead_code)]` → drop on use

**Источник:** `cli.rs:44-46`:

```rust
// TODO(phase-3 TEST-01): iterate LensMode::ALL in production code
// (see 01-02-SUMMARY.md), then drop the dead_code allow below.
#[allow(dead_code)]
pub(crate) const ALL: &[Self] = &[
```

**Применимо к:** Phase 3 это и есть TODO-consumer → удалить `#[allow(dead_code)]` и TODO-комментарий при расширении до `pub`.

### `serial_test = "3"` идиома (не задействуется в Phase 3, но сохраняется для справки)

**Источник:** `crates/base60-cli/Cargo.toml:27` и `main.rs:186, 199, 207, 217, 223`.

**Применимо к:** inline-тестам в `lib.rs` (переедут verbatim из `main.rs`). Integration-тесты Phase 3 НЕ мутируют parent env (они дают env через `.env(...)` на assert_cmd child), поэтому `serial_test` им не нужен. Env-discipline gate не фаулит.

### Cargo `[dev-dependencies]` shape

**Источник:** `crates/xtask/Cargo.toml:11-12` и `crates/base60-cli/Cargo.toml:26-27`.

```toml
[dev-dependencies]
walkdir = "2"  # xtask
```

```toml
[dev-dependencies]
serial_test = { version = "3", default-features = false }  # base60-cli existing
```

**Применимо к:** Phase 3 добавляет `assert_cmd = "2"` и `predicates = "3"` в `[dev-dependencies]` `base60-cli` — caret-style, без feature-маппингов (D-22).

---

## No Analog Found

| Файл | Почему нет аналога | Источник шаблона |
|------|---------------------|-------------------|
| `crates/base60-cli/tests/roundtrip.rs` | Первый integration test в репо. | RESEARCH §Pattern 2 (paste-ready) |
| `crates/base60-cli/tests/fixtures.rs` | Первый integration test. | RESEARCH §Code Examples/fixtures.rs |
| `crates/base60-cli/tests/cli.rs` | Первый integration test + первый BrokenPipe-тест. | RESEARCH §Code Examples/cli.rs + §BrokenPipe Test Shape |

**Все три файла** идут через helper-модуль `tests/common/mod.rs` и получают стилевую привязку к inline-test convention из `main.rs:173-227` (naming, `#[test]`-атрибуты, assertion-стиль через `assert!`/`assert_eq!`).

---

## Metadata

**Analog search scope:**
- `crates/xtask/tests/` — 1 file found (`env_discipline.rs`).
- `crates/base60-core/src/lib.rs` — re-export/module-decl pattern.
- `crates/base60-cli/src/cli.rs` — `LensMode::ALL` precedent.
- `crates/base60-cli/src/main.rs` — inline test-module style.
- `crates/base60-cli/Cargo.toml` — existing dev-dep shape.
- `crates/xtask/Cargo.toml` — walkdir dev-dep precedent.

**Files scanned:** 6 repo files + 2 planning docs (CONTEXT, RESEARCH).

**Pattern extraction date:** 2026-04-24.

**Key insight:** 7 из 9 файлов имеют прямой (`env_discipline.rs` → `spawn_discipline.rs`, `LensMode::ALL` → `Format::ALL`, `core/lib.rs` → `cli/lib.rs`) или стилевой аналог; 3 новых integration-test файла не имеют прямого precedent'а в репо, но RESEARCH.md содержит paste-ready skeletons для всех трёх. Планнер не обязан повторно открывать source-файлы — выдержки выше покрывают все точки копирования.
