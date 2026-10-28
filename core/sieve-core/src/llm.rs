//! The local model: a llama.cpp server (`llama-server`) on 127.0.0.1, spoken to through its
//! OpenAI-style chat endpoint, with answers constrained by a JSON schema (a grammar) when asked.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

/// What the rest of Sieve needs from a model (a trait, so tests can use a scripted one).
pub trait Model: Send + Sync {
    /// The model's answer to a system and a user message; with `schema`, a JSON text matching it.
    fn chat(&self, system: &str, user: &str, schema: Option<&Value>, max_tokens: u32) -> Result<String>;
    fn name(&self) -> String;
}

pub struct LlamaServer {
    url: String,
    name: String,
    http: reqwest::blocking::Client,
}

impl LlamaServer {
    pub fn new(url: &str, name: &str) -> LlamaServer {
        LlamaServer {
            url: url.trim_end_matches('/').to_string(),
            name: name.to_string(),
            http: reqwest::blocking::Client::builder().timeout(Duration::from_secs(600)).build().expect("an HTTP client"),
        }
    }

    pub fn healthy(&self) -> bool {
        self.http
            .get(format!("{}/health", self.url))
            .timeout(Duration::from_secs(3))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Waits until the server answers (it loads the model first), up to `timeout`.
    pub fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let t0 = Instant::now();
        while t0.elapsed() < timeout {
            if self.healthy() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        bail!("the language model server at {} did not start", self.url)
    }
}

impl Model for LlamaServer {
    fn chat(&self, system: &str, user: &str, schema: Option<&Value>, max_tokens: u32) -> Result<String> {
        let mut body = json!({
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}],
            "temperature": 0,
            "max_tokens": max_tokens,
            "cache_prompt": true,
        });
        if let Some(s) = schema {
            body["response_format"] = json!({"type": "json_schema", "json_schema": {"name": "answer", "schema": s}});
        }
        let r: Value = self
            .http
            .post(format!("{}/v1/chat/completions", self.url))
            .json(&body)
            .send()
            .with_context(|| format!("the language model server at {} is not answering", self.url))?
            .error_for_status()?
            .json()?;
        r["choices"][0]["message"]["content"].as_str().map(|s| s.to_string()).context("the language model returned no text")
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// Parses a model's JSON answer, tolerating text around it.
pub fn parse_json(text: &str) -> Result<Value> {
    if let Ok(v) = serde_json::from_str(text) {
        return Ok(v);
    }
    let (a, b) = (text.find('{'), text.rfind('}'));
    match (a, b) {
        (Some(a), Some(b)) if b > a => Ok(serde_json::from_str(&text[a..=b])?),
        _ => bail!("the model's answer is not JSON: {}", text.chars().take(200).collect::<String>()),
    }
}

/// A model that answers from a script: (a substring of the user message, the answer). For tests and
/// for checking the pipeline without a model.
pub struct Scripted(pub Vec<(String, String)>);

impl Model for Scripted {
    fn chat(&self, _system: &str, user: &str, _schema: Option<&Value>, _max_tokens: u32) -> Result<String> {
        self.0.iter().find(|(k, _)| user.contains(k.as_str())).map(|(_, v)| v.clone()).context("no scripted answer")
    }
    fn name(&self) -> String {
        "scripted".into()
    }
}

/// Truncates text to about `max` characters at a word boundary.
pub fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let s: String = text.chars().take(max).collect();
    match s.rfind(char::is_whitespace) {
        Some(i) if i > max / 2 => format!("{} […]", &s[..i]),
        _ => format!("{s} […]"),
    }
}
