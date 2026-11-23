# ADR 0071: Connector management API, `curvyoctl` and maintenance mode

**Status:** Accepted (customer, 2026-11-18: Frage 243 CONN-SETUP-SECRET A, Frage 244
CONN-CONFIRM A, Frage 245 CONN-LAN-ROLES A, Frage 246 CONN-OWNER A; see "Customer answers" at the
end). Proposed 2026-11-18 (architect draft). `needs-customer` (answered): security model (an
unauthenticated setup window on the workshop network), ownership of a machine (who answers for
it), and changes to accepted behaviour (the press at the connector of spec `0064` criterion 9 and
ADR 0070 decision 2; the start rule for LAN clients).
Asked by the customer's answer to Frage 242 (CONN-LOCALSTART-SERVICE, 2026-11-18, translated):
"We build the connector from the API side, like e.g. talosctl. So it can only be set up via an
insecure mode, and nothing can be armed in it. I don't like ssh for this kind of thing. Then we
have one more small CLI tool, but we can couple the laser cutter, responsibility-wise, to a
Curvyo instance. If you lose access because you deleted the secrets, the connector has to be set
up again." Amends: ADR 0035 §1 (configuration file) and §4 (who issues a pairing
code), ADR 0070 (what starts an enrolment, what "the press at the machine" is, decision 9's
flags), addendum A5, A10 and A11 of `specs/0064-machine-connector/adrs.md`. Choices marked
(Proposal) are the architect's.

## Context

The connector normally runs as a systemd service next to the laser, and "it shouldn't have to be
operated". Today it is configured by flags (`--serial`, `--fake-controller`,
`--require-local-start`, ...), prints its pairing code on stdout at start or on the terminal line
`pair`, takes the local start as an empty line on stdin, and manages paired apps with
`--list-clients` and `--revoke` on its own host. Under systemd there is no terminal: stdin is
`/dev/null`, a code in the journal needs a shell on the host, and nobody can press Enter.

Talos Linux has the model the customer names: an unconfigured node boots into **maintenance
mode**, whose API is unauthenticated and accepts only a few read calls and `apply-config`
(`talosctl apply-config --insecure`); the applied configuration carries the PKI, after which every
call needs mutual TLS with the client certificate in the operator's `talosconfig`; `reset` wipes
the node back to maintenance; a lost `talosconfig` means reprovisioning. The node prints its
certificate fingerprint on the console so the operator can check it (`--cert-fingerprint`).
There is no shell and no ssh.

What exists and is reused: the TLS endpoint with the pinned key (ADR 0035 §3, A3, A4), SPAKE2
pairing with a code and bearer tokens (A5 to A7), the connector-core frames and codec (ADR 0035
§5, §6), GRBL over serial with safe stop on SIGTERM (PR #386, its addendum C11), and ADR 0070's
machine room, enrolment and start rule C (not built).

## Options

**Management API.**
- **A. More `curvyo-connector-core` messages on the existing WebSocket (chosen).** Same port,
  same TLS, same JSON framing, same gate in `-io`'s session; the same frames later travel inside
  ADR 0070's Noise channel unchanged, so the relay path needs nothing of its own. The browser and
  the app can send them (core builds for wasm32).
- **B. gRPC, as Talos.** Rejected: a second stack (`tonic`, `prost`, `h2`, an async runtime) and a
  second schema beside core's messages for a dozen calls; browsers need a grpc-web proxy.
- **C. A local admin socket or HTTP API on the host.** Rejected: it needs a shell on the host,
  which is what the customer rules out.

**Setup.**
- **A. Maintenance mode (chosen, the customer's choice).** Unconfigured means: one call that
  configures, nothing that moves.
- **B. A configuration file edited on the host.** Rejected by the customer (needs ssh).

**The CLI tool.**
- **A. A new binary crate `curvyo-connector-ctl`, binary `curvyoctl` (chosen).** It runs on the
  maker's computer (Linux, macOS, Windows) and links only the client side of `-io`.
- **B. A subcommand of `curvyo-connector-server`.** Rejected: the maker would install the
  connector program on a laptop to manage a Raspberry Pi; the server binary carries serial,
  signals and, with ADR 0070 decision 8b, `tokio`, `axum` and the relay; it is Linux first.
- **C. Only the desktop app.** Rejected as the only tool: the customer asks for a CLI, and setup
  must work before any app is paired. The app gains owner actions where the UI needs them
  (decision 7).

## Decision

1. **Two modes, chosen by one file.** `<state dir>/connector.toml` absent: **maintenance mode**.
   Present: **configured mode**. The run flags (`--serial`, `--baud`, `--fake-*`,
   `--require-local-start`, `--pair`) become **foreground mode**: a configuration held in memory
   for development, tests and demos, with today's terminal behaviour (code on stdout, Enter,
   `pair`, `unlock`); never written to disk, refused when `connector.toml` exists, never used by
   the unit file. Removing foreground mode once tests and demos use a configuration is a debt
   entry (Proposal).

2. **Maintenance mode.**
   - *On start:* the identity key is made if missing (A10, so the fingerprint exists before any
     setup) and the listener opens on `0.0.0.0:7447`. The serial port is **not opened** (no GRBL
     reset, no status poll), no relay link, no pairing code. The journal (stdout) says:
     `Maintenance mode until 14:32 UTC: waiting for curvyoctl apply-config --insecure. Key
     fingerprint: XXXX-…`; the fingerprint line is printed in every mode at every start.
   - *Reachable, without authentication:* `Hello`/`Welcome` (name = host name, software, version,
     protocol, `mode`, see 4), `ListPorts` (the serial device paths under `/dev/serial/by-id/`
     and `/dev/tty{USB,ACM}*`, names only, so the port can be chosen without a shell), and
     `ApplyConfig`. Nothing else.
   - *Impossible:* every job and control message (`SubmitJob`, `JobChunk`, `Start`, `Pause`,
     `Resume`, `Stop`, `Unlock`, later Frame, Home, jog), `Pair`, `PairSpake`, `Auth`, every
     owner call; all answer `Error { Maintenance }`. No byte goes to a serial port.
   - *Ends* with the first valid `ApplyConfig`: the connector writes `connector.toml` (temp file
     and rename), adds the sender to `clients.json` as **owner** with a fresh token, answers
     `ConfigApplied { token }`, closes every maintenance connection and switches in process to
     configured mode (controller opened per the configuration). An invalid configuration answers
     `Error { InvalidConfig }` naming the field and leaves the window as it was.
   - *Time box:* `ApplyConfig` is accepted for **30 minutes after the process starts**
     (Frage 243 A). After that it answers `Error { MaintenanceClosed }` while `Hello` and `ListPorts`
     still work; the window reopens only with a new start of the process (power cycle, or
     `systemctl restart` on the host).
   - *Source addresses:* maintenance connections are accepted only from loopback, private
     (RFC 1918, IPv6 ULA), link-local and shared (100.64.0.0/10, e.g. a tailnet) peer addresses;
     any other peer is closed after accept. A port forward from the internet therefore cannot
     claim the machine.
   - *The risk, explicitly:* during the window, **anyone on the workshop network who connects
     first becomes the owner** and can later start the laser. Talos accepts the same. Bounds:
     only an unconfigured connector, only for 30 minutes after a start, only from local
     addresses, and the maker notices because their own `apply-config` then fails with "already
     configured" (the journal names the claiming client and address). A man in the middle at
     setup is defeated only by checking the fingerprint: `curvyoctl apply-config --insecure
     --fingerprint XXXX-…` refuses a different key; without it the first key seen is pinned
     (trust on first use, as ADR 0035 §4's pairing). A one-time setup code would close the
     window to bystanders but must be read on the host's console or journal; the customer chose
     no secret (Frage 243 A) knowing this risk.

3. **Configured mode: owners and members.** `clients.json` (format 2) gives each paired client a
   `role`, `owner` or `member`; format 1 entries read as `member`. Owner calls (decision 4)
   from a member answer `Error { OwnerRequired }`. Owners (Frage 246 A) are: the client that applied the first
   configuration, apps paired with an owner code (decision 6), and, after enrolment through the
   relay, the admins of the machine room (decision 5). Members are other paired LAN apps and the
   machine room's members. Losing one owner credential needs no reset while another owner remains.

4. **Messages** (core, protocol 0 while unreleased, additive; golden file each):

   | Request | Allowed in | Answer |
   |---|---|---|
   | `Hello` | every mode | `Welcome` + `mode: Maintenance { closes_in_s } \| Configured { owner }` |
   | `ListPorts` | maintenance, owner | `Ports { paths }` |
   | `ApplyConfig { config, client_name }` | maintenance, owner | `ConfigApplied { token: Option }` (token only from maintenance) |
   | `GetConfig` | owner | `Config { config }` |
   | `IssueCode { client_name, role }` | owner | `CodeIssued { code, expires_in_s }` |
   | `ListClients` / `RevokeClient { name }` | owner | `Clients { name, role, paired_at }` / `ClientRevoked` |
   | `GetLog { log: Jobs \| Admin, last }` | owner | `LogLines { lines }` |
   | `Reset { connector_name }` | owner | `ResetDone`, then wipe and exit (decision 8) |

   The owner's confirmation of a start is `Start { job_id }` sent by an owner while the job is
   `AwaitingLocalStart` (decision 7): no new message. `Status` in that state gains
   `requested_by: Option<String>` (the client's name). New `ErrorCode`s: `Maintenance`,
   `MaintenanceClosed`, `InvalidConfig`, `OwnerRequired`. An older connector answers a newer call
   with `UnknownType` (ADR 0035 §6), which `curvyoctl` reports as "connector too old". The first
   release that ships `curvyoctl` and the connector apart from the app freezes this as protocol 1
   (Proposal).

5. **Configuration and the owner's responsibility.** `ConnectorConfig` is a serde struct in
   `curvyo-connector-core` (module `config`, pure validation), JSON on the wire and TOML on disk
   (the workspace's `toml` in `-io`), `format = 1`, golden files for both spellings:
   `name` (shown in `Welcome`, replaces the host name), `owner` (a label such as "Marc's
   workshop": whose machine this is, shown to every client), `listen` (default `0.0.0.0:7447`),
   `[controller] kind = "grbl-serial" | "fake"` with `port`, `baud` or the fake's pace and alarm
   line. No start option (decision 7) and no relay fields until the relay slice adds them.
   *Responsibility* means: the owner is whoever holds an owner credential; the connector records
   **who** did everything. `jobs.log` lines gain the client that sent the job and the one that
   started or confirmed it; a new `admin.log` (`0600`, append only, golden) records configured
   (client, address), configuration changed, code issued (name, role, by), paired, revoked,
   unlocked, start confirmed (job, requester, owner) and reset. Both stay on the host (R-SYS-007)
   and are read by an owner with `curvyoctl jobs` and `curvyoctl audit`. *Coupling to a Curvyo
   instance* (the maker's device group on one relay, ADR 0040) is ADR 0070's enrolment with one
   change: it is started by an owner (`curvyoctl enrol <connector> --relay <url>`) instead of at
   first start, and the code and check words appear in `curvyoctl`, not on the connector's
   stdout; CPace, the label `curvyo-enrol-machine-v1`, "Add machine" in the app, the machine
   room and decision 8c (one relay per maker) are unchanged. A connector belongs to at most one
   instance; a second `enrol` is refused until `reset`. After enrolment, owners = owner tokens ∪
   the machine room's admins (the maker's devices); the management frames travel through the
   Noise channel like every other frame.

6. **LAN pairing, kept and re-triggered.** SPAKE2, the pin and the token stay byte for byte (A4
   to A8; the app changes nothing to pair). In configured mode a code exists only when an owner
   asks: `curvyoctl pair <connector> --name Laptop [--owner]` sends `IssueCode`, prints the code
   the connector returns, and the app types it as today; the code never reaches stdout or a log
   (`admin.log` records that one was issued). Ten minutes, one use, three wrong tries, as A5.

7. **Start confirmation without a terminal.** A start (ADR 0070 decision 2's list: `Start`,
   `Resume`, Frame and `0118`'s motion) from a **member** goes to `AwaitingLocalStart` and runs
   when an **owner** sends `Start` for that job: `curvyoctl confirm-start <connector>` or the
   Start button of an owner's app, on the LAN or, later, through the relay. An owner's own start
   runs at once. Foreground mode keeps Enter and `--require-local-start` as built. No
   configuration field asks for a physical press until a physical source exists (a GPIO button,
   Frage 244 B, not chosen; later if asked). **Consequence, plainly: physical presence at the laser is no longer implied by
   any confirmation.** An owner can confirm a stranger's job from another city; safety then rests
   on the owner's judgement, the accountability in `admin.log`, and the machine's own lid
   switch and emergency stop (Frage 244 A, chosen knowingly). The same rule on the LAN: a LAN app
   paired as `member` waits like a relay member (Frage 245 A); this replaces ADR 0070's "LAN clients keep the flag-only rule", whose
   reason (the code was read at the connector's own terminal) no longer holds.

8. **Reset and recovery.** `curvyoctl reset <connector> --confirm <name>` (owner only, the name
   typed as a check): a running job is stopped as on SIGTERM (C11), the whole state directory is
   deleted (configuration, identity key, clients, logs, relay files: a factory reset, so a sold
   machine keeps nothing of the old owner's), and the process exits 0. The unit's
   `Restart=on-failure` leaves it stopped: **the maintenance window opens only at the next start
   at the host** (power cycle or `systemctl start`), never on a network command. Lost every owner
   credential: physical access only, by design: `curvyo-connector-server --reset --state-dir
   <dir>` on the host (refused while a running connector holds `<state dir>/lock`, an exclusive
   lock by `std::fs::File::try_lock`), deleting the directory, or rewriting the SD card. That
   needs a keyboard and screen or a shell once; there is no reset over the network without a
   credential. A reset machine's room on the relay remains until the maker removes it in My
   devices (relay slice). Rejected: a reset window after every boot (whoever can cut the power
   could take the machine), a trigger file on the boot partition (distribution-specific; later
   if asked).

9. **systemd and the state directory.** `curvyo-connector-server/dist/curvyo-connector.service`
   (Proposal, verified in the slice on Debian and Arch): `ExecStart=/usr/bin/curvyo-connector-server
   --state-dir /var/lib/curvyo-connector`, `DynamicUser=yes`, `StateDirectory=curvyo-connector`
   (`0700`), `SupplementaryGroups=dialout` (Arch: a drop-in with `uucp`, in the guide),
   `Restart=on-failure`, `KillSignal=SIGTERM`, `TimeoutStopSec=15` (above G12's 5 s stop),
   `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`, `DevicePolicy=closed`
   with `DeviceAllow=char-ttyUSB rw` and `char-ttyACM rw`. State directory: `connector.toml`,
   `identity.pem`, `clients.json` (format 2), `jobs.log`, `admin.log`, `lock`, later `relay/`
   (ADR 0070). The command line keeps `--state-dir`, `--listen` (overridden by the file's
   `listen`), the host-side `--list-clients`, `--revoke`, `--fingerprint` (print and exit) and
   `--reset`, and foreground mode. `docs/guides/connector-service-linux.md` (new) covers
   install, unit, groups and `curvyoctl`, beside the serial guide.

10. **`curvyoctl`.** Crate `curvyo-connector-ctl` (binary, `anyhow`, AGPL-3.0-or-later with its
    `deny.toml` exception), edges `-core`, `-io` (the client side: TLS with `PinVerifier`, SPAKE2
    is not needed), `curvyo-storage-io` (the ADR 0007 credential store, keychain or passphrase
    file; heavier than the CLI needs, measured in the slice), `toml`, `dirs`; hand-parsed
    arguments like the server. Local state: `<config dir>/curvyo/curvyoctl.toml` lists connectors
    by name (address, fingerprint, connector name); each token lives in the credential store as
    `curvyoctl/<name>`, never in that file (ADR 0007 §1). Commands: `identify --insecure <addr>`,
    `apply-config [--insecure <addr> [--fingerprint F]] <connector> -f <file>`, `get-config`,
    `status`, `jobs`, `audit`, `confirm-start [job]`, `stop`, `unlock`, `pair --name N
    [--owner]`, `clients`, `revoke <name>`, `reset --confirm <name>`, `enrol --relay <url>`
    (relay slice), `version`. Every command prints plain lines; exit codes 0, 1, 2 as the server.
    The suffix `-ctl` is new (`CLAUDE.md` §6 lists `-core`, `-app`, `-io`, `-wasm`, `-server`):
    a command-line client, neither the Tauri host (`-app`) nor a program next to the machine
    (`-server`).

11. **Crates.** `curvyo-connector-core`: `config`, the messages and error codes of decision 4,
    `Status.requested_by`, the start rule taking the peer's role (ADR 0070 decision 2's "local
    start required" input, now "peer is member"). `curvyo-connector-io`: session gate by mode and
    role, `maintenance` (window clock passed in, peer address check), `admin` (owner calls),
    `config_file` (TOML), `clients` format 2, `admin_log`, `ports`. `curvyo-connector-server`:
    mode choice, lock, `--reset`, `--fingerprint`, the unit file. `curvyo-app` and the frontend:
    the owner label, "Waiting for <owner> to confirm" and Start for owners on the Connector row.
    No new external dependency (`toml`, `dirs` and the credential store are locked).

## Consequences

- The connector is set up and run without a shell: `curvyoctl` against a fresh service, then
  apps paired by owner codes. A shell is needed only to recover from lost credentials.
- **For 30 minutes after each start of an unconfigured connector, the first host on the workshop
  network that connects and applies a configuration becomes its owner** (decision 2). The
  customer chose this with Frage 243 A, with the risk described; the bounds are the time box, local
  addresses only and the offered fingerprint check.
- **Nobody needs to stand at the laser to start a job any more.** A member's job waits for an
  owner's confirmation, which can come from anywhere. Spec `0064` criterion 9's "button or key on
  the connector's host" holds only in foreground mode until a physical button exists.
- LAN apps paired as members wait for confirmation like relay members (Frage 245 A): the maker's
  own laptop should be paired with `--owner`. Apps paired in slice 2 read as members.
- Ownership is a credential, not an account: no email, no server-side identity (ADR 0004 §10).
  Lose every owner credential and the machine is set up again, as the customer said.
- One more crate and one more binary to release; the lead amends `CLAUDE.md` §6's suffix list.
- ADR 0070 decision 9's flags `--relay`, `--enrol` and `--host-relay` become configuration fields
  and `curvyoctl enrol` in the relay slice.

## Slice order (updates `specs/0064-machine-connector/adrs.md`)

- Slice 4, GRBL over serial (PR #386): unchanged.
- **Slice 5 (new), connector API, `curvyoctl` and maintenance mode:** decisions 1 to 4 and 6 to
  11; decision 5 without enrolment. Demo: install the unit on a Linux box, `curvyoctl identify`,
  `apply-config --insecure` with the fake controller (and with a real GRBL port), `pair --owner`
  and `pair` for a second app, send from the member app, `confirm-start`, `jobs`, `audit`,
  `reset`. Needs: PR #386 merged (controller fields, SIGTERM stop), a spec from the product owner
  (next free number), UX notes (Connector row; `curvyoctl` output text). Not 0096 or 0097.
- Slice 6, enrol and reach through the relay (was 5): as ADR 0070, started by `curvyoctl enrol`.
- Slice 7, share with a workspace and remote start rule C (was 6): the press at the machine is
  an owner's confirmation (decision 7).
- Later: a GPIO start button (only if asked; Frage 244 B was not chosen), EST, the web app served on the LAN, the direct
  channel of ADR 0070 decision 8c, removing foreground mode.

## Concepts this touches

- ADR 0035 §1 (config file), §3 (pinned identity kept), §4 (code issued by an owner), §6
  (messages, protocol 1 at first release), §7 (local start), §9 (logs stay local).
- ADR 0070 decisions 1 (enrolment trigger), 2 (local press = owner confirmation; LAN rule), 5
  (remote stop still needs a working path), 8 (built-in relay as config), 9 (flags).
- ADR 0040 decision 4 (pairing slot opened on an owner's request; the opener's confirm happens in
  `curvyoctl`), ADR 0007 §1 (ctl tokens in the credential store), ADR 0004 §10 (no accounts).
- ADR 0011 and `CLAUDE.md` §6 (new crate, new suffix), `deny.toml` (AGPL exception).
- Specs `0064` criteria 1, 2, 9; `0125` (Send unchanged; a member's start waits); PR #386's
  serial work (C10 groups in the unit, C11 SIGTERM); `0118` (motion from a member waits).
- `docs/technical-debt.md` (foreground mode; the open window), R-SYS-007 (no telemetry).

## Customer answers (2026-11-18)

- Frage 243 CONN-SETUP-SECRET: **A**, no secret, as Talos: 30 minutes after each start, local
  network addresses only, fingerprint check offered. Chosen with the risk of decision 2 described:
  during the window the first connecting host on the LAN becomes owner.
- Frage 244 CONN-CONFIRM: **A**, an owner confirms a member's start from anywhere; nobody has to
  stand at the laser.
- Frage 245 CONN-LAN-ROLES: **A**, one start rule on the LAN and through the relay: apps paired as
  owner start at once, others wait for an owner.
- Frage 246 CONN-OWNER: **A**, owners are the `curvyoctl` that set the connector up, apps paired as
  owner, and after enrolment the maker's Curvyo devices.
- Earlier answers stand (Frage 242): a `talosctl`-like API, no ssh, lost secrets mean setting the
  connector up again.

## Questions for the customer (as asked)

Frage 243 (CONN-SETUP-SECRET): How open is the setup window of a fresh connector?
- A: No secret, as Talos: 30 minutes after each start, local network addresses only; checking the
  key fingerprint is offered, not required.
- B: As A, plus a one-time setup code that the connector shows on its screen or in its journal;
  without it, no setup.
- C: The window opens only after a physical act at the host (a button or a file on the SD card).
- Recommendation: A. Default if unanswered: A.

Frage 244 (CONN-CONFIRM): What replaces "press Enter at the connector" when it runs as a service?
- A: A start by someone who is not an owner waits until an owner confirms it, with `curvyoctl`
  or the app, from anywhere. Nobody has to be at the laser.
- B: As A, plus a physical start button (GPIO) built now, which can be required for every start.
- C: Only owners can start; others can only send.
- Recommendation: A. Default if unanswered: A.

Frage 245 (CONN-LAN-ROLES): Does the start rule apply to apps on the local network too?
- A: Yes, one rule everywhere: apps you pair as owner start at once, others wait for an owner.
- B: No, every app paired on the local network starts like an owner, as today.
- Recommendation: A. Default if unanswered: A.

Frage 246 (CONN-OWNER): Who is the owner of a connector?
- A: The `curvyoctl` that set it up, apps you pair as owner, and after enrolment your Curvyo
  devices.
- B: Only your Curvyo devices; `curvyoctl` becomes one of your devices (and so holds your
  workspace keys).
- C: Only the `curvyoctl` that set it up; apps never manage or confirm.
- Recommendation: A. Default if unanswered: A.

## Amendment 2026-11-23: the job list (spec `0132`, slice 1)

The management messages gain the job list on the same socket, protocol 0, additive: `MoveJob`,
`RemoveJob`, `SetRunNext` and the pushed `JobList` (before a connection's first `Status`);
`JobAccepted` carries `position` and `RejectReason::ListFull` exists. The access table (decision
3) gains the list rows: a member adds jobs, removes and flags its own, never reorders; an owner
does all; in foreground mode every paired client acts as an owner. `jobs/` joins the state
directory (decision 9): `<job_id>.gcode` and `list.json`, written by one thread outside the hub's
lock, deleted by Reset. `admin.log` gains `job-removed`, `run-next-set`, `job-dropped` and
`job-list-dropped`. No new crate, no new dependency. Details: `specs/0132-connector-job-queue/adrs.md`.
