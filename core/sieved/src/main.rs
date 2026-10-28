//! sieved: Sieve's background service. Syncs your mail, sorts it with the local model, watches for
//! answers you are waiting on, and serves the desktop app on 127.0.0.1.

use anyhow::{Context, Result};
use rand::RngCore;
use sieve_core::config::{Config, data_dir};
use sieve_core::store::Store;
use sieved::{api, daemon, llama};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let cfg = Config::load(&dir)?;
    if !dir.join("config.json").exists() {
        cfg.save(&dir)?;
    }

    // A fresh token every start: the app reads it from the data folder.
    let mut bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut bytes);
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    std::fs::write(dir.join("api-token"), &token)?;

    tracing::info!("data in {}", dir.display());
    let llama = tokio::task::spawn_blocking({
        let cfg = cfg.clone();
        move || llama::start(&cfg)
    })
    .await??;
    tracing::info!("model {} ready at {}", cfg.llm.model_name(), cfg.llm.url());

    let db = dir.join("sieve.db");
    let state = daemon::State::new(Store::open(&db)?, Some(db), llama.client.clone(), cfg.clone());
    std::fs::write(dir.join("sieved.pid"), std::process::id().to_string())?;
    {
        let s = state.clone();
        std::thread::spawn(move || daemon::run(s));
    }
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", cfg.api_port))
        .await
        .with_context(|| format!("port {} is taken (is sieved already running?)", cfg.api_port))?;
    tracing::info!("API on http://127.0.0.1:{}", cfg.api_port);
    axum::serve(listener, api::router(state, token))
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;
    // `llama` lives until here, so the model server stops with the daemon.
    drop(llama);
    Ok(())
}
