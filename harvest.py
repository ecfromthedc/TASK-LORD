#!/usr/bin/env python3
"""TASK LORD harvester — clock every project's loose ends into one Kanban board.

Pipeline:
  1. Collect raw records from three sources (sessions, code, Trello).
  2. Merge git 'cold facts' onto session records sharing a cwd.
  3. For each session/code record, ask local Ollama to classify status and
     reconstruct where it left off (the token-grind — never burns Claude).
  4. Emit board.json the single-file HTML board reads.

Run:  python3 harvest.py            (full run)
      python3 harvest.py --no-llm   (heuristics only, fast, no Ollama)
"""
from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import ollama_client
import sources_code
import sources_trello
import sources_transcripts
import store

HERE = Path(__file__).resolve().parent
BOARD_JSON = HERE / "board" / "board.json"
VALID_STATUS = {"backlog", "in_progress", "blocked", "done"}

SUMMARY_PROMPT = """You are a project triage assistant for a founder's software factory.
Given the tail of a work session and git facts for ONE project, output STRICT JSON:
{{
  "status": one of "backlog" | "in_progress" | "blocked" | "done",
  "left_off": "1-2 sentences: concretely where this was left off / last state",
  "next_step": "the single most useful next action, imperative voice",
  "blockers": "what's blocking it, or empty string if nothing"
}}

Rules:
- "done" only if the work clearly reached a finished, shipped, or verified state.
- "blocked" if it's waiting on a person, decision, credential, or external thing.
- "in_progress" if mid-build with a clear unfinished thread.
- "backlog" if barely started or just scoped.
- Be specific and concrete. No preamble. JSON only.

PROJECT: {label}
GIT FACTS: {facts}
SESSION TAIL:
{tail}
"""


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def _days_since(iso: str | None) -> int | None:
    if not iso:
        return None
    try:
        dt = datetime.fromisoformat(iso.replace("Z", "+00:00"))
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        return (datetime.now(timezone.utc) - dt).days
    except (ValueError, TypeError):
        return None


def _facts_str(rec: dict) -> str:
    bits = []
    if rec.get("is_git"):
        bits.append(f"git branch={rec.get('branch')}")
        if rec.get("last_commit_msg"):
            bits.append(f"last commit: {rec['last_commit_msg']!r}")
        if rec.get("dirty"):
            bits.append(f"{rec.get('dirty_count', 0)} uncommitted changes")
    if rec.get("plan_files"):
        bits.append("has " + ", ".join(rec["plan_files"]))
    age = _days_since(rec.get("last_activity"))
    if age is not None:
        bits.append(f"last active {age}d ago")
    return "; ".join(bits) or "none"


def _heuristic(rec: dict) -> dict:
    """Status guess without an LLM — fallback when Ollama is down."""
    age = _days_since(rec.get("last_activity"))
    tail = rec.get("tail", "")
    low = tail.lower()
    if any(w in low for w in ("blocked", "waiting on", "need eric", "can't proceed")):
        status = "blocked"
    elif any(w in low for w in ("shipped", "done.", "complete", "deployed", "verified")):
        status = "done"
    elif age is not None and age > 45:
        status = "backlog"
    else:
        status = "in_progress"
    last_user = ""
    for chunk in reversed(tail.split("\n\n")):
        if chunk.startswith("USER:"):
            last_user = chunk[5:].strip()[:200]
            break
    return {
        "status": status,
        "left_off": last_user or "No recent summary available.",
        "next_step": "Review session and resume.",
        "blockers": "" if status != "blocked" else "See session tail.",
    }


def _summarize(rec: dict, use_llm: bool) -> dict:
    if not use_llm:
        return _heuristic(rec)
    prompt = SUMMARY_PROMPT.format(
        label=rec.get("label", "?"),
        facts=_facts_str(rec),
        tail=rec.get("tail", "(no session tail)"),
    )
    result = ollama_client.generate_json(prompt)
    if not result or result.get("status") not in VALID_STATUS:
        return _heuristic(rec)
    return {
        "status": result.get("status", "in_progress"),
        "left_off": (result.get("left_off") or "").strip()[:400] or "—",
        "next_step": (result.get("next_step") or "").strip()[:300] or "—",
        "blockers": (result.get("blockers") or "").strip()[:300],
    }


def build(use_llm: bool = True) -> dict:
    sessions = sources_transcripts.collect()
    code = sources_code.collect()
    print(f"[harvest] {len(sessions)} session groups, {len(code)} code projects", file=sys.stderr)

    # Merge git facts onto sessions by path; track which code paths are covered.
    covered = set()
    for rec in sessions:
        path = rec.get("path")
        if path and path in code:
            rec.update({k: v for k, v in code[path].items() if k != "label"})
            covered.add(path)

    # Code projects with NO recent session → cold cards (still loose ends).
    for path, facts in code.items():
        if path in covered:
            continue
        sessions.append(
            {
                "source": "code",
                "label": facts["label"],
                "path": path,
                "last_activity": facts.get("last_commit"),
                "tail": "",
                **{k: v for k, v in facts.items() if k != "label"},
            }
        )

    cards = []
    total = len(sessions)
    for i, rec in enumerate(sessions, 1):
        summ = _summarize(rec, use_llm=use_llm and bool(rec.get("tail")))
        if not rec.get("tail") and not use_llm:
            summ = _heuristic(rec)
        age = _days_since(rec.get("last_activity"))
        cards.append(
            {
                "id": rec.get("group") or rec.get("path") or rec.get("label"),
                "label": rec.get("label", "?"),
                "source": rec.get("source", "session"),
                "status": summ["status"],
                "left_off": summ["left_off"],
                "next_step": summ["next_step"],
                "blockers": summ["blockers"],
                "path": rec.get("path"),
                "session_id": rec.get("session_id"),
                "last_activity": rec.get("last_activity"),
                "days_idle": age,
                "stale": age is not None and age > 30,
                "git": {
                    "branch": rec.get("branch"),
                    "dirty": rec.get("dirty"),
                    "dirty_count": rec.get("dirty_count"),
                    "last_commit_msg": rec.get("last_commit_msg"),
                }
                if rec.get("is_git")
                else None,
                "msg_count": rec.get("msg_count"),
            }
        )
        if i % 10 == 0 or i == total:
            print(f"[harvest] summarized {i}/{total}", file=sys.stderr)

    # Trello business cards (already classified, no LLM needed).
    for t in sources_trello.collect():
        age = _days_since(t.get("last_activity"))
        cards.append(
            {
                "id": "trello:" + t["label"],
                "label": t["label"],
                "source": "business",
                "status": t.get("status", "backlog"),
                "left_off": t.get("left_off") or "—",
                "next_step": t.get("next_step") or "—",
                "blockers": "",
                "path": t.get("path"),
                "last_activity": t.get("last_activity"),
                "days_idle": age,
                "stale": age is not None and age > 30,
                "git": None,
                "trello_list": t.get("trello_list"),
                "board": t.get("board"),
                "due": t.get("due"),
            }
        )

    # Sort each card by recency (most recent first); None last.
    cards.sort(key=lambda c: c.get("last_activity") or "", reverse=True)

    counts = {s: sum(1 for c in cards if c["status"] == s) for s in VALID_STATUS}
    return {
        "generated_at": _now_iso(),
        "total": len(cards),
        "counts": counts,
        "cards": cards,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-llm", action="store_true", help="heuristics only, skip Ollama")
    args = ap.parse_args()

    use_llm = not args.no_llm
    if use_llm and not ollama_client.is_up():
        print("[harvest] Ollama not reachable — falling back to heuristics", file=sys.stderr)
        use_llm = False

    board = build(use_llm=use_llm)
    # SQLite is the source of truth (+ keeps status history); board.json/.js are
    # exported FROM the db for the static/file:// UI.
    summary = store.upsert_cards(board["cards"])
    print(f"[harvest] db upsert: {summary}", file=sys.stderr)
    _write_board(store.export_board())
    print(f"[harvest] wrote {store.DB_PATH.name} + {BOARD_JSON.name} — "
          f"{board['total']} cards {board['counts']}", file=sys.stderr)
    return 0


def _write_board(board: dict) -> None:
    """Write both board.json (served) and board.js (file:// double-click)."""
    BOARD_JSON.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(board, indent=2)
    BOARD_JSON.write_text(payload)
    # board.js lets index.html load via file:// without CORS-blocked fetch.
    (BOARD_JSON.parent / "board.js").write_text("window.BOARD = " + payload + ";\n")


if __name__ == "__main__":
    raise SystemExit(main())
