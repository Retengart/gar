//! 140-cell roundtrip matrix (5 fixtures × 7 lens configs × 4 formats).
//!
//! Asserts the Core Value: `base60 FILE | base60 decode` round-trips
//! byte-identically for every `(fixture, LensConfig, Format)` cell.
//! Single `#[test]` by design (D-18) — one libtest entry, trivial
//! coverage arithmetic. First failing cell short-circuits; the panic
//! message (from `assert_roundtrip`) names the exact cell so re-runs
//! zero in immediately.

mod common;

use base60::Format;
use common::{ALL_LENS_CONFIGS, LensConfig, assert_roundtrip, base60_cmd, fixtures};

#[test]
fn roundtrip_matrix_byte_identical() {
    let all_fixtures: &[(&str, Vec<u8>)] = &[
        ("minimal_elf", fixtures::minimal_elf()),
        ("minimal_png", fixtures::minimal_png()),
        ("minimal_zip", fixtures::minimal_zip()),
        ("zero_fill_1kib", fixtures::zero_fill_1kib()),
        ("hello_world", fixtures::hello_world()),
    ];

    for (fx_label, fx_bytes) in all_fixtures {
        for lens in ALL_LENS_CONFIGS {
            for fmt in Format::ALL {
                one_cell(fx_label, fx_bytes, *lens, *fmt);
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

    // Hop 1: stdin → base60 → stdout (the dump).
    let fmt_arg = format!("--format={}", fmt_value(fmt));
    let mut args: Vec<&str> = vec!["--color=never", &fmt_arg];
    let lens_args = lens.cli_args();
    args.extend(lens_args.iter().copied());
    let dump_out = base60_cmd()
        .args(&args)
        .write_stdin(fx_bytes.to_vec())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Hop 2: dump → base60 decode → stdout (raw bytes).
    let decoded = base60_cmd()
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

fn fmt_value(f: Format) -> &'static str {
    match f {
        Format::Ansi => "ansi",
        Format::Plain => "plain",
        Format::Json => "json",
        Format::Html => "html",
    }
}
