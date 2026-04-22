# Sumerian Base-60 Viewer — Roadmap

**Дата:** 2026-04-23
**Статус:** Proposed
**Scope:** 4 фазы (A, B, C, D). Порядок = приоритет дифференциации.

## Overview

Текущее состояние: CLI + TUI, base-60 heatmap, mmap reader, стрикт clippy.
Цель: превратить утилиту из hex-dump-альтернативы в **уникальный шумерский инструмент** с археологической эстетикой, семантическими линзами и экосистемной интеграцией.

Фазы независимы по коду, но имеют общий core (`convert.rs`, `color.rs`). A делается первой — даёт визуальный wow-эффект и reuse компонентов для B/C.

```
A (Lens)  ──► B (Navigator) ──► C (Archaeologist) ──► D (Ecosystem)
   │              │                    │                    │
   └─ cuneiform   └─ cursor state      └─ entropy calc     └─ encoder
      module         + bookmarks          + detectors         + lib crate
```

---

## Phase A — Sumerian Lens

**Goal:** Единый флаг `--lens` меняет семантику вывода, добавляя шумерский контекст к каждому чанку.

### Scope

- Новый модуль `src/lens.rs` с trait `Lens` (метод `interpret(&[u8; 8]) -> LensAnnotation`)
- Флаг `--lens={none,time,angle,tablet,cuneiform}` (default `none` = текущее поведение)
- Линзы:
  - `time` — u64 как timestamp: `DDDd HH:MM:SS.sss` (шумерское время)
  - `angle` — u64 как угол в milliarcseconds: `DDD°MM'SS"` (живое применение base-60!)
  - `tablet` — frame с «seal header» + placeholder-пробелы вместо leading zeros (historical accuracy)
  - `cuneiform` — цифры клинописью U+12000..U+12399 с auto-fallback на ASCII если ширина терминала ≠ моноширине
- Split-вывод: каждая линза добавляет ещё одну колонку справа от ASCII
- Новый модуль `src/cuneiform.rs` — LUT для 0..59 → клинописные глифы

### Non-goals

- Обратная конверсия (Phase D)
- Взаимодействие в TUI с линзой (Phase B добавит toggle key)

### Design sketch

```rust
// lens.rs
pub(crate) trait Lens {
    fn name(&self) -> &'static str;
    fn interpret(&self, chunk: [u8; 8]) -> String;
}

pub(crate) struct TimeLens;
pub(crate) struct AngleLens;
pub(crate) struct TabletLens;
pub(crate) struct CuneiformLens { fallback: bool }
```

`dump::write_line` получает `Option<&dyn Lens>`; если Some — append к выводу.
`styled_line` аналогично для TUI.

### Acceptance

- `base60 --lens time /bin/ls` показывает сумерское время справа от ASCII
- `base60 --lens cuneiform /bin/ls` рендерит глифами, при `$COLUMNS < N` fallback на base-60
- `--purist` подфлаг: пробел вместо `00` в ведущих нулях (только для `tablet`)
- Все линзы проходят `cargo test`, zero-alloc в hot path кроме cuneiform (String::push на каждую цифру)
- Clippy pedantic+nursery+cargo clean

### Risks

- Клинопись требует специфичных шрифтов (DejaVu, Noto Sans Cuneiform) — mitigation: runtime-width detection через `unicode-width`
- `--lens time` для файлов > 2⁶³ секунд даёт абсурдные значения — mitigation: showing raw ns для очень больших

### Effort estimate

~1-2 дня. Файлов: 3 новых (`lens.rs`, `cuneiform.rs`, `tests/lens_integration.rs`), 2 модификации (`dump.rs`, `cli.rs`).

---

## Phase B — Tablet Navigator

**Goal:** TUI превращается в «интерактивную клинописную табличку» — семантическая навигация + курсорная связка колонок.

### Scope

- Курсорный режим: `h/j/k/l` двигают байт-курсор; подсветка синхронно в offset/digits/ascii
- Base-60 координаты в статус-баре: `row 01:23  col 07` вместо hex offsets (опционально через `--coord-base60`)
- Закладки: `m<a-z>` → `'<a-z>` (vim-style), визуальный маркер в gutter (glyph 𒑭)
- Поиск: `/` → input в status-bar, ищет байт-паттерн (hex или ascii literal), подсветка hit
- Семантические прыжки: `]p` next printable run, `]z` next zero run, `]e` next entropy spike
- Toggle линзы (из Phase A): `L` циклит через lens modes
- Persistent state: последняя позиция per file → `$XDG_STATE_HOME/base60/positions.json`

### Non-goals

- Редактирование файла (read-only)
- Regex (только литеральный паттерн)

### Design sketch

```rust
// tui.rs extensions
struct CursorState { byte: usize }
struct BookmarkSet(HashMap<char, usize>)
enum Mode { Normal, Search, BookmarkSet, BookmarkJump }

struct ViewState {
    // existing fields
    cursor: CursorState,
    bookmarks: BookmarkSet,
    mode: Mode,
    search_query: String,
    active_lens: Option<Box<dyn Lens>>,
}
```

Entropy prejump: precompute Shannon per 256-byte window при загрузке, cache в Vec<f32>.

### Acceptance

- Курсор виден, навигация hjkl работает, подсветка синхронна
- `m a` → `'a` возвращает к позиции после скролла
- `/foo` подсвечивает все вхождения, `n`/`N` между ними
- `]p` перепрыгивает на следующий printable run
- State перезапускается: второй запуск на том же файле восстанавливает позицию

### Risks

- Bookmark persistence — файл-ключ через canonical path + xxhash первых 4КБ (mtime unreliable)
- Entropy window size trade-off — default 256, экспозить через флаг

### Effort estimate

~2-3 дня. Файлы: `tui.rs` расширение, новые `src/cursor.rs`, `src/bookmarks.rs`, `src/search.rs`, `src/entropy.rs`.

---

## Phase C — Data Archaeologist

**Goal:** Нижняя панель превращается в аналитическую: entropy sparkline, byte histogram, auto-detection регионов, diff-mode.

### Scope

- Статус-бар расширяется до footer-panel (Constraint::Length(5))
- Левая половина: Shannon entropy sparkline по окнам (ratatui Sparkline widget)
- Правая половина: byte histogram (64 bucket = base-60 digits + 4 special)
- Автодетект:
  - ASCII runs ≥ 4 chars → подсветка зелёным в gutter
  - UTF-8 валидные регионы → cyan
  - Высокая энтропия (>7.5) → red (сжато/шифровано)
  - Low entropy (<1.0) → gray (zero fills, паддинг)
- **Diff mode:** `base60 --diff a.bin b.bin` — side-by-side, расхождения подсвечены
- Новый CLI subcommand `base60 analyze file.bin` — non-TUI статистика в stdout

### Non-goals

- File carving / извлечение embedded форматов (отдельный проект)
- Structural parsing (ELF/PE/ZIP)

### Design sketch

```rust
// analyze.rs
pub(crate) struct Analysis {
    entropy_windows: Vec<f32>,
    byte_freq: [u32; 256],
    regions: Vec<Region>,  // {start, end, kind}
}

pub(crate) enum RegionKind { Ascii, Utf8, HighEntropy, LowEntropy, Mixed }
```

Diff через `similar` crate (myers diff на 8-byte чанках).

### Acceptance

- Footer отображается корректно, sparkline обновляется при скролле
- ASCII region в /bin/ls подсвечивается (strings table)
- `base60 analyze file.bin` выводит summary: total bytes, entropy mean, region counts
- `base60 --diff a.bin b.bin` подсвечивает отличающиеся строки

### Risks

- Entropy на малых окнах (<32 bytes) шумит — min window size 64
- Diff на больших файлах O(n²) — отключить `--diff` если any > 100MB или использовать chunked LCS

### Effort estimate

~2 дня. Файлы: `analyze.rs`, `diff.rs`, `footer.rs`, зависимость `similar = "2.7"`.

---

## Phase D — Ecosystem

**Goal:** Превратить утилиту из standalone в переиспользуемый компонент экосистемы.

### Scope

- **Library crate split:** workspace — `base60-core` (`no_std`-compatible) + `base60-cli` (binary)
- `--decode` обратная конверсия: `echo '01:23:45' | base60 --decode` → bytes (pipe-friendly)
- `--format={ansi,plain,json,html}`:
  - `plain` — no ANSI, для grep/awk
  - `json` — `{offset, digits, ascii, lens?}` per line
  - `html` — self-contained report с тем же heatmap (inline CSS)
- **URL-safe base60 encoding:** алфавит `0-9A-Za-x` (62 - 2 амбигуус = 60), функция `encode_url(bytes) -> String`, полезно для hash prefixes в URL короче чем hex
- Shell completions: `base60 completions {bash,zsh,fish}` через `clap_complete`
- Man page: `clap_mangen` в build.rs
- GitHub Actions CI: matrix test (linux/macos/windows × stable/beta), clippy, fmt, release build artifacts
- `cargo-dist` для релизов (бинари на tag)

### Non-goals

- Публикация на crates.io (после стабилизации API в v1.0)
- WASM target (хотя core crate будет совместим)

### Design sketch

```
workspace/
├── Cargo.toml          # [workspace]
├── crates/
│   ├── base60-core/    # no_std, pure conversion + encoding
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   └── base60-cli/     # binary, имеет все features + зависимость на core
│       ├── src/main.rs
│       └── Cargo.toml
└── .github/workflows/ci.yml
```

### Acceptance

- `cargo test --workspace` проходит
- `base60 --format json /bin/ls | jq '.offset'` работает
- `echo '01:23:45' | base60 --decode | xxd` даёт валидные байты
- CI зелёный на трёх OS × двух toolchains
- `cargo install --path crates/base60-cli` ставит working binary
- Downstream crate может `use base60_core::{u64_to_base60, encode_url}` в `no_std`

### Risks

- Workspace split ломает текущие пути — mitigation: delicate migration, все тесты остаются
- `no_std` требует удалить `String` из core — mitigation: LUT в `&'static [[u8; 2]; 60]` вместо format

### Effort estimate

~2-3 дня. Структурный рефакторинг, много touchpoints но каждый локальный.

---

## Rollout

**Порядок:** A → B → C → D (каждый делается через `/gsd-quick` отдельно)
**Branching:** ветка `phase-{a,b,c,d}` на группу, PR на main после прохождения тестов
**Commit strategy:** atomic commits per acceptance criterion
**Quality gate** (каждая фаза):
- `cargo test --all-targets` — все тесты зелёные
- `cargo clippy --all-targets -- -D warnings` — pedantic+nursery+cargo clean
- `cargo fmt --check` — formatted
- Manual smoke test binary на `/bin/ls` и `/dev/urandom | head -c 1K`

---

## Unresolved questions

- Фаза A `--lens=time` — интерпретировать u64 как секунды UNIX или как raw шумерские единицы?
- Фаза B — перехватывать `Ctrl-C` в search mode или эскейпить через Esc?
- Фаза C — хранить entropy precompute в памяти или lazy-вычислять при скролле?
- Фаза D — workspace миграция ломает git history blame, делать отдельным commit с `--follow` осознанно?
- Все фазы в одном PR или по одному PR на фазу?
- Нужна ли отдельная фаза **E — Documentation**: README с примерами, asciinema демо, docs.rs комментарии?
- Порядок A→B→C→D оптимален или предпочитаешь D→A→B→C (сначала infra для stable API)?
