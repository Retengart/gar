//! Benchmark for `base60_core::convert::u64_to_base60` — the hot-path
//! conversion called once per dump line.
//!
//! Input: 1024 deterministic `u64` values generated via `wrapping_mul`.
//! Run: `cargo bench -p base60-core --bench convert`. Advisory only —
//! see `../benches/README.md`.

#![allow(
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation
)]

use base60_core::convert::u64_to_base60;
use criterion::{Criterion, criterion_group, criterion_main};

/// Deterministic 1024-element input; seed pattern locked so re-runs are
/// bit-identical. `0x9E3779B97F4A7C15` is `2^64 / φ` (the Fibonacci hash
/// multiplier) — well-distributed bit patterns across 64 bits.
const FIB_MUL: u64 = 0x9E37_79B9_7F4A_7C15;
const INPUTS: [u64; 1024] = {
    let mut arr = [0_u64; 1024];
    let mut i = 0_u64;
    while i < 1024 {
        arr[i as usize] = i.wrapping_mul(FIB_MUL);
        i += 1;
    }
    arr
};

fn bench_u64_to_base60(c: &mut Criterion) {
    c.bench_function("u64_to_base60/1024_fib_inputs", |b| {
        b.iter(|| {
            let mut total: u64 = 0;
            for &n in &INPUTS {
                let digits = u64_to_base60(std::hint::black_box(n));
                total = total.wrapping_add(u64::from(digits[0]));
            }
            total
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_u64_to_base60
}
criterion_main!(benches);
