#!/usr/bin/env bash
# Machine setup for a Curvyo worker VM (Arch Linux, passwordless sudo, `gh` logged in).
# Idempotent: safe to run again. Run once per machine, then project-setup.sh once per project.
# Usage: GIT_NAME="..." GIT_EMAIL="..." bash agent-vm-setup.sh [lead]
#   "lead" skips the Rust build tools and sets the smaller memory cap (the lead VM does not build).
#   GIT_NAME and GIT_EMAIL are optional; without them the existing git identity is kept.
set -euo pipefail

ROLE="${1:-agent}"
HERE="$(cd "$(dirname "$0")" && pwd)"

# 1. System packages (cargo-deny for the gate, inetutils for `hostname`).
sudo pacman -Syu --needed --noconfirm cargo-deny inetutils

# 2. Git identity and git over the gh login.
[ -n "${GIT_NAME:-}" ] && git config --global user.name "$GIT_NAME"
[ -n "${GIT_EMAIL:-}" ] && git config --global user.email "$GIT_EMAIL"
gh auth setup-git

if [ "$ROLE" != "lead" ]; then
  # 3. The pinned Rust toolchain from rust-toolchain.toml (rustup would fetch it on first build).
  rustup toolchain install 1.99.0 --profile minimal --component rustfmt,clippy --target wasm32-unknown-unknown

  # 4. wasm-bindgen CLI: must match the wasm-bindgen crate in Cargo.lock exactly.
  command -v wasm-bindgen >/dev/null && wasm-bindgen --version | grep -q 0.2.129 \
    || cargo +1.99.0 install wasm-bindgen-cli --version 0.2.129 --locked

  # 5. ProjectAtlas (code navigation MCP server).
  command -v projectatlas >/dev/null && projectatlas --version | grep -q 0.4.5 \
    || cargo +1.99.0 install --locked --git https://github.com/styler-ai/ProjectAtlas --tag v0.4.5 projectatlas-cli
fi

# 6. The Remote Control service template, one instance per project (see project-setup.sh).
#    The lead VM (4 GB) gets a smaller memory cap than the agent VMs (16 GB).
mkdir -p "$HOME/.config/systemd/user"
cp "$HERE/claude-rc@.service" "$HOME/.config/systemd/user/claude-rc@.service"
if [ "$ROLE" = "lead" ]; then
  mkdir -p "$HOME/.config/systemd/user/claude-rc@.service.d"
  cat > "$HOME/.config/systemd/user/claude-rc@.service.d/memory.conf" <<EOF
[Service]
MemoryHigh=3G
MemoryMax=3500M
EOF
fi
systemctl --user daemon-reload
echo "machine setup done ($ROLE). Next: bash $HERE/project-setup.sh <project> <git-url> [$ROLE]"
