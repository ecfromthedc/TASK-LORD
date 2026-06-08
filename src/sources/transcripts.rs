//! Source: Claude Code session transcripts (~/.claude/projects/*/*.jsonl).
//! The loose-ends goldmine — reconstructs where each session left off.

use crate::config;
use crate::model::Card;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const TAIL_MESSAGES: usize = 18;
const MAX_CHARS: usize = 7000;

fn text_from_content(content: &Value) -> String {
    if let Some(s) = content.as_str() {
        return s.to_string();
    }
    if let Some(arr) = content.as_array() {
        let mut parts = Vec::new();
        for b in arr {
            match b.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                        parts.push(t.to_string());
                    }
                }
                Some("tool_use") => {
                    let name = b.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    parts.push(format!("[tool: {}]", name));
                }
                Some("tool_result") => parts.push("[tool result]".to_string()),
                _ => {}
            }
        }
        return parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join("\n");
    }
    String::new()
}

fn newest_jsonl(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            if let Ok(m) = entry.metadata().and_then(|md| md.modified()) {
                if best.as_ref().map(|(t, _)| m > *t).unwrap_or(true) {
                    best = Some((m, p));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

struct Parsed {
    cwd: Option<String>,
    last_activity: Option<String>,
    tail: String,
    msg_count: usize,
}

fn parse_transcript(path: &Path) -> Option<Parsed> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut cwd = None;
    let mut last_ts: Option<String> = None;
    let mut messages: Vec<String> = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        let rec: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if cwd.is_none() {
            if let Some(c) = rec.get("cwd").and_then(|c| c.as_str()) {
                cwd = Some(c.to_string());
            }
        }
        if let Some(ts) = rec.get("timestamp").and_then(|t| t.as_str()) {
            if last_ts.as_deref().map(|l| ts > l).unwrap_or(true) {
                last_ts = Some(ts.to_string());
            }
        }
        let typ = rec.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if typ == "user" || typ == "assistant" {
            if let Some(msg) = rec.get("message") {
                if let Some(content) = msg.get("content") {
                    let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or(typ);
                    let text = text_from_content(content);
                    let text = text.trim();
                    if !text.is_empty() && text != "[tool result]" {
                        messages.push(format!("{}: {}", role.to_uppercase(), text));
                    }
                }
            }
        }
    }

    let start = messages.len().saturating_sub(TAIL_MESSAGES);
    let mut convo = messages[start..].join("\n\n");
    if convo.len() > MAX_CHARS {
        convo = convo[convo.len() - MAX_CHARS..].to_string();
    }
    Some(Parsed {
        cwd,
        last_activity: last_ts,
        tail: convo,
        msg_count: messages.len(),
    })
}

pub fn collect() -> Vec<Card> {
    let dir = config::projects_dir();
    let mut out = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    let home = config::home();
    for entry in entries.flatten() {
        let group_dir = entry.path();
        if !group_dir.is_dir() {
            continue;
        }
        let Some(newest) = newest_jsonl(&group_dir) else { continue };
        let Some(parsed) = parse_transcript(&newest) else { continue };
        if parsed.tail.is_empty() {
            continue;
        }
        let group_name = group_dir.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let label = match &parsed.cwd {
            Some(c) => Path::new(c)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(c)
                .to_string(),
            None => group_name.trim_start_matches('-').rsplit('-').next().unwrap_or("").to_string(),
        };
        // skip rootless / home-level noise
        let is_home = parsed.cwd.as_deref() == Some(home.to_str().unwrap_or(""));
        if label.trim().is_empty() || label == "/" || parsed.cwd.as_deref() == Some("/") || is_home {
            continue;
        }
        let session_id = newest.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string());
        out.push(Card {
            id: group_name.clone(),
            source: "session".into(),
            label,
            path: parsed.cwd,
            group: Some(group_name),
            session_id,
            last_activity: parsed.last_activity,
            msg_count: Some(parsed.msg_count as i64),
            tail: parsed.tail,
            ..Default::default()
        });
    }
    out
}
