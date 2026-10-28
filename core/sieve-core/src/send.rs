//! Turning a draft into a real message: saving it to the server's Drafts folder, or — only when you
//! confirm — sending it by SMTP and filing a copy in Sent. Nothing else in Sieve sends mail.

use anyhow::{Context, Result, bail};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use mail_builder::MessageBuilder;

use crate::imap_sync;
use crate::model::{Draft, Message};
use crate::store::{Account, Store};

/// The draft as an RFC 5322 message, threaded under the message it answers.
pub fn build(a: &Account, d: &Draft, reply_to: &Message) -> Result<Vec<u8>> {
    let mut refs: Vec<String> = reply_to.references.clone();
    if !reply_to.message_id.is_empty() {
        refs.push(reply_to.message_id.clone());
    }
    let mut b = MessageBuilder::new()
        .from((a.name.as_str(), a.email.as_str()))
        .to(d.to.iter().map(|t| t.as_str()).collect::<Vec<_>>())
        .subject(d.subject.as_str())
        .text_body(d.body.as_str())
        .message_id(format!("{}.{}@sieve.local", chrono::Utc::now().timestamp_millis(), d.id));
    if !reply_to.message_id.is_empty() {
        b = b.in_reply_to(reply_to.message_id.as_str()).references(refs);
    }
    Ok(b.write_to_vec()?)
}

/// Saves the draft in the account's Drafts folder (where any mail client sees it).
pub fn save_to_drafts(store: &Store, d: &Draft) -> Result<()> {
    let m = store.message(d.reply_to)?.context("the message it answers is gone")?;
    let a = store.account(m.account_id)?.context("no account")?;
    let raw = build(&a, d, &m)?;
    imap_sync::append(&a, &imap_sync::password(&a)?, "drafts", &raw, &[imap::types::Flag::Draft, imap::types::Flag::Seen])?;
    store.update_draft(d.id, None, Some("saved"))?;
    Ok(())
}

/// Sends the draft. `confirmation` must be the draft's id and recipients as the app showed them to
/// you ("<id>:<to>"), so a stale or mistaken request cannot send something else.
pub fn send(store: &Store, d: &Draft, confirmation: &str) -> Result<()> {
    if confirmation != confirmation_for(d) {
        bail!("not sent: the confirmation does not match this draft");
    }
    if d.status == "sent" {
        bail!("already sent");
    }
    let m = store.message(d.reply_to)?.context("the message it answers is gone")?;
    let a = store.account(m.account_id)?.context("no account")?;
    let password = imap_sync::password(&a)?;
    let raw = build(&a, d, &m)?;
    let envelope = lettre::address::Envelope::new(Some(a.email.parse()?), d.to.iter().map(|t| t.parse()).collect::<Result<Vec<_>, _>>()?)?;
    let creds = Credentials::new(a.username.clone(), password.clone());
    let mailer = match a.smtp_security.as_str() {
        "tls" => SmtpTransport::relay(&a.smtp_host)?.port(a.smtp_port).credentials(creds).build(),
        "starttls" => SmtpTransport::starttls_relay(&a.smtp_host)?.port(a.smtp_port).credentials(creds).build(),
        "none" if matches!(a.smtp_host.as_str(), "localhost" | "127.0.0.1") => {
            SmtpTransport::builder_dangerous(&a.smtp_host).port(a.smtp_port).credentials(creds).build()
        }
        other => bail!("unsupported SMTP security '{other}'"),
    };
    mailer.send_raw(&envelope, &raw).context("the SMTP server refused the message")?;
    store.update_draft(d.id, None, Some("sent"))?;
    // A copy in Sent, as mail clients do (some servers file it themselves; a duplicate is harmless).
    imap_sync::append(&a, &password, "sent", &raw, &[imap::types::Flag::Seen]).ok();
    Ok(())
}

pub fn confirmation_for(d: &Draft) -> String {
    format!("{}:{}", d.id, d.to.join(","))
}
