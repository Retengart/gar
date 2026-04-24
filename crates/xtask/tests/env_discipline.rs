//! Env-discipline gate: every `env::set_var` / `env::remove_var` call in
//! `base60-core/src/**/*.rs` and `base60-cli/src/**/*.rs` must live inside a
//! test function bearing `#[serial(env)]` — no alternate keys, no production
//! code exceptions. Phase 2 (TEST-04) invariant.
//!
//! Walks both crate sources via `walkdir`. Line-based parser: for each
//! `env::set_var` / `env::remove_var` occurrence, walks upward to find the
//! enclosing `fn`, then confirms the preceding attribute block contains
//! exactly `#[serial(env)]` (no `#[serial(no_color)]` etc.) AND the function
//! also bears `#[test]`. Any deviation fails the test with a precise
//! `file:line` diagnostic.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Relative roots from this crate's manifest to walk.
const WALK_ROOTS: &[&str] = &["../base60-core/src", "../base60-cli/src"];

/// Attribute key shapes that are explicitly rejected (Phase 2 D-13).
const FORBIDDEN_SERIAL_KEYS: &[&str] = &[
    "#[serial(no_color)]",
    "#[serial(no_unicode)]",
    "#[serial(term)]",
];

#[test]
fn every_env_mutation_is_serialised() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut failures: Vec<String> = Vec::new();

    for root in WALK_ROOTS {
        let root_path: PathBuf = Path::new(manifest_dir).join(root);
        assert!(
            root_path.is_dir(),
            "walk root does not exist: {}",
            root_path.display()
        );

        for entry in WalkDir::new(&root_path).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().is_none_or(|e| e != "rs") {
                continue;
            }

            let path = entry.path();
            let contents = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let lines: Vec<&str> = contents.lines().collect();

            for (idx, line) in lines.iter().enumerate() {
                // Skip commented lines to avoid false positives on `SAFETY:`
                // comments that mention `env::set_var` for documentation.
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }

                let mentions_mutation =
                    line.contains("env::set_var(") || line.contains("env::remove_var(");
                if !mentions_mutation {
                    continue;
                }

                let line_no = idx + 1;
                let rel = path
                    .strip_prefix(manifest_dir)
                    .unwrap_or(path)
                    .display()
                    .to_string();

                // Walk upward to the enclosing `fn` declaration.
                let Some(fn_idx) = find_enclosing_fn(&lines, idx) else {
                    failures.push(format!(
                        "{rel}:{line_no}: env mutation has no enclosing `fn` — \
                         env-discipline requires this to be inside a \
                         `#[serial(env)]` test"
                    ));
                    continue;
                };

                // Scan attributes immediately above the fn declaration.
                let attrs = collect_attributes_above(&lines, fn_idx);

                let has_test = attrs.iter().any(|a| a.trim() == "#[test]");
                let has_serial_env = attrs.iter().any(|a| a.trim() == "#[serial(env)]");
                let forbidden: Vec<&str> = attrs
                    .iter()
                    .map(String::as_str)
                    .map(str::trim)
                    .filter(|a| FORBIDDEN_SERIAL_KEYS.iter().any(|forbidden| a == forbidden))
                    .collect();

                if !has_test {
                    failures.push(format!(
                        "{rel}:{line_no}: env mutation in non-`#[test]` \
                         function — env-discipline forbids env mutation \
                         outside tests"
                    ));
                }
                if !forbidden.is_empty() {
                    failures.push(format!(
                        "{rel}:{line_no}: found forbidden serial_test key \
                         {forbidden:?}; use only `#[serial(env)]` (shared key)"
                    ));
                }
                if !has_serial_env {
                    failures.push(format!(
                        "{rel}:{line_no}: env mutation missing \
                         `#[serial(env)]` attribute — add \
                         `#[serial(env)]` above the enclosing `fn`"
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "env-discipline gate failed ({count} issue(s)):\n{details}",
        count = failures.len(),
        details = failures.join("\n"),
    );
}

/// Walks backwards from `line_idx` to find the first line whose trimmed
/// prefix begins with `fn `, `pub fn `, `pub(crate) fn `, `pub(super) fn `,
/// `async fn ` or `const fn `. Returns the 0-based line index of that `fn`,
/// or `None` if no such line exists above.
fn find_enclosing_fn(lines: &[&str], line_idx: usize) -> Option<usize> {
    for i in (0..=line_idx).rev() {
        let t = lines[i].trim_start();
        if t.starts_with("fn ")
            || t.starts_with("pub fn ")
            || t.starts_with("pub(crate) fn ")
            || t.starts_with("pub(super) fn ")
            || t.starts_with("async fn ")
            || t.starts_with("const fn ")
        {
            return Some(i);
        }
    }
    None
}

/// Collects the contiguous block of attribute lines (`#[...]`) immediately
/// preceding `fn_idx`. Stops at the first non-attribute, non-blank line.
fn collect_attributes_above(lines: &[&str], fn_idx: usize) -> Vec<String> {
    let mut out = Vec::new();
    if fn_idx == 0 {
        return out;
    }
    for i in (0..fn_idx).rev() {
        let t = lines[i].trim_start();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("#[") {
            out.push(lines[i].to_string());
            continue;
        }
        // First non-attribute, non-blank line: stop.
        break;
    }
    out
}
