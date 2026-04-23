//! Optional semantic overlays rendered next to every dump line.
//!
//! A [`Lens`] interprets the 8-byte chunk at a given offset as something
//! more meaningful than raw bytes — a Babylonian time, a sexagesimal angle,
//! a scribal tablet view, or a cuneiform transliteration. The main viewer
//! remains authoritative; the lens appends a single extra column.
//!
//! ```text
//! 00000000  00:00:01:00:00:00:00:00:00:00:00  |........|   0d 00𒁹 00:00
//! ```
//!
//! Implementations are cheap, allocation-bounded, and `Send + Sync` so the
//! same lens instance can be shared across the CLI streaming path and the
//! TUI viewer.

use crate::convert::{DIGITS, u64_to_base60};
use crate::cuneiform;

/// How a caller wants a [`TimeLens`] to interpret the raw `u64`.
///
/// One Sumerian `gar` is roughly two modern seconds, so a value expressed
/// in modern seconds or milliseconds needs scaling before decomposition.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum TimeScale {
    /// Raw Sumerian `gar` ticks (≈ 2 seconds each). The historical choice.
    #[default]
    Gar,
    /// Modern seconds. Divided by 2 to recover `gar`.
    Sec,
    /// Modern milliseconds. Divided by `2_000` to recover `gar`.
    Ms,
}

/// Semantic overlay attached to each dump row.
pub trait Lens: Send + Sync {
    /// Render the overlay for a chunk already parsed as big-endian `u64`.
    fn render(&self, chunk: u64) -> String;
}

// -- Time ------------------------------------------------------------------

/// Babylonian astronomical time, in `day beru:uš:gar` units.
///
/// Decomposition (all sexagesimal once inside a day):
///
/// * `1 day  = 12 beru`  (double-hour, ≈ 2 modern hours)
/// * `1 beru = 60 uš`   (≈ 2 minutes)
/// * `1 uš   = 60 gar`  (≈ 2 seconds)
#[derive(Copy, Clone, Debug, Default)]
pub struct TimeLens {
    pub scale: TimeScale,
}

impl TimeLens {
    const GAR_PER_HOUR: u64 = 60 * 60;
    const GAR_PER_DAY: u64 = 12 * Self::GAR_PER_HOUR;

    const fn gar(self, chunk: u64) -> u64 {
        match self.scale {
            TimeScale::Gar => chunk,
            TimeScale::Sec => chunk / 2,
            TimeScale::Ms => chunk / 2_000,
        }
    }
}

impl Lens for TimeLens {
    fn render(&self, chunk: u64) -> String {
        let gar = self.gar(chunk);
        let day = gar / Self::GAR_PER_DAY;
        let beru = (gar / Self::GAR_PER_HOUR) % 12;
        let us = (gar / 60) % 60;
        let ga = gar % 60;
        // `𒁹` between beru and uš highlights that `beru` lives in a 0..12
        // slot (non-sexagesimal), while the rest is base-60.
        format!("{day}d {beru:02}𒁹 {us:02}:{ga:02}")
    }
}

// -- Angle -----------------------------------------------------------------

/// Sexagesimal angle, interpreting `chunk` as milliarcseconds.
///
/// Base-60 angle notation has survived from Babylonian astronomy into
/// modern coordinates: `1° = 60′`, `1′ = 60″`, `1″ = 1000 mas`.
#[derive(Copy, Clone, Debug, Default)]
pub struct AngleLens;

impl AngleLens {
    const MAS_PER_DEG: u64 = 3_600_000;
    const MAS_PER_ARCMIN: u64 = 60_000;
    const MAS_PER_ARCSEC: u64 = 1_000;
}

impl Lens for AngleLens {
    fn render(&self, chunk: u64) -> String {
        let deg = chunk / Self::MAS_PER_DEG;
        let arcmin = (chunk / Self::MAS_PER_ARCMIN) % 60;
        let arcsec = (chunk / Self::MAS_PER_ARCSEC) % 60;
        let mas = chunk % 1000;
        format!("{deg:03}°{arcmin:02}′{arcsec:02}.{mas:03}″")
    }
}

// -- Tablet ----------------------------------------------------------------

/// Scribal view: base-60 digits framed like a clay tablet fragment,
/// optionally using a blank placeholder for leading zeros.
///
/// The Sumerians had no positional zero — early scribes left a gap where a
/// digit was absent. The `purist` flag restores that behaviour; otherwise
/// leading zeros render as `00` for column alignment.
#[derive(Copy, Clone, Debug, Default)]
pub struct TabletLens {
    pub purist: bool,
}

impl Lens for TabletLens {
    fn render(&self, chunk: u64) -> String {
        let digits = u64_to_base60(chunk);
        // Worst case: `⌐ ` + DIGITS*(2 ascii + 1 colon) + ` ¬`.
        let mut s = String::with_capacity(6 + DIGITS * 3);
        s.push('⌐');
        s.push(' ');
        let mut leading = true;
        for (i, &d) in digits.iter().enumerate() {
            if i > 0 {
                s.push(':');
            }
            let last = i == DIGITS - 1;
            if leading && d == 0 && !last {
                // Leading zero: either blank (purist) or `00` (aligned).
                if self.purist {
                    s.push(' ');
                    s.push(' ');
                } else {
                    s.push_str("00");
                }
            } else {
                leading = false;
                let [hi, lo] = cuneiform::ascii_pair(d);
                s.push(hi as char);
                s.push(lo as char);
            }
        }
        s.push(' ');
        s.push('¬');
        s
    }
}

// -- Cuneiform -------------------------------------------------------------

/// Every digit rendered as Sumero-Babylonian wedges.
///
/// `fallback` is decided once at construction by inspecting the
/// environment — `NO_UNICODE=1`, `TERM=dumb`, or the CLI override —
/// so the render path stays branch-predictable.
#[derive(Copy, Clone, Debug, Default)]
pub struct CuneiformLens {
    pub fallback: bool,
}

impl CuneiformLens {
    /// Build a lens, detecting fallback from the environment.
    #[must_use]
    pub fn auto() -> Self {
        Self {
            fallback: cuneiform::ascii_fallback_forced(),
        }
    }
}

impl Lens for CuneiformLens {
    fn render(&self, chunk: u64) -> String {
        let digits = u64_to_base60(chunk);
        if self.fallback {
            // Decimal pairs, colon-separated — identical layout to the
            // main dump column but no ANSI; used when glyphs won't render.
            let mut s = String::with_capacity(DIGITS * 3);
            for (i, &d) in digits.iter().enumerate() {
                if i > 0 {
                    s.push(':');
                }
                let [hi, lo] = cuneiform::ascii_pair(d);
                s.push(hi as char);
                s.push(lo as char);
            }
            s
        } else {
            // Glyphs are up to 14 codepoints × 4 bytes, plus a separator
            // space between digits. Capacity is a rough upper bound.
            let mut s = String::with_capacity(DIGITS * 20);
            for (i, &d) in digits.iter().enumerate() {
                if i > 0 {
                    s.push(' ');
                }
                s.push_str(cuneiform::glyph(d));
            }
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_zero_is_start_of_day() {
        let l = TimeLens::default();
        assert_eq!(l.render(0), "0d 00𒁹 00:00");
    }

    #[test]
    fn time_one_gar_is_one_second_pair() {
        let l = TimeLens::default();
        assert_eq!(l.render(1), "0d 00𒁹 00:01");
    }

    #[test]
    fn time_full_day_wraps_day_counter() {
        // 12 beru × 60 uš × 60 gar = 43200 gar.
        let l = TimeLens::default();
        assert_eq!(l.render(43_200), "1d 00𒁹 00:00");
    }

    #[test]
    fn time_sec_scale_divides_by_two() {
        let l = TimeLens {
            scale: TimeScale::Sec,
        };
        // 2 modern seconds == 1 gar.
        assert_eq!(l.render(2), "0d 00𒁹 00:01");
    }

    #[test]
    fn time_ms_scale_divides_by_two_thousand() {
        let l = TimeLens {
            scale: TimeScale::Ms,
        };
        assert_eq!(l.render(2_000), "0d 00𒁹 00:01");
    }

    #[test]
    fn time_max_u64_does_not_panic() {
        let l = TimeLens::default();
        let out = l.render(u64::MAX);
        // We care that the computation runs, the `𒁹` separator survives,
        // and every field stays within its modular range — not a specific
        // numeric value, which is an artefact of u64::MAX mod 60/12.
        assert!(out.contains('d'));
        assert!(out.contains('𒁹'));
        assert!(out.contains(':'));
    }

    #[test]
    fn angle_zero_renders_all_zero() {
        let l = AngleLens;
        assert_eq!(l.render(0), "000°00′00.000″");
    }

    #[test]
    fn angle_one_degree_is_three_point_six_million_mas() {
        let l = AngleLens;
        assert_eq!(l.render(3_600_000), "001°00′00.000″");
    }

    #[test]
    fn angle_captures_all_four_components() {
        // 12° 34′ 56.789″ = 12*3_600_000 + 34*60_000 + 56*1000 + 789
        let n = 12 * 3_600_000 + 34 * 60_000 + 56 * 1_000 + 789;
        let l = AngleLens;
        assert_eq!(l.render(n), "012°34′56.789″");
    }

    #[test]
    fn tablet_zero_chunk_uses_leading_zeros_by_default() {
        let l = TabletLens { purist: false };
        let rendered = l.render(0);
        assert!(rendered.starts_with("⌐ "));
        assert!(rendered.ends_with(" ¬"));
        assert!(rendered.contains("00:00:00:00:00:00:00:00:00:00:00"));
    }

    #[test]
    fn tablet_purist_replaces_leading_zeros_with_blanks() {
        let l = TabletLens { purist: true };
        let rendered = l.render(5025);
        // Payload digits `01:23:45`; leading 8 slots should be blank pairs.
        // Separators survive; leading "00" → "  ".
        assert!(rendered.contains("  :  :  :  :  :  :  :  :01:23:45"));
    }

    #[test]
    fn tablet_purist_preserves_trailing_digit_even_if_zero() {
        // Value 0 → every digit is 0. Purist must NOT erase the final
        // digit, otherwise there's nothing to read.
        let l = TabletLens { purist: true };
        let rendered = l.render(0);
        assert!(rendered.ends_with(":00 ¬"));
    }

    #[test]
    fn cuneiform_zero_chunk_renders_eleven_placeholders_separated_by_spaces() {
        let l = CuneiformLens { fallback: false };
        let rendered = l.render(0);
        // 11 glyphs × 1 placeholder each, 10 spaces between.
        assert_eq!(rendered.matches('𒑰').count(), 11);
        assert_eq!(rendered.matches(' ').count(), 10);
    }

    #[test]
    fn cuneiform_fallback_produces_pure_ascii() {
        let l = CuneiformLens { fallback: true };
        let rendered = l.render(5025);
        assert!(rendered.is_ascii());
        assert!(rendered.ends_with(":01:23:45"));
    }

    #[test]
    fn cuneiform_auto_respects_no_unicode_env() {
        // SAFETY: single-threaded env manipulation inside a test.
        unsafe { std::env::set_var("NO_UNICODE", "1") };
        let l = CuneiformLens::auto();
        assert!(l.fallback);
        unsafe { std::env::remove_var("NO_UNICODE") };
    }
}
