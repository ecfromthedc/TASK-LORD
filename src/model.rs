//! Shared data shapes for the harvest pipeline and the exported board.

use serde::{Deserialize, Serialize};

/// A harvested loose-end before it is mapped into the Linear model.
/// Produced by the source scanners, enriched by the summarizer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub source: String, // session | code | business
    pub label: String,
    pub status: String, // backlog | in_progress | blocked | done
    pub left_off: String,
    pub next_step: String,
    pub blockers: String,
    pub path: Option<String>,
    pub session_id: Option<String>,
    pub last_activity: Option<String>,
    pub days_idle: Option<i64>,
    pub stale: bool,
    pub git: Option<GitInfo>,
    pub board: Option<String>,
    pub trello_list: Option<String>,
    pub due: Option<String>,
    pub msg_count: Option<i64>,

    // internal-only, not part of the issue record
    #[serde(skip)]
    pub tail: String,
    #[serde(skip)]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: Option<String>,
    pub dirty: bool,
    pub dirty_count: i64,
    pub last_commit_msg: Option<String>,
}

/// What the LLM (or heuristic) returns for one card.
#[derive(Debug, Clone, Deserialize)]
pub struct Summary {
    pub status: String,
    #[serde(default)]
    pub left_off: String,
    #[serde(default)]
    pub next_step: String,
    #[serde(default)]
    pub blockers: String,
}

/// The enriched record the board UI consumes (board.json `cards[]`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCard {
    pub id: String,
    pub identifier: Option<String>,
    pub label: String,
    pub title: Option<String>,
    pub source: Option<String>,
    pub status: String, // column key: backlog|in_progress|blocked|done
    pub state: Option<String>,
    pub area: Option<String>,
    pub area_key: Option<String>,
    pub area_color: Option<String>,
    pub priority: Option<i64>,
    pub priority_label: Option<String>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub type_key: Option<String>,
    pub type_color: Option<String>,
    pub health: Option<String>,
    pub left_off: String,
    pub next_step: String,
    pub blockers: String,
    pub path: Option<String>,
    pub session_id: Option<String>,
    pub last_activity: Option<String>,
    pub days_idle: Option<i64>,
    pub stale: bool,
    pub git: Option<GitInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Board {
    pub generated_at: String,
    pub total: usize,
    pub counts: std::collections::BTreeMap<String, usize>,
    pub areas: std::collections::BTreeMap<String, usize>,
    pub cards: Vec<ExportCard>,
}
