//! URL-safe base-60 encoding of a `u64`.
//!
//! A `u64` renders to exactly 11 characters drawn from an unambiguous
//! subset of the ASCII printables (`0-9A-Za-x`). Letters `y` and `z`
//! are intentionally omitted — together with their upper-case forms
//! they're the two alphabet positions most easily confused with `Y`/`Z`
//! in handwriting and low-DPI fonts, and dropping them gives us exactly
//! 60 symbols (10 digits + 26 upper + 24 lower).
//!
//! Example:
//!
//! ```
//! use gar_core::url::{decode_u64, encode_u64};
//! let n = 0xDEAD_BEEF_u64;
//! let encoded = encode_u64(n);
//! assert_eq!(encoded.len(), 11);
//! assert_eq!(decode_u64(&encoded).unwrap(), n);
//! ```
//!
//! Eleven characters is shorter than the 16 a hex encoding would
//! produce, and still URL-safe without percent-escaping.

use crate::convert::{DIGITS, u64_to_base60};

/// Unambiguous 60-symbol alphabet: `0-9`, `A-Z`, `a-x`.
///
/// Index `i` is the character for base-60 digit `i` (`0..60`).
pub const ALPHABET: &[u8; 60] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwx";

/// Every way `decode_u64` can refuse an input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodeError {
    /// Input was not exactly [`DIGITS`] ASCII characters.
    WrongLength,
    /// A character outside [`ALPHABET`] appeared.
    InvalidCharacter,
}

/// Render `n` as an 11-character URL-safe string.
#[must_use]
pub fn encode_u64(n: u64) -> String {
    let digits = u64_to_base60(n);
    digits
        .iter()
        .map(|&d| ALPHABET[d as usize] as char)
        .collect()
}

/// Parse a string produced by [`encode_u64`] back into a `u64`.
///
/// # Errors
///
/// Returns [`DecodeError::WrongLength`] if the input is not exactly
/// [`DIGITS`] characters, and [`DecodeError::InvalidCharacter`] if any
/// character is not in [`ALPHABET`].
pub fn decode_u64(s: &str) -> Result<u64, DecodeError> {
    if s.len() != DIGITS {
        return Err(DecodeError::WrongLength);
    }
    let mut value: u64 = 0;
    for c in s.bytes() {
        let d = alphabet_index(c).ok_or(DecodeError::InvalidCharacter)?;
        // `u64_to_base60` produces digits in `0..60`, so 11 digits of
        // `59` yield `60¹¹ − 1 ≈ 3.65 · 10¹⁹` — just under `u64::MAX *
        // 2`. Using `wrapping_mul` would silently corrupt such values,
        // so we prefer checked arithmetic and report overflow as an
        // invalid-character error (the alphabet is the only thing the
        // caller can influence).
        value = value
            .checked_mul(60)
            .and_then(|v| v.checked_add(u64::from(d)))
            .ok_or(DecodeError::InvalidCharacter)?;
    }
    Ok(value)
}

const fn alphabet_index(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'Z' => Some(c - b'A' + 10),
        b'a'..=b'x' => Some(c - b'a' + 36),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alphabet_has_exactly_sixty_unique_characters() {
        assert_eq!(ALPHABET.len(), 60);
        let mut sorted = ALPHABET.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 60);
    }

    #[test]
    fn alphabet_excludes_ambiguous_letters_y_and_z() {
        assert!(!ALPHABET.contains(&b'y'));
        assert!(!ALPHABET.contains(&b'z'));
        // Upper-case Y/Z remain — the distinction between upper/lower
        // cases never shows up in lower-case-only handwriting issues.
        assert!(ALPHABET.contains(&b'Y'));
        assert!(ALPHABET.contains(&b'Z'));
    }

    #[test]
    fn encode_always_produces_eleven_chars() {
        for n in [0, 1, 42, u64::MAX, u64::MAX / 2, 0xdead_beef] {
            assert_eq!(encode_u64(n).len(), DIGITS);
        }
    }

    #[test]
    fn roundtrip_samples() {
        for n in [
            0,
            1,
            42,
            60,
            60 * 60,
            1_000_000,
            u64::MAX / 3,
            u64::MAX - 1,
            u64::MAX,
        ] {
            let s = encode_u64(n);
            assert_eq!(decode_u64(&s), Ok(n), "roundtrip failed for {n}");
        }
    }

    #[test]
    fn zero_encodes_to_all_first_alphabet_character() {
        assert_eq!(encode_u64(0), "00000000000");
    }

    #[test]
    fn decode_rejects_wrong_length() {
        assert_eq!(decode_u64(""), Err(DecodeError::WrongLength));
        assert_eq!(decode_u64("short"), Err(DecodeError::WrongLength));
        assert_eq!(
            decode_u64("00000000000extra"),
            Err(DecodeError::WrongLength)
        );
    }

    #[test]
    fn decode_rejects_invalid_character() {
        // Underscore is outside the alphabet.
        assert_eq!(
            decode_u64("___________"),
            Err(DecodeError::InvalidCharacter)
        );
        // Neither 'y' nor 'z' is in the alphabet.
        assert_eq!(
            decode_u64("00000000000".replace('0', "y").as_str()),
            Err(DecodeError::InvalidCharacter)
        );
    }

    #[test]
    fn decode_rejects_overflow() {
        // `yyyyyyyyyyy` would encode `59·60⁰ + ... + 59·60¹⁰ = 60¹¹ − 1`
        // which overflows `u64::MAX`. But `y` isn't in the alphabet; use
        // the actual 60th position, which is `x` (index 59).
        let max_alphabet_string = "xxxxxxxxxxx";
        assert_eq!(
            decode_u64(max_alphabet_string),
            Err(DecodeError::InvalidCharacter)
        );
    }

    #[test]
    fn encoded_strings_are_url_safe() {
        // Every character in the alphabet is URL-safe without percent
        // encoding per RFC 3986 "unreserved".
        for &c in ALPHABET {
            assert!(
                c.is_ascii_alphanumeric(),
                "alphabet byte {c:#x} is not alphanumeric"
            );
        }
    }

    #[test]
    fn hash_prefix_fits_in_eleven_chars() {
        // Practical use case: take first 8 bytes of a SHA-256 digest and
        // encode. Hex of 8 bytes = 16 chars; URL-safe gar = 11 chars.
        let prefix = 0xDEAD_BEEF_CAFE_BABE_u64;
        let encoded = encode_u64(prefix);
        assert!(encoded.len() < 16, "{encoded} should be shorter than hex");
    }
}
