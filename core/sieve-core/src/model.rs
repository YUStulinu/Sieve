//! The things Sieve keeps: messages, their triage, drafts, follow-ups.

use serde::{Deserialize, Serialize};

/// The four piles of the inbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Someone is waiting for you: a question, a request, a decision, a meeting to confirm.
    ReplyToday,
    /// Worth reading, nothing to do: updates, confirmations, notifications, FYIs.
    Informative,
    /// Bulk mail you subscribed to: newsletters, digests, promotions.
    Newsletter,
    /// Money: invoices, bills, receipts, payment reminders.
    Invoice,
}

impl Category {
    pub const ALL: [Category; 4] = [Category::ReplyToday, Category::Informative, Category::Newsletter, Category::Invoice];

    pub fn as_str(self) -> &'static str {
        match self {
            Category::ReplyToday => "reply_today",
            Category::Informative => "informative",
            Category::Newsletter => "newsletter",
            Category::Invoice => "invoice",
        }
    }

    pub fn parse(s: &str) -> Option<Category> {
        Category::ALL.into_iter().find(|c| c.as_str() == s)
    }
}

/// A message as stored: headers that matter, the readable text, and where it lives on the server.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub account_id: i64,
    pub folder: String,
    pub uid: u32,
    pub message_id: String,
    pub in_reply_to: String,
    pub references: Vec<String>,
    pub thread_id: String,
    pub from_addr: String,
    pub from_name: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    /// Unix seconds.
    pub date: i64,
    /// The text the reader sees: plain part (or HTML turned into text), without quoted replies.
    pub body: String,
    pub lang: String,
    /// The sender offers List-Unsubscribe (bulk mail).
    pub list_unsubscribe: bool,
    pub attachments: Vec<String>,
    /// Sent by the account's owner (it is in the Sent folder, or from their address).
    pub outgoing: bool,
    pub seen: bool,
}

/// The model's verdict on a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Triage {
    pub message_id: i64,
    pub category: Category,
    pub reason: String,
    pub model: String,
    pub prompt: String,
    pub ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    pub id: i64,
    /// The message it answers.
    pub reply_to: i64,
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    /// proposed, saved (in the server's Drafts), sent, discarded
    pub status: String,
    pub created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUp {
    pub id: i64,
    /// The message you sent.
    pub message_id: i64,
    pub thread_id: String,
    pub subject: String,
    /// Who should answer.
    pub waiting_on: String,
    /// What you are waiting for, in a few words.
    pub what: String,
    pub sent: i64,
    /// When to remind you (unix seconds).
    pub due: i64,
    /// waiting, replied, dismissed
    pub status: String,
}
