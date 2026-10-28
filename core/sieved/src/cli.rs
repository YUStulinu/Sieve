//! sieve: the command line. Accounts and one-off runs, and the commands the evaluation harness uses
//! (they run exactly the code the daemon runs, on JSON lines instead of a mailbox).

use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sieve_core::config::{Config, data_dir};
use sieve_core::drafts::{self, DraftOptions};
use sieve_core::llm::{LlamaServer, Model};
use sieve_core::model::Category;
use sieve_core::store::{Account, Store};
use sieve_core::triage::{self, TriagePrompt};
use sieve_core::{followup, imap_sync, style, summary};
use sieved::daemon;
use sieved::jsonl::{self, Record};

#[derive(Parser)]
#[command(name = "sieve", about = "Sieve: local inbox triage")]
struct Cli {
    /// The llama.cpp server (default: from config.json)
    #[arg(long, global = true)]
    url: Option<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Adds a mail account; the password is read from standard input (or SIEVE_PASSWORD) and kept
    /// in the system's credential store.
    AddAccount {
        #[arg(long)]
        name: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        imap_host: String,
        #[arg(long, default_value_t = 993)]
        imap_port: u16,
        #[arg(long, default_value = "tls")]
        imap_security: String,
        #[arg(long)]
        smtp_host: String,
        #[arg(long, default_value_t = 587)]
        smtp_port: u16,
        #[arg(long, default_value = "starttls")]
        smtp_security: String,
        /// Login name (default: the email address)
        #[arg(long)]
        username: Option<String>,
    },
    /// Lists the accounts.
    Accounts,
    /// One round of what the daemon does: sync, triage, follow-ups, summaries.
    Once,
    /// The triaged inbox.
    Inbox {
        #[arg(long)]
        category: Option<String>,
        #[arg(long, default_value_t = 30)]
        limit: usize,
    },
    /// Triage of JSON-lines messages (evaluation).
    Classify {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
        /// A system prompt file instead of the built-in one.
        #[arg(long)]
        system: Option<PathBuf>,
        /// Category first, reason after (no thinking first).
        #[arg(long)]
        no_reason_first: bool,
        /// No header facts (List-Unsubscribe, automated sender, attachments, Cc).
        #[arg(long)]
        no_hints: bool,
        #[arg(long, default_value_t = 1500)]
        body_chars: usize,
        /// The variant's name, written with each prediction.
        #[arg(long, default_value = "default")]
        name: String,
    },
    /// Drafts for messages whose real replies are known (evaluation of the style).
    DraftEval {
        /// {"incoming": record, "reply": record} per line
        #[arg(long)]
        input: PathBuf,
        /// The owner's other sent messages: the style and the examples come from these.
        #[arg(long)]
        sent: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        owner: String,
        #[arg(long)]
        no_style: bool,
        #[arg(long, default_value_t = 3)]
        examples: usize,
    },
    /// Follow-up detection on sent messages (evaluation).
    FollowupEval {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
        /// Ask the model about every message, without the regex screen.
        #[arg(long)]
        no_screen: bool,
    },
    /// Summaries of threads (records grouped by "thread") (evaluation).
    SummarizeEval {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Your labelled messages as JSON lines (for the evaluation).
    ExportLabels {
        #[arg(long)]
        output: PathBuf,
    },
}

fn model(url: &Option<String>, cfg: &Config) -> Arc<LlamaServer> {
    let u = url.clone().unwrap_or_else(|| cfg.llm.url());
    let name = std::env::var("SIEVE_MODEL_NAME").unwrap_or_else(|_| cfg.llm.model_name());
    Arc::new(LlamaServer::new(&u, &name))
}

#[derive(Serialize)]
struct Prediction {
    id: String,
    label: Option<String>,
    predicted: Category,
    reason: String,
    ms: u64,
    model: String,
    variant: String,
}

#[derive(Deserialize)]
struct DraftCase {
    incoming: Record,
    reply: Record,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()))
        .init();
    let cli = Cli::parse();
    let dir = data_dir();
    let cfg = Config::load(&dir)?;
    match cli.cmd {
        Cmd::AddAccount { name, email, imap_host, imap_port, imap_security, smtp_host, smtp_port, smtp_security, username } => {
            let password = match std::env::var("SIEVE_PASSWORD") {
                Ok(p) => p,
                Err(_) => {
                    let mut s = String::new();
                    std::io::stdin().read_to_string(&mut s)?;
                    s.trim_end_matches(['\r', '\n']).to_string()
                }
            };
            let a = Account {
                id: 0,
                name,
                username: username.unwrap_or_else(|| email.clone()),
                email,
                imap_host,
                imap_port,
                imap_security,
                smtp_host,
                smtp_port,
                smtp_security,
            };
            imap_sync::connect(&a, &password).context("checking the IMAP login")?.logout().ok();
            if std::env::var("SIEVE_PASSWORD").is_err() {
                imap_sync::store_password(&a.email, &password)?;
            }
            std::fs::create_dir_all(&dir)?;
            let id = Store::open(&dir.join("sieve.db"))?.add_account(&a)?;
            println!("account {id} added: {}", a.email);
        }
        Cmd::Accounts => {
            for a in Store::open(&dir.join("sieve.db"))?.accounts()? {
                println!("{}\t{}\t{}:{} ({})", a.id, a.email, a.imap_host, a.imap_port, a.imap_security);
            }
        }
        Cmd::Once => {
            let db = dir.join("sieve.db");
            let state = daemon::State::new(Store::open(&db)?, Some(db), model(&cli.url, &cfg), cfg);
            daemon::round(&state)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"status": *state.status.lock().unwrap(), "counts": state.store.lock().unwrap().counts()?})
                )?
            );
        }
        Cmd::Inbox { category, limit } => {
            let st = Store::open(&dir.join("sieve.db"))?;
            for (m, t, _) in st.inbox(category.as_deref().and_then(Category::parse), limit)? {
                let c = t.map(|t| t.category.as_str()).unwrap_or("-");
                let when = chrono::DateTime::from_timestamp(m.date, 0).map(|d| d.format("%m-%d %H:%M").to_string()).unwrap_or_default();
                println!("{when}  {c:<12} {:<28} {}", m.from_addr.chars().take(28).collect::<String>(), m.subject);
            }
        }
        Cmd::Classify { input, output, system, no_reason_first, no_hints, body_chars, name } => {
            let m = model(&cli.url, &cfg);
            let p = TriagePrompt {
                system: match system {
                    Some(f) => std::fs::read_to_string(&f).with_context(|| format!("reading {}", f.display()))?,
                    None => triage::DEFAULT_SYSTEM.to_string(),
                },
                reason_first: !no_reason_first,
                hints: !no_hints,
                body_chars,
                name: name.clone(),
            };
            let recs = jsonl::read(&input)?;
            let mut out = vec![];
            let t0 = Instant::now();
            for (i, r) in recs.iter().enumerate() {
                let msg = r.to_message(i as i64 + 1);
                let reader = if r.owner.is_empty() { r.to.first().cloned().unwrap_or_default() } else { r.owner.clone() };
                let t = triage::classify(m.as_ref(), &msg, &reader, &p)?;
                out.push(Prediction {
                    id: r.id.clone(),
                    label: r.label.clone(),
                    predicted: t.category,
                    reason: t.reason,
                    ms: t.ms,
                    model: t.model,
                    variant: name.clone(),
                });
                if (i + 1) % 25 == 0 {
                    eprintln!("{}/{} ({:.1} s per message)", i + 1, recs.len(), t0.elapsed().as_secs_f64() / (i + 1) as f64);
                }
            }
            jsonl::write(&output, &out)?;
        }
        Cmd::DraftEval { input, sent, output, owner, no_style, examples } => {
            let m = model(&cli.url, &cfg);
            let st = Store::memory()?;
            st.add_account(&Account {
                id: 1,
                name: owner.clone(),
                email: owner.clone(),
                imap_host: "-".into(),
                imap_port: 0,
                imap_security: "-".into(),
                smtp_host: "-".into(),
                smtp_port: 0,
                smtp_security: "-".into(),
                username: "-".into(),
            })?;
            // The owner's sent mail: style and examples. Each case's real reply is left out of both.
            let sent_recs = jsonl::read(&sent)?;
            for (i, r) in sent_recs.iter().enumerate() {
                let mut msg = r.to_message(i as i64 + 1);
                msg.outgoing = true;
                msg.folder = "Sent".into();
                st.insert_message(&msg)?;
            }
            let cases: Vec<DraftCase> = std::fs::read_to_string(&input)?
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<Result<_, _>>()?;
            let opts = DraftOptions { describe_style: !no_style, examples };
            let mut out = vec![];
            for c in &cases {
                let incoming = c.incoming.to_message(1_000_000);
                let others: Vec<_> = st
                    .sent(1, 10_000)?
                    .into_iter()
                    .filter(|s| s.message_id != format!("{}@bench", c.reply.id) && s.message_id != c.reply.message_id)
                    .collect();
                let learned = style::learn(&others, &owner);
                let exclude: Vec<i64> = st.sent(1, 10_000)?.iter().filter(|s| !others.iter().any(|o| o.id == s.id)).map(|s| s.id).collect();
                let ctx = drafts::Context { before: vec![], examples: drafts::examples(&st, 1, &incoming, opts.examples, &exclude)? };
                let t0 = Instant::now();
                let p =
                    drafts::compose(m.as_ref(), &owner, if opts.describe_style { Some(&learned) } else { None }, &incoming, &ctx, &opts)?;
                out.push(json!({"id": c.incoming.id, "draft": p.body, "reply": c.reply.body, "lang": incoming.lang, "ms": t0.elapsed().as_millis() as u64,
                    "examples": ctx.examples.len(), "style": !no_style}));
                eprint!(".");
            }
            eprintln!();
            jsonl::write(&output, &out)?;
        }
        Cmd::FollowupEval { input, output, no_screen } => {
            let m = model(&cli.url, &cfg);
            let mut out = vec![];
            for (i, r) in jsonl::read(&input)?.iter().enumerate() {
                let msg = r.to_message(i as i64 + 1);
                let screened = followup::might_ask(&msg.body) || followup::might_ask(&msg.subject);
                let t0 = Instant::now();
                let e = if no_screen || screened {
                    let v = sieve_core::llm::parse_json(&m.chat_followup(&msg)?)?;
                    followup::Expectation {
                        expects_reply: v["expects_reply"].as_bool().unwrap_or(false),
                        waiting_on: v["waiting_on"].as_str().unwrap_or("").into(),
                        what: v["what"].as_str().unwrap_or("").into(),
                    }
                } else {
                    followup::Expectation { expects_reply: false, waiting_on: String::new(), what: String::new() }
                };
                out.push(
                    json!({"id": r.id, "label": r.label, "expects_reply": e.expects_reply, "waiting_on": e.waiting_on, "what": e.what,
                    "screened": screened, "ms": t0.elapsed().as_millis() as u64}),
                );
            }
            jsonl::write(&output, &out)?;
        }
        Cmd::SummarizeEval { input, output } => {
            let m = model(&cli.url, &cfg);
            let recs = jsonl::read(&input)?;
            let mut threads: Vec<(String, Vec<sieve_core::Message>)> = vec![];
            for (i, r) in recs.iter().enumerate() {
                let msg = r.to_message(i as i64 + 1);
                match threads.iter_mut().find(|(t, _)| *t == msg.thread_id) {
                    Some((_, v)) => v.push(msg),
                    None => threads.push((msg.thread_id.clone(), vec![msg])),
                }
            }
            let mut out = vec![];
            for (t, mut msgs) in threads {
                msgs.sort_by_key(|m| m.date);
                let t0 = Instant::now();
                let s = summary::summarize(m.as_ref(), &msgs)?;
                out.push(json!({"thread": t, "messages": msgs.len(), "summary": s, "source": summary::render(&msgs), "ms": t0.elapsed().as_millis() as u64}));
                eprint!(".");
            }
            eprintln!();
            jsonl::write(&output, &out)?;
        }
        Cmd::ExportLabels { output } => {
            let st = Store::open(&dir.join("sieve.db"))?;
            let rows: Vec<Record> = st.labelled()?.iter().map(|(m, c)| Record::from_message(m, Some(c.as_str()))).collect();
            if rows.is_empty() {
                bail!("no labelled messages yet: label some in the app (Label mode)");
            }
            jsonl::write(&output, &rows)?;
            println!("{} labelled messages written to {}", rows.len(), output.display());
        }
    }
    Ok(())
}

/// The follow-up question without the regex screen (for --no-screen).
trait FollowupChat {
    fn chat_followup(&self, m: &sieve_core::Message) -> Result<String>;
}

impl FollowupChat for LlamaServer {
    fn chat_followup(&self, m: &sieve_core::Message) -> Result<String> {
        self.chat(followup::SYSTEM, &followup::render(m), Some(&followup::schema()), 120)
    }
}
