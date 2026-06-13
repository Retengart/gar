# Benchmarks — advisory only, NEVER CI-gated

These `criterion` benches are a local-only baseline-tracking tool, not a
CI gate. Shared GitHub Actions runners have a 10–15% noise floor that
exceeds any reasonable threshold. CI will **never** run `cargo bench`.

## Running locally

```bash
# Capture a baseline on the current commit:
cargo bench -p gar --bench <name> -- --save-baseline pre

# Apply your change, then compare:
cargo bench -p gar --bench <name> -- --baseline pre
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
| `gar-core/benches/convert.rs` | `u64_to_base60` hot loop | Every dump line calls this |
| `gar-core/benches/lens.rs` | `Lens::render` × 4 impls | Baseline for lens rendering |
| `gar-cli/benches/dump.rs` | `dump_all` over 1 MiB mono | Baseline for streaming path |
| `gar-cli/benches/decode.rs` | `decode_stream` over 1 MiB dump | Protects roundtrip perf |
| `gar-cli/benches/search.rs` | `find_all` × 4 cells | Baseline for memchr search |

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
