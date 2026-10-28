//! Keeping the local copy in step with the server: for each folder that matters (the inbox, the
//! sent folder), fetch the messages with UIDs above the last one seen; if the folder's UIDVALIDITY
//! changed, start that folder over. Messages are read with BODY.PEEK, so nothing is marked as read.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use anyhow::{Context, Result, bail};

use crate::mail;
use crate::store::{Account, Store};

/// A connection's byte stream, TLS or not.
pub trait ReadWrite: Read + Write + Send {}
impl<T: Read + Write + Send> ReadWrite for T {}

pub struct Stream(Box<dyn ReadWrite>);

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}
impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

pub type Session = imap::Session<Stream>;

fn is_local(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1") || host.ends_with(".localhost") || host.ends_with(".test")
}

/// The account's password: SIEVE_PASSWORD (for a local test server), else the system's credential store.
pub fn password(a: &Account) -> Result<String> {
    if let Ok(p) = std::env::var("SIEVE_PASSWORD") {
        return Ok(p);
    }
    let e = keyring::Entry::new("sieve", &a.email)?;
    e.get_password().with_context(|| format!("no password stored for {}", a.email))
}

pub fn store_password(email: &str, password: &str) -> Result<()> {
    keyring::Entry::new("sieve", email)?.set_password(password)?;
    Ok(())
}

pub fn delete_password(email: &str) -> Result<()> {
    let _ = keyring::Entry::new("sieve", email)?.delete_credential();
    Ok(())
}

pub fn connect(a: &Account, password: &str) -> Result<Session> {
    let tcp =
        TcpStream::connect((a.imap_host.as_str(), a.imap_port)).with_context(|| format!("cannot reach {}:{}", a.imap_host, a.imap_port))?;
    tcp.set_read_timeout(Some(Duration::from_secs(120)))?;
    let stream = match a.imap_security.as_str() {
        "tls" => {
            let tls = native_tls::TlsConnector::new()?;
            Stream(Box::new(tls.connect(&a.imap_host, tcp).context("TLS handshake failed")?))
        }
        // Unencrypted only to a server on this computer: never send a password in clear over a network.
        "none" if is_local(&a.imap_host) => Stream(Box::new(tcp)),
        "none" => bail!("an unencrypted connection is only allowed to a server on this computer"),
        other => bail!("unsupported IMAP security '{other}' (use tls)"),
    };
    let mut client = imap::Client::new(stream);
    client.read_greeting()?;
    client.login(&a.username, password).map_err(|(e, _)| anyhow::anyhow!("login to {} failed: {e}", a.imap_host))
}

/// The folders to sync: (name, role). The inbox, and the sent folder found by its special-use
/// flag or a usual name (created when there is none, as on a fresh test server).
pub fn folders(s: &mut Session) -> Result<Vec<(String, &'static str)>> {
    let names = s.list(Some(""), Some("*"))?;
    let mut sent = None;
    let mut drafts = None;
    for n in names.iter() {
        let attrs: Vec<String> = n.attributes().iter().map(|a| format!("{a:?}").to_lowercase()).collect();
        let name = n.name().to_string();
        let lower = name.to_lowercase();
        if sent.is_none()
            && (attrs.iter().any(|a| a.contains("\\sent"))
                || ["sent", "sent items", "sent messages", "[gmail]/sent mail", "inbox.sent", "trimise", "elemente trimise"]
                    .contains(&lower.as_str()))
        {
            sent = Some(name.clone());
        }
        if drafts.is_none()
            && (attrs.iter().any(|a| a.contains("\\drafts"))
                || ["drafts", "[gmail]/drafts", "inbox.drafts", "ciorne"].contains(&lower.as_str()))
        {
            drafts = Some(name.clone());
        }
    }
    drop(names);
    for (slot, default) in [(&mut sent, "Sent"), (&mut drafts, "Drafts")] {
        if slot.is_none() {
            s.create(default).ok();
            *slot = Some(default.to_string());
        }
    }
    Ok(vec![("INBOX".into(), "inbox"), (sent.unwrap(), "sent"), (drafts.unwrap(), "drafts")])
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SyncReport {
    pub new_messages: usize,
    pub folders: usize,
}

/// Fetches what is new in the inbox and the sent folder.
pub fn sync(store: &Store, a: &Account, password: &str, initial: u32) -> Result<SyncReport> {
    let mut s = connect(a, password)?;
    let mut report = SyncReport::default();
    for (folder, role) in folders(&mut s)? {
        if role == "drafts" {
            store.set_folder_state(a.id, &folder, role, 0, 0)?;
            continue;
        }
        report.new_messages += sync_folder(store, &mut s, a, &folder, role, initial)?;
        report.folders += 1;
    }
    s.logout().ok();
    Ok(report)
}

fn sync_folder(store: &Store, s: &mut Session, a: &Account, folder: &str, role: &str, initial: u32) -> Result<usize> {
    let mb = s.select(folder)?;
    let validity = mb.uid_validity.unwrap_or(0);
    let (old_validity, mut last) = store.folder_state(a.id, folder)?.unwrap_or((0, 0));
    if old_validity != 0 && old_validity != validity {
        tracing::warn!("{folder}: UIDVALIDITY changed, fetching it again");
        store.forget_folder(a.id, folder)?;
        last = 0;
    }
    let mut uids: Vec<u32> = if last == 0 {
        let mut all: Vec<u32> = s.uid_search("ALL")?.into_iter().collect();
        all.sort_unstable();
        let skip = all.len().saturating_sub(initial as usize);
        all.split_off(skip)
    } else {
        s.uid_search(format!("UID {}:*", last + 1))?.into_iter().filter(|u| *u > last).collect()
    };
    uids.sort_unstable();
    let mut added = 0;
    for chunk in uids.chunks(50) {
        let set = chunk.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
        let fetched = s.uid_fetch(&set, "(UID FLAGS BODY.PEEK[])")?;
        for f in fetched.iter() {
            let (Some(uid), Some(raw)) = (f.uid, f.body()) else { continue };
            let Some(mut m) = mail::parse(raw, &a.email) else {
                tracing::warn!("{folder}/{uid}: could not parse");
                continue;
            };
            m.account_id = a.id;
            m.folder = folder.to_string();
            m.uid = uid;
            m.outgoing = m.outgoing || role == "sent";
            m.seen = f.flags().iter().any(|fl| matches!(fl, imap::types::Flag::Seen));
            if m.message_id.is_empty() {
                m.message_id = format!("{}-{folder}-{uid}@sieve.local", a.id);
            }
            m.thread_id = store.thread_of(a.id, &m.in_reply_to, &m.references)?.unwrap_or_else(|| m.message_id.clone());
            if store.insert_message(&m)?.is_some() {
                store.adopt_children(a.id, &m.message_id, &m.thread_id)?;
                added += 1;
            }
        }
        let max = *chunk.iter().max().unwrap_or(&last);
        store.set_folder_state(a.id, folder, role, validity, max.max(last))?;
    }
    if uids.is_empty() {
        store.set_folder_state(a.id, folder, role, validity, last)?;
    }
    Ok(added)
}

/// Puts a message into a folder (a draft into Drafts, a sent message into Sent).
pub fn append(a: &Account, password: &str, folder_role: &str, raw: &[u8], flags: &[imap::types::Flag]) -> Result<()> {
    let mut s = connect(a, password)?;
    let folder = folders(&mut s)?.into_iter().find(|(_, r)| *r == folder_role).map(|(n, _)| n).context("no such folder")?;
    s.append_with_flags(&folder, raw, flags)?;
    s.logout().ok();
    Ok(())
}
