"""The triage evaluation: how often Sieve puts a message in the pile a person would.

  python -m sieve_eval.triage run --model PATH.gguf [--variant default|no_hints|category_first|all]
  python -m sieve_eval.triage baselines
  python -m sieve_eval.triage report            # tables from everything run so far
  python -m sieve_eval.triage mine              # on your own labels (exported from the app's Label mode)

Each run calls `sieve classify` — the daemon's own code — on the labelled set (the hand-written
Romanian/English messages plus the Enron sample), and writes predictions to eval/work and the
measures to eval/results/triage.json.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

import numpy as np

from . import metrics
from .paths import BENCH, RESULTS, WORK, sieve_cli
from .server import llama

VARIANTS = {
    "default": [],
    "no_hints": ["--no-hints"],
    "category_first": ["--no-reason-first"],
}


def split(r: dict) -> str:
    """dev (40%) for trying prompts, test (60%) for the numbers reported: fixed by each message's id."""
    import hashlib

    return "dev" if int(hashlib.sha256(r["id"].encode()).hexdigest(), 16) % 10 < 4 else "test"


def dataset() -> list[dict]:
    rows = [json.loads(l) for l in (BENCH / "triage_written.jsonl").read_text(encoding="utf-8").splitlines() if l.strip()]
    enron = WORK / "triage-enron.jsonl"
    if not enron.exists():
        raise SystemExit("build the Enron part first: python -m sieve_eval.enron build")
    rows += [json.loads(l) for l in enron.read_text(encoding="utf-8").splitlines() if l.strip()]
    return rows


def write_set(rows: list[dict]) -> Path:
    p = WORK / "triage-all.jsonl"
    p.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows), encoding="utf-8")
    return p


def run(model: Path, variant: str, url: str | None = None, system: Path | None = None, part: str = "all") -> Path:
    """Predictions of one variant: a built-in one, or a system prompt file (named after the file)."""
    rows = [r for r in dataset() if part == "all" or split(r) == part]
    src = write_set(rows)
    name = system.stem if system else variant
    out = WORK / f"pred-{model.stem}-{name}{'-' + part if part != 'all' else ''}.jsonl"
    cmd = [sieve_cli(), "classify", "--input", str(src), "--output", str(out), "--name", name, *VARIANTS[variant]]
    if system:
        cmd += ["--system", str(system)]
    env = {"SIEVE_MODEL_NAME": model.stem}
    import os

    if url:
        subprocess.run([*cmd, "--url", url], check=True, env={**os.environ, **env})
    else:
        with llama(model) as u:
            subprocess.run([*cmd, "--url", u], check=True, env={**os.environ, **env})
    return out


def predictions(path: Path, rows: list[dict]) -> list[dict]:
    meta = {r["id"]: r for r in rows}
    out = []
    for l in path.read_text(encoding="utf-8").splitlines():
        if l.strip():
            p = json.loads(l)
            r = meta[p["id"]]
            out.append({**p, "lang": r.get("lang", "en"), "source": r.get("source", "enron"), "label": r["label"], "split": split(r)})
    return out


def measure(preds: list[dict], part: str = "test") -> dict:
    """Measures on one split ("dev", "test", or "all")."""
    preds = [p for p in preds if part == "all" or p["split"] == part]

    def sub(f):
        s = [p for p in preds if f(p)]
        return metrics.report([p["label"] for p in s], [p["predicted"] for p in s]) if s else None

    ms = [p.get("ms", 0) for p in preds]
    return {
        "all": sub(lambda p: True),
        "written_ro": sub(lambda p: p["source"] == "written" and p["lang"] == "ro"),
        "written_en": sub(lambda p: p["source"] == "written" and p["lang"] == "en"),
        "enron": sub(lambda p: p["source"] == "enron"),
        "ms_median": float(np.median(ms)) if ms else 0.0,
        "ms_p90": float(np.percentile(ms, 90)) if ms else 0.0,
    }


# --- baselines ----------------------------------------------------------------------------------

MONEY = re.compile(r"(?i)\b(invoice|receipt|bill|statement|payment|paid|charged|factur\w*|chitan\w*|plat\w*|scaden\w*|extras\w*|întreținere|intretinere)\b")


def rules(r: dict) -> str:
    """What a few header and keyword rules would do."""
    text = f"{r['subject']}\n{r['body']}"
    if r.get("list_unsubscribe") or re.search(r"(?i)unsubscribe|dezabon", text):
        return "newsletter"
    if MONEY.search(r["subject"]) or any(re.search(r"(?i)factur|invoice|receipt", a) for a in r.get("attachments", [])):
        return "invoice"
    if "?" in r["body"][:1500]:
        return "reply_today"
    return "informative"


def tfidf_cv(rows: list[dict], folds: int = 5, seed: int = 0) -> list[str]:
    """A TF-IDF + logistic regression classifier, each message predicted by a model trained on the
    other folds (there is no separate training set)."""
    from sklearn.feature_extraction.text import TfidfVectorizer
    from sklearn.linear_model import LogisticRegression
    from sklearn.model_selection import StratifiedKFold
    from sklearn.pipeline import make_pipeline

    texts = [f"{r['from']} {r['subject']} {'UNSUBSCRIBE_HEADER' if r.get('list_unsubscribe') else ''} {r['body'][:2000]}" for r in rows]
    y = np.array([r["label"] for r in rows])
    pred = np.empty(len(rows), dtype=object)
    for tr, te in StratifiedKFold(folds, shuffle=True, random_state=seed).split(texts, y):
        clf = make_pipeline(TfidfVectorizer(ngram_range=(1, 2), min_df=1, sublinear_tf=True, strip_accents="unicode"),
                            LogisticRegression(max_iter=2000, C=4.0, class_weight="balanced"))
        clf.fit([texts[i] for i in tr], y[tr])
        pred[te] = clf.predict([texts[i] for i in te])
    return list(pred)


def baselines() -> dict:
    rows = dataset()
    labels = [r["label"] for r in rows]
    out = {}
    for name, preds in [
        ("majority", ["informative"] * len(rows)),
        ("rules", [rules(r) for r in rows]),
        ("tfidf_logreg_5fold", tfidf_cv(rows)),
    ]:
        ps = [{"id": r["id"], "predicted": p, "ms": 0, "label": r["label"], "lang": r.get("lang", "en"), "source": r.get("source", "enron"),
               "split": split(r)} for r, p in zip(rows, preds)]
        out[name] = {part: measure(ps, part) for part in ("dev", "test")}
    return out


# --- reporting ----------------------------------------------------------------------------------

def collect() -> dict:
    rows = dataset()
    res = {"dataset": {"n": len(rows), "by_label": {c: sum(r["label"] == c for r in rows) for c in metrics.CATEGORIES},
                       "written": sum(r.get("source") == "written" for r in rows), "enron": sum(r.get("source") == "enron" for r in rows),
                       "dev": sum(split(r) == "dev" for r in rows), "test": sum(split(r) == "test" for r in rows)}}
    res["baselines"] = baselines()
    res["runs"] = {}
    for p in sorted(WORK.glob("pred-*.jsonl")):
        preds = predictions(p, rows)
        if p.stem.endswith("-dev"):  # prompt trials: dev only
            res["runs"][p.stem[len("pred-"):]] = {"dev": measure(preds, "dev")}
        else:
            res["runs"][p.stem[len("pred-"):]] = {part: measure(preds, part) for part in ("dev", "test")}
    return res


def fmt(r: dict | None, key: str = "accuracy") -> str:
    if not r:
        return "–"
    lo, hi = r[f"{key}_ci"]
    return f"{r[key]:.2f} ({lo:.2f}–{hi:.2f})"


def table(res: dict, part: str = "test") -> str:
    lines = ["| System | Accuracy (95% CI) | Macro-F1 (95% CI) | Romanian | English (written) | Enron | reply_today recall | Median time |",
             "|---|---|---|---|---|---|---|---|"]
    def row(name, m):
        rt = m["all"]["per_class"]["reply_today"]["recall"]
        lines.append(f"| {name} | {fmt(m['all'])} | {fmt(m['all'], 'macro_f1')} | {m['written_ro']['accuracy']:.2f} | "
                     f"{m['written_en']['accuracy']:.2f} | {m['enron']['accuracy']:.2f} | {rt:.2f} | "
                     f"{m['ms_median'] / 1000:.1f} s |" if m["ms_median"] else
                     f"| {name} | {fmt(m['all'])} | {fmt(m['all'], 'macro_f1')} | {m['written_ro']['accuracy']:.2f} | "
                     f"{m['written_en']['accuracy']:.2f} | {m['enron']['accuracy']:.2f} | {rt:.2f} | – |")
    for k, m in res["baselines"].items():
        row(k, m[part])
    for k, m in res["runs"].items():
        if part in m:
            row(k, m[part])
    return "\n".join(lines)


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("what", choices=["run", "baselines", "report", "mine"])
    ap.add_argument("--model", type=Path)
    ap.add_argument("--variant", default="default", choices=[*VARIANTS, "all"])
    ap.add_argument("--url", help="a llama.cpp server already running with --model")
    ap.add_argument("--system", type=Path, help="run: a system prompt file to try instead of the built-in one")
    ap.add_argument("--part", default="all", choices=["all", "dev", "test"], help="run: only this split")
    ap.add_argument("--labels", type=Path, help="mine: the exported labels (default: export from the app's database)")
    a = ap.parse_args()
    WORK.mkdir(parents=True, exist_ok=True)
    RESULTS.mkdir(parents=True, exist_ok=True)
    if a.what == "run":
        if not a.model:
            raise SystemExit("--model PATH.gguf")
        for v in (VARIANTS if a.variant == "all" else [a.variant]):
            out = run(a.model, v, a.url, a.system, a.part)
            for part in (("dev", "test") if a.part == "all" else (a.part,)):
                m = measure(predictions(out, dataset()), part)
                print(f"{a.model.stem} {v} [{part}]: accuracy {fmt(m['all'])}, macro-F1 {fmt(m['all'], 'macro_f1')}", flush=True)
    elif a.what == "baselines":
        for k, m in baselines().items():
            for part in ("dev", "test"):
                print(f"{k} [{part}]: accuracy {fmt(m[part]['all'])}, macro-F1 {fmt(m[part]['all'], 'macro_f1')}")
    elif a.what == "report":
        res = collect()
        (RESULTS / "triage.json").write_text(json.dumps(res, indent=1), encoding="utf-8")
        print("dev:\n" + table(res, "dev") + "\n\ntest:\n" + table(res, "test"))
    else:
        mine(a)


def mine(a) -> None:
    """The triage on your own labelled mail (from the app's Label mode)."""
    labels = a.labels or WORK / "my-labels.jsonl"
    if not a.labels:
        subprocess.run([sieve_cli(), "export-labels", "--output", str(labels)], check=True)
    if not a.model:
        raise SystemExit("--model PATH.gguf (the model the daemon uses)")
    rows = [json.loads(l) for l in labels.read_text(encoding="utf-8").splitlines() if l.strip()]
    out = WORK / f"mine-{a.model.stem}.jsonl"
    with llama(a.model) as u:
        subprocess.run([sieve_cli(), "classify", "--input", str(labels), "--output", str(out), "--url", u], check=True)
    preds = [json.loads(l) for l in out.read_text(encoding="utf-8").splitlines() if l.strip()]
    r = metrics.report([p["label"] for p in preds], [p["predicted"] for p in preds])
    print(f"your {len(rows)} labelled messages: accuracy {fmt(r)}, macro-F1 {fmt(r, 'macro_f1')}")
    for c, v in r["per_class"].items():
        print(f"  {c:<12} precision {v['precision']:.2f} recall {v['recall']:.2f} ({v['support']} messages)")
    # Your mail stays yours: the result goes to eval/work (not in git), not to eval/results.
    (WORK / "mine.json").write_text(json.dumps(r, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
