mod cli;
mod convert;
mod dump;
mod reader;
mod tui;

use anyhow::Result;
use clap::Parser;
use std::io::stdout;

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    let bytes = reader::load(args.file.as_deref(), args.skip, args.length)?;

    if args.interactive {
        tui::run(bytes.as_slice(), args.skip)?;
    } else {
        let mut out = stdout().lock();
        dump::dump_all(bytes.as_slice(), args.skip, &mut out)?;
    }
    Ok(())
}
