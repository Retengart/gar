# Phase 4: Tighten `parse_run` + Close Coverage Gaps - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-24
**Phase:** 04-tighten-parse-run-close-coverage-gaps
**Areas discussed:** REF-04 length metadata, REF-04 HTML decode strategy, REF-03 migration strategy, Scope + ordering + TEST-05

---

## Gray Area Selection

| Option | Description | Selected |
|--------|-------------|----------|
| REF-04 length metadata | How dump communicates real length for ansi/plain/html outputs to decode | ✓ |
| REF-04 HTML decode strategy | Tag-strip vs state machine vs regex; format detection; malformed-input policy | ✓ |
| REF-03 migration strategy | In-place vs parse_run_strict alongside vs v2-rename; error-pin strength | ✓ |
| Scope + ordering + TEST-05 | Phase ordering (REF-04 first?); matrix widen commit location; TEST-05 sub-targets; plan count | ✓ |

**User's choice:** All four areas selected.

---

## Area 1: REF-04 length metadata

### Q1: Какой формат метаданных длины в ansi/plain/html dump?

| Option | Description | Selected |
|--------|-------------|----------|
| Trailing `# length=N` комментарий | Last line; ignored by old decode; additive | ✓ |
| First-line header `# base60 length=N` | Top of file; less streaming-friendly | |
| Суффикс на последней dump-строке | Inside top line; fragile to ascii column width | |
| Drop-incomplete-chunk | Hide the short tail; contradicts display guarantee | |

**User's choice:** Trailing comment line (recommended).

### Q2: Всегда эмитить length-метаданные, или только для некратных 8?

| Option | Description | Selected |
|--------|-------------|----------|
| Всегда | Uniform output; decoder has one path | ✓ |
| Только когда input % 8 != 0 | Preserves v1 byte-for-byte for aligned inputs | |

**User's choice:** Always (recommended).

### Q3: Что decode делает, если meta отсутствует (старый dump-файл)?

| Option | Description | Selected |
|--------|-------------|----------|
| 8-байтная граница как дефолт + warn | Backwards-compatible per PROJECT.md | ✓ |
| Strict: ошибка без meta | Breaks old dumps | |
| Опции `--legacy-8byte` и `--strict` | Scope creep with new flags | |

**User's choice:** 8-byte default with stderr warning (recommended).

### Q4: Имя поля? Формат значения?

| Option | Description | Selected |
|--------|-------------|----------|
| `# length=N` (decimal) | Clearest; uniform decimal | |
| `# base60:length=N` | Namespaced; redundant prefix | |
| `# bytes=N` (hex) | Hex matches offset column; shorter for >4 GiB | ✓ |

**User's choice:** `# bytes=N` with hex value.

**Synthesis in CONTEXT.md D-01/D-04:** Trailing `# bytes=0x<hex>\n` in ansi/plain. HTML: `<!-- bytes=0x<hex> -->` at end of `<body>`. JSON: NDJSON meta chunk `{"type":"meta","bytes":<decimal>}\n` (hex is non-idiomatic in JSON — decimal there only).

---

## Area 2: REF-04 HTML decode strategy

### Q1: Как decode разбирает HTML вход?

| Option | Description | Selected |
|--------|-------------|----------|
| Strip all tags + reuse find_digit_run | ~30 lines; simplest | |
| State machine on tag patterns | ~60 lines; strict; coupled to emit_html | ✓ |
| Regex-based extract | Heavy dep for one function | |

**User's choice:** State machine on tag patterns.

### Q2: Как decode знает, что input — HTML (а не plain)?

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-detect first non-empty line | Zero flags; works on stdin/file | |
| `--input-format` flag | Explicit; awkward for pipe scenarios | |
| Both (auto + flag override) | Auto default, flag overrides | ✓ |

**User's choice:** Both (auto-detect + flag override).

### Q3: HTML dump содержит <head>, <body>, inline CSS. Что с этим?

| Option | Description | Selected |
|--------|-------------|----------|
| Strip всё вне <body>, парсить body | Cleaner; robust for our emit | ✓ |
| Ignore shell; strip all tags одним пассом | Brittle if CSS contains `<` | |

**User's choice:** Strip outside body, parse body (recommended).

### Q4: Что если HTML input malformed?

| Option | Description | Selected |
|--------|-------------|----------|
| Skip bad lines, continue | Matches existing decode_stream tolerance | ✓ |
| Fail hard | Less forgiving | |

**User's choice:** Skip bad lines (recommended).

---

## Area 3: REF-03 migration strategy

### Q1: Как переводим parse_run к новому контракту?

| Option | Description | Selected |
|--------|-------------|----------|
| In-place rewrite + expanded error-pin | 1 caller; compact; compiler catches drift | ✓ |
| parse_run_strict alongside + migrate + delete | Pitfall 8 boilerplate for multi-caller world | |
| Rename to parse_run_v2, deprecate old | Internal fn; leaves dead weight | |

**User's choice:** In-place rewrite (recommended). Pitfall 8 advice drops because there's exactly one caller (`decode_stream` line 36).

### Q2: Какую защиту добавить к существующему decoder-тесту в cli.rs?

| Option | Description | Selected |
|--------|-------------|----------|
| Full-message contains | Pins line + pair + digit | ✓ |
| Два отдельных pin: число и позиция | Granular but verbose | |
| Только assert on err kind | Pitfall 8 warns against this | |

**User's choice:** Full-message contains (recommended).

### Q3: Нужны ли дополнительные error-path тесты?

| Option | Description | Selected |
|--------|-------------|----------|
| Да — 2-3 новых теста | Pin pair-position semantics at multiple sites | ✓ |
| Нет — достаточно текущего pin | Only one parse_run caller | |

**User's choice:** Yes, 2-3 new tests (recommended). Details: pair-1 error, pair-5 error, non-digit-run-line tolerance.

---

## Area 4: Scope + ordering + TEST-05

### Q1: Порядок work внутри фазы

| Option | Description | Selected |
|--------|-------------|----------|
| REF-04 первым, REF-03, parallel TEST-05 | Strengthens safety net before REF-03 | ✓ |
| REF-03 первым (historical roadmap order) | 28-cell matrix already covers error-pin | |
| TEST-05 первым | Coverage of untested paths before refactors | |

**User's choice:** REF-04 → REF-03 → TEST-05 (recommended).

### Q2: Расширение matrix 28 → 140: в этой фазе или отдельно?

| Option | Description | Selected |
|--------|-------------|----------|
| В этой фазе, в REF-04 коммите | Logical — REF-04 enables the widen | ✓ |
| Отдельным коммитом в этой фазе | Cleaner history, +1 commit | |
| Отдельной фазой (4.1 gap-closure) | Over-isolation | |

**User's choice:** In REF-04 commit (recommended).

### Q3: TEST-05 scope

| Option | Description | Selected |
|--------|-------------|----------|
| reader::load_file (mmap) + load_stdin | Closes TEST-05 SC2 | ✓ |
| persist::state_base_dir XDG→HOME | TEST-05 SC4 | ✓ |
| TUI TestBackend exit-with-save | TEST-05 SC3 | ✓ |
| search::Pattern round-trip | Deferred to Phase 5 fuzz | |

**User's choice:** All three TEST-05 sub-targets (recommended). search::Pattern deferred.

### Q4: Сколько планов/коммитов в Phase 4?

| Option | Description | Selected |
|--------|-------------|----------|
| 4 плана: REF-04, REF-03, TEST-05 reader, TEST-05 TUI/persist | Natural granularity | ✓ |
| 3 плана: REF-04, REF-03, TEST-05 всё | Mixes 3 disjoint files in one commit | |
| 5 планов: REF-04 split further | Too granular | |

**User's choice:** 4 plans (recommended).

---

## Claude's Discretion

- Exact HTML state-machine implementation shape (~60 lines, per D-05).
- Exact wording of the legacy-fallback stderr warning.
- Format-detection order (JSON sniff vs HTML sniff first).
- `#[clap(default_value = "auto")]` vs omit-flag-when-auto syntax for `--input-format`.
- TUI TestBackend canvas size + keystroke sequence (recommended: 80×24 + `j j j b1 q`).
- `# bytes=` trailing newline style (recommended: always `\n`).
- `reader::load_stdin` synthetic BufRead style (`io::Cursor<Vec<u8>>` vs custom impl).

## Deferred Ideas

- `cargo public-api` diff tooling — Phase 7.
- `search::Pattern` property / fuzz tests — Phase 5.
- Additional decode-input formats beyond 4 emitted formats — v3 theme.
- `--output-format` flag on `decode` — v3 features.
- Widening TUI coverage beyond exit-with-save — only if a bug demands it.
- Streaming byte-level scanner replacing line-based `find_digit_run` — Phase 6 PERF.
