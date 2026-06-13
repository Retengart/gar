// IMPORTANT: Err returns are the happy path — only panics are bugs.
// On reported crash: reproduce with `--release` first to confirm.
// Platform: Ubuntu + pinned nightly only (libFuzzer is Linux-x86_64/aarch64 only).

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let mut out = Vec::new();
    let _ = base60::__fuzz::decode_stream(
        Cursor::new(data),
        &mut out,
        base60::__fuzz::InputFormat::Auto,
    );
});
