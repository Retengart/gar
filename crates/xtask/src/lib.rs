//! Workspace-level automation helpers for gar.
//!
//! This crate hosts repo-wide invariant checks that run as integration
//! tests under `cargo test --workspace --all-targets --locked`. It has
//! no runtime code; all behaviour is in `tests/*.rs`.
