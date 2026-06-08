#!/usr/bin/env python3
"""TASK LORD launchpad server — click a card, land back in the work.

Serves the board from 127.0.0.1 and exposes /cook, which opens Terminal.app in
the card's project and launches Claude Code:
  - session card -> `claude --resume <session_id>`  (exact conversation)
  - code card    -> `claude "<seeded next-step prompt>"`  (fresh, with context)
  - business     -> opens the Trello board in the browser

Security: binds to loopback only; every action requires a per-run token that is
injected into the served page. Actions are looked up from board.json by id —
the server never executes arbitrary strings from the request.

Run:  python3 serve.py            (opens the board in your browser)
"""
from __future__ import annotations

import json
import secrets
import shlex
import shutil
import subprocess
import tempfile
import urllib.parse
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import handoff

HERE = Path(__file__).resolve().parent
BOARD_DIR = HERE / "board"
TOKEN = secrets.token_urlsafe(18)
HOST, PORT = "127.0.0.1", 7666
CLAUDE = shutil.which("claude") or "claude"


def _card_by_id(card_id: str) -> dict | None:
    try:
        data = json.loads((BOARD_DIR / "board.json").read_text())
    except (OSError, json.JSONDecodeError):
        return None
    for c in data.get("cards", []):
        if c.get("id") == card_id:
            return c
    return None


TASKLORD_DIR = HERE


def _directive_prompt(card: dict, handoff_path=None) -> str:
    """The prompt copied to the clipboard to paste into the fresh session.
    Points the new agent at the handoff doc + TASK LORD, then the next step."""
    label = card.get("label", "this project")
    lines = []
    if handoff_path:
        lines += [
            f"Read the context handoff first: {handoff_path}",
            "It distills a prior session on this project so you can continue in a "
            "fresh context window. After reading it, pick up the work.",
            "",
        ]
    lines += [
        f"Also check TASK LORD for state/history: the SQLite board at "
        f"{TASKLORD_DIR}/tasklord.db (tables: projects, issues, updates, task_types).",
        "",
        f"Project: {label}",
    ]
    if card.get("identifier"):
        lines.append(f"Issue: {card['identifier']} — {card.get('title','')}")
    if card.get("left_off") and card["left_off"] != "—":
        lines.append(f"Where it left off: {card['left_off']}")
    if card.get("blockers"):
        lines.append(f"Blocker: {card['blockers']}")
    if card.get("next_step") and card["next_step"] != "—":
        lines.append(f"Your task (next step): {card['next_step']}")
    lines += [
        "",
        "When you finish or change state, update the TASK LORD issue (status, "
        "left_off, next_step) so the board stays truthful.",
    ]
    return "\n".join(lines)


def _pbcopy(text: str) -> None:
    try:
        subprocess.run(["pbcopy"], input=text.encode(), check=False)
    except OSError:
        pass


def _launch_terminal(shell_cmd: str) -> None:
    """Open a new Terminal.app window running shell_cmd interactively."""
    # Write to a temp launcher to dodge AppleScript quoting hell.
    fd = tempfile.NamedTemporaryFile(
        "w", suffix=".command", prefix="tasklord-cook-", delete=False
    )
    fd.write("#!/bin/bash\n" + shell_cmd + "\n")
    fd.close()
    Path(fd.name).chmod(0o755)
    script = fd.name
    osa = (
        f'tell application "Terminal" to do script {json.dumps(script)}\n'
        'tell application "Terminal" to activate'
    )
    subprocess.run(["osascript", "-e", osa], check=False)


def _cook(card: dict, dry: bool = False) -> dict:
    src = card.get("source")
    path = card.get("path")

    if src == "business":
        url = path or "https://trello.com"
        directive = _directive_prompt(card)
        if not dry:
            _pbcopy(directive)
            webbrowser.open(url)
        return {"ok": True, "action": "opened Trello", "target": url,
                "clipboard": "directive copied"}

    if not path:
        return {"ok": False, "error": "no project path for this card"}

    # Session card: distill the old session into a handoff, then start FRESH in
    # the same dir — clean context window, full continuity.
    handoff_path = None
    if src == "session" and card.get("session_id"):
        h = handoff.build_handoff(card["id"], card["session_id"], card)
        handoff_path = str(h) if h else None

    directive = _directive_prompt(card, handoff_path)
    cd = f"cd {shlex.quote(path)} || {{ echo 'path gone'; exit 1; }}"
    cmd = f"{cd}\nexec {shlex.quote(CLAUDE)}"   # always fresh; paste the directive
    action = "fresh session + handoff" if handoff_path else "fresh session"
    if dry:
        return {"ok": True, "dry": True, "action": action, "cmd": cmd,
                "handoff": handoff_path, "clipboard": directive}
    _pbcopy(directive)
    _launch_terminal(cmd)
    return {"ok": True, "action": action, "path": path, "handoff": handoff_path,
            "clipboard": "directive copied — paste into the session"}


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *a):  # quiet
        pass

    def _send(self, code: int, body: bytes, ctype: str):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        route = parsed.path
        q = urllib.parse.parse_qs(parsed.query)

        if route in ("/", "/index.html"):
            html = (BOARD_DIR / "index.html").read_text()
            # Inject in <head> so TOKEN is defined BEFORE the body scripts read it.
            inject = f'<script>window.TASKLORD_TOKEN={json.dumps(TOKEN)};</script>'
            html = html.replace("</head>", inject + "\n</head>", 1)
            return self._send(200, html.encode(), "text/html; charset=utf-8")

        if route in ("/board.json", "/board.js"):
            f = BOARD_DIR / route.lstrip("/")
            if f.exists():
                ctype = "application/json" if route.endswith(".json") else "application/javascript"
                return self._send(200, f.read_bytes(), ctype)
            return self._send(404, b"not found", "text/plain")

        if route == "/cook":
            if q.get("token", [""])[0] != TOKEN:
                return self._send(403, b'{"ok":false,"error":"bad token"}', "application/json")
            card = _card_by_id(q.get("id", [""])[0])
            if not card:
                return self._send(404, b'{"ok":false,"error":"card not found"}', "application/json")
            result = _cook(card, dry=q.get("dry", ["0"])[0] == "1")
            return self._send(200, json.dumps(result).encode(), "application/json")

        if route == "/refresh":
            if q.get("token", [""])[0] != TOKEN:
                return self._send(403, b'{"ok":false}', "application/json")
            subprocess.Popen(["bash", str(HERE / "run.sh")])
            return self._send(200, b'{"ok":true,"action":"refreshing"}', "application/json")

        return self._send(404, b"not found", "text/plain")


def main():
    url = f"http://{HOST}:{PORT}/?token={TOKEN}"
    srv = ThreadingHTTPServer((HOST, PORT), Handler)
    print(f"TASK LORD launchpad: {url}")
    print("Click a card to cook. Ctrl+C to stop.")
    webbrowser.open(url)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\nstopped.")


if __name__ == "__main__":
    main()
