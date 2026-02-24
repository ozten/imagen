#!/usr/bin/env bash
# Run code coverage using cargo-llvm-cov.
# Excludes main.rs and live adapters (which require real API keys).
# Target: >= 80% line coverage on included source files.
#
# Usage:
#   ./scripts/coverage.sh           # Show summary table
#   ./scripts/coverage.sh --html    # Generate HTML report in target/llvm-cov/html/
#   ./scripts/coverage.sh --lcov    # Generate LCOV report (for CI)

set -euo pipefail

IGNORE_PATTERN="(main|adapters/live)"

case "${1:-}" in
  --html)
    exec cargo llvm-cov --ignore-filename-regex "$IGNORE_PATTERN" --open
    ;;
  --lcov)
    exec cargo llvm-cov --ignore-filename-regex "$IGNORE_PATTERN" --lcov --output-path target/lcov.info
    ;;
  *)
    cargo llvm-cov --ignore-filename-regex "$IGNORE_PATTERN"
    ;;
esac
