//! Workspace-level automation helpers for base60.
//!
//! This crate hosts repo-wide invariant checks that run as integration
//! tests under `cargo test --workspace --all-targets --locked`. It has
//! no runtime code; all behaviour is in `tests/*.rs`.
