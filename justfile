[default]
default:
    @just --list

check:
    cargo check --workspace --all-targets --locked --message-format=json

test:
    cargo nextest run --workspace --all-targets --locked
    cargo test --workspace --doc --locked

lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

deny:
    cargo deny check

fuzz target="decode_stream":
    cargo +nightly fuzz run {{target}} --target "$(rustc +nightly --print host-tuple)" -- -max_total_time=60

verify:
    just check
    just test
    just lint
    just doc
    just deny
