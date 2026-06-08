"""SQLite store for TASK LORD — Linear's data model, adapted for a solo
high-agency operator (content + software + growth).

Faithful to Linear's core schema, trimmed/retuned for one operator who ships
across many domains:

  areas        (Linear: Teams)        life/work buckets — Content, Software, …
  initiatives  (Linear: Initiatives)  big bets spanning projects
  projects     (Linear: Projects)     status + health + target date + lead
  milestones   (Linear: Milestones)   checkpoints inside a project
  cycles       (Linear: Cycles)       weekly focus sprints
  states       (Linear: WorkflowStates) categorized: triage/backlog/unstarted/
                                       started/completed/canceled
  issues       (Linear: Issues)       the atomic unit — priority, estimate,
                                       assignee, parent (sub-issues), due date
  labels       (Linear: Labels)       + issue_labels join
  relations    (Linear: Relations)    blocks / blocked_by / relates / duplicate
  updates      (Linear: Project Updates) the "where it left off" log over time

The harvester maps each detected loose-end -> a Project (container) + an Issue
(the actionable next step) + an Update (left-off snapshot). The HTML board reads
board.json, which is exported FROM this db, so the db is the source of truth.
"""
from __future__ import annotations

import json
import sqlite3
from datetime import datetime, timezone
from pathlib import Path

DB_PATH = Path(__file__).resolve().parent / "tasklord.db"

# Linear workflow-state categories (exact set).
CATEGORIES = ("triage", "backlog", "unstarted", "started", "completed", "canceled")

# Default workflow states (name, category, position, color). Board columns map
# onto these. "Blocked" lives in the 'started' category but renders its own column.
DEFAULT_STATES = [
    ("Triage", "triage", 0, "#8a93a6"),
    ("Backlog", "backlog", 1, "#6b7280"),
    ("Todo", "unstarted", 2, "#9ca3af"),
    ("In Progress", "started", 3, "#3b82f6"),
    ("Blocked", "started", 4, "#ef4444"),
    ("In Review", "started", 5, "#a855f7"),
    ("Done", "completed", 6, "#22c55e"),
    ("Canceled", "canceled", 7, "#52525b"),
]

# Areas for a high-agency content/software operator (Linear Teams, retuned).
DEFAULT_AREAS = [
    ("content", "Content", "Owned pages, shorts, carousels, course", "#fbbf24"),
    ("software", "Software", "Tools, agents, services, infra", "#38bdf8"),
    ("growth", "Growth", "Campaigns, clients, sales, business ops", "#34d399"),
    ("monrovia", "Mon Rovia", "Artist project — proof of concept", "#f472b6"),
    ("personal", "Personal", "Health, learning, systems", "#a78bfa"),
]

# Linear priority scale.
PRIORITY = {0: "No priority", 1: "Urgent", 2: "High", 3: "Medium", 4: "Low"}

# Task-type taxonomy — the living vocabulary of "the kinds of work we do".
# Starter set for a content/software/high-agency operator; we refine this over
# time together. (key, name, area_key, color, keyword hints for inference)
DEFAULT_TASK_TYPES = [
    # Content
    ("script", "Script", "content", "#fbbf24", ["script", "hook", "caption", "copy"]),
    ("film", "Film / Record", "content", "#f59e0b", ["film", "record", "shoot", "footage"]),
    ("edit", "Edit", "content", "#f97316", ["edit", "cut", "clip", "reframe", "caption burn"]),
    ("publish", "Publish / Post", "content", "#eab308", ["post", "publish", "schedule", "upload", "distribute"]),
    ("carousel", "Carousel", "content", "#facc15", ["carousel", "slides", "midnight press"]),
    ("repurpose", "Repurpose", "content", "#fde047", ["repurpose", "remix", "atomize"]),
    # Software
    ("build", "Build / Feature", "software", "#38bdf8", ["build", "feature", "implement", "add", "wire", "ship"]),
    ("bug", "Bug", "software", "#ef4444", ["bug", "fix", "broken", "error", "crash", "regress"]),
    ("refactor", "Refactor", "software", "#22d3ee", ["refactor", "cleanup", "simplify", "migrate"]),
    ("spike", "Research / Spike", "software", "#0ea5e9", ["research", "spike", "investigate", "explore", "evaluate"]),
    ("review", "Review", "software", "#a855f7", ["review", "audit", "qa", "verify"]),
    ("infra", "Deploy / Infra", "software", "#14b8a6", ["deploy", "infra", "cron", "launchd", "server", "pipeline"]),
    ("docs", "Docs", "software", "#64748b", ["docs", "readme", "document", "write up"]),
    # Growth
    ("outreach", "Outreach", "growth", "#34d399", ["outreach", "dm", "cold", "lead", "reach out"]),
    ("proposal", "Proposal", "growth", "#10b981", ["proposal", "pitch", "deck", "quote"]),
    ("campaign", "Campaign", "growth", "#059669", ["campaign", "creators", "ugc", "brief"]),
    ("client", "Client Comms", "growth", "#2dd4bf", ["client", "email", "reply", "update", "report"]),
    # Cross-cutting
    ("decision", "Decision", None, "#c084fc", ["decide", "decision", "choose", "should we"]),
    ("idea", "Idea / Capture", None, "#94a3b8", ["idea", "capture", "maybe", "someday", "explore"]),
    ("followup", "Follow-up", None, "#fb7185", ["follow up", "follow-up", "waiting", "ping", "chase"]),
    ("admin", "Admin", "personal", "#a1a1aa", ["admin", "invoice", "payment", "schedule", "logistics"]),
]

SCHEMA = """
CREATE TABLE IF NOT EXISTS areas (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  key TEXT UNIQUE, name TEXT, description TEXT, color TEXT
);
CREATE TABLE IF NOT EXISTS states (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT, category TEXT, position INTEGER, color TEXT,
  UNIQUE(name)
);
CREATE TABLE IF NOT EXISTS initiatives (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT, description TEXT, status TEXT, health TEXT, target_date TEXT,
  created_at TEXT
);
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,                 -- stable key (path, or trello:<board>)
  name TEXT, description TEXT,
  area_id INTEGER, initiative_id INTEGER,
  status TEXT,                         -- backlog|planned|started|paused|completed|canceled
  health TEXT,                         -- on_track|at_risk|off_track
  lead TEXT, priority INTEGER,
  source TEXT, path TEXT,
  start_date TEXT, target_date TEXT,
  first_seen TEXT, updated_at TEXT,
  FOREIGN KEY(area_id) REFERENCES areas(id)
);
CREATE TABLE IF NOT EXISTS milestones (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id TEXT, name TEXT, target_date TEXT, status TEXT, position INTEGER
);
CREATE TABLE IF NOT EXISTS cycles (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  area_id INTEGER, number INTEGER, name TEXT,
  starts_at TEXT, ends_at TEXT, status TEXT
);
CREATE TABLE IF NOT EXISTS issues (
  id TEXT PRIMARY KEY,                 -- harvest card id
  identifier TEXT,                     -- e.g. SW-12 (area-key + seq)
  title TEXT, description TEXT,
  area_id INTEGER, project_id TEXT, milestone_id INTEGER, cycle_id INTEGER,
  type_id INTEGER,
  state_id INTEGER, priority INTEGER, estimate INTEGER,
  assignee TEXT, parent_id TEXT,
  source TEXT, session_id TEXT, path TEXT,
  due_date TEXT, last_activity TEXT, days_idle INTEGER, stale INTEGER,
  left_off TEXT, next_step TEXT, blockers TEXT, git TEXT,
  created_at TEXT, updated_at TEXT, completed_at TEXT,
  FOREIGN KEY(state_id) REFERENCES states(id),
  FOREIGN KEY(area_id) REFERENCES areas(id),
  FOREIGN KEY(project_id) REFERENCES projects(id)
);
CREATE TABLE IF NOT EXISTS task_types (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  key TEXT UNIQUE, name TEXT, area_key TEXT, color TEXT,
  hints TEXT,                          -- json array of keyword hints
  created_at TEXT
);
CREATE TABLE IF NOT EXISTS labels (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT UNIQUE, color TEXT, grp TEXT
);
CREATE TABLE IF NOT EXISTS issue_labels (
  issue_id TEXT, label_id INTEGER,
  PRIMARY KEY(issue_id, label_id)
);
CREATE TABLE IF NOT EXISTS relations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  issue_id TEXT, related_issue_id TEXT, type TEXT   -- blocks|blocked_by|relates|duplicate
);
CREATE TABLE IF NOT EXISTS updates (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type TEXT, entity_id TEXT,    -- project|issue
  health TEXT, state TEXT, body TEXT, observed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_issue_project ON issues(project_id);
CREATE INDEX IF NOT EXISTS idx_updates_entity ON updates(entity_type, entity_id);
"""


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def connect() -> sqlite3.Connection:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    conn.executescript(SCHEMA)
    return conn


def _seed(conn: sqlite3.Connection) -> None:
    cur = conn.cursor()
    for name, cat, pos, color in DEFAULT_STATES:
        cur.execute(
            "INSERT OR IGNORE INTO states(name,category,position,color) VALUES (?,?,?,?)",
            (name, cat, pos, color),
        )
    for key, name, desc, color in DEFAULT_AREAS:
        cur.execute(
            "INSERT OR IGNORE INTO areas(key,name,description,color) VALUES (?,?,?,?)",
            (key, name, desc, color),
        )
    now = _now()
    for key, name, area_key, color, hints in DEFAULT_TASK_TYPES:
        cur.execute(
            "INSERT OR IGNORE INTO task_types(key,name,area_key,color,hints,created_at)"
            " VALUES (?,?,?,?,?,?)",
            (key, name, area_key, color, json.dumps(hints), now),
        )
    conn.commit()


# ---- inference: map a harvested card onto the Linear model -------------------

# harvest status -> workflow state name
STATUS_TO_STATE = {
    "backlog": "Backlog",
    "in_progress": "In Progress",
    "blocked": "Blocked",
    "done": "Done",
}
# project status mirrors Linear's project status set
STATUS_TO_PROJECT = {
    "backlog": "backlog",
    "in_progress": "started",
    "blocked": "paused",
    "done": "completed",
}


def _infer_area_key(card: dict) -> str:
    blob = f"{card.get('label','')} {card.get('path','') or ''} {card.get('board','') or ''}".lower()
    if any(k in blob for k in ("monrovia", "mon rovia", "mon-rovia")):
        return "monrovia"
    if card.get("source") == "business":
        return "growth"
    if any(k in blob for k in (
        "carousel", "content", "reel", "tiktok", "youtube", "clip", "viral",
        "post", "script", "hook", "social",
    )):
        return "content"
    if any(k in blob for k in ("campaign", "sales", "outreach", "crm", "proposal", "client")):
        return "growth"
    if any(k in blob for k in ("health", "whoop", "memory", "personal", "learn")):
        return "personal"
    return "software"


def _infer_priority(card: dict) -> int:
    if card.get("blockers"):
        return 1  # Urgent
    status = card.get("status")
    if status == "blocked":
        return 2  # High
    if status == "in_progress":
        return 2 if not card.get("stale") else 3
    if status == "done":
        return 0
    return 4  # backlog -> Low


def _project_health(card: dict) -> str:
    if card.get("blockers") or card.get("status") == "blocked":
        return "off_track"
    if card.get("stale"):
        return "at_risk"
    return "on_track"


def _infer_type_id(card: dict, type_index: list[tuple]) -> int | None:
    """Best-effort task-type from title/next_step/left_off keyword hints.

    type_index is [(id, key, [hints...]), ...]. Falls back to 'build' for code
    and None otherwise — the taxonomy is refined by hand over time.
    """
    blob = " ".join(str(card.get(k, "") or "") for k in ("next_step", "title", "left_off", "label")).lower()
    for tid, key, hints in type_index:
        if any(h in blob for h in hints):
            return tid
    if card.get("source") == "code":
        for tid, key, _ in type_index:
            if key == "build":
                return tid
    return None


# ---- write path -------------------------------------------------------------

def upsert_cards(cards: list[dict]) -> dict:
    """Map harvested cards into projects + issues + updates. Returns a summary."""
    conn = connect()
    _seed(conn)
    now = _now()
    cur = conn.cursor()

    area_ids = {r["key"]: r["id"] for r in cur.execute("SELECT key,id FROM areas")}
    state_ids = {r["name"]: r["id"] for r in cur.execute("SELECT name,id FROM states")}
    type_index = [
        (r["id"], r["key"], json.loads(r["hints"] or "[]"))
        for r in cur.execute("SELECT id,key,hints FROM task_types")
    ]
    existing_issue = {
        r["id"]: r for r in cur.execute("SELECT id,state_id,left_off,created_at FROM issues")
    }
    existing_proj = {
        r["id"]: r for r in cur.execute("SELECT id,first_seen FROM projects")
    }
    # per-area running counter for identifiers
    seq = {k: (cur.execute(
        "SELECT COUNT(*) c FROM issues WHERE area_id=?", (aid,)
    ).fetchone()["c"]) for k, aid in area_ids.items()}

    new_i = changed_i = new_p = 0
    for c in cards:
        akey = _infer_area_key(c)
        aid = area_ids.get(akey, area_ids["software"])
        proj_id = c.get("path") or c["id"]
        prio = _infer_priority(c)
        health = _project_health(c)
        pstatus = STATUS_TO_PROJECT.get(c.get("status"), "started")
        git_json = json.dumps(c.get("git")) if c.get("git") else None

        # --- project (container) ---
        p_first = existing_proj.get(proj_id, {}).get("first_seen") if proj_id in existing_proj else now
        if proj_id not in existing_proj:
            new_p += 1
        cur.execute(
            """INSERT INTO projects
               (id,name,description,area_id,status,health,priority,source,path,
                target_date,first_seen,updated_at)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?)
               ON CONFLICT(id) DO UPDATE SET
                 name=excluded.name, area_id=excluded.area_id, status=excluded.status,
                 health=excluded.health, priority=excluded.priority,
                 source=excluded.source, path=excluded.path, updated_at=excluded.updated_at""",
            (proj_id, c.get("label"), c.get("left_off"), aid, pstatus, health, prio,
             c.get("source"), c.get("path"), c.get("due"), p_first, now),
        )

        # --- issue (the actionable loose-end) ---
        iid = c["id"]
        state_name = STATUS_TO_STATE.get(c.get("status"), "Backlog")
        sid = state_ids.get(state_name, state_ids["Backlog"])
        title = c.get("next_step") if c.get("next_step") and c["next_step"] != "—" else f"Resume {c.get('label')}"
        prev = existing_issue.get(iid)
        created = prev["created_at"] if prev else now
        if not prev:
            new_i += 1
            seq[akey] = seq.get(akey, 0) + 1
            identifier = f"{akey[:2].upper()}-{seq[akey]}"
        else:
            identifier = None  # keep existing
        moved = prev and (prev["state_id"] != sid or (prev["left_off"] or "") != (c.get("left_off") or ""))
        if not prev or moved:
            cur.execute(
                "INSERT INTO updates(entity_type,entity_id,health,state,body,observed_at)"
                " VALUES ('issue',?,?,?,?,?)",
                (iid, health, state_name, c.get("left_off"), now),
            )
            if prev:
                changed_i += 1
        completed_at = now if c.get("status") == "done" else None
        type_id = _infer_type_id({**c, "title": title}, type_index)
        cur.execute(
            """INSERT INTO issues
               (id,identifier,title,description,area_id,project_id,type_id,state_id,priority,
                source,session_id,path,due_date,last_activity,days_idle,stale,
                left_off,next_step,blockers,git,created_at,updated_at,completed_at)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
               ON CONFLICT(id) DO UPDATE SET
                 title=excluded.title, description=excluded.description,
                 area_id=excluded.area_id, project_id=excluded.project_id,
                 type_id=excluded.type_id, state_id=excluded.state_id, priority=excluded.priority,
                 source=excluded.source, session_id=excluded.session_id, path=excluded.path,
                 due_date=excluded.due_date, last_activity=excluded.last_activity,
                 days_idle=excluded.days_idle, stale=excluded.stale,
                 left_off=excluded.left_off, next_step=excluded.next_step,
                 blockers=excluded.blockers, git=excluded.git, updated_at=excluded.updated_at,
                 completed_at=excluded.completed_at""",
            (iid, identifier or (prev["id"] if prev else iid), title, c.get("left_off"),
             aid, proj_id, type_id, sid, prio, c.get("source"), c.get("session_id"), c.get("path"),
             c.get("due"), c.get("last_activity"), c.get("days_idle"),
             1 if c.get("stale") else 0, c.get("left_off"), c.get("next_step"),
             c.get("blockers"), git_json, created, now, completed_at),
        )
        # fix identifier on first insert (ON CONFLICT path can't set it cleanly)
        if not prev:
            cur.execute("UPDATE issues SET identifier=? WHERE id=? AND identifier=?",
                        (f"{akey[:2].upper()}-{seq[akey]}", iid, iid))

    conn.commit()
    conn.close()
    return {"issues_new": new_i, "issues_changed": changed_i, "projects_new": new_p,
            "total": len(cards)}


# ---- read path --------------------------------------------------------------

# board column key -> the state name(s) that fill it (keeps the 4-column UI)
COLUMN_STATES = {
    "backlog": ("Backlog", "Todo", "Triage"),
    "in_progress": ("In Progress", "In Review"),
    "blocked": ("Blocked",),
    "done": ("Done",),
}
STATE_TO_COLUMN = {s: col for col, names in COLUMN_STATES.items() for s in names}


def export_board() -> dict:
    """Render issues (joined to area/project/state) into the board.json shape."""
    conn = connect()
    rows = conn.execute(
        """SELECT i.*, s.name AS state_name, s.category AS state_category,
                  a.key AS area_key, a.name AS area_name, a.color AS area_color,
                  p.name AS project_name, p.health AS project_health,
                  t.name AS type_name, t.key AS type_key, t.color AS type_color
           FROM issues i
           LEFT JOIN states s ON i.state_id=s.id
           LEFT JOIN areas a ON i.area_id=a.id
           LEFT JOIN projects p ON i.project_id=p.id
           LEFT JOIN task_types t ON i.type_id=t.id"""
    ).fetchall()
    conn.close()

    cards = []
    for r in rows:
        d = dict(r)
        column = STATE_TO_COLUMN.get(d.get("state_name"), "backlog")
        cards.append({
            "id": d["id"],
            "identifier": d.get("identifier"),
            "label": d.get("project_name") or d.get("title"),
            "title": d.get("title"),
            "source": d.get("source"),
            "status": column,                       # keeps existing 4-column UI
            "state": d.get("state_name"),
            "area": d.get("area_name"),
            "area_key": d.get("area_key"),
            "area_color": d.get("area_color"),
            "priority": d.get("priority"),
            "priority_label": PRIORITY.get(d.get("priority"), ""),
            "type": d.get("type_name"),
            "type_key": d.get("type_key"),
            "type_color": d.get("type_color"),
            "health": d.get("project_health"),
            "left_off": d.get("left_off") or "—",
            "next_step": d.get("next_step") or "—",
            "blockers": d.get("blockers") or "",
            "path": d.get("path"),
            "session_id": d.get("session_id"),
            "last_activity": d.get("last_activity"),
            "days_idle": d.get("days_idle"),
            "stale": bool(d.get("stale")),
            "git": json.loads(d["git"]) if d.get("git") else None,
        })
    cards.sort(key=lambda c: (c.get("priority") if c.get("priority") else 9,
                              c.get("last_activity") or ""), reverse=False)
    # within priority, recent first: re-sort by (priority asc, activity desc)
    cards.sort(key=lambda c: c.get("last_activity") or "", reverse=True)
    cards.sort(key=lambda c: c.get("priority") if c.get("priority") not in (None, 0) else 9)

    statuses = ("backlog", "in_progress", "blocked", "done")
    counts = {s: sum(1 for c in cards if c["status"] == s) for s in statuses}
    areas = {}
    for c in cards:
        areas[c.get("area")] = areas.get(c.get("area"), 0) + 1
    return {
        "generated_at": _now(),
        "total": len(cards),
        "counts": counts,
        "areas": areas,
        "cards": cards,
    }


def issue_history(issue_id: str) -> list[dict]:
    conn = connect()
    rows = conn.execute(
        "SELECT health,state,body,observed_at FROM updates"
        " WHERE entity_type='issue' AND entity_id=? ORDER BY observed_at",
        (issue_id,),
    ).fetchall()
    conn.close()
    return [dict(r) for r in rows]
