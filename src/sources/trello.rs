//! Source: Trello business workstreams via the local `trello` CLI.
//! Each board rolls up to one workstream card (its cards are sub-tasks).

use crate::model::Card;
use serde_json::Value;
use std::process::Command;

const MAX_BOARDS: usize = 20;
const MAX_CARDS_PER_LIST: usize = 60;

fn list_status(list_name: &str) -> &'static str {
    let l = list_name.to_lowercase();
    let hint = |k: &str| l.contains(k);
    if hint("done") || hint("complete") || hint("shipped") {
        "done"
    } else if hint("doing") || hint("in progress") || hint("go mode") || hint("wip") || hint("active") {
        "in_progress"
    } else if hint("blocked") || hint("waiting") || hint("on hold") || hint("stuck") {
        "blocked"
    } else {
        "backlog"
    }
}

fn run(args: &[&str]) -> Option<String> {
    let out = Command::new("trello").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

fn run_json(args: &[&str]) -> Option<Value> {
    serde_json::from_str(&run(args)?).ok()
}

pub fn collect() -> Vec<Card> {
    let _ = run(&["sync"]); // refresh name->id cache
    let boards = match run_json(&["board:list", "--format", "json"]) {
        Some(Value::Array(b)) => b,
        _ => return Vec::new(),
    };

    let mut out = Vec::new();
    for board in boards.into_iter().take(MAX_BOARDS) {
        let Some(bname) = board.get("name").and_then(|n| n.as_str()) else { continue };
        let lists = match run_json(&["list:list", "--board", bname, "--format", "json"]) {
            Some(Value::Array(l)) => l,
            _ => continue,
        };

        let mut open_cards = 0i64;
        let mut counts: std::collections::BTreeMap<&str, i64> = std::collections::BTreeMap::new();
        let mut last_activity: Option<String> = None;
        let mut recent: Option<(String, String, String)> = None; // (act, name, list)
        let mut recent_active: Option<(String, String)> = None;

        for lst in lists {
            let Some(lname) = lst.get("name").and_then(|n| n.as_str()) else { continue };
            let lstatus = list_status(lname);
            let cards = match run_json(&["card:list", "--board", bname, "--list", lname, "--format", "json"]) {
                Some(Value::Array(c)) => c,
                _ => continue,
            };
            for c in cards.into_iter().take(MAX_CARDS_PER_LIST) {
                if c.get("closed").and_then(|x| x.as_bool()).unwrap_or(false) {
                    continue;
                }
                let Some(name) = c.get("name").and_then(|n| n.as_str()) else { continue };
                if lstatus != "done" {
                    open_cards += 1;
                }
                *counts.entry(lstatus).or_insert(0) += 1;
                let act = c.get("dateLastActivity").and_then(|a| a.as_str()).unwrap_or("");
                if !act.is_empty() {
                    if last_activity.as_deref().map(|l| act > l).unwrap_or(true) {
                        last_activity = Some(act.to_string());
                    }
                    if recent.as_ref().map(|(a, _, _)| act > a.as_str()).unwrap_or(true) {
                        recent = Some((act.to_string(), name.to_string(), lname.to_string()));
                    }
                    if lstatus == "in_progress"
                        && recent_active.as_ref().map(|(a, _)| act > a.as_str()).unwrap_or(true)
                    {
                        recent_active = Some((act.to_string(), name.to_string()));
                    }
                }
            }
        }

        if counts.is_empty() {
            continue;
        }
        let bstatus = if open_cards == 0 {
            "done"
        } else if counts.get("blocked").copied().unwrap_or(0) > 0 {
            "blocked"
        } else if counts.get("in_progress").copied().unwrap_or(0) > 0 {
            "in_progress"
        } else {
            "backlog"
        };
        let mix: Vec<String> = counts
            .iter()
            .map(|(s, n)| format!("{} {}", n, s.replace('_', " ")))
            .collect();
        let mut left = format!("{} open cards ({}).", open_cards, mix.join(", "));
        if let Some((_, n, l)) = &recent {
            left.push_str(&format!(" Most recent: “{}” in {}.", &n[..n.len().min(80)], l));
        }
        let next = recent_active
            .map(|(_, n)| format!("Advance “{}”", &n[..n.len().min(80)]))
            .unwrap_or_else(|| "—".into());

        out.push(Card {
            id: format!("trello:{}", bname),
            source: "business".into(),
            label: bname.to_string(),
            status: bstatus.into(),
            left_off: left,
            next_step: next,
            board: Some(bname.to_string()),
            trello_list: Some(format!("{} open", open_cards)),
            last_activity,
            ..Default::default()
        });
    }
    out
}
