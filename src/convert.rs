/// Number of base-60 digits needed to represent any u64.
/// 60^11 ≈ 3.65·10^19 > u64::MAX ≈ 1.84·10^19, so 11 digits always suffice.
pub const DIGITS: usize = 11;

/// Convert a u64 into its base-60 digits, most-significant first.
/// Index 0 is the highest-order digit, index DIGITS-1 is the ones place.
pub fn u64_to_base60(mut n: u64) -> [u8; DIGITS] {
    let mut out = [0u8; DIGITS];
    for i in (0..DIGITS).rev() {
        out[i] = (n % 60) as u8;
        n /= 60;
    }
    out
}

/// Format digits as zero-padded decimal pairs joined by ':',
/// e.g. [0,0,0,0,0,0,0,0,1,23,45] → "00:00:00:00:00:00:00:00:01:23:45".
pub fn format_digits(digits: &[u8; DIGITS]) -> String {
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

/// Width in columns of the formatted digit string (33 for DIGITS=11).
pub const DIGIT_STR_WIDTH: usize = DIGITS * 3 - 1;

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(n: u64) -> String {
        format_digits(&u64_to_base60(n))
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
    fn u64_max_fits_in_eleven_digits() {
        let d = u64_to_base60(u64::MAX);
        // Every digit must be a valid base-60 digit.
        for &x in &d {
            assert!(x < 60);
        }
        // Round-trip: reconstruct and compare.
        let mut n: u128 = 0;
        for &digit in &d {
            n = n * 60 + digit as u128;
        }
        assert_eq!(n, u64::MAX as u128);
    }

    #[test]
    fn roundtrip_random_sample() {
        for &n in &[
            1u64, 42, 3599, 3600, 1_000_000, 1_u64 << 32, (1u64 << 60) + 7,
        ] {
            let d = u64_to_base60(n);
            let mut back: u128 = 0;
            for &digit in &d {
                assert!(digit < 60);
                back = back * 60 + digit as u128;
            }
            assert_eq!(back, n as u128, "roundtrip failed for {n}");
        }
    }

    #[test]
    fn format_width() {
        let s = fmt(0);
        assert_eq!(s.len(), DIGIT_STR_WIDTH);
    }
}
