//! Spawn-discipline gate: every `Command::cargo_bin` invocation in
//! `crates/base60-cli/tests/**/*.rs` must live under `tests/common/`.
//! All other integration tests spawn the binary exclusively through the
//! `base60_cmd()` helper, giving one enforcement point for
//! `.env_clear()` + env-restore invariants. Phase 3 (TEST-03) invariant.
//!
//! Line-based scanner — no `syn`, no regex. Forks the shape of
//! `env_discipline.rs` (Phase 2 TEST-04): walkdir over the crate
//! tests root, skip `//`-comments, skip files whose path contains a
//! `common` component, flag every remaining line matching the literal
//! `Command::cargo_bin`.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Relative root from this crate's manifest to walk.
const WALK_ROOT: &str = "../base60-cli/tests";

/// Path-component signalling "this file may legitimately spawn the binary".
const EXEMPT_DIR: &str = "common";

/// Literal substring flagged by the gate.
const SPAWN_LITERAL: &str = "Command::cargo_bin";

#[test]
fn no_raw_spawn_outside_common() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let root_path: PathBuf = Path::new(manifest_dir).join(WALK_ROOT);

    // Phase 3 ships `tests/` in the same commit as this gate, but we
    // keep the no-op branch so the gate is safe in any future state
    // where `tests/` might be removed (e.g. test directory renames).
    if !root_path.is_dir() {
        return;
    }

    let mut failures: Vec<String> = Vec::new();

    for entry in WalkDir::new(&root_path).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().is_none_or(|e| e != "rs") {
            continue;
        }
        if entry
            .path()
            .components()
            .any(|c| c.as_os_str() == EXEMPT_DIR)
        {
            continue;
        }

        let path = entry.path();
        let contents = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

        for (idx, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            if !line.contains(SPAWN_LITERAL) {
                continue;
            }
            let rel = path
                .strip_prefix(manifest_dir)
                .unwrap_or(path)
                .display()
                .to_string();
            failures.push(format!(
                "{rel}:{lno}: raw Command::cargo_bin outside tests/common/ \
                 — use base60_cmd() from tests/common/mod.rs",
                lno = idx + 1,
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "spawn-discipline gate failed ({count} issue(s)):\n{details}",
        count = failures.len(),
        details = failures.join("\n"),
    );
}
