"""Classification measures, with bootstrap confidence intervals."""

from __future__ import annotations

import numpy as np

CATEGORIES = ["reply_today", "informative", "newsletter", "invoice"]


def confusion(truth: list[str], pred: list[str], labels: list[str] = CATEGORIES) -> np.ndarray:
    idx = {c: i for i, c in enumerate(labels)}
    m = np.zeros((len(labels), len(labels)), dtype=int)
    for t, p in zip(truth, pred):
        m[idx[t], idx[p]] += 1
    return m


def per_class(truth: list[str], pred: list[str], labels: list[str] = CATEGORIES) -> dict:
    m = confusion(truth, pred, labels)
    out = {}
    for i, c in enumerate(labels):
        tp, fp, fn = m[i, i], m[:, i].sum() - m[i, i], m[i, :].sum() - m[i, i]
        p = tp / (tp + fp) if tp + fp else 0.0
        r = tp / (tp + fn) if tp + fn else 0.0
        f = 2 * p * r / (p + r) if p + r else 0.0
        out[c] = {"precision": float(p), "recall": float(r), "f1": float(f), "support": int(m[i, :].sum())}
    return out


def accuracy(truth, pred) -> float:
    return float(np.mean([t == p for t, p in zip(truth, pred)])) if truth else 0.0


def macro_f1(truth, pred, labels: list[str] = CATEGORIES) -> float:
    pc = per_class(truth, pred, labels)
    present = [c for c in labels if pc[c]["support"] > 0]
    return float(np.mean([pc[c]["f1"] for c in present])) if present else 0.0


def bootstrap(truth, pred, fn, n: int = 2000, seed: int = 0) -> tuple[float, float]:
    """95% interval of fn(truth, pred) over resamples of the messages."""
    rng = np.random.default_rng(seed)
    t, p = np.array(truth), np.array(pred)
    vals = []
    for _ in range(n):
        i = rng.integers(0, len(t), len(t))
        vals.append(fn(list(t[i]), list(p[i])))
    lo, hi = np.percentile(vals, [2.5, 97.5])
    return float(lo), float(hi)


def report(truth: list[str], pred: list[str]) -> dict:
    return {
        "n": len(truth),
        "accuracy": accuracy(truth, pred),
        "accuracy_ci": bootstrap(truth, pred, accuracy),
        "macro_f1": macro_f1(truth, pred),
        "macro_f1_ci": bootstrap(truth, pred, macro_f1),
        "per_class": per_class(truth, pred),
        "confusion": confusion(truth, pred).tolist(),
    }
