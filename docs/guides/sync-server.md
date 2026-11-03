# Running curvyo-sync-server on a local VM

The relay of cooperative editing (ADR 0031 decision 11, `specs/0092-cooperative-editing`
criteria 35 to 39). It stores only sealed bytes and keyring entries; it never sees a document.
Slice 1 runs it on a VM in the maker's LAN over plain `ws://`; Kubernetes with a TLS ingress
comes later and needs no code change.

## What it needs

- One open TCP port (default 8787) and one writable data directory (default `/data`).
- Nothing else: no config file, no database, no other service.
- Exactly one server process per data directory (one replica).

Configuration, flag wins over environment:

| Flag | Environment | Default |
|---|---|---|
| `--listen <address:port>` | `CURVYO_SYNC_LISTEN` | `0.0.0.0:8787` |
| `--data <directory>` | `CURVYO_SYNC_DATA` | `/data` |

## Option A: the static binary

On the build machine (Linux, from the repository root):

```text
rustup target add x86_64-unknown-linux-musl
cargo build --release --locked -p curvyo-sync-server --target x86_64-unknown-linux-musl
scp target/x86_64-unknown-linux-musl/release/curvyo-sync-server vm:/usr/local/bin/
```

The binary is statically linked and runs on any x86-64 Linux VM. On the VM, as a systemd
service (`/etc/systemd/system/curvyo-sync-server.service`):

```text
[Unit]
Description=Curvyo sync relay
After=network-online.target

[Service]
ExecStart=/usr/local/bin/curvyo-sync-server --data /var/lib/curvyo-sync
DynamicUser=yes
StateDirectory=curvyo-sync
Restart=on-failure
KillSignal=SIGTERM
TimeoutStopSec=15

[Install]
WantedBy=multi-user.target
```

```text
sudo systemctl daemon-reload
sudo systemctl enable --now curvyo-sync-server
journalctl -u curvyo-sync-server -f
```

## Option B: the container image

Build by hand from the repository root (Docker or Podman):

```text
docker build -f curvyo-sync-server/Dockerfile -t curvyo-sync-server .
docker volume create curvyo-sync-data
docker run -d --name curvyo-sync --restart unless-stopped \
  -p 8787:8787 -v curvyo-sync-data:/data curvyo-sync-server
docker logs -f curvyo-sync
```

The image runs as the distroless `nonroot` user (uid 65532). A named volume is writable as is;
for a bind mount, give the host directory to that user first
(`sudo chown 65532:65532 /srv/curvyo-sync`). `docker stop` sends SIGTERM; the server closes every
socket with code 1012 and exits within 10 s, and the apps reconnect on their own.

## Check it

```text
curl -i http://<vm-address>:8787/healthz
```

`200` once the startup scan is done and the data directory is writable, `503` otherwise. Open
port 8787 in the VM's firewall (for example `sudo ufw allow 8787/tcp`).

In both Curvyo apps, set the server address to `ws://<vm-address>:8787`. The desktop app
connects over plain `ws://` on the LAN; the browser build needs `wss://` and so joins only once
the server sits behind a TLS-terminating ingress (the Kubernetes step).

## Data, backup and restart

Everything the server stores is in the data directory: one subdirectory per room (the room id in
hex) with `keyring.log`, `log` and `snapshot`. To back it up, stop the server and copy the
directory; starting the server on the same directory restores every room. The apps keep the
complete document in their own `.curvyo` files, so losing the server loses no document.

The log goes to stdout and holds lifecycle lines only (listening, startup scan, shutdown); no
client address and no message body.

## Later: Kubernetes

The same image runs unchanged: one replica (`replicas: 1`, `strategy: Recreate`), a
`PersistentVolumeClaim` mounted on `/data`, `securityContext.fsGroup: 65532`, readiness and
liveness probes `httpGet /healthz` on port 8787, and an ingress terminating TLS that forwards
WebSocket upgrades.
