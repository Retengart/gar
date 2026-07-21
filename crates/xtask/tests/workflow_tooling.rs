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
fn release_context_values_are_not_interpolated_into_shell_source() {
    let workflow = read(".github/workflows/release.yml");

    assert!(
        !workflow.contains("gh release create \"${{ github.ref_name }}\""),
        "tag names must enter the release command through an environment variable so shell syntax in a tag cannot alter the script"
    );
    assert!(
        workflow.contains("RELEASE_TAG: ${{ github.ref_name }}"),
        "the release tag must be passed to the shell as data"
    );
    assert!(
        workflow.contains("REPOSITORY: ${{ github.repository }}"),
        "the repository name must be passed to the shell as data"
    );
}

#[test]
fn windows_release_explicitly_installs_the_cross_target() {
    let workflow = read(".github/workflows/release.yml").replace("\r\n", "\n");
    let target_install = concat!(
        "- name: Ensure Rust target\n",
        "        shell: bash\n",
        "        env:\n",
        "          RUST_TARGET: ${{ matrix.target }}\n",
        "        run: rustup target add \"$RUST_TARGET\"",
    );

    assert!(
        workflow.contains(target_install),
        "install the Windows cross-target for the toolchain selected by rust-toolchain.toml, and pass the matrix value as data"
    );
}

#[test]
fn packaged_cli_requires_the_matching_core_version() {
    let workspace = read("Cargo.toml");
    let manifest = read("crates/gar-cli/Cargo.toml");
    let version = workspace
        .lines()
        .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
        .expect("workspace package version");
    let dependency = format!("gar-core = {{ version = \"{version}\", path = \"../gar-core\" }}");

    assert_eq!(
        manifest.matches(&dependency).count(),
        2,
        "the registry package must not resolve an older gar-core that lacks APIs used by the CLI"
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
