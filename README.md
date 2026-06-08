# TASK LORD

A local Kanban command board that clocks **every project across all your AI
agents and coding sessions** and shows where each one left off — then lets you
click a card to drop straight back into the work. Loose ends, surfaced and
sorted. Your operation as a software factory.

Built for people running many parallel threads through Claude Code (or any tool
that leaves session transcripts): content creators, software builders, and
high-agency operators who lose track of half-finished work across sessions.

```
TASK LORD ───────────────────────────────────────────────────
 BACKLOG        IN PROGRESS        BLOCKED         DONE
 ───────        ───────────        ───────         ────
 [SO-2 build]   [GR-1 outreach] ◀  [CRM blocked]   [done]
                 left off:          waiting on…
                 "wired the API"
                 ▶ resume session
```

- **Dark vaporwave UI** — minimal, fast, single file.
- **Linear-modeled** — areas, projects, issues, states, priorities, task types,
  and an update log that tracks how each project moves over time.
- **Local-first** — SQLite source of truth, no SaaS, no account.
- **Click to cook** — resume the exact Claude session where you left off.

## How it works

Three sources → one board:

| Source     | Where it reads                          | What it gives |
|------------|-----------------------------------------|---------------|
| `session`  | `~/.claude/projects/*/*.jsonl`          | Every agent/session, summarized to *where it left off* |
| `code`     | `~/Projects`, configurable scan roots   | Git status, dirty trees, plan files |
| `business` | Trello boards → lists → cards (optional)| Workstreams, classified by list name |

The heavy lifting — reconstructing "where did this leave off" from session
tails — runs on a **local LLM via [Ollama](https://ollama.com)** (`llama3.1:8b`
by default), so nothing leaves your machine and it costs nothing to run.

## Requirements

- macOS (uses launchd for scheduling; the harvester/board are cross-platform)
- Python 3.10+
- [Ollama](https://ollama.com) with a model pulled: `ollama pull llama3.1:8b`
- Optional: [`trello` CLI](https://www.npmjs.com/package/trello-cli) for the
  business swimlane

No Python dependencies — standard library only.

## Quick start

```bash
git clone https://github.com/<you>/TASK-LORD.git tasklord
cd tasklord

python3 harvest.py        # scan sources, summarize, build the board (uses Ollama)
python3 serve.py          # open the live board — click a card to resume work
```

Don't want the LLM? `python3 harvest.py --no-llm` uses fast heuristics instead.

> **macOS tip:** clone outside `~/Documents`, `~/Desktop`, `~/Downloads`.
> launchd (used for nightly refresh) can't touch those folders without Full
> Disk Access.

## Cook — click a card, land back in the work

Served by `serve.py` (loopback-only, per-run security token). Clicking a card:

- **session** → opens your terminal in the project and runs
  `claude --resume <session-id>` — the *exact* conversation — and copies a
  directive prompt to your clipboard to paste.
- **code** → fresh `claude` in the project, with the directive on your clipboard.
- **business** → opens the Trello board.

The directive tells the resumed agent to check TASK LORD first, work the issue's
next step, then update the issue so the board stays truthful.

## Nightly auto-refresh (macOS)

```bash
./install.sh     # registers a launchd agent; re-clocks at 5:30 AM nightly
```

Uninstall: `launchctl unload ~/Library/LaunchAgents/com.tasklord.plist && rm ~/Library/LaunchAgents/com.tasklord.plist`

## Data model — Linear, adapted for a solo high-agency operator

State lives in **SQLite** (`tasklord.db`), modeled on Linear:

| Linear            | TASK LORD table | notes |
|-------------------|-----------------|-------|
| Teams             | `areas`         | your life/work buckets (Content, Software, …) |
| Initiatives       | `initiatives`   | big bets across projects |
| Projects          | `projects`      | status + health + target date |
| Milestones        | `milestones`    | checkpoints in a project |
| Cycles            | `cycles`        | weekly focus sprints |
| Workflow States   | `states`        | triage/backlog/unstarted/started/completed/canceled |
| Issues            | `issues`        | the atomic loose-end: priority, type, sub-issues |
| Labels            | `labels`        | tags |
| (issue types)     | `task_types`    | **the living vocabulary** — edit to match your work |
| Project Updates   | `updates`       | the "where it left off" log over time |

`task_types` is the taxonomy you grow over time — the kinds of work you actually
do (Script, Edit, Publish, Build, Bug, Outreach, Proposal, Decision…). The
harvester infers a type per issue from keyword hints; edit the rows to teach it.

Configure your areas, scan roots, and task types by editing the seed lists in
`store.py` and `sources_code.py`.

## Layout

```
harvest.py              orchestrator: sources → Ollama → SQLite → board.json
store.py                SQLite store (Linear data model) — source of truth
serve.py                launchpad server: serves the board + /cook click-to-launch
ollama_client.py        local LLM client (urllib, no deps)
sources_transcripts.py  session loose-ends from jsonl tails (+ session UUID)
sources_code.py         git/plan facts for code projects
sources_trello.py       business workstreams via the trello CLI (board rollup)
board/index.html        Kanban UI (dark vaporwave) with cook buttons
run.sh                  nightly wrapper (ensures Ollama is up, logs)
install.sh              registers the launchd nightly agent
scheduler/*.template    launchd plist template (filled in by install.sh)
```

See `GOAL.md` for the design north-star.

## License

MIT — see `LICENSE`.
