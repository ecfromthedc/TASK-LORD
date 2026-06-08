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

- [Rust](https://rustup.rs) (stable) — `cargo` to build
- macOS for the cook/launchpad features (Terminal + launchd); the harvester and
  board are cross-platform
- [Ollama](https://ollama.com) with a model pulled: `ollama pull llama3.1:8b`
- Optional: [`trello` CLI](https://www.npmjs.com/package/trello-cli) for the
  business swimlane

## Quick start

```bash
git clone https://github.com/<you>/TASK-LORD.git tasklord
cd tasklord
cargo build --release

./target/release/tasklord harvest   # scan sources, summarize, build the board
./target/release/tasklord serve     # open the live launchpad — click to cook
```

Don't want the LLM? `tasklord harvest --no-llm` uses fast heuristics instead.

## Model & provider

TASK LORD summarizes with one of two backends:

- **Local (Ollama)** — default, free, private. Model via `TASKLORD_MODEL`
  (default `deepseek-r1:8b`).
- **Hosted DeepSeek API** — higher quality. Auto-used when a key is present.
  Model via `TASKLORD_DEEPSEEK_MODEL` (`deepseek-chat` = V3, default; or
  `deepseek-reasoner` = R1).

Provide the key (never committed) one of two ways:

```bash
# env var
export DEEPSEEK_API_KEY="sk-..."

# or a key file (works headless / for the launchd job)
mkdir -p ~/.config/tasklord
printf %s "sk-..." > ~/.config/tasklord/deepseek.key && chmod 600 ~/.config/tasklord/deepseek.key
```

Force a backend with `TASKLORD_PROVIDER=deepseek` or `=ollama`.

> **macOS tip:** clone outside `~/Documents`, `~/Desktop`, `~/Downloads`.
> launchd (used for nightly refresh) can't touch those folders without Full
> Disk Access.

## Cook — click a card, land back in the work

Served by `tasklord serve` (loopback-only, per-run security token). Clicking a card:

- **session** → distills the prior session into a **context handoff doc**, then
  opens your terminal in the project and starts a **fresh** `claude` — clean
  context window, full continuity — with a directive on your clipboard to paste.
- **code** → fresh `claude` in the project, with the directive on your clipboard.
- **business** → opens the Trello board.

The directive tells the new agent to read the handoff, check TASK LORD, work the
issue's next step, then update the issue so the board stays truthful.

## Dismiss — ✕ a task you're done with

Hover a card and click **✕**. After an "are you sure?" confirm, the task is
removed and added to a `dismissals` table so it **never repopulates** on future
scans. Restore with `tasklord undismiss -- <id>`.

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

Configure your areas, scan roots, and task types by editing the seed tables in
`src/config.rs`.

## Layout

```
src/main.rs             CLI (clap): harvest · serve · dismiss · undismiss
src/config.rs           paths + seed taxonomy (areas, states, task types)
src/model.rs            shared data shapes
src/ollama.rs           local LLM client (reqwest)
src/store.rs            SQLite store (Linear data model) — source of truth
src/harvest.rs          orchestrator: sources → Ollama → SQLite → board.json
src/handoff.rs          session → context handoff doc (map-reduce for big ones)
src/serve.rs            axum launchpad: serves board + /cook + /dismiss
src/sources/            transcripts · code · trello scanners
board/index.html        Kanban UI (dark vaporwave) — cook + dismiss
run.sh                  nightly wrapper (ensures Ollama is up, logs)
install.sh              registers the launchd nightly agent
scheduler/*.template    launchd plist template (filled in by install.sh)
```

Built with **tokio · axum · rusqlite · reqwest · clap**.

See `GOAL.md` for the design north-star.

## License

MIT — see `LICENSE`.
