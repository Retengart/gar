//! Command-line interface definition.

use clap::Parser;
use std::path::PathBuf;

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
}
