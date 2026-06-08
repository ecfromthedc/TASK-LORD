# TASK LORD — Design North-Star

## Mission

Clock **every project across your AI agents and coding sessions** and show where
each one left off — then make each card a launchpad back into the work. Kill the
"loose ends scattered across sessions" problem: treat your whole operation like a
software factory where every project is an agenda item with a known state and a
known next move.

## Principles

1. **Local-first.** SQLite is the source of truth. The board is a dumb static
   reader exported from the db. Nothing leaves your machine.
2. **The grind is local, never the cloud.** All "where did this leave off"
   summarization runs on a local LLM (Ollama). The orchestrator only routes.
3. **Status is real, not cosmetic.** Each issue lands in a workflow state based
   on the actual session content + git facts, with a concrete `left_off`,
   `next_step`, and `blockers`.
4. **Resilient by default.** Any source that fails (LLM down, Trello offline, a
   malformed transcript) degrades gracefully — the board still renders.
5. **Linear as the reference.** The schema follows Linear's proven model
   (areas/projects/issues/states/priorities/updates), adapted for one operator
   running many threads.
6. **A living taxonomy.** `task_types` is meant to be edited over time to match
   the real kinds of work you do.

## Definition of done

1. `harvest.py` reads all three sources, runs clean, writes `tasklord.db` and
   exports `board/board.json`.
2. Summarization runs on Ollama; `--no-llm` falls back to heuristics.
3. `serve.py` renders the board and `/cook` launches a terminal in the project,
   resuming the exact session (or a seeded fresh one) with a directive on the
   clipboard.
4. `install.sh` registers a launchd agent that re-clocks nightly.
5. No secrets or private data are ever committed or printed.

## Next horizons

- Sub-issues, milestones, and cycles populated by hand or by agents.
- Two-way sync: marking an issue Done archives the matching Trello card.
- A SessionEnd hook so a project re-clocks the moment a session closes (live).
- Per-area scan-root and task-type config in a single `config.toml`.
