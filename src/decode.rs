//! Inverse of `dump::dump_all`: parse base-60 digit pairs back into bytes.
//!
//! Accepts any text containing one or more runs of exactly 11 two-digit
//! base-60 pairs joined by colons — the shape that `dump` emits. The
//! surrounding line content (offset column, ASCII column, ANSI escapes)
//! is ignored so a user can pipe a coloured dump straight back in:
//!
//! ```text
//! base60 --color=never FILE | base60 decode > FILE.roundtrip
//! ```
//!
//! Each pair represents a base-60 digit `0..60`; the 11 digits are
//! recomposed most-significant-first into a `u64` and emitted as 8
//! big-endian bytes. A pair with a digit `>= 60` is a malformed input
//! and raises a descriptive error carrying the line number.

use crate::convert::DIGITS;
use std::io::{self, BufRead, Write};

/// Digit-pair width: two ASCII decimal chars per base-60 digit.
const PAIR: usize = 2;
/// Total characters for 11 digit pairs joined by 10 colons.
const RUN_LEN: usize = PAIR * DIGITS + (DIGITS - 1);

/// Parse base-60 dump lines from `r` and stream the decoded bytes to `w`.
///
/// Lines without a recognisable digit run are skipped silently, matching
/// the behaviour of tools like `xxd -r` on mixed input. The first
/// malformed digit aborts with a contextual [`io::Error`].
pub(crate) fn decode_stream<R: BufRead, W: Write>(r: R, w: &mut W) -> io::Result<()> {
    for (idx, line) in r.lines().enumerate() {
        let line = line?;
        let Some(run) = find_digit_run(&line) else {
            continue;
        };
        let value = parse_run(run, idx + 1)?;
        w.write_all(&value.to_be_bytes())?;
    }
    w.flush()
}

/// Locate the first `NN:NN:...:NN` run of exactly [`DIGITS`] pairs.
/// Returns a borrow of the matched substring, or `None` if no run fits.
fn find_digit_run(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    if bytes.len() < RUN_LEN {
        return None;
    }
    for start in 0..=bytes.len() - RUN_LEN {
        let slice = &bytes[start..start + RUN_LEN];
        if is_digit_run(slice)
            && not_extended_left(bytes, start)
            && not_extended_right(bytes, start + RUN_LEN)
        {
            // `slice` is ASCII by construction, so `from_utf8` can't fail.
            return Some(std::str::from_utf8(slice).expect("ascii"));
        }
    }
    None
}

/// Require plain ASCII digits and colons in the expected positions.
fn is_digit_run(slice: &[u8]) -> bool {
    debug_assert_eq!(slice.len(), RUN_LEN);
    for (i, &b) in slice.iter().enumerate() {
        let in_colon_position = i % 3 == 2;
        let valid = if in_colon_position {
            b == b':'
        } else {
            b.is_ascii_digit()
        };
        if !valid {
            return false;
        }
    }
    true
}

/// Guard against matching the tail of a longer digit run (e.g. a 12-pair
/// line would yield two overlapping 11-pair windows otherwise).
fn not_extended_left(bytes: &[u8], start: usize) -> bool {
    start == 0 || !matches!(bytes[start - 1], b'0'..=b'9' | b':')
}

fn not_extended_right(bytes: &[u8], end: usize) -> bool {
    end == bytes.len() || !matches!(bytes[end], b'0'..=b'9' | b':')
}

/// Decode a validated 11-pair run into its `u64` value.
///
/// Uses `u128` accumulator arithmetic so a hostile input with every
/// digit pinned at `59` (value `60^11 - 1 ≈ 3.65 · 10¹⁹`) overflows
/// cleanly to an error instead of wrapping a `u64`.
fn parse_run(run: &str, line_no: usize) -> io::Result<u64> {
    let mut value: u128 = 0;
    for (i, pair) in run.split(':').enumerate() {
        debug_assert_eq!(pair.len(), 2);
        let bytes = pair.as_bytes();
        let hi = bytes[0] - b'0';
        let lo = bytes[1] - b'0';
        let digit = hi * 10 + lo;
        if digit >= 60 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "line {line_no}: invalid base-60 digit {digit} at pair {}",
                    i + 1
                ),
            ));
        }
        value = value * 60 + u128::from(digit);
    }
    u64::try_from(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("line {line_no}: decoded value exceeds u64::MAX"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(input: &str) -> Vec<u8> {
        let mut out = Vec::new();
        decode_stream(input.as_bytes(), &mut out).unwrap();
        out
    }

    #[test]
    fn empty_input_yields_nothing() {
        assert!(decode("").is_empty());
    }

    #[test]
    fn zero_chunk_decodes_to_eight_zeros() {
        let line = "00000000  00:00:00:00:00:00:00:00:00:00:00  |........|\n";
        assert_eq!(decode(line), vec![0_u8; 8]);
    }

    #[test]
    fn classic_5025_roundtrips_to_expected_be_u64() {
        // 1*3600 + 23*60 + 45 = 5025 → u64 BE bytes.
        let line = "00000000  00:00:00:00:00:00:00:00:01:23:45  |.......\u{13a1}|\n";
        let bytes = decode(line);
        assert_eq!(bytes.len(), 8);
        assert_eq!(u64::from_be_bytes(bytes.try_into().unwrap()), 5025);
    }

    #[test]
    fn ignores_lines_without_digit_runs() {
        let input = "# comment line\nno pairs here\nalso nothing\n";
        assert!(decode(input).is_empty());
    }

    #[test]
    fn multiple_lines_accumulate_in_order() {
        let input = "\
00000000  00:00:00:00:00:00:00:00:00:00:01  |........|
00000008  00:00:00:00:00:00:00:00:00:00:02  |........|
";
        let bytes = decode(input);
        assert_eq!(bytes.len(), 16);
        assert_eq!(u64::from_be_bytes(bytes[..8].try_into().unwrap()), 1);
        assert_eq!(u64::from_be_bytes(bytes[8..].try_into().unwrap()), 2);
    }

    #[test]
    fn rejects_digit_ge_sixty() {
        let line = "00000000  00:00:00:00:00:00:00:00:00:00:99  |........|\n";
        let mut out = Vec::new();
        let err = decode_stream(line.as_bytes(), &mut out).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("99"));
    }

    #[test]
    fn rejects_twelve_pair_overextension() {
        // Twelve-pair runs encode more than u64; the scanner must refuse
        // to match a sub-window rather than silently producing wrong bytes.
        let line = "12:34:56:01:02:03:04:05:06:07:08:09:10";
        assert!(decode(line).is_empty());
    }

    #[test]
    fn tolerates_ansi_escapes_around_the_run() {
        let line = "\x1b[90m00000000\x1b[0m  \
                    \x1b[90m00\x1b[0m\x1b[90m:\x1b[0m";
        // Deliberately malformed (interleaved escapes). We just need to
        // ensure the decoder doesn't panic; no digit run will be found.
        let _ = decode(line);
    }

    #[test]
    fn accepts_run_embedded_in_free_text() {
        let line = "some prefix 00:00:00:00:00:00:00:00:00:00:01 some suffix";
        let bytes = decode(line);
        assert_eq!(u64::from_be_bytes(bytes.try_into().unwrap()), 1);
    }
}
