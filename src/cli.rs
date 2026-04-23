//! Command-line interface definition.

use clap::{Parser, ValueEnum};
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

/// View binary data as base-60 (sexagesimal) digit pairs in the
/// Sumero-Babylonian positional notation.
#[derive(Parser, Debug)]
#[command(name = "base60", version, about, long_about = None)]
pub(crate) struct Cli {
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
}
