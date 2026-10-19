#!/usr/bin/env bash
# Add one project to a Curvyo/agent VM: bash project-setup.sh <project> <git-url> [lead]
# Example: bash project-setup.sh curvyo https://github.com/curvyo/curvyo.git
# Clones the repo to ~/<project> (the project directory is the clone), sets trust,
# the ProjectAtlas index (agent VMs) and starts claude-rc@<project>.service.
set -euo pipefail
P="$1"; URL="$2"; ROLE="${3:-agent}"
W="$HOME/$P"
[ -d "$W/base/.git" ] && { echo "old base/ layout in $W: move it away and re-run" >&2; exit 1; }
[ -d "$W/.git" ] || git clone "$URL" "$W"
cd "$W"
python3 - "$W" <<'PY'
import json, os, sys
w = sys.argv[1]
p = os.path.expanduser("~/.claude.json")
d = json.load(open(p)) if os.path.exists(p) else {}
d["remoteDialogSeen"] = True
proj = d.setdefault("projects", {})
for path in [w]:
    proj.setdefault(path, {})["hasTrustDialogAccepted"] = True
json.dump(d, open(p, "w"), indent=2)
PY
if [ "$ROLE" != "lead" ]; then
  [ -f frontend/package-lock.json ] && npm ci --prefix frontend
  if [ -f .projectatlas/config.toml ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
    rm -f .projectatlas/projectatlas.db .projectatlas/projectatlas.db-shm .projectatlas/projectatlas.db-wal
    (projectatlas init --force-rescan >/dev/null 2>&1; projectatlas root verify | grep -E "verified" || true)
    # Local scope: the MCP entry belongs to this project directory only.
    (cd "$W" && "$HOME/.local/bin/claude" mcp remove --scope local projectatlas >/dev/null 2>&1 || true
     cd "$W" && "$HOME/.local/bin/claude" mcp add --scope local projectatlas -- "$HOME/.cargo/bin/projectatlas" --db "$W/.projectatlas/projectatlas.db" --config "$W/.projectatlas/config.toml" mcp)
  fi
fi
systemctl --user daemon-reload
systemctl --user enable --now "claude-rc@$P.service"
echo "project $P ready ($ROLE)"
