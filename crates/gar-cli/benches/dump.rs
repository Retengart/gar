//! Benchmark `dump::dump_all` throughput on a 1 MiB compile-time-constant
//! byte array with the monochrome palette and no lens.
//!
//! Phase 6 PERF-01 may extend this with a streaming-path comparison.
//! Run: `cargo bench -p gar --bench dump`. Advisory only — see README.md.

#![allow(
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::large_stack_arrays,
    reason = "bench helper, not public API"
)]

use gar::__bench::{PALETTE_NONE, dump_all};
use criterion::{Criterion, criterion_group, criterion_main};
use std::io::sink;
use std::sync::LazyLock;

const SIZE: usize = 1 << 20; // 1 MiB

// Deterministic pseudo-random fill via wrapping u8 arithmetic (D-28).
// No `rand` dep; same bytes every run. Computed once at first access via
// `LazyLock` so `const` evaluation doesn't hit `long_running_const_eval`
// for a 1 MiB array.
static INPUT: LazyLock<Vec<u8>> = LazyLock::new(|| {
    (0..SIZE)
        .map(|i| (i.wrapping_mul(13).wrapping_add(7)) as u8)
        .collect()
});

fn bench_dump_all_mono(c: &mut Criterion) {
    c.bench_function("dump_all/1mib_mono_no_lens", |b| {
        b.iter(|| {
            // `sink()` drains the writer — no real I/O, no allocation per line.
            let _ = dump_all(
                std::hint::black_box(INPUT.as_slice()),
                0,
                sink(),
                &PALETTE_NONE,
                None,
            );
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_dump_all_mono
}
criterion_main!(benches);
