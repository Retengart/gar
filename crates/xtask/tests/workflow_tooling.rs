//! CI tooling gate: workflows must use immutable action revisions and install
//! cargo-fuzz as a Cargo subcommand, while the declared Rust toolchain stays in
//! sync across local and CI entry points.

use std::path::{Path, PathBuf};

const WORKFLOWS: &[&str] = &["ci.yml", "fuzz.yml", "release.yml"];
const CARGO_DENY_ACTION: &str =
    "EmbarkStudios/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25";
const CARGO_FUZZ_INSTALL: &str = "cargo install cargo-fuzz --version 0.13.2 --locked";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[test]
fn workflow_actions_use_immutable_revisions() {
    let mut failures = Vec::new();

    for workflow in WORKFLOWS {
        let relative = format!(".github/workflows/{workflow}");
        for (index, line) in read(&relative).lines().enumerate() {
            let trimmed = line.trim();
            let Some(spec) = trimmed
                .strip_prefix("- uses: ")
                .or_else(|| trimmed.strip_prefix("uses: "))
            else {
                continue;
            };
            let Some((_, revision)) = spec.split_once('@') else {
                failures.push(format!("{relative}:{}: action has no revision", index + 1));
                continue;
            };
            let revision = revision.split_whitespace().next().unwrap_or_default();
            if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                failures.push(format!(
                    "{relative}:{}: `{spec}` is not pinned to a full commit SHA",
                    index + 1
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "workflow action pin gate failed:\n{}",
        failures.join("\n")
    );
}

#[test]
fn dependency_policy_uses_compatible_cargo_deny() {
    let workflow = read(".github/workflows/ci.yml");
    assert!(
        workflow.contains(CARGO_DENY_ACTION),
        "cargo-deny action must be pinned to v2.1.1, whose cargo-deny 0.20.2 accepts the current deny.toml"
    );
    assert!(
        workflow.contains("manifest-path: fuzz/Cargo.toml"),
        "CI must audit the independent fuzz workspace as well as the main workspace"
    );
}

#[test]
fn fuzz_dependencies_have_an_explicit_license_policy() {
    let manifest = read("fuzz/Cargo.toml");
    let policy = read("deny.toml");

    assert!(
        manifest.contains("license = \"MIT OR Apache-2.0\""),
        "the independent fuzz package must declare the repository license"
    );
    assert!(
        policy.contains("\"NCSA\""),
        "libfuzzer-sys includes the OSI-approved NCSA license"
    );
}

#[test]
fn fuzz_workflow_installs_cargo_fuzz_as_a_cargo_subcommand() {
    let workflow = read(".github/workflows/fuzz.yml");
    assert!(
        !workflow.contains("components: cargo-fuzz"),
        "cargo-fuzz is not a rustup component"
    );
    assert!(
        workflow.contains(CARGO_FUZZ_INSTALL),
        "fuzz workflow must install the audited cargo-fuzz version explicitly"
    );
}

#[test]
fn msrv_matches_the_pinned_toolchain_and_ci_matrix() {
    let manifest = read("Cargo.toml");
    let toolchain = read("rust-toolchain.toml");
    let workflow = read(".github/workflows/ci.yml");

    assert!(manifest.contains("rust-version = \"1.97.1\""));
    assert!(toolchain.contains("channel = \"1.97.1\""));
    assert!(workflow.contains("rust: ['1.97.1', stable, beta]"));
}
