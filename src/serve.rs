//! Launchpad server (axum) — serves the board and turns clicks into action:
//! cook (handoff + fresh session) and dismiss (drop a task for good).
//! Loopback only; every action requires a per-run token.

use crate::model::{Board, ExportCard};
use crate::{config, handoff, store};
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Json, Redirect},
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;

struct App {
    token: String,
    claude: String,
}

fn random_token() -> String {
    let n: u128 = rand::random();
    format!("{:x}", n)
}

fn resolve_claude() -> String {
    // resolve via a login shell so PATH matches the user's terminal
    if let Ok(out) = Command::new("bash").args(["-lc", "command -v claude"]).output() {
        let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !p.is_empty() {
            return p;
        }
    }
    "claude".into()
}

fn board() -> Option<Board> {
    let raw = std::fs::read_to_string(config::board_dir().join("board.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

fn card_by_id(id: &str) -> Option<ExportCard> {
    board()?.cards.into_iter().find(|c| c.id == id)
}

fn pbcopy(text: &str) {
    if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}

fn launch_terminal(shell_cmd: &str) {
    let tmp = std::env::temp_dir().join(format!("tasklord-cook-{:x}.command", rand::random::<u32>()));
    if std::fs::write(&tmp, format!("#!/bin/bash\n{}\n", shell_cmd)).is_err() {
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
    }
    let script = tmp.to_string_lossy().to_string();
    let osa = format!(
        "tell application \"Terminal\" to do script {}\ntell application \"Terminal\" to activate",
        serde_json::to_string(&script).unwrap_or_default()
    );
    let _ = Command::new("osascript").args(["-e", &osa]).spawn();
}

fn directive(card: &ExportCard, handoff_path: Option<&str>) -> String {
    let mut lines: Vec<String> = Vec::new();
    if let Some(h) = handoff_path {
        lines.push(format!("Read the context handoff first: {}", h));
        lines.push(
            "It distills a prior session on this project so you can continue in a fresh \
             context window. After reading it, pick up the work."
                .into(),
        );
        lines.push(String::new());
    }
    lines.push(format!(
        "Also check TASK LORD for state/history: the SQLite board at {}/tasklord.db \
         (tables: projects, issues, updates, task_types, dismissals).",
        config::engine_dir().display()
    ));
    lines.push(String::new());
    lines.push(format!("Project: {}", card.label));
    if let Some(i) = &card.identifier {
        lines.push(format!("Issue: {} — {}", i, card.title.clone().unwrap_or_default()));
    }
    if card.left_off != "—" {
        lines.push(format!("Where it left off: {}", card.left_off));
    }
    if !card.blockers.is_empty() {
        lines.push(format!("Blocker: {}", card.blockers));
    }
    if card.next_step != "—" {
        lines.push(format!("Your task (next step): {}", card.next_step));
    }
    lines.push(String::new());
    lines.push(
        "When you finish or change state, update the TASK LORD issue (status, left_off, \
         next_step) so the board stays truthful."
            .into(),
    );
    lines.join("\n")
}

async fn cook_card(app: &App, card: &ExportCard) -> serde_json::Value {
    let src = card.source.clone().unwrap_or_default();

    if src == "business" {
        let url = card.path.clone().unwrap_or_else(|| "https://trello.com".into());
        pbcopy(&directive(card, None));
        let _ = Command::new("open").arg(&url).spawn();
        return serde_json::json!({"ok": true, "action": "opened Trello", "target": url});
    }

    let Some(path) = card.path.clone() else {
        return serde_json::json!({"ok": false, "error": "no project path for this card"});
    };

    let mut handoff_path = None;
    if src == "session" {
        if let Some(sid) = &card.session_id {
            if let Some(p) = handoff::build(&card.id, sid, card).await {
                handoff_path = Some(p.to_string_lossy().to_string());
            }
        }
    }

    pbcopy(&directive(card, handoff_path.as_deref()));
    let cmd = format!(
        "cd {} || {{ echo 'path gone'; exit 1; }}\nexec {}",
        shell_quote(&path),
        shell_quote(&app.claude)
    );
    launch_terminal(&cmd);
    let action = if handoff_path.is_some() { "fresh session + handoff" } else { "fresh session" };
    serde_json::json!({"ok": true, "action": action, "path": path, "handoff": handoff_path})
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

// ---- handlers ---------------------------------------------------------------

async fn index(State(app): State<Arc<App>>) -> impl IntoResponse {
    let html = std::fs::read_to_string(config::board_dir().join("index.html"))
        .unwrap_or_else(|_| "<h1>board/index.html missing</h1>".into());
    let inject = format!("<script>window.TASKLORD_TOKEN={};</script>", serde_json::to_string(&app.token).unwrap());
    Html(html.replacen("</head>", &format!("{}\n</head>", inject), 1))
}

async fn board_json() -> impl IntoResponse {
    let body = std::fs::read_to_string(config::board_dir().join("board.json")).unwrap_or_else(|_| "{}".into());
    ([("content-type", "application/json")], body)
}

async fn board_js() -> impl IntoResponse {
    let body = std::fs::read_to_string(config::board_dir().join("board.js")).unwrap_or_default();
    ([("content-type", "application/javascript")], body)
}

async fn cook(State(app): State<Arc<App>>, Query(q): Query<HashMap<String, String>>) -> impl IntoResponse {
    if q.get("token").map(|t| t != &app.token).unwrap_or(true) {
        return Json(serde_json::json!({"ok": false, "error": "bad token"}));
    }
    let Some(card) = q.get("id").and_then(|id| card_by_id(id)) else {
        return Json(serde_json::json!({"ok": false, "error": "card not found"}));
    };
    Json(cook_card(&app, &card).await)
}

async fn dismiss(State(app): State<Arc<App>>, Query(q): Query<HashMap<String, String>>) -> impl IntoResponse {
    if q.get("token").map(|t| t != &app.token).unwrap_or(true) {
        return Json(serde_json::json!({"ok": false, "error": "bad token"}));
    }
    let Some(id) = q.get("id") else {
        return Json(serde_json::json!({"ok": false, "error": "no id"}));
    };
    let reason = q.get("reason").cloned().unwrap_or_default();
    match store::dismiss(id, &reason) {
        Ok(_) => {
            // regenerate board.json so the dismissed card is gone immediately
            if let Ok(b) = store::export_board() {
                let _ = std::fs::write(
                    config::board_dir().join("board.json"),
                    serde_json::to_string_pretty(&b).unwrap_or_default(),
                );
            }
            Json(serde_json::json!({"ok": true, "dismissed": id}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn refresh(State(app): State<Arc<App>>, Query(q): Query<HashMap<String, String>>) -> impl IntoResponse {
    if q.get("token").map(|t| t != &app.token).unwrap_or(true) {
        return Redirect::to("/");
    }
    let _ = Command::new("bash").arg(config::engine_dir().join("run.sh")).spawn();
    Redirect::to("/")
}

pub async fn run() -> anyhow::Result<()> {
    let app = Arc::new(App {
        token: random_token(),
        claude: resolve_claude(),
    });
    let url = format!("http://127.0.0.1:{}/?token={}", config::SERVE_PORT, app.token);

    let router = Router::new()
        .route("/", get(index))
        .route("/index.html", get(index))
        .route("/board.json", get(board_json))
        .route("/board.js", get(board_js))
        .route("/cook", get(cook))
        .route("/dismiss", get(dismiss))
        .route("/refresh", get(refresh))
        .with_state(app);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", config::SERVE_PORT)).await?;
    println!("TASK LORD launchpad: {}", url);
    println!("Click a card to cook, ✕ to dismiss. Ctrl+C to stop.");
    let _ = Command::new("open").arg(&url).spawn();
    axum::serve(listener, router).await?;
    Ok(())
}
