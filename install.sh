#!/bin/bash
# Install the nightly TASK LORD harvest as a macOS launchd agent.
# Generates the plist from the template with this repo's absolute path.
#
# NOTE (macOS): launchd cannot read/write under ~/Documents, ~/Desktop, or
# ~/Downloads without Full Disk Access. Clone TASK LORD somewhere else
# (e.g. ~/Projects, ~/code) so the nightly job runs without extra permissions.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
TEMPLATE="$HERE/scheduler/com.tasklord.plist.template"
PLIST="$HERE/scheduler/com.tasklord.plist"
DEST="$HOME/Library/LaunchAgents/com.tasklord.plist"

mkdir -p "$HOME/Library/LaunchAgents" "$HERE/logs"
sed "s|__TASKLORD_DIR__|$HERE|g" "$TEMPLATE" > "$PLIST"
cp "$PLIST" "$DEST"
launchctl unload "$DEST" 2>/dev/null || true
launchctl load "$DEST"

echo "✓ Installed com.tasklord — nightly re-clock at 5:30 AM (+ once now)."
echo "  Engine: $HERE"
echo "  Uninstall: launchctl unload \"$DEST\" && rm \"$DEST\""
