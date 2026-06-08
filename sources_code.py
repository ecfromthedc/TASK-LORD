"""Source: code/tool projects on disk (~/Projects/active, ~/Documents/Development).

Provides git-based 'cold facts' (last commit, dirty working tree, plan files)
keyed by absolute path. The harvester merges these onto session records that
share the same cwd, and surfaces real projects that have no recent session.
"""
from __future__ import annotations

import subprocess
from pathlib import Path

SCAN_ROOTS = [
    Path.home() / "Projects" / "active",
    Path.home() / "Documents" / "Development",
]

PLAN_FILES = ("task_plan.md", "progress.md", "findings.md", "TODO.md", "TODO")


def _git(cwd: Path, *args: str) -> str | None:
    try:
        res = subprocess.run(
            ["git", *args],
            cwd=str(cwd),
            capture_output=True,
            text=True,
            timeout=15,
        )
        if res.returncode != 0:
            return None
        return res.stdout.strip()
    except (subprocess.SubprocessError, OSError):
        return None


def _project_facts(d: Path) -> dict | None:
    is_git = (d / ".git").exists()
    plan = [f for f in PLAN_FILES if (d / f).exists()]
    # Qualify as a real project: a git repo, OR has a plan/progress file.
    if not is_git and not plan:
        return None
    facts = {"path": str(d), "label": d.name, "is_git": is_git, "plan_files": plan}
    if is_git:
        facts["last_commit"] = _git(d, "log", "-1", "--format=%cI")
        facts["last_commit_msg"] = _git(d, "log", "-1", "--format=%s")
        dirty = _git(d, "status", "--porcelain")
        facts["dirty"] = bool(dirty)
        facts["dirty_count"] = len(dirty.splitlines()) if dirty else 0
        facts["branch"] = _git(d, "rev-parse", "--abbrev-ref", "HEAD")
    return facts


def collect() -> dict[str, dict]:
    """Return {abspath: facts} for every qualifying code project."""
    out: dict[str, dict] = {}
    for root in SCAN_ROOTS:
        if not root.exists():
            continue
        for child in root.iterdir():
            if not child.is_dir() or child.name.startswith("."):
                continue
            facts = _project_facts(child)
            if facts:
                out[facts["path"]] = facts
    return out
