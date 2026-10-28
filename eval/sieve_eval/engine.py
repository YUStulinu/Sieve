"""The other evaluations, each through the daemon's own code (`sieve` commands):

  python -m sieve_eval.engine followup  --model M.gguf    does a sent message expect an answer?
  python -m sieve_eval.engine drafts    --model M.gguf    replies in the owner's style, against their real replies
  python -m sieve_eval.engine summaries --model M.gguf    thread summaries: facts kept, numbers invented, language
  python -m sieve_eval.engine report                      tables from what was run

Results: eval/results/{followup,drafts,summaries}.json.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import statistics
import subprocess
import sys
from collections import Counter
from pathlib import Path

from .paths import BENCH, RESULTS, WORK, sieve_cli
from .server import llama


def jl(p: Path) -> list[dict]:
    return [json.loads(l) for l in p.read_text(encoding="utf-8").splitlines() if l.strip()]


def write_jl(p: Path, rows: list[dict]) -> Path:
    p.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows), encoding="utf-8")
    return p


def sieve(args: list[str], url: str, model: Path) -> None:
    subprocess.run([sieve_cli(), *args, "--url", url], check=True, env={**os.environ, "SIEVE_MODEL_NAME": model.stem})


# --- follow-ups ---------------------------------------------------------------------------------

def followup(model: Path, url: str) -> dict:
    rows = jl(BENCH / "ana_sent.jsonl") + jl(WORK / "followup-enron.jsonl")
    for r in rows:
        r.setdefault("source", "written")
    src = write_jl(WORK / "followup-all.jsonl", rows)
    out = {}
    for name, extra in [("screen+model", []), ("model only", ["--no-screen"])]:
        p = WORK / f"followup-{model.stem}-{'screen' if not extra else 'noscreen'}.jsonl"
        sieve(["followup-eval", "--input", str(src), "--output", str(p), *extra], url, model)
        preds = {x["id"]: x for x in jl(p)}
        out[name] = followup_measure(rows, preds)
    return out


def prf(truth: list[bool], pred: list[bool]) -> dict:
    tp = sum(t and p for t, p in zip(truth, pred))
    fp = sum(p and not t for t, p in zip(truth, pred))
    fn = sum(t and not p for t, p in zip(truth, pred))
    pr = tp / (tp + fp) if tp + fp else 0.0
    rc = tp / (tp + fn) if tp + fn else 0.0
    return {"precision": pr, "recall": rc, "f1": 2 * pr * rc / (pr + rc) if pr + rc else 0.0,
            "accuracy": sum(t == p for t, p in zip(truth, pred)) / len(truth), "n": len(truth), "positives": sum(truth)}


def followup_measure(rows: list[dict], preds: dict) -> dict:
    res = {}
    for name, f in [("all", lambda r: True), ("written", lambda r: r["source"] == "written"), ("enron", lambda r: r["source"] == "enron")]:
        s = [r for r in rows if f(r)]
        res[name] = prf([r["label"] == "expects_reply" for r in s], [preds[r["id"]]["expects_reply"] for r in s])
    res["screened_out"] = sum(not p["screened"] for p in preds.values())
    res["missed_by_screen"] = sum(1 for r in rows if r["label"] == "expects_reply" and not preds[r["id"]]["screened"])
    res["ms_median"] = statistics.median(p["ms"] for p in preds.values())
    return res


# --- drafts ---------------------------------------------------------------------------------------

ANA_PAIRS = {  # Ana's sent reply -> the message it answers (both in bench/)
    "s002": "w002", "s004": "w009", "s005": "w011", "s006": "w013", "s008": "w006", "s010": "w015", "s012": "w005",
    "s014": "w062", "s016": "w065", "s017": "w071", "s018": "w072", "s020": "w075", "s021": "w066", "s022": "w061", "s024": "w074",
}

CONDITIONS = {
    "no style": ["--no-style", "--examples", "0"],
    "style description": ["--examples", "0"],
    "3 examples": ["--no-style", "--examples", "3"],
    "style + 3 examples": ["--examples", "3"],
}


def chrf(hyp: str, ref: str, n: int = 6, beta: float = 2.0) -> float:
    """chrF (Popović 2015): character n-gram F-score, n = 1..6, recall weighted twice."""
    def grams(s, k):
        s = re.sub(r"\s+", " ", s.strip())
        return Counter(s[i : i + k] for i in range(len(s) - k + 1))
    ps, rs = [], []
    for k in range(1, n + 1):
        h, r = grams(hyp, k), grams(ref, k)
        if not h or not r:
            continue
        overlap = sum((h & r).values())
        ps.append(overlap / sum(h.values()))
        rs.append(overlap / sum(r.values()))
    if not ps:
        return 0.0
    p, r = sum(ps) / len(ps), sum(rs) / len(rs)
    return 100 * (1 + beta**2) * p * r / (beta**2 * p + r) if p + r else 0.0


def first_word(t: str) -> str:
    for l in t.strip().splitlines():
        if l.strip():
            return re.sub(r"[^\w]", "", l.split()[0].lower())
    return ""


def last_line(t: str) -> str:
    ls = [l.strip() for l in t.strip().splitlines() if l.strip()]
    return re.sub(r"[^\w]", "", ls[-1].lower()) if ls else ""


def draft_sets() -> dict[str, tuple[Path, Path, str]]:
    sent = {r["id"]: r for r in jl(BENCH / "ana_sent.jsonl")}
    inbox = {r["id"]: r for r in jl(BENCH / "triage_written.jsonl")}
    ana_cases = [{"incoming": inbox[w], "reply": sent[s]} for s, w in ANA_PAIRS.items()]
    return {
        "ana": (write_jl(WORK / "drafts-ana.jsonl", ana_cases), BENCH / "ana_sent.jsonl", "Ana Popescu <ana.popescu@sieve.test>"),
        "enron": (WORK / "drafts-enron.jsonl", WORK / "sent-enron.jsonl", "Chris Germany <chris.germany@enron.com>"),
    }


def drafts(model: Path, url: str) -> dict:
    out = {}
    for setname, (cases, corpus, owner) in draft_sets().items():
        out[setname] = {}
        for cond, extra in CONDITIONS.items():
            p = WORK / f"drafts-{setname}-{model.stem}-{cond.replace(' ', '_').replace('+', 'and')}.jsonl"
            sieve(["draft-eval", "--input", str(cases), "--sent", str(corpus), "--output", str(p), "--owner", owner, *extra], url, model)
            out[setname][cond] = draft_measure(jl(p))
            print(f"  {setname} / {cond}: chrF {out[setname][cond]['chrf']:.1f}")
    return out


def draft_measure(rows: list[dict]) -> dict:
    ratios = [len(r["draft"].split()) / max(1, len(r["reply"].split())) for r in rows]
    return {
        "n": len(rows),
        "chrf": statistics.mean(chrf(r["draft"], r["reply"]) for r in rows),
        "length_ratio_median": statistics.median(ratios),
        "same_greeting_word": sum(first_word(r["draft"]) == first_word(r["reply"]) for r in rows) / len(rows),
        "same_last_line": sum(last_line(r["draft"]) == last_line(r["reply"]) for r in rows) / len(rows),
        "ms_median": statistics.median(r["ms"] for r in rows),
        "example": {"reply": rows[0]["reply"], "draft": rows[0]["draft"]},
    }


# --- summaries ------------------------------------------------------------------------------------

def norm(t: str) -> str:
    return re.sub(r"\s+", " ", t.lower().replace(" ", " ")).strip()


def numbers(t: str) -> set[str]:
    return {re.sub(r"[.,\s]", "", n) for n in re.findall(r"\d[\d.,\s]*\d|\d", t)}


def summaries(model: Path, url: str) -> dict:
    p = WORK / f"summaries-{model.stem}.jsonl"
    sieve(["summarize-eval", "--input", str(BENCH / "threads.jsonl"), "--output", str(p)], url, model)
    facts = json.loads((BENCH / "threads_facts.json").read_text(encoding="utf-8"))
    langs = {r["thread"]: r["lang"] for r in jl(BENCH / "threads.jsonl")}
    per = []
    for r in jl(p):
        s = r["summary"]
        text = " ".join([s.get("summary", ""), *s.get("decisions", []), *s.get("open_questions", []), s.get("waiting_on", "")])
        kept = [f for f in facts[r["thread"]]["keep"] if norm(f) in norm(text) or numbers(f) and numbers(f) <= numbers(text)]
        invented = sorted(numbers(text) - numbers(r["source"]))
        ro = len(re.findall(r"[ăâîșțşţ]", text.lower())) > 0 or bool(re.search(r"\b(și|este|pentru|care)\b", text.lower()))
        per.append({"thread": r["thread"], "facts": len(facts[r["thread"]]["keep"]), "kept": len(kept), "invented_numbers": invented,
                    "language_ok": (langs[r["thread"]] == "ro") == ro, "decisions": len(s.get("decisions", [])), "ms": r["ms"], "summary": s})
    return {
        "threads": len(per),
        "fact_recall": sum(x["kept"] for x in per) / sum(x["facts"] for x in per),
        "summaries_with_invented_numbers": sum(bool(x["invented_numbers"]) for x in per),
        "language_ok": sum(x["language_ok"] for x in per) / len(per),
        "ms_median": statistics.median(x["ms"] for x in per),
        "per_thread": per,
    }


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("what", choices=["followup", "drafts", "summaries", "all"])
    ap.add_argument("--model", type=Path, required=True)
    ap.add_argument("--url")
    a = ap.parse_args()
    RESULTS.mkdir(parents=True, exist_ok=True)

    def go(url: str) -> None:
        for what, fn in [("followup", followup), ("summaries", summaries), ("drafts", drafts)]:
            if a.what not in (what, "all"):
                continue
            res = fn(a.model, url)
            f = RESULTS / f"{what}.json"
            prev = json.loads(f.read_text(encoding="utf-8")) if f.exists() else {}
            prev[a.model.stem] = res
            f.write_text(json.dumps(prev, indent=1, ensure_ascii=False), encoding="utf-8")
            print(f"{what}: {json.dumps({k: v for k, v in res.items() if k not in ('per_thread',)}, ensure_ascii=False)[:600]}")

    if a.url:
        go(a.url)
    else:
        with llama(a.model) as u:
            go(u)


if __name__ == "__main__":
    main()
