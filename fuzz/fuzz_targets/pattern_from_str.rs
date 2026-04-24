// IMPORTANT: Err returns are the happy path — only panics are bugs.
// On reported crash: reproduce with `--release` first to confirm.
// Platform: Ubuntu + pinned nightly only (libFuzzer is Linux-x86_64/aarch64 only).

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::str::FromStr;

fuzz_target!(|data: &[u8]| {
    // UTF-8 guard matches rust-fuzz/book's canonical pattern — `Pattern::from_str`
    // takes `&str`, so we skip invalid-UTF-8 inputs without treating them as bugs.
    // Do NOT use `std::panic::catch_unwind` — cargo-fuzz compiles with
    // `-Cpanic=abort`, which prevents unwinding.
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = base60::__fuzz::Pattern::from_str(s);
    }
});
