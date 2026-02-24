//! CLI argument parsing and validation tests — no network I/O.
//!
//! These tests verify that invalid arguments are rejected before any cassette
//! or live adapter is consulted.

use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("imagen").unwrap()
}

#[test]
fn missing_prompt_exits_with_error() {
    // Neither prompt nor --prompt-file given → resolve_prompt() returns an error
    cmd().assert().failure().stderr(predicate::str::contains("Provide a prompt string"));
}

#[test]
fn invalid_model_exits_with_error() {
    // Model that doesn't start with "gemini" or "gpt-image" → detect_provider() rejects it
    cmd()
        .args(["--model", "dall-e-3", "a cat"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown provider for model"));
}

#[test]
fn invalid_aspect_ratio_exits_with_error() {
    // Validation fires before any cassette is opened; no API key needed
    cmd()
        .args(["--model", "nano-banana", "--aspect-ratio", "100:200", "a cat"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported aspect ratio"));
}

#[test]
fn invalid_format_exits_with_error() {
    cmd()
        .args(["--model", "nano-banana", "--format", "gif", "a cat"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported format"));
}

#[test]
fn invalid_quality_exits_with_error() {
    cmd()
        .args(["--model", "nano-banana", "--quality", "ultra", "a cat"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported quality"));
}
