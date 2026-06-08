//! Source: code/tool projects on disk. Git/plan facts keyed by absolute path.

use crate::config;
use crate::model::GitInfo;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

const PLAN_FILES: &[&str] = &["task_plan.md", "progress.md", "findings.md", "TODO.md", "TODO"];

#[derive(Debug, Clone)]
pub struct CodeFacts {
    pub path: String,
    pub label: String,
    pub is_git: bool,
    pub git: Option<GitInfo>,
    pub last_commit: Option<String>,
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn facts_for(dir: &Path) -> Option<CodeFacts> {
    let is_git = dir.join(".git").exists();
    let has_plan = PLAN_FILES.iter().any(|f| dir.join(f).exists());
    if !is_git && !has_plan {
        return None;
    }
    let label = dir.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
    let path = dir.to_string_lossy().to_string();
    let mut facts = CodeFacts {
        path,
        label,
        is_git,
        git: None,
        last_commit: None,
    };
    if is_git {
        let dirty = git(dir, &["status", "--porcelain"]).unwrap_or_default();
        facts.last_commit = git(dir, &["log", "-1", "--format=%cI"]);
        facts.git = Some(GitInfo {
            branch: git(dir, &["rev-parse", "--abbrev-ref", "HEAD"]),
            dirty: !dirty.is_empty(),
            dirty_count: dirty.lines().filter(|l| !l.is_empty()).count() as i64,
            last_commit_msg: git(dir, &["log", "-1", "--format=%s"]),
        });
    }
    Some(facts)
}

pub fn collect() -> BTreeMap<String, CodeFacts> {
    let mut out = BTreeMap::new();
    for root in config::scan_roots() {
        let Ok(entries) = std::fs::read_dir(&root) else { continue };
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            if p.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with('.')).unwrap_or(false) {
                continue;
            }
            if let Some(f) = facts_for(&p) {
                out.insert(f.path.clone(), f);
            }
        }
    }
    out
}
