//! From raw RFC 5322 bytes to a [`Message`]: the headers that matter, and the text a reader sees —
//! the plain part (or the HTML one turned into text), without the quoted message it replies to or
//! the signature.

use std::sync::LazyLock;

use mail_parser::{AddressList, MessageParser, TextList};
use regex::Regex;

use crate::lang;
use crate::model::Message;

/// Longest body kept (characters): enough for any real message, bounded for newsletters.
pub const MAX_BODY: usize = 20_000;

fn addresses(a: Option<AddressList>) -> Vec<(String, String)> {
    a.map(|a| a.mailboxes().map(|m| (m.name().unwrap_or("").to_string(), m.address().unwrap_or("").to_lowercase())).collect())
        .unwrap_or_default()
}

fn ids(v: Option<TextList>) -> Vec<String> {
    v.map(|l| l.iter().map(|t| t.to_string()).collect()).unwrap_or_default()
}

/// Parses a message. `own` is the account's address (to tell outgoing from incoming).
pub fn parse(raw: &[u8], own: &str) -> Option<Message> {
    let p = MessageParser::default().parse(raw)?;
    let from = addresses(p.from());
    let (from_name, from_addr) = from.into_iter().next().unwrap_or_default();
    let to: Vec<String> = addresses(p.to()).into_iter().map(|(_, a)| a).filter(|a| !a.is_empty()).collect();
    let cc: Vec<String> = addresses(p.cc()).into_iter().map(|(_, a)| a).filter(|a| !a.is_empty()).collect();
    let subject = p.subject().unwrap_or("").trim().to_string();
    let message_id = p.message_id().unwrap_or("").to_string();
    let in_reply_to = ids(p.in_reply_to()).into_iter().next().unwrap_or_default();
    let references = ids(p.references());
    let date = p.date().map(|d| d.to_timestamp()).unwrap_or(0);
    let text = p.body_text(0).map(|t| t.to_string()).unwrap_or_default();
    let body = clean_body(&text);
    let attachments: Vec<String> = p.attachments().filter_map(|a| a.attachment_name().map(|s| s.to_string())).collect();
    let list_unsubscribe = p.list_unsubscribe().is_some() || p.list_id().is_some();
    let lang = lang::detect(&format!("{subject}\n{body}")).to_string();
    Some(Message {
        message_id,
        in_reply_to,
        references,
        outgoing: !own.is_empty() && from_addr.eq_ignore_ascii_case(own),
        from_addr,
        from_name,
        to,
        cc,
        subject,
        date,
        body,
        lang,
        list_unsubscribe,
        attachments,
        ..Default::default()
    })
}

static QUOTE_HEAD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?im)^(?:On .{0,200}wrote:\s*$|În data de .{0,200}a scris:\s*$|In data de .{0,200}a scris:\s*$|-{2,}\s*Original Message\s*-{2,}|-{2,}\s*Mesaj original\s*-{2,}|-{2,}\s*Forwarded message\s*-{2,}|From: .+\r?\nSent: .+)",
    )
    .unwrap()
});

/// The new text of a message: up to the quoted reply or the signature separator, without quoted
/// (">") lines, with runs of blank lines collapsed.
pub fn clean_body(text: &str) -> String {
    let mut t = text.replace("\r\n", "\n");
    if let Some(m) = QUOTE_HEAD.find(&t) {
        // A forward keeps its content (it is the message); a reply drops what it quotes.
        if !m.as_str().to_lowercase().contains("forwarded") {
            t.truncate(m.start());
        }
    }
    if let Some(i) = t.find("\n-- \n") {
        t.truncate(i);
    }
    let mut out = String::new();
    let mut blank = 0;
    for line in t.lines() {
        let l = line.trim_end();
        if l.trim_start().starts_with('>') {
            continue;
        }
        if l.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(l);
        out.push('\n');
    }
    let out = out.trim().to_string();
    if out.chars().count() > MAX_BODY { out.chars().take(MAX_BODY).collect() } else { out }
}

/// "Re: Fwd: RE: Factura" -> "Factura"
pub fn base_subject(s: &str) -> String {
    static PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*((re|fw|fwd|tr|aw|răsp|rasp)\s*(\[\d+\])?\s*:\s*)+").unwrap());
    PREFIX.replace(s, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW: &str = "From: Mihai Ionescu <Mihai@Example.ro>\r\nTo: ana@sieve.test\r\nCc: Dan <dan@example.ro>\r\n\
Subject: Re: Oferta pentru proiect\r\nMessage-ID: <b2@example.ro>\r\nIn-Reply-To: <a1@sieve.test>\r\n\
References: <a0@sieve.test> <a1@sieve.test>\r\nDate: Tue, 27 Oct 2026 09:30:00 +0200\r\n\
List-Unsubscribe: <mailto:unsub@example.ro>\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n\
Bună Ana,\r\n\r\nPoți să-mi trimiți oferta până mâine?\r\n\r\nMulțumesc,\r\nMihai\r\n-- \r\nMihai Ionescu | Director\r\n\r\n\
În data de luni, 26 oct. 2026, Ana a scris:\r\n> Vă trimit oferta.\r\n";

    #[test]
    fn parses_headers_and_clean_text() {
        let m = parse(RAW.as_bytes(), "ana@sieve.test").unwrap();
        assert_eq!(m.from_addr, "mihai@example.ro");
        assert_eq!(m.from_name, "Mihai Ionescu");
        assert_eq!(m.to, vec!["ana@sieve.test"]);
        assert_eq!(m.cc, vec!["dan@example.ro"]);
        assert_eq!(m.message_id, "b2@example.ro");
        assert_eq!(m.in_reply_to, "a1@sieve.test");
        assert_eq!(m.references, vec!["a0@sieve.test", "a1@sieve.test"]);
        assert_eq!(m.date, 1_793_086_200); // 2026-10-27 07:30 UTC
        assert!(m.list_unsubscribe);
        assert!(!m.outgoing);
        assert_eq!(m.lang, "ro");
        assert_eq!(m.body, "Bună Ana,\n\nPoți să-mi trimiți oferta până mâine?\n\nMulțumesc,\nMihai");
    }

    #[test]
    fn outgoing_is_from_the_owner() {
        let raw = "From: Ana <ana@sieve.test>\r\nTo: x@y.z\r\nSubject: Hi\r\n\r\nHello\r\n";
        assert!(parse(raw.as_bytes(), "ANA@sieve.test").unwrap().outgoing);
    }

    #[test]
    fn quoted_replies_go_but_forwards_stay() {
        let reply = "Sounds good.\n\nOn Mon, 26 Oct 2026 at 10:00, Dan <dan@x.ro> wrote:\n> Shall we meet?\n";
        assert_eq!(clean_body(reply), "Sounds good.");
        let outlook = "Agreed.\n\n-----Original Message-----\nFrom: Dan\nSent: Monday\n";
        assert_eq!(clean_body(outlook), "Agreed.");
        let fwd = "FYI\n\n---------- Forwarded message ---------\nFrom: Bank\nYour statement is ready.";
        assert!(clean_body(fwd).contains("Your statement is ready."));
    }

    #[test]
    fn subjects_lose_reply_prefixes() {
        assert_eq!(base_subject("Re: Fwd: RE: Factura"), "Factura");
        assert_eq!(base_subject("Răsp: Întâlnire"), "Întâlnire");
        assert_eq!(base_subject("Report"), "Report");
    }
}
