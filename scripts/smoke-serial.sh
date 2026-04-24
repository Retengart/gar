#!/usr/bin/env bash
# smoke-serial.sh — Run the full workspace test matrix 10 times with
# --test-threads=8 to catch residual races in env-touching tests.
#
# Expected usage: run once locally before landing a Phase 2 commit.
# Does not replace the CI --test-threads=8 step (.github/workflows/ci.yml);
# this is the phase-handoff gate from TEST-04 Success Criterion 3.
#
# Exit 0 on 10/10 success. Exit non-zero with the iteration number on
# the first failure.

set -euo pipefail

ITERATIONS=10

for i in $(seq 1 "$ITERATIONS"); do
    echo "=== smoke-serial iteration $i / $ITERATIONS ==="
    if ! cargo test --workspace --all-targets --locked -- --test-threads=8; then
        echo "smoke-serial: iteration $i failed" >&2
        exit 1
    fi
done

echo "smoke-serial: $ITERATIONS / $ITERATIONS iterations passed."
