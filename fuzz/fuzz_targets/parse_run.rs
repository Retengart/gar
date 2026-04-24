// IMPORTANT: Err returns are the happy path — only panics are bugs.
// On reported crash: reproduce with `--release` first to confirm.
// Platform: Ubuntu + pinned nightly only (libFuzzer is Linux-x86_64/aarch64 only).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Length-gate: `parse_run` takes `&[u8; RUN_LEN]` (Phase 4 D-09).
    // libFuzzer will still mutate inputs past RUN_LEN; we skip those.
    if data.len() != base60::__fuzz::RUN_LEN {
        return;
    }
    let Ok(arr) = <&[u8; base60::__fuzz::RUN_LEN]>::try_from(data) else {
        return;
    };
    // Errors are happy path; only panics are bugs.
    let _ = base60::__fuzz::parse_run(arr, 1);
});
