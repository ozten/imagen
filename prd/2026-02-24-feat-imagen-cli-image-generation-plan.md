---
title: "feat: Imagen CLI - AI Image Generation Tool"
type: feat
status: active
date: 2026-02-24
---

# feat: Imagen CLI - AI Image Generation Tool

## Overview

Imagen is a Rust CLI tool for generating images via AI model APIs. It provides a unified interface across Google Gemini and OpenAI image generation models with short-name aliases for popular models, configurable aspect ratios, sizes, quality, and output formats. Built with hexagonal architecture (ports/adapters pattern stolen from `../speck`) with cassette-based record/replay testing infrastructure. Distributed as curl-installable binaries (release pipeline stolen from `../blacksmith`).

## Problem Statement / Motivation

Generating images via AI APIs currently requires:
- Writing custom HTTP requests per provider
- Remembering different parameter formats (Gemini uses "1:1" aspect ratio, ChatGPT uses "1024x1024" pixel sizes)
- Managing API keys across providers
- No unified CLI exists that normalizes these differences

Imagen solves this by providing a single CLI with consistent parameter semantics that translates to each provider's native API format.

## Proposed Solution

A Rust binary (`imagen`) with:
- `clap`-based CLI parsing
- Hexagonal architecture with `ImageGenerator` port trait
- Live adapters for Gemini and OpenAI APIs
- Recording/replaying adapters for deterministic testing
- Curl-installable release pipeline via GitHub Actions

### CLI Interface

```
imagen [OPTIONS] [PROMPT]

Arguments:
  [PROMPT]  Text prompt describing the desired image

Options:
  -p, --prompt-file <PATH>     Path to a file containing the prompt text
  -m, --model <MODEL>          Model name [default: nano-banana]
  -a, --aspect-ratio <RATIO>   Aspect ratio [default: 1:1]
  -s, --size <SIZE>            Image size: 1K, 2K, 4K [default: 1K]
  -q, --quality <QUALITY>      Quality (ChatGPT only) [default: auto]
  -f, --format <FORMAT>        Output format: jpeg, png, webp [default: jpeg]
  -o, --output <PATH>          Output file path [default: auto-generated]
  -n, --count <N>              Number of images [default: 1]
      --config <PATH>          Config file path override
  -v, --verbose                Verbose output
  -h, --help                   Print help
  -V, --version                Print version
```

The prompt can be provided as a positional argument or via `-p/--prompt-file` (mutually exclusive). One of the two must be provided.

### Model Name Resolution

| Short Name | Resolved Model | Provider |
|---|---|---|
| `nano-banana` (default) | `gemini-3-pro-image-preview` | Gemini |
| `gpt-1.5` | `gpt-image-1.5` | OpenAI |
| `gpt-1` | `gpt-image-1` | OpenAI |
| `gpt-1-mini` | `gpt-image-1-mini` | OpenAI |

Any exact model name (e.g., `gemini-3-pro-image-preview`, `gpt-image-1.5`) is also accepted. Provider is inferred from the resolved model name prefix (`gemini-*` = Gemini, `gpt-*` = OpenAI).

### Parameter Translation

#### Aspect Ratio
- **Gemini**: Passed directly as string (`"1:1"`, `"16:9"`, `"9:16"`, `"3:4"`, `"4:3"`, `"2:3"`, `"3:2"`, `"4:5"`, `"5:4"`, `"21:9"`)
- **OpenAI**: Translated to pixel dimensions:

| Aspect Ratio | OpenAI Size |
|---|---|
| `1:1` | `1024x1024` |
| `16:9`, `3:2`, `4:3`, `21:9` (landscape) | `1536x1024` |
| `9:16`, `2:3`, `3:4` (portrait) | `1024x1536` |

#### Image Size
- **Gemini**: Passed directly (`"1K"`, `"2K"`, `"4K"`)
- **OpenAI**: The API only accepts four fixed pixel sizes: `1024x1024`, `1536x1024`, `1024x1536`, `auto`. Arbitrary dimensions (e.g., `2048x2048`) are rejected. We map:
  - `1K` → use the standard aspect-ratio-derived dimensions
  - `2K` / `4K` → use `auto` (model picks best) and log a note in verbose mode that OpenAI doesn't support higher resolutions natively

#### Quality
- **Gemini**: Ignored (not configurable)
- **OpenAI**: Passed directly (`"auto"`, `"low"`, `"medium"`, `"high"`)

#### Output Format
- **Gemini**: Not configurable via API (always returns JPEG). Client-side conversion to png/webp when requested.
- **OpenAI**: Passed directly as `output_format` (`"png"`, `"jpeg"`, `"webp"`)

### Auto-Generated Output Filenames

When no `-o` flag is provided:
- Sanitize the first 50 chars of the prompt to kebab-case
- Append timestamp: `{sanitized-prompt}-{unix-timestamp}.{format}`
- Example: `sunset-over-mountains-1740422400.jpg`

## Technical Approach

### Architecture

Hexagonal architecture following the `speck` project patterns:

```
src/
├── main.rs                    # CLI entry point, arg parsing, context wiring
├── cli.rs                     # Clap derive structs
├── config.rs                  # Config file loading (~/.config/imagen/config.toml)
├── context.rs                 # ServiceContext wiring (live/recording/replaying)
├── model.rs                   # Model name resolution, provider detection
├── params.rs                  # Parameter translation (aspect ratio, size, etc.)
├── output.rs                  # File naming, image saving, format conversion
│
├── ports/
│   ├── mod.rs                 # Re-exports
│   └── image_generator.rs     # ImageGenerator trait + request/response types
│
├── adapters/
│   ├── live/
│   │   ├── mod.rs
│   │   ├── gemini.rs          # Gemini API adapter
│   │   └── openai.rs          # OpenAI API adapter
│   ├── recording/
│   │   ├── mod.rs             # record_interaction / record_result helpers
│   │   └── image_generator.rs # Recording wrapper
│   └── replaying/
│       ├── mod.rs             # next_output / replay_result helpers
│       └── image_generator.rs # Replaying adapter
│
├── cassette/
│   ├── mod.rs
│   ├── format.rs              # Cassette + Interaction structs (YAML serde)
│   ├── recorder.rs            # Write interactions to YAML
│   ├── replayer.rs            # Read interactions from YAML
│   └── config.rs              # Cassette configuration
│
└── error.rs                   # Unified error type
```

### Port Trait

```rust
// src/ports/image_generator.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest {
    pub model: String,
    pub prompt: String,
    pub aspect_ratio: String,
    pub size: String,
    pub quality: String,
    pub format: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponse {
    pub images: Vec<GeneratedImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    pub data: Vec<u8>,       // Raw image bytes
    pub mime_type: String,   // e.g., "image/jpeg"
}

pub type GenerateFuture<'a> = Pin<Box<dyn Future<Output = Result<ImageResponse, ImageError>> + Send + 'a>>;

pub trait ImageGenerator: Send + Sync {
    fn generate(&self, request: &ImageRequest) -> GenerateFuture<'_>;
}
```

### Configuration File

Location: `~/.config/imagen/config.toml`

Discovery order:
1. `--config <path>` CLI flag
2. `IMAGEN_CONFIG` environment variable
3. `~/.config/imagen/config.toml`

```toml
# ~/.config/imagen/config.toml
[keys]
gemini = "your-gemini-api-key"      # or set GEMINI_API_KEY env var
openai = "your-openai-api-key"      # or set OPENAI_API_KEY env var

[defaults]
model = "nano-banana"
aspect_ratio = "1:1"
size = "1K"
quality = "auto"
format = "jpeg"
```

Environment variables take precedence over config file values.

### Implementation Phases

#### Phase 1: Foundation (Scaffold + Core Types)

**Tasks:**
- [ ] `cargo init --name imagen` with workspace setup
- [ ] `Cargo.toml` with dependencies: `clap`, `serde`, `serde_json`, `serde_yaml`, `reqwest`, `tokio`, `chrono`, `base64`, `image` (for format conversion)
- [ ] `clippy.toml` (MSRV 1.85, pedantic)
- [ ] `rustfmt.toml` (max_width 100)
- [ ] `src/cli.rs` - Clap derive structs for all CLI args
- [ ] `src/error.rs` - `ImageError` enum with `thiserror`
- [ ] `src/model.rs` - Model name resolution and provider detection
- [ ] `src/params.rs` - Parameter translation logic
- [ ] `src/config.rs` - TOML config loading with env var override
- [ ] `src/output.rs` - Auto-naming and image saving

**Success criteria:** `cargo build` compiles, `imagen --help` prints usage, `cargo clippy` passes, `cargo fmt --check` passes.

**Files:**
- `Cargo.toml`
- `clippy.toml`
- `rustfmt.toml`
- `src/main.rs`
- `src/cli.rs`
- `src/error.rs`
- `src/model.rs`
- `src/params.rs`
- `src/config.rs`
- `src/output.rs`

#### Phase 2: Hexagonal Core (Ports + Adapters)

**Tasks:**
- [ ] `src/ports/image_generator.rs` - `ImageGenerator` trait, request/response types
- [ ] `src/adapters/live/gemini.rs` - Gemini API adapter using `reqwest`
- [ ] `src/adapters/live/openai.rs` - OpenAI API adapter using `reqwest`
- [ ] `src/context.rs` - `ServiceContext` with `live()` constructor
- [ ] `src/main.rs` - Wire CLI args → ImageRequest → provider dispatch → save output

**Success criteria:** `GEMINI_API_KEY=xxx imagen "a cat"` generates an image. `OPENAI_API_KEY=xxx imagen -m gpt-1 "a cat"` generates an image.

**Files:**
- `src/ports/mod.rs`
- `src/ports/image_generator.rs`
- `src/adapters/live/mod.rs`
- `src/adapters/live/gemini.rs`
- `src/adapters/live/openai.rs`
- `src/context.rs`
- `src/main.rs` (updated)

#### Phase 2.5: Record Initial Cassettes (Manual, One-Time)

This is a manual step that requires real API keys. The agentic harness must be stopped so the user can configure keys.

**Tasks:**
- [ ] Ensure Phases 1+2 produce a working `imagen` binary that can record
- [ ] **`touch STOP`** to stop the agentic harness
- [ ] User creates `~/.config/imagen/config.toml` with API keys:
  ```toml
  [keys]
  gemini = "your-gemini-api-key"
  openai = "your-openai-api-key"
  ```
- [ ] User records Gemini cassette: `IMAGEN_REC=true cargo run -- "a cat"`
- [ ] User records OpenAI cassette: `IMAGEN_REC=true cargo run -- -m gpt-1 "a cat"`
- [ ] Copy recorded cassettes to `test_fixtures/`:
  - `test_fixtures/gemini_cat.cassette.yaml`
  - `test_fixtures/openai_cat.cassette.yaml`

**Success criteria:** Two cassette YAML files exist in `test_fixtures/` with real recorded API interactions.

**Note:** After this step, all subsequent automated tests replay from these cassettes. No test ever hits a live API.

#### Phase 3: Record/Replay (Cassette Infrastructure)

**Tasks:**
- [ ] `src/cassette/format.rs` - Cassette + Interaction structs
- [ ] `src/cassette/recorder.rs` - CassetteRecorder
- [ ] `src/cassette/replayer.rs` - CassetteReplayer with per-port queues
- [ ] `src/cassette/config.rs` - CassetteConfig for per-port cassette paths
- [ ] `src/adapters/recording/image_generator.rs` - RecordingImageGenerator
- [ ] `src/adapters/replaying/image_generator.rs` - ReplayingImageGenerator
- [ ] `src/adapters/recording/mod.rs` - `record_interaction` / `record_result` helpers
- [ ] `src/adapters/replaying/mod.rs` - `next_output` / `replay_result` helpers
- [ ] `src/context.rs` - Add `recording()` and `replaying()` constructors
- [ ] Wire `IMAGEN_REC=true` and `IMAGEN_REPLAY=<path>` env vars in `main.rs`

**Success criteria:** `IMAGEN_REC=true imagen "a cat"` records to `.imagen/cassettes/<timestamp>/`. `IMAGEN_REPLAY=<path> imagen "a cat"` replays deterministically.

**Files:**
- `src/cassette/mod.rs`
- `src/cassette/format.rs`
- `src/cassette/recorder.rs`
- `src/cassette/replayer.rs`
- `src/cassette/config.rs`
- `src/adapters/recording/mod.rs`
- `src/adapters/recording/image_generator.rs`
- `src/adapters/replaying/mod.rs`
- `src/adapters/replaying/image_generator.rs`
- `src/context.rs` (updated)
- `src/main.rs` (updated)

#### Phase 4: Testing + Quality

**CRITICAL CONSTRAINT: No automated test may ever hit a live API endpoint.** All tests use either pure unit logic or pre-recorded cassette fixtures from `test_fixtures/`. Live adapters (`src/adapters/live/`) are excluded from code coverage targets.

**Tasks:**
- [ ] Unit tests for `model.rs` (name resolution, provider detection)
- [ ] Unit tests for `params.rs` (aspect ratio translation, size mapping)
- [ ] Unit tests for `config.rs` (TOML parsing, env var override)
- [ ] Unit tests for `output.rs` (filename sanitization, auto-naming)
- [ ] Unit tests for `cassette/` modules (recorder, replayer, format round-trip)
- [ ] Unit tests for recording/replaying adapters
- [ ] Integration test: `tests/cli.rs` - CLI arg parsing and validation (no network)
- [ ] Integration test: `tests/record_replay.rs` - Full cassette round-trip using `test_fixtures/` cassettes
- [ ] Verify all integration tests use `IMAGEN_REPLAY=<cassette>` to replay, never call live APIs
- [ ] Code coverage with `cargo-llvm-cov` targeting 80%
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo fmt --all -- --check`

**Success criteria:** `cargo test` passes with NO API keys set, `cargo llvm-cov` reports >= 80% coverage (excluding `main.rs` and live adapters).

**Files:**
- `tests/cli.rs`
- `tests/record_replay.rs`
- `test_fixtures/gemini_cat.cassette.yaml`
- `test_fixtures/openai_cat.cassette.yaml`
- Unit tests inline in each module

#### Phase 5: Release Pipeline + Installation

Adapt directly from `../blacksmith`. Each file below references the blacksmith source to copy and modify:

**Tasks:**
- [ ] `scripts/install.sh` ← Adapt from `../blacksmith/scripts/install.sh`
  - Change `REPO="ozten/blacksmith"` → `REPO="ozten/imagen"`
  - Change `BLACKSMITH_VERSION` env var → `IMAGEN_VERSION`
  - Remove dual binary handling (only `imagen`, no `-ui`)
  - Keep: platform detection (linux_amd64, linux_arm64, darwin_amd64, darwin_arm64), smart install dir (`/usr/local/bin` → fallback `~/.local/bin`), macOS code-signing, curl/wget fallback
  - Archive naming: `imagen_{version}_{platform}.tar.gz`
- [ ] `scripts/release.sh` ← Adapt from `../blacksmith/scripts/release.sh`
  - Change to single Cargo.toml (no workspace member)
  - Same semver validation, clean tree check, tag creation
  - `sed` updates version in `Cargo.toml`, runs `cargo check`, commits + tags
- [ ] `.github/workflows/release.yml` ← Adapt from `../blacksmith/.github/workflows/release.yml`
  - Same 4-platform matrix (linux amd64/arm64, darwin amd64/arm64)
  - Same `cross` for linux ARM64
  - Single binary package (`imagen` only, no workspace)
  - Same SHA256 checksums + `softprops/action-gh-release@v2`
  - Same `latest` tag update
- [ ] `.github/workflows/ci.yml` ← New file, pattern from blacksmith's quality gates
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-targets --all-features`
- [ ] Update `README.md` install section matching blacksmith's pattern:
  ```
  curl -fsSL https://raw.githubusercontent.com/ozten/imagen/main/scripts/install.sh | bash
  ```

**Success criteria:** `curl -fsSL https://raw.githubusercontent.com/ozten/imagen/main/scripts/install.sh | bash` installs the binary. `./scripts/release.sh 0.1.0 && git push origin main v0.1.0` triggers a release.

**Files:**
- `scripts/install.sh` (source: `../blacksmith/scripts/install.sh`)
- `scripts/release.sh` (source: `../blacksmith/scripts/release.sh`)
- `.github/workflows/release.yml` (source: `../blacksmith/.github/workflows/release.yml`)
- `.github/workflows/ci.yml` (new)

#### Phase 6: Documentation

**Tasks:**
- [ ] `README.md` - Install, usage, model table, examples
- [ ] `docs/record-replay.md` - How to record and replay sessions
- [ ] `docs/development.md` - Build, test, lint, coverage instructions
- [ ] `docs/architecture.md` - Hexagonal architecture overview

**Success criteria:** A new developer can `cargo build`, run tests, record a session, and replay it by following the docs.

**Files:**
- `README.md`
- `docs/record-replay.md`
- `docs/development.md`
- `docs/architecture.md`

## Alternative Approaches Considered

### 1. Single Provider (Gemini only)
**Rejected:** The value of this tool is the unified interface across providers. Supporting only one doesn't justify a CLI over curl.

### 2. Plugin architecture for providers
**Rejected:** Over-engineering for 2 providers. The port trait is sufficient. If a third provider is added later, the pattern already supports it via a new adapter.

### 3. gRPC or SDK-based API clients
**Rejected:** Direct HTTP via `reqwest` is simpler, more transparent, and easier to record/replay. Both APIs are straightforward REST endpoints.

## System-Wide Impact

### Interaction Graph

1. CLI args parsed by `clap` → `ImageRequest` built by `params.rs`
2. Model name resolved by `model.rs` → provider detected
3. `ServiceContext` selects adapter (live/recording/replaying) based on env vars
4. Adapter calls external API (Gemini or OpenAI) → returns base64 image data
5. `output.rs` decodes base64, optionally converts format (Gemini JPEG→PNG/WebP), writes file

### Error Propagation

- API errors (4xx, 5xx) → `ImageError::ApiError { status, message }` → printed to stderr, exit code 1
- Config errors → `ImageError::ConfigError` → printed with helpful message about config file location
- Network errors → `ImageError::NetworkError` → printed with retry suggestion
- File write errors → `ImageError::IoError` → printed with path info

### State Lifecycle Risks

- **No persistent state beyond config file.** Each invocation is stateless.
- **Cassette recordings** are append-only YAML files. Partial failure leaves a valid but incomplete cassette.
- **Output images** are written atomically (write to temp file, rename).

### API Surface Parity

Both providers support the same CLI interface. Differences are handled internally:
- Gemini ignores `--quality` (logged in verbose mode)
- Gemini does not support output format control; client-side conversion is used
- OpenAI does not support `2K`/`4K` sizes natively; uses max available size

### Integration Test Scenarios

1. **Gemini happy path**: Record a Gemini generation, replay it, verify identical output file
2. **OpenAI happy path**: Record an OpenAI generation, replay it, verify identical output file
3. **Aspect ratio translation**: Verify `--aspect-ratio 16:9` produces `1536x1024` for OpenAI
4. **Format conversion**: Verify `--format png` with Gemini converts JPEG response to PNG
5. **Config precedence**: Verify env var overrides config file API key

## Acceptance Criteria

### Functional Requirements

- [ ] `imagen "a cat"` generates an image using default model (nano-banana/Gemini)
- [ ] `imagen -m gpt-1 "a cat"` generates an image using OpenAI gpt-image-1
- [ ] `imagen -p /tmp/abc.txt` reads prompt from file via `-p/--prompt-file` flag
- [ ] `imagen -a 16:9 "landscape"` generates a 16:9 image
- [ ] `imagen -s 4K "detailed scene"` generates a 4K image (Gemini) or max-res (OpenAI)
- [ ] `imagen -q high -m gpt-1 "portrait"` generates a high-quality OpenAI image
- [ ] `imagen -f png "logo"` generates a PNG (converts from JPEG for Gemini)
- [ ] `imagen -o output.jpg "a cat"` saves to specified path
- [ ] `imagen "a cat"` auto-generates filename when no -o flag
- [ ] Config file at `~/.config/imagen/config.toml` is loaded for API keys
- [ ] `GEMINI_API_KEY` / `OPENAI_API_KEY` env vars override config file
- [ ] `IMAGEN_REC=true imagen "a cat"` records cassette
- [ ] `IMAGEN_REPLAY=path imagen "a cat"` replays from cassette

### Non-Functional Requirements

- [ ] Compiles on Linux x86_64, Linux ARM64, macOS x86_64, macOS ARM64
- [ ] Binary size under 20MB
- [ ] Cold start under 100ms (excluding network latency)
- [ ] Curl-installable from GitHub releases

### Quality Gates

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --all-targets --all-features` passes
- [ ] Code coverage >= 80% (excluding `main.rs` and live adapters)
- [ ] `unsafe_code = "forbid"` in Cargo.toml lints

## Success Metrics

- A user can install via `curl | bash` and generate an image in under 2 minutes
- All tests pass without any API keys (using cassette replay)
- Adding a new provider requires only: one new adapter file + model mapping entry

## Dependencies & Prerequisites

### Rust Crate Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `clap` | 4.x | CLI argument parsing (derive) |
| `serde` | 1.x | Serialization framework |
| `serde_json` | 1.x | JSON serialization |
| `serde_yaml` | 0.9.x | YAML cassette format |
| `toml` | 0.8.x | Config file parsing |
| `reqwest` | 0.12.x | HTTP client (rustls-tls) |
| `tokio` | 1.x | Async runtime (macros, rt) |
| `chrono` | 0.4.x | Timestamps for cassettes |
| `base64` | 0.22.x | Base64 decoding of API responses |
| `image` | 0.25.x | Image format conversion (JPEG→PNG/WebP) |
| `thiserror` | 2.x | Error type derivation |

### Dev Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `assert_cmd` | 2.x | CLI integration testing |
| `predicates` | 3.x | Test assertions |

### External Requirements

- `GEMINI_API_KEY` for live Gemini calls (Tier 1+ required for image generation)
- `OPENAI_API_KEY` for live OpenAI calls
- Rust toolchain (MSRV 1.85)
- `cross` for ARM64 cross-compilation in CI

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Gemini API changes (preview model) | Medium | High | Pin API version, cassette tests catch regressions |
| OpenAI rate limiting | Medium | Low | Exponential backoff, clear error messages |
| Image format conversion quality loss | Low | Medium | Use `image` crate with lossless PNG, configurable WebP quality |
| Config file location confusion | Medium | Low | `--config` flag override, `imagen --config-path` to print location |
| DALL-E 3 sunset (May 2026) | Certain | None | Not supporting DALL-E 3, only GPT image models |

## Documentation Plan

### docs/record-replay.md

```markdown
# Record/Replay

Imagen includes a cassette-based record/replay system for deterministic
testing without real API calls.

## Recording a Session

Set IMAGEN_REC=true to record:

    IMAGEN_REC=true imagen "a cat sitting on a windowsill"

Interactions are saved to .imagen/cassettes/<timestamp>/.

## Replaying a Session

Set IMAGEN_REPLAY to the cassette path:

    IMAGEN_REPLAY=test_fixtures/gemini_cat.cassette.yaml imagen "a cat sitting on a windowsill"

Output is deterministic - identical every run.

## How It Works

Every API call flows through an ImageGenerator port trait. Three adapter
implementations exist:

1. Live - real API calls (default)
2. Recording - wraps live, saves request/response to YAML cassette
3. Replaying - serves responses from a recorded cassette

## Cassette Format

Cassettes are YAML files:

    name: "2026-02-24T10-30-00-image_generator"
    recorded_at: "2026-02-24T10:30:00Z"
    commit: "abc123"
    interactions:
      - seq: 0
        port: image_generator
        method: generate
        input: { model: "gemini-3-pro-image-preview", prompt: "a cat", ... }
        output: { Ok: { images: [{ data: "base64...", mime_type: "image/jpeg" }] } }
```

### docs/development.md

Covers: build, test, lint, format, coverage, CI, git hooks.

## Cargo.toml

```toml
[package]
name = "imagen"
version = "0.1.0"
edition = "2021"
description = "AI image generation CLI - unified interface for Gemini and OpenAI"
license = "MIT"
repository = "https://github.com/ozten/imagen"

[dependencies]
base64 = "0.22"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
image = "0.25"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
toml = "0.8"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"

[profile.release]
opt-level = "s"
lto = true
strip = true
```

## Sources & References

### Internal References (Stolen Patterns)

- **Hexagonal architecture**: `/home/admin/speck/src/ports/`, `/home/admin/speck/src/adapters/`, `/home/admin/speck/src/context.rs`
- **Cassette infrastructure**: `/home/admin/speck/src/cassette/`
- **Record/replay docs**: `/home/admin/speck/docs/record-replay.md`
- **Install script**: `/home/admin/blacksmith/scripts/install.sh`
- **Release script**: `/home/admin/blacksmith/scripts/release.sh`
- **Release workflow**: `/home/admin/blacksmith/.github/workflows/release.yml`
- **Cargo.toml patterns**: `/home/admin/speck/Cargo.toml`, `/home/admin/blacksmith/Cargo.toml`
- **Code quality config**: `/home/admin/speck/clippy.toml`, `/home/admin/speck/rustfmt.toml`

### External References

- Gemini Image Generation API: `POST https://generativelanguage.googleapis.com/v1beta/models/gemini-3-pro-image-preview:generateContent`
- Gemini auth: `x-goog-api-key` header or `?key=` query param
- Gemini response: `candidates[].content.parts[].inline_data.data` (base64 JPEG)
- Gemini aspect ratios: 1:1, 2:3, 3:2, 3:4, 4:3, 4:5, 5:4, 9:16, 16:9, 21:9
- Gemini image sizes: 1K (default), 2K, 4K
- Gemini format control: **not supported** - always returns JPEG, convert client-side
- OpenAI Image Generation API: `POST https://api.openai.com/v1/images/generations`
- OpenAI auth: `Authorization: Bearer <key>` header
- OpenAI response: `data[].b64_json` (base64) or `data[].url`
- OpenAI sizes: 1024x1024, 1536x1024, 1024x1536, auto
- OpenAI quality: auto, low, medium, high
- OpenAI format: png (default), jpeg, webp via `output_format` param
- OpenAI models: gpt-image-1.5 (best), gpt-image-1, gpt-image-1-mini
- DALL-E 3 sunset: May 12, 2026 (not supported)
