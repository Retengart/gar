//! Integration tests for the TUI exit-with-save path (`tui::run_with_terminal`).
//! Plan 04-04 (TEST-05). Every test is `#[serial(env)]` because
//! `XDG_STATE_HOME` is process-global (Phase 2 D-07, Pitfall 5).

mod common;

use common::{drive_tui_to_quit_with_fixture, fixtures};
use serial_test::serial;
use std::io::Write;

#[test]
#[serial(env)]
fn tui_quit_with_save_writes_expected_state_file() {
    // Redirect the state directory under a tempdir so the test doesn't
    // pollute the developer's real $XDG_STATE_HOME/base60/.
    let tmpdir = tempfile::tempdir().expect("tempdir");

    // Fixture: 1024 bytes of zeroes — enough for 5 j-presses worth of
    // cursor movement (5 × CHUNK = 40 bytes) to stay well inside the
    // data. Must be a real file because persist::state_file calls
    // `fs::canonicalize` on the input path.
    let mut fixture = tempfile::NamedTempFile::new().expect("mktemp fixture");
    fixture
        .write_all(&fixtures::zero_fill_1kib())
        .expect("write fixture");
    fixture.flush().expect("flush fixture");
    let fixture_bytes = fixtures::zero_fill_1kib();

    // SAFETY: Rust 2024 marks `env::set_var` unsafe because parallel
    // threads may observe a half-updated environment. Cargo runs each
    // `#[test]` on its own thread but within the same process, so tests
    // touching env vars must not run concurrently. `#[serial(env)]`
    // (shared key, Phase 2 D-07) enforces that invariant.
    unsafe { std::env::set_var("XDG_STATE_HOME", tmpdir.path()) };

    drive_tui_to_quit_with_fixture(&fixture_bytes, fixture.path());

    // State file: persist writes to `$XDG_STATE_HOME/base60/<hash>.state`.
    // Rather than recomputing the FNV-1a of the canonical path (which
    // varies on macOS due to /private/tmp symlinks — Pitfall 6), we
    // glob the directory for the single file that appears.
    let state_dir = tmpdir.path().join("base60");
    let entries: Vec<_> = std::fs::read_dir(&state_dir)
        .expect("state dir exists")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one state file under {state_dir:?}",
    );
    let contents = std::fs::read_to_string(entries[0].path()).expect("read state file");

    // 5 × CHUNK=8 = cursor offset 40. Bookmark slot 'a' captures the
    // cursor at slot-set time, so bookmarks=a:40. (scroll=0 because
    // offset 40 fits on-screen at 80×24.)
    assert!(
        contents.contains("cursor=40"),
        "state file missing cursor=40; got: {contents:?}",
    );
    assert!(
        contents.contains("bookmarks=a:40"),
        "state file missing bookmarks=a:40; got: {contents:?}",
    );

    // SAFETY: see above — cleanup under the same `#[serial(env)]` scope.
    unsafe { std::env::remove_var("XDG_STATE_HOME") };
}
