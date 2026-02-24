//! Imagen - AI image generation CLI.

mod adapters;
mod cassette;
mod cli;
mod config;
mod context;
mod error;
mod model;
mod output;
mod params;
mod ports;

use std::path::Path;
use std::process;

use clap::Parser;

use crate::cli::Cli;
use crate::config::{Config, DefaultsConfig};
use crate::context::ServiceContext;
use crate::model::{detect_provider, resolve_model};
use crate::output::{resolve_output_path, save_image};
use crate::params::{validate_aspect_ratio, validate_format, validate_quality, validate_size};
use crate::ports::ImageRequest;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), error::ImageError> {
    // Load config
    let config_path = config::discover_config_path(cli.config.as_deref());
    let config = Config::load(&config_path).map_err(error::ImageError::Config)?;

    // Apply config-file defaults for any CLI flags still at their built-in defaults.
    let cli_defaults = DefaultsConfig::default();
    let effective_model = apply_defaults(&cli.model, &cli_defaults.model, &config.defaults.model);
    let effective_aspect_ratio = apply_defaults(
        &cli.aspect_ratio,
        &cli_defaults.aspect_ratio,
        &config.defaults.aspect_ratio,
    );
    let effective_size = apply_defaults(&cli.size, &cli_defaults.size, &config.defaults.size);
    let effective_quality =
        apply_defaults(&cli.quality, &cli_defaults.quality, &config.defaults.quality);
    let effective_format =
        apply_defaults(&cli.format, &cli_defaults.format, &config.defaults.format);

    // Resolve prompt
    let prompt = cli.resolve_prompt().map_err(error::ImageError::Io)?;

    // Resolve model and provider
    let resolved_model = resolve_model(&effective_model);
    let provider = detect_provider(&resolved_model).map_err(error::ImageError::InvalidArgument)?;

    if cli.verbose {
        eprintln!("Model: {resolved_model} (resolved from '{effective_model}')");
        eprintln!("Provider: {provider:?}");
    }

    // Validate parameters
    validate_aspect_ratio(&effective_aspect_ratio, provider)
        .map_err(error::ImageError::InvalidArgument)?;
    validate_size(&effective_size).map_err(error::ImageError::InvalidArgument)?;
    validate_quality(&effective_quality).map_err(error::ImageError::InvalidArgument)?;
    validate_format(&effective_format).map_err(error::ImageError::InvalidArgument)?;

    // Build request
    let request = ImageRequest {
        model: resolved_model,
        prompt: prompt.clone(),
        aspect_ratio: effective_aspect_ratio.clone(),
        size: effective_size.clone(),
        quality: effective_quality.clone(),
        format: effective_format.clone(),
        count: cli.count,
    };

    // Create context based on mode (live / recording / replaying)
    let replay_path = std::env::var("IMAGEN_REPLAY").ok();
    let is_recording = std::env::var("IMAGEN_REC").is_ok_and(|v| v == "true" || v == "1");

    let (ctx, recording_session) = if let Some(ref cassette_path) = replay_path {
        if cli.verbose {
            eprintln!("Replaying from: {cassette_path}");
        }
        (ServiceContext::replaying(Path::new(cassette_path))?, None)
    } else if is_recording {
        if cli.verbose {
            eprintln!("Recording mode enabled");
        }
        let (ctx, session) = ServiceContext::recording(provider, &config)?;
        (ctx, Some(session))
    } else {
        (ServiceContext::live(provider, &config)?, None)
    };

    // Generate
    let response = ctx.generator.generate(&request).await?;

    // Save images
    for (i, image) in response.images.iter().enumerate() {
        let suffix = if response.images.len() > 1 { format!("-{}", i + 1) } else { String::new() };

        let base_path = resolve_output_path(cli.output.as_deref(), &prompt, &effective_format);
        let output_path = if suffix.is_empty() {
            base_path
        } else {
            let stem = base_path.file_stem().unwrap_or_default().to_string_lossy();
            let ext = base_path.extension().unwrap_or_default().to_string_lossy();
            base_path.with_file_name(format!("{stem}{suffix}.{ext}"))
        };

        save_image(&image.data, &image.mime_type, &effective_format, &output_path)?;
        eprintln!("Saved: {}", output_path.display());
    }

    // Finish recording if active
    if let Some(session) = recording_session {
        match session.finish() {
            Ok(path) => eprintln!("Cassette saved: {}", path.display()),
            Err(e) => eprintln!("Warning: failed to save cassette: {e}"),
        }
    }

    Ok(())
}

/// Returns `cli_val` if it differs from `cli_default` (the user explicitly passed the flag),
/// otherwise returns `config_val` (from the config-file defaults section).
fn apply_defaults(cli_val: &str, cli_default: &str, config_val: &str) -> String {
    if cli_val == cli_default {
        config_val.to_string()
    } else {
        cli_val.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_overrides_cli_default() {
        // When the CLI value is still "nano-banana" (the built-in default), the config default wins.
        assert_eq!(apply_defaults("nano-banana", "nano-banana", "gpt-1"), "gpt-1");
    }

    #[test]
    fn explicit_cli_flag_overrides_config_default() {
        // When the user explicitly sets a different model, that value wins.
        assert_eq!(apply_defaults("dall-e", "nano-banana", "gpt-1"), "dall-e");
    }
}
