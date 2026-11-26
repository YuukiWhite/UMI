use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "umi", version, about = "UMI — Ambient Dynamic Island companion for KDE Plasma Wayland")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum CliCommand {
    /// Toggle notch visibility / expanded state on the running instance
    Toggle,
    /// Open Ask UMI AI prompt directly inside the notch
    Ask,
    /// Trigger Smart Screen OCR (Gemini Vision)
    Ocr,
    /// Open the UMI Settings window
    Settings,
    /// Jump directly to a notch page (music, focus, prayer, water, shelf, clip, todo, widgets, ai, limits)
    Page { name: String },
    /// Print current running state over D-Bus IPC
    Status,
    /// Save an encrypted AI provider API key into KDE Wallet (gemini, claude, openai)
    SetKey {
        provider: String,
        api_key: String,
    },
}
