# imagen

AI image generation CLI — unified interface for Gemini and OpenAI image models.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/ozten/imagen/main/scripts/install.sh | bash
```

To install a specific version:

```bash
IMAGEN_VERSION=0.1.0 curl -fsSL https://raw.githubusercontent.com/ozten/imagen/main/scripts/install.sh | bash
```

## Setup

Set an API key for at least one provider:

```bash
export GEMINI_API_KEY="your-gemini-api-key"
export OPENAI_API_KEY="your-openai-api-key"
```

Keys can also be stored in `~/.config/imagen/config.toml` (see [Configuration](#configuration)).

## Quick Start

Generate an image from a prompt:

```bash
imagen "a cat sitting on a moonlit rooftop"
```

Use a specific model:

```bash
imagen -m gpt-1 "product photo of a red ceramic mug"
```

Save to a specific file:

```bash
imagen -o logo.png "minimalist logo for a coffee shop"
```

Use a prompt file:

```bash
imagen -p my-prompt.txt -f png -a 16:9
```

## Models

| Short Name | Resolved Model | Provider |
|---|---|---|
| `nano-banana` (default) | `gemini-3-pro-image-preview` | Gemini |
| `gpt-1.5` | `gpt-image-1.5` | OpenAI |
| `gpt-1` | `gpt-image-1` | OpenAI |
| `gpt-1-mini` | `gpt-image-1-mini` | OpenAI |

Any exact model name is also accepted (e.g., `gemini-3-pro-image-preview`, `gpt-image-1.5`).

## Options

```
imagen [OPTIONS] [PROMPT]

Arguments:
  [PROMPT]  Text prompt describing the desired image

Options:
  -p, --prompt-file <PATH>     Path to a file containing the prompt text
  -m, --model <MODEL>          Model name [default: nano-banana]
  -a, --aspect-ratio <RATIO>   Aspect ratio [default: 1:1]
  -s, --size <SIZE>            Image size: 1K, 2K, 4K [default: 1K]
  -q, --quality <QUALITY>      Quality: auto, low, medium, high [default: auto]
  -f, --format <FORMAT>        Output format: jpeg, png, webp [default: jpeg]
  -o, --output <PATH>          Output file path [default: auto-generated]
  -n, --count <N>              Number of images [default: 1]
      --config <PATH>          Config file path override
  -v, --verbose                Verbose output
  -h, --help                   Print help
  -V, --version                Print version
```

## Configuration

Create `~/.config/imagen/config.toml`:

```toml
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

API keys are read from config file or environment variables:
- `GEMINI_API_KEY` for Gemini models
- `OPENAI_API_KEY` for OpenAI models

Config discovery order:
1. `--config <path>` CLI flag
2. `IMAGEN_CONFIG` environment variable
3. `~/.config/imagen/config.toml`

### Aspect Ratios

Supported values: `1:1`, `16:9`, `9:16`, `3:4`, `4:3`, `2:3`, `3:2`, `4:5`, `5:4`, `21:9`

Gemini accepts all ratios natively. OpenAI ratios are translated to pixel dimensions automatically.

### Output Filenames

When no `-o` flag is provided, imagen auto-generates a filename:

```
{sanitized-prompt}-{unix-timestamp}.{format}
# Example: a-cat-on-a-rooftop-1740422400.jpg
```

## Documentation

- [Record & Replay](docs/record-replay.md) — Cassette-based testing with recorded API responses
- [Development](docs/development.md) — Build, test, lint, and coverage instructions
- [Architecture](docs/architecture.md) — Hexagonal architecture and adapter design

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test --release

# Lint
cargo clippy

# Coverage
./scripts/coverage.sh
```

See [docs/development.md](docs/development.md) for the full development guide.
