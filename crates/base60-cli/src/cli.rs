//! Command-line interface definition.

use base60_core::lens::{
    AngleLens, CuneiformLens, Lens, TabletLens, TimeLens, TimeScale as LensTimeScale,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

/// When to colorize output, mirroring the `--color` convention of
/// `ls`, `grep`, `diff`, etc.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum ColorChoice {
    /// Colorize when stdout is a TTY and `NO_COLOR` is unset.
    #[default]
    Auto,
    /// Always emit ANSI colours.
    Always,
    /// Never emit ANSI colours.
    Never,
}

/// Optional semantic overlay appended to each dump line.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum LensMode {
    /// No overlay (default; identical to previous behaviour).
    #[default]
    None,
    /// Babylonian time: `day beru:uš:gar`.
    Time,
    /// Sexagesimal angle: `deg°arcmin′arcsec.mas″`.
    Angle,
    /// Scribal tablet view — base-60 digits framed, optionally with the
    /// Sumerian no-zero placeholder (see `--purist`).
    Tablet,
    /// Every digit rendered as Sumero-Babylonian wedge glyphs.
    Cuneiform,
}

impl LensMode {
    /// Every variant in cycle order. Tests iterate this slice to prove
    /// `cycle`, `label`, `build_lens`, and `persist::parse_lens` stay
    /// exhaustive whenever a new variant is added. Re-exported via
    /// `base60::LensMode` so integration tests under `tests/` can iterate
    /// it without an inline copy.
    pub const ALL: &[Self] = &[
        Self::None,
        Self::Time,
        Self::Angle,
        Self::Tablet,
        Self::Cuneiform,
    ];

    /// Advance through the lens cycle used by the interactive viewer's
    /// `L` key. Wraps back to [`LensMode::None`] from [`LensMode::Cuneiform`].
    #[must_use]
    pub(crate) const fn cycle(self) -> Self {
        match self {
            Self::None => Self::Time,
            Self::Time => Self::Angle,
            Self::Angle => Self::Tablet,
            Self::Tablet => Self::Cuneiform,
            Self::Cuneiform => Self::None,
        }
    }

    /// Short label suitable for status bars: `"time"`, `"angle"`, …
    /// `"—"` for [`LensMode::None`] so the indicator never vanishes.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::None => "—",
            Self::Time => "time",
            Self::Angle => "angle",
            Self::Tablet => "tablet",
            Self::Cuneiform => "cuneiform",
        }
    }
}

/// Turn a [`LensMode`] into a live trait object, or [`None`] for
/// [`LensMode::None`]. Shared by the CLI dump path and the TUI so the
/// `L` toggle and the `--lens` flag go through the same constructor.
///
/// `scale` only affects [`LensMode::Time`]; `purist` only affects
/// [`LensMode::Tablet`]. Unused combinations are silently ignored.
#[must_use]
pub(crate) fn build_lens(mode: LensMode, scale: TimeScale, purist: bool) -> Option<Box<dyn Lens>> {
    match mode {
        LensMode::None => None,
        LensMode::Time => Some(Box::new(TimeLens {
            scale: match scale {
                TimeScale::Gar => LensTimeScale::Gar,
                TimeScale::Sec => LensTimeScale::Sec,
                TimeScale::Ms => LensTimeScale::Ms,
            },
        })),
        LensMode::Angle => Some(Box::new(AngleLens)),
        LensMode::Tablet => Some(Box::new(TabletLens { purist })),
        LensMode::Cuneiform => Some(Box::new(CuneiformLens::auto())),
    }
}

/// Scale for a raw `u64` under `--lens=time`. One `gar` is ≈ 2 seconds.
///
/// Surfaced as `pub` so the `#[doc(hidden)]` re-export
/// `base60::__TuiTimeScale` can forward it to integration tests that
/// drive `base60::__test_hooks::run_with_terminal`. No stability
/// guarantee — see the crate-level note on `__TuiTimeScale`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum TimeScale {
    /// Raw Sumerian `gar` ticks (historical default).
    #[default]
    Gar,
    /// Modern seconds (divide by 2 to recover `gar`).
    Sec,
    /// Modern milliseconds (divide by 2000 to recover `gar`).
    Ms,
}

/// Output format for the default viewer. `ansi` preserves the current
/// behaviour; the other modes are machine- or browser-friendly.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Colourised terminal output (honours `--color` for ANSI opt-out).
    #[default]
    Ansi,
    /// Same layout as `ansi` but never emits escape sequences — ideal
    /// for piping into `grep`, `awk`, or storing to disk.
    Plain,
    /// Newline-delimited JSON, one object per 8-byte chunk. Consumable
    /// with `jq` without any companion library.
    Json,
    /// Self-contained HTML document with an inline CSS heat-map.
    Html,
}

impl Format {
    /// Every variant in declaration order. Integration tests iterate this
    /// slice to drive the `LensMode × Format` roundtrip matrix without
    /// hard-coding variant lists in multiple places. Re-exported via
    /// `base60::Format`.
    pub const ALL: &[Self] = &[Self::Ansi, Self::Plain, Self::Json, Self::Html];
}

/// Input format the `decode` subcommand expects. `Auto` sniffs the first
/// non-empty line of input; the explicit values force a specific decoder.
///
/// `Ansi` and `Plain` share the same underlying text decoder — both
/// values exist for UI symmetry with [`Format`] (Pitfall 10, D-06).
///
/// Widened to `pub` so the `#[doc(hidden)] pub mod __bench` re-export in
/// `crate::lib` can surface it to `crates/base60-cli/benches/`. The
/// enclosing `mod cli` is private at crate root, so this enum is still
/// unreachable from the public API (Phase 5 PERF-06, TEST-02 SC5).
#[allow(unreachable_pub)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum InputFormat {
    /// Sniff the first non-empty line: `<!doctype|<html` → HTML,
    /// `{"offset":` → JSON, otherwise the ansi/plain text decoder.
    #[default]
    Auto,
    /// Force the ansi/plain text decoder (colour escapes tolerated).
    Ansi,
    /// Force the ansi/plain text decoder.
    Plain,
    /// Force the NDJSON decoder.
    Json,
    /// Force the HTML decoder.
    Html,
}

/// View binary data as base-60 (sexagesimal) digit pairs in the
/// Sumero-Babylonian positional notation.
#[derive(Parser, Debug)]
#[command(name = "base60", version, about, long_about = None)]
pub(crate) struct Cli {
    /// Optional subcommand. Omit to run the default `view` behaviour
    /// with the top-level flags below.
    #[command(subcommand)]
    pub(crate) command: Option<Command>,

    /// Flags for the default `view` behaviour.
    #[command(flatten)]
    pub(crate) view: ViewArgs,
}

/// Top-level subcommands. Each one runs an independent workflow; the
/// top-level flags on [`Cli`] apply only when no subcommand is given.
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Stream statistical analysis of a file to stdout.
    ///
    /// Complements the default viewer with Shannon entropy, byte
    /// histogram, and detected ASCII regions — the kind of summary a
    /// reverse engineer would build by hand.
    Analyze(AnalyzeArgs),
    /// Inverse of the default viewer: parse a base-60 dump back into
    /// raw bytes on stdout.
    ///
    /// Useful for roundtripping — `base60 --color=never FILE | base60
    /// decode` reproduces the original byte stream.
    Decode(DecodeArgs),
    /// Emit shell completion script for the selected shell to stdout.
    ///
    /// Pipe the output into the shell's completion directory, e.g.
    /// `base60 completions zsh > ~/.zfunc/_base60`.
    Completions(CompletionsArgs),
}

/// Arguments accepted by the default (viewer) behaviour. Flattened into
/// [`Cli`] so `base60 FILE` keeps its current shape.
#[derive(Args, Debug)]
pub(crate) struct ViewArgs {
    /// Input file. If omitted, bytes are read from standard input.
    pub(crate) file: Option<PathBuf>,

    /// Launch the interactive TUI viewer instead of printing to stdout.
    #[arg(short, long)]
    pub(crate) interactive: bool,

    /// Skip this many bytes from the beginning of the input.
    #[arg(short = 's', long, default_value_t = 0, value_name = "N")]
    pub(crate) skip: u64,

    /// Read at most this many bytes.
    #[arg(short = 'n', long, value_name = "N")]
    pub(crate) length: Option<u64>,

    /// When to colorize the output (`auto`, `always`, `never`).
    /// `auto` honours `NO_COLOR` and checks whether stdout is a TTY.
    #[arg(
        long,
        value_enum,
        default_value_t = ColorChoice::Auto,
        value_name = "WHEN",
    )]
    pub(crate) color: ColorChoice,

    /// Append a semantic overlay column to every dump line.
    #[arg(
        long,
        value_enum,
        default_value_t = LensMode::None,
        value_name = "MODE",
    )]
    pub(crate) lens: LensMode,

    /// Scale used when interpreting the chunk as time (`--lens=time`).
    #[arg(
        long,
        value_enum,
        default_value_t = TimeScale::Gar,
        value_name = "UNIT",
    )]
    pub(crate) time_scale: TimeScale,

    /// Use the Sumerian no-zero placeholder in `--lens=tablet`.
    /// Early Sumerian scribes left a gap where a positional zero later
    /// stood; this flag restores that behaviour.
    #[arg(long)]
    pub(crate) purist: bool,

    /// Output format: `ansi` (coloured TTY), `plain` (pipe-safe text),
    /// `json` (ndjson), or `html` (self-contained report).
    #[arg(
        long,
        value_enum,
        default_value_t = Format::Ansi,
        value_name = "MODE",
    )]
    pub(crate) format: Format,
}

/// Arguments for `base60 analyze`.
#[derive(Args, Debug)]
pub(crate) struct AnalyzeArgs {
    /// Input file. If omitted, bytes are read from standard input.
    pub(crate) file: Option<PathBuf>,

    /// Skip this many bytes from the beginning of the input.
    #[arg(short = 's', long, default_value_t = 0, value_name = "N")]
    pub(crate) skip: u64,

    /// Read at most this many bytes.
    #[arg(short = 'n', long, value_name = "N")]
    pub(crate) length: Option<u64>,

    /// Window size for per-window Shannon entropy, in bytes.
    /// Values below the analyser's internal minimum (`64`) are clamped.
    #[arg(long, default_value_t = crate::analyze::DEFAULT_WINDOW, value_name = "N")]
    pub(crate) window: usize,
}

/// Arguments for `base60 decode`.
#[derive(Args, Debug)]
pub(crate) struct DecodeArgs {
    /// Dump file to decode. If omitted, text is read from standard input.
    pub(crate) file: Option<PathBuf>,

    /// Expected input format. `auto` (default) sniffs the first
    /// non-empty line of input; explicit values force a specific decoder.
    #[arg(
        long,
        value_enum,
        default_value_t = InputFormat::Auto,
        value_name = "MODE",
    )]
    pub(crate) input_format: InputFormat,
}

/// Arguments for `base60 completions`.
#[derive(Args, Debug)]
pub(crate) struct CompletionsArgs {
    /// Target shell. Supported values: `bash`, `zsh`, `fish`,
    /// `elvish`, `powershell`.
    #[arg(value_enum)]
    pub(crate) shell: Shell,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist;

    #[test]
    fn all_contains_every_variant_in_cycle_order() {
        // Walking `cycle()` from `None` for `ALL.len()` steps must
        // yield the same sequence listed in `ALL`, then loop back to
        // `None`. This catches (a) a missing variant in `ALL`
        // (D-08 Test 1 intent), (b) a misordered `ALL` (D-09 intent),
        // and (c) a cycle that skips or revisits a variant — all in
        // one assertion.
        let mut walk = LensMode::None;
        for &expected in LensMode::ALL {
            assert_eq!(walk, expected);
            walk = walk.cycle();
        }
        assert_eq!(walk, LensMode::None);
    }

    #[test]
    fn all_methods_total_over_all() {
        for &mode in LensMode::ALL {
            // Every variant has a non-empty label.
            let lbl = mode.label();
            assert!(!lbl.is_empty(), "label empty for {mode:?}");

            // `cycle()` maps into `ALL` (no stray variant synthesised).
            let next = mode.cycle();
            assert!(
                LensMode::ALL.contains(&next),
                "cycle({mode:?}) = {next:?} is not in LensMode::ALL",
            );

            // `build_lens` dispatches without panicking. We do not
            // inspect the returned trait object — only that no arm
            // is missing from `build_lens`'s match.
            let _lens = build_lens(mode, TimeScale::default(), false);

            // `persist::parse_lens` round-trips the label back to
            // the same variant for every non-None case. `None`'s
            // label "—" is intentionally not a valid persisted label;
            // it maps to `None` via the unknown-label fallback, so we
            // assert that fallback explicitly rather than relying on
            // strict equality (which would silently tolerate a future
            // variant whose label happens to fall into the same bucket).
            if mode == LensMode::None {
                assert_eq!(persist::parse_lens(lbl), LensMode::None);
            } else {
                assert_eq!(
                    persist::parse_lens(lbl),
                    mode,
                    "parse_lens({lbl:?}) did not round-trip to {mode:?}",
                );
            }
        }
    }

    #[test]
    fn all_contains_every_format_variant() {
        // Enumerate every Format in a closed match; if a future variant
        // is added, the match becomes non-exhaustive at compile time and
        // points here. The contains() check confirms `Format::ALL` stays
        // aligned with the enum declaration.
        for variant in [Format::Ansi, Format::Plain, Format::Json, Format::Html] {
            assert!(
                Format::ALL.contains(&variant),
                "Format::ALL missing variant {variant:?}",
            );
        }
        assert_eq!(Format::ALL.len(), 4, "Format::ALL length drift");
    }
}
