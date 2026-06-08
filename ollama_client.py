"""Thin Ollama client — local LLM does the token-grind, Claude stays out of it.

Uses the local Ollama HTTP API (localhost:11434). No external deps (urllib only)
so it runs under stock python3 with no pip install.
"""
from __future__ import annotations

import json
import urllib.request
import urllib.error

OLLAMA_URL = "http://localhost:11434/api/generate"
DEFAULT_MODEL = "llama3.1:8b"


def generate_json(prompt: str, model: str = DEFAULT_MODEL, timeout: int = 120) -> dict:
    """Call Ollama in JSON mode and return the parsed object.

    Returns {} on any failure (network, timeout, bad JSON) so callers can fall
    back to heuristics rather than crash the whole harvest.
    """
    payload = {
        "model": model,
        "prompt": prompt,
        "stream": False,
        "format": "json",
        "options": {"temperature": 0.1, "num_ctx": 8192},
    }
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        OLLAMA_URL, data=data, headers={"Content-Type": "application/json"}
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = json.loads(resp.read().decode("utf-8"))
        raw = body.get("response", "").strip()
        if not raw:
            return {}
        return json.loads(raw)
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError, ValueError):
        return {}


def is_up() -> bool:
    try:
        req = urllib.request.Request("http://localhost:11434/api/tags")
        with urllib.request.urlopen(req, timeout=5):
            return True
    except Exception:
        return False
