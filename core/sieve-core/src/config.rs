//! Settings, in `config.json` in the data folder (%APPDATA%\Sieve on Windows, or SIEVE_HOME).
//! Passwords are never here: they live in the system's credential store.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The llama.cpp server: Sieve starts `server` with `model` on `port`, unless `url` points to
    /// one already running.
    pub llm: LlmConfig,
    /// Seconds between checks for new mail (IDLE wakes it sooner when the server supports it).
    pub sync_interval: u64,
    /// How many recent messages per folder the first sync fetches.
    pub initial_messages: u32,
    /// Remind after this many days without an answer.
    pub followup_days: i64,
    /// Threads with at least this many messages get a summary.
    pub summary_min_messages: usize,
    /// Port of the local API the app uses (127.0.0.1 only).
    pub api_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    pub url: Option<String>,
    pub server: String,
    pub model: String,
    pub port: u16,
    pub gpu_layers: i32,
    pub context: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        LlmConfig { url: None, server: "llama-server".into(), model: String::new(), port: 8711, gpu_layers: 99, context: 8192 }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            llm: LlmConfig::default(),
            sync_interval: 60,
            initial_messages: 500,
            followup_days: 3,
            summary_min_messages: 4,
            api_port: 7878,
        }
    }
}

impl LlmConfig {
    pub fn url(&self) -> String {
        self.url.clone().unwrap_or_else(|| format!("http://127.0.0.1:{}", self.port))
    }

    pub fn model_name(&self) -> String {
        Path::new(&self.model).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "llama-server".into())
    }
}

/// The data folder: SIEVE_HOME, or the platform's (e.g. %APPDATA%\Sieve).
pub fn data_dir() -> PathBuf {
    if let Ok(h) = std::env::var("SIEVE_HOME") {
        return PathBuf::from(h);
    }
    directories::ProjectDirs::from("", "", "Sieve").map(|d| d.data_dir().to_path_buf()).unwrap_or_else(|| PathBuf::from(".sieve"))
}

impl Config {
    pub fn load(dir: &Path) -> Result<Config> {
        let p = dir.join("config.json");
        if !p.exists() {
            return Ok(Config::default());
        }
        let text = std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
        serde_json::from_str(&text).with_context(|| format!("{} is not valid", p.display()))
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join("config.json"), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}
