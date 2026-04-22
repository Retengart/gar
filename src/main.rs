#![forbid(unsafe_op_in_unsafe_fn)]
// This is a binary crate: `pub(crate)` on items in private modules is
// semantically accurate (they are not part of any public API) and is
// exactly what `unreachable_pub` asks for. The nursery lint
// `redundant_pub_crate` disagrees; we silence it in favor of the
// correctness-oriented `unreachable_pub`.
#![allow(clippy::redundant_pub_crate)]

//! Entry point for the `base60` binary viewer.

mod cli;
mod color;
mod convert;
mod dump;
mod reader;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::ColorChoice;
use color::Palette;
use std::io::{BufWriter, IsTerminal, stdout};

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    let bytes = reader::load(args.file.as_deref(), args.skip, args.length)?;

    if args.interactive {
        tui::run(bytes.as_slice(), args.skip)?;
    } else {
        let stdout = stdout();
        let palette = pick_palette(args.color, stdout.is_terminal());
        // `dump_all` wraps in its own BufWriter; wrap again only to coalesce
        // writes before the lock is released.
        let result = dump::dump_all(
            bytes.as_slice(),
            args.skip,
            BufWriter::new(stdout.lock()),
            palette,
        );
        match result {
            Ok(()) => {}
            // Downstream consumers like `head` close the pipe after reading
            // their fill; treat that as a clean early exit rather than a
            // user-visible error, matching the behaviour of `cat`, `grep`,
            // `hexdump`, etc.
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Resolve the user's [`ColorChoice`] against the current environment.
///
/// `auto` (the default) honours the de-facto-standard `NO_COLOR` env var
/// (<https://no-color.org>) and falls back to monochrome when stdout is
/// redirected to a file or a pipe.
fn pick_palette(choice: ColorChoice, stdout_is_tty: bool) -> &'static Palette {
    let want_color = match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            stdout_is_tty && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        }
    };
    if want_color {
        &color::PALETTE_ANSI
    } else {
        &color::PALETTE_NONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ANSI palette emits a non-empty reset sequence; the mono palette
    /// does not. This is a stable content-level discriminator that avoids
    /// pointer-identity comparisons.
    fn is_ansi(p: &Palette) -> bool {
        !p.reset.is_empty()
    }

    #[test]
    fn auto_with_tty_and_no_env_is_ansi() {
        // SAFETY: Rust 2024 marks `env::remove_var` unsafe because parallel
        // threads may observe a half-updated environment. Cargo runs each
        // `#[test]` on its own thread but within the same process, so tests
        // touching env vars must not run concurrently. The risk here is
        // limited to this small set of env-sensitive tests; they only read
        // their own variable and clean up after themselves.
        unsafe { std::env::remove_var("NO_COLOR") };
        assert!(is_ansi(pick_palette(ColorChoice::Auto, true)));
    }

    #[test]
    fn auto_with_no_tty_is_mono() {
        // SAFETY: see `auto_with_tty_and_no_env_is_ansi`.
        unsafe { std::env::remove_var("NO_COLOR") };
        assert!(!is_ansi(pick_palette(ColorChoice::Auto, false)));
    }

    #[test]
    fn auto_with_no_color_env_is_mono() {
        // SAFETY: see `auto_with_tty_and_no_env_is_ansi`.
        unsafe { std::env::set_var("NO_COLOR", "1") };
        assert!(!is_ansi(pick_palette(ColorChoice::Auto, true)));
        // SAFETY: see `auto_with_tty_and_no_env_is_ansi`.
        unsafe { std::env::remove_var("NO_COLOR") };
    }

    #[test]
    fn always_forces_ansi_even_without_tty() {
        assert!(is_ansi(pick_palette(ColorChoice::Always, false)));
    }

    #[test]
    fn never_forces_mono_even_with_tty() {
        assert!(!is_ansi(pick_palette(ColorChoice::Never, true)));
    }
}
