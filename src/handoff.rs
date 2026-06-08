//! Context handoff generator: distill a session into a doc so a fresh session
//! can continue without inheriting a bloated, expensive context window.

use crate::config;
use crate::model::ExportCard;
use crate::ollama;
use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const SINGLE_PASS_CHARS: usize = 11000;
const CHUNK_CHARS: usize = 8000;
const MAX_CHUNKS: usize = 8;
const EDIT_TOOLS: &[&str] = &["Edit", "Write", "NotebookEdit", "MultiEdit"];

#[derive(Debug, Default, Deserialize)]
struct Fields {
    #[serde(default)]
    tldr: String,
    #[serde(default)]
    goal: String,
    #[serde(default)]
    done: Vec<String>,
    #[serde(default)]
    current_state: String,
    #[serde(default)]
    next_steps: Vec<String>,
    #[serde(default)]
    open_questions: Vec<String>,
    #[serde(default)]
    blockers: String,
    #[serde(default)]
    gotchas: Vec<String>,
}

const PROMPT: &str = r#"You are writing a CONTEXT HANDOFF so a brand-new agent can pick up this project
in a fresh session with zero prior memory. Be concrete: name files, commands,
decisions, exact next actions. No fluff.

Output STRICT JSON: {"tldr","goal","done":[],"current_state","next_steps":[],
"open_questions":[],"blockers","gotchas":[]}.

PROJECT: {label}
WORKING DIR: {path}
SESSION CONTENT:
{content}
"#;

const CHUNK_PROMPT: &str = r#"Extract durable facts from this slice of a work session — decisions, what was
built/changed, current state, next intentions, blockers, file paths/commands.
Terse bullet notes only.

SLICE:
{slice}
"#;

fn text_from_content(content: &Value) -> String {
    if let Some(s) = content.as_str() {
        return s.to_string();
    }
    if let Some(arr) = content.as_array() {
        return arr
            .iter()
            .filter_map(|b| {
                if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                    b.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    String::new()
}

fn read_session(path: &Path) -> (String, Vec<String>) {
    let Ok(file) = File::open(path) else { return (String::new(), Vec::new()) };
    let mut msgs = Vec::new();
    let mut files: Vec<String> = Vec::new();
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let Ok(rec) = serde_json::from_str::<Value>(&line) else { continue };
        let typ = rec.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if typ != "user" && typ != "assistant" {
            continue;
        }
        let Some(msg) = rec.get("message") else { continue };
        let Some(content) = msg.get("content") else { continue };
        if let Some(arr) = content.as_array() {
            for b in arr {
                if b.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                    && EDIT_TOOLS.contains(&b.get("name").and_then(|n| n.as_str()).unwrap_or(""))
                {
                    if let Some(fp) = b.get("input").and_then(|i| i.get("file_path")).and_then(|f| f.as_str()) {
                        if !files.iter().any(|x| x == fp) {
                            files.push(fp.to_string());
                        }
                    }
                }
            }
        }
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or(typ);
        let text = text_from_content(content);
        if !text.trim().is_empty() {
            msgs.push(format!("{}: {}", role.to_uppercase(), text.trim()));
        }
    }
    (msgs.join("\n\n"), files)
}

async fn distill(text: &str) -> String {
    if text.len() <= SINGLE_PASS_CHARS {
        return text.to_string();
    }
    let mut chunks: Vec<&str> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let end = (i + CHUNK_CHARS).min(bytes.len());
        // align to char boundary
        let mut e = end;
        while e < bytes.len() && !text.is_char_boundary(e) {
            e += 1;
        }
        chunks.push(&text[i..e]);
        i = e;
    }
    if chunks.len() > MAX_CHUNKS {
        let head = chunks[..MAX_CHUNKS / 2].to_vec();
        let tail = chunks[chunks.len() - (MAX_CHUNKS - head.len())..].to_vec();
        chunks = head.into_iter().chain(tail).collect();
    }
    let mut notes = Vec::new();
    for (i, ch) in chunks.iter().enumerate() {
        let n = ollama::generate_text(&CHUNK_PROMPT.replace("{slice}", ch)).await;
        if !n.is_empty() {
            notes.push(format!("[part {}]\n{}", i + 1, n));
        }
    }
    if notes.is_empty() {
        text[text.len().saturating_sub(SINGLE_PASS_CHARS)..].to_string()
    } else {
        notes.join("\n\n")
    }
}

fn bullets(items: &[String]) -> String {
    if items.is_empty() {
        "- (none)".into()
    } else {
        items.iter().filter(|x| !x.is_empty()).map(|x| format!("- {}", x)).collect::<Vec<_>>().join("\n")
    }
}

fn render(f: &Fields, label: &str, path: &str, files: &[String], session_id: &str) -> String {
    format!(
        "# Context Handoff — {label}\n\n\
> Generated by TASK LORD from session `{session_id}`. Fresh session, same working dir. Read this, then continue.\n\n\
**TL;DR:** {tldr}\n\n\
**Working dir:** `{path}`\n\n\
## Goal\n{goal}\n\n\
## Done so far\n{done}\n\n\
## Current state\n{state}\n\n\
## Next steps\n{next}\n\n\
## Open questions\n{oq}\n\n\
## Blockers\n{blockers}\n\n\
## Gotchas\n{gotchas}\n\n\
## Files touched this session\n{files}\n",
        label = label,
        session_id = session_id,
        tldr = if f.tldr.is_empty() { "—" } else { &f.tldr },
        path = path,
        goal = if f.goal.is_empty() { "—" } else { &f.goal },
        done = bullets(&f.done),
        state = if f.current_state.is_empty() { "—" } else { &f.current_state },
        next = bullets(&f.next_steps),
        oq = bullets(&f.open_questions),
        blockers = if f.blockers.is_empty() { "(none)" } else { &f.blockers },
        gotchas = bullets(&f.gotchas),
        files = bullets(&files.iter().take(40).cloned().collect::<Vec<_>>()),
    )
}

fn safe(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').chars().take(48).collect::<String>();
    if s.is_empty() { "project".into() } else { s }
}

/// Build (and cache) a handoff doc for a session card. Returns its path.
pub async fn build(group: &str, session_id: &str, card: &ExportCard) -> Option<PathBuf> {
    let transcript = config::projects_dir().join(group).join(format!("{}.jsonl", session_id));
    if !transcript.exists() {
        return None;
    }
    let dir = config::handoff_dir();
    let _ = fs::create_dir_all(&dir);
    let label = &card.label;
    let out = dir.join(format!("{}-{}.md", safe(label), &session_id[..session_id.len().min(8)]));

    // cache: reuse if newer than transcript
    if let (Ok(om), Ok(tm)) = (
        fs::metadata(&out).and_then(|m| m.modified()),
        fs::metadata(&transcript).and_then(|m| m.modified()),
    ) {
        if om >= tm {
            return Some(out);
        }
    }

    let (text, files) = read_session(&transcript);
    if text.is_empty() {
        return None;
    }
    let content = distill(&text).await;
    let content: String = content.chars().take(SINGLE_PASS_CHARS).collect();
    let path = card.path.clone().unwrap_or_else(|| "?".into());
    let prompt = PROMPT
        .replace("{label}", label)
        .replace("{path}", &path)
        .replace("{content}", &content);
    let fields: Fields = ollama::generate_json(&prompt).await.unwrap_or_else(|| Fields {
        tldr: card.left_off.clone(),
        current_state: card.left_off.clone(),
        next_steps: if card.next_step != "—" { vec![card.next_step.clone()] } else { vec![] },
        blockers: card.blockers.clone(),
        ..Default::default()
    });
    let md = render(&fields, label, &path, &files, session_id);
    fs::write(&out, md).ok()?;
    Some(out)
}
