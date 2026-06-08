//! Orchestrator: sources -> summarize (local LLM) -> SQLite -> board.json.

use crate::model::{Card, Summary};
use crate::sources::{code, transcripts, trello};
use crate::{config, llm, store};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::fs;

const VALID: [&str; 4] = ["backlog", "in_progress", "blocked", "done"];

const SUMMARY_PROMPT: &str = r#"You are a project triage assistant for a founder's software factory.
Given the tail of a work session and git facts for ONE project, output STRICT JSON:
{"status": "backlog"|"in_progress"|"blocked"|"done",
 "left_off": "1-2 sentences: concretely where this was left off",
 "next_step": "the single most useful next action, imperative voice",
 "blockers": "what's blocking it, or empty string"}

Rules: "done" only if clearly finished/shipped/verified. "blocked" if waiting on a
person/decision/credential/external thing. "in_progress" if mid-build. "backlog" if
barely started. Be specific. JSON only.

PROJECT: {label}
GIT FACTS: {facts}
SESSION TAIL:
{tail}
"#;

fn days_since(iso: &Option<String>) -> Option<i64> {
    let s = iso.as_ref()?;
    let dt = DateTime::parse_from_rfc3339(s).ok()?;
    Some((Utc::now() - dt.with_timezone(&Utc)).num_days())
}

fn facts_str(c: &Card) -> String {
    let mut bits = Vec::new();
    if let Some(g) = &c.git {
        if let Some(b) = &g.branch {
            bits.push(format!("git branch={}", b));
        }
        if let Some(m) = &g.last_commit_msg {
            bits.push(format!("last commit: {:?}", m));
        }
        if g.dirty {
            bits.push(format!("{} uncommitted changes", g.dirty_count));
        }
    }
    if let Some(d) = c.days_idle {
        bits.push(format!("last active {}d ago", d));
    }
    if bits.is_empty() {
        "none".into()
    } else {
        bits.join("; ")
    }
}

fn heuristic(c: &Card) -> Summary {
    let low = c.tail.to_lowercase();
    let status = if ["blocked", "waiting on", "need eric", "can't proceed"].iter().any(|w| low.contains(w)) {
        "blocked"
    } else if ["shipped", "done.", "complete", "deployed", "verified"].iter().any(|w| low.contains(w)) {
        "done"
    } else if c.days_idle.map(|d| d > 45).unwrap_or(false) {
        "backlog"
    } else {
        "in_progress"
    };
    let chunks: Vec<&str> = c.tail.split("\n\n").collect();
    let last_user = chunks
        .iter()
        .rev()
        .find(|chunk| chunk.starts_with("USER:"))
        .map(|s| s.trim_start_matches("USER:").trim().chars().take(200).collect::<String>())
        .unwrap_or_default();
    Summary {
        status: status.into(),
        left_off: if last_user.is_empty() { "No recent summary available.".into() } else { last_user },
        next_step: "Review session and resume.".into(),
        blockers: if status == "blocked" { "See session tail.".into() } else { String::new() },
    }
}

async fn summarize(c: &Card, use_llm: bool) -> Summary {
    if use_llm && !c.tail.is_empty() {
        let prompt = SUMMARY_PROMPT
            .replace("{label}", &c.label)
            .replace("{facts}", &facts_str(c))
            .replace("{tail}", &c.tail);
        if let Some(s) = llm::generate_json::<Summary>(&prompt).await {
            if VALID.contains(&s.status.as_str()) {
                return Summary {
                    status: s.status,
                    left_off: trunc(&s.left_off, 400, "—"),
                    next_step: trunc(&s.next_step, 300, "—"),
                    blockers: s.blockers.trim().chars().take(300).collect(),
                };
            }
        }
    }
    heuristic(c)
}

fn trunc(s: &str, n: usize, dflt: &str) -> String {
    let t: String = s.trim().chars().take(n).collect();
    if t.is_empty() {
        dflt.into()
    } else {
        t
    }
}

pub async fn run(use_llm: bool) -> Result<()> {
    let mut sessions = transcripts::collect();
    let code = code::collect();
    eprintln!("[harvest] {} session groups, {} code projects", sessions.len(), code.len());

    // merge git facts onto sessions by path
    let mut covered = std::collections::HashSet::new();
    for c in sessions.iter_mut() {
        if let Some(p) = &c.path {
            if let Some(f) = code.get(p) {
                c.git = f.git.clone();
                covered.insert(p.clone());
            }
        }
    }
    // cold code projects (no recent session)
    for (path, f) in &code {
        if covered.contains(path) {
            continue;
        }
        sessions.push(Card {
            id: path.clone(),
            source: "code".into(),
            label: f.label.clone(),
            path: Some(path.clone()),
            git: f.git.clone(),
            last_activity: f.last_commit.clone(),
            ..Default::default()
        });
    }

    let use_llm = use_llm && llm::available().await;
    if use_llm {
        eprintln!("[harvest] summarizing via {}", llm::describe());
    } else {
        eprintln!("[harvest] LLM unavailable — using heuristics");
    }

    let total = sessions.len();
    let mut cards: Vec<Card> = Vec::with_capacity(total);
    for (i, mut c) in sessions.into_iter().enumerate() {
        c.days_idle = days_since(&c.last_activity);
        c.stale = c.days_idle.map(|d| d > 30).unwrap_or(false);
        let s = summarize(&c, use_llm).await;
        c.status = s.status;
        c.left_off = s.left_off;
        c.next_step = s.next_step;
        c.blockers = s.blockers;
        cards.push(c);
        if (i + 1) % 10 == 0 || i + 1 == total {
            eprintln!("[harvest] summarized {}/{}", i + 1, total);
        }
    }

    // trello business cards (already classified)
    for mut t in trello::collect() {
        t.days_idle = days_since(&t.last_activity);
        t.stale = t.days_idle.map(|d| d > 30).unwrap_or(false);
        cards.push(t);
    }

    let (new_i, changed_i) = store::upsert_cards(&cards)?;
    eprintln!("[harvest] db: {} new issues, {} changed", new_i, changed_i);

    let board = store::export_board()?;
    write_board(&board)?;
    eprintln!(
        "[harvest] wrote tasklord.db + board.json — {} cards {:?}",
        board.total, board.counts
    );
    Ok(())
}

fn write_board(board: &crate::model::Board) -> Result<()> {
    let dir = config::board_dir();
    fs::create_dir_all(&dir)?;
    let payload = serde_json::to_string_pretty(board)?;
    fs::write(dir.join("board.json"), &payload)?;
    fs::write(dir.join("board.js"), format!("window.BOARD = {};\n", payload))?;
    Ok(())
}
