//! Statistical analysis of arbitrary byte slices.
//!
//! Produces three derived views in a single [`Analysis`] record:
//!
//! * Overall **Shannon entropy** (bits/byte) + per-window sparkline.
//! * **Byte frequency** histogram (counts, most-common, least-common).
//! * **ASCII regions** — runs of at least [`MIN_ASCII_RUN`] consecutive
//!   printable bytes, plus per-window tiers classifying each window as
//!   low-entropy (`< 1 bit/byte`, likely padding) or high-entropy
//!   (`> 7.5 bits/byte`, likely compressed/encrypted).
//!
//! The scan is single-pass for window entropy, region classification,
//! and summary stats — all three are computed in the same loop. Overall
//! byte frequency and ASCII regions each require their own pass over the
//! data (different axes). All passes are O(n) with `window_size` factored
//! out. Memory footprint is bounded: the global histogram is fixed-size,
//! only one window worth of counts lives on the stack at a time, and
//! summary statistics use an online accumulator.

use std::io::{self, Write};

/// Minimum number of consecutive printable bytes needed to promote a span
/// into an [`RegionKind::Ascii`] region. `4` matches the `strings(1)`
/// default and filters out coincidental printable bytes in binary data.
const MIN_ASCII_RUN: usize = 4;

/// Smallest `window_size` accepted by [`analyze`]. Smaller windows make
/// Shannon entropy dominated by noise (eight possible bit patterns fit in
/// 3 bits; you need at least tens of samples to distinguish distributions).
pub(crate) const MIN_WINDOW: usize = 64;

/// Default window size — a byte-block boundary that keeps memory locality
/// good on modern caches while still giving ~256 samples per window.
pub(crate) const DEFAULT_WINDOW: usize = 256;

/// Threshold above which a window is classified as high-entropy
/// ("likely compressed/encrypted"). Uniform random data tends to sit in
/// the `7.9..=8.0` range for 8-bit samples; `7.5` admits slight skew.
const HIGH_ENTROPY: f32 = 7.5;

use crate::chunk::is_printable;

/// Threshold below which a window is classified as low-entropy
/// ("likely padding or zero fill"). A window of a single repeated byte
/// has entropy `0`; `1.0` admits a handful of distinct bytes.
const LOW_ENTROPY: f32 = 1.0;

/// A contiguous range of bytes classified under a single [`RegionKind`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Region {
    /// Inclusive start offset into the analysed slice.
    pub(crate) start: usize,
    /// Exclusive end offset. `end - start` is the region's length.
    pub(crate) end: usize,
    /// What classification qualifies this range.
    pub(crate) kind: RegionKind,
}

/// Semantic category of a detected byte region.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum RegionKind {
    /// Run of consecutive ASCII graphic bytes or spaces (≥ [`MIN_ASCII_RUN`]).
    Ascii,
    /// Entropy-window span classified as likely compressed/encrypted.
    HighEntropy,
    /// Entropy-window span classified as likely zero/padding.
    LowEntropy,
}

/// Online accumulator for per-window Shannon entropy statistics.
///
/// Tracks min, max, and running sum in a single pass without storing
/// individual window values, keeping memory use bounded regardless of
/// input size.
#[derive(Copy, Clone, Debug)]
struct EntropyStats {
    min: f32,
    max: f32,
    sum: f64,
    count: usize,
}

impl EntropyStats {
    const fn new() -> Self {
        Self {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
            sum: 0.0,
            count: 0,
        }
    }

    fn push(&mut self, h: f32) {
        if h < self.min {
            self.min = h;
        }
        if h > self.max {
            self.max = h;
        }
        self.sum += f64::from(h);
        self.count += 1;
    }

    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, reason = "entropy calculation uses f64 casts")]
    fn mean(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }
        (self.sum / self.count as f64).clamp(0.0, 8.0) as f32
    }
}

/// Computed view over a byte slice.
#[derive(Clone, Debug)]
pub(crate) struct Analysis {
    /// Number of bytes considered.
    pub(crate) total_bytes: usize,
    /// Window size actually used (always `>= MIN_WINDOW`).
    pub(crate) window_size: usize,
    /// Overall Shannon entropy, in bits per byte (`0.0..=8.0`).
    pub(crate) entropy: f32,
    /// Shannon entropy for each complete window, in order. The trailing
    /// partial window (if any) is not included so short tails don't
    /// skew the sparkline.
    pub(crate) entropy_windows: Vec<f32>,
    /// Frequency of every byte value. `byte_freq[b]` is the count of byte
    /// `b`. Stack-allocated (1 KiB) — cheaper than heap-boxing for a
    /// single-pass analysis that is created once and consumed once.
    pub(crate) byte_freq: [u32; 256],
    /// Disjoint classified regions, in start-offset order.
    pub(crate) regions: Vec<Region>,
    /// Min, max, mean of per-window entropy values.
    pub(crate) entropy_stats: (f32, f32, f32),
}

/// Run the full analysis pipeline on `data`.
///
/// `window_size` is silently clamped to [`MIN_WINDOW`] if smaller, so
/// callers cannot accidentally trigger single-sample entropy noise.
#[must_use]
pub(crate) fn analyze(data: &[u8], window_size: usize) -> Analysis {
    let window = window_size.max(MIN_WINDOW);
    let mut byte_freq = [0_u32; 256];
    for &b in data {
        byte_freq[b as usize] = byte_freq[b as usize].saturating_add(1);
    }
    let entropy = shannon_entropy(&byte_freq, data.len());

    // Single pass: compute per-window entropy, detect entropy-tier
    // regions, and accumulate min/max/mean stats simultaneously.
    let complete = data.len() / window;
    let mut entropy_windows = Vec::with_capacity(complete);
    let mut stats = EntropyStats::new();
    let mut regions = Vec::new();

    for (idx, chunk) in data.chunks_exact(window).enumerate() {
        let mut hist = [0_u32; 256];
        for &b in chunk {
            hist[b as usize] += 1;
        }
        let h = shannon_entropy(&hist, chunk.len());
        entropy_windows.push(h);
        stats.push(h);

        let kind = if h >= HIGH_ENTROPY {
            RegionKind::HighEntropy
        } else if h <= LOW_ENTROPY {
            RegionKind::LowEntropy
        } else {
            continue;
        };
        let start = idx * window;
        regions.push(Region {
            start,
            end: start + window,
            kind,
        });
    }

    detect_ascii_regions(data, &mut regions);
    regions.sort_by_key(|r| r.start);

    let entropy_stats = if stats.count == 0 {
        (0.0, 0.0, 0.0)
    } else {
        (stats.min, stats.max, stats.mean())
    };

    Analysis {
        total_bytes: data.len(),
        window_size: window,
        entropy,
        entropy_windows,
        byte_freq,
        regions,
        entropy_stats,
    }
}

/// Shannon entropy, in bits/byte, over a pre-computed `256`-bin histogram.
/// Returns `0.0` for an empty sample so the caller never divides by zero.
fn shannon_entropy(hist: &[u32; 256], total: usize) -> f32 {
    if total == 0 {
        return 0.0;
    }
    // `total` fits in f64 without loss up to 2^52 bytes (~4 PB) — far
    // beyond any realistic file. Using f64 internally avoids log2
    // accuracy loss on the probabilities for strongly-skewed inputs.
    #[allow(clippy::cast_precision_loss, reason = "byte ratio as percentage")]
    let t = total as f64;
    let mut h = 0_f64;
    for &c in hist {
        if c == 0 {
            continue;
        }
        let p = f64::from(c) / t;
        h = p.mul_add(-p.log2(), h);
    }
    // Truncating to f32 for storage; the value is bounded in [0, 8] so
    // only the mantissa shrinks, never the exponent.
    #[allow(clippy::cast_possible_truncation, reason = "region offset fits in usize")]
    let out = h as f32;
    out.clamp(0.0, 8.0)
}

/// ASCII-run detection, appending to an existing region vec.
fn detect_ascii_regions(data: &[u8], regions: &mut Vec<Region>) {
    let mut run_start: Option<usize> = None;

    for (i, &b) in data.iter().enumerate() {
        if is_printable(b) {
            run_start.get_or_insert(i);
        } else if let Some(start) = run_start.take()
            && i - start >= MIN_ASCII_RUN
        {
            regions.push(Region {
                start,
                end: i,
                kind: RegionKind::Ascii,
            });
        }
    }
    if let Some(start) = run_start
        && data.len() - start >= MIN_ASCII_RUN
    {
        regions.push(Region {
            start,
            end: data.len(),
            kind: RegionKind::Ascii,
        });
    }
}

/// Write a human-readable summary of `a` to `w`, using `data` for the
/// ASCII preview lines.
///
/// Format is plain text, newline-terminated, suitable for piping into
/// `grep`, `awk`, or copy/paste into reports. No ANSI escapes.
pub(crate) fn write_summary<W: Write>(a: &Analysis, data: &[u8], w: &mut W) -> io::Result<()> {
    writeln!(w, "bytes         {}", a.total_bytes)?;
    writeln!(w, "entropy       {:.3} bits/byte", a.entropy)?;
    writeln!(w, "window        {}", a.window_size)?;
    writeln!(w, "windows       {}", a.entropy_windows.len())?;

    if !a.entropy_windows.is_empty() {
        let (min, max, mean) = a.entropy_stats;
        writeln!(w, "window range  [{min:.3}, {max:.3}]  mean {mean:.3}")?;
    }

    let unique = a.byte_freq.iter().filter(|&&c| c > 0).count();
    writeln!(w, "unique bytes  {unique} / 256")?;

    // Top 5 most-frequent bytes — handy for spotting padding (`0x00`) or
    // text-like dominance (`0x20`, `0x65`, …).
    let mut freq_idx: Vec<(usize, u32)> = a
        .byte_freq
        .iter()
        .enumerate()
        .map(|(i, &c)| (i, c))
        .filter(|&(_, c)| c > 0)
        .collect();
    freq_idx.sort_unstable_by_key(|&(_, c)| std::cmp::Reverse(c));
    writeln!(w, "top bytes")?;
    for &(b, c) in freq_idx.iter().take(5) {
        let pct = percentage(c, a.total_bytes);
        let byte = u8::try_from(b).unwrap_or(0);
        let glyph = if is_printable(byte) {
            format!("{:?}", byte as char)
        } else {
            String::from("    ")
        };
        writeln!(w, "  0x{b:02x} {glyph:<6}  {c:>10}  {pct:>6.2}%")?;
    }

    // Region tally and first handful of ASCII strings (cheap preview).
    let (ascii, high, low) = region_counts(&a.regions);
    writeln!(
        w,
        "regions       ascii={ascii}  high-entropy={high}  low-entropy={low}"
    )?;
    let previews: Vec<&Region> = a
        .regions
        .iter()
        .filter(|r| r.kind == RegionKind::Ascii)
        .take(5)
        .collect();
    if !previews.is_empty() {
        writeln!(w, "ascii preview")?;
        for r in previews {
            let s = std::str::from_utf8(data.get(r.start..r.end).unwrap_or(b"")).unwrap_or("");
            writeln!(w, "  0x{:08x}..0x{:08x}  {s:?}", r.start, r.end)?;
        }
    }

    Ok(())
}

fn region_counts(regions: &[Region]) -> (usize, usize, usize) {
    let mut ascii = 0;
    let mut high = 0;
    let mut low = 0;
    for r in regions {
        match r.kind {
            RegionKind::Ascii => ascii += 1,
            RegionKind::HighEntropy => high += 1,
            RegionKind::LowEntropy => low += 1,
        }
    }
    (ascii, high, low)
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, reason = "entropy calculation uses f64 casts")]
fn percentage(count: u32, total: usize) -> f32 {
    if total == 0 {
        return 0.0;
    }
    (f64::from(count) * 100.0 / total as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_analyses_without_nan() {
        let a = analyze(&[], DEFAULT_WINDOW);
        assert_eq!(a.total_bytes, 0);
        assert!(a.entropy.abs() < f32::EPSILON);
        assert!(a.entropy_windows.is_empty());
        assert!(a.regions.is_empty());
    }

    #[test]
    fn zero_fill_has_zero_entropy() {
        let a = analyze(&[0_u8; 1024], DEFAULT_WINDOW);
        assert!(a.entropy.abs() < f32::EPSILON);
        // Every window is low-entropy.
        assert!(a.regions.iter().all(|r| r.kind == RegionKind::LowEntropy));
    }

    #[test]
    fn uniform_byte_distribution_approaches_eight_bits() {
        // Every byte value appears exactly 64 times → perfectly uniform.
        let mut data = Vec::with_capacity(256 * 64);
        for b in 0..=255_u8 {
            for _ in 0..64 {
                data.push(b);
            }
        }
        let a = analyze(&data, 256);
        // Shannon entropy of uniform 8-bit distribution = 8.000 bits/byte.
        assert!((a.entropy - 8.0).abs() < 1e-3, "entropy={}", a.entropy);
    }

    #[test]
    fn window_size_is_clamped_to_minimum() {
        let a = analyze(&[0_u8; 200], 1);
        assert_eq!(a.window_size, MIN_WINDOW);
    }

    #[test]
    fn ascii_run_is_detected() {
        // Control bytes around a plain ASCII string.
        let mut data = vec![0_u8; 10];
        data.extend_from_slice(b"Hello, world!");
        data.extend_from_slice(&[0_u8; 10]);

        let a = analyze(&data, DEFAULT_WINDOW);
        let ascii: Vec<_> = a
            .regions
            .iter()
            .filter(|r| r.kind == RegionKind::Ascii)
            .collect();
        assert_eq!(ascii.len(), 1);
        assert_eq!(ascii[0].start, 10);
        assert_eq!(ascii[0].end, 23);
    }

    #[test]
    fn short_ascii_run_is_ignored() {
        // 3 bytes < MIN_ASCII_RUN.
        let data = b"\x00\x00Hi\x00\x00";
        let a = analyze(data, DEFAULT_WINDOW);
        assert!(a.regions.iter().all(|r| r.kind != RegionKind::Ascii));
    }

    #[test]
    fn byte_histogram_matches_input() {
        let data = b"aaabbbbcccccdddddd";
        let a = analyze(data, DEFAULT_WINDOW);
        assert_eq!(a.byte_freq[b'a' as usize], 3);
        assert_eq!(a.byte_freq[b'b' as usize], 4);
        assert_eq!(a.byte_freq[b'c' as usize], 5);
        assert_eq!(a.byte_freq[b'd' as usize], 6);
    }

    #[test]
    fn regions_are_sorted_by_start() {
        let mut data = vec![0_u8; 128];
        data.extend_from_slice(b"Middle string here");
        data.extend_from_slice(&[0_u8; 128]);
        let a = analyze(&data, 64);
        let starts: Vec<_> = a.regions.iter().map(|r| r.start).collect();
        let mut sorted = starts.clone();
        sorted.sort_unstable();
        assert_eq!(starts, sorted);
    }

    #[test]
    fn summary_writes_non_empty_output() {
        let data = b"The quick brown fox jumps over the lazy dog. ".repeat(10);
        let a = analyze(&data, 64);
        let mut buf = Vec::new();
        write_summary(&a, &data, &mut buf).unwrap();
        let s = std::str::from_utf8(&buf).unwrap();
        assert!(s.contains("bytes"));
        assert!(s.contains("entropy"));
        assert!(s.contains("top bytes"));
        assert!(s.contains("ascii preview"));
    }
}
