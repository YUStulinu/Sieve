//! Reply drafts in your style: the model gets the message (and the thread before it), a
//! description of how you write, and a few of your past messages most like this one. The draft is
//! only a proposal: it is sent when you confirm it, never otherwise.

use anyhow::Result;

use crate::llm::{Model, clip};
use crate::mail::base_subject;
use crate::model::Message;
use crate::store::Store;
use crate::style::Style;

/// How a reply is written (the evaluation turns the parts on and off).
#[derive(Debug, Clone)]
pub struct DraftOptions {
    /// Describe the learned style (greetings, sign-off, length, diacritics).
    pub describe_style: bool,
    /// Show this many of your most similar past messages.
    pub examples: usize,
}

impl Default for DraftOptions {
    fn default() -> Self {
        // Measured (eval/results/drafts.json): three past messages alone match the real replies best;
        // the description adds nothing on top of them with a 3B model.
        DraftOptions { describe_style: false, examples: 3 }
    }
}

/// Your past messages most like a reply to `m`: first to the same person, then by text (BM25).
pub fn examples(store: &Store, account: i64, m: &Message, n: usize, exclude: &[i64]) -> Result<Vec<Message>> {
    if n == 0 {
        return Ok(vec![]);
    }
    let mut out: Vec<Message> = vec![];
    let query = format!("{} {}", m.subject, clip(&m.body, 600));
    for c in store.search_sent(account, &query, 40)? {
        if exclude.contains(&c.id) || c.body.trim().is_empty() || out.iter().any(|o| o.id == c.id) {
            continue;
        }
        out.push(c);
    }
    // Same recipient first (people write differently to different people), then same language.
    out.sort_by_key(|c| (!c.to.iter().any(|a| a.eq_ignore_ascii_case(&m.from_addr)), c.lang != m.lang));
    out.truncate(n);
    Ok(out)
}

pub fn system_prompt(owner: &str, style: Option<&Style>, examples: &[Message], lang: &str, to_first_name: &str) -> String {
    let mut s = format!(
        "You draft an email reply for {owner}, written as them, in the language of the message they received ({}). \
Write only the body of the reply: no subject line, no quoted text, no placeholders like [name] — if you do not know \
something, leave it out or ask for it. Do not promise or accept anything the message does not make clear; prefer a short \
answer that the writer can edit.",
        if lang == "ro" { "Romanian" } else { "English" }
    );
    if let Some(st) = style {
        let d = st.describe(lang, to_first_name);
        if !d.is_empty() {
            s.push_str(&format!("\n\nHow {owner} writes:\n{d}"));
        }
    }
    if !examples.is_empty() {
        s.push_str(&format!("\n\nSome messages {owner} wrote before (match their tone, length and wording):"));
        for e in examples {
            s.push_str(&format!("\n---\nTo: {}\nSubject: {}\n\n{}", e.to.join(", "), e.subject, clip(&e.body, 700)));
        }
        s.push_str("\n---");
    }
    s
}

pub fn user_prompt(m: &Message, before: &[Message]) -> String {
    let mut s = String::new();
    if !before.is_empty() {
        s.push_str("Earlier in the thread:\n");
        for b in before.iter().rev().take(2).rev() {
            s.push_str(&format!("[{}] {}\n\n", b.from_name.clone() + " <" + &b.from_addr + ">", clip(&b.body, 600)));
        }
    }
    s.push_str(&format!(
        "The message to answer:\nFrom: {} <{}>\nSubject: {}\n\n{}\n\nWrite the reply.",
        m.from_name,
        m.from_addr,
        m.subject,
        clip(&m.body, 2500)
    ));
    s
}

/// The sender's first name, for the greeting ("Mihai Ionescu" -> "Mihai", "Ionescu, Mihai" -> "Mihai").
pub fn first_name(m: &Message) -> String {
    let n = m.from_name.trim().trim_matches('"');
    let n = match n.split_once(',') {
        Some((_, first)) if !first.trim().is_empty() => first.trim().to_string(),
        _ => n.split_whitespace().next().unwrap_or("").to_string(),
    };
    if n.contains('@') || n.chars().any(|c| c.is_ascii_digit()) { String::new() } else { n }
}

/// Removes what models add around a reply: a subject line, a leading label, surrounding quotes.
pub fn tidy(text: &str) -> String {
    let mut lines: Vec<&str> = text.trim().lines().collect();
    while let Some(first) = lines.first() {
        let l = first.trim().to_lowercase();
        let preamble =
            (l.starts_with("sure") || l.starts_with("here") || l.starts_with("iată") || l.starts_with("desigur")) && l.ends_with(':');
        if preamble
            || l.starts_with("subject:")
            || l.starts_with("subiect:")
            || l.starts_with("reply:")
            || l.starts_with("răspuns:")
            || l.is_empty()
        {
            lines.remove(0);
        } else {
            break;
        }
    }
    lines.join("\n").trim().trim_matches('"').trim().to_string()
}

pub struct Proposal {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
}

/// What a draft needs from the store: the thread before the message, and the examples.
pub struct Context {
    pub before: Vec<Message>,
    pub examples: Vec<Message>,
}

pub fn context(store: &Store, m: &Message, opts: &DraftOptions) -> Result<Context> {
    let thread = store.thread(&m.thread_id)?;
    let before = thread.iter().filter(|t| t.date < m.date && t.id != m.id).cloned().collect();
    Ok(Context { before, examples: examples(store, m.account_id, m, opts.examples, &[])? })
}

/// Drafts a reply to `m` as `owner` (the model call; no store needed, so no lock held).
pub fn compose(model: &dyn Model, owner: &str, style: Option<&Style>, m: &Message, ctx: &Context, opts: &DraftOptions) -> Result<Proposal> {
    let sys = system_prompt(owner, if opts.describe_style { style } else { None }, &ctx.examples, &m.lang, &first_name(m));
    let body = tidy(&model.chat(&sys, &user_prompt(m, &ctx.before), None, 400)?);
    Ok(Proposal { to: vec![m.from_addr.clone()], subject: format!("Re: {}", base_subject(&m.subject)), body })
}

pub fn draft(model: &dyn Model, store: &Store, owner: &str, style: Option<&Style>, m: &Message, opts: &DraftOptions) -> Result<Proposal> {
    compose(model, owner, style, m, &context(store, m, opts)?, opts)
}

#[cfg(test)]
mod tests {
    use super::tidy;

    #[test]
    fn tidy_removes_subject_and_quotes() {
        assert_eq!(tidy("Subject: Re: Oferta\n\nBună Mihai,\nDa."), "Bună Mihai,\nDa.");
        assert_eq!(tidy("Sure, here's your reply:\n\nThanks, will do."), "Thanks, will do.");
        assert_eq!(tidy("\"Hi Dan, sure.\""), "Hi Dan, sure.");
    }
}
