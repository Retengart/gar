//! Cuneiform glyph lookup for base-60 digits `0..60`.
//!
//! Digits are composed from two Sumero-Babylonian numeric wedges:
//!
//! | Glyph | Codepoint | Meaning |
//! |-------|-----------|---------|
//! | `𒁹`   | U+12079   | `AŠ` — vertical wedge, value `1`   |
//! | `𒌋`   | U+1230B   | `U`  — corner wedge, value `10`    |
//!
//! A digit `d` in `1..60` is rendered as `(d / 10)` copies of `𒌋` followed
//! by `(d % 10)` copies of `𒁹`. The Sumerians themselves had no true zero;
//! Late-Babylonian astronomers introduced a positional placeholder. We use
//! `𒑰` (U+12470) — a recognised Cuneiform punctuation sign — to stand in
//! for `d == 0`, since an empty string would collapse formatting.
//!
//! The table is built once on first access via [`std::sync::LazyLock`] and
//! returned as `&'static str`, so repeated lookups in the render hot path
//! are allocation-free.

use std::sync::LazyLock;

/// Vertical-wedge glyph (`1`). UTF-8: `F0 92 81 B9`.
const AS_WEDGE: &str = "𒁹";

/// Corner-wedge glyph (`10`). UTF-8: `F0 92 8C 8B`.
const U_WEDGE: &str = "𒌋";

/// Late-Babylonian zero placeholder. Chosen over an empty string so the
/// output preserves column alignment.
const ZERO_MARK: &str = "𒑰";

/// ASCII fallback rendering of a base-60 digit as a two-character decimal
/// pair (e.g. `07`, `42`). Used when the terminal cannot render cuneiform.
#[inline]
#[must_use]
pub(crate) fn ascii_pair(d: u8) -> [u8; 2] {
    debug_assert!(d < 60);
    [b'0' + d / 10, b'0' + d % 10]
}

static GLYPHS: LazyLock<[String; 60]> = LazyLock::new(|| {
    std::array::from_fn(|i| {
        if i == 0 {
            return ZERO_MARK.to_owned();
        }
        // `i` is in `1..60`, so the `u8` cast is exact.
        let d = u8::try_from(i).expect("digit < 60");
        let tens = usize::from(d / 10);
        let ones = usize::from(d % 10);
        let mut s = String::with_capacity((tens + ones) * AS_WEDGE.len());
        for _ in 0..tens {
            s.push_str(U_WEDGE);
        }
        for _ in 0..ones {
            s.push_str(AS_WEDGE);
        }
        s
    })
});

/// Return the cuneiform glyph string for base-60 digit `d` (`0..60`).
///
/// The returned reference is valid for the remainder of the process.
///
/// # Panics
///
/// Panics in debug builds if `d >= 60`. In release builds the indexing
/// panic fires instead — both are bugs in the caller.
#[inline]
#[must_use]
pub(crate) fn glyph(d: u8) -> &'static str {
    debug_assert!(d < 60);
    &GLYPHS[d as usize]
}

/// Return `true` if the current environment suggests cuneiform cannot be
/// rendered legibly and callers should prefer [`ascii_pair`].
///
/// Checks, in order:
///
/// 1. `NO_UNICODE` env var set and non-empty — explicit opt-out.
/// 2. `TERM=dumb` — terminal does not support rich glyphs.
#[must_use]
pub(crate) fn ascii_fallback_forced() -> bool {
    if std::env::var_os("NO_UNICODE").is_some_and(|v| !v.is_empty()) {
        return true;
    }
    matches!(std::env::var("TERM").as_deref(), Ok("dumb"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_uses_placeholder() {
        assert_eq!(glyph(0), ZERO_MARK);
    }

    #[test]
    fn one_is_single_vertical_wedge() {
        assert_eq!(glyph(1), AS_WEDGE);
    }

    #[test]
    fn nine_is_nine_verticals() {
        assert_eq!(glyph(9), AS_WEDGE.repeat(9));
    }

    #[test]
    fn ten_is_single_corner_wedge() {
        assert_eq!(glyph(10), U_WEDGE);
    }

    #[test]
    fn fifty_nine_is_five_corners_plus_nine_verticals() {
        let expected = format!("{}{}", U_WEDGE.repeat(5), AS_WEDGE.repeat(9));
        assert_eq!(glyph(59), expected);
    }

    #[test]
    fn decomposition_is_consistent() {
        // Every digit d in 1..60 must be (d/10) corners + (d%10) verticals.
        for d in 1_u8..60 {
            let tens = d / 10;
            let ones = d % 10;
            let g = glyph(d);
            let corners = g.matches(U_WEDGE).count();
            let verticals = g.matches(AS_WEDGE).count();
            assert_eq!(corners, usize::from(tens), "d={d} corners");
            assert_eq!(verticals, usize::from(ones), "d={d} verticals");
        }
    }

    #[test]
    fn static_references_are_stable_across_calls() {
        // LazyLock should cache, so repeated calls return the same &str.
        assert!(std::ptr::eq(glyph(42), glyph(42)));
    }

    #[test]
    fn ascii_pair_matches_decimal_formatting() {
        for d in 0..60 {
            let pair = ascii_pair(d);
            let expected = format!("{d:02}");
            assert_eq!(core::str::from_utf8(&pair).unwrap(), expected);
        }
    }

    #[test]
    fn fallback_detection_respects_no_unicode_env() {
        // SAFETY: env manipulation is unsafe in Rust 2024; tests are
        // single-threaded inside a process and clean up after themselves.
        unsafe { std::env::set_var("NO_UNICODE", "1") };
        assert!(ascii_fallback_forced());
        unsafe { std::env::remove_var("NO_UNICODE") };
        // Only assert the negative when TERM isn't 'dumb'; CI may set it.
        if std::env::var("TERM").as_deref() != Ok("dumb") {
            assert!(!ascii_fallback_forced());
        }
    }
}
