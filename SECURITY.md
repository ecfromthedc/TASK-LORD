# Security Model — TASK LORD

TASK LORD's `serve` command runs a small local web server (`tasklord serve`)
that turns clicks on the board into **real actions on your machine**. Read this
before running it anywhere other than your own laptop.

## What the server actually does

When you click a card, the server can:

- **Cook** — write a temporary `.command` script and drive **AppleScript**
  (`osascript` → Terminal) to open a shell in the card's project path and launch
  `claude`. The project path comes straight from `board.json`.
- **Refresh** — spawn `run.sh` to re-clock the board.
- **Dismiss** — write to the local SQLite database and regenerate `board.json`.

In short: **the server executes shell / AppleScript derived from `board.json`
card contents.** Card paths are shell-quoted before use, but the trust boundary
is the file itself, not the quoting.

## The trust boundary — `board.json` is trusted input

`board.json` is a **trusted-input file**. It is generated locally by the
harvester from your own session transcripts, git repos, and (optionally) Trello.
The server treats its contents as authorized to be executed.

**Anyone who can write `board.json`, or feed malicious content into the sources
that generate it, can cause code to run on your machine.** Treat it with the
same care as a shell script you would run yourself.

## Why this is safe for its intended (local) use

Two controls keep this to a single-user, local-only tool:

1. **Loopback-only bind.** The listener is hardcoded to `127.0.0.1`
   (`src/serve.rs`). There is no configuration option to change the bind
   address, so the server cannot be exposed on a LAN or public interface by
   misconfiguration. Do **not** patch this to `0.0.0.0`.
2. **Per-run random token.** A fresh 128-bit random token is minted on every
   start. **Every** action endpoint — `/cook`, `/dismiss`, and `/refresh` —
   requires that exact token; requests without a valid token are rejected. The
   token is printed once (and injected into the page) at launch. Read-only
   endpoints (`/`, `/board.json`, `/board.js`) serve local board data only and
   perform no execution.

## Do NOT

- **Do not expose this server on a network or tunnel** (ngrok, Cloudflare
  Tunnel, SSH port-forward, `0.0.0.0` bind, reverse proxy, etc.) **without
  adding your own authentication and authorization** in front of it. The
  per-run token is a CSRF/local-guard, not a substitute for network auth.
- **Do not run it on a shared / multi-user host** where another user could read
  the token from the process table or the page, or write to `board.json`.
- **Do not point the harvester at untrusted sources** and then cook cards
  without reviewing them — a malicious project path or session artifact becomes
  executed shell.

## Reporting

This is a personal, local-first tool. If you find a way for a non-loopback or
unauthenticated request to trigger execution, open an issue (omit any working
exploit details from the public description).
