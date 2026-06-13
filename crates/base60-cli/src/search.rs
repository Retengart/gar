//! Byte-pattern search for the interactive viewer.
//!
//! Patterns come in three syntactic shapes. The parser dispatches on the
//! shape before attempting a conversion, so an ambiguous-looking input
//! like `cafebabe` is handled by exactly one branch:
//!
//! | Input                 | Interpretation               |
//! |-----------------------|------------------------------|
//! | `hex:cafebabe`        | four bytes `ca fe ba be`     |
//! | `str:cafe`            | four bytes `c a f e` (ASCII) |
//! | `"cafe"`              | same as `str:cafe`           |
//! | `cafebabe`            | hex (auto-detected)          |
//! | `Hello, world!`       | string (auto-detected)       |
//!
//! Auto-detection uses a cheap rule: if every non-whitespace character is
//! a hex digit and the total digit count is even and non-zero, treat as
//! hex; otherwise treat as a UTF-8 string.

use std::str::FromStr;

/// Parsed pattern ready to hand to [`find_all`].
///
/// Widened to `pub` so the `#[cfg(fuzzing)] pub mod __fuzz` re-export in
/// `crate::lib` can surface it to the repo-root `fuzz/` crate. The
/// enclosing `mod search` is private at crate root, so this type is
/// still unreachable from the public API in non-fuzz builds
/// (Phase 5 TEST-02 SC5).
#[allow(unreachable_pub)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pattern(pub(crate) Vec<u8>);

/// Distinguishes user-facing failure modes from programming bugs.
///
/// Widened to `pub` alongside [`Pattern`]: `FromStr::Err` on a `pub` type
/// must itself be `pub`. `mod search` stays private at crate root, so
/// this enum is unreachable from the public API in non-fuzz builds.
#[allow(unreachable_pub)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// Pattern was empty after stripping any prefix/quotes.
    Empty,
    /// `hex:` prefix or auto-detected hex contained an odd number of digits
    /// or a non-hex character.
    InvalidHex,
}

impl FromStr for Pattern {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(ParseError::Empty);
        }

        if let Some(rest) = trimmed.strip_prefix("hex:") {
            return parse_hex(rest).map(Self);
        }
        if let Some(rest) = trimmed.strip_prefix("str:") {
            return Ok(Self(rest.as_bytes().to_vec()));
        }
        if let (Some(a), Some(b)) = (trimmed.strip_prefix('"'), trimmed.strip_suffix('"'))
            && b.len() >= 2
        {
            // `"x` (single quote only) fails the length guard and falls
            // through to the auto-detect branch below.
            return Ok(Self(a.as_bytes()[..a.len() - 1].to_vec()));
        }

        if looks_like_hex(trimmed) {
            parse_hex(trimmed).map(Self)
        } else {
            Ok(Self(trimmed.as_bytes().to_vec()))
        }
    }
}

fn looks_like_hex(s: &str) -> bool {
    let digits: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    !digits.is_empty()
        && digits.len().is_multiple_of(2)
        && digits.chars().all(|c| c.is_ascii_hexdigit())
}

fn parse_hex(input: &str) -> Result<Vec<u8>, ParseError> {
    let digits: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if digits.is_empty() || !digits.len().is_multiple_of(2) {
        return Err(ParseError::InvalidHex);
    }
    let mut out = Vec::with_capacity(digits.len() / 2);
    let bytes = digits.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let hi = hex_digit(pair[0]).ok_or(ParseError::InvalidHex)?;
        let lo = hex_digit(pair[1]).ok_or(ParseError::InvalidHex)?;
        out.push(hi * 16 + lo);
    }
    Ok(out)
}

const fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Every byte offset where `needle` occurs in `haystack`, non-overlapping.
///
/// `needle` is a byte slice, not a UTF-8 string — the viewer searches
/// binary data, so the pattern may contain arbitrary bytes.
///
/// Widened to `pub` so the `#[doc(hidden)] pub mod __bench` re-export in
/// `crate::lib` can surface it to `crates/base60-cli/benches/`. The
/// enclosing `mod search` is private at crate root, so this function is
/// still unreachable from the public API (Phase 5 PERF-06, TEST-02 SC5).
#[allow(unreachable_pub)]
#[must_use]
pub fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return Vec::new();
    }
    if needle.len() == 1 {
        memchr::memchr_iter(needle[0], haystack).collect()
    } else {
        memchr::memmem::find_iter(haystack, needle).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_prefix_wins_over_auto_detection() {
        let p: Pattern = "hex:0F".parse().unwrap();
        assert_eq!(p.0, vec![0x0f]);
    }

    #[test]
    fn str_prefix_takes_everything_literally() {
        let p: Pattern = "str:deadbeef".parse().unwrap();
        assert_eq!(p.0, b"deadbeef");
    }

    #[test]
    fn quoted_string_strips_quotes() {
        let p: Pattern = r#""ELF""#.parse().unwrap();
        assert_eq!(p.0, b"ELF");
    }

    #[test]
    fn auto_detects_hex_when_all_hex_digits_and_even() {
        let p: Pattern = "cafebabe".parse().unwrap();
        assert_eq!(p.0, vec![0xca, 0xfe, 0xba, 0xbe]);
    }

    #[test]
    fn spaces_in_hex_are_ignored() {
        let p: Pattern = "de ad be ef".parse().unwrap();
        assert_eq!(p.0, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn odd_hex_length_falls_through_or_errors() {
        let p: Pattern = "abc".parse().unwrap();
        // 3 hex chars is odd — auto-detect rejects it, so we fall back to
        // raw bytes.
        assert_eq!(p.0, b"abc");
    }

    #[test]
    fn explicit_hex_with_odd_length_errors() {
        assert_eq!("hex:abc".parse::<Pattern>(), Err(ParseError::InvalidHex));
    }

    #[test]
    fn non_hex_letters_are_plain_string() {
        let p: Pattern = "Hello".parse().unwrap();
        assert_eq!(p.0, b"Hello");
    }

    #[test]
    fn empty_pattern_errors() {
        assert_eq!("".parse::<Pattern>(), Err(ParseError::Empty));
        assert_eq!("   ".parse::<Pattern>(), Err(ParseError::Empty));
    }

    #[test]
    fn find_all_finds_every_non_overlapping_match() {
        assert_eq!(find_all(b"abcabcabc", b"abc"), vec![0, 3, 6]);
    }

    #[test]
    fn find_all_respects_non_overlap() {
        assert_eq!(find_all(b"aaaa", b"aa"), vec![0, 2]);
    }

    #[test]
    fn find_all_returns_empty_for_missing_needle() {
        assert_eq!(find_all(b"abc", b"xyz"), Vec::<usize>::new());
    }

    #[test]
    fn find_all_handles_empty_needle() {
        assert_eq!(find_all(b"abc", b""), Vec::<usize>::new());
    }

    #[test]
    fn find_all_handles_needle_longer_than_haystack() {
        assert_eq!(find_all(b"a", b"abcd"), Vec::<usize>::new());
    }
}
