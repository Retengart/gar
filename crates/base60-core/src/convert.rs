//! Base-60 (sexagesimal) integer conversion, as used by the Sumerians
//! and later Babylonians.
//!
//! A [`u64`] is rendered as a sequence of exactly [`DIGITS`] base-60 digits,
//! most significant first. Eleven digits suffice because
//! `60.pow(11) ≈ 3.65 · 10¹⁹ > u64::MAX ≈ 1.84 · 10¹⁹`.

/// Number of base-60 digits required to represent any [`u64`].
pub const DIGITS: usize = 11;

/// Convert `n` into its base-60 digits, most-significant first.
///
/// Index `0` holds the highest-order digit; index `DIGITS - 1` holds the
/// ones place. Every returned byte is guaranteed to be `< 60`.
#[must_use]
#[inline]
pub fn u64_to_base60(mut n: u64) -> [u8; DIGITS] {
    let mut out = [0_u8; DIGITS];
    for slot in out.iter_mut().rev() {
        // `n % 60` is always in `0..60`, so the truncating cast is exact.
        *slot = (n % 60) as u8;
        n /= 60;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(n: u64) -> String {
        let digits = u64_to_base60(n);
        let mut s = String::with_capacity(DIGITS * 3 - 1);
        for (i, d) in digits.iter().enumerate() {
            if i > 0 {
                s.push(':');
            }
            s.push((b'0' + d / 10) as char);
            s.push((b'0' + d % 10) as char);
        }
        s
    }

    fn recompose(digits: &[u8; DIGITS]) -> u128 {
        digits
            .iter()
            .fold(0_u128, |acc, &d| acc * 60 + u128::from(d))
    }

    #[test]
    fn zero() {
        assert_eq!(fmt(0), "00:00:00:00:00:00:00:00:00:00:00");
    }

    #[test]
    fn fifty_nine() {
        assert_eq!(fmt(59), "00:00:00:00:00:00:00:00:00:00:59");
    }

    #[test]
    fn sixty_rolls_over() {
        assert_eq!(fmt(60), "00:00:00:00:00:00:00:00:00:01:00");
    }

    #[test]
    fn classic_example_5025() {
        // 1·60² + 23·60 + 45 = 5025
        assert_eq!(fmt(5025), "00:00:00:00:00:00:00:00:01:23:45");
    }

    #[test]
    fn u64_max_roundtrips_in_eleven_digits() {
        let d = u64_to_base60(u64::MAX);
        assert!(d.iter().all(|&x| x < 60));
        assert_eq!(recompose(&d), u128::from(u64::MAX));
    }

    #[test]
    fn roundtrip_samples() {
        for &n in &[
            0_u64,
            1,
            42,
            3599,
            3600,
            1_000_000,
            1_u64 << 32,
            (1_u64 << 60) + 7,
            u64::MAX - 1,
            u64::MAX,
        ] {
            let d = u64_to_base60(n);
            assert!(d.iter().all(|&x| x < 60));
            assert_eq!(recompose(&d), u128::from(n), "roundtrip failed for {n}");
        }
    }

    #[test]
    fn every_digit_is_valid() {
        for n in (0_u64..60_u64.pow(3)).step_by(7) {
            for &d in &u64_to_base60(n) {
                assert!(d < 60);
            }
        }
    }
}
