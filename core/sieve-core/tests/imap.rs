//! Against a real IMAP/SMTP server (GreenMail; `scripts/test-server.sh` starts one): a mailbox is
//! filled, synced, triaged and followed up with a scripted model; a draft goes to Drafts; sending
//! needs the right confirmation, and the sent message arrives.
//!
//! Runs when SIEVE_TEST_IMAP is set (e.g. 127.0.0.1:3143, with SMTP on 3025); skipped otherwise.

use mail_builder::MessageBuilder;
use sieve_core::followup;
use sieve_core::llm::Scripted;
use sieve_core::model::Category;
use sieve_core::store::{Account, Store};
use sieve_core::triage::{self, TriagePrompt};
use sieve_core::{imap_sync, send};

fn server() -> Option<(String, u16, u16)> {
    let v = std::env::var("SIEVE_TEST_IMAP").ok()?;
    let (host, port) = v.split_once(':')?;
    let smtp = std::env::var("SIEVE_TEST_SMTP_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3025);
    Some((host.to_string(), port.parse().ok()?, smtp))
}

fn account(host: &str, imap: u16, smtp: u16, user: &str) -> Account {
    Account {
        id: 0,
        name: "Ana Popescu".into(),
        email: format!("{user}@sieve.test"),
        imap_host: host.into(),
        imap_port: imap,
        imap_security: "none".into(),
        smtp_host: host.into(),
        smtp_port: smtp,
        smtp_security: "none".into(),
        username: format!("{user}@sieve.test"),
    }
}

fn raw(from: (&str, &str), to: &str, subject: &str, body: &str, id: &str, reply_to: Option<&str>, unsubscribe: bool, date: i64) -> Vec<u8> {
    let mut b = MessageBuilder::new().from(from).to(to).subject(subject).text_body(body).message_id(id).date(date);
    if let Some(r) = reply_to {
        b = b.in_reply_to(r).references(r);
    }
    if unsubscribe {
        b = b.header("List-Unsubscribe", mail_builder::headers::raw::Raw::new("<mailto:unsub@shop.test>"));
    }
    b.write_to_vec().unwrap()
}

#[test]
fn sync_triage_followup_draft_and_confirmed_send() {
    let Some((host, imap, smtp)) = server() else {
        eprintln!("skipped: SIEVE_TEST_IMAP not set");
        return;
    };
    let user = format!("ana{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    let mut a = account(&host, imap, smtp, &user);
    let pw = "test";
    let now = chrono::Utc::now().timestamp();

    // The mailbox: three incoming messages and one sent message that asks something.
    let mut s = imap_sync::connect(&a, pw).expect("login");
    let folders = imap_sync::folders(&mut s).unwrap();
    let sent_folder = folders.iter().find(|(_, r)| *r == "sent").unwrap().0.clone();
    s.append(
        "INBOX",
        &raw(
            ("Mihai", "mihai@client.test"),
            &a.email,
            "Oferta",
            "Bună Ana, poți să-mi trimiți oferta până vineri?",
            "<m1@client.test>",
            None,
            false,
            now - 3600,
        ),
    )
    .unwrap();
    s.append(
        "INBOX",
        &raw(
            ("Shop", "news@shop.test"),
            &a.email,
            "Reduceri de toamnă",
            "Doar azi: -30% la tot.",
            "<n1@shop.test>",
            None,
            true,
            now - 1800,
        ),
    )
    .unwrap();
    s.append(
        &sent_folder,
        &raw(
            ("Ana Popescu", &a.email),
            "dan@partner.test",
            "Contract",
            "Salut Dan, poți să-mi confirmi data semnării?",
            "<s1@sieve.test>",
            None,
            false,
            now - 7200,
        ),
    )
    .unwrap();
    s.append(
        "INBOX",
        &raw(
            ("Dan", "dan@partner.test"),
            &a.email,
            "Re: Contract",
            "Da, joi la 10.",
            "<d2@partner.test>",
            Some("<s1@sieve.test>"),
            false,
            now - 600,
        ),
    )
    .unwrap();
    s.logout().ok();

    let store = Store::memory().unwrap();
    a.id = store.add_account(&a).unwrap();
    let r = imap_sync::sync(&store, &a, pw, 100).unwrap();
    assert_eq!(r.new_messages, 4);
    // A second sync finds nothing new.
    assert_eq!(imap_sync::sync(&store, &a, pw, 100).unwrap().new_messages, 0);

    // Dan's answer joined the thread of Ana's message.
    let sent = store.sent(a.id, 10).unwrap();
    assert_eq!(sent.len(), 1);
    let thread = store.thread(&sent[0].thread_id).unwrap();
    assert_eq!(thread.len(), 2, "{thread:?}");

    // Triage with a scripted model.
    let model = Scripted(vec![
        ("Oferta".into(), r#"{"reason":"Mihai asks for the offer.","category":"reply_today"}"#.into()),
        ("Reduceri".into(), r#"{"reason":"A promotion.","category":"newsletter"}"#.into()),
        ("Contract".into(), r#"{"reason":"Dan confirms.","category":"informative"}"#.into()),
        ("confirmi".into(), r#"{"expects_reply":true,"waiting_on":"Dan","what":"data semnării"}"#.into()),
    ]);
    for m in store.untriaged(10).unwrap() {
        let t = triage::classify(&model, &m, &a.email, &TriagePrompt::default()).unwrap();
        store.set_triage(&t).unwrap();
    }
    let inbox = store.inbox(Some(Category::ReplyToday), 10).unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].0.subject, "Oferta");
    assert_eq!(store.inbox(Some(Category::Newsletter), 10).unwrap()[0].0.list_unsubscribe, true);

    // Ana asked Dan something; Dan already answered, so no follow-up stays open.
    assert_eq!(followup::scan(&model, &store, 3, 10).unwrap(), 0);
    assert!(store.followups(None).unwrap().is_empty());

    // A draft for Mihai's message: saved to Drafts, then sent only with the right confirmation.
    let mihai = inbox[0].0.clone();
    let id = store.add_draft(mihai.id, &[mihai.from_addr.clone()], "Re: Oferta", "Bună Mihai,\n\nȚi-o trimit mâine.\n\nAna").unwrap();
    let d = store.draft(id).unwrap().unwrap();
    // SAFETY: this test binary has one test; nothing else reads the environment concurrently.
    unsafe { std::env::set_var("SIEVE_PASSWORD", pw) };
    send::save_to_drafts(&store, &d).unwrap();
    let mut s = imap_sync::connect(&a, pw).unwrap();
    let drafts_folder = imap_sync::folders(&mut s).unwrap().into_iter().find(|(_, r)| *r == "drafts").unwrap().0;
    assert_eq!(s.select(&drafts_folder).unwrap().exists, 1);
    s.logout().ok();

    let d = store.draft(id).unwrap().unwrap();
    assert!(send::send(&store, &d, "wrong").is_err(), "sending without the confirmation must fail");
    assert!(send::send(&store, &d, &format!("{}:someone@else.test", d.id)).is_err());
    send::send(&store, &d, &send::confirmation_for(&d)).unwrap();
    assert_eq!(store.draft(id).unwrap().unwrap().status, "sent");
    assert!(send::send(&store, &store.draft(id).unwrap().unwrap(), &send::confirmation_for(&d)).is_err(), "never twice");

    // It arrived, threaded under Mihai's message.
    let mihai_account =
        Account { email: "mihai@client.test".into(), username: "mihai@client.test".into(), ..account(&host, imap, smtp, "mihai") };
    let mut s = imap_sync::connect(&mihai_account, pw).unwrap();
    s.select("INBOX").unwrap();
    let uids = s.uid_search("SUBJECT \"Re: Oferta\"").unwrap();
    assert!(!uids.is_empty(), "the reply reached Mihai");
    let f = s.uid_fetch(uids.iter().next().unwrap().to_string(), "BODY.PEEK[]").unwrap();
    let got = sieve_core::mail::parse(f.iter().next().unwrap().body().unwrap(), "").unwrap();
    assert_eq!(got.in_reply_to, "m1@client.test");
    assert!(got.body.contains("Ți-o trimit mâine."));
    s.logout().ok();
}
