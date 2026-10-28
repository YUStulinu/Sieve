//! Triage: which of the four piles a message goes to. The model reads the sender, the subject, a
//! few header facts that hint at bulk mail or money (unless turned off), and the start of the text,
//! and answers in a JSON schema whose `category` can only be one of the four.

use std::time::Instant;

use anyhow::Result;
use serde_json::{Value, json};

use crate::llm::{Model, clip, parse_json};
use crate::model::{Category, Message, Triage};

pub const DEFAULT_SYSTEM: &str = include_str!("../prompts/triage.txt");

/// How the question is put to the model (the evaluation compares these).
#[derive(Debug, Clone)]
pub struct TriagePrompt {
    pub system: String,
    /// The model writes a one-line reason before the category (a little thinking first).
    pub reason_first: bool,
    /// Header facts: List-Unsubscribe, no-reply senders, attachment names, whether the reader is in To.
    pub hints: bool,
    /// Characters of the body shown.
    pub body_chars: usize,
    /// A name for this variant, kept with each verdict.
    pub name: String,
}

impl Default for TriagePrompt {
    fn default() -> Self {
        TriagePrompt { system: DEFAULT_SYSTEM.into(), reason_first: true, hints: true, body_chars: 1500, name: "default".into() }
    }
}

fn is_noreply(addr: &str) -> bool {
    let a = addr.to_lowercase();
    ["noreply", "no-reply", "donotreply", "do-not-reply", "notifications@", "newsletter", "mailer", "info@", "news@", "marketing"]
        .iter()
        .any(|k| a.contains(k))
}

fn mentions_unsubscribe(body: &str) -> bool {
    let b = body.to_lowercase();
    ["unsubscribe", "dezabon", "to be removed from", "opt out", "opt-out", "manage your subscription", "manage preferences"]
        .iter()
        .any(|k| b.contains(k))
}

/// The user message: the email as the model sees it.
pub fn render(m: &Message, reader: &str, p: &TriagePrompt) -> String {
    let mut s = String::new();
    s.push_str(&format!("From: {} <{}>\n", m.from_name, m.from_addr));
    if !m.to.is_empty() {
        s.push_str(&format!("To: {}\n", m.to.iter().take(6).cloned().collect::<Vec<_>>().join(", ")));
    }
    if !m.cc.is_empty() {
        s.push_str(&format!("Cc: {}\n", m.cc.iter().take(6).cloned().collect::<Vec<_>>().join(", ")));
    }
    s.push_str(&format!("Subject: {}\n", m.subject));
    if p.hints {
        let mut h = vec![];
        if m.list_unsubscribe {
            h.push("it has a List-Unsubscribe header (sent to a mailing list)".to_string());
        } else if mentions_unsubscribe(&m.body) {
            h.push("the text offers a way to unsubscribe".to_string());
        }
        if is_noreply(&m.from_addr) {
            h.push("the sender address looks automated".to_string());
        }
        if !m.attachments.is_empty() {
            h.push(format!("attachments: {}", m.attachments.iter().take(5).cloned().collect::<Vec<_>>().join(", ")));
        }
        let domain = |a: &str| a.rsplit_once('@').map(|(_, d)| d.to_lowercase()).unwrap_or_default();
        if !reader.is_empty() && !m.from_addr.is_empty() && domain(reader) == domain(&m.from_addr) && !is_noreply(&m.from_addr) {
            h.push("the sender is from the reader's own organization (same domain)".to_string());
        }
        if !reader.is_empty() {
            let direct = m.to.iter().any(|a| a.eq_ignore_ascii_case(reader));
            let cc = m.cc.iter().any(|a| a.eq_ignore_ascii_case(reader));
            if cc && !direct {
                h.push("the reader is only in Cc".to_string());
            } else if !direct && !m.to.is_empty() {
                h.push("the reader is not among the recipients (a list or Bcc)".to_string());
            }
        }
        if !h.is_empty() {
            s.push_str(&format!("Facts: {}\n", h.join("; ")));
        }
    }
    s.push('\n');
    s.push_str(&clip(&m.body, p.body_chars));
    s
}

pub fn schema(p: &TriagePrompt) -> Value {
    let cats: Vec<&str> = Category::ALL.iter().map(|c| c.as_str()).collect();
    if p.reason_first {
        json!({
            "type": "object",
            "properties": {"reason": {"type": "string"}, "category": {"type": "string", "enum": cats}},
            "required": ["reason", "category"]
        })
    } else {
        json!({
            "type": "object",
            "properties": {"category": {"type": "string", "enum": cats}, "reason": {"type": "string"}},
            "required": ["category", "reason"]
        })
    }
}

pub fn classify(model: &dyn Model, m: &Message, reader: &str, p: &TriagePrompt) -> Result<Triage> {
    let t0 = Instant::now();
    let system = format!(
        "{}\nAnswer in JSON: {}.",
        p.system.trim(),
        if p.reason_first {
            "first \"reason\" (one short sentence: what the message asks of the reader), then \"category\""
        } else {
            "\"category\", then \"reason\" (one short sentence)"
        }
    );
    let out = model.chat(&system, &render(m, reader, p), Some(&schema(p)), 120)?;
    let v = parse_json(&out)?;
    let category = v["category"].as_str().and_then(Category::parse).unwrap_or(Category::Informative);
    Ok(Triage {
        message_id: m.id,
        category,
        reason: v["reason"].as_str().unwrap_or("").to_string(),
        model: model.name(),
        prompt: p.name.clone(),
        ms: t0.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Scripted;

    fn msg() -> Message {
        Message {
            from_name: "Orange".into(),
            from_addr: "no-reply@orange.ro".into(),
            to: vec!["ana@sieve.test".into()],
            subject: "Factura ta din octombrie".into(),
            body: "Factura în valoare de 54,20 lei este disponibilă.".into(),
            attachments: vec!["factura.pdf".into()],
            list_unsubscribe: true,
            ..Default::default()
        }
    }

    #[test]
    fn hints_describe_the_headers() {
        let r = render(&msg(), "ana@sieve.test", &TriagePrompt::default());
        assert!(r.contains("List-Unsubscribe"));
        assert!(r.contains("looks automated"));
        assert!(r.contains("factura.pdf"));
        let plain = render(&msg(), "ana@sieve.test", &TriagePrompt { hints: false, ..Default::default() });
        assert!(!plain.contains("Facts:"));
    }

    #[test]
    fn colleagues_are_noted() {
        let mut m = msg();
        m.from_addr = "elena@studionord.ro".into();
        assert!(render(&m, "ana@studionord.ro", &TriagePrompt::default()).contains("own organization"));
        assert!(!render(&m, "ana@gmail.com", &TriagePrompt::default()).contains("own organization"));
    }

    #[test]
    fn reader_only_in_cc_is_noted() {
        let mut m = msg();
        m.to = vec!["dan@x.ro".into()];
        m.cc = vec!["ana@sieve.test".into()];
        assert!(render(&m, "ana@sieve.test", &TriagePrompt::default()).contains("only in Cc"));
    }

    #[test]
    fn the_answer_becomes_a_verdict() {
        let model = Scripted(vec![("Factura".into(), r#"{"reason":"A phone bill.","category":"invoice"}"#.into())]);
        let t = classify(&model, &msg(), "ana@sieve.test", &TriagePrompt::default()).unwrap();
        assert_eq!(t.category, Category::Invoice);
        assert_eq!(t.reason, "A phone bill.");
        let bad = Scripted(vec![("Factura".into(), r#"{"reason":"?","category":"spam"}"#.into())]);
        assert_eq!(classify(&bad, &msg(), "", &TriagePrompt::default()).unwrap().category, Category::Informative);
    }
}
