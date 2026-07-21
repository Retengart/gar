//! Edge-case integration tests for the `gar` CLI:
//! - stdin piping into `dump`
//! - `BrokenPipe` exit-0 contract on `dump`
//! - `NO_COLOR` env + `--color={auto,always,never}` precedence
//! - `--skip` / `--length` clamping
//! - Decoder error-message pin — full-message + position-pinning
//!   (Pitfall 8 remediation; Phase 4 Plan 04-02 D-10/D-11 tightens
//!   the loose `"99" + "invalid"` assertion inherited from Phase 3).
//!
//! Phase 3 TEST-03 (D-13); Phase 4 REF-03 (Plan 04-02).

mod common;

use common::{fixtures, gar_cmd, spawn_with_closed_stdout};
use predicates::prelude::PredicateBooleanExt;
use tempfile::NamedTempFile;

fn temp_file(bytes: &[u8]) -> NamedTempFile {
    let file = NamedTempFile::new().expect("create temporary input");
    std::fs::write(file.path(), bytes).expect("write temporary input");
    file
}

// One MiB expands to more than 10 MiB of formatted output. That exceeds
// anonymous-pipe buffers on every CI platform, so closing the read end forces
// BrokenPipe even when the child is scheduled before the parent resumes.
const BROKEN_PIPE_INPUT_BYTES: usize = 1024 * 1024;

// ---------------------------------------------------------------------
// Binary diff: stable markers, offsets, colour policy, and exit codes.
// ---------------------------------------------------------------------

#[test]
fn diff_identical_files_exits_zero_and_marks_equal_rows() {
    let old = temp_file(b"same bytes");
    let new = temp_file(b"same bytes");

    gar_cmd()
        .arg("diff")
        .args([old.path(), new.path()])
        .assert()
        .code(0)
        .stdout(predicates::str::contains("00000000 ="));
}

#[test]
fn diff_changed_files_exits_one_with_plain_side_by_side_output() {
    let old = temp_file(b"abcdefgh");
    let new = temp_file(b"abcdEfgh");

    gar_cmd()
        .arg("diff")
        .arg("--color=never")
        .args([old.path(), new.path()])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("00000000 !"))
        .stdout(predicates::str::contains("abcdefgh"))
        .stdout(predicates::str::contains("abcdEfgh"))
        .stdout(predicates::str::contains("\x1b[").not());
}

#[test]
fn diff_unequal_files_distinguishes_left_and_right_only_rows() {
    let short = temp_file(b"12345678");
    let long = temp_file(b"12345678abcdefgh");

    gar_cmd()
        .arg("diff")
        .args([short.path(), long.path()])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("00000008 >"));

    gar_cmd()
        .arg("diff")
        .args([long.path(), short.path()])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("00000008 <"));
}

#[test]
fn diff_color_always_emphasizes_changes() {
    let old = temp_file(b"abcdefgh");
    let new = temp_file(b"abcdEfgh");

    gar_cmd()
        .arg("diff")
        .arg("--color=always")
        .args([old.path(), new.path()])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("\x1b["));
}

#[test]
fn diff_missing_file_exits_two_and_names_the_path() {
    let old = temp_file(b"left");
    let missing = old.path().with_extension("missing");

    gar_cmd()
        .arg("diff")
        .args([old.path(), missing.as_path()])
        .assert()
        .code(2)
        .stderr(predicates::str::contains(
            missing.to_string_lossy().as_ref(),
        ));
}

#[test]
fn diff_exits_zero_on_broken_pipe() {
    let mut changed = vec![0_u8; BROKEN_PIPE_INPUT_BYTES];
    let old = temp_file(&changed);
    changed[0] = 1;
    let new = temp_file(&changed);
    let old_path = old.path().to_str().expect("UTF-8 temporary path");
    let new_path = new.path().to_str().expect("UTF-8 temporary path");

    let status = spawn_with_closed_stdout(&["diff", old_path, new_path], &[]);
    assert!(
        status.success(),
        "gar diff must exit 0 on BrokenPipe, got {status:?}",
    );
}

// ---------------------------------------------------------------------
// Analyze pattern reporting: same grammar as TUI search, absolute range
// offsets, and bounded output for repetitive inputs.
// ---------------------------------------------------------------------

#[test]
fn analyze_pattern_combines_range_and_absolute_match_offsets() {
    let file = temp_file(b"zzABxxAByy");

    gar_cmd()
        .arg("analyze")
        .args(["--skip", "3", "--length", "5", "--pattern", "hex:4142"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("bytes         5"))
        .stdout(predicates::str::contains("matches       1"))
        .stdout(predicates::str::contains("0x00000003..0x00000008"))
        .stdout(predicates::str::contains("0x00000006"));
}

#[test]
fn analyze_pattern_accepts_string_grammar() {
    let file = temp_file(b"deadbeef deadbeef");

    gar_cmd()
        .arg("analyze")
        .args(["--pattern", "str:deadbeef"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("matches       2"));
}

#[test]
fn analyze_pattern_rejects_invalid_explicit_hex() {
    let file = temp_file(b"anything");

    gar_cmd()
        .arg("analyze")
        .args(["--pattern", "hex:abc"])
        .arg(file.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("invalid pattern"));
}

#[test]
fn analyze_pattern_limits_offsets_and_reports_omitted_matches() {
    let file = temp_file(&b"AB".repeat(25));

    gar_cmd()
        .arg("analyze")
        .args(["--pattern", "str:AB"])
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("matches       25"))
        .stdout(predicates::str::contains("... 5 more"));
}

// ---------------------------------------------------------------------
// Stdin piping
// ---------------------------------------------------------------------

#[test]
fn stdin_piped_dump_produces_output() {
    // `assert_cmd::Command::write_stdin` writes the fixture bytes to
    // the child stdin and closes it. Any non-empty dump stdout proves
    // the stdin path wires through to the renderer.
    gar_cmd()
        .args(["--color=never", "--format=plain"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::is_empty().not());
}

// ---------------------------------------------------------------------
// BrokenPipe: `gar dump` must exit 0 when the stdout pipe closes
// mid-write. Exercised by dropping the child's stdout handle right
// after spawn so the first write hits EPIPE. (Uses the one helper in
// `tests/common/mod.rs` that is permitted to spawn raw.)
// ---------------------------------------------------------------------

#[test]
fn dump_exits_zero_on_broken_pipe() {
    // The helper closes stdout before feeding stdin, so the child cannot write
    // any output until the read end is already gone.
    let status = spawn_with_closed_stdout(
        &["--color=never", "--format=plain"],
        &fixtures::zero_fill_1kib(),
    );
    assert!(
        status.success(),
        "gar dump must exit 0 on BrokenPipe, got {status:?}",
    );
}

// ---------------------------------------------------------------------
// Color precedence — three-way matrix of env + flag.
// ---------------------------------------------------------------------

#[test]
fn no_color_env_suppresses_ansi_on_auto() {
    // NO_COLOR=1 with --color=auto → no ANSI escape sequences in stdout.
    // `.env(...)` mutates only the child process — `env_clear()` in
    // `gar_cmd()` already stripped NO_COLOR, so we add it back
    // explicitly for this case.
    gar_cmd()
        .env("NO_COLOR", "1")
        .args(["--color=auto", "--format=ansi"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::contains("\x1b[").not());
}

#[test]
fn color_always_forces_ansi_even_in_pipe() {
    // `--color=always` overrides TTY-detection. assert_cmd captures
    // stdout via a pipe (not a TTY), so without `--color=always` the
    // auto path would yield no ANSI. Confirm the flag forces escapes.
    gar_cmd()
        .args(["--color=always", "--format=ansi"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::contains("\x1b["));
}

#[test]
fn color_never_suppresses_ansi_with_clicolor_force() {
    // `--color=never` wins over `CLICOLOR_FORCE=1` (and anything else
    // a hostile env might inject). Pins the "never really means never"
    // contract documented in PROJECT.md Constraints.
    gar_cmd()
        .env("CLICOLOR_FORCE", "1")
        .args(["--color=never", "--format=ansi"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::contains("\x1b[").not());
}

// ---------------------------------------------------------------------
// --skip / --length clamping: saturating paths per reader::clamp_range.
// ---------------------------------------------------------------------

#[test]
fn skip_past_end_yields_empty_dump() {
    // 14-byte hello_world with --skip=1024 → zero bytes surface → the
    // dump body is empty (or at most a trailing newline). The read
    // path saturates rather than errors; the binary exits 0.
    gar_cmd()
        .args(["--color=never", "--format=plain", "--skip=1024"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success();
}

#[test]
fn length_clamps_to_available_bytes() {
    // --length=9999 on a 14-byte input must clamp to 14 without error.
    // The first dump line still begins at offset 0.
    gar_cmd()
        .args(["--color=never", "--format=plain", "--length=9999"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::starts_with("00000000  "));
}

#[test]
fn zero_skip_is_identity() {
    // --skip=0 is the default; this test pins the behaviour by
    // asserting the first line begins at offset 0 (not 8, 16, etc.)
    // and no error surfaces. Cheap; catches a future off-by-one.
    gar_cmd()
        .args(["--color=never", "--format=plain", "--skip=0"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::starts_with("00000000  "));
}

// ---------------------------------------------------------------------
// Decoder error-message pin (Pitfall 8 / D-10 / D-11).
//
// Plan 04-02 tightens the loose `"99" + "invalid"` pin from Phase 3 to
// a FULL-MESSAGE contains on the literal error format, locking the
// line-number + pair-position + digit-value + exact phrasing in one
// assertion. The three follow-up tests prove the pair index advances
// correctly (pair 1, pair 5) and that non-digit-run lines stay silent.
// ---------------------------------------------------------------------

#[test]
fn decoder_invalid_digit_99_error_contains_the_digit() {
    // Last pair `99` decodes to hi=9, lo=9 → digit = 99 (≥ 60) →
    // `InvalidData`. A valid pair is `00..59`, so 11 leading `00`
    // pairs plus the `99` tail hit exactly one digit-run boundary.
    let dump = "00000000  00:00:00:00:00:00:00:00:00:00:99  |........|\n";
    gar_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "line 1: invalid base-60 digit 99 at pair 11",
        ));
}

#[test]
fn decoder_invalid_digit_at_pair_1_reports_pair_1() {
    // FIRST pair is 99 (hi=9, lo=9 → 99 ≥ 60). `parse_run` returns on
    // the first invalid digit, so the error must report pair 1.
    let dump = "00000000  99:00:00:00:00:00:00:00:00:00:00  |........|\n";
    gar_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "line 1: invalid base-60 digit 99 at pair 1",
        ));
}

#[test]
fn decoder_invalid_digit_at_pair_5_reports_pair_5() {
    // FIFTH pair is 99; pairs 1–4 are valid `00`, so `parse_run`
    // advances to pair 5 before raising.
    let dump = "00000000  00:00:00:00:99:00:00:00:00:00:00  |........|\n";
    gar_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "line 1: invalid base-60 digit 99 at pair 5",
        ));
}

#[test]
fn decoder_ignores_non_digit_run_lines() {
    // Free-form text without an 11-pair run is skipped silently by
    // `find_digit_run` (D-11). If REF-03 accidentally collapsed
    // "no run found" into an error, this test fails immediately.
    let garbage = "some prefix\n# bytes=0x10\nhello world\n\n";
    gar_cmd()
        .arg("decode")
        .write_stdin(garbage)
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}

// ---------------------------------------------------------------------
// REF-04 / Plan 04-01 — decode input-format override + legacy warning.
// ---------------------------------------------------------------------

#[test]
fn decode_legacy_no_trailer_warns_and_continues() {
    // A dump without the `# bytes=0x<hex>` trailer must still decode (D-03)
    // and print a single stderr warning containing "no length metadata".
    let dump = "00000000  00:00:00:00:00:00:00:00:00:00:00  |........|\n";
    gar_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .success()
        .stderr(predicates::str::contains("no length metadata"));
}

#[test]
fn decode_input_format_flag_is_advertised_in_help() {
    gar_cmd()
        .arg("decode")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("--input-format"));
}

#[test]
fn decode_input_format_override_forces_json() {
    // Single chunk encoding "Hi" with a meta trailer pinning total = 2.
    let ndjson = "\
{\"offset\":0,\"bytes\":[72,105],\"digits\":[0,0,0,0,0,0,0,0,0,30,9],\"ascii\":\"Hi\"}
{\"type\":\"meta\",\"bytes\":2}
";
    gar_cmd()
        .arg("decode")
        .args(["--input-format=json"])
        .write_stdin(ndjson)
        .assert()
        .success()
        .stdout(predicates::ord::eq(b"Hi".as_slice()));
}

#[test]
fn decode_input_format_override_forces_html() {
    // Minimal HTML with one row decoding to 0x0000_0000_0000_0001
    // (11 digit pairs, final one `01`). Trailer clamps to 8 bytes.
    let html = "<!doctype html><html><body><pre>\
<span class=\"offset\">00000000</span>  \
<span class=\"d-zero\">00</span><span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span><span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span><span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span><span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span><span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span><span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span><span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span><span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span><span class=\"sep\">:</span>\
<span class=\"d-zero\">00</span><span class=\"sep\">:</span>\
<span class=\"d-low\">01</span>\
\n<!-- bytes=0x8 --></pre></body></html>\n";
    gar_cmd()
        .arg("decode")
        .args(["--input-format=html"])
        .write_stdin(html)
        .assert()
        .success()
        .stdout(predicates::ord::eq([0_u8, 0, 0, 0, 0, 0, 0, 1].as_slice()));
}
