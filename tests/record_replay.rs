//! Cassette replay integration tests — zero network I/O.
//!
//! All tests set `IMAGEN_REPLAY` to a cassette file path so that the binary
//! never contacts a live API endpoint.

use assert_cmd::Command;
use base64::Engine;
use predicates::prelude::*;
use std::path::PathBuf;

fn cmd() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("imagen")
}

/// Absolute path to the `test_fixtures` directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_fixtures")
}

#[test]
fn gemini_happy_path_creates_file() {
    let cassette = fixtures_dir().join("gemini_cat.cassette.yaml");
    let out = std::env::temp_dir().join("imagen_test_gemini_happy.jpg");
    let _ = std::fs::remove_file(&out);

    cmd()
        .env("IMAGEN_REPLAY", cassette.to_str().unwrap())
        .env_remove("GEMINI_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args(["--model", "nano-banana", "--output", out.to_str().unwrap(), "a cat"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Saved:"));

    assert!(out.exists(), "Output file should have been created");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn openai_happy_path_creates_file() {
    let cassette = fixtures_dir().join("openai_cat.cassette.yaml");
    let out = std::env::temp_dir().join("imagen_test_openai_happy.jpg");
    let _ = std::fs::remove_file(&out);

    cmd()
        .env("IMAGEN_REPLAY", cassette.to_str().unwrap())
        .env_remove("GEMINI_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args(["--model", "gpt-1", "--output", out.to_str().unwrap(), "a cat"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Saved:"));

    assert!(out.exists(), "Output file should have been created");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn auto_filename_uses_kebab_case_with_timestamp() {
    let cassette = fixtures_dir().join("gemini_cat.cassette.yaml");
    let work_dir = std::env::temp_dir().join("imagen_test_autofile");
    std::fs::create_dir_all(&work_dir).unwrap();
    // Remove any leftover files from a previous run
    for entry in std::fs::read_dir(&work_dir).unwrap().flatten() {
        let _ = std::fs::remove_file(entry.path());
    }

    cmd()
        .env("IMAGEN_REPLAY", cassette.to_str().unwrap())
        .env_remove("GEMINI_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args(["--model", "nano-banana", "a cat"])
        .current_dir(&work_dir)
        .assert()
        .success();

    // Auto-generated filename: "a-cat-<timestamp>.jpg"
    let files: Vec<_> = std::fs::read_dir(&work_dir).unwrap().flatten().collect();
    assert_eq!(files.len(), 1, "Exactly one file should be created");
    let name = files[0].file_name();
    let name = name.to_string_lossy();
    assert!(name.starts_with("a-cat-"), "Filename should start with 'a-cat-', got: {name}");
    assert!(name.ends_with(".jpg"), "Filename should end with .jpg, got: {name}");

    let _ = std::fs::remove_dir_all(&work_dir);
}

#[test]
fn format_png_converts_jpeg_to_png() {
    // Generate a real 1×1 JPEG using the image crate, embed it in a temporary
    // cassette, and verify that --format png produces a valid PNG file.
    let jpeg_bytes = {
        let img = image::DynamicImage::new_rgb8(1, 1);
        let mut buf = std::io::Cursor::new(Vec::<u8>::new());
        img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();
        buf.into_inner()
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes);

    let cassette_content = format!(
        "name: convert-test\nrecorded_at: \"2026-02-01T00:00:00Z\"\ncommit: test\ninteractions:\n  - seq: 0\n    port: image_generator\n    method: generate\n    input: {{}}\n    output:\n      Ok:\n        images:\n          - data: {b64}\n            mime_type: image/jpeg\n"
    );

    let cassette_path = std::env::temp_dir().join("imagen_test_convert.cassette.yaml");
    std::fs::write(&cassette_path, &cassette_content).unwrap();

    let out = std::env::temp_dir().join("imagen_test_convert_output.png");
    let _ = std::fs::remove_file(&out);

    cmd()
        .env("IMAGEN_REPLAY", cassette_path.to_str().unwrap())
        .env_remove("GEMINI_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .args([
            "--model",
            "nano-banana",
            "--format",
            "png",
            "--output",
            out.to_str().unwrap(),
            "a cat",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Saved:"));

    assert!(out.exists(), "PNG output file should have been created");
    // Verify the output starts with the PNG magic bytes
    let data = std::fs::read(&out).unwrap();
    assert_eq!(
        &data[..8],
        &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        "Output should be a valid PNG file"
    );

    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&cassette_path);
}
