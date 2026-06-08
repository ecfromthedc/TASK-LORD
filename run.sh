#!/bin/bash
# TASK LORD nightly re-clock. Runs the harvester, logs the result, opens nothing.
# Wired to launchd (see scheduler/) and safe to run by hand any time.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LOG="$HERE/logs/harvest-$(date +%Y%m%d).log"
mkdir -p "$HERE/logs"

# Ensure Ollama is up (launchd has no GUI session PATH; use full path).
export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"
if ! curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
  echo "[$(date)] starting ollama serve" >> "$LOG"
  nohup ollama serve >/dev/null 2>&1 &
  sleep 5
fi

echo "[$(date)] harvest start" >> "$LOG"
cd "$HERE"
python3 harvest.py >> "$LOG" 2>&1
echo "[$(date)] harvest done" >> "$LOG"
