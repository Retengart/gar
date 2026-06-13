//! 140-cell roundtrip matrix — the full slice reinstated by Phase 4
//! REF-04 (length-preserving decode + JSON/HTML decode paths).
//!
//! Asserts the Core Value: `gar FILE | gar decode` round-trips
//! byte-identically for every `(fixture, LensConfig, Format)` cell:
//! 5 fixtures × 7 lens configs × 4 formats = 140 cells.
//!
//! Single `#[test]` by design (D-18) — one libtest entry, trivial
//! coverage arithmetic. First failing cell short-circuits; the panic
//! message (from `assert_roundtrip`) names the exact cell so re-runs
//! zero in immediately.

mod common;

use gar::Format;
use common::{
    ALL_FIXTURES, ALL_LENS_CONFIGS, LensConfig, ROUNDTRIP_FORMATS, assert_roundtrip, gar_cmd,
};

#[test]
fn roundtrip_matrix_byte_identical() {
    for (fx_label, fx_factory) in ALL_FIXTURES {
        let fx_bytes = fx_factory();
        for lens in ALL_LENS_CONFIGS {
            for fmt in ROUNDTRIP_FORMATS {
                one_cell(fx_label, &fx_bytes, *lens, *fmt);
            }
        }
    }
}

fn one_cell(fx_label: &str, fx_bytes: &[u8], lens: LensConfig, fmt: Format) {
    let cell_label = format!(
        "lens={lens_label} fmt={fmt:?} fixture={fx_label}",
        lens_label = lens.label(),
    );

    // Debug-build soft budget check (D-21). Never fails the test — just
    // prints a warning so local dev sees runaway cells. CI (release or
    // default profile) skips the check entirely.
    let cell_start = std::time::Instant::now();

    // Hop 1: stdin → gar → stdout (the dump).
    let fmt_arg = format!("--format={}", fmt_value(fmt));
    let mut args: Vec<&str> = vec!["--color=never", &fmt_arg];
    let lens_args = lens.cli_args();
    args.extend(lens_args.iter().copied());
    let dump_out = gar_cmd()
        .args(&args)
        .write_stdin(fx_bytes.to_vec())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Hop 2: dump → gar decode → stdout (raw bytes).
    let decoded = gar_cmd()
        .arg("decode")
        .write_stdin(dump_out)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_roundtrip(fx_bytes, &decoded, &cell_label);

    #[cfg(debug_assertions)]
    {
        let elapsed = cell_start.elapsed();
        if elapsed.as_millis() > 500 {
            eprintln!("WARN: cell '{cell_label}' took {elapsed:?} (budget 500ms)");
        }
    }
    // Silence "unused when not debug_assertions".
    let _ = cell_start;
}

const fn fmt_value(f: Format) -> &'static str {
    match f {
        Format::Ansi => "ansi",
        Format::Plain => "plain",
        Format::Json => "json",
        Format::Html => "html",
    }
}
