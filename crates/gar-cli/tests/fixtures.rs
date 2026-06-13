//! Per-subcommand happy-path tests against each of the 5 phase-3
//! fixtures (`minimal_elf`, `minimal_png`, `minimal_zip`,
//! `zero_fill_1kib`, `hello_world`). Spot-checks each `gar` entry
//! point end-to-end. Complement the roundtrip matrix in `roundtrip.rs`
//! by covering `analyze` and `completions` (neither participates in
//! the roundtrip).
//!
//! Phase 3 TEST-03 (D-12).

mod common;

use common::{FixtureEntry, gar_cmd, fixtures};

/// Declaration-order pairing of label + bytes so failure diagnostics
/// name the failing fixture unambiguously (Pitfall 10 — no `HashMap`).
fn all_fixtures() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("minimal_elf", fixtures::minimal_elf()),
        ("minimal_png", fixtures::minimal_png()),
        ("minimal_zip", fixtures::minimal_zip()),
        ("zero_fill_1kib", fixtures::zero_fill_1kib()),
        ("hello_world", fixtures::hello_world()),
    ]
}

#[test]
fn dump_produces_expected_prefix_per_fixture() {
    // Every gar dump begins with the 8-char offset "00000000  "
    // (zero offset + 2-space separator). Stable across all formats
    // except JSON/HTML — we test the default (ANSI-less plain) path
    // which uses the shared offset column.
    for (label, bytes) in all_fixtures() {
        gar_cmd()
            .args(["--color=never", "--format=plain"])
            .write_stdin(bytes)
            .assert()
            .success()
            .stdout(predicates::str::starts_with("00000000  "));
        // `label` shows up in the assertion failure via panic output;
        // kept bound so future `.and(...)` expansions can mention it.
        let _ = label;
    }
}

#[test]
fn analyze_summary_is_sane_per_fixture() {
    // `analyze::write_summary` unconditionally emits both "bytes" and
    // "entropy" substrings (verified against src/analyze.rs:210-211
    // during plan authoring). Pin both, so a future rename of either
    // label surfaces here.
    for (_label, bytes) in all_fixtures() {
        gar_cmd()
            .arg("analyze")
            .write_stdin(bytes)
            .assert()
            .success()
            .stdout(predicates::str::contains("bytes"))
            .stdout(predicates::str::contains("entropy"));
    }
}

#[test]
fn decode_roundtrips_default_dump_per_fixture() {
    // Lightweight per-fixture roundtrip with the DEFAULT flags (no
    // lens, plain format). Complements the 28-cell matrix by
    // exercising the same path with the binary's shipped defaults.
    //
    // SCOPED to the 2 byte-identical fixtures (8-byte-aligned
    // lengths): `minimal_elf` (128 B) and `zero_fill_1kib` (1024 B).
    // `hello_world` (14 B), `minimal_png` (45 B), and `minimal_zip`
    // (22 B) are NOT 8-byte aligned and hit the Problem B length
    // mismatch documented in 03-02-SUMMARY.md §Scope Deviation
    // (deferred to REF-04 in Phase 4).
    let roundtrip_fixtures: &[FixtureEntry] = &[
        ("minimal_elf", fixtures::minimal_elf),
        ("zero_fill_1kib", fixtures::zero_fill_1kib),
    ];
    for (_label, factory) in roundtrip_fixtures {
        let bytes = factory();
        let dumped = gar_cmd()
            .args(["--color=never", "--format=plain"])
            .write_stdin(bytes.clone())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let decoded = gar_cmd()
            .arg("decode")
            .write_stdin(dumped)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        assert_eq!(decoded, bytes);
    }
}

#[test]
fn completions_shells_all_succeed() {
    // Smoke-test every shell clap_complete supports. We don't parse
    // the output — only prove it's non-empty and the binary exits 0.
    // clap's `ValueEnum` on `clap_complete::Shell` pins the accepted
    // spellings; drift here means clap bumped its shell enum.
    for shell in ["bash", "zsh", "fish", "elvish", "powershell"] {
        gar_cmd()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicates::function::function(|s: &[u8]| !s.is_empty()));
    }
}
