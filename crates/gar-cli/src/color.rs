//! Color palette shared between the ANSI streaming renderer and the TUI.
//!
//! Both paths pick from the same four-tier heatmap so a digit rendered in
//! the CLI and in the TUI has the same perceptual weight:
//!
//! | Digit value | Colour      | Meaning                     |
//! |-------------|-------------|-----------------------------|
//! | `0`         | dark grey   | zero byte — visual noise    |
//! | `1..20`     | green       | low                         |
//! | `20..40`    | yellow      | medium                      |
//! | `40..60`    | red         | high (top of the base-60)   |

use ratatui::style::{Color, Modifier, Style};

/// ANSI escape fragments for each token kind used by the CLI path.
///
/// [`PALETTE_NONE`] is an all-empty palette: no escapes are emitted, which
/// is both the correct behaviour for non-TTY output and a way to remove
/// all branching from the hot path — the writes become zero-byte no-ops.
///
/// Widened to `pub` so the `#[doc(hidden)] pub mod __bench` re-export in
/// `crate::lib` can surface [`PALETTE_NONE`] and [`dump_all`] (which takes
/// `&Palette`) to `crates/gar-cli/benches/`. The enclosing `mod color`
/// is private at crate root, so this struct is still unreachable from the
/// public API (Phase 5 PERF-06, TEST-02 SC5).
#[allow(unreachable_pub, reason = "pub for __bench re-export; unreachable from public API in private mod")]
#[derive(Debug)]
pub struct Palette {
    pub(crate) offset: &'static str,
    pub(crate) sep: &'static str,
    pub(crate) delim: &'static str,
    pub(crate) printable: &'static str,
    pub(crate) dot: &'static str,
    pub(crate) lens: &'static str,
    pub(crate) reset: &'static str,
    zero: &'static str,
    low: &'static str,
    mid: &'static str,
    high: &'static str,
}

/// No-colour palette: every field is `""`, so emitting it is a no-op.
/// Exposed as a `static` (not `const`) so it has a single stable address
/// that callers can compare via [`std::ptr::eq`].
///
/// Widened to `pub` so the `#[doc(hidden)] pub mod __bench` re-export in
/// `crate::lib` can surface it to `crates/gar-cli/benches/`. The
/// enclosing `mod color` is private at crate root, so this static is
/// still unreachable from the public API (Phase 5 PERF-06, TEST-02 SC5).
#[allow(unreachable_pub, reason = "pub for __bench re-export; unreachable from public API in private mod")]
pub static PALETTE_NONE: Palette = Palette {
    offset: "",
    sep: "",
    delim: "",
    printable: "",
    dot: "",
    lens: "",
    reset: "",
    zero: "",
    low: "",
    mid: "",
    high: "",
};

/// Standard 8/16-colour ANSI palette. Works everywhere that speaks ANSI
/// (xterm, tmux, modern Windows, VS Code terminal, etc.).
pub(crate) static PALETTE_ANSI: Palette = Palette {
    offset: "\x1b[90m",
    sep: "\x1b[90m",
    delim: "\x1b[2;36m",
    printable: "\x1b[36m",
    dot: "\x1b[90m",
    // Magenta distinguishes the lens overlay from the cyan ASCII column,
    // so a user can tell at a glance which bytes are raw vs. interpreted.
    lens: "\x1b[35m",
    reset: "\x1b[0m",
    zero: "\x1b[90m",
    low: "\x1b[32m",
    mid: "\x1b[33m",
    high: "\x1b[31m",
};

impl Palette {
    /// Escape code for the heat-map tier of a single base-60 digit.
    #[inline]
    pub(crate) const fn digit(&self, d: u8) -> &'static str {
        match d {
            0 => self.zero,
            1..20 => self.low,
            20..40 => self.mid,
            _ => self.high,
        }
    }
}

/// ratatui [`Style`] for a single base-60 digit, following the same tiers
/// as [`Palette::digit`].
#[inline]
pub(crate) const fn digit_style(d: u8) -> Style {
    let color = match d {
        0 => Color::DarkGray,
        1..20 => Color::Green,
        20..40 => Color::Yellow,
        _ => Color::Red,
    };
    Style::new().fg(color)
}

pub(crate) const fn offset_style() -> Style {
    Style::new().fg(Color::DarkGray)
}

pub(crate) const fn sep_style() -> Style {
    Style::new().fg(Color::DarkGray)
}

pub(crate) const fn delim_style() -> Style {
    Style::new().fg(Color::Cyan).add_modifier(Modifier::DIM)
}

pub(crate) const fn printable_style() -> Style {
    Style::new().fg(Color::Cyan)
}

pub(crate) const fn dot_style() -> Style {
    Style::new().fg(Color::DarkGray)
}

pub(crate) const fn lens_style() -> Style {
    Style::new().fg(Color::Magenta)
}

pub(crate) const fn title_style() -> Style {
    Style::new()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub(crate) const fn border_style() -> Style {
    Style::new().fg(Color::Cyan)
}

pub(crate) const fn status_style() -> Style {
    Style::new()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heatmap_tiers() {
        assert_eq!(PALETTE_ANSI.digit(0), "\x1b[90m");
        assert_eq!(PALETTE_ANSI.digit(1), "\x1b[32m");
        assert_eq!(PALETTE_ANSI.digit(19), "\x1b[32m");
        assert_eq!(PALETTE_ANSI.digit(20), "\x1b[33m");
        assert_eq!(PALETTE_ANSI.digit(39), "\x1b[33m");
        assert_eq!(PALETTE_ANSI.digit(40), "\x1b[31m");
        assert_eq!(PALETTE_ANSI.digit(59), "\x1b[31m");
    }

    #[test]
    fn none_palette_is_empty() {
        for d in 0..60 {
            assert_eq!(PALETTE_NONE.digit(d), "");
        }
        assert_eq!(PALETTE_NONE.offset, "");
        assert_eq!(PALETTE_NONE.reset, "");
        assert_eq!(PALETTE_NONE.lens, "");
    }

    #[test]
    fn ansi_palette_has_distinct_lens_colour() {
        // Magenta for lens, cyan for the ASCII column — the two should
        // never collide or a user loses the visual distinction.
        assert_ne!(PALETTE_ANSI.lens, PALETTE_ANSI.printable);
        assert!(PALETTE_ANSI.lens.starts_with("\x1b["));
    }
}
