//! Sieve's core: a local copy of your mail (IMAP into SQLite), and what a local language model does
//! with it — triage into four piles, summaries of long threads, reply drafts in your style, and
//! reminders when someone has not answered you. Nothing leaves the computer: the model runs in a
//! llama.cpp server on 127.0.0.1, and mail is sent only by an explicit, confirmed request.

pub mod config;
pub mod drafts;
pub mod followup;
pub mod imap_sync;
pub mod lang;
pub mod llm;
pub mod mail;
pub mod model;
pub mod send;
pub mod store;
pub mod style;
pub mod summary;
pub mod triage;

pub use model::{Category, Message};
pub use store::Store;
