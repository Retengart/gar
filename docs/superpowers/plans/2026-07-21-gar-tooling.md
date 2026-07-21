# gar Tooling and CI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make contributor workflow, Rust version, dependency policy, and common commands explicit and enforced for P13-P16.

**Architecture:** Keep CI jobs independent, pin local Rust to the manifest MSRV, and make `just` a thin command index over authoritative Cargo commands. Documentation links to existing architecture material instead of copying it.

**Tech Stack:** Rustup toolchain files, cargo-deny Action v1, just, GitHub Actions, Markdown, GitButler CLI.

## Global Constraints

- All work stays on branch `fix/all-gar-improvements`.
- MSRV is exactly Rust 1.95.
- `verify` includes every required local gate and stops on the first failure.
- No generated artifacts or user-owned `research/` files are committed.

---

### Task 1: Pin and verify the declared MSRV (P14)

**Files:**
- Create: `rust-toolchain.toml`
- Modify: `.github/workflows/ci.yml`
- Modify: `CLAUDE.md`

**Interfaces:**
- Pins: channel `1.95.0`, components `rustfmt` and `clippy`.

- [ ] **Step 1: Add the toolchain file**

```toml
[toolchain]
channel = "1.95.0"
components = ["clippy", "rustfmt"]
profile = "minimal"
```

- [ ] **Step 2: Align CI and docs**

Replace only the CI matrix entry `'1.96.0'` with `'1.95.0'` and the stale
CLAUDE matrix text with `1.95/stable/beta`.

- [ ] **Step 3: Verify the pin**

Run:

```text
rustc --version
cargo check --workspace --all-targets --locked --message-format=json
```

Expected: rustc reports 1.95.0 and check exits 0. If current dependencies no
longer support the declared MSRV, fix the dependency resolution rather than
silently raising MSRV.

- [ ] **Step 4: Commit**

Commit the three files with `but` as `build: pin rust 1.95 toolchain`.

### Task 2: Add contributor guide and just recipes (P13, P16)

**Files:**
- Create: `CONTRIBUTING.md`
- Create: `justfile`
- Modify: `README.md`

**Interfaces:**
- Adds recipes: default, check, test, lint, doc, deny, fuzz, verify.

- [ ] **Step 1: Create the contributor contract**

Document prerequisites, `CLAUDE.md` architecture link, zero-dependency core,
test-first changes, output compatibility, required checks, conventional commit
style, and focused PRs.

- [ ] **Step 2: Create exact just recipes**

```just
[default]
default:
  @just --list

check:
  cargo check --workspace --all-targets --locked --message-format=json

test:
  cargo test --workspace --all-targets --locked

lint:
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --locked -- -D warnings

doc:
  RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

deny:
  cargo deny check

fuzz target="decode_stream":
  cargo +nightly fuzz run {{target}} -- -max_total_time=60

verify: check test lint doc deny
```

Add a README contributor link and one-line `just verify` usage.

- [ ] **Step 3: Verify recipes**

Run `just --list`, `just check`, and `just --dry-run verify`.

Expected: every documented recipe is listed; command expansion matches CI.

- [ ] **Step 4: Commit**

Commit with `but` as `docs: add contributor workflow`.

### Task 3: Enforce cargo-deny in CI (P15)

**Files:**
- Modify: `.github/workflows/ci.yml`
- Verify: `deny.toml`

**Interfaces:**
- Adds independent `cargo-deny` job using `EmbarkStudios/cargo-deny-action@v1`.

- [ ] **Step 1: Add the dedicated job**

```yaml
cargo-deny:
  name: cargo deny
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: EmbarkStudios/cargo-deny-action@v1
```

- [ ] **Step 2: Verify the configured policy locally**

Run: `cargo deny check`

Expected: advisories, bans, licenses, and sources all complete without denial.
Warnings configured as non-blocking must remain visible.

- [ ] **Step 3: Validate workflow syntax and commit**

Parse `.github/workflows/ci.yml` with an available YAML-aware checker, inspect
the job through `rg -n "cargo-deny|EmbarkStudios"`, then commit with `but` as
`ci: enforce cargo deny policy`.

### Task 4: Complete documentation and run the P1-P16 audit

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`
- Modify: implementation files only when verification proves a defect.

- [ ] **Step 1: Document all new user-visible behavior**

README must include `gar diff`, analyze `--pattern`, TUI `<`/`>` and `v`/`V`,
`gar_core::format_chunk`, contribution entry point, and `just verify`.

- [ ] **Step 2: Run fresh full verification**

```text
cargo check --workspace --all-targets --locked --message-format=json
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo deny check
```

Run real CLI checks for roundtrip, diff statuses, and combined analyze filters.
Compile all fuzz targets and run bounded campaigns when cargo-fuzz is present.

- [ ] **Step 3: Audit P1-P16 one by one**

For each item, record its proving file/test/command. P8 must cite the real
140-cell integration matrix; no item may be closed solely because aggregate
tests pass.

- [ ] **Step 4: Commit final documentation or verification fixes**

Use `but diff`, exclude `research/`, and commit only necessary files as
`docs: document completed improvement program`.

