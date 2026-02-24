# Architecture

Imagen uses hexagonal architecture (also called ports and adapters). The core domain logic is isolated from infrastructure concerns — API providers, file I/O, and cassette replay are all adapters that plug into defined ports.

## Overview

```
┌─────────────────────────────────────────────────────┐
│                      main.rs                        │
│              CLI parsing, arg wiring                │
└────────────────────────┬────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                   ServiceContext                    │
│         Wires ports to concrete adapters            │
└────────────────────────┬────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│              Port: ImageGenerator                   │
│     trait ImageGenerator: Send + Sync {             │
│         fn generate(&self, req) -> Future<...>      │
│     }                                               │
└───────────┬─────────────────────┬───────────────────┘
            │                     │
            ▼                     ▼
┌──────────────────┐   ┌───────────────────────────────┐
│  Live Adapters   │   │      Test Adapters            │
│                  │   │                               │
│  GeminiGenerator │   │  RecordingImageGenerator      │
│  OpenAiGenerator │   │    wraps a live adapter,      │
│                  │   │    writes interactions to     │
│                  │   │    cassette YAML              │
│                  │   │                               │
│                  │   │  ReplayingImageGenerator      │
│                  │   │    reads interactions from    │
│                  │   │    cassette YAML              │
└──────────────────┘   └───────────────────────────────┘
```

## Layers

### Core Domain

The core domain has no dependencies on external crates or I/O:

- **`model.rs`** — resolves short model names (`nano-banana`) to full model IDs and detects the provider (`gemini-*` vs `gpt-*`)
- **`params.rs`** — validates aspect ratios, sizes, quality, and formats; translates parameters to provider-specific formats
- **`output.rs`** — generates output filenames, saves image bytes to disk

### Port

`src/ports/image_generator.rs` defines the `ImageGenerator` trait:

```rust
pub trait ImageGenerator: Send + Sync {
    fn generate(&self, request: &ImageRequest) -> GenerateFuture<'_>;
}
```

`ImageRequest` and `ImageResponse` are plain data types — no HTTP, no YAML, no filesystem. Any adapter that implements this trait can be substituted without touching the rest of the code.

### Live Adapters

`src/adapters/live/` contains HTTP adapters for each provider:

- **`GeminiGenerator`** — calls the Gemini image generation API; handles base64-encoded `inlineData` responses
- **`OpenAiGenerator`** — calls the OpenAI images API; translates aspect ratios to pixel dimensions

Both adapters receive API keys via `ServiceContext` and build `reqwest` HTTP requests.

### Test Adapters

`src/adapters/recording/` and `src/adapters/replaying/` implement cassette-based testing:

- **`RecordingImageGenerator`** — wraps any `ImageGenerator`, passes requests through to the inner adapter, and writes each request/response pair to a cassette YAML file via `CassetteRecorder`
- **`ReplayingImageGenerator`** — reads a cassette YAML file and returns pre-recorded responses without any network I/O

### Cassette Format

`src/cassette/` handles serialization of cassette files. Each cassette is a YAML list of `Interaction` structs, each containing an `ImageRequest` and `ImageResponse`. See [record-replay.md](record-replay.md) for the full format.

### Service Context

`src/context.rs` wires the application together. It provides three constructors — `live()`, `recording()`, and `replaying()` — that `main.rs` selects based on the `IMAGEN_RECORD` and `IMAGEN_REPLAY` environment variables:

- If `IMAGEN_REPLAY` is set → `ServiceContext::replaying()` uses `ReplayingImageGenerator` (no API key needed)
- If `IMAGEN_RECORD` is set → `ServiceContext::recording()` wraps the live adapter with `RecordingImageGenerator`
- Otherwise → `ServiceContext::live()` uses the live adapter directly

## Design Decisions

**Why hexagonal architecture?**
Isolating the port trait from HTTP and YAML concerns makes it trivial to swap providers, add new providers, or write tests without live API dependencies. The recording/replaying adapters are only possible because the port is clean.

**Why cassette files over mocks?**
Cassette replay tests against real API shapes rather than hand-written mocks. When the API changes, re-recording the cassette catches the breakage immediately. Mocks can silently drift from the real API.

**Why Rust?**
The binary is curl-installable and needs to be fast, small, and cross-platform. The release profile compiles with `opt-level = "s"`, LTO, and binary stripping for minimal download size.
