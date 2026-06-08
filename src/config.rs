//! Static configuration: paths, seed taxonomy (areas, states, task types),
//! and the mappings the harvester uses. Edit the seeds to fit your work.

use std::path::PathBuf;

pub fn home() -> PathBuf {
    dirs::home_dir().expect("no home dir")
}

/// Repo / engine root (dir containing the binary's project files).
pub fn engine_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR at build; fall back to current dir at runtime.
    std::env::var("TASKLORD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

pub fn db_path() -> PathBuf {
    engine_dir().join("tasklord.db")
}
pub fn board_dir() -> PathBuf {
    engine_dir().join("board")
}
pub fn handoff_dir() -> PathBuf {
    engine_dir().join("handoffs")
}
pub fn projects_dir() -> PathBuf {
    home().join(".claude").join("projects")
}

/// Where to look for code projects (git repos / plan files).
pub fn scan_roots() -> Vec<PathBuf> {
    vec![
        home().join("Projects").join("active"),
        home().join("Documents").join("Development"),
    ]
}

// (name, category, position, color)
pub const STATES: &[(&str, &str, i64, &str)] = &[
    ("Triage", "triage", 0, "#8a93a6"),
    ("Backlog", "backlog", 1, "#6b7280"),
    ("Todo", "unstarted", 2, "#9ca3af"),
    ("In Progress", "started", 3, "#3b82f6"),
    ("Blocked", "started", 4, "#ef4444"),
    ("In Review", "started", 5, "#a855f7"),
    ("Done", "completed", 6, "#22c55e"),
    ("Canceled", "canceled", 7, "#52525b"),
];

// (key, name, description, color)
pub const AREAS: &[(&str, &str, &str, &str)] = &[
    ("content", "Content", "Owned pages, shorts, carousels, course", "#fbbf24"),
    ("software", "Software", "Tools, agents, services, infra", "#38bdf8"),
    ("growth", "Growth", "Campaigns, clients, sales, business ops", "#34d399"),
    ("monrovia", "Mon Rovia", "Artist project — proof of concept", "#f472b6"),
    ("personal", "Personal", "Health, learning, systems", "#a78bfa"),
];

// (key, name, area_key (or ""), color, hints)
pub const TASK_TYPES: &[(&str, &str, &str, &str, &[&str])] = &[
    ("script", "Script", "content", "#fbbf24", &["script", "hook", "caption", "copy"]),
    ("film", "Film / Record", "content", "#f59e0b", &["film", "record", "shoot", "footage"]),
    ("edit", "Edit", "content", "#f97316", &["edit", "cut", "clip", "reframe"]),
    ("publish", "Publish / Post", "content", "#eab308", &["post", "publish", "schedule", "upload", "distribute"]),
    ("carousel", "Carousel", "content", "#facc15", &["carousel", "slides", "midnight press"]),
    ("repurpose", "Repurpose", "content", "#fde047", &["repurpose", "remix", "atomize"]),
    ("build", "Build / Feature", "software", "#38bdf8", &["build", "feature", "implement", "add", "wire", "ship"]),
    ("bug", "Bug", "software", "#ef4444", &["bug", "fix", "broken", "error", "crash", "regress"]),
    ("refactor", "Refactor", "software", "#22d3ee", &["refactor", "cleanup", "simplify", "migrate"]),
    ("spike", "Research / Spike", "software", "#0ea5e9", &["research", "spike", "investigate", "explore", "evaluate"]),
    ("review", "Review", "software", "#a855f7", &["review", "audit", "qa", "verify"]),
    ("infra", "Deploy / Infra", "software", "#14b8a6", &["deploy", "infra", "cron", "launchd", "server", "pipeline"]),
    ("docs", "Docs", "software", "#64748b", &["docs", "readme", "document", "write up"]),
    ("outreach", "Outreach", "growth", "#34d399", &["outreach", "dm", "cold", "lead", "reach out"]),
    ("proposal", "Proposal", "growth", "#10b981", &["proposal", "pitch", "deck", "quote"]),
    ("campaign", "Campaign", "growth", "#059669", &["campaign", "creators", "ugc", "brief"]),
    ("client", "Client Comms", "growth", "#2dd4bf", &["client", "email", "reply", "update", "report"]),
    ("decision", "Decision", "", "#c084fc", &["decide", "decision", "choose", "should we"]),
    ("idea", "Idea / Capture", "", "#94a3b8", &["idea", "capture", "maybe", "someday"]),
    ("followup", "Follow-up", "", "#fb7185", &["follow up", "follow-up", "waiting", "ping", "chase"]),
    ("admin", "Admin", "personal", "#a1a1aa", &["admin", "invoice", "payment", "logistics"]),
];

pub const PRIORITY: &[(i64, &str)] = &[
    (0, "No priority"),
    (1, "Urgent"),
    (2, "High"),
    (3, "Medium"),
    (4, "Low"),
];

/// harvest status -> workflow state name
pub fn status_to_state(status: &str) -> &'static str {
    match status {
        "backlog" => "Backlog",
        "in_progress" => "In Progress",
        "blocked" => "Blocked",
        "done" => "Done",
        _ => "Backlog",
    }
}

/// harvest status -> Linear project status
pub fn status_to_project(status: &str) -> &'static str {
    match status {
        "backlog" => "backlog",
        "in_progress" => "started",
        "blocked" => "paused",
        "done" => "completed",
        _ => "started",
    }
}

/// state name -> board column key (keeps the 4-column UI)
pub fn state_to_column(state: &str) -> &'static str {
    match state {
        "In Progress" | "In Review" => "in_progress",
        "Blocked" => "blocked",
        "Done" => "done",
        _ => "backlog",
    }
}

pub const OLLAMA_URL: &str = "http://localhost:11434";
pub const OLLAMA_MODEL: &str = "llama3.1:8b";
pub const SERVE_PORT: u16 = 7666;
