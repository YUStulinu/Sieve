"""Tests of the evaluation's own code: measures, baselines, splits, the bench files, the demo mailbox."""

import email
import json

import pytest

from sieve_eval import engine, metrics, triage
from sieve_eval.demo import message
from sieve_eval.paths import BENCH


def test_confusion_and_per_class():
    t = ["reply_today", "reply_today", "informative", "newsletter", "invoice"]
    p = ["reply_today", "informative", "informative", "newsletter", "newsletter"]
    m = metrics.confusion(t, p)
    assert m.sum() == 5 and m[0, 0] == 1 and m[0, 1] == 1 and m[3, 2] == 1
    pc = metrics.per_class(t, p)
    assert pc["reply_today"] == {"precision": 1.0, "recall": 0.5, "f1": pytest.approx(2 / 3), "support": 2}
    assert pc["invoice"]["recall"] == 0.0
    assert metrics.accuracy(t, p) == pytest.approx(0.6)


def test_macro_f1_ignores_absent_classes_and_bootstrap_brackets_the_value():
    t = ["reply_today"] * 10 + ["informative"] * 10
    p = ["reply_today"] * 10 + ["informative"] * 8 + ["reply_today"] * 2
    f = metrics.macro_f1(t, p)
    assert 0.8 < f < 1.0
    lo, hi = metrics.bootstrap(t, p, metrics.accuracy, n=500)
    assert lo <= metrics.accuracy(t, p) <= hi


def test_chrf():
    assert engine.chrf("Thanks, see you Monday.", "Thanks, see you Monday.") == pytest.approx(100.0)
    assert engine.chrf("Thanks, see you Monday.", "Quarterly revenue grew strongly.") < 30
    assert engine.chrf("", "anything") == 0.0
    assert engine.first_word("Hi Dan,\n\nok") == "hi"
    assert engine.last_line("ok\n\nBest,\nAna") == "ana"


def test_follow_up_measures():
    r = engine.prf([True, True, False, False], [True, False, True, False])
    assert r["precision"] == 0.5 and r["recall"] == 0.5 and r["accuracy"] == 0.5


def test_rules_baseline():
    assert triage.rules({"subject": "Weekly digest", "body": "...", "list_unsubscribe": True}) == "newsletter"
    assert triage.rules({"subject": "Factura Enel", "body": "Total 100 lei", "list_unsubscribe": False}) == "invoice"
    assert triage.rules({"subject": "Hi", "body": "Can you send it?", "list_unsubscribe": False}) == "reply_today"
    assert triage.rules({"subject": "Update", "body": "Done.", "list_unsubscribe": False}) == "informative"


def test_split_is_fixed_and_about_40_60():
    rows = [{"id": f"x{i}"} for i in range(2000)]
    dev = sum(triage.split(r) == "dev" for r in rows)
    assert 0.35 < dev / 2000 < 0.45
    assert [triage.split(r) for r in rows[:50]] == [triage.split(r) for r in rows[:50]]


def test_bench_files_are_consistent():
    written = [json.loads(l) for l in (BENCH / "triage_written.jsonl").read_text(encoding="utf-8").splitlines() if l.strip()]
    assert len(written) == 120 and len({r["id"] for r in written}) == 120
    assert {r["label"] for r in written} == set(metrics.CATEGORIES)
    assert {r["lang"] for r in written} == {"ro", "en"}
    labels = json.loads((BENCH / "enron_triage_labels.json").read_text(encoding="utf-8"))["labels"]
    assert len(labels) == 170 and set(labels.values()) <= set(metrics.CATEGORIES)
    follow = json.loads((BENCH / "enron_followup_labels.json").read_text(encoding="utf-8"))["labels"]
    assert set(follow.values()) == {"expects_reply", "no_reply"}
    sent = {r["id"] for r in map(json.loads, (BENCH / "ana_sent.jsonl").read_text(encoding="utf-8").splitlines())}
    ids = {r["id"] for r in written}
    for s, w in engine.ANA_PAIRS.items():
        assert s in sent and w in ids
    facts = json.loads((BENCH / "threads_facts.json").read_text(encoding="utf-8"))
    threads = {json.loads(l)["thread"] for l in (BENCH / "threads.jsonl").read_text(encoding="utf-8").splitlines()}
    assert set(facts) == threads


def test_demo_messages_carry_the_headers_sieve_reads():
    raw = message({"from": "news@shop.test", "from_name": "Shop", "subject": "Reduceri", "body": "Doar azi.",
                   "list_unsubscribe": True, "attachments": ["catalog.pdf"]}, 1_793_000_000, mid="<n1@test>", reply_to="<m0@test>")
    m = email.message_from_bytes(raw)
    assert m["List-Unsubscribe"] and m["Message-ID"] == "<n1@test>" and m["In-Reply-To"] == "<m0@test>"
    assert any(p.get_filename() == "catalog.pdf" for p in m.walk())
