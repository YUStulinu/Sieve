"""Messages from the Enron corpus: real mail, in English, from 2000-2002.

The texts are not in this repository: `bench/enron_*.json` hold only message ids and labels, and
this module rebuilds the sets from the published corpus (one pinned shard, checked by SHA-256).

  python -m sieve_eval.enron candidates   draw the messages to label (deterministic)
  python -m sieve_eval.enron build        rebuild the labelled sets in eval/work from the labels
  python -m sieve_eval.enron drafts       real replies of one mailbox's owner, with the message each answers
  python -m sieve_eval.enron sent         sent messages to label for the follow-up evaluation
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys

import pandas as pd

from .paths import BENCH, ENRON_REVISION, ENRON_SHA256, ENRON_SHARD, WORK

URL = f"https://huggingface.co/datasets/corbt/enron-emails/resolve/{ENRON_REVISION}/data/train-00000-of-00003.parquet"


def load() -> pd.DataFrame:
    if not ENRON_SHARD.exists():
        raise SystemExit(f"download {URL}\n  to {ENRON_SHARD} (or run scripts/fetch-deps.sh --eval)")
    h = hashlib.sha256(ENRON_SHARD.read_bytes()).hexdigest()
    if h != ENRON_SHA256:
        raise SystemExit(f"{ENRON_SHARD} has SHA-256 {h}, expected {ENRON_SHA256}")
    d = pd.read_parquet(ENRON_SHARD)
    d["user"] = d["file_name"].str.split("/").str[0]
    d["folder"] = d["file_name"].str.split("/").str[1]
    # Each mailbox's owner: the most frequent sender in its sent folders.
    sent = d[d["folder"].isin(["sent", "sent_items", "_sent_mail"])]
    owners = sent.groupby("user")["from"].agg(lambda s: s.value_counts().index[0])
    d["owner"] = d["user"].map(owners).fillna("")
    return d


def clean(body: str) -> str:
    """The new text of a message (as Sieve's own cleaning does: up to the quoted original)."""
    b = body.replace("\r\n", "\n")
    m = re.search(r"(?im)^\s*-{2,}\s*Original Message\s*-{2,}|^\s*-{3,} Forwarded by|^.{0,80} on \d\d/\d\d/\d{4} \d\d:\d\d (AM|PM)\s*$", b)
    if m and m.start() > 0:
        b = b[: m.start()]
    return re.sub(r"\n{3,}", "\n\n", b).strip()


def record(r, label: str | None = None) -> dict:
    to = [t for t in (r.to if r.to is not None else []) if t]
    cc = [t for t in (r.cc if r.cc is not None else []) if t]
    return {
        "id": r.message_id,
        "from": r["from"] if isinstance(r, pd.Series) else getattr(r, "_3"),
        "from_name": "",
        "to": list(to),
        "cc": list(cc),
        "subject": r.subject or "",
        "date": int(pd.Timestamp(r.date).timestamp()) if r.date is not None else 0,
        "body": clean(r.body or "")[:6000],
        "attachments": [],
        "list_unsubscribe": False,
        "lang": "en",
        "label": label,
        "owner": r.owner,
    }


def candidates(n_random: int = 110, n_bulk: int = 30, n_money: int = 30, seed: int = 7) -> pd.DataFrame:
    d = load()
    inbox = d[d["folder"].isin(["inbox", "notes_inbox"]) & (d["body"].str.len() > 40)].drop_duplicates("message_id")
    inbox = inbox[~inbox["body"].str.contains("-----Original Message-----", regex=False) | (inbox["body"].str.len() < 4000)]
    bulk = inbox[inbox["subject"].str.contains(r"(?i)newsletter|digest|weekly|daily|update|news|bulletin|offer|subscribe", regex=True)]
    bulk = bulk[bulk["body"].str.contains(r"(?i)unsubscribe|mailing list|to be removed|subscription", regex=True)]
    money = inbox[inbox["subject"].str.contains(r"(?i)invoice|bill|statement|receipt|payment|charge|expense report|order confirm", regex=True)]
    pick = pd.concat([
        inbox.sample(n_random, random_state=seed),
        bulk.sample(min(n_bulk, len(bulk)), random_state=seed),
        money.sample(min(n_money, len(money)), random_state=seed),
    ]).drop_duplicates("message_id")
    return pick.sample(frac=1, random_state=seed)


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("what", choices=["candidates", "build", "drafts", "sent"])
    a = ap.parse_args()
    WORK.mkdir(parents=True, exist_ok=True)
    if a.what == "candidates":
        c = candidates()
        out = WORK / "enron-candidates.jsonl"
        with out.open("w", encoding="utf-8") as f:
            for _, r in c.iterrows():
                f.write(json.dumps(record(r), ensure_ascii=False) + "\n")
        print(f"{len(c)} candidates in {out}")
    elif a.what == "drafts":
        drafts()
    elif a.what == "sent":
        sent_candidates()
    else:
        build()


DRAFT_USER = "germany-c"
MARK = "-----Original Message-----"


def quoted(body: str) -> tuple[str, str, str] | None:
    """The first quoted original in an Outlook-style reply: (from, subject, text)."""
    parts = body.split(MARK)
    if len(parts) < 2:
        return None
    lines = parts[1].strip("\n").split("\n")
    head = {}
    i = 0
    while i < len(lines) and lines[i].strip():
        k, _, v = lines[i].partition(":")
        head[k.strip().lower()] = v.strip()
        i += 1
    if "from" not in head:
        return None
    return head["from"], head.get("subject", ""), "\n".join(lines[i:]).strip()


def drafts(n: int = 40, seed: int = 7) -> None:
    """Pairs (the message, the owner's real reply) from one mailbox; the owner's other sent mail is
    the style corpus. Short business replies, as most mail is."""
    d = load()
    mine = d[(d["user"] == DRAFT_USER) & d["folder"].isin(["sent", "sent_items", "_sent_mail"])].drop_duplicates("message_id")
    owner = mine["owner"].iloc[0]
    pairs = []
    for _, r in mine.iterrows():
        body = r.body or ""
        if "-----Original Message-----" not in body:
            continue
        reply = clean(body)
        q = quoted(body)
        if not q:
            continue
        incoming = re.sub(r"\n{3,}", "\n\n", q[2]).strip()
        rw, iw = len(reply.split()), len(incoming.split())
        if 5 <= rw <= 120 and 10 <= iw <= 400:
            pairs.append((r, reply, q[0], q[1], incoming))
    rng = pd.Series(range(len(pairs))).sample(min(n, len(pairs)), random_state=seed).tolist()
    chosen = [pairs[i] for i in rng]
    chosen_ids = {c[0].message_id for c in chosen}
    # The same message often sits in several sent folders under different ids: leave out every copy of a
    # chosen reply, or the examples would show the model the answer.
    norm = lambda t: " ".join(t.lower().split())
    chosen_bodies = {norm(c[1]) for c in chosen}
    with (WORK / "drafts-enron.jsonl").open("w", encoding="utf-8") as f:
        for r, reply, frm, subject, incoming in chosen:
            addr = re.search(r"[\w.+-]+@[\w.-]+", frm)
            inc = {"id": r.message_id + "#in", "from": addr.group(0).lower() if addr else frm, "from_name": frm.split("<")[0].split("[")[0].strip(" \"'"),
                   "to": [owner], "subject": subject, "body": incoming[:4000], "lang": "en", "owner": owner}
            rep = {**record(r), "body": reply}
            f.write(json.dumps({"incoming": inc, "reply": rep}, ensure_ascii=False) + "\n")
    written = 0
    seen = set(chosen_bodies)
    with (WORK / "sent-enron.jsonl").open("w", encoding="utf-8") as f:
        for _, r in mine.iterrows():
            body = clean(r.body or "")
            if r.message_id not in chosen_ids and norm(body) not in seen and len(body.split()) >= 3:
                seen.add(norm(body))
                written += 1
                f.write(json.dumps({**record(r), "body": clean(r.body or "")}, ensure_ascii=False) + "\n")
    print(f"{len(chosen)} reply pairs of {owner} ({len(pairs)} usable); style corpus: {written} other sent messages (one copy each)")


def sent_candidates(n: int = 60, seed: int = 11) -> None:
    d = load()
    sent = d[d["folder"].isin(["sent", "sent_items", "_sent_mail"])].drop_duplicates("message_id")
    sent = sent[sent["body"].map(lambda b: 5 <= len(clean(b or "").split()) <= 250)]
    pick = sent.sample(n, random_state=seed)
    with (WORK / "followup-enron-candidates.jsonl").open("w", encoding="utf-8") as f:
        for _, r in pick.iterrows():
            f.write(json.dumps({**record(r), "body": clean(r.body or "")}, ensure_ascii=False) + "\n")
    print(f"{len(pick)} sent messages in {WORK / 'followup-enron-candidates.jsonl'}")


def build_followups() -> None:
    d = load().drop_duplicates("message_id").set_index("message_id", drop=False)
    labels = json.loads((BENCH / "enron_followup_labels.json").read_text(encoding="utf-8"))["labels"]
    with (WORK / "followup-enron.jsonl").open("w", encoding="utf-8") as f:
        for mid, label in labels.items():
            r = d.loc[mid]
            f.write(json.dumps({**record(r, label), "body": clean(r.body or ""), "source": "enron"}, ensure_ascii=False) + "\n")


def build() -> None:
    d = load().drop_duplicates("message_id").set_index("message_id", drop=False)
    labels = json.loads((BENCH / "enron_triage_labels.json").read_text(encoding="utf-8"))
    out = WORK / "triage-enron.jsonl"
    with out.open("w", encoding="utf-8") as f:
        for mid, label in labels["labels"].items():
            f.write(json.dumps({**record(d.loc[mid], label), "source": "enron"}, ensure_ascii=False) + "\n")
    print(f"{len(labels['labels'])} labelled Enron messages in {out}")
    if (BENCH / "enron_followup_labels.json").exists():
        build_followups()
    drafts()


if __name__ == "__main__":
    main()
