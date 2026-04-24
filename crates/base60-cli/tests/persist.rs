//! Integration tests for `persist::state_base_dir` XDG → HOME → None
//! fallback ladder. Plan 04-04 (TEST-05). Every test is `#[serial(env)]`
//! because `XDG_STATE_HOME` and `HOME` are process-global.

mod common;

use common::{drive_tui_to_quit_with_fixture, fixtures};
use serial_test::serial;
use std::io::Write;

#[test]
#[serial(env)]
fn state_goes_to_xdg_when_set() {
    let tmpdir = tempfile::tempdir().expect("tempdir");
    let mut fixture = tempfile::NamedTempFile::new().expect("mktemp fixture");
    fixture
        .write_all(&fixtures::zero_fill_1kib())
        .expect("write");
    fixture.flush().expect("flush");

    // Snapshot HOME so we can restore it cleanly at end-of-test on
    // platforms that have it preset (virtually every Unix CI).
    let prev_home = std::env::var_os("HOME");

    // SAFETY: Rust 2024 env mutation is unsafe because parallel threads
    // may observe a half-updated environment. Cargo runs each `#[test]`
    // on its own thread within a single process, so env-touching tests
    // must not run concurrently. `#[serial(env)]` (Phase 2 D-07 shared
    // key) enforces that invariant.
    unsafe { std::env::set_var("XDG_STATE_HOME", tmpdir.path()) };
    // SAFETY: see above.
    unsafe { std::env::remove_var("HOME") };

    drive_tui_to_quit_with_fixture(&fixtures::zero_fill_1kib(), fixture.path());

    let state_dir = tmpdir.path().join("base60");
    assert!(state_dir.is_dir(), "state dir missing at {state_dir:?}");
    let entries: Vec<_> = std::fs::read_dir(&state_dir)
        .expect("read state dir")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(entries.len(), 1, "expected one state file under XDG path");

    // SAFETY: see above.
    unsafe { std::env::remove_var("XDG_STATE_HOME") };
    if let Some(home) = prev_home {
        // SAFETY: see above.
        unsafe { std::env::set_var("HOME", home) };
    }
}

#[test]
#[serial(env)]
fn state_falls_back_to_home_when_xdg_unset() {
    let home_tmp = tempfile::tempdir().expect("home tempdir");
    let mut fixture = tempfile::NamedTempFile::new().expect("mktemp fixture");
    fixture
        .write_all(&fixtures::zero_fill_1kib())
        .expect("write");
    fixture.flush().expect("flush");

    let prev_xdg = std::env::var_os("XDG_STATE_HOME");
    let prev_home = std::env::var_os("HOME");

    // SAFETY: Rust 2024 env mutation — see module-level safety rationale.
    unsafe { std::env::remove_var("XDG_STATE_HOME") };
    // SAFETY: see above.
    unsafe { std::env::set_var("HOME", home_tmp.path()) };

    drive_tui_to_quit_with_fixture(&fixtures::zero_fill_1kib(), fixture.path());

    let state_dir = home_tmp.path().join(".local").join("state").join("base60");
    assert!(state_dir.is_dir(), "state dir missing at {state_dir:?}");
    let entries: Vec<_> = std::fs::read_dir(&state_dir)
        .expect("read state dir")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected one state file under HOME fallback path",
    );

    // SAFETY: see above. Restore previous values.
    if let Some(xdg) = prev_xdg {
        // SAFETY: see above.
        unsafe { std::env::set_var("XDG_STATE_HOME", xdg) };
    }
    if let Some(home) = prev_home {
        // SAFETY: see above.
        unsafe { std::env::set_var("HOME", home) };
    } else {
        // SAFETY: see above.
        unsafe { std::env::remove_var("HOME") };
    }
}

#[test]
#[serial(env)]
fn state_noops_when_both_unset() {
    let mut fixture = tempfile::NamedTempFile::new().expect("mktemp fixture");
    fixture
        .write_all(&fixtures::zero_fill_1kib())
        .expect("write");
    fixture.flush().expect("flush");

    let prev_xdg = std::env::var_os("XDG_STATE_HOME");
    let prev_home = std::env::var_os("HOME");

    // SAFETY: Rust 2024 env mutation — see module-level safety rationale.
    unsafe { std::env::remove_var("XDG_STATE_HOME") };
    // SAFETY: see above.
    unsafe { std::env::remove_var("HOME") };

    // state_base_dir returns None; persist::save silently drops the
    // write. The TUI must still exit cleanly.
    drive_tui_to_quit_with_fixture(&fixtures::zero_fill_1kib(), fixture.path());

    // No assertion on filesystem — success is just "didn't panic".

    // SAFETY: see above. Restore previous values.
    if let Some(xdg) = prev_xdg {
        // SAFETY: see above.
        unsafe { std::env::set_var("XDG_STATE_HOME", xdg) };
    }
    if let Some(home) = prev_home {
        // SAFETY: see above.
        unsafe { std::env::set_var("HOME", home) };
    }
}
