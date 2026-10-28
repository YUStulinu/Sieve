//! Summaries of long threads: what it is about, what was decided, what is still open, and who is
//! waiting for whom — in the thread's language.

use anyhow::Result;
use serde_json::{Value, json};

use crate::llm::{Model, clip, parse_json};
use crate::model::Message;

/// Characters of thread text shown to the model (the newest messages win when it is longer).
pub const BUDGET: usize = 9000;

pub fn render(thread: &[Message]) -> String {
    let mut parts: Vec<String> = thread
        .iter()
        .map(|m| {
            let when = chrono::DateTime::from_timestamp(m.date, 0).map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default();
            let who = if m.from_name.is_empty() { m.from_addr.clone() } else { format!("{} <{}>", m.from_name, m.from_addr) };
            format!("--- {when} — {who}\n{}", clip(&m.body, 1500))
        })
        .collect();
    // Drop the oldest messages until it fits, keeping the first (it says what the thread is about).
    while parts.iter().map(|p| p.len()).sum::<usize>() > BUDGET && parts.len() > 2 {
        parts.remove(1);
    }
    format!("Subject: {}\n\n{}", thread.first().map(|m| m.subject.as_str()).unwrap_or(""), parts.join("\n\n"))
}

pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": {"type": "string"},
            "decisions": {"type": "array", "items": {"type": "string"}, "maxItems": 5},
            "open_questions": {"type": "array", "items": {"type": "string"}, "maxItems": 5},
            "waiting_on": {"type": "string"}
        },
        "required": ["summary", "decisions", "open_questions", "waiting_on"]
    })
}

const SYSTEM: &str = "You summarize an email thread for one of its participants, who is busy. Write in the thread's language \
(Romanian or English). summary: two to four sentences — what the thread is about and where it stands now. decisions: what was \
agreed (empty if nothing). open_questions: what is still unanswered or undecided (empty if nothing). waiting_on: who has to \
act next and on what, in one short phrase (empty if nobody). Use only facts from the thread; names, dates and amounts exactly \
as written.";

pub fn summarize(model: &dyn Model, thread: &[Message]) -> Result<Value> {
    let out = model.chat(SYSTEM, &render(thread), Some(&schema()), 500)?;
    parse_json(&out)
}
