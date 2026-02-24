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
use crate::config::Config;
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

    // Resolve prompt
    let prompt = cli.resolve_prompt().map_err(error::ImageError::Io)?;

    // Resolve model and provider
    let resolved_model = resolve_model(&cli.model);
    let provider = detect_provider(&resolved_model).map_err(error::ImageError::InvalidArgument)?;

    if cli.verbose {
        eprintln!("Model: {} (resolved from '{}')", resolved_model, cli.model);
        eprintln!("Provider: {provider:?}");
    }

    // Validate parameters
    validate_aspect_ratio(&cli.aspect_ratio, provider)
        .map_err(error::ImageError::InvalidArgument)?;
    validate_size(&cli.size).map_err(error::ImageError::InvalidArgument)?;
    validate_quality(&cli.quality).map_err(error::ImageError::InvalidArgument)?;
    validate_format(&cli.format).map_err(error::ImageError::InvalidArgument)?;

    // Build request
    let request = ImageRequest {
        model: resolved_model,
        prompt: prompt.clone(),
        aspect_ratio: cli.aspect_ratio.clone(),
        size: cli.size.clone(),
        quality: cli.quality.clone(),
        format: cli.format.clone(),
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

        let base_path = resolve_output_path(cli.output.as_deref(), &prompt, &cli.format);
        let output_path = if suffix.is_empty() {
            base_path
        } else {
            let stem = base_path.file_stem().unwrap_or_default().to_string_lossy();
            let ext = base_path.extension().unwrap_or_default().to_string_lossy();
            base_path.with_file_name(format!("{stem}{suffix}.{ext}"))
        };

        save_image(&image.data, &image.mime_type, &cli.format, &output_path)?;
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
