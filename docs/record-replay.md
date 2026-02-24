# Record & Replay

Imagen uses cassette-based record/replay testing to run integration tests without live API calls. This lets you record real API interactions once, then replay them deterministically in CI.

## How It Works

Cassettes are YAML files that store port-level interactions (request/response pairs serialized at the `ImageGenerator` trait boundary). When replaying, the `ReplayingImageGenerator` adapter reads interactions from the cassette instead of hitting the live API.

```
test_fixtures/
├── gemini_cat.cassette.yaml
└── openai_cat.cassette.yaml
```

Each cassette entry captures:
- The outbound `ImageRequest` (model, prompt, aspect ratio, etc.)
- The inbound `ImageResponse` (image bytes as base64 and MIME type)

## Recording a Cassette

Set the `IMAGEN_RECORD` environment variable, then run imagen normally with a live API key.

To record to an auto-generated path under `.imagen/cassettes/`:

```bash
GEMINI_API_KEY=your-key \
IMAGEN_RECORD=1 \
  imagen "a simple red square"
```

To record to a specific path:

```bash
GEMINI_API_KEY=your-key \
IMAGEN_RECORD=test_fixtures/my_cassette.cassette.yaml \
  imagen "a simple red square"
```

`IMAGEN_RECORD=1` and `IMAGEN_RECORD=true` both use auto-generated paths. Any other value is treated as a file path.

This writes the request and response to the cassette file. The generated image is also saved normally. Each recording creates a fresh cassette file (it does not append to an existing one).

## Replaying a Cassette

Set `IMAGEN_REPLAY` to the cassette path. No API key is required:

```bash
IMAGEN_REPLAY=test_fixtures/gemini_cat.cassette.yaml \
  imagen "a cat"
```

The `ReplayingImageGenerator` returns recorded responses in order. If more requests are made than interactions recorded, an error is returned.

## Cassette Format

Cassettes are YAML files with metadata and a list of interactions:

```yaml
name: gemini-cat
recorded_at: "2026-02-01T00:00:00Z"
commit: abc123
interactions:
  - seq: 0
    port: image_generator
    method: generate
    input:
      model: gemini-3-pro-image-preview
      prompt: a cat
      aspect_ratio: "1:1"
      size: 1K
      quality: auto
      format: jpeg
      count: 1
    output:
      Ok:
        images:
          - data: /9j/4AAQ...   # base64-encoded image bytes
            mime_type: image/jpeg
```

Fields:
- **name** — human-readable cassette label
- **recorded_at** — ISO 8601 timestamp of recording
- **commit** — git commit hash at recording time
- **interactions** — ordered list; each has a `seq` number, `port` and `method` identifying the trait call, `input` (the `ImageRequest`), and `output` (the `Result<ImageResponse, ImageError>`)

## Writing Tests with Cassettes

Integration tests in `tests/` use cassettes via the `IMAGEN_REPLAY` environment variable and `assert_cmd`:

```rust
#[test]
fn gemini_happy_path_creates_file() {
    let cassette = fixtures_dir().join("gemini_cat.cassette.yaml");
    let out = std::env::temp_dir().join("imagen_test_gemini_happy.jpg");

    cmd()
        .env("IMAGEN_REPLAY", cassette.to_str().unwrap())
        .env_remove("GEMINI_API_KEY")
        .args(["--model", "nano-banana", "--output", out.to_str().unwrap(), "a cat"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Saved:"));

    assert!(out.exists());
}
```

See `tests/record_replay.rs` for full examples.

## Tips

- Record cassettes on a developer machine with real API keys
- Commit cassette files to the repository so CI can replay them
- When the API changes, re-record the affected cassettes
- Use separate cassettes per test scenario for clarity
- Cassette files use the `.cassette.yaml` extension by convention
