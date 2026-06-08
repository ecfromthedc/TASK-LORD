"""Source: Trello (business/content workstreams) via the local `trello` CLI.

The CLI resolves boards/lists by NAME from a synced local cache, and Trello
list names (TODO / DOING / BLOCKED / DONE) map directly onto kanban columns.
Secrets stay inside the CLI's own config — we never read or print them.
Failures are non-fatal: if Trello is unreachable the board renders from the
other sources.
"""
from __future__ import annotations

import json
import subprocess

# Trello list-name -> kanban status. Unmatched lists fall back to backlog.
LIST_STATUS_HINTS = {
    "done": "done",
    "complete": "done",
    "shipped": "done",
    "doing": "in_progress",
    "in progress": "in_progress",
    "go mode": "in_progress",
    "wip": "in_progress",
    "active": "in_progress",
    "blocked": "blocked",
    "waiting": "blocked",
    "on hold": "blocked",
    "stuck": "blocked",
    "todo": "backlog",
    "to do": "backlog",
    "backlog": "backlog",
    "brainstorm": "backlog",
    "inbox": "backlog",
    "ideas": "backlog",
}

# Bounds so the nightly run stays quick over many boards.
MAX_BOARDS = 20
MAX_CARDS_PER_LIST = 60  # for accurate per-board rollup counts


def _run(*args: str, timeout: int = 25) -> str | None:
    try:
        res = subprocess.run(
            ["trello", *args], capture_output=True, text=True, timeout=timeout
        )
        if res.returncode != 0:
            return None
        return res.stdout
    except (subprocess.SubprocessError, OSError):
        return None


def _json(text: str | None):
    if not text:
        return None
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return None


def _status_for_list(list_name: str) -> str:
    low = (list_name or "").lower()
    for key, status in LIST_STATUS_HINTS.items():
        if key in low:
            return status
    return "backlog"


def collect() -> list[dict]:
    """Walk boards -> lists -> cards and emit ONE rolled-up record per board.

    A Trello board is a business workstream/project; its cards are sub-tasks.
    Surfacing every card floods the command board, so we roll each board into a
    single workstream card: open-card count, status mix, and the most recently
    touched active card as the 'where it left off' signal.
    """
    # Refresh the name->id cache so board lookups resolve.
    _run("sync", timeout=40)

    boards = _json(_run("board:list", "--format", "json"))
    if not isinstance(boards, list):
        return []

    out: list[dict] = []
    for board in boards[:MAX_BOARDS]:
        bname = board.get("name") if isinstance(board, dict) else None
        if not bname:
            continue
        lists = _json(_run("list:list", "--board", bname, "--format", "json"))
        if not isinstance(lists, list):
            continue

        status_counts: dict[str, int] = {}
        open_cards = 0
        recent_card = None  # (last_activity, name, list, status)
        recent_active = None  # most recent card in a non-done/non-backlog list
        last_activity = None

        for lst in lists:
            lname = lst.get("name") if isinstance(lst, dict) else None
            if not lname:
                continue
            lstatus = _status_for_list(lname)
            cards = _json(
                _run("card:list", "--board", bname, "--list", lname, "--format", "json")
            )
            if not isinstance(cards, list):
                continue
            for c in cards[:MAX_CARDS_PER_LIST]:
                if not isinstance(c, dict) or c.get("closed"):
                    continue
                name = c.get("name")
                if not name:
                    continue
                if lstatus != "done":
                    open_cards += 1
                status_counts[lstatus] = status_counts.get(lstatus, 0) + 1
                act = c.get("dateLastActivity")
                if act and (last_activity is None or act > last_activity):
                    last_activity = act
                if act and (recent_card is None or act > recent_card[0]):
                    recent_card = (act, name, lname, lstatus)
                if lstatus == "in_progress" and act and (
                    recent_active is None or act > recent_active[0]
                ):
                    recent_active = (act, name, lname)

        if not status_counts:
            continue

        # Board-level status: blocked if anything's blocked, else in_progress if
        # active work exists, else backlog. Done only if literally nothing open.
        if open_cards == 0:
            bstatus = "done"
        elif status_counts.get("blocked"):
            bstatus = "blocked"
        elif status_counts.get("in_progress"):
            bstatus = "in_progress"
        else:
            bstatus = "backlog"

        mix = ", ".join(f"{n} {s.replace('_', ' ')}" for s, n in sorted(status_counts.items()))
        left = f"{open_cards} open cards ({mix})."
        if recent_card:
            left += f" Most recent: “{recent_card[1][:80]}” in {recent_card[2]}."
        nxt = f"Advance “{recent_active[1][:80]}”" if recent_active else "—"

        out.append(
            {
                "source": "business",
                "label": bname,
                "board": bname,
                "trello_list": f"{open_cards} open",
                "path": None,
                "status": bstatus,
                "last_activity": last_activity,
                "left_off": left,
                "next_step": nxt,
                "due": None,
            }
        )
    return out
