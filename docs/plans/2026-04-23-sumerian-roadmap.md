# Sumerian Base-60 Viewer — Roadmap

**Дата:** 2026-04-23
**Статус:** Approved (in progress)
**Scope:** 7 последовательных фаз. Порядок оптимизирован на финальное качество.

## Progress

| # | Phase | Status |
|---|-------|--------|
| 1 | CI | ✓ shipped |
| 2 | Lens | ✓ shipped |
| 3 | Analyze | ✓ shipped (diff-mode deferred, see Phase 3 updates) |
| 4 | Navigator | ◐ in progress — cursor + lens toggle + cuneiform offset shipped; bookmarks/search/semantic-jumps/persistence remain |
| 5 | Decode + format | — |
| 6 | Workspace split | — |
| 7 | Release engineering | — |

## Overview

Текущее состояние: CLI + TUI, base-60 heatmap, mmap reader, strict clippy (pedantic + nursery + cargo).
Цель: превратить утилиту из hex-dump-альтернативы в **уникальный шумерский инструмент** с археологической эстетикой, семантическими линзами и экосистемной интеграцией.

Порядок и обоснование:

```
Phase 1 (CI)      ──► 2 (Lens A)  ──► 3 (Archaeologist C) ──► 4 (Navigator B)
                                                                     │
                                                                     ▼
Phase 7 (release) ◄── 6 (workspace) ◄── 5 (decode+format)   ◄───────┘
```

- **Phase 1 (CI)** первой — ловит regressions на каждой следующей фазе.
- **Phase 2 (Lens A)** до analysis: `Lens` trait переиспользуется analysis для аннотаций регионов.
- **Phase 3 (Archaeologist C)** до Navigator: entropy sparkline + region detection поставляют данные для семантических прыжков в B.
- **Phase 4 (Navigator B)** последним из core — композитор Lens + Analysis в единый интерактив.
- **Phase 5 (decode/format)** после того как все фичи отображения стабильны — pipe-экосистема «замораживает» семантику вывода.
- **Phase 6 (workspace split)** — API наиболее стабилен именно сейчас, core выносится без последующих перемешиваний.
- **Phase 7 (release eng)** — completions, man page, cargo-dist, README, asciinema. Только после API freeze.

---

## Phase 1 — CI Foundation

**Commit prefix:** `ci:`
**Goal:** Regression net с нулевого дня каждой следующей фазы.

### Scope

- `.github/workflows/ci.yml` с матрицей:
  - OS: `ubuntu-latest`, `macos-latest`, `windows-latest`
  - Toolchain: `1.95.0` (MSRV), `stable`, `beta`
- Jobs:
  - `test` — `cargo test --all-targets --locked`
  - `clippy` — `cargo clippy --all-targets --locked -- -D warnings` (inherits `[lints.clippy]` из Cargo.toml)
  - `fmt` — `cargo fmt --all --check`
  - `build-release` — `cargo build --release --locked`
  - `doc` — `cargo doc --no-deps --locked` с `RUSTDOCFLAGS="-D warnings"`
- Кэш через `Swatinem/rust-cache@v2`.
- Fail-fast: `false` (хочу видеть результаты всех OS даже при падении одного).

### Acceptance

- Все 4 jobs зелёные на push в main и на PR.
- Badge `[![CI]()]` добавлен в README (создаётся в Phase 7).
- Concurrency: новые коммиты в PR отменяют предыдущие прогоны.

### Effort

~1 час.

---

## Phase 2 — Sumerian Lens (ex-A)

**Commit prefix:** `feat(lens):`
**Goal:** Единый флаг `--lens` добавляет шумерский семантический слой к выводу.

### Scope

- `src/lens.rs` — trait `Lens` + реализации:

  ```rust
  pub(crate) trait Lens: Send + Sync {
      fn name(&self) -> &'static str;
      fn render(&self, chunk_be: u64) -> String;
  }
  ```

- `src/cuneiform.rs` — LUT `[&str; 60]` отображения 0..59 в клинопись (комбинация U+12079 `𒁹` для 1, U+1230B `𒌋` для 10) + ASCII fallback.
- Линзы:
  - **`time`** — u64 интерпретируется как **Sumerian gar ticks** (1 gar ≈ 2 современные секунды).
    Декомпозиция:
    - `day  = u / (12 · 60 · 60)`
    - `beru = (u / (60 · 60)) % 12` (двойной час)
    - `uš   = (u / 60) % 60` (≈ 2 минуты каждый)
    - `gar  = u % 60` (≈ 2 секунды каждый)
    - Рендер: `{day}d {beru:02}𒁹 {uš:02}:{gar:02}`
    - Флаг `--time-scale=gar|sec|ms` для альтернативных интерпретаций (default `gar`).
  - **`angle`** — u64 как milliarcseconds (до сих пор живое sexagesimal применение):
    - `°    = u / 3_600_000`
    - `′    = (u / 60_000) % 60`
    - `″    = (u / 1000) % 60`
    - `ms   = u % 1000`
    - Рендер: `{°:03}°{′:02}′{″:02}.{ms:03}″`
  - **`tablet`** — ведущие нули → пробел (historical accuracy: в ранней шумерской записи нуля не было).
    - `--purist` флаг (отдельно или как часть `tablet`): placeholder ` `, не `00`.
    - Опционально обрамляет строку ASCII-углами `⌐ ¬` имитируя края таблички.
  - **`cuneiform`** — каждая из 11 base-60 цифр рендерится `cuneiform::glyph(d)`.
    - Runtime-детект ширины через `unicode-width::UnicodeWidthStr` — если любая из глифов > 2 cells, fallback на десятичные пары.
    - Если `NO_UNICODE=1` в env, безусловный fallback.
- Поток:
  - `dump::write_line` и `dump::styled_line` получают `lens: Option<&dyn Lens>`.
  - Если `Some`, после ASCII-колонки добавляется `  {lens.render(chunk_be_u64)}`.
  - `PALETTE_ANSI` получает новое поле `lens: "\x1b[35m"` (magenta) для отличия колонки.

### CLI

```
--lens=<MODE>        none|time|angle|tablet|cuneiform (default: none)
--time-scale=<UNIT>  gar|sec|ms (default: gar, только с --lens=time)
--purist             placeholder-пробел в tablet mode
```

### Non-goals

- TUI toggle линзы (делается в Phase 4).
- Обратная конверсия линзы (отдельная идея, не в scope).
- Сериализация линзы в JSON (Phase 5).

### Acceptance

- `base60 --lens=time /bin/ls` показывает `0d 00𒁹 00:00` в первой строке (zero chunk).
- `base60 --lens=angle` на u64=3_600_000 даёт `001°00′00.000″`.
- `base60 --lens=cuneiform /bin/ls` рендерит клинописью; `NO_UNICODE=1 base60 --lens=cuneiform` → fallback на digits.
- `base60 --lens=tablet --purist` показывает пробелы вместо ведущих нулей.
- Все линзы покрыты unit-тестами с граничными значениями (0, u64::MAX, single-digit, etc).
- Clippy clean, fmt clean.
- Phase 1 CI зелёный.

### Effort

~1-2 дня. Новые файлы: `lens.rs`, `cuneiform.rs`. Модификации: `cli.rs`, `color.rs`, `dump.rs`, `main.rs`.

---

## Phase 3 — Data Archaeologist (ex-C)

**Commit prefix:** `feat(analyze):`
**Goal:** Аналитический слой — Shannon entropy, byte histogram, region detection, diff-mode.

### Scope

- `src/analyze.rs` — single module, no separate `detect.rs` (detection is one more pass over the same data and benefits from sharing helpers):

  ```rust
  pub(crate) struct Analysis {
      pub(crate) total_bytes: usize,
      pub(crate) window_size: usize,
      pub(crate) entropy: f32,                 // overall bits/byte
      pub(crate) entropy_windows: Vec<f32>,    // Shannon bits/byte per window
      pub(crate) byte_freq: Box<[u32; 256]>,
      pub(crate) regions: Vec<Region>,
  }

  pub(crate) struct Region {
      pub(crate) start: usize,
      pub(crate) end: usize,
      pub(crate) kind: RegionKind,
  }

  pub(crate) enum RegionKind { Ascii, HighEntropy, LowEntropy }
  ```

  UTF-8 detection dropped from scope — ASCII runs cover the valuable
  signal; full UTF-8 scanning would need its own phase. `Mixed` dropped
  because it only exists to disambiguate UTF-8 vs ASCII, which we don't
  detect.

- Parameters: `window_size` (default 256, minimum 64) via the analyzer's
  internal clamp. `--no-precompute` escape hatch dropped — eager
  precompute on a 1 GB file still fits in a few MB, and the lazy path
  would only help in theoretical configurations nobody asked for.
- Detection rules (live inside `analyze.rs`):
  - ASCII run ≥ 4 graphic/space → `RegionKind::Ascii`
  - Window entropy ≥ 7.5 → `HighEntropy` (likely compressed/encrypted)
  - Window entropy ≤ 1.0 → `LowEntropy` (zero fill, padding)
- Subcommand `base60 analyze <FILE>` — non-TUI summary of total bytes,
  entropy, region counts, top-5 bytes, first ASCII string previews.
- **Diff mode deferred.** `base60 --diff a.bin b.bin` moves to a follow-up
  commit on the same phase once `base60-core` (Phase 6) exposes the
  diff-friendly primitives; doing it now would couple the viewer to
  the `similar` crate before a clean boundary exists.

### TUI integration (предварительная)

- Footer-panel вырастает до `Constraint::Length(5)`:
  - Row 0: status (как сейчас).
  - Rows 1-3: entropy sparkline (ratatui Sparkline widget).
  - Rows 4: mini byte histogram (64 buckets: 60 digits + 4 special).
- Этот блок оставляет полную интеграцию на Phase 4.

### Acceptance

- `base60 analyze /bin/ls` выводит correct counts, pass smoke test на 10KB, 1MB.
- ASCII strings в `/bin/ls` определяются как `RegionKind::Ascii` (verified — finds actual ELF strings like `_IO_stdin_used` and the dynamic linker path).
- `head -c 1K /dev/urandom | base60 analyze` показывает entropy близкую к 8.0.
- CI зелёный.
- **Diff mode and TUI footer sparkline moved to Phase 3.follow-up once Phase 6 exposes a stable library boundary.**

### Effort

~2 дня (actual: core analyze ~half-day; diff + TUI footer still outstanding).

---

## Phase 4 — Tablet Navigator (ex-B)

**Commit prefix:** `feat(tui):`
**Goal:** Интерактивный TUI с курсором, закладками, поиском, семантическими прыжками, toggle линз.

### Scope

**Done:**

- **Cursor mode** (`hjkl`, `^`/`$`, `g`/`G`, Ctrl-d/u, PgUp/PgDn) — byte-granular cursor with reverse-video highlight in the ASCII column. Viewport auto-tracks via `scroll_into_view`. Shipped in commit `2e71004`.
- **Lens toggle** — Shift-`L` cycles `None → Time → Angle → Tablet → Cuneiform → None`. Active mode shown in the status bar. Shipped in commit `6e6a27d`.
- **Cuneiform cursor offset** — when the cuneiform lens is active, the status bar renders the cursor's absolute offset as Sumero-Babylonian wedges alongside the hex form (`𒁹𒌋 𒑰`). Shipped in `6e6a27d`.

**Remaining:**

- **Bookmarks:** `m<a-z>` ставит, `'<a-z>` прыгает. Gutter-маркер глиф `𒑭`. Хранение в `ViewState`.
- **Search:** `/` открывает input в status-bar (modal mode). Парсер: hex literal `0xdeadbeef`, string literal `"foo"`, byte sequence `de ad be ef`. `n`/`N` между hits. `Esc` отменяет search mode.
- **Ctrl-C behaviour:** в search mode — `Esc` для cancel, `Ctrl-C` — полный выход из TUI (signal-like, консистентно с CLI-инструментами).
- **Semantic jumps:** `]p` next printable run, `]z` next zero run, `]e` next entropy spike (использует `Analysis` из Phase 3). `[` — предыдущие. Requires wiring `analyze::analyze()` into TUI startup.
- **Persistent state:**
  - Файл: `$XDG_STATE_HOME/base60/positions.json` (fallback: `~/.local/state/base60/`).
  - Ключ: `blake3(canonical_path + first_4KB)` — надёжнее mtime.
  - Значение: `{scroll, cursor, bookmarks, active_lens}`.
  - Запись только при `q` / `ESC` quit. Adds a `blake3` dependency.

### Design skeleton

```rust
enum Mode { Normal, Search { buf: String }, BookmarkSet, BookmarkJump }

struct ViewState {
    // existing fields
    cursor: Option<usize>,      // byte index
    bookmarks: HashMap<char, usize>,
    mode: Mode,
    last_search: Option<SearchQuery>,
    active_lens: Option<Box<dyn Lens>>,
    analysis: Option<Arc<Analysis>>,
    session_id: Option<SessionKey>,
}
```

### Acceptance

- `hjkl` перемещают курсор, подсветка синхронна между 3-4 колонками.
- `m a` → скролл → `'a` восстанавливает позицию.
- `/foo` → подсветка всех вхождений, `n` прыгает к следующему, `N` к предыдущему.
- `]e` прыгает к следующему high-entropy региону.
- `L` переключает линзы.
- Повторный запуск на том же файле восстанавливает scroll + cursor + bookmarks.
- Smoke test: ручная сессия на /bin/ls + /dev/urandom dump.

### Effort

~2-3 дня.

---

## Phase 5 — Decode & Format outputs

**Commit prefix:** `feat(io):`
**Goal:** Pipe-friendly экосистема.

### Scope

- **`base60 decode`** (subcommand, matching the `analyze` shape): `cat dump.txt | base60 decode > original.bin`.
  - Парсер: line-based, извлекает 11 base-60 пар вида `NN:NN:...` (8 bytes BE per match).
  - Толерантен к mixed-content (dump с ASCII/offset колонками). Ручной one-pass-scan, без `regex` dependency (11 digits × 3 chars is small enough for `memchr`-style scanning).
  - Error-out при невалидных digits (≥ 60) с указанием строки.
- **`--format=<MODE>`** (top-level flag on `view`):
  - `ansi` (default, current behaviour)
  - `plain` — no ANSI, для grep/awk pipelines
  - `json` — newline-delimited (ndjson) per-line object: `{"offset":N,"digits":[...],"ascii":"..","lens":"..."}`
  - `html` — self-contained report: inline CSS с тем же heatmap, `<pre>` с span-ами
- **URL-safe base60 encoding** (lands in `base60-core` at Phase 6; Phase 5 adds the CLI entry point):
  - Алфавит `0-9A-Za-x` (62 ascii printable минус `y`/`z` для ambiguity) → ровно 60 символов.
  - `pub fn encode_url(bytes: &[u8]) -> String`
  - `pub fn decode_url(s: &str) -> Result<Vec<u8>, DecodeError>`
  - Применение: короче hex для hash prefixes (`b60:3QvZ7aK` в URL).

### Acceptance

- `base60 --format=plain /bin/ls | base60 decode | cmp - /bin/ls` → silent (roundtrip).
- `base60 --format=json /bin/ls | jq -c '.offset' | head -3` → три строки.
- `base60 --format=plain` не содержит ANSI escapes (grep без `-P`).
- `base60 --format=html > out.html`, открывается в браузере с цветами.
- Unit-тесты на `encode_url`/`decode_url` roundtrip (property test via `proptest`).

### Effort

~1-1.5 дня.

---

## Phase 6 — Workspace split (core + cli)

**Commit prefix:** `refactor:` (с тегом `BREAKING` в body)
**Goal:** Вынос pure-логики в `no_std`-совместимый library crate.

### Scope

- Новая структура:

  ```
  ├── Cargo.toml              # [workspace]
  ├── crates/
  │   ├── base60-core/        # #![no_std] совместимый
  │   │   ├── Cargo.toml
  │   │   └── src/lib.rs      # u64_to_base60, encode_url, decode_url, Lens, CuneiformLUT
  │   └── base60-cli/         # binary
  │       ├── Cargo.toml
  │       └── src/
  │           ├── main.rs
  │           ├── cli.rs
  │           ├── reader.rs
  │           ├── dump.rs
  │           ├── color.rs
  │           ├── tui.rs
  │           └── analyze.rs  # нужен std
  └── .github/workflows/ci.yml
  ```

- `base60-core` dependencies: none (`no_std`). Exports: `u64_to_base60`, `u64_from_base60`, `encode_url`, `decode_url`, `CuneiformGlyph`, `Lens` trait (если возможно — trait objects требуют alloc, так что feature-flag).
- `base60-core` features: `std` (default off), `alloc` (default off).
- `base60-cli` depends on `base60-core = { path = "../base60-core", features = ["std","alloc"] }`.
- Миграция делается atomic commits per-file где возможно; один финальный commit `chore: workspace split` который двигает файлы + reorganises Cargo.toml.
- **Git blame preservation:** использовать `git mv` для переноса и добавить `.git-blame-ignore-revs` с одной строкой reformat-commit'а (если такой нужен). После split `git blame --follow` работает автоматически через history detection.

### Acceptance

- `cargo test --workspace` — все тесты зелёные.
- `cargo build -p base60-core --no-default-features --target thumbv7em-none-eabihf` — core собирается для embedded target.
- `cargo install --path crates/base60-cli` ставит working binary.
- Downstream mini-crate может `use base60_core::{u64_to_base60, encode_url}` без `std`.
- CI matrix добавляет `embedded-check` job (no_std build).

### Effort

~1.5-2 дня. Осторожный рефакторинг, но каждое touchpoint локален.

---

## Phase 7 — Release engineering

**Commit prefix:** `chore(release):`
**Goal:** Professional-grade distribution.

### Scope

- **Shell completions:** `clap_complete` генерация, subcommand `base60 completions <SHELL>`.
- **Man page:** `clap_mangen`, сгенерировано в `build.rs` → `target/man/base60.1`.
- **`cargo-dist`:** `.github/workflows/release.yml` на tag `v*`, builds для `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. Artifacts как `.tar.gz`/`.zip` + checksums + provenance.
- **README.md:**
  - Hero asciinema демо (`.cast` файл, играется через asciinema-player.js в README через GitHub-trick или externally hosted).
  - Install секция: `cargo install`, `brew tap`, `scoop`, precompiled binary.
  - Feature matrix: что делает каждый `--lens`, `--format`, subcommand.
  - CI badge, crates.io badge (если published — под вопросом).
- **docs.rs:** публичные items в `base60-core` с rustdoc examples (doctest-verified).
- **`CHANGELOG.md`:** Keep a Changelog format, v0.1.0 release notes на основе commit history.

### Acceptance

- `base60 completions zsh > ~/.zfunc/_base60` работает.
- `man base60` показывает корректную page.
- Tag `v0.1.0` триггерит release workflow, артефакты появляются в GitHub Releases.
- README видим на GitHub с работающими badges.
- `cargo doc --open` для `base60-core` показывает документацию без warnings.

### Effort

~0.5-1 день.

---

## Rollout policy

- **Branching:** каждая фаза в отдельной ветке `phase-{N}-{slug}`, PR на main. Не squash-merge — preserve atomic commits.
- **Commit style:** Conventional commits (см. commit prefix каждой фазы), body объясняет **почему**, не **что**.
- **Quality gate на фазу:**
  - `cargo test --all-targets --locked` — zero failures
  - `cargo clippy --all-targets --locked -- -D warnings` — clean (pedantic + nursery + cargo)
  - `cargo fmt --all --check` — formatted
  - CI зелёный на 3 OS × 3 toolchains (после Phase 1)
  - Manual smoke test binary: `/bin/ls`, `/dev/urandom | head -c 1K`, empty file, 100MB file
- **Rollback:** если фаза введёт regression, revert commit(ов) — atomic history делает это чистым.

## Decisions locked

- `--lens=time` использует Sumerian time units (**beru:UŠ:gar**, 1 gar ≈ 2 sec). Альтернативные масштабы через `--time-scale`.
- Phase 4 Ctrl-C — **exit TUI**, Esc — cancel search/bookmark mode.
- Phase 3 entropy — **eager precompute** (single-pass при загрузке), `--no-precompute` escape hatch.
- Phase 6 — **отдельный PR**, добавить `.git-blame-ignore-revs`, git mv для file-tracking.
- Каждая фаза — **отдельный PR** (не батч). Upstream merge после green CI.

## Open questions

- Phase 2 — `--lens=cuneiform` width detection: runtime через `unicode-width`, достаточно или пробовать `terminfo`?
- Phase 3 — `RegionKind::Utf8` пересекается с Ascii — единый kind `Text { encoding }` или отдельные?
- Phase 5 — JSON format: newline-delimited (ndjson) или array?
- Phase 7 — публиковать ли `base60-core` на crates.io, или держать path-only?
