"""Where things are."""

from __future__ import annotations

import os
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EVAL = ROOT / "eval"
BENCH = ROOT / "bench"            # labels and hand-written messages (in git)
WORK = EVAL / "work"              # built sets and predictions (not in git)
RESULTS = EVAL / "results"        # summaries of each run (in git)
DOWNLOADS = ROOT / "third_party" / "downloads"

# The Enron corpus as published on Hugging Face (corbt/enron-emails), first of three shards.
ENRON_REVISION = "cfc06c758093d90993abce1a43668fb7357258a6"
ENRON_SHARD = DOWNLOADS / "enron-train-00000-of-00003.parquet"
ENRON_SHA256 = "97165e587875e3e57715c14f63547fedbf664f748de1124122d4d2952c56a9fc"


def sieve_cli() -> str:
    """The `sieve` command (release build preferred)."""
    exe = "sieve.exe" if os.name == "nt" else "sieve"
    for build in ["release", "debug"]:
        p = ROOT / "target" / build / exe
        if p.exists():
            return str(p)
    found = shutil.which("sieve")
    if found:
        return found
    raise SystemExit("build the Rust workspace first: cargo build --release")
