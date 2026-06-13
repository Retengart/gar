//! Benchmark `Lens::render(&self, u64) -> String` for all four implementations.
//!
//! Phase 6 PERF-04 adds `render_to<W: Write>`; this bench gets extended
//! there. For Phase 5 we measure only the current `render` surface.
//! Run: `cargo bench -p gar-core --bench lens`.

#![allow(
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    reason = "bench helper, not public API"
)]

use criterion::{Criterion, criterion_group, criterion_main};
use gar_core::{AngleLens, CuneiformLens, Lens, TabletLens, TimeLens};

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

fn bench_lens_render(c: &mut Criterion) {
    let mut g = c.benchmark_group("lens/render");

    let time = TimeLens::default();
    g.bench_function("time", |b| {
        b.iter(|| {
            for &n in &INPUTS {
                let s = time.render(std::hint::black_box(n));
                std::hint::black_box(s);
            }
        });
    });

    let angle = AngleLens;
    g.bench_function("angle", |b| {
        b.iter(|| {
            for &n in &INPUTS {
                std::hint::black_box(angle.render(std::hint::black_box(n)));
            }
        });
    });

    let tablet = TabletLens { purist: false };
    g.bench_function("tablet", |b| {
        b.iter(|| {
            for &n in &INPUTS {
                std::hint::black_box(tablet.render(std::hint::black_box(n)));
            }
        });
    });

    // Use `fallback: true` (ASCII path) for deterministic CI results —
    // `CuneiformLens::auto()` reads env vars which violates bench
    // reproducibility.
    let cuneiform = CuneiformLens { fallback: true };
    g.bench_function("cuneiform_ascii", |b| {
        b.iter(|| {
            for &n in &INPUTS {
                std::hint::black_box(cuneiform.render(std::hint::black_box(n)));
            }
        });
    });

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().noise_threshold(0.05).sample_size(50);
    targets = bench_lens_render
}
criterion_main!(benches);
