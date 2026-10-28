//! The local API with a scripted model and an in-memory store: the token is required, the inbox and
//! labels work, a draft is proposed, and sending without the exact confirmation is refused.

use std::sync::Arc;

use serde_json::{Value, json};
use sieve_core::config::Config;
use sieve_core::llm::Scripted;
use sieve_core::model::{Category, Message, Triage};
use sieve_core::store::{Account, Store};
use sieved::{api, daemon};

async fn serve() -> (String, String, Arc<daemon::State>) {
    let store = Store::memory().unwrap();
    let a = store
        .add_account(&Account {
            id: 0,
            name: "Ana Popescu".into(),
            email: "ana@sieve.test".into(),
            imap_host: "127.0.0.1".into(),
            imap_port: 1,
            imap_security: "none".into(),
            smtp_host: "127.0.0.1".into(),
            smtp_port: 1,
            smtp_security: "none".into(),
            username: "ana@sieve.test".into(),
        })
        .unwrap();
    for (i, (from, subject, body, cat)) in [
        ("mihai@client.test", "Oferta", "Poți să-mi trimiți oferta până mâine?", Category::ReplyToday),
        ("news@shop.test", "Reduceri", "Doar azi -30%.", Category::Newsletter),
    ]
    .into_iter()
    .enumerate()
    {
        let id = store
            .insert_message(&Message {
                account_id: a,
                folder: "INBOX".into(),
                uid: i as u32 + 1,
                message_id: format!("m{i}@test"),
                thread_id: format!("m{i}@test"),
                from_addr: from.into(),
                subject: subject.into(),
                body: body.into(),
                lang: "ro".into(),
                date: 1_793_000_000 + i as i64,
                ..Default::default()
            })
            .unwrap()
            .unwrap();
        store
            .set_triage(&Triage {
                message_id: id,
                category: cat,
                reason: "r".into(),
                model: "scripted".into(),
                prompt: "default".into(),
                ms: 1,
            })
            .unwrap();
    }
    let model = Arc::new(Scripted(vec![("Oferta".into(), "Bună Mihai,\n\nȚi-o trimit mâine dimineață.\n\nAna".into())]));
    let state = daemon::State::new(store, None, model, Config::default());
    let token = "t0ken".to_string();
    let app = api::router(state.clone(), token.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, token, state)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_api_needs_the_token_and_never_sends_unconfirmed() {
    let (url, token, _state) = serve().await;
    let c = reqwest::Client::new();
    let get = |p: &str| c.get(format!("{url}{p}")).bearer_auth(&token);

    // No token, wrong token: refused.
    assert_eq!(c.get(format!("{url}/api/status")).send().await.unwrap().status(), 401);
    assert_eq!(c.get(format!("{url}/api/status")).bearer_auth("nope").send().await.unwrap().status(), 401);

    let st: Value = get("/api/status").send().await.unwrap().json().await.unwrap();
    assert_eq!(st["counts"]["by_category"]["reply_today"], 1);

    let today: Vec<Value> = get("/api/inbox?category=reply_today").send().await.unwrap().json().await.unwrap();
    assert_eq!(today.len(), 1);
    let id = today[0]["id"].as_i64().unwrap();
    assert_eq!(today[0]["subject"], "Oferta");

    // Labels: set, then the message is no longer the next to label.
    let r = c
        .post(format!("{url}/api/messages/{id}/label"))
        .bearer_auth(&token)
        .json(&json!({"category": "reply_today"}))
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success());
    let next: Value = get("/api/label/next").send().await.unwrap().json().await.unwrap();
    assert_eq!(next["labelled"], 1);
    assert_ne!(next["message"]["id"].as_i64(), Some(id));

    // A draft is proposed in the owner's name, addressed to the sender.
    let d: Value = c.post(format!("{url}/api/messages/{id}/draft")).bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(d["status"], "proposed");
    assert_eq!(d["to"], json!(["mihai@client.test"]));
    assert_eq!(d["subject"], "Re: Oferta");
    assert!(d["body"].as_str().unwrap().contains("mâine dimineață"));

    // Sending with a confirmation for other recipients is refused, and nothing changes.
    let did = d["id"].as_i64().unwrap();
    let r = c
        .post(format!("{url}/api/drafts/{did}/send"))
        .bearer_auth(&token)
        .json(&json!({"confirmation": format!("{did}:someone@else.test")}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    let detail: Value = get(&format!("/api/messages/{id}")).send().await.unwrap().json().await.unwrap();
    assert_eq!(detail["drafts"][0]["status"], "proposed");

    // Accounts never come back with a password.
    let accounts: Value = get("/api/accounts").send().await.unwrap().json().await.unwrap();
    assert!(!accounts.to_string().to_lowercase().contains("password"));
}
