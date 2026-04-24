# Benchmarks — advisory only, NEVER CI-gated

These `criterion` benches are a local-only baseline-tracking tool, not a
CI gate. Shared GitHub Actions runners have a 10–15% noise floor that
exceeds any reasonable threshold (PROJECT.md Key Decision row 8;
PITFALLS.md Pitfall 9). CI will **never** run `cargo bench`; Phase 7 SC4
only adds a `cargo bench --workspace --no-run --locked` compile smoke.

## Running locally

```bash
# Capture a baseline on the current commit:
cargo bench -p base60 --bench <name> -- --save-baseline pre

# Apply your change, then compare:
cargo bench -p base60 --bench <name> -- --baseline pre
```

Or for all benches across the workspace:

```bash
cargo bench --workspace -- --save-baseline pre
# ... make changes ...
cargo bench --workspace -- --baseline pre
```

Paste the before/after numbers into the PR description. Reviewers look
at the delta, not a CI checkmark.

## Per-bench scope

| Bench file | Target | Why it exists |
|-----------|--------|---------------|
| `base60-core/benches/convert.rs` | `u64_to_base60` hot loop | Every dump line calls this; regression gate for future `convert` work |
| `base60-core/benches/lens.rs` | `Lens::render` × 4 impls | Baseline for Phase 6 PERF-04 `render_to<W>` migration |
| `base60-cli/benches/dump.rs` | `dump_all` over 1 MiB mono | Baseline for Phase 6 PERF-01 streaming path |
| `base60-cli/benches/decode.rs` | `decode_stream` over 1 MiB dump | Protects roundtrip perf; no REQ-IDs currently depend on it but cheap to track |
| `base60-cli/benches/search.rs` | `find_all` × 4 cells | **Gates Phase 6 PERF-03** `memchr::memmem` swap (PITFALLS Pitfall 4). Every cell must not regress when the swap lands. |

## Noise threshold

Every `Criterion::default()` instance in this project uses
`.noise_threshold(0.05)` — 5% tolerance for local laptop runs. Shared
GHA runners exceed this; running benches on CI would produce false
positives. This is the entire reason benches are advisory-only.

## Determinism

Bench inputs are **compile-time constants** generated via
`wrapping_mul` / `wrapping_add` — no `rand` dep, no checked-in binary
fixtures. Re-running on a different machine will produce the same
microbenchmark inputs, so local baselines are portable across
developer laptops (within the ±5% noise band).
