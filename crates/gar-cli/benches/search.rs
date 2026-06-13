//! Benchmark `search::find_all` with the four mandatory cells from
//! PITFALLS Pitfall 4. This bench is the gating baseline for Phase 6
//! PERF-03 (`memchr::memmem` swap).
//!
//! Run: `cargo bench -p gar --bench search`. Advisory only — see README.md.

#![allow(
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::large_stack_arrays,
    reason = "bench helper, not public API"
)]

use gar::__bench::find_all;
use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::LazyLock;

const HAY_SIZE: usize = 1 << 20; // 1 MiB haystack

// Heap-allocated via `LazyLock` so the haystack lives in `.bss` / heap
// rather than being copied into the bench closure's stack frame
// (avoids clippy's `large_stack_frames` on 1 MiB inputs).
static ZERO_FILL: LazyLock<Vec<u8>> = LazyLock::new(|| vec![0_u8; HAY_SIZE]);

// Deterministic pseudo-random fill via `LazyLock` to avoid
// `long_running_const_eval` on a 1 MiB `const` array.
static RANDOM_FILL: LazyLock<Vec<u8>> = LazyLock::new(|| {
    (0..HAY_SIZE)
        .map(|i| (i.wrapping_mul(13).wrapping_add(7)) as u8)
        .collect()
});

fn bench_find_all(c: &mut Criterion) {
    let mut g = c.benchmark_group("find_all");

    // Cell 1: 1-byte needle on zero-fill haystack (1-byte dispatch).
    g.bench_function("zero_fill/1byte_null", |b| {
        b.iter(|| {
            find_all(
                std::hint::black_box(ZERO_FILL.as_slice()),
                std::hint::black_box(b"\x00"),
            )
        });
    });

    // Cell 2: 2-byte needle on zero-fill haystack (packed-pair prefilter).
    g.bench_function("zero_fill/2byte_ffff", |b| {
        b.iter(|| {
            find_all(
                std::hint::black_box(ZERO_FILL.as_slice()),
                std::hint::black_box(b"\xff\xff"),
            )
        });
    });

    // Cell 3: 3-byte needle on random haystack.
    g.bench_function("random/3byte_elf", |b| {
        b.iter(|| {
            find_all(
                std::hint::black_box(RANDOM_FILL.as_slice()),
                std::hint::black_box(b"ELF"),
            )
        });
    });

    // Cell 4: 8-byte needle on random haystack.
    g.bench_function("random/8byte_cafebabe", |b| {
        b.iter(|| {
            find_all(
                std::hint::black_box(RANDOM_FILL.as_slice()),
                std::hint::black_box(b"cafebabe"),
            )
        });
    });

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_find_all
}
criterion_main!(benches);
