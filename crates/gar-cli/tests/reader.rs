//! Integration tests for the `reader` module — mmap + stdin + file-open
//! error paths. Plan 04-03 (TEST-05).
//!
//! All tests spawn the `gar` binary via the shared hermetic
//! `gar_cmd()` helper (`tests/common/mod.rs`) and assert on
//! observable CLI behaviour — `reader::load` / `reader::load_file` /
//! `reader::load_stdin` stay `pub(crate)` (Phase 3 D-07 narrow-surface).
//!
//! This file is env-free — no `env::set_var` / `env::remove_var` calls,
//! no `#[serial(env)]` annotations required.

mod common;

use common::gar_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::io::Write;

#[test]
fn load_file_via_mmap_returns_file_contents() {
    // 11-byte fixture exercises load_file's mmap path via a tempfile.
    // tempfile::NamedTempFile auto-deletes on drop, cleaning up even if
    // the test panics.
    let mut tmp = tempfile::NamedTempFile::new().expect("mktemp");
    tmp.write_all(b"hello world").expect("write");
    tmp.flush().expect("flush");

    gar_cmd()
        .args(["--color=never", "--format=plain"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("|hello wo|"));
}

#[test]
fn load_stdin_via_write_stdin_dumps_piped_bytes() {
    // No file arg → load_stdin. assert_cmd's .write_stdin() feeds the
    // child's stdin, which reader::load_stdin consumes via read_to_end.
    gar_cmd()
        .args(["--color=never", "--format=plain"])
        .write_stdin(&b"piped!\n"[..])
        .assert()
        .success()
        // `\n` renders as `.` in the ASCII column; 7 bytes fit in one chunk.
        .stdout(predicates::str::contains("|piped!.|"));
}

#[test]
fn load_file_nonexistent_returns_error() {
    // reader::load_file's anyhow::Context chain produces "open <path>"
    // on File::open failure (reader.rs:52). Asserting on both "open"
    // and the bare filename covers Windows path-separator differences.
    gar_cmd()
        .args(["--color=never", "--format=plain"])
        .arg("/definitely/does/not/exist/nope.bin")
        .assert()
        .failure()
        .stderr(predicates::str::contains("open").and(predicates::str::contains("nope.bin")));
}
