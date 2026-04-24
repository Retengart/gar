#![forbid(unsafe_op_in_unsafe_fn)]
// This is a binary crate: `pub(crate)` on items in private modules is
// semantically accurate (they are not part of any public API) and is
// exactly what `unreachable_pub` asks for. The nursery lint
// `redundant_pub_crate` disagrees; we silence it in favor of the
// correctness-oriented `unreachable_pub`.
#![allow(clippy::redundant_pub_crate)]

//! Entry point for the `base60` binary viewer.

fn main() -> anyhow::Result<()> {
    base60::run()
}
