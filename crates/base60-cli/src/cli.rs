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
pub(crate) enum LensMode {
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
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum TimeScale {
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
pub(crate) enum Format {
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
}

/// Arguments for `base60 completions`.
#[derive(Args, Debug)]
pub(crate) struct CompletionsArgs {
    /// Target shell. Supported values: `bash`, `zsh`, `fish`,
    /// `elvish`, `powershell`.
    #[arg(value_enum)]
    pub(crate) shell: Shell,
}
