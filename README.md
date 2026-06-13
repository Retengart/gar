# gar — Sumero-Babylonian binary viewer

`gar` is a hex-dump alternative that renders every 8 bytes of input as
eleven sexagesimal (base-60) digit pairs — the positional notation the
Sumerians and Babylonians used for four millennia before they gave us
the 60-second minute, 360-degree circle, and 24-hour day.

```text
00000000  15:10:00:41:41:42:35:28:21:24:16  |.ELF....|
00000008  00:00:00:00:00:00:00:00:00:00:00  |........|
00000010  00:21:27:26:34:03:50:30:56:25:36  |..>.....|
```

It comes with an interactive TUI, statistical analysis, roundtrip
decoding, four optional overlay lenses (including actual cuneiform), and
JSON/HTML output for pipeline and report use.

## Install

```sh
cargo install --path crates/gar-cli
```

`gar` will live at `$HOME/.cargo/bin/gar`. Workspace layout:

* `crates/gar-core` — reusable library (`u64_to_base60`, lenses,
  cuneiform glyphs, URL-safe encoding). `pub` API; depend on it with
  `path = "../gar-core"` until it's published to crates.io.
* `crates/gar-cli`  — the `gar` binary that wraps it.

## Quick start

```sh
gar /bin/ls | head               # coloured dump to TTY
gar --format=plain file.bin      # pipeline-friendly (no ANSI)
gar --lens=time file.bin         # annotate every row with
                                    #   Babylonian day beru:uš:gar
gar --lens=cuneiform /bin/ls     # render digits as 𒁹𒌋 wedges
gar -i file.bin                  # launch the interactive TUI
gar analyze /bin/ls              # entropy + ASCII-strings summary
gar --format=plain file.bin | gar decode > file.bin.rt
                                    # dump → bytes roundtrip
```

## Features

### Lenses — `--lens=<MODE>`

| Mode       | What it adds                                                |
|------------|-------------------------------------------------------------|
| `none`     | default; no extra column                                     |
| `time`     | Sumerian day · beru · uš · gar (1 gar ≈ 2 s)                 |
| `angle`    | sexagesimal angle (deg°arcmin′arcsec.mas″)                   |
| `tablet`   | scribal framing; `--purist` uses the Sumerian no-zero gap    |
| `cuneiform`| digits rendered as wedge glyphs (`𒁹` = 1, `𒌋` = 10, `𒑰` = 0)|

`--time-scale={gar,sec,ms}` rescales the time lens when the u64 is
already in modern units. `NO_UNICODE=1` or `TERM=dumb` forces the
cuneiform lens to fall back to decimal pairs.

### Output formats — `--format=<MODE>`

| Mode    | Use case                                                       |
|---------|----------------------------------------------------------------|
| `ansi`  | default; coloured terminal output (respects `--color`)         |
| `plain` | layout identical to ansi, no escape codes — pipe-friendly      |
| `json`  | newline-delimited JSON, one object per 8-byte chunk            |
| `html`  | self-contained HTML with inline CSS heatmap                    |

JSON schema:

```json
{"offset":0,"bytes":[127,69,76,70,2,1,1,0],
 "digits":[15,10,0,41,41,42,35,28,21,24,16],
 "ascii":".ELF....","lens":"0d 00𒁹 00:00"}
```

`"lens"` is present only when a lens is active. Parses directly with
`jq`, with no companion library.

### Interactive TUI — `-i`

Launches a ratatui-based viewer. All keybinds:

| Key                      | Action                                       |
|--------------------------|----------------------------------------------|
| `h` `j` `k` `l`          | move cursor ±1 byte / ±1 row (8 bytes)       |
| `0` / `^` / `$`          | start of row / start of row / end of row     |
| `g` / `G` or Home/End    | first / last byte                            |
| `Ctrl-d` / `Ctrl-u`      | half-page forward / backward                 |
| PgDn / PgUp              | full page forward / backward                 |
| `L`                      | cycle lens (none → time → angle → tablet → cuneiform → none) |
| `/`                      | search (`hex:DEADBEEF`, `str:foo`, `"foo"`, or auto-detect)  |
| `n` / `N`                | next / previous search match                 |
| `m<letter>` / `'<letter>`| set / jump bookmark (26 slots, `a-z`)        |
| `]p` / `]z` / `]e`       | next printable run / zero-run / entropy spike|
| `[p` / `[z` / `[e`       | previous of the same                         |
| `q` / Esc                | quit (saves state to `$XDG_STATE_HOME/gar/`)|

State (cursor, scroll, active lens, bookmarks) is persisted per-file
across runs. Reopening the same file resumes where you left off.

### Statistical analysis — `gar analyze FILE`

Streams a Shannon-entropy + byte-histogram + region-detection summary:

```
bytes         199336
entropy       6.053 bits/byte
window        256
windows       778
window range  [0.000, 6.026]  mean 4.546
unique bytes  256 / 256
top bytes
  0x00              49178   24.67%
  0xff              11306    5.67%
  0x48 'H'           7923    3.97%
  ...
regions       ascii=1782  high-entropy=0  low-entropy=53
ascii preview
  0x000003c4..0x000003df  "/lib64/ld-linux-x86-64.so.2"
  0x000010e1..0x000010fc  "_ITM_deregisterTMCloneTable"
  ...
```

`--window N` tunes the Shannon window size (default 256, clamped to ≥64).
Detected region kinds: `ascii` (≥4 printable), `high-entropy` (>7.5
bits/byte, likely compressed/encrypted), `low-entropy` (<1.0, likely
padding).

### Decoding — `gar decode`

Reverses the default view output. The parser scans for runs of eleven
two-digit base-60 pairs joined by colons and ignores surrounding
content (offset column, ASCII column, ANSI escapes). Roundtrip:

```sh
gar --format=plain file | gar decode > file.roundtrip
cmp file file.roundtrip  # silent = identical
```

### Shell completions — `gar completions <SHELL>`

Supported: `bash`, `zsh`, `fish`, `elvish`, `powershell`. Install
pattern:

```sh
mkdir -p ~/.zfunc && gar completions zsh > ~/.zfunc/_gar
```

## Library usage

`gar-core` exposes the pure-Rust building blocks:

```rust
use gar_core::{DIGITS, u64_to_base60, encode_u64, decode_u64};

let digits = u64_to_base60(5025);
assert_eq!(digits[8..], [1, 23, 45]);

// URL-safe hash-prefix encoding: u64 → 11-char string using 0-9A-Za-x.
let prefix = 0xDEAD_BEEF_u64;
let short = encode_u64(prefix);
assert_eq!(decode_u64(&short).unwrap(), prefix);
```

Also exported: the `Lens` trait + four implementations (`TimeLens`,
`AngleLens`, `TabletLens`, `CuneiformLens`), and the cuneiform glyph
table.

## Design

* **8-byte chunks**: a u64 fits in exactly 11 base-60 digits, since
  `60¹¹ ≈ 3.65·10¹⁹ > u64::MAX`.
* **Heatmap palette** — digits coloured by magnitude:
  `0` dark gray · `1..20` green · `20..40` yellow · `40..60` red. The
  NONE palette is literally empty strings so no-color output costs
  nothing per token.
* **mmap** for file input via `memmap2`; stdin streams into a `Vec`.
* **Persistence key** for the TUI state store is FNV-1a of the
  canonicalised path — stable, non-crypto, zero deps.

## License

MIT OR Apache-2.0
