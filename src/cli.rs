//! CLI argument parsing with clap.

use clap::Parser;

/// AI image generation CLI - unified interface for Gemini and `OpenAI`.
#[derive(Parser, Debug)]
#[command(name = "imagen", version, about)]
pub struct Cli {
    /// Text prompt describing the desired image.
    #[arg(conflicts_with = "prompt_file")]
    pub prompt: Option<String>,

    /// Path to a file containing the prompt text.
    #[arg(short = 'p', long, conflicts_with = "prompt")]
    pub prompt_file: Option<String>,

    /// Model name or short alias.
    #[arg(short, long, default_value = "nano-banana")]
    pub model: String,

    /// Aspect ratio (e.g., 1:1, 16:9, 9:16).
    #[arg(short, long, default_value = "1:1")]
    pub aspect_ratio: String,

    /// Image size: 1K, 2K, 4K.
    #[arg(short, long, default_value = "1K")]
    pub size: String,

    /// Quality (`ChatGPT` only): auto, low, medium, high.
    #[arg(short, long, default_value = "auto")]
    pub quality: String,

    /// Output format: jpeg, png, webp.
    #[arg(short, long, default_value = "jpeg")]
    pub format: String,

    /// Output file path (auto-generated if not specified).
    #[arg(short, long)]
    pub output: Option<String>,

    /// Number of images to generate.
    #[arg(short = 'n', long, default_value = "1")]
    pub count: u32,

    /// Config file path override.
    #[arg(long)]
    pub config: Option<String>,

    /// Thinking level (Gemini only): none, minimal, low, medium, high.
    #[arg(short, long)]
    pub thinking: Option<String>,

    /// Verbose output.
    #[arg(short, long)]
    pub verbose: bool,
}

impl Cli {
    /// Resolve the prompt from either the positional argument or the file flag.
    ///
    /// # Errors
    ///
    /// Returns an error if neither prompt nor prompt-file is provided,
    /// or if the file cannot be read.
    pub fn resolve_prompt(&self) -> Result<String, std::io::Error> {
        if let Some(ref text) = self.prompt {
            Ok(text.clone())
        } else if let Some(ref path) = self.prompt_file {
            std::fs::read_to_string(path)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Provide a prompt string or use -p/--prompt-file",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_prompt() {
        let cli = Cli::parse_from(["imagen", "a cat"]);
        assert_eq!(cli.prompt.as_deref(), Some("a cat"));
        assert!(cli.prompt_file.is_none());
        assert_eq!(cli.resolve_prompt().unwrap(), "a cat");
    }

    #[test]
    fn prompt_file_flag() {
        let dir = std::env::temp_dir().join("imagen_cli_pf_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("prompt.txt");
        std::fs::write(&path, "prompt from file").unwrap();

        let cli = Cli::parse_from(["imagen", "-p", path.to_str().unwrap()]);
        assert!(cli.prompt.is_none());
        assert!(cli.prompt_file.is_some());
        assert_eq!(cli.resolve_prompt().unwrap(), "prompt from file");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_values() {
        let cli = Cli::parse_from(["imagen", "a cat"]);
        assert_eq!(cli.model, "nano-banana");
        assert_eq!(cli.aspect_ratio, "1:1");
        assert_eq!(cli.size, "1K");
        assert_eq!(cli.quality, "auto");
        assert_eq!(cli.format, "jpeg");
        assert!(cli.output.is_none());
        assert_eq!(cli.count, 1);
        assert!(!cli.verbose);
    }

    #[test]
    fn all_options() {
        let cli = Cli::parse_from([
            "imagen",
            "-m",
            "gpt-1",
            "-a",
            "16:9",
            "-s",
            "4K",
            "-q",
            "high",
            "-f",
            "png",
            "-o",
            "out.png",
            "-n",
            "3",
            "-v",
            "a landscape",
        ]);
        assert_eq!(cli.model, "gpt-1");
        assert_eq!(cli.aspect_ratio, "16:9");
        assert_eq!(cli.size, "4K");
        assert_eq!(cli.quality, "high");
        assert_eq!(cli.format, "png");
        assert_eq!(cli.output.as_deref(), Some("out.png"));
        assert_eq!(cli.count, 3);
        assert!(cli.verbose);
        assert_eq!(cli.prompt.as_deref(), Some("a landscape"));
    }

    #[test]
    fn no_prompt_errors() {
        let cli = Cli::parse_from(["imagen"]);
        assert!(cli.resolve_prompt().is_err());
    }
}
