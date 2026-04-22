use clap::Parser;
use std::path::PathBuf;

/// Просмотрщик бинарных данных в 60-ричной системе счисления
/// (шумеро-вавилонская позиционная запись: пары десятичных 00..59, разделитель ':').
#[derive(Parser, Debug)]
#[command(
    name = "base60",
    about = "View binary data as base-60 (sexagesimal) digit pairs",
    version
)]
pub struct Cli {
    /// Path to the file to view. If omitted, read from stdin.
    pub file: Option<PathBuf>,

    /// Launch the interactive TUI viewer instead of printing to stdout.
    #[arg(short, long)]
    pub interactive: bool,

    /// Skip this many bytes from the beginning of the input.
    #[arg(short = 's', long, default_value_t = 0)]
    pub skip: u64,

    /// Read at most this many bytes.
    #[arg(short = 'n', long)]
    pub length: Option<u64>,
}
