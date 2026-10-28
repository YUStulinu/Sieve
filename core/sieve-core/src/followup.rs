//! Follow-ups: messages you sent that expect an answer. Sieve notes who should answer and what you
//! asked; when nobody in the thread has written back by the due date (a few working days later), it
//! reminds you. An answer in the thread closes the follow-up by itself.

use std::sync::LazyLock;

use anyhow::Result;
use chrono::{Datelike, Weekday};
use regex::Regex;
use serde_json::{Value, json};

use crate::llm::{Model, clip, parse_json};
use crate::model::{FollowUp, Message};
use crate::store::{Store, now};

/// Cheap first pass: a question mark, or a request or deadline phrase (Romanian or English). Only
/// these go to the model.
static ASKS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\?|\b(please|could you|can you|would you|let me know|waiting for|looking forward|by (monday|tuesday|wednesday|thursday|friday|tomorrow|end of)|deadline|asap|te rog|vă rog|va rog|poți|poti|puteți|puteti|aștept|astept|spune-mi|spuneți-mi|anunță-mă|anunta-ma|până (luni|marți|miercuri|joi|vineri|mâine)|pana (luni|marti|miercuri|joi|vineri|maine)|confirmă|confirma|confirmați)\b",
    )
    .unwrap()
});

pub fn might_ask(body: &str) -> bool {
    ASKS.is_match(body)
}

pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "expects_reply": {"type": "boolean"},
            "waiting_on": {"type": "string"},
            "what": {"type": "string"}
        },
        "required": ["expects_reply", "waiting_on", "what"]
    })
}

pub const SYSTEM: &str = "You read an email the user sent and decide whether they are now waiting for an answer from the \
recipient: they asked a question, requested something (a document, a decision, a payment, a confirmation, a date), or \
proposed something that needs a yes or no. A message that only informs, thanks, confirms, or answers someone else's \
question expects no reply. waiting_on: the name (or address) of who should answer. what: what the user is waiting for, in \
at most eight words, in the message's language. If expects_reply is false, waiting_on and what are empty.";

#[derive(Debug, Clone, PartialEq)]
pub struct Expectation {
    pub expects_reply: bool,
    pub waiting_on: String,
    pub what: String,
}

pub fn render(m: &Message) -> String {
    format!("To: {}\nSubject: {}\n\n{}", m.to.join(", "), m.subject, clip(&m.body, 2000))
}

/// Does this sent message expect an answer? The regex screens; the model decides.
pub fn expects(model: &dyn Model, m: &Message) -> Result<Expectation> {
    if !might_ask(&m.body) && !might_ask(&m.subject) {
        return Ok(Expectation { expects_reply: false, waiting_on: String::new(), what: String::new() });
    }
    let v = parse_json(&model.chat(SYSTEM, &render(m), Some(&schema()), 120)?)?;
    Ok(Expectation {
        expects_reply: v["expects_reply"].as_bool().unwrap_or(false),
        waiting_on: v["waiting_on"].as_str().unwrap_or("").to_string(),
        what: v["what"].as_str().unwrap_or("").to_string(),
    })
}

/// `days` working days after `t` (unix seconds), skipping Saturdays and Sundays.
pub fn working_days_after(t: i64, days: i64) -> i64 {
    let mut d = chrono::DateTime::from_timestamp(t, 0).unwrap_or_default();
    let mut left = days;
    while left > 0 {
        d += chrono::Duration::days(1);
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            left -= 1;
        }
    }
    d.timestamp()
}

/// Looks at sent messages not seen yet; returns how many follow-ups it opened.
pub fn scan(model: &dyn Model, store: &Store, days: i64, limit: usize) -> Result<usize> {
    let mut opened = 0;
    for m in store.unchecked_outgoing(limit)? {
        // Older than a month: too late to remind about.
        if m.date < now() - 30 * 86400 {
            store.mark_checked(m.id)?;
            continue;
        }
        let e = expects(model, &m)?;
        if e.expects_reply && !store.answered_after(&m.thread_id, m.date)? {
            store.add_followup(&FollowUp {
                id: 0,
                message_id: m.id,
                thread_id: m.thread_id.clone(),
                subject: m.subject.clone(),
                waiting_on: if e.waiting_on.is_empty() { m.to.first().cloned().unwrap_or_default() } else { e.waiting_on },
                what: e.what,
                sent: m.date,
                due: working_days_after(m.date, days),
                status: "waiting".into(),
            })?;
            opened += 1;
        } else {
            store.mark_checked(m.id)?;
        }
    }
    Ok(opened)
}

/// Closes the follow-ups whose thread got an answer.
pub fn close_answered(store: &Store) -> Result<usize> {
    let mut n = 0;
    for f in store.followups(Some("waiting"))? {
        if store.answered_after(&f.thread_id, f.sent)? {
            store.set_followup_status(f.id, "replied")?;
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_requests_in_both_languages() {
        assert!(might_ask("Poți să-mi trimiți contractul până vineri?"));
        assert!(might_ask("Te rog să confirmi data."));
        assert!(might_ask("Could you send the report by Friday"));
        assert!(!might_ask("Mulțumesc, am primit documentele."));
        assert!(!might_ask("Thanks, all received."));
    }

    #[test]
    fn working_days_skip_weekends() {
        // Thursday 2026-10-29 12:00 UTC + 3 working days = Tuesday 2026-11-03
        let thu = chrono::NaiveDate::from_ymd_opt(2026, 10, 29).unwrap().and_hms_opt(12, 0, 0).unwrap().and_utc().timestamp();
        let due = chrono::DateTime::from_timestamp(working_days_after(thu, 3), 0).unwrap();
        assert_eq!(due.date_naive(), chrono::NaiveDate::from_ymd_opt(2026, 11, 3).unwrap());
    }
}
