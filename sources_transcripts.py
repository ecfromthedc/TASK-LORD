"""Source: Claude Code session transcripts (~/.claude/projects/*/*.jsonl).

Every agent/session Eric has run is a transcript. This is the loose-ends
goldmine: it reconstructs *where each project left off* from the actual
conversation tail. Token-grind (summarizing tails) is pushed to Ollama.
"""
from __future__ import annotations

import json
import os
from pathlib import Path  # noqa: F401  (used in collect guard)

PROJECTS_DIR = Path.home() / ".claude" / "projects"

# How many recent text messages from the tail to feed the summarizer.
TAIL_MESSAGES = 18
# Hard cap on characters sent to Ollama per project (keeps it fast).
MAX_CHARS = 7000


def _text_from_content(content) -> str:
    """Flatten a message 'content' (str or list of blocks) to plain text."""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts = []
        for block in content:
            if not isinstance(block, dict):
                continue
            btype = block.get("type")
            if btype == "text":
                parts.append(block.get("text", ""))
            elif btype == "tool_result":
                # keep a short marker so we know a tool ran, not the payload
                parts.append("[tool result]")
            elif btype == "tool_use":
                parts.append(f"[tool: {block.get('name', '?')}]")
        return "\n".join(p for p in parts if p)
    return ""


def _newest_transcript(group_dir: Path) -> Path | None:
    files = sorted(
        group_dir.glob("*.jsonl"), key=lambda p: p.stat().st_mtime, reverse=True
    )
    return files[0] if files else None


def _parse_transcript(path: Path) -> dict:
    """Pull cwd, last activity, and the conversation tail (text only)."""
    cwd = None
    last_ts = None
    messages = []  # (role, text)
    try:
        with open(path, "r", errors="ignore") as fh:
            for line in fh:
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if cwd is None and rec.get("cwd"):
                    cwd = rec["cwd"]
                ts = rec.get("timestamp")
                if ts and (last_ts is None or ts > last_ts):
                    last_ts = ts
                if rec.get("type") in ("user", "assistant"):
                    msg = rec.get("message", {})
                    if isinstance(msg, dict):
                        text = _text_from_content(msg.get("content", ""))
                        text = text.strip()
                        # skip empty / pure-tool noise
                        if text and text not in ("[tool result]",):
                            messages.append((msg.get("role", rec["type"]), text))
    except OSError:
        return {}
    tail = messages[-TAIL_MESSAGES:]
    convo = "\n\n".join(f"{role.upper()}: {text}" for role, text in tail)
    if len(convo) > MAX_CHARS:
        convo = convo[-MAX_CHARS:]
    return {"cwd": cwd, "last_activity": last_ts, "tail": convo, "msg_count": len(messages)}


def collect() -> list[dict]:
    """Return one raw record per transcript group (project/agent)."""
    if not PROJECTS_DIR.exists():
        return []
    out = []
    for group_dir in PROJECTS_DIR.iterdir():
        if not group_dir.is_dir():
            continue
        newest = _newest_transcript(group_dir)
        if newest is None:
            continue
        parsed = _parse_transcript(newest)
        if not parsed or not parsed.get("tail"):
            continue
        # Human-readable label: prefer real cwd basename, else decode dir name.
        cwd = parsed.get("cwd")
        if cwd:
            label = os.path.basename(cwd.rstrip("/")) or cwd
        else:
            label = group_dir.name.lstrip("-").split("-")[-1]
        # Skip rootless / home-level noise that isn't a real project.
        if cwd in ("/", str(Path.home())) or not label.strip() or label == "/":
            continue
        out.append(
            {
                "source": "session",
                "label": label,
                "path": cwd,
                "group": group_dir.name,
                "session_id": newest.stem,  # jsonl filename = session UUID for --resume
                "last_activity": parsed.get("last_activity"),
                "msg_count": parsed.get("msg_count", 0),
                "tail": parsed["tail"],
            }
        )
    return out
