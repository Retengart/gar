//! Edge-case integration tests for the `base60` CLI:
//! - stdin piping into `dump`
//! - `BrokenPipe` exit-0 contract on `dump`
//! - `NO_COLOR` env + `--color={auto,always,never}` precedence
//! - `--skip` / `--length` clamping
//! - Decoder error-message pin (Pitfall 8 — locks the format so
//!   Phase 4's REF-03 refactor cannot silently drift it).
//!
//! Phase 3 TEST-03 (D-13).

mod common;

use common::{base60_cmd, fixtures, spawn_with_closed_stdout};
use predicates::prelude::PredicateBooleanExt;

// ---------------------------------------------------------------------
// Stdin piping
// ---------------------------------------------------------------------

#[test]
fn stdin_piped_dump_produces_output() {
    // `assert_cmd::Command::write_stdin` writes the fixture bytes to
    // the child stdin and closes it. Any non-empty dump stdout proves
    // the stdin path wires through to the renderer.
    base60_cmd()
        .args(["--color=never", "--format=plain"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::is_empty().not());
}

// ---------------------------------------------------------------------
// BrokenPipe: `base60 dump` must exit 0 when the stdout pipe closes
// mid-write. Exercised by dropping the child's stdout handle right
// after spawn so the first write hits EPIPE. (Uses the one helper in
// `tests/common/mod.rs` that is permitted to spawn raw.)
// ---------------------------------------------------------------------

#[test]
fn dump_exits_zero_on_broken_pipe() {
    // 1 KiB of zero fill → ~128 dump lines, enough to saturate any
    // reasonable pipe buffer before the child finishes writing. The
    // child's BrokenPipe handler in `lib.rs::run_view` must absorb
    // the error and yield exit status 0.
    let status = spawn_with_closed_stdout(
        &["--color=never", "--format=plain"],
        &fixtures::zero_fill_1kib(),
    );
    assert!(
        status.success(),
        "base60 dump must exit 0 on BrokenPipe, got {status:?}",
    );
}

// ---------------------------------------------------------------------
// Color precedence — three-way matrix of env + flag.
// ---------------------------------------------------------------------

#[test]
fn no_color_env_suppresses_ansi_on_auto() {
    // NO_COLOR=1 with --color=auto → no ANSI escape sequences in stdout.
    // `.env(...)` mutates only the child process — `env_clear()` in
    // `base60_cmd()` already stripped NO_COLOR, so we add it back
    // explicitly for this case.
    base60_cmd()
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
    base60_cmd()
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
    base60_cmd()
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
    base60_cmd()
        .args(["--color=never", "--format=plain", "--skip=1024"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success();
}

#[test]
fn length_clamps_to_available_bytes() {
    // --length=9999 on a 14-byte input must clamp to 14 without error.
    // The first dump line still begins at offset 0.
    base60_cmd()
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
    base60_cmd()
        .args(["--color=never", "--format=plain", "--skip=0"])
        .write_stdin(fixtures::hello_world())
        .assert()
        .success()
        .stdout(predicates::str::starts_with("00000000  "));
}

// ---------------------------------------------------------------------
// Decoder error-message pin (Pitfall 8 / D-13).
//
// Phase 4 (REF-03) will tighten `decode::parse_run`'s signature. If
// the refactor silently changes the error `format!` string, this test
// fails with a clear diagnostic pointing at the semantic drift. The
// assertion requires BOTH "99" (the offending digit) AND "invalid"
// (the human-readable category). Two substrings is the sweet spot:
// tight enough to catch drift, loose enough to tolerate harmless
// reword (e.g., changing "at pair N" phrasing).
// ---------------------------------------------------------------------

#[test]
fn decoder_invalid_digit_99_error_contains_the_digit() {
    // Last pair `99` decodes to hi=9, lo=9 → digit = 99 (≥ 60) →
    // `InvalidData`. A valid pair is `00..59`, so 11 leading `00`
    // pairs plus the `99` tail hit exactly one digit-run boundary.
    let dump = "00000000  00:00:00:00:00:00:00:00:00:00:99  |........|\n";
    base60_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .failure()
        .stderr(predicates::str::contains("99").and(predicates::str::contains("invalid")));
}

// ---------------------------------------------------------------------
// REF-04 / Plan 04-01 — decode input-format override + legacy warning.
// ---------------------------------------------------------------------

#[test]
fn decode_legacy_no_trailer_warns_and_continues() {
    // A dump without the `# bytes=0x<hex>` trailer must still decode (D-03)
    // and print a single stderr warning containing "no length metadata".
    let dump = "00000000  00:00:00:00:00:00:00:00:00:00:00  |........|\n";
    base60_cmd()
        .arg("decode")
        .write_stdin(dump)
        .assert()
        .success()
        .stderr(predicates::str::contains("no length metadata"));
}

#[test]
fn decode_input_format_flag_is_advertised_in_help() {
    base60_cmd()
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
    base60_cmd()
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
    base60_cmd()
        .arg("decode")
        .args(["--input-format=html"])
        .write_stdin(html)
        .assert()
        .success()
        .stdout(predicates::ord::eq([0_u8, 0, 0, 0, 0, 0, 0, 1].as_slice()));
}
