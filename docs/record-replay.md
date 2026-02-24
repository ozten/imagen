# Record & Replay

Imagen uses cassette-based record/replay testing to run integration tests without live API calls. This lets you record real API interactions once, then replay them deterministically in CI.

## How It Works

Cassettes are YAML files that store HTTP request/response pairs. When replaying, the replaying adapter reads interactions from the cassette instead of hitting the live API.

```
test_fixtures/
└── cassettes/
    ├── gemini_generate.yaml
    └── openai_generate.yaml
```

Each cassette entry captures:
- The outbound `ImageRequest` (model, prompt, aspect ratio, etc.)
- The inbound `ImageResponse` (image bytes and MIME type)

## Recording a Cassette

Set the `IMAGEN_RECORD` environment variable to a cassette file path, then run imagen normally with a live API key:

```bash
GEMINI_API_KEY=your-key \
IMAGEN_RECORD=test_fixtures/cassettes/my_cassette.yaml \
  imagen "a simple red square"
```

This writes the request and response to the cassette file. The generated image is also saved normally.

To record multiple interactions into one cassette, run imagen multiple times with the same `IMAGEN_RECORD` path — each run appends a new interaction.

## Replaying a Cassette

Set `IMAGEN_REPLAY` to the cassette path. No API key is required:

```bash
IMAGEN_REPLAY=test_fixtures/cassettes/my_cassette.yaml \
  imagen "a simple red square"
```

The replaying adapter matches requests in order and returns the recorded response. If more requests are made than cassette interactions recorded, an error is returned.

## Cassette Format

Cassettes are YAML files with a list of interactions:

```yaml
interactions:
  - request:
      model: gemini-3-pro-image-preview
      prompt: "a simple red square"
      aspect_ratio: "1:1"
      size: "1K"
      quality: auto
      format: jpeg
      count: 1
    response:
      images:
        - mime_type: image/jpeg
          data: /9j/4AAQ...   # base64-encoded image bytes
```

## Writing Tests with Cassettes

Integration tests in `tests/` use cassettes via the `IMAGEN_REPLAY` environment variable:

```rust
#[test]
fn test_generate_with_gemini_cassette() {
    let cassette = "test_fixtures/cassettes/gemini_generate.yaml";
    let output = Command::new(env!("CARGO_BIN_EXE_imagen"))
        .env("IMAGEN_REPLAY", cassette)
        .args(["a simple red square", "-o", "out.jpg"])
        .output()
        .unwrap();

    assert!(output.status.success());
}
```

See `tests/record_replay.rs` for full examples.

## Tips

- Record cassettes on a developer machine with real API keys
- Commit cassette files to the repository so CI can replay them
- When the API changes, re-record the affected cassettes
- Use separate cassettes per test scenario for clarity
