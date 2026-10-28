//! Messages as JSON lines: the format the evaluation reads and writes (one message per line).

use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sieve_core::Message;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub id: String,
    pub from: String,
    pub from_name: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub date: i64,
    pub body: String,
    pub attachments: Vec<String>,
    pub list_unsubscribe: bool,
    pub lang: String,
    pub label: Option<String>,
    /// For draft evaluation: the message it answers, and its thread.
    pub thread: String,
    pub in_reply_to: String,
    pub message_id: String,
    /// The mailbox's owner (whose inbox it is).
    pub owner: String,
}

impl Record {
    pub fn to_message(&self, id: i64) -> Message {
        let lang = if self.lang.is_empty() {
            sieve_core::lang::detect(&format!("{}\n{}", self.subject, self.body)).to_string()
        } else {
            self.lang.clone()
        };
        Message {
            id,
            account_id: 1,
            folder: "INBOX".into(),
            uid: id as u32,
            message_id: if self.message_id.is_empty() { format!("{}@bench", self.id) } else { self.message_id.clone() },
            in_reply_to: self.in_reply_to.clone(),
            thread_id: if self.thread.is_empty() { self.id.clone() } else { self.thread.clone() },
            from_addr: self.from.to_lowercase(),
            from_name: self.from_name.clone(),
            to: self.to.clone(),
            cc: self.cc.clone(),
            subject: self.subject.clone(),
            date: self.date,
            body: self.body.clone(),
            lang,
            list_unsubscribe: self.list_unsubscribe,
            attachments: self.attachments.clone(),
            outgoing: !self.owner.is_empty() && self.from.eq_ignore_ascii_case(&self.owner),
            ..Default::default()
        }
    }

    pub fn from_message(m: &Message, label: Option<&str>) -> Record {
        Record {
            id: m.id.to_string(),
            from: m.from_addr.clone(),
            from_name: m.from_name.clone(),
            to: m.to.clone(),
            cc: m.cc.clone(),
            subject: m.subject.clone(),
            date: m.date,
            body: m.body.clone(),
            attachments: m.attachments.clone(),
            list_unsubscribe: m.list_unsubscribe,
            lang: m.lang.clone(),
            label: label.map(|s| s.to_string()),
            thread: m.thread_id.clone(),
            in_reply_to: m.in_reply_to.clone(),
            message_id: m.message_id.clone(),
            owner: String::new(),
        }
    }
}

pub fn read(path: &Path) -> Result<Vec<Record>> {
    let f = std::fs::File::open(path).with_context(|| format!("reading {}", path.display()))?;
    let mut out = vec![];
    for (i, line) in std::io::BufReader::new(f).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str(&line).with_context(|| format!("{}:{}", path.display(), i + 1))?);
    }
    Ok(out)
}

pub fn write<T: Serialize>(path: &Path, rows: &[T]) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    for r in rows {
        serde_json::to_writer(&mut f, r)?;
        f.write_all(b"\n")?;
    }
    Ok(())
}
