#![forbid(unsafe_op_in_unsafe_fn)]
// This is a binary crate: `pub(crate)` on items in private modules is
// semantically accurate (they are not part of any public API) and is
// exactly what `unreachable_pub` asks for. The nursery lint
// `redundant_pub_crate` disagrees; we silence it in favor of the
// correctness-oriented `unreachable_pub`.
#![allow(clippy::redundant_pub_crate)]

//! Entry point for the `base60` binary viewer.

mod cli;
mod convert;
mod dump;
mod reader;
mod tui;

use anyhow::Result;
use clap::Parser;
use std::io::{BufWriter, stdout};

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    let bytes = reader::load(args.file.as_deref(), args.skip, args.length)?;

    if args.interactive {
        tui::run(bytes.as_slice(), args.skip)?;
    } else {
        let stdout = stdout().lock();
        // `dump_all` wraps in its own BufWriter; wrap again only to coalesce
        // writes before the lock is released.
        dump::dump_all(bytes.as_slice(), args.skip, BufWriter::new(stdout))?;
    }
    Ok(())
}
