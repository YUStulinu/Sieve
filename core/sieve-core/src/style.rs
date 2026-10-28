//! How you write, learned from the messages you sent: per language, your usual greetings and
//! sign-offs; how long your messages are; whether you write Romanian with diacritics, and whether
//! you use the polite plural. With each new reply, the model also sees a few of your past messages
//! most like it (see `drafts`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::Message;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LangStyle {
    pub messages: usize,
    /// The most frequent first lines ("Salut {name},", "Hi {name},"): {name} is the recipient's first name.
    pub greetings: Vec<String>,
    /// The most frequent closings, signed with the owner's name or initials ("Mersi,\nAna").
    pub signoffs: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Style {
    pub messages: usize,
    /// Median words per message.
    pub median_words: usize,
    /// Share of Romanian messages written with diacritics.
    pub diacritics: f32,
    /// Share of Romanian messages using the polite plural (vă, dumneavoastră).
    pub formal_ro: f32,
    pub by_lang: HashMap<String, LangStyle>,
}

fn first_line(body: &str) -> Option<String> {
    let l = body.lines().map(str::trim).find(|l| !l.is_empty())?;
    // Not a quoted or forwarded header ("-----Original Message-----", "From: …").
    if !l.chars().next().is_some_and(|c| c.is_alphabetic()) || l.contains(':') {
        return None;
    }
    // A greeting is short and ends with a comma or an exclamation mark.
    let words = l.split_whitespace().count();
    if words <= 4 && (l.ends_with(',') || l.ends_with('!')) {
        // "Hi Dan," -> "Hi {name},": a capitalized last word before the punctuation is a name.
        let mut ws: Vec<String> = l.split_whitespace().map(|s| s.to_string()).collect();
        if ws.len() >= 2 {
            let last = ws.last().unwrap().trim_end_matches([',', '!']).to_string();
            let punct = ws.last().unwrap()[last.len()..].to_string();
            if last.chars().next().is_some_and(|c| c.is_uppercase()) {
                let n = ws.len() - 1;
                ws[n] = format!("{{name}}{punct}");
            }
        }
        return Some(ws.join(" "));
    }
    None
}

/// The ways the owner signs: first and last name, the email's name parts, and initials.
pub fn signatures(owner: &str) -> Vec<String> {
    let (name, email) = match owner.split_once('<') {
        Some((n, e)) => (n.trim(), e.trim_end_matches('>')),
        None if owner.contains('@') => ("", owner),
        None => (owner, ""),
    };
    let mut parts: Vec<String> = name.split_whitespace().map(|s| s.to_lowercase()).collect();
    if let Some((local, _)) = email.split_once('@') {
        parts.extend(local.split(['.', '_', '-']).filter(|p| p.len() > 1).map(|s| s.to_lowercase()));
    }
    let initials: String = parts.iter().take(2).filter_map(|p| p.chars().next()).collect();
    if initials.len() == 2 {
        parts.push(initials);
    }
    parts.sort();
    parts.dedup();
    parts
}

/// "Mulțumesc,\nAna": a short closing line, then the owner's name (or initials) on its own.
fn closing(body: &str, signs: &[String]) -> Option<String> {
    let lines: Vec<&str> = body.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.len() < 2 {
        return None;
    }
    let (close, name) = (lines[lines.len() - 2], lines[lines.len() - 1]);
    let signed = |l: &str| {
        let w: Vec<String> = l.split_whitespace().map(|x| x.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase()).collect();
        !w.is_empty() && w.len() <= 3 && w.iter().all(|x| signs.contains(x))
    };
    if signed(name) && close.split_whitespace().count() <= 4 && close.ends_with([',', '!']) {
        return Some(format!("{close}\n{name}"));
    }
    // A one-line "Thanks, Chris".
    if let Some((c, n)) = name.split_once(',') {
        if !signs.is_empty() && signed(n) && c.split_whitespace().count() <= 3 {
            return Some(name.to_string());
        }
    }
    None
}

fn top(counts: HashMap<String, usize>, n: usize, min: usize) -> Vec<String> {
    let mut v: Vec<_> = counts.into_iter().filter(|(_, c)| *c >= min).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.into_iter().take(n).map(|(s, _)| s).collect()
}

/// Learns the style of `owner` ("Ana Popescu <ana@…>") from the messages they sent.
pub fn learn(sent: &[Message], owner: &str) -> Style {
    let signs = signatures(owner);
    let mut greetings: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut signoffs: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut words = vec![];
    let (mut ro, mut ro_diacritics, mut ro_formal) = (0usize, 0usize, 0usize);
    for m in sent.iter().filter(|m| !m.body.trim().is_empty()) {
        let lang = if m.lang.is_empty() { "en".to_string() } else { m.lang.clone() };
        *counts.entry(lang.clone()).or_insert(0) += 1;
        if let Some(g) = first_line(&m.body) {
            *greetings.entry(lang.clone()).or_default().entry(g).or_insert(0) += 1;
        }
        if let Some(s) = closing(&m.body, &signs) {
            *signoffs.entry(lang.clone()).or_default().entry(s).or_insert(0) += 1;
        }
        words.push(m.body.split_whitespace().count());
        if lang == "ro" {
            ro += 1;
            let lower = m.body.to_lowercase();
            if lower.chars().any(|c| "ăâîșşțţ".contains(c)) {
                ro_diacritics += 1;
            }
            if lower.split(|c: char| !c.is_alphabetic()).any(|w| ["vă", "dumneavoastră", "dumneavoastra", "dvs"].contains(&w)) {
                ro_formal += 1;
            }
        }
    }
    words.sort_unstable();
    let by_lang = counts
        .into_iter()
        .map(|(lang, n)| {
            // A habit is something done at least twice, and in at least 5% of the messages.
            let min = 2.max(n / 20);
            let ls = LangStyle {
                messages: n,
                greetings: top(greetings.remove(&lang).unwrap_or_default(), 3, min),
                signoffs: top(signoffs.remove(&lang).unwrap_or_default(), 3, min),
            };
            (lang, ls)
        })
        .collect();
    Style {
        messages: words.len(),
        median_words: words.get(words.len() / 2).copied().unwrap_or(0),
        diacritics: if ro > 0 { ro_diacritics as f32 / ro as f32 } else { 0.0 },
        formal_ro: if ro > 0 { ro_formal as f32 / ro as f32 } else { 0.0 },
        by_lang,
    }
}

impl Style {
    /// The style as instructions for the model, for a reply in `lang` to someone called `first_name`.
    pub fn describe(&self, lang: &str, first_name: &str) -> String {
        let mut s = vec![];
        let fill = |g: &str| if first_name.is_empty() { g.replace(" {name}", "") } else { g.replace("{name}", first_name) };
        if let Some(ls) = self.by_lang.get(lang) {
            if !ls.greetings.is_empty() {
                s.push(format!(
                    "Start the reply with the line {} (their usual greeting).",
                    ls.greetings.iter().map(|g| format!("\"{}\"", fill(g))).collect::<Vec<_>>().join(" or ")
                ));
            } else {
                s.push("They usually start directly, without a greeting line.".into());
            }
            if let Some(sig) = ls.signoffs.first() {
                s.push(format!("End the reply with their usual sign-off, exactly:\n{sig}"));
            } else {
                s.push("They usually end without a sign-off.".into());
            }
        }
        if lang == "ro" {
            s.push(if self.diacritics >= 0.5 {
                "They write Romanian with diacritics (ă, â, î, ș, ț).".into()
            } else {
                "They write Romanian without diacritics.".into()
            });
        }
        s.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(body: &str, lang: &str) -> Message {
        Message { body: body.into(), lang: lang.into(), outgoing: true, ..Default::default() }
    }

    #[test]
    fn learns_greetings_and_signoffs_per_language() {
        let s = learn(
            &[
                sent("Salut Dan,\n\nPerfect, ne vedem joi.\n\nMersi,\nAna", "ro"),
                sent("Salut Mihai,\n\nÎți trimit fișierul mâine.\n\nMersi,\nAna", "ro"),
                sent("Hi Tom,\n\nSounds good, see you Thursday.\n\nBest,\nAna", "en"),
                sent("Hi Sara,\n\nThanks, all clear.\n\nBest,\nAna", "en"),
            ],
            "Ana Popescu <ana.popescu@sieve.test>",
        );
        assert_eq!(s.messages, 4);
        assert_eq!(s.by_lang["ro"].greetings, vec!["Salut {name},"]);
        assert_eq!(s.by_lang["ro"].signoffs, vec!["Mersi,\nAna"]);
        assert_eq!(s.by_lang["en"].signoffs, vec!["Best,\nAna"]);
        assert_eq!(s.diacritics, 0.5);
        let d = s.describe("ro", "Elena");
        assert!(d.contains("\"Salut Elena,\"") && d.contains("Mersi,\nAna") && !d.contains("Best"), "{d}");
        assert!(s.median_words >= 6 && s.median_words <= 10);
    }

    #[test]
    fn forwarded_text_and_other_peoples_names_are_not_learned() {
        let s = learn(
            &[
                sent("-----Original Message-----\nFrom: Brenda\nThanks,\nBrenda", "en"),
                sent("-----Original Message-----\nFrom: Brenda\nThanks,\nBrenda", "en"),
                sent("ok, will do.\n\nthanks\ncg", "en"),
                sent("Done.\nThanks, Chris", "en"),
                sent("Sounds good.\nThanks, Chris", "en"),
            ],
            "Chris Germany <chris.germany@enron.com>",
        );
        let en = &s.by_lang["en"];
        assert!(en.greetings.is_empty(), "{:?}", en.greetings);
        assert_eq!(en.signoffs, vec!["Thanks, Chris"]);
        assert!(signatures("Chris Germany <chris.germany@enron.com>").contains(&"cg".to_string()));
    }
}
