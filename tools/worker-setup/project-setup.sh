#!/usr/bin/env bash
# Add one project to a Curvyo/agent VM: bash project-setup.sh <project> <git-url> [lead]
# Example: bash project-setup.sh curvyo https://github.com/curvyo/curvyo.git
# Creates ~/<project> with the main checkout base/, the CLAUDE.md/.claude symlinks, trust,
# the ProjectAtlas index (agent VMs) and starts claude-rc@<project>.service.
set -euo pipefail
P="$1"; URL="$2"; ROLE="${3:-agent}"
W="$HOME/$P"
mkdir -p "$W"
[ -d "$W/base/.git" ] || git clone "$URL" "$W/base"
cd "$W"
[ -e base/CLAUDE.md ] && ln -sfn base/CLAUDE.md CLAUDE.md
[ -e base/.claude ] && ln -sfn base/.claude .claude
python3 - "$W" <<'PY'
import json, os, sys
w = sys.argv[1]
p = os.path.expanduser("~/.claude.json")
d = json.load(open(p)) if os.path.exists(p) else {}
d["remoteDialogSeen"] = True
proj = d.setdefault("projects", {})
for path in [w, w + "/base"]:
    proj.setdefault(path, {})["hasTrustDialogAccepted"] = True
json.dump(d, open(p, "w"), indent=2)
PY
if [ "$ROLE" != "lead" ]; then
  [ -f base/frontend/package-lock.json ] && npm ci --prefix base/frontend
  if [ -f base/.projectatlas/config.toml ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
    rm -f base/.projectatlas/projectatlas.db base/.projectatlas/projectatlas.db-shm base/.projectatlas/projectatlas.db-wal
    (cd base && projectatlas init --force-rescan >/dev/null 2>&1; projectatlas root verify | grep -E "verified" || true)
    # Local scope: the MCP entry belongs to this project directory only.
    (cd "$W" && "$HOME/.local/bin/claude" mcp remove --scope local projectatlas >/dev/null 2>&1 || true
     cd "$W" && "$HOME/.local/bin/claude" mcp add --scope local projectatlas -- "$HOME/.cargo/bin/projectatlas" --require-version 0.4.5 --db "$W/base/.projectatlas/projectatlas.db" --config "$W/base/.projectatlas/config.toml" mcp)
  fi
fi
systemctl --user daemon-reload
systemctl --user enable --now "claude-rc@$P.service"
echo "project $P ready ($ROLE)"
