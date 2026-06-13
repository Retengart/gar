//! Benchmark `decode::decode_stream` throughput over a pre-computed 1 MiB
//! dump. Dump generation runs once per bench process via `LazyLock`; only
//! `decode_stream` is inside the `b.iter(...)` block.
//!
//! Run: `cargo bench -p gar --bench decode`. Advisory only — see README.md.

#![allow(
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::large_stack_arrays,
    reason = "bench helper, not public API"
)]

use gar::__bench::{InputFormat, PALETTE_NONE, decode_stream, dump_all};
use criterion::{Criterion, criterion_group, criterion_main};
use std::io::sink;
use std::sync::LazyLock;

const SIZE: usize = 1 << 20; // 1 MiB raw input

// Deterministic pseudo-random raw input — `LazyLock` avoids the
// `long_running_const_eval` lint that fires on 1 MiB `const` arrays.
static RAW: LazyLock<Vec<u8>> = LazyLock::new(|| {
    (0..SIZE)
        .map(|i| (i.wrapping_mul(13).wrapping_add(7)) as u8)
        .collect()
});

// Render the 1 MiB raw input to plain-text dump bytes exactly once; reuse
// in every iteration. LazyLock keeps the cost out of the `b.iter` block.
static DUMPED: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut out = Vec::with_capacity(SIZE * 5); // rough upper bound
    dump_all(RAW.as_slice(), 0, &mut out, &PALETTE_NONE, None).expect("dump to Vec cannot fail");
    out
});

fn bench_decode_stream(c: &mut Criterion) {
    c.bench_function("decode_stream/1mib_plain_no_lens", |b| {
        b.iter(|| {
            let dumped: &[u8] = std::hint::black_box(&DUMPED);
            // Explicit Plain input format — bypasses auto-sniff for
            // deterministic measurement.
            let _ = decode_stream(dumped, &mut sink(), InputFormat::Plain);
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_decode_stream
}
criterion_main!(benches);
