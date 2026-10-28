//! Starting the llama.cpp server Sieve talks to, when the configuration names a model rather than
//! the URL of a server already running.

use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use sieve_core::config::Config;
use sieve_core::llm::LlamaServer;

/// The running server, stopped when dropped.
pub struct Llama {
    pub client: Arc<LlamaServer>,
    child: Option<Child>,
}

impl Drop for Llama {
    fn drop(&mut self) {
        if let Some(c) = self.child.as_mut() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

pub fn start(cfg: &Config) -> Result<Llama> {
    let client = Arc::new(LlamaServer::new(&cfg.llm.url(), &cfg.llm.model_name()));
    if cfg.llm.url.is_some() || client.healthy() {
        // Someone else runs it (or it is still running from before).
        return Ok(Llama { client, child: None });
    }
    anyhow::ensure!(!cfg.llm.model.is_empty(), "no language model configured (llm.model in config.json)");
    let mut cmd = Command::new(&cfg.llm.server);
    cmd.args([
        "-m",
        &cfg.llm.model,
        "--host",
        "127.0.0.1",
        "--port",
        &cfg.llm.port.to_string(),
        "-c",
        &cfg.llm.context.to_string(),
        "-ngl",
        &cfg.llm.gpu_layers.to_string(),
        "--no-ui",
        "-np",
        "2",
    ]);
    // On a machine with an NVIDIA GPU, use it explicitly (the prebuilt CUDA server otherwise may not).
    if cfg.llm.gpu_layers > 0 && std::env::var("SIEVE_CPU_ONLY").is_err() {
        if let Ok(out) = Command::new(&cfg.llm.server).arg("--list-devices").output() {
            if String::from_utf8_lossy(&out.stdout).contains("CUDA0") || String::from_utf8_lossy(&out.stderr).contains("CUDA0") {
                cmd.args(["-dev", "CUDA0"]);
            }
        }
    }
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    let child = cmd.spawn().with_context(|| format!("cannot start {}", cfg.llm.server))?;
    let llama = Llama { client, child: Some(child) };
    llama.client.wait_ready(Duration::from_secs(180))?;
    Ok(llama)
}
