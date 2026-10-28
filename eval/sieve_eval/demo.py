"""Fills the local test server (scripts/test-server.sh) with Ana Popescu's mailbox: the hand-written
inbox, her sent mail and the threads, dated relative to now — for trying the app and for the
end-to-end check. Nothing here touches a real account.

  python -m sieve_eval.demo [--host 127.0.0.1 --port 3143]
"""

from __future__ import annotations

import argparse
import imaplib
import json
import time
from email.message import EmailMessage
from email.utils import format_datetime, make_msgid
from datetime import datetime, timezone

from .paths import BENCH

ANA = "ana.popescu@sieve.test"


def message(r: dict, date: float, mid: str | None = None, reply_to: str | None = None) -> bytes:
    m = EmailMessage()
    m["From"] = f"{r.get('from_name') or r['from']} <{r['from']}>"
    m["To"] = ", ".join(r.get("to") or [ANA])
    if r.get("cc"):
        m["Cc"] = ", ".join(r["cc"])
    m["Subject"] = r["subject"]
    m["Date"] = format_datetime(datetime.fromtimestamp(date, timezone.utc))
    m["Message-ID"] = mid or make_msgid(domain="sieve.test")
    if reply_to:
        m["In-Reply-To"] = reply_to
        m["References"] = reply_to
    if r.get("list_unsubscribe"):
        m["List-Unsubscribe"] = "<mailto:unsubscribe@lists.example>"
    m.set_content(r["body"])
    for a in r.get("attachments", []):
        m.add_attachment(b"%PDF-1.4 demo\n", maintype="application", subtype="pdf", filename=a)
    return bytes(m)


def read(name: str) -> list[dict]:
    return [json.loads(l) for l in (BENCH / name).read_text(encoding="utf-8").splitlines() if l.strip()]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=3143)
    a = ap.parse_args()
    now = time.time()
    s = imaplib.IMAP4(a.host, a.port)
    s.login(ANA, "test")
    for f in ["Sent", "Drafts"]:
        s.create(f)

    def put(folder: str, raw: bytes, date: float, seen: bool = False):
        s.append(folder, "(\\Seen)" if seen else "", imaplib.Time2Internaldate(date), raw)

    inbox = read("triage_written.jsonl")
    for i, r in enumerate(inbox):
        d = now - (len(inbox) - i) * 2400  # spread over the last three days, newest last
        put("INBOX", message(r, d), d, seen=i < len(inbox) // 2)

    sent = read("ana_sent.jsonl")
    for i, r in enumerate(sent):
        d = now - (6 - i % 6) * 86400 - 3600 * (i % 5)
        put("Sent", message(r, d, mid=f"<{r['id']}@sieve.test>"), d, seen=True)
    # Two of the people she wrote to have answered: those follow-ups close by themselves.
    for sid, who, name, body in [
        ("s001", "mihai.ionescu@clientnord.ro", "Mihai Ionescu", "Salut Ana, mergem pe varianta a doua. Revin cu detalii."),
        ("s013", "tom.baker@northwind.co.uk", "Tom Baker", "Hi Ana, comments are attached. Looks great!"),
    ]:
        orig = next(r for r in sent if r["id"] == sid)
        d = now - 3600
        put("INBOX", message({"from": who, "from_name": name, "subject": "Re: " + orig["subject"], "body": body}, d,
                             reply_to=f"<{sid}@sieve.test>"), d)

    threads = read("threads.jsonl")
    for t in sorted({r["thread"] for r in threads}):
        msgs = [r for r in threads if r["thread"] == t]
        prev = None
        for j, r in enumerate(msgs):
            d = now - 5 * 86400 + j * 7200 + int(t[1:]) * 600
            mid = f"<{r['id']}@sieve.test>"
            folder = "Sent" if r["from"] == ANA else "INBOX"
            put(folder, message(r, d, mid=mid, reply_to=prev), d, seen=True)
            prev = mid
    s.logout()
    print(f"{len(inbox) + 2 + sum(r['from'] != ANA for r in threads)} incoming and {len(sent) + sum(r['from'] == ANA for r in threads)} sent messages "
          f"for {ANA} on {a.host}:{a.port}")


if __name__ == "__main__":
    main()
