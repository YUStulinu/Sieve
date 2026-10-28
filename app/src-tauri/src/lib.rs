//! The desktop app's native side: it finds Sieve's daemon (and starts it if it is not running) and
//! tells the interface where the local API is and which token it needs. Everything else the
//! interface asks the daemon directly.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use serde::Serialize;
use sieve_core::config::{Config, data_dir};

#[derive(Serialize)]
struct Connection {
    url: String,
    token: String,
    data_dir: String,
}

fn api_up(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_millis(300)).is_ok()
}

/// The daemon's program: SIEVED, next to the app, or in the build folder during development.
fn sieved() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SIEVED") {
        return Some(PathBuf::from(p));
    }
    let exe = if cfg!(windows) { "sieved.exe" } else { "sieved" };
    let here = std::env::current_exe().ok()?.parent()?.to_path_buf();
    [here.join(exe), here.join("../../../target/release").join(exe), here.join("../../../target/debug").join(exe)]
        .into_iter()
        .find(|p| p.exists())
}

#[tauri::command]
fn connection() -> Result<Connection, String> {
    let dir = data_dir();
    let cfg = Config::load(&dir).map_err(|e| e.to_string())?;
    if !api_up(cfg.api_port) {
        let exe = sieved().ok_or("Sieve's background service (sieved) was not found next to the app")?;
        let mut cmd = Command::new(exe);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        cmd.spawn().map_err(|e| format!("cannot start sieved: {e}"))?;
        // It loads the model first: up to a few minutes on a slow disk.
        let t0 = Instant::now();
        while !api_up(cfg.api_port) {
            if t0.elapsed() > Duration::from_secs(240) {
                return Err("sieved did not start; see its log".into());
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    let token = std::fs::read_to_string(dir.join("api-token")).map_err(|e| format!("no API token: {e}"))?;
    Ok(Connection {
        url: format!("http://127.0.0.1:{}", cfg.api_port),
        token: token.trim().to_string(),
        data_dir: dir.display().to_string(),
    })
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![connection])
        .run(tauri::generate_context!())
        .expect("error while running Sieve");
}
