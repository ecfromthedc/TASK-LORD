//! SQLite store — Linear's data model, adapted for a solo high-agency operator.
//! Source of truth; the board.json the UI reads is exported from here.

use crate::config;
use crate::model::{Board, Card, ExportCard, GitInfo};
use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::collections::{BTreeMap, HashSet};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS areas (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  key TEXT UNIQUE, name TEXT, description TEXT, color TEXT
);
CREATE TABLE IF NOT EXISTS states (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT UNIQUE, category TEXT, position INTEGER, color TEXT
);
CREATE TABLE IF NOT EXISTS initiatives (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT, description TEXT, status TEXT, health TEXT, target_date TEXT, created_at TEXT
);
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY, name TEXT, description TEXT,
  area_id INTEGER, initiative_id INTEGER, status TEXT, health TEXT,
  lead TEXT, priority INTEGER, source TEXT, path TEXT,
  start_date TEXT, target_date TEXT, first_seen TEXT, updated_at TEXT
);
CREATE TABLE IF NOT EXISTS milestones (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id TEXT, name TEXT, target_date TEXT, status TEXT, position INTEGER
);
CREATE TABLE IF NOT EXISTS cycles (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  area_id INTEGER, number INTEGER, name TEXT, starts_at TEXT, ends_at TEXT, status TEXT
);
CREATE TABLE IF NOT EXISTS task_types (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  key TEXT UNIQUE, name TEXT, area_key TEXT, color TEXT, hints TEXT, created_at TEXT
);
CREATE TABLE IF NOT EXISTS issues (
  id TEXT PRIMARY KEY, identifier TEXT, title TEXT, description TEXT,
  area_id INTEGER, project_id TEXT, milestone_id INTEGER, cycle_id INTEGER,
  type_id INTEGER, state_id INTEGER, priority INTEGER, estimate INTEGER,
  assignee TEXT, parent_id TEXT, source TEXT, session_id TEXT, path TEXT,
  due_date TEXT, last_activity TEXT, days_idle INTEGER, stale INTEGER,
  left_off TEXT, next_step TEXT, blockers TEXT, git TEXT,
  created_at TEXT, updated_at TEXT, completed_at TEXT
);
CREATE TABLE IF NOT EXISTS labels (
  id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE, color TEXT, grp TEXT
);
CREATE TABLE IF NOT EXISTS issue_labels (issue_id TEXT, label_id INTEGER, PRIMARY KEY(issue_id,label_id));
CREATE TABLE IF NOT EXISTS relations (
  id INTEGER PRIMARY KEY AUTOINCREMENT, issue_id TEXT, related_issue_id TEXT, type TEXT
);
CREATE TABLE IF NOT EXISTS updates (
  id INTEGER PRIMARY KEY AUTOINCREMENT, entity_type TEXT, entity_id TEXT,
  health TEXT, state TEXT, body TEXT, observed_at TEXT
);
CREATE TABLE IF NOT EXISTS dismissals (
  card_id TEXT PRIMARY KEY, reason TEXT, dismissed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_issue_project ON issues(project_id);
CREATE INDEX IF NOT EXISTS idx_updates_entity ON updates(entity_type, entity_id);
"#;

fn now() -> String {
    Utc::now().to_rfc3339()
}

pub fn open() -> Result<Connection> {
    let conn = Connection::open(config::db_path())?;
    conn.execute_batch(SCHEMA)?;
    seed(&conn)?;
    Ok(conn)
}

fn seed(conn: &Connection) -> Result<()> {
    for (name, cat, pos, color) in config::STATES {
        conn.execute(
            "INSERT OR IGNORE INTO states(name,category,position,color) VALUES (?1,?2,?3,?4)",
            params![name, cat, pos, color],
        )?;
    }
    for (key, name, desc, color) in config::AREAS {
        conn.execute(
            "INSERT OR IGNORE INTO areas(key,name,description,color) VALUES (?1,?2,?3,?4)",
            params![key, name, desc, color],
        )?;
    }
    let ts = now();
    for (key, name, area_key, color, hints) in config::TASK_TYPES {
        let hints_json = serde_json::to_string(hints).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "INSERT OR IGNORE INTO task_types(key,name,area_key,color,hints,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![key, name, area_key, color, hints_json, ts],
        )?;
    }
    Ok(())
}

// ---- dismissals -------------------------------------------------------------

pub fn dismiss(id: &str, reason: &str) -> Result<()> {
    let conn = open()?;
    conn.execute(
        "INSERT OR REPLACE INTO dismissals(card_id,reason,dismissed_at) VALUES (?1,?2,?3)",
        params![id, reason, now()],
    )?;
    conn.execute("DELETE FROM issues WHERE id=?1", params![id])?;
    Ok(())
}

pub fn undismiss(id: &str) -> Result<()> {
    let conn = open()?;
    conn.execute("DELETE FROM dismissals WHERE card_id=?1", params![id])?;
    Ok(())
}

fn dismissed_set(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT card_id FROM dismissals")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

// ---- inference --------------------------------------------------------------

fn infer_area_key(c: &Card) -> &'static str {
    let blob = format!(
        "{} {} {}",
        c.label,
        c.path.clone().unwrap_or_default(),
        c.board.clone().unwrap_or_default()
    )
    .to_lowercase();
    let has = |kws: &[&str]| kws.iter().any(|k| blob.contains(k));
    if has(&["monrovia", "mon rovia", "mon-rovia"]) {
        return "monrovia";
    }
    if c.source == "business" {
        return "growth";
    }
    if has(&["carousel", "content", "reel", "tiktok", "youtube", "clip", "viral", "post", "script", "hook", "social"]) {
        return "content";
    }
    if has(&["campaign", "sales", "outreach", "crm", "proposal", "client"]) {
        return "growth";
    }
    if has(&["health", "whoop", "memory", "personal", "learn"]) {
        return "personal";
    }
    "software"
}

fn infer_priority(c: &Card) -> i64 {
    if !c.blockers.is_empty() {
        return 1;
    }
    match c.status.as_str() {
        "blocked" => 2,
        "in_progress" => {
            if c.stale {
                3
            } else {
                2
            }
        }
        "done" => 0,
        _ => 4,
    }
}

fn project_health(c: &Card) -> &'static str {
    if !c.blockers.is_empty() || c.status == "blocked" {
        "off_track"
    } else if c.stale {
        "at_risk"
    } else {
        "on_track"
    }
}

fn infer_type_id(c: &Card, type_index: &[(i64, String, Vec<String>)]) -> Option<i64> {
    let blob = format!("{} {} {}", c.next_step, c.left_off, c.label).to_lowercase();
    for (id, _key, hints) in type_index {
        if hints.iter().any(|h| blob.contains(h)) {
            return Some(*id);
        }
    }
    if c.source == "code" {
        for (id, key, _) in type_index {
            if key == "build" {
                return Some(*id);
            }
        }
    }
    None
}

// ---- write path -------------------------------------------------------------

pub fn upsert_cards(cards: &[Card]) -> Result<(usize, usize)> {
    let conn = open()?;
    let ts = now();
    let dismissed = dismissed_set(&conn)?;

    let mut area_ids: BTreeMap<String, i64> = BTreeMap::new();
    {
        let mut stmt = conn.prepare("SELECT key,id FROM areas")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for r in rows.flatten() {
            area_ids.insert(r.0, r.1);
        }
    }
    let mut state_ids: BTreeMap<String, i64> = BTreeMap::new();
    {
        let mut stmt = conn.prepare("SELECT name,id FROM states")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for r in rows.flatten() {
            state_ids.insert(r.0, r.1);
        }
    }
    let mut type_index: Vec<(i64, String, Vec<String>)> = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT id,key,hints FROM task_types")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        })?;
        for (id, key, hints) in rows.flatten() {
            let h: Vec<String> = serde_json::from_str(&hints).unwrap_or_default();
            type_index.push((id, key, h));
        }
    }

    let software_aid = *area_ids.get("software").unwrap_or(&1);
    let backlog_sid = *state_ids.get("Backlog").unwrap_or(&1);

    // per-area identifier counters
    let mut seq: BTreeMap<String, i64> = BTreeMap::new();
    for (key, aid) in &area_ids {
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE area_id=?1",
            params![aid],
            |r| r.get(0),
        )?;
        seq.insert(key.clone(), n);
    }

    let mut new_i = 0usize;
    let mut changed_i = 0usize;

    for c in cards {
        if dismissed.contains(&c.id) {
            continue; // never repopulate a dismissed task
        }
        let akey = infer_area_key(c);
        let aid = *area_ids.get(akey).unwrap_or(&software_aid);
        let proj_id = c.path.clone().unwrap_or_else(|| c.id.clone());
        let prio = infer_priority(c);
        let health = project_health(c);
        let pstatus = config::status_to_project(&c.status);
        let git_json = c.git.as_ref().map(|g| serde_json::to_string(g).unwrap_or_default());

        // project
        let p_first: Option<String> = conn
            .query_row("SELECT first_seen FROM projects WHERE id=?1", params![proj_id], |r| r.get(0))
            .ok();
        let first_seen = p_first.unwrap_or_else(|| ts.clone());
        conn.execute(
            "INSERT INTO projects (id,name,description,area_id,status,health,priority,source,path,target_date,first_seen,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name,area_id=excluded.area_id,status=excluded.status,
               health=excluded.health,priority=excluded.priority,source=excluded.source,path=excluded.path,updated_at=excluded.updated_at",
            params![proj_id, c.label, c.left_off, aid, pstatus, health, prio, c.source, c.path, c.due, first_seen, ts],
        )?;

        // issue
        let state_name = config::status_to_state(&c.status);
        let sid = *state_ids.get(state_name).unwrap_or(&backlog_sid);
        let title = if !c.next_step.is_empty() && c.next_step != "—" {
            c.next_step.clone()
        } else {
            format!("Resume {}", c.label)
        };
        let prev: Option<(i64, String, String)> = conn
            .query_row(
                "SELECT state_id,COALESCE(left_off,''),created_at FROM issues WHERE id=?1",
                params![c.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .ok();
        let created = prev.as_ref().map(|p| p.2.clone()).unwrap_or_else(|| ts.clone());
        let identifier = if prev.is_none() {
            let n = seq.entry(akey.to_string()).or_insert(0);
            *n += 1;
            format!("{}-{}", akey[..2].to_uppercase(), n)
        } else {
            conn.query_row("SELECT identifier FROM issues WHERE id=?1", params![c.id], |r| r.get(0))
                .unwrap_or_else(|_| c.id.clone())
        };
        let moved = match &prev {
            Some((psid, ploff, _)) => *psid != sid || *ploff != c.left_off,
            None => true,
        };
        if moved {
            conn.execute(
                "INSERT INTO updates(entity_type,entity_id,health,state,body,observed_at) VALUES ('issue',?1,?2,?3,?4,?5)",
                params![c.id, health, state_name, c.left_off, ts],
            )?;
            if prev.is_some() {
                changed_i += 1;
            }
        }
        if prev.is_none() {
            new_i += 1;
        }
        let type_id = infer_type_id(c, &type_index);
        let completed_at = if c.status == "done" { Some(ts.clone()) } else { None };
        conn.execute(
            "INSERT INTO issues
             (id,identifier,title,description,area_id,project_id,type_id,state_id,priority,
              source,session_id,path,due_date,last_activity,days_idle,stale,
              left_off,next_step,blockers,git,created_at,updated_at,completed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title,description=excluded.description,
               area_id=excluded.area_id,project_id=excluded.project_id,type_id=excluded.type_id,
               state_id=excluded.state_id,priority=excluded.priority,source=excluded.source,
               session_id=excluded.session_id,path=excluded.path,due_date=excluded.due_date,
               last_activity=excluded.last_activity,days_idle=excluded.days_idle,stale=excluded.stale,
               left_off=excluded.left_off,next_step=excluded.next_step,blockers=excluded.blockers,
               git=excluded.git,updated_at=excluded.updated_at,completed_at=excluded.completed_at",
            params![
                c.id, identifier, title, c.left_off, aid, proj_id, type_id, sid, prio,
                c.source, c.session_id, c.path, c.due, c.last_activity, c.days_idle,
                c.stale as i64, c.left_off, c.next_step, c.blockers, git_json, created, ts, completed_at
            ],
        )?;
    }
    Ok((new_i, changed_i))
}

// ---- read path --------------------------------------------------------------

pub fn export_board() -> Result<Board> {
    let conn = open()?;
    let prio_label: BTreeMap<i64, &str> = config::PRIORITY.iter().cloned().collect();

    let mut stmt = conn.prepare(
        "SELECT i.id,i.identifier,i.title,i.source,i.left_off,i.next_step,i.blockers,
                i.path,i.session_id,i.last_activity,i.days_idle,i.stale,i.priority,i.git,
                s.name,a.key,a.name,a.color,p.name,p.health,t.name,t.key,t.color
         FROM issues i
         LEFT JOIN states s ON i.state_id=s.id
         LEFT JOIN areas a ON i.area_id=a.id
         LEFT JOIN projects p ON i.project_id=p.id
         LEFT JOIN task_types t ON i.type_id=t.id",
    )?;
    let rows = stmt.query_map([], |r| {
        let git_raw: Option<String> = r.get(13)?;
        let git: Option<GitInfo> = git_raw.and_then(|s| serde_json::from_str(&s).ok());
        let state_name: Option<String> = r.get(14)?;
        let priority: Option<i64> = r.get(12)?;
        let title: Option<String> = r.get(2)?;
        let project_name: Option<String> = r.get(18)?;
        let column = config::state_to_column(state_name.as_deref().unwrap_or("Backlog")).to_string();
        Ok(ExportCard {
            id: r.get(0)?,
            identifier: r.get(1)?,
            label: project_name.clone().or_else(|| title.clone()).unwrap_or_default(),
            title,
            source: r.get(3)?,
            status: column,
            state: state_name,
            area: r.get(16)?,
            area_key: r.get(15)?,
            area_color: r.get(17)?,
            priority,
            priority_label: priority.and_then(|p| prio_label.get(&p).map(|s| s.to_string())),
            type_name: r.get(20)?,
            type_key: r.get(21)?,
            type_color: r.get(22)?,
            health: r.get(19)?,
            left_off: r.get::<_, Option<String>>(4)?.unwrap_or_else(|| "—".into()),
            next_step: r.get::<_, Option<String>>(5)?.unwrap_or_else(|| "—".into()),
            blockers: r.get::<_, Option<String>>(6)?.unwrap_or_default(),
            path: r.get(7)?,
            session_id: r.get(8)?,
            last_activity: r.get(9)?,
            days_idle: r.get(10)?,
            stale: r.get::<_, Option<i64>>(11)?.unwrap_or(0) != 0,
            git,
        })
    })?;

    let mut cards: Vec<ExportCard> = rows.filter_map(|r| r.ok()).collect();
    // priority (non-zero first), then most recent
    cards.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
    cards.sort_by_key(|c| match c.priority {
        Some(0) | None => 9,
        Some(p) => p,
    });

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for s in ["backlog", "in_progress", "blocked", "done"] {
        counts.insert(s.to_string(), cards.iter().filter(|c| c.status == s).count());
    }
    let mut areas: BTreeMap<String, usize> = BTreeMap::new();
    for c in &cards {
        if let Some(a) = &c.area {
            *areas.entry(a.clone()).or_insert(0) += 1;
        }
    }

    Ok(Board {
        generated_at: now(),
        total: cards.len(),
        counts,
        areas,
        cards,
    })
}
