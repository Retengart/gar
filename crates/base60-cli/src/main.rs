#![forbid(unsafe_op_in_unsafe_fn)]
// This is a binary crate: `pub(crate)` on items in private modules is
// semantically accurate (they are not part of any public API) and is
// exactly what `unreachable_pub` asks for. The nursery lint
// `redundant_pub_crate` disagrees; we silence it in favor of the
// correctness-oriented `unreachable_pub`.
#![allow(clippy::redundant_pub_crate)]

//! Entry point for the `base60` binary viewer.

mod analyze;
mod chunk;
mod cli;
mod color;
mod decode;
mod dump;
mod format;
mod persist;
mod reader;
mod search;
mod tui;

use anyhow::Result;
use base60_core::Lens;
use clap::CommandFactory;
use clap::Parser;
use cli::{AnalyzeArgs, ColorChoice, Command, CompletionsArgs, DecodeArgs, Format, ViewArgs};
use color::Palette;
use std::fs::File;
use std::io::{BufReader, BufWriter, IsTerminal, stdout};

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    match &args.command {
        None => run_view(&args.view),
        Some(Command::Analyze(a)) => run_analyze(a),
        Some(Command::Decode(d)) => run_decode(d),
        Some(Command::Completions(c)) => {
            run_completions(c);
            Ok(())
        }
    }
}

fn run_view(view: &ViewArgs) -> Result<()> {
    let bytes = reader::load(view.file.as_deref(), view.skip, view.length)?;

    if view.interactive {
        // The TUI owns the lens so the `L` key can cycle variants without
        // the main process having to rebuild state between frames. The
        // file path (if any) is plumbed through so the viewer can
        // persist per-file cursor/scroll/bookmarks across runs.
        tui::run(
            bytes.as_slice(),
            view.skip,
            view.lens,
            view.time_scale,
            view.purist,
            view.file.as_deref(),
        )?;
    } else {
        let lens = cli::build_lens(view.lens, view.time_scale, view.purist);
        let lens_ref: Option<&dyn Lens> = lens.as_deref();
        let stdout = stdout();
        let is_tty = stdout.is_terminal();
        let result = match view.format {
            // `dump_all` wraps its own BufWriter; we wrap again only to
            // coalesce writes before the lock is released.
            Format::Ansi => dump::dump_all(
                bytes.as_slice(),
                view.skip,
                BufWriter::new(stdout.lock()),
                pick_palette(view.color, is_tty),
                lens_ref,
            ),
            // Plain == ANSI layout with the no-op palette, regardless of
            // what `--color` says. Users who ask for plain mean it.
            Format::Plain => dump::dump_all(
                bytes.as_slice(),
                view.skip,
                BufWriter::new(stdout.lock()),
                &color::PALETTE_NONE,
                lens_ref,
            ),
            Format::Json => format::emit_json(
                bytes.as_slice(),
                view.skip,
                BufWriter::new(stdout.lock()),
                lens_ref,
            ),
            Format::Html => format::emit_html(
                bytes.as_slice(),
                view.skip,
                BufWriter::new(stdout.lock()),
                lens_ref,
            ),
        };
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

fn run_analyze(args: &AnalyzeArgs) -> Result<()> {
    let bytes = reader::load(args.file.as_deref(), args.skip, args.length)?;
    let analysis = analyze::analyze(bytes.as_slice(), args.window);
    let stdout = stdout();
    let mut out = BufWriter::new(stdout.lock());
    match analyze::write_summary(&analysis, bytes.as_slice(), &mut out) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn run_decode(args: &DecodeArgs) -> Result<()> {
    // Decode reads text line-by-line; it doesn't benefit from mmap
    // (dump files are tiny compared to their source), so we lean on
    // `BufRead` straight from file or stdin.
    let stdout = stdout();
    let mut out = BufWriter::new(stdout.lock());
    let result = if let Some(path) = args.file.as_deref() {
        let file = File::open(path)?;
        decode::decode_stream(BufReader::new(file), &mut out)
    } else {
        let stdin = std::io::stdin();
        decode::decode_stream(stdin.lock(), &mut out)
    };
    match result {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn run_completions(args: &CompletionsArgs) {
    // `CommandFactory` rebuilds the full `clap::Command` tree for free
    // so completions always match whatever flags the binary actually
    // exposes — no drift between the CLI definition and the script.
    let mut cmd = cli::Cli::command();
    let bin_name = cmd.get_name().to_owned();
    let stdout = stdout();
    clap_complete::generate(args.shell, &mut cmd, bin_name, &mut stdout.lock());
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
