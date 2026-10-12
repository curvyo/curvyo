# Worker setup

Scripts that prepare a Linux VM to run Claude sessions for this project: the lead VM,
which coordinates and does not build, and the agent VMs, which build and test. Tested on Arch Linux
with passwordless sudo and `gh` already logged in. Claude Code itself must be installed at
`~/.local/bin/claude`.

| File | What it does |
|---|---|
| `agent-vm-setup.sh` | Once per machine: system packages, git over `gh`, the pinned Rust toolchain, `wasm-bindgen-cli` and ProjectAtlas (agent VMs only), and the `claude-rc@.service` unit. |
| `project-setup.sh` | Once per project: creates `~/<project>` with the main checkout `base/`, the `CLAUDE.md` and `.claude` symlinks, workspace trust, `npm ci` and the ProjectAtlas index (agent VMs only), then starts `claude-rc@<project>.service`. |
| `claude-rc@.service` | systemd user unit: Claude Remote Control in `~/<project>`, session name `<host>-<project>`, restarted on every exit, memory capped at 13 GB. |

## Use

```sh
# agent VM
GIT_NAME="..." GIT_EMAIL="..." bash agent-vm-setup.sh
bash project-setup.sh curvyo https://github.com/curvyo/curvyo.git

# lead VM
bash agent-vm-setup.sh lead
bash project-setup.sh curvyo https://github.com/curvyo/curvyo.git lead
```

Both scripts can be run again. For the service to keep running after logout, enable lingering
once: `sudo loginctl enable-linger "$USER"`.
