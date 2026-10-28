//! The background work: sync every account, triage what is new, look for follow-ups, summarize
//! long threads — then wait for the interval, or for a nudge (new mail requested from the app).

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use anyhow::Result;
use serde::Serialize;
use sieve_core::config::Config;
use sieve_core::llm::Model;
use sieve_core::store::{Store, now};
use sieve_core::triage::{self, TriagePrompt};
use sieve_core::{followup, imap_sync, summary};

#[derive(Debug, Default, Clone, Serialize)]
pub struct Status {
    pub syncing: bool,
    pub last_sync: i64,
    pub last_error: Option<String>,
    pub triaged: usize,
    pub model: String,
    pub model_ready: bool,
}

pub struct State {
    pub store: Mutex<Store>,
    /// The database file: sync opens its own connection, so the app is not kept waiting on the network.
    pub db_path: Option<std::path::PathBuf>,
    pub model: Arc<dyn Model>,
    pub cfg: Config,
    pub status: Mutex<Status>,
    nudge: (Mutex<bool>, Condvar),
}

impl State {
    pub fn new(store: Store, db_path: Option<std::path::PathBuf>, model: Arc<dyn Model>, cfg: Config) -> Arc<State> {
        let status = Status { model: model.name(), model_ready: true, ..Default::default() };
        Arc::new(State {
            store: Mutex::new(store),
            db_path,
            model,
            cfg,
            status: Mutex::new(status),
            nudge: (Mutex::new(false), Condvar::new()),
        })
    }

    /// Wakes the worker now (e.g. "check for new mail").
    pub fn nudge(&self) {
        *self.nudge.0.lock().unwrap() = true;
        self.nudge.1.notify_all();
    }

    fn wait(&self, d: Duration) {
        let g = self.nudge.0.lock().unwrap();
        let (mut g, _) = self.nudge.1.wait_timeout_while(g, d, |n| !*n).unwrap();
        *g = false;
    }

    fn error(&self, e: impl std::fmt::Display) {
        tracing::warn!("{e}");
        self.status.lock().unwrap().last_error = Some(e.to_string());
    }
}

/// One round: sync, triage, follow-ups, summaries.
pub fn round(state: &State) -> Result<()> {
    let accounts = state.store.lock().unwrap().accounts()?;
    state.status.lock().unwrap().syncing = true;
    let mut errors = vec![];
    for a in &accounts {
        let pw = match imap_sync::password(a) {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("{}: {e}", a.email));
                continue;
            }
        };
        let r = match &state.db_path {
            Some(p) => Store::open(p).and_then(|own| imap_sync::sync(&own, a, &pw, state.cfg.initial_messages)),
            None => imap_sync::sync(&state.store.lock().unwrap(), a, &pw, state.cfg.initial_messages),
        };
        match r {
            Ok(r) if r.new_messages > 0 => tracing::info!("{}: {} new", a.email, r.new_messages),
            Ok(_) => {}
            Err(e) => errors.push(format!("{}: {e:#}", a.email)),
        }
    }
    {
        let mut st = state.status.lock().unwrap();
        st.syncing = false;
        st.last_sync = now();
        st.last_error = if errors.is_empty() { None } else { Some(errors.join("; ")) };
    }

    // Triage, newest first, in small batches so the app sees results as they come.
    let prompt = TriagePrompt::default();
    loop {
        let batch = state.store.lock().unwrap().untriaged(8)?;
        if batch.is_empty() {
            break;
        }
        for m in batch {
            let reader = accounts.iter().find(|a| a.id == m.account_id).map(|a| a.email.clone()).unwrap_or_default();
            match triage::classify(state.model.as_ref(), &m, &reader, &prompt) {
                Ok(t) => {
                    state.store.lock().unwrap().set_triage(&t)?;
                    state.status.lock().unwrap().triaged += 1;
                }
                Err(e) => {
                    state.error(format!("triage: {e:#}"));
                    return Ok(());
                }
            }
        }
    }

    // Follow-ups: new sent messages that expect an answer; answered ones close.
    {
        let pending = state.store.lock().unwrap().unchecked_outgoing(20)?;
        for m in pending {
            let e = match followup::expects(state.model.as_ref(), &m) {
                Ok(e) => e,
                Err(e) => {
                    state.error(format!("follow-up: {e:#}"));
                    break;
                }
            };
            let store = state.store.lock().unwrap();
            if e.expects_reply && m.date > now() - 30 * 86400 && !store.answered_after(&m.thread_id, m.date)? {
                store.add_followup(&sieve_core::model::FollowUp {
                    id: 0,
                    message_id: m.id,
                    thread_id: m.thread_id.clone(),
                    subject: m.subject.clone(),
                    waiting_on: if e.waiting_on.is_empty() { m.to.first().cloned().unwrap_or_default() } else { e.waiting_on },
                    what: e.what,
                    sent: m.date,
                    due: followup::working_days_after(m.date, state.cfg.followup_days),
                    status: "waiting".into(),
                })?;
            } else {
                store.mark_checked(m.id)?;
            }
        }
        followup::close_answered(&state.store.lock().unwrap())?;
    }

    // Summaries of long threads (a few per round).
    let threads = state.store.lock().unwrap().threads_to_summarize(state.cfg.summary_min_messages, 3)?;
    for t in threads {
        let msgs = state.store.lock().unwrap().thread(&t)?;
        match summary::summarize(state.model.as_ref(), &msgs) {
            Ok(s) => state.store.lock().unwrap().set_summary(&t, msgs.len(), &s, &state.model.name())?,
            Err(e) => {
                state.error(format!("summary: {e:#}"));
                break;
            }
        }
    }
    Ok(())
}

/// Runs rounds forever.
pub fn run(state: Arc<State>) {
    loop {
        if let Err(e) = round(&state) {
            state.error(format!("{e:#}"));
        }
        state.wait(Duration::from_secs(state.cfg.sync_interval.max(10)));
    }
}
