# Development

## Prerequisites

- Rust (stable toolchain) — install via [rustup](https://rustup.rs/)
- `cargo-llvm-cov` for coverage (optional)

## Build

```bash
# Debug build
cargo build

# Release build (optimized, stripped)
cargo build --release
```

The release binary is written to `target/release/imagen`.

## Run Tests

```bash
# Run all tests
cargo test --release

# Run a specific test
cargo test --release -- test_name_filter

# Run integration tests only
cargo test --release --test '*'
```

Integration tests live in `tests/` and use cassette replay — no API keys required.

## Lint

```bash
# Check for lint warnings
cargo clippy

# Auto-fix clippy warnings
cargo clippy --fix --allow-dirty
```

The project enforces `clippy::pedantic` warnings. Zero warnings are required before merging.

## Format

```bash
# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt
```

## Coverage

Requires `cargo-llvm-cov`:

```bash
cargo install cargo-llvm-cov
```

Then run:

```bash
./scripts/coverage.sh
```

This generates an HTML coverage report in `target/llvm-cov/html/index.html` and prints a summary to stdout. The project targets 85% line coverage.

## CI

GitHub Actions runs on every push and pull request:

- **test** — `cargo test --release`
- **lint** — `cargo clippy` (pedantic, zero warnings)
- **format** — `cargo fmt --check`

See `.github/workflows/ci.yml` for the full workflow.

## Release

Releases are built and published via GitHub Actions when a version tag is pushed:

```bash
./scripts/release.sh 0.2.0
```

This script:
1. Bumps the version in `Cargo.toml`
2. Creates a git commit and tag
3. Pushes to GitHub

The CI release workflow then builds binaries for Linux and macOS (x86_64 and aarch64) and uploads them as GitHub release assets. The install script downloads the correct binary for the user's platform.

See `scripts/release.sh` for details.

## Project Structure

```
src/
├── main.rs          # Entry point, CLI wiring
├── cli.rs           # Clap argument structs
├── config.rs        # Config file loading
├── context.rs       # Service context and adapter wiring
├── model.rs         # Model name resolution
├── params.rs        # Parameter validation and translation
├── output.rs        # File naming and image saving
├── error.rs         # Unified error type
│
├── ports/
│   └── image_generator.rs   # ImageGenerator trait
│
├── adapters/
│   ├── live/                # Gemini and OpenAI HTTP adapters
│   ├── recording/           # Records interactions to cassette
│   └── replaying/           # Replays interactions from cassette
│
└── cassette/                # YAML cassette format, recorder, replayer

tests/
├── cli.rs           # CLI argument and output tests
└── record_replay.rs # Cassette replay integration tests

test_fixtures/               # Recorded cassettes for replay tests
```
