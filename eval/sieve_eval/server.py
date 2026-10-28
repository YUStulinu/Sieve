"""A llama.cpp server for a run of the evaluation: started with a given model, stopped after."""

from __future__ import annotations

import os
import subprocess
import time
import urllib.request
from contextlib import contextmanager
from pathlib import Path

from .paths import ROOT


def server_binary() -> str:
    exe = "llama-server.exe" if os.name == "nt" else "llama-server"
    for d in [ROOT / "third_party" / "llama", ROOT / "third_party" / "llama" / "bin"]:
        if (d / exe).exists():
            return str(d / exe)
    return exe


def healthy(url: str) -> bool:
    try:
        with urllib.request.urlopen(f"{url}/health", timeout=3) as r:
            return r.status == 200
    except Exception:
        return False


@contextmanager
def llama(model: Path, port: int = 8790, context: int = 8192, gpu: bool = True):
    """Yields the server's URL. An already running server on the port is used as it is."""
    url = f"http://127.0.0.1:{port}"
    if healthy(url):
        yield url
        return
    args = [server_binary(), "-m", str(model), "--host", "127.0.0.1", "--port", str(port), "-c", str(context), "--no-ui", "-np", "1"]
    if gpu:
        args += ["-ngl", "99", "-dev", "CUDA0"] if os.environ.get("SIEVE_CPU_ONLY") is None else []
    log = open(ROOT / "eval" / "work" / f"llama-{model.stem}.log", "w")
    p = subprocess.Popen(args, stdout=log, stderr=subprocess.STDOUT)
    try:
        t0 = time.time()
        while not healthy(url):
            if p.poll() is not None:
                raise RuntimeError(f"llama-server exited; see {log.name}")
            if time.time() - t0 > 300:
                raise RuntimeError("llama-server did not start in 5 minutes")
            time.sleep(1)
        yield url
    finally:
        p.terminate()
        try:
            p.wait(timeout=20)
        except subprocess.TimeoutExpired:
            p.kill()
        log.close()
