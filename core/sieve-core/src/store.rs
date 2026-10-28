//! The local index: one SQLite file with the messages (and a full-text index over them), the
//! model's triage, your own labels, summaries, drafts and follow-ups.

use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::model::{Category, Draft, FollowUp, Message, Triage};

pub struct Store {
    db: Connection,
}

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    imap_host TEXT NOT NULL, imap_port INTEGER NOT NULL, imap_security TEXT NOT NULL,
    smtp_host TEXT NOT NULL, smtp_port INTEGER NOT NULL, smtp_security TEXT NOT NULL,
    username TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS folders (
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    role TEXT NOT NULL,              -- inbox, sent, drafts
    uidvalidity INTEGER NOT NULL DEFAULT 0,
    last_uid INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (account_id, name)
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    folder TEXT NOT NULL,
    uid INTEGER NOT NULL,
    message_id TEXT NOT NULL,
    in_reply_to TEXT NOT NULL,
    refs TEXT NOT NULL,              -- JSON array
    thread_id TEXT NOT NULL,
    from_addr TEXT NOT NULL,
    from_name TEXT NOT NULL,
    to_addrs TEXT NOT NULL,          -- JSON array
    cc_addrs TEXT NOT NULL,          -- JSON array
    subject TEXT NOT NULL,
    date INTEGER NOT NULL,
    body TEXT NOT NULL,
    lang TEXT NOT NULL,
    list_unsubscribe INTEGER NOT NULL,
    attachments TEXT NOT NULL,       -- JSON array
    outgoing INTEGER NOT NULL,
    seen INTEGER NOT NULL,
    UNIQUE (account_id, folder, uid)
);
CREATE INDEX IF NOT EXISTS messages_thread ON messages(thread_id, date);
CREATE INDEX IF NOT EXISTS messages_mid ON messages(message_id);
CREATE INDEX IF NOT EXISTS messages_date ON messages(date);

CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    subject, body, from_name, content = 'messages', content_rowid = 'id', tokenize = 'unicode61 remove_diacritics 2'
);
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, subject, body, from_name) VALUES (new.id, new.subject, new.body, new.from_name);
END;
CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, subject, body, from_name) VALUES ('delete', old.id, old.subject, old.body, old.from_name);
END;

CREATE TABLE IF NOT EXISTS triage (
    message_id INTEGER PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
    category TEXT NOT NULL,
    reason TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt TEXT NOT NULL,
    ms INTEGER NOT NULL,
    at INTEGER NOT NULL
);

-- Your own labels: the truth the evaluation measures the triage against.
CREATE TABLE IF NOT EXISTS labels (
    message_id INTEGER PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
    category TEXT NOT NULL,
    at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS summaries (
    thread_id TEXT PRIMARY KEY,
    messages INTEGER NOT NULL,       -- how many messages it covered (redone when the thread grows)
    summary TEXT NOT NULL,           -- JSON
    model TEXT NOT NULL,
    at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS drafts (
    id INTEGER PRIMARY KEY,
    reply_to INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    to_addrs TEXT NOT NULL,
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    status TEXT NOT NULL,            -- proposed, saved, sent, discarded
    created INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS followups (
    id INTEGER PRIMARY KEY,
    message_id INTEGER NOT NULL UNIQUE REFERENCES messages(id) ON DELETE CASCADE,
    thread_id TEXT NOT NULL,
    subject TEXT NOT NULL,
    waiting_on TEXT NOT NULL,
    what TEXT NOT NULL,
    sent INTEGER NOT NULL,
    due INTEGER NOT NULL,
    status TEXT NOT NULL
);

-- Outgoing messages already looked at for follow-ups (whether or not they expect an answer).
CREATE TABLE IF NOT EXISTS followup_checked (
    message_id INTEGER PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE
);
"#;

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".into())
}

fn vec_of(s: String) -> Vec<String> {
    serde_json::from_str(&s).unwrap_or_default()
}

pub const MESSAGE_COLUMNS: &str = "id, account_id, folder, uid, message_id, in_reply_to, refs, thread_id, from_addr, from_name, \
     to_addrs, cc_addrs, subject, date, body, lang, list_unsubscribe, attachments, outgoing, seen";

fn message_of(r: &Row) -> rusqlite::Result<Message> {
    Ok(Message {
        id: r.get(0)?,
        account_id: r.get(1)?,
        folder: r.get(2)?,
        uid: r.get(3)?,
        message_id: r.get(4)?,
        in_reply_to: r.get(5)?,
        references: vec_of(r.get(6)?),
        thread_id: r.get(7)?,
        from_addr: r.get(8)?,
        from_name: r.get(9)?,
        to: vec_of(r.get(10)?),
        cc: vec_of(r.get(11)?),
        subject: r.get(12)?,
        date: r.get(13)?,
        body: r.get(14)?,
        lang: r.get(15)?,
        list_unsubscribe: r.get(16)?,
        attachments: vec_of(r.get(17)?),
        outgoing: r.get(18)?,
        seen: r.get(19)?,
    })
}

fn draft_of(r: &Row) -> rusqlite::Result<Draft> {
    Ok(Draft {
        id: r.get(0)?,
        reply_to: r.get(1)?,
        to: vec_of(r.get(2)?),
        subject: r.get(3)?,
        body: r.get(4)?,
        status: r.get(5)?,
        created: r.get(6)?,
    })
}

fn followup_of(r: &Row) -> rusqlite::Result<FollowUp> {
    Ok(FollowUp {
        id: r.get(0)?,
        message_id: r.get(1)?,
        thread_id: r.get(2)?,
        subject: r.get(3)?,
        waiting_on: r.get(4)?,
        what: r.get(5)?,
        sent: r.get(6)?,
        due: r.get(7)?,
        status: r.get(8)?,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub imap_host: String,
    pub imap_port: u16,
    /// tls, starttls, or none (only for a server on this computer)
    pub imap_security: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_security: String,
    pub username: String,
}

fn account_of(r: &Row) -> rusqlite::Result<Account> {
    Ok(Account {
        id: r.get(0)?,
        name: r.get(1)?,
        email: r.get(2)?,
        imap_host: r.get(3)?,
        imap_port: r.get(4)?,
        imap_security: r.get(5)?,
        smtp_host: r.get(6)?,
        smtp_port: r.get(7)?,
        smtp_security: r.get(8)?,
        username: r.get(9)?,
    })
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        let db = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        Self::init(db)
    }

    pub fn memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(db: Connection) -> Result<Store> {
        db.busy_timeout(std::time::Duration::from_secs(10))?;
        db.execute_batch(SCHEMA)?;
        Ok(Store { db })
    }

    pub fn conn(&self) -> &Connection {
        &self.db
    }

    // --- accounts and folders ---

    pub fn add_account(&self, a: &Account) -> Result<i64> {
        self.db.execute(
            "INSERT INTO accounts (name, email, imap_host, imap_port, imap_security, smtp_host, smtp_port, smtp_security, username)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![a.name, a.email, a.imap_host, a.imap_port, a.imap_security, a.smtp_host, a.smtp_port, a.smtp_security, a.username],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn accounts(&self) -> Result<Vec<Account>> {
        let mut st = self.db.prepare(
            "SELECT id, name, email, imap_host, imap_port, imap_security, smtp_host, smtp_port, smtp_security, username FROM accounts ORDER BY id",
        )?;
        Ok(st.query_map([], account_of)?.collect::<rusqlite::Result<_>>()?)
    }

    pub fn account(&self, id: i64) -> Result<Option<Account>> {
        Ok(self.accounts()?.into_iter().find(|a| a.id == id))
    }

    pub fn delete_account(&self, id: i64) -> Result<()> {
        self.db.execute("DELETE FROM accounts WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn folder_state(&self, account: i64, name: &str) -> Result<Option<(u32, u32)>> {
        Ok(self
            .db
            .query_row("SELECT uidvalidity, last_uid FROM folders WHERE account_id = ?1 AND name = ?2", params![account, name], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?)
    }

    pub fn set_folder_state(&self, account: i64, name: &str, role: &str, uidvalidity: u32, last_uid: u32) -> Result<()> {
        self.db.execute(
            "INSERT INTO folders (account_id, name, role, uidvalidity, last_uid) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(account_id, name) DO UPDATE SET role = excluded.role, uidvalidity = excluded.uidvalidity, last_uid = excluded.last_uid",
            params![account, name, role, uidvalidity, last_uid],
        )?;
        Ok(())
    }

    /// A folder whose UIDVALIDITY changed: its old copies no longer match the server.
    pub fn forget_folder(&self, account: i64, name: &str) -> Result<()> {
        self.db.execute("DELETE FROM messages WHERE account_id = ?1 AND folder = ?2", params![account, name])?;
        Ok(())
    }

    // --- messages ---

    /// Stores a message (ignored if that UID is already there); returns its id, or None if it existed.
    pub fn insert_message(&self, m: &Message) -> Result<Option<i64>> {
        let n = self.db.execute(
            "INSERT OR IGNORE INTO messages (account_id, folder, uid, message_id, in_reply_to, refs, thread_id, from_addr, from_name,
                 to_addrs, cc_addrs, subject, date, body, lang, list_unsubscribe, attachments, outgoing, seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                m.account_id,
                m.folder,
                m.uid,
                m.message_id,
                m.in_reply_to,
                json(&m.references),
                m.thread_id,
                m.from_addr,
                m.from_name,
                json(&m.to),
                json(&m.cc),
                m.subject,
                m.date,
                m.body,
                m.lang,
                m.list_unsubscribe,
                json(&m.attachments),
                m.outgoing,
                m.seen
            ],
        )?;
        Ok(if n == 1 { Some(self.db.last_insert_rowid()) } else { None })
    }

    pub fn message(&self, id: i64) -> Result<Option<Message>> {
        Ok(self.db.query_row(&format!("SELECT {MESSAGE_COLUMNS} FROM messages WHERE id = ?1"), [id], message_of).optional()?)
    }

    /// The thread a message with these headers belongs to: the thread of the message it answers or
    /// references, if we have it.
    pub fn thread_of(&self, account: i64, in_reply_to: &str, references: &[String]) -> Result<Option<String>> {
        for mid in std::iter::once(in_reply_to).chain(references.iter().rev().map(|s| s.as_str())) {
            if mid.is_empty() {
                continue;
            }
            let t: Option<String> = self
                .db
                .query_row("SELECT thread_id FROM messages WHERE account_id = ?1 AND message_id = ?2 LIMIT 1", params![account, mid], |r| {
                    r.get(0)
                })
                .optional()?;
            if t.is_some() {
                return Ok(t);
            }
        }
        Ok(None)
    }

    /// Messages that arrived before their parent got a thread of their own; now that the parent is
    /// here, they join its thread.
    pub fn adopt_children(&self, account: i64, message_id: &str, thread_id: &str) -> Result<()> {
        if message_id.is_empty() {
            return Ok(());
        }
        self.db.execute(
            "UPDATE messages SET thread_id = ?3 WHERE account_id = ?1 AND thread_id != ?3 AND (in_reply_to = ?2 OR refs LIKE '%' || ?4 || '%')",
            params![account, message_id, thread_id, serde_json::to_string(message_id)?],
        )?;
        Ok(())
    }

    pub fn thread(&self, thread_id: &str) -> Result<Vec<Message>> {
        let mut st = self.db.prepare(&format!("SELECT {MESSAGE_COLUMNS} FROM messages WHERE thread_id = ?1 ORDER BY date, id"))?;
        Ok(st.query_map([thread_id], message_of)?.collect::<rusqlite::Result<_>>()?)
    }

    /// Incoming messages, newest first, with their triage (and label) if any.
    pub fn inbox(&self, category: Option<Category>, limit: usize) -> Result<Vec<(Message, Option<Triage>, Option<Category>)>> {
        let mut st = self.db.prepare(&format!(
            "SELECT {}, t.category, t.reason, t.model, t.prompt, t.ms, l.category FROM messages m
             LEFT JOIN triage t ON t.message_id = m.id LEFT JOIN labels l ON l.message_id = m.id
             WHERE m.outgoing = 0 AND (?1 IS NULL OR t.category = ?1) ORDER BY m.date DESC LIMIT ?2",
            MESSAGE_COLUMNS.split(", ").map(|c| format!("m.{c}")).collect::<Vec<_>>().join(", ")
        ))?;
        let rows = st.query_map(params![category.map(|c| c.as_str()), limit as i64], |r| {
            let m = message_of(r)?;
            let t = match r.get::<_, Option<String>>(20)? {
                Some(c) => Some(Triage {
                    message_id: m.id,
                    category: Category::parse(&c).unwrap_or(Category::Informative),
                    reason: r.get(21)?,
                    model: r.get(22)?,
                    prompt: r.get(23)?,
                    ms: r.get::<_, i64>(24)? as u64,
                }),
                None => None,
            };
            let label = r.get::<_, Option<String>>(25)?.and_then(|c| Category::parse(&c));
            Ok((m, t, label))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Incoming messages the model has not looked at yet, newest first.
    pub fn untriaged(&self, limit: usize) -> Result<Vec<Message>> {
        let mut st = self.db.prepare(&format!(
            "SELECT {MESSAGE_COLUMNS} FROM messages WHERE outgoing = 0 AND id NOT IN (SELECT message_id FROM triage) ORDER BY date DESC LIMIT ?1"
        ))?;
        Ok(st.query_map([limit as i64], message_of)?.collect::<rusqlite::Result<_>>()?)
    }

    /// Messages you sent, newest first.
    pub fn sent(&self, account: i64, limit: usize) -> Result<Vec<Message>> {
        let mut st = self.db.prepare(&format!(
            "SELECT {MESSAGE_COLUMNS} FROM messages WHERE account_id = ?1 AND outgoing = 1 ORDER BY date DESC LIMIT ?2"
        ))?;
        Ok(st.query_map(params![account, limit as i64], message_of)?.collect::<rusqlite::Result<_>>()?)
    }

    /// Full-text search among messages you sent (BM25).
    pub fn search_sent(&self, account: i64, query: &str, limit: usize) -> Result<Vec<Message>> {
        let q = fts_query(query);
        if q.is_empty() {
            return Ok(vec![]);
        }
        let mut st = self.db.prepare(&format!(
            "SELECT {} FROM messages_fts f JOIN messages m ON m.id = f.rowid
             WHERE messages_fts MATCH ?1 AND m.account_id = ?2 AND m.outgoing = 1 ORDER BY bm25(messages_fts) LIMIT ?3",
            MESSAGE_COLUMNS.split(", ").map(|c| format!("m.{c}")).collect::<Vec<_>>().join(", ")
        ))?;
        Ok(st.query_map(params![q, account, limit as i64], message_of)?.collect::<rusqlite::Result<_>>()?)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Message>> {
        let q = fts_query(query);
        if q.is_empty() {
            return Ok(vec![]);
        }
        let mut st = self.db.prepare(&format!(
            "SELECT {} FROM messages_fts f JOIN messages m ON m.id = f.rowid WHERE messages_fts MATCH ?1 ORDER BY bm25(messages_fts) LIMIT ?2",
            MESSAGE_COLUMNS.split(", ").map(|c| format!("m.{c}")).collect::<Vec<_>>().join(", ")
        ))?;
        Ok(st.query_map(params![q, limit as i64], message_of)?.collect::<rusqlite::Result<_>>()?)
    }

    // --- triage and labels ---

    pub fn set_triage(&self, t: &Triage) -> Result<()> {
        self.db.execute(
            "INSERT INTO triage (message_id, category, reason, model, prompt, ms, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(message_id) DO UPDATE SET category = excluded.category, reason = excluded.reason, model = excluded.model,
                 prompt = excluded.prompt, ms = excluded.ms, at = excluded.at",
            params![t.message_id, t.category.as_str(), t.reason, t.model, t.prompt, t.ms as i64, now()],
        )?;
        Ok(())
    }

    pub fn set_label(&self, message_id: i64, c: Option<Category>) -> Result<()> {
        match c {
            Some(c) => self.db.execute(
                "INSERT INTO labels (message_id, category, at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(message_id) DO UPDATE SET category = excluded.category, at = excluded.at",
                params![message_id, c.as_str(), now()],
            )?,
            None => self.db.execute("DELETE FROM labels WHERE message_id = ?1", [message_id])?,
        };
        Ok(())
    }

    /// The next incoming message without your label (oldest first, so labelling follows the inbox).
    pub fn next_unlabelled(&self) -> Result<Option<Message>> {
        Ok(self
            .db
            .query_row(
                &format!(
                    "SELECT {MESSAGE_COLUMNS} FROM messages WHERE outgoing = 0 AND id NOT IN (SELECT message_id FROM labels) ORDER BY date LIMIT 1"
                ),
                [],
                message_of,
            )
            .optional()?)
    }

    /// Your labelled messages, for the evaluation.
    pub fn labelled(&self) -> Result<Vec<(Message, Category)>> {
        let mut st = self.db.prepare(&format!(
            "SELECT {}, l.category FROM messages m JOIN labels l ON l.message_id = m.id ORDER BY m.date",
            MESSAGE_COLUMNS.split(", ").map(|c| format!("m.{c}")).collect::<Vec<_>>().join(", ")
        ))?;
        let rows = st.query_map([], |r| Ok((message_of(r)?, r.get::<_, String>(20)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?.into_iter().filter_map(|(m, c)| Category::parse(&c).map(|c| (m, c))).collect())
    }

    pub fn counts(&self) -> Result<serde_json::Value> {
        let q = |sql: &str| -> Result<i64> { Ok(self.db.query_row(sql, [], |r| r.get(0))?) };
        let mut by = serde_json::Map::new();
        for c in Category::ALL {
            let n: i64 = self.db.query_row(
                "SELECT count(*) FROM triage t JOIN messages m ON m.id = t.message_id WHERE t.category = ?1 AND m.outgoing = 0",
                [c.as_str()],
                |r| r.get(0),
            )?;
            by.insert(c.as_str().into(), n.into());
        }
        Ok(serde_json::json!({
            "messages": q("SELECT count(*) FROM messages")?,
            "incoming": q("SELECT count(*) FROM messages WHERE outgoing = 0")?,
            "untriaged": q("SELECT count(*) FROM messages WHERE outgoing = 0 AND id NOT IN (SELECT message_id FROM triage)")?,
            "labelled": q("SELECT count(*) FROM labels")?,
            "followups_due": self.db.query_row("SELECT count(*) FROM followups WHERE status = 'waiting' AND due <= ?1", [now()], |r| r.get::<_, i64>(0))?,
            "by_category": by,
        }))
    }

    // --- summaries ---

    pub fn summary(&self, thread_id: &str) -> Result<Option<(usize, serde_json::Value)>> {
        Ok(self
            .db
            .query_row("SELECT messages, summary FROM summaries WHERE thread_id = ?1", [thread_id], |r| {
                Ok((r.get::<_, i64>(0)? as usize, r.get::<_, String>(1)?))
            })
            .optional()?
            .map(|(n, s)| (n, serde_json::from_str(&s).unwrap_or(serde_json::Value::Null))))
    }

    pub fn set_summary(&self, thread_id: &str, messages: usize, summary: &serde_json::Value, model: &str) -> Result<()> {
        self.db.execute(
            "INSERT INTO summaries (thread_id, messages, summary, model, at) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(thread_id) DO UPDATE SET messages = excluded.messages, summary = excluded.summary, model = excluded.model, at = excluded.at",
            params![thread_id, messages as i64, summary.to_string(), model, now()],
        )?;
        Ok(())
    }

    /// Threads long enough to summarize (at least `min_messages`) whose summary is missing or old.
    pub fn threads_to_summarize(&self, min_messages: usize, limit: usize) -> Result<Vec<String>> {
        let mut st = self.db.prepare(
            "SELECT m.thread_id FROM messages m LEFT JOIN summaries s ON s.thread_id = m.thread_id
             GROUP BY m.thread_id HAVING count(*) >= ?1 AND (max(s.messages) IS NULL OR max(s.messages) < count(*))
             ORDER BY max(m.date) DESC LIMIT ?2",
        )?;
        Ok(st.query_map(params![min_messages as i64, limit as i64], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?)
    }

    // --- drafts ---

    pub fn add_draft(&self, reply_to: i64, to: &[String], subject: &str, body: &str) -> Result<i64> {
        self.db.execute(
            "INSERT INTO drafts (reply_to, to_addrs, subject, body, status, created) VALUES (?1, ?2, ?3, ?4, 'proposed', ?5)",
            params![reply_to, json(&to), subject, body, now()],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn draft(&self, id: i64) -> Result<Option<Draft>> {
        Ok(self
            .db
            .query_row("SELECT id, reply_to, to_addrs, subject, body, status, created FROM drafts WHERE id = ?1", [id], draft_of)
            .optional()?)
    }

    pub fn drafts_for(&self, reply_to: i64) -> Result<Vec<Draft>> {
        let mut st = self.db.prepare(
            "SELECT id, reply_to, to_addrs, subject, body, status, created FROM drafts WHERE reply_to = ?1 AND status != 'discarded' ORDER BY id DESC",
        )?;
        Ok(st.query_map([reply_to], draft_of)?.collect::<rusqlite::Result<_>>()?)
    }

    pub fn update_draft(&self, id: i64, body: Option<&str>, status: Option<&str>) -> Result<()> {
        if let Some(b) = body {
            self.db.execute("UPDATE drafts SET body = ?2 WHERE id = ?1", params![id, b])?;
        }
        if let Some(s) = status {
            self.db.execute("UPDATE drafts SET status = ?2 WHERE id = ?1", params![id, s])?;
        }
        Ok(())
    }

    // --- follow-ups ---

    /// Outgoing messages not yet looked at for follow-ups.
    pub fn unchecked_outgoing(&self, limit: usize) -> Result<Vec<Message>> {
        let mut st = self.db.prepare(&format!(
            "SELECT {MESSAGE_COLUMNS} FROM messages WHERE outgoing = 1 AND id NOT IN (SELECT message_id FROM followup_checked)
             ORDER BY date DESC LIMIT ?1"
        ))?;
        Ok(st.query_map([limit as i64], message_of)?.collect::<rusqlite::Result<_>>()?)
    }

    pub fn mark_checked(&self, message_id: i64) -> Result<()> {
        self.db.execute("INSERT OR IGNORE INTO followup_checked (message_id) VALUES (?1)", [message_id])?;
        Ok(())
    }

    pub fn add_followup(&self, f: &FollowUp) -> Result<i64> {
        self.db.execute(
            "INSERT INTO followups (message_id, thread_id, subject, waiting_on, what, sent, due, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(message_id) DO UPDATE SET waiting_on = excluded.waiting_on, what = excluded.what, due = excluded.due, status = excluded.status",
            params![f.message_id, f.thread_id, f.subject, f.waiting_on, f.what, f.sent, f.due, f.status],
        )?;
        self.mark_checked(f.message_id)?;
        Ok(self.db.query_row("SELECT id FROM followups WHERE message_id = ?1", [f.message_id], |r| r.get(0))?)
    }

    pub fn followups(&self, status: Option<&str>) -> Result<Vec<FollowUp>> {
        let mut st = self.db.prepare(
            "SELECT id, message_id, thread_id, subject, waiting_on, what, sent, due, status FROM followups
             WHERE (?1 IS NULL OR status = ?1) ORDER BY due",
        )?;
        Ok(st.query_map([status], followup_of)?.collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_followup_status(&self, id: i64, status: &str) -> Result<()> {
        self.db.execute("UPDATE followups SET status = ?2 WHERE id = ?1", params![id, status])?;
        Ok(())
    }

    pub fn snooze_followup(&self, id: i64, due: i64) -> Result<()> {
        self.db.execute("UPDATE followups SET due = ?2, status = 'waiting' WHERE id = ?1", params![id, due])?;
        Ok(())
    }

    /// Did anyone but the account's owner write in this thread after `after`?
    pub fn answered_after(&self, thread_id: &str, after: i64) -> Result<bool> {
        Ok(self.db.query_row(
            "SELECT count(*) FROM messages WHERE thread_id = ?1 AND date > ?2 AND outgoing = 0",
            params![thread_id, after],
            |r| r.get::<_, i64>(0),
        )? > 0)
    }
}

/// Words of a free text as an FTS5 query: any of them (quoted, so punctuation is harmless).
pub fn fts_query(text: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3)
        .map(|w| w.to_lowercase())
        .filter(|w| seen.insert(w.clone()))
        .take(32)
        .map(|w| format!("\"{w}\""))
        .collect::<Vec<_>>()
        .join(" OR ")
}
