# The connector as a service on Linux

`curvyo-connector-server` runs next to the laser as a systemd service and is set up from your own
computer with `curvyoctl` (`specs/0128-connector-api-and-ctl`, ADR 0071). No shell on the
connector's host is needed after the install. The serial port itself is covered by
[connector-serial-linux.md](connector-serial-linux.md).

## Install

1. Copy the binary to `/usr/bin/curvyo-connector-server` and the unit
   `curvyo-connector-server/dist/curvyo-connector.service` to
   `/etc/systemd/system/curvyo-connector.service`.
2. `systemctl daemon-reload && systemctl enable --now curvyo-connector`.
3. `journalctl -u curvyo-connector` shows "Maintenance mode until <HH:MM> UTC" and the key
   fingerprint, eight groups of four.

The unit runs under `DynamicUser=yes`; its state lives in `/var/lib/curvyo-connector`
(`StateDirectoryMode=0700`): `connector.toml`, `identity.pem`, `clients.json`, `jobs.log`,
`admin.log` and `lock`. A value set in `connector.toml` wins over a flag; there is no other
configuration file.

## Groups

The unit adds the group `dialout` (Debian, Ubuntu, Raspberry Pi OS, Fedora). Arch has no
`dialout`; the unit then fails with "Failed to resolve group". A drop-in fixes it:

```ini
# systemctl edit curvyo-connector
[Service]
SupplementaryGroups=
SupplementaryGroups=uucp
```

A Raspberry Pi's own UART (`/dev/ttyAMA0`) is not allowed by the unit; add
`DeviceAllow=char-ttyAMA rw` in the same drop-in.

## Set up with curvyoctl

From your computer on the same local network:

```text
curvyoctl identify --insecure 192.168.1.20
curvyoctl ports --insecure 192.168.1.20
curvyoctl apply-config --insecure 192.168.1.20 --fingerprint 3F2A-9C10-77B4-E2D1-0A5C-61F8-9B3E-4D27 -f connector.toml
curvyoctl pair --name Laptop --owner
```

`connector.toml`:

```toml
format = 1
name = "workshop-laser"
owner = "Marc's workshop"
[controller]
kind = "grbl-serial"
port = "/dev/serial/by-id/usb-1a86_USB_Serial-if00-port0"
baud = 115200
```

The setup window is open for 30 minutes after each start of an unconfigured connector, to local
addresses only, and nothing moves in it. Set the connector up before it is reachable through a
proxy or a tunnel: a relay makes every peer look local. A Pi without a real-time clock may print a
wrong "until HH:MM" before its clock is synchronised; the 30 minutes themselves are counted from
the start.

A second computer becomes an owner with `curvyoctl login 192.168.1.20 --code <code>`, the code
from `curvyoctl pair --name <any name> --owner` on the first one; the second computer is listed as
`curvyoctl@<its host name>`.

## The job list

While a job runs or waits for its start, the connector takes further jobs into a list of at most
20 jobs and 256 MiB together; owners reorder it and press Start, members add and remove their own
jobs. The list is kept in `<state dir>/jobs/` and comes back after a restart: a job that ran shows
"Interrupted at line <n>" and is never restarted by itself. A job is on disk a moment after the
connector answered that it holds it, so a crash in that moment loses it. The waiting texts also
stay in RAM (about 320 MiB at most); a Pi needs 512 MB.

## Reset

`curvyoctl reset --confirm workshop-laser` stops a running job, deletes the state and ends the
service; it stays stopped until the next boot or `systemctl start`. On the host,
`curvyo-connector-server --reset --state-dir /var/lib/curvyo-connector` does the same while the
service is stopped. If every owner credential is lost on every computer, the only way back is that
host-side reset.
