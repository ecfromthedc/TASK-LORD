"""Context handoff generator.

Instead of resuming a giant old session (bloated, expensive context window), we
distill it into a tight handoff document and start a FRESH session seeded with
it. The distillation is the token-grind, so it runs on the local LLM (Ollama),
with map-reduce for sessions too large for one context window.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

import ollama_client

PROJECTS_DIR = Path.home() / ".claude" / "projects"
HANDOFF_DIR = Path(__file__).resolve().parent / "handoffs"

# Single-pass if the distilled transcript fits; else map-reduce in chunks.
SINGLE_PASS_CHARS = 11000
CHUNK_CHARS = 8000
MAX_CHUNKS = 8
EDIT_TOOLS = {"Edit", "Write", "NotebookEdit", "MultiEdit"}

HANDOFF_FIELDS = {
    "type": "object",
    "properties": {
        "tldr": {"type": "string"},
        "goal": {"type": "string"},
        "done": {"type": "array", "items": {"type": "string"}},
        "current_state": {"type": "string"},
        "next_steps": {"type": "array", "items": {"type": "string"}},
        "open_questions": {"type": "array", "items": {"type": "string"}},
        "blockers": {"type": "string"},
        "gotchas": {"type": "array", "items": {"type": "string"}},
    },
    "required": ["tldr", "current_state", "next_steps"],
}

HANDOFF_PROMPT = """You are writing a CONTEXT HANDOFF so a brand-new agent can pick up this
project in a fresh session with zero prior memory. Be concrete and specific —
name files, commands, decisions, and exact next actions. No fluff.

Output STRICT JSON with these keys:
- tldr: one sentence — what this work is and its current status.
- goal: what the work is trying to achieve.
- done: array of concrete things already accomplished.
- current_state: 2-4 sentences on exactly where things stand right now.
- next_steps: array of the next actions, most important first, imperative voice.
- open_questions: array of unresolved decisions/questions (or empty).
- blockers: what's blocking progress, or "" if nothing.
- gotchas: array of non-obvious things the next agent must know (or empty).

PROJECT: {label}
WORKING DIR: {path}
SESSION CONTENT (may be summarized notes from a long session):
{content}
"""

CHUNK_PROMPT = """Extract the durable facts from this slice of a work session — decisions made,
what was built/changed, current state, next intentions, blockers, and any
file paths or commands mentioned. Terse bullet notes only, no preamble.

SLICE:
{slice}
"""


def _text_from_content(content) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        out = []
        for b in content:
            if isinstance(b, dict) and b.get("type") == "text":
                out.append(b.get("text", ""))
        return "\n".join(p for p in out if p)
    return ""


def _read_session(jsonl_path: Path) -> tuple[str, list[str]]:
    """Return (conversation_text, files_touched)."""
    msgs: list[str] = []
    files: list[str] = []
    try:
        with open(jsonl_path, errors="ignore") as fh:
            for line in fh:
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if rec.get("type") not in ("user", "assistant"):
                    continue
                m = rec.get("message", {})
                if not isinstance(m, dict):
                    continue
                content = m.get("content", "")
                # capture files touched from tool_use blocks
                if isinstance(content, list):
                    for b in content:
                        if isinstance(b, dict) and b.get("type") == "tool_use" \
                                and b.get("name") in EDIT_TOOLS:
                            fp = (b.get("input") or {}).get("file_path")
                            if fp and fp not in files:
                                files.append(fp)
                text = _text_from_content(content).strip()
                if text:
                    msgs.append(f"{m.get('role', rec['type']).upper()}: {text}")
    except OSError:
        return "", []
    return "\n\n".join(msgs), files


def _distill(text: str, label: str) -> str:
    """Reduce a long transcript to notes that fit one context window."""
    if len(text) <= SINGLE_PASS_CHARS:
        return text
    # map-reduce: chunk, summarize each, concatenate notes
    chunks = [text[i:i + CHUNK_CHARS] for i in range(0, len(text), CHUNK_CHARS)]
    # keep the earliest and latest chunks (goal + current state matter most)
    if len(chunks) > MAX_CHUNKS:
        head = chunks[: MAX_CHUNKS // 2]
        tail = chunks[-(MAX_CHUNKS - len(head)):]
        chunks = head + tail
    notes = []
    for i, ch in enumerate(chunks):
        n = ollama_client.generate_text(CHUNK_PROMPT.format(slice=ch), timeout=120)
        if n:
            notes.append(f"[part {i + 1}]\n{n}")
    return "\n\n".join(notes) if notes else text[-SINGLE_PASS_CHARS:]


def _render_md(fields: dict, label: str, path: str, files: list[str], session_id: str) -> str:
    def bullets(items):
        return "\n".join(f"- {x}" for x in items if x) or "- (none)"

    md = [
        f"# Context Handoff — {label}",
        "",
        f"> Generated by TASK LORD from session `{session_id}`. Fresh session, "
        f"same working dir. Read this, then continue.",
        "",
        f"**TL;DR:** {fields.get('tldr', '—')}",
        "",
        f"**Working dir:** `{path}`",
        "",
        "## Goal",
        fields.get("goal", "—"),
        "",
        "## Done so far",
        bullets(fields.get("done", [])),
        "",
        "## Current state",
        fields.get("current_state", "—"),
        "",
        "## Next steps",
        bullets(fields.get("next_steps", [])),
        "",
        "## Open questions",
        bullets(fields.get("open_questions", [])),
        "",
        f"## Blockers\n{fields.get('blockers') or '(none)'}",
        "",
        "## Gotchas",
        bullets(fields.get("gotchas", [])),
        "",
        "## Files touched this session",
        bullets(files[:40]),
        "",
    ]
    return "\n".join(md)


def _safe(name: str) -> str:
    return re.sub(r"[^a-zA-Z0-9_-]+", "-", name).strip("-")[:48] or "project"


def build_handoff(group: str, session_id: str, card: dict) -> Path | None:
    """Generate (and cache) a handoff doc for a session card. Returns its path."""
    transcript = PROJECTS_DIR / group / f"{session_id}.jsonl"
    if not transcript.exists():
        return None
    HANDOFF_DIR.mkdir(exist_ok=True)
    out = HANDOFF_DIR / f"{_safe(card.get('label', group))}-{session_id[:8]}.md"

    # cache: reuse if newer than the transcript
    if out.exists() and out.stat().st_mtime >= transcript.stat().st_mtime:
        return out

    text, files = _read_session(transcript)
    if not text:
        return None
    content = _distill(text, card.get("label", group))
    fields = ollama_client.generate_json(
        HANDOFF_PROMPT.format(
            label=card.get("label", group),
            path=card.get("path", "?"),
            content=content[:SINGLE_PASS_CHARS],
        ),
        timeout=180,
    )
    if not fields:
        # fall back to whatever the harvest already knew
        fields = {
            "tldr": card.get("left_off", "—"),
            "current_state": card.get("left_off", "—"),
            "next_steps": [card.get("next_step")] if card.get("next_step") else [],
            "blockers": card.get("blockers", ""),
        }
    md = _render_md(fields, card.get("label", group), card.get("path", "?"), files, session_id)
    out.write_text(md)
    return out
