//! The local API the desktop app uses: on 127.0.0.1 only, every request carrying the token from
//! the data folder (`api-token`), which only the user's own programs can read.

use std::sync::Arc;

use axum::extract::{Path, Query, Request, State as AxState};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use sieve_core::model::Category;
use sieve_core::store::{Account, now};
use sieve_core::{drafts, imap_sync, send, style, summary};
use tower_http::cors::CorsLayer;

use crate::daemon::State;

type S = Arc<State>;

pub struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(json!({"error": format!("{:#}", self.0)}))).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self {
        ApiError(e.into())
    }
}

type R = Result<Json<Value>, ApiError>;

/// Runs blocking work (the database, the model, the network) off the async threads.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> Result<T, ApiError> {
    Ok(tokio::task::spawn_blocking(f).await.map_err(anyhow::Error::from)??)
}

pub fn router(state: S, token: String) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            ["tauri://localhost", "http://tauri.localhost", "https://tauri.localhost", "http://localhost:1420"]
                .map(|o| o.parse::<HeaderValue>().unwrap()),
        )
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);
    let token = Arc::new(token);
    Router::new()
        .route("/api/status", get(status))
        .route("/api/inbox", get(inbox))
        .route("/api/search", get(search))
        .route("/api/messages/{id}", get(message))
        .route("/api/messages/{id}/label", post(label))
        .route("/api/messages/{id}/draft", post(make_draft))
        .route("/api/label/next", get(next_unlabelled))
        .route("/api/drafts/{id}", post(edit_draft))
        .route("/api/drafts/{id}/save", post(save_draft))
        .route("/api/drafts/{id}/send", post(send_draft))
        .route("/api/drafts/{id}/discard", post(discard_draft))
        .route("/api/threads/{id}/summarize", post(summarize))
        .route("/api/followups", get(followups))
        .route("/api/followups/{id}", post(followup_action))
        .route("/api/accounts", get(accounts).post(add_account))
        .route("/api/accounts/{id}/delete", post(delete_account))
        .route("/api/sync", post(sync_now))
        .route("/api/style", get(get_style))
        .layer(middleware::from_fn(move |req: Request, next: Next| {
            let token = token.clone();
            async move {
                if req.method() == Method::OPTIONS {
                    return next.run(req).await;
                }
                let ok = req
                    .headers()
                    .get(header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.strip_prefix("Bearer ").is_some_and(|t| constant_eq(t, &token)));
                if ok { next.run(req).await } else { (StatusCode::UNAUTHORIZED, "token required").into_response() }
            }
        }))
        .layer(cors)
        .with_state(state)
}

fn constant_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn brief(m: &sieve_core::Message) -> Value {
    json!({
        "id": m.id, "thread_id": m.thread_id, "from": m.from_addr, "from_name": m.from_name, "to": m.to, "cc": m.cc,
        "subject": m.subject, "date": m.date, "lang": m.lang, "seen": m.seen, "outgoing": m.outgoing,
        "snippet": m.body.chars().take(160).collect::<String>().replace('\n', " "),
        "attachments": m.attachments, "list_unsubscribe": m.list_unsubscribe,
    })
}

async fn status(AxState(s): AxState<S>) -> R {
    let st = s.status.lock().unwrap().clone();
    let s2 = s.clone();
    let counts = blocking(move || s2.store.lock().unwrap().counts()).await?;
    Ok(Json(json!({"status": st, "counts": counts})))
}

#[derive(Deserialize)]
struct InboxQ {
    category: Option<String>,
    limit: Option<usize>,
}

async fn inbox(AxState(s): AxState<S>, Query(q): Query<InboxQ>) -> R {
    let cat = q.category.as_deref().and_then(Category::parse);
    let rows = blocking(move || s.store.lock().unwrap().inbox(cat, q.limit.unwrap_or(200))).await?;
    Ok(Json(json!(
        rows.into_iter()
            .map(|(m, t, l)| {
                let mut b = brief(&m);
                b["category"] = json!(t.as_ref().map(|t| t.category));
                b["reason"] = json!(t.as_ref().map(|t| t.reason.clone()));
                b["label"] = json!(l);
                b
            })
            .collect::<Vec<_>>()
    )))
}

#[derive(Deserialize)]
struct SearchQ {
    q: String,
}

async fn search(AxState(s): AxState<S>, Query(q): Query<SearchQ>) -> R {
    let rows = blocking(move || s.store.lock().unwrap().search(&q.q, 50)).await?;
    Ok(Json(json!(rows.iter().map(brief).collect::<Vec<_>>())))
}

async fn message(AxState(s): AxState<S>, Path(id): Path<i64>) -> R {
    blocking(move || {
        let st = s.store.lock().unwrap();
        let m = st.message(id)?.ok_or_else(|| anyhow::anyhow!("no message {id}"))?;
        let thread = st.thread(&m.thread_id)?;
        let summary = st.summary(&m.thread_id)?;
        let drafts = st.drafts_for(id)?;
        let triage = st.inbox(None, 100_000)?.into_iter().find(|(x, _, _)| x.id == id).and_then(|(_, t, l)| Some((t, l)));
        Ok(Json(json!({
            "message": m,
            "thread": thread.iter().map(|t| json!({"brief": brief(t), "body": t.body})).collect::<Vec<_>>(),
            "summary": summary.map(|(n, v)| json!({"messages": n, "summary": v})),
            "drafts": drafts,
            "category": triage.as_ref().and_then(|(t, _)| t.as_ref().map(|t| t.category)),
            "reason": triage.as_ref().and_then(|(t, _)| t.as_ref().map(|t| t.reason.clone())),
            "label": triage.and_then(|(_, l)| l),
        })))
    })
    .await
}

#[derive(Deserialize)]
struct LabelBody {
    category: Option<String>,
}

async fn label(AxState(s): AxState<S>, Path(id): Path<i64>, Json(b): Json<LabelBody>) -> R {
    let c = match b.category.as_deref() {
        None => None,
        Some(x) => Some(Category::parse(x).ok_or_else(|| anyhow::anyhow!("unknown category {x}"))?),
    };
    blocking(move || s.store.lock().unwrap().set_label(id, c)).await?;
    Ok(Json(json!({"ok": true})))
}

async fn next_unlabelled(AxState(s): AxState<S>) -> R {
    blocking(move || {
        let st = s.store.lock().unwrap();
        let m = st.next_unlabelled()?;
        let labelled = st.counts()?["labelled"].clone();
        Ok(Json(json!({"message": m, "labelled": labelled})))
    })
    .await
}

async fn make_draft(AxState(s): AxState<S>, Path(id): Path<i64>) -> R {
    blocking(move || {
        let opts = drafts::DraftOptions::default();
        let (m, owner, sent, ctx) = {
            let st = s.store.lock().unwrap();
            let m = st.message(id)?.ok_or_else(|| anyhow::anyhow!("no message {id}"))?;
            let a = st.account(m.account_id)?.ok_or_else(|| anyhow::anyhow!("no account"))?;
            let sent = st.sent(a.id, 500)?;
            let ctx = drafts::context(&st, &m, &opts)?;
            (m, format!("{} <{}>", a.name, a.email), sent, ctx)
        };
        let learned = style::learn(&sent, &owner);
        // The model call is slow: the store is not locked meanwhile.
        let p = drafts::compose(s.model.as_ref(), &owner, Some(&learned), &m, &ctx, &opts)?;
        let st = s.store.lock().unwrap();
        let did = st.add_draft(id, &p.to, &p.subject, &p.body)?;
        Ok(Json(json!(st.draft(did)?)))
    })
    .await
}

#[derive(Deserialize)]
struct EditBody {
    body: String,
}

async fn edit_draft(AxState(s): AxState<S>, Path(id): Path<i64>, Json(b): Json<EditBody>) -> R {
    blocking(move || {
        let st = s.store.lock().unwrap();
        st.update_draft(id, Some(&b.body), None)?;
        Ok(Json(json!(st.draft(id)?)))
    })
    .await
}

async fn save_draft(AxState(s): AxState<S>, Path(id): Path<i64>) -> R {
    blocking(move || {
        let st = s.store.lock().unwrap();
        let d = st.draft(id)?.ok_or_else(|| anyhow::anyhow!("no draft {id}"))?;
        send::save_to_drafts(&st, &d)?;
        Ok(Json(json!(st.draft(id)?)))
    })
    .await
}

#[derive(Deserialize)]
struct SendBody {
    confirmation: String,
}

/// The only way Sieve sends mail: the app shows you the recipients and the text, you press Send,
/// and the request carries the confirmation string for exactly that draft.
async fn send_draft(AxState(s): AxState<S>, Path(id): Path<i64>, Json(b): Json<SendBody>) -> R {
    blocking(move || {
        let st = s.store.lock().unwrap();
        let d = st.draft(id)?.ok_or_else(|| anyhow::anyhow!("no draft {id}"))?;
        send::send(&st, &d, &b.confirmation)?;
        tracing::info!("draft {id} sent to {:?} (confirmed)", d.to);
        Ok(Json(json!(st.draft(id)?)))
    })
    .await
}

async fn discard_draft(AxState(s): AxState<S>, Path(id): Path<i64>) -> R {
    blocking(move || {
        s.store.lock().unwrap().update_draft(id, None, Some("discarded"))?;
        Ok(Json(json!({"ok": true})))
    })
    .await
}

async fn summarize(AxState(s): AxState<S>, Path(id): Path<String>) -> R {
    blocking(move || {
        let msgs = s.store.lock().unwrap().thread(&id)?;
        anyhow::ensure!(!msgs.is_empty(), "no thread {id}");
        let v = summary::summarize(s.model.as_ref(), &msgs)?;
        s.store.lock().unwrap().set_summary(&id, msgs.len(), &v, &s.model.name())?;
        Ok(Json(json!({"messages": msgs.len(), "summary": v})))
    })
    .await
}

#[derive(Deserialize)]
struct FollowQ {
    status: Option<String>,
}

async fn followups(AxState(s): AxState<S>, Query(q): Query<FollowQ>) -> R {
    let rows = blocking(move || s.store.lock().unwrap().followups(q.status.as_deref())).await?;
    let t = now();
    Ok(Json(json!(
        rows.into_iter()
            .map(|f| {
                let due = f.status == "waiting" && f.due <= t;
                let mut v = json!(f);
                v["overdue"] = json!(due);
                v
            })
            .collect::<Vec<_>>()
    )))
}

#[derive(Deserialize)]
struct FollowAction {
    action: String,
    days: Option<i64>,
}

async fn followup_action(AxState(s): AxState<S>, Path(id): Path<i64>, Json(b): Json<FollowAction>) -> R {
    blocking(move || {
        let st = s.store.lock().unwrap();
        match b.action.as_str() {
            "dismiss" => st.set_followup_status(id, "dismissed")?,
            "done" => st.set_followup_status(id, "replied")?,
            "snooze" => st.snooze_followup(id, now() + b.days.unwrap_or(2) * 86400)?,
            other => anyhow::bail!("unknown action {other}"),
        }
        Ok(Json(json!({"ok": true})))
    })
    .await
}

async fn accounts(AxState(s): AxState<S>) -> R {
    let a = blocking(move || s.store.lock().unwrap().accounts()).await?;
    Ok(Json(json!(a)))
}

#[derive(Deserialize)]
struct NewAccount {
    #[serde(flatten)]
    account: Account,
    /// Goes to the system's credential store, never to the database or a file.
    password: String,
}

async fn add_account(AxState(s): AxState<S>, Json(b): Json<NewAccount>) -> R {
    let s2 = s.clone();
    let id = blocking(move || {
        // Check the settings before keeping them: a login that fails is an error now, not later.
        let mut a = b.account.clone();
        a.id = 0;
        imap_sync::connect(&a, &b.password)?.logout().ok();
        imap_sync::store_password(&a.email, &b.password)?;
        s2.store.lock().unwrap().add_account(&a)
    })
    .await?;
    s.nudge();
    Ok(Json(json!({"id": id})))
}

async fn delete_account(AxState(s): AxState<S>, Path(id): Path<i64>) -> R {
    blocking(move || {
        let st = s.store.lock().unwrap();
        if let Some(a) = st.account(id)? {
            imap_sync::delete_password(&a.email)?;
        }
        st.delete_account(id)?;
        Ok(Json(json!({"ok": true})))
    })
    .await
}

async fn sync_now(AxState(s): AxState<S>) -> R {
    s.nudge();
    Ok(Json(json!({"ok": true})))
}

async fn get_style(AxState(s): AxState<S>) -> R {
    blocking(move || {
        let st = s.store.lock().unwrap();
        let mut out = vec![];
        for a in st.accounts()? {
            let owner = format!("{} <{}>", a.name, a.email);
            out.push(json!({"account": a.email, "style": style::learn(&st.sent(a.id, 500)?, &owner)}));
        }
        Ok(Json(json!(out)))
    })
    .await
}
