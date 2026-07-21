# Contributing to gar

Thanks for improving gar. Keep changes focused: the project deliberately has
a small core library and a narrow CLI surface.

## Prerequisites

- Rust 1.95.0 is selected automatically by `rust-toolchain.toml`.
- [`cargo-nextest`](https://nexte.st/) runs the test suite.
- [`cargo-deny`](https://embarkstudios.github.io/cargo-deny/) checks dependency policy.
- [`just`](https://just.systems/) provides the command shortcuts below.
- A nightly toolchain and `cargo-fuzz` are optional unless changing parsers.

Read [CLAUDE.md](CLAUDE.md) before changing architecture or output formats. In
particular, `gar-core` must retain zero runtime dependencies, and the JSON,
HTML, plain-text, and decode roundtrip contracts must remain compatible.

## Workflow

1. Add a failing test that states why the behavior matters.
2. Make the smallest implementation change that passes it.
3. Run the focused test, then `just verify`.
4. For parser changes, also run the relevant fuzz target with
   `just fuzz <target>`.

`just verify` runs the locked workspace check, nextest suite, doctests,
formatting, strict clippy, rustdoc warnings, and cargo-deny policy. No failed or
skipped gate should be described as passing.

## Changes and pull requests

- Use conventional commit subjects such as `fix:`, `feat:`, `test:`, or `docs:`.
- Keep each commit reviewable and avoid unrelated formatting or refactors.
- Explain user-visible output changes and include representative command output.
- Call out compatibility, unsafe-code, dependency, or MSRV effects explicitly.
