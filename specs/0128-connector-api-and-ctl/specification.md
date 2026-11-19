# Connector API, `curvyoctl` and maintenance mode: set up the machine without a shell

Status: Ready (UX notes and architect addendum D1 to D17 done 2026-11-18; Fragen 247 to 249 are applied by default and vetoable at the demo; Fragen 250 to 252 are decided by the customer, 2026-11-18)
Priority: Should
Origin: Customer (Frage 242 CONN-LOCALSTART-SERVICE, answered 2026-11-18, translated: "We build the connector from the API side, like e.g. talosctl. So it can only be set up via an insecure mode, and nothing can be armed in it. I don't like ssh for this kind of thing. Then we have one more small CLI tool, but we can couple the laser cutter, responsibility-wise, to a Curvyo instance. If you lose access because you deleted the secrets, the connector has to be set up again."). Decided by the customer 2026-11-18 as Fragen 243 to 246, all A (ADR 0071, Accepted). This is slice 5 of `0064` (order in `specs/0064-machine-connector/adrs.md` and "Slice order" of ADR 0071). Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker whose laser sits behind a Raspberry Pi or a workshop PC that runs the connector as a service, I want to set that connector up, pair my apps, confirm other people's starts and reset it from my own computer with one small command (`curvyoctl`), without ssh and without anyone at the machine's terminal, so that the connector can be installed once and never operated (`R-MFG-LASER-002`, `R-MFG-011`).

**What other tools do.** `talosctl` (Talos Linux): an unconfigured node boots into maintenance mode whose API is open and accepts only a few calls and `apply-config --insecure`; afterwards every call needs the client certificate in the operator's config; `reset` wipes the node back to maintenance; a lost operator config means reprovisioning. OctoPrint and Mainsail set up through a web page on the device with a first-user wizard, open to anyone who reaches it first. LightBurn has no machine-side service. **We do better:** the open window is short (30 minutes after each start), local addresses only, nothing can move in it, the fingerprint check is offered, and every owner action is recorded with the name of the one who did it.

**One-line meaning (Proposal, builds on ADR 0071):** *Owner* is whoever holds an owner credential of this connector; an owner can configure, pair, confirm and reset it; a *member* can send jobs, and a member's start waits until an owner confirms it.

## Consequences, as plain facts

- **Nobody needs to stand at the laser any more.** A member's job runs when an owner confirms it, with `curvyoctl` or the app, from anywhere. The old "button or key on the connector's host" (`0064` criterion 9) holds only in foreground mode (A28). Safety then rests on the owner's judgement, `admin.log`, and the machine's own lid switch and emergency stop (Frage 244 A).
- **For 30 minutes after each start of an unconfigured connector, the first host on the workshop network that connects and applies a configuration becomes its owner** (Frage 243 A). Bounds: only an unconfigured connector, only local network addresses, only 30 minutes after a start, nothing can move in it; the maker notices because their own `apply-config` then fails with "already configured". Checking the key fingerprint closes the man-in-the-middle case and is offered, not required.
- Lost every owner credential: the connector is set up again, with physical access to the host (A25). There is no way back over the network.
- Apps paired in slice 2 become **members**, so their starts now wait for an owner (Frage 245 A); the maker's own laptop is paired again with `--owner`.

## What changes in the demo

The customer runs `curvyoctl` on their computer against a fresh connector (the unit file on a Linux box, or `curvyo-connector-server --state-dir <empty dir>` in a terminal; see A29):

- `curvyoctl identify --insecure <address>` prints the connector's name, version, mode ("Maintenance mode, closes in 28 min") and key fingerprint. The connector's journal prints the same fingerprint, in the same form: eight groups of four upper-case hex characters (`3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27`).
- `curvyoctl apply-config --insecure <address> --fingerprint 3F2A-9C10-77B4-E2D1-0A5C-61F8-9B3E-4D27 -f connector.toml` (fake controller, or a real GRBL port chosen from `curvyoctl ports`) sets the connector up; it answers "Configured. You are the owner." and the connector now needs a login.
- `curvyoctl pair --name Laptop --owner` prints a code; the app pairs with it as today and its Connector row shows "Owner". `curvyoctl pair --name Phone` makes a second app a member.
- From the member app: Send to machine, then Start. The Connector row shows "Waiting for <owner> to confirm". `curvyoctl confirm-start` (or Start in the owner app) runs it. `curvyoctl jobs` and `curvyoctl audit` show who sent and who started it.
- On a second computer: `curvyoctl login <address> --code <code>` with a code from `curvyoctl pair --owner` makes that computer an owner too; `curvyoctl whoami` shows its name and role, `curvyoctl clients` lists both computers, and `curvyoctl logout` removes the connector from it again.
- `curvyoctl reset --confirm <name>` wipes the connector and stops the service; it opens a setup window again only after a start on the host.

## Boundary (Proposal)

- **In:** maintenance mode and configured mode (ADR 0071 decisions 1 to 4); owners and members (3, Fragen 245 and 246); `ConnectorConfig`, its file and `ApplyConfig`/`GetConfig` (5, without enrolment); LAN pairing issued by an owner (6); start confirmation by an owner (7); reset and recovery (8); the unit file, the state directory and `--config` (9); the new crate `curvyo-connector-ctl` with `curvyoctl`, including `login`, `logout` and `whoami` for a second computer (10, Frage 252 B); the app's Connector row learning owner and member (11); `admin.log` and the sender and starter in `jobs.log`.
- **Reuses, does not change:** TLS with the pinned key, SPAKE2 pairing, bearer tokens, the job messages, the GRBL controller of `0127`, Send to machine of `0125`. The app pairs exactly as today.
- **No new external dependency** (ADR 0071 decision 11), no `format_version` change of the Curvyo project file.

## Acceptance criteria

Prefix `A`, so that the criteria of `0064` (`P`, `Q`), `0125` (`S`) and `0127` (`G`) keep their numbers. "Fake" means the fake controller; no criterion needs hardware. "Maintenance mode" and "configured mode" are as in ADR 0071.

*Modes and the setup window*

- A1. Given `connector.toml` does not exist in the state directory, when the connector starts without foreground flags, then it is in maintenance mode: it creates the identity key if missing, listens on the configured or default address, does not open a serial port, has no pairing code, and prints on stdout "Maintenance mode until <HH:MM> UTC: waiting for curvyoctl apply-config --insecure" and the key fingerprint. The fingerprint line is printed at every start in every mode.
- A2. Given maintenance mode, when a client sends `Hello`, `ListPorts` or `ApplyConfig`, then each is answered without a login; `Welcome` carries `mode` (`Maintenance { closes_in_s }`). `ListPorts` returns only device path names (`/dev/serial/by-id/*`, `/dev/ttyUSB*`, `/dev/ttyACM*`).
- A3. Given maintenance mode, when any other message arrives (`SubmitJob`, `JobChunk`, `Start`, `Pause`, `Resume`, `Stop`, `Unlock`, `Pair`, `PairSpake`, `Auth`, any owner call), then the answer is `Error { Maintenance }` and no byte goes to any serial port; a test with a recording controller asserts zero writes.
- A4. Given maintenance mode, then a connection from a public (non-local) address is closed after accept; local means loopback, RFC 1918, IPv6 ULA, link-local and 100.64.0.0/10. Tested with injected peer addresses on the loopback.
- A5. Given maintenance mode and the 30-minute window, then `ApplyConfig` is accepted until 30 minutes after process start and answers `Error { MaintenanceClosed }` afterwards, while `Hello` and `ListPorts` still work; no network command reopens the window. The clock is passed in; the test uses a fake clock.
- A6. Given a valid `ApplyConfig`, then the connector writes `clients.json`, then `connector.toml` (each temporary file and rename; `connector.toml` is the commit point), records the sender as owner with a fresh token, answers `ConfigApplied { token }`, closes the other maintenance connections, and switches without restart to configured mode with the controller opened as configured. Given an invalid configuration, then `Error { InvalidConfig }` names the field and the window stays as it was.
- A7. Given a second `ApplyConfig` from another host after the first succeeded, then it is refused with "already configured" (an unauthenticated `ApplyConfig` in configured mode answers `Error { OwnerRequired }`), and the connector's stdout names the client that configured it and its address.

*Roles and login*

- A8. Given configured mode, then every message except `Hello` needs a login (`Auth`) or pairing; after `Paired` and after a valid `Auth` the connector sends `Authenticated { client_name, role }` before the `Status`, so an app or `curvyoctl` knows its own name and role (the role is not stored by the client); `clients.json` has `format = 2` with a `role` per client; a slice-2 `clients.json` (format 1) reads with every client as `member` and is rewritten as format 2 on the next change; a golden file covers both.
- A9. Given a member, when it sends an owner call (`GetConfig`, `IssueCode`, `ListClients`, `RevokeClient`, `GetLog`, `Reset`, `ListPorts`, `ApplyConfig`), then the answer is `Error { OwnerRequired }` and nothing changes; the whole message-by-mode table is one pure function in core (architect D4) with a unit test.
- A10. Given an owner, when it sends `IssueCode { client_name, role }`, then the connector returns a pairing code (ten minutes, one use, three wrong tries, as `Q`-pairing) and records the issue in `admin.log`; the code is never printed on stdout or written to any log.
- A11. Given an owner, then `ListClients` returns name, role and pairing time per client, and `RevokeClient { name }` removes the client at once and closes its connection. Given the last owner (Frage 248 A), then the revoke (and a pairing that would turn the only owner's name into a member) answers `Error { WrongState }`, and `curvyoctl` prints `Laptop is the last owner of workshop-laser` (a statement of the state, no operating tip); a `RevokeClient` of a name nobody has answers `Error { UnknownName }`, printed `workshop-laser has no client called Phon`. The host-side `--revoke` may remove the last owner. Pairing under a name that exists replaces that client (architect D7), so `curvyoctl` has no "name taken" refusal.

*Start rule*

- A12. Given an owner sends `Start` for a held job, then it runs at once, as before with the flag off.
- A13. Given a member sends `Start` (or `Resume`), then the state is `AwaitingLocalStart` and `Status` carries `requested_by` with the member's name; the job runs only when an owner sends `Start` for that job id (A14). `Pause` and `Stop` by any paired client are accepted at once (Frage 249 default A). This is one rule on the LAN, and the same rule travels through the relay later.
- A14. Given `AwaitingLocalStart`, when an owner sends `Start { job_id }`, then the job runs and `admin.log` records "start confirmed" with the job, the requester and the owner; a `Start` with another job id gets `Error`. Given the member sends `Stop`, then a waiting start is discarded and the state is `Idle`; a waiting resume is a paused job and stops through the controller (`Done { Stopped }`, `0127` G12).
- A15. Given the job log, then every line names the client that sent the job and the client that started or confirmed it.

*Configuration, logs, reset*

- A16. Given `ConnectorConfig` (`format = 1`: `name`, `owner` label, `listen`, `[controller] kind = "grbl-serial"|"fake"` with its port, baud or fake pace), then core validates it purely (`name` and `owner` label 1 to 64 characters, client names 1 to 32 **(Proposal)**, no control characters, address parse, baud range 1 to 4,000,000 as `G1`, unknown fields refused); golden files exist for the JSON and the TOML spelling; `GetConfig` (owner) returns it; a controller change by a later `ApplyConfig` from an owner takes effect without a restart when no job is held or running, and answers `Error { WrongState }` while one is (`Receiving` to `Paused`).
- A17. Given `Welcome`, then it carries the connector's `name` (else the host name) and `mode`: `Maintenance { closes_in_s }`, `Configured { owner }` with the owner label, or `Foreground` (no owners, the run flags of A28), shown to every client; the goldens `welcome_maintenance.json`, `welcome_configured.json` and `welcome_foreground.json` cover the three.
- A18. Given `admin.log` (mode `0600`, append only, golden file), then it records one line each: configured (client, address), configuration changed, code issued (name, role, by), paired, revoked, unlocked, start confirmed; every line has the time and the acting client's name. An owner reads the last n lines of `jobs.log` or `admin.log` with `GetLog`; both stay on the host (`R-SYS-007`). The reset is printed on stdout only, since the reset deletes the log (A19).
- A19. Given an owner sends `Reset { connector_name }` with the name typed correctly, then a running job is stopped as on SIGTERM (laser off, `0127` G12), the answer `ResetDone` is sent, the connector stops serving, everything in the state directory is deleted (configuration, identity key, clients, logs; the directory itself stays, it belongs to systemd under A30), and the process exits 0 (ADR 0071 decision 4's order); a name that differs answers `Error { UnknownName }` and changes nothing. Under the unit file the service then stays stopped (`Restart=on-failure` does not restart exit 0).
- A20. Given a reset connector, then the next start on the host is a fresh maintenance mode with a new identity key and a new fingerprint, and the old tokens no longer work.
- A21. Given `curvyo-connector-server --reset --state-dir <dir>` on the host, then it deletes everything in the directory unless a running connector holds `<dir>/lock`, in which case it refuses with a message and exit code 1; `--fingerprint --state-dir <dir>` prints the fingerprint and exits.

*`curvyoctl`*

- A22. Given the crate `curvyo-connector-ctl` with binary `curvyoctl` on Linux, then these commands exist, each with `--help`, plain output lines and exit codes 0 (done), 1 (the connector refused or is unreachable), 2 (usage): `identify`, `ports`, `apply-config`, `get-config`, `status`, `jobs`, `audit`, `confirm-start`, `stop`, `unlock`, `pair [--owner]`, `clients`, `revoke`, `reset --confirm`, `login`, `logout`, `whoami`, `version`. `enrol` is not part of this slice (out of scope). The list is ADR 0071 decision 10's plus `ports` (for `ListPorts`) and `login`, `logout`, `whoami` (A35 to A38).
- A23. Given `apply-config --insecure <address> [--fingerprint F]`, then a key whose fingerprint differs from F is refused before anything is sent; without F the first key seen is remembered in `curvyoctl`'s local list (trust on first use) and printed. A changed key on a later call is refused with "the key of <name> changed", the stored and the presented key shown. **Way on after a reset on the host (Proposal):** `apply-config --insecure <address> --fingerprint F` whose `F` matches the key now presented replaces the stored entry of that address (and drops its old token); without a matching `--fingerprint` the stored key stays authoritative and the call is refused. `--fingerprint` takes all 32 hex characters (spaces or hyphens allowed, no prefix).
- A24. Given `curvyoctl`'s local state, then the connector list (name, address, fingerprint; one entry per connector address) is a TOML file in the user's config directory, created with mode `0600` in a `0700` folder, and each token lives in the credential store (ADR 0007), never in that file; a test with a temporary config directory and a test credential store asserts the token is absent from the file.
- A25. Given a connector that answers a newer call with `UnknownType`, then `curvyoctl` says "connector too old (version <x>)". Given every owner credential lost on every computer, then the only way back is A21 or rewriting the storage, and the documentation says so in one sentence.

*The app*

- A26. Given a paired connector, then the Connector row's trust block shows the rows "Belongs to" (the owner label, absent when empty) and "This app" ("Owner" or "Member", from `Authenticated`), between "Paired with" and "Key". Given a member's Start or Resume, then the job block reads "Waiting for Marc to confirm" ("Waiting for an owner to confirm" without a label; another member's request: "Phone waits for Marc to confirm"). Given an owner app, then the same state reads "Start requested by Phone" ("Resume requested by Phone at line 120") and the existing Start slot is named "Confirm start" ("Confirm resume"), no fifth button, not armed; it sends `Start { job_id }`. Given a waiting request that someone else discarded, then the member app reads "Not started: sign.gcode" until the next state change. Given a foreground connector (no `requested_by`), the line stays "Waiting for the start at the machine". The Output group's state line (`0125`) takes the same words. A member's Pause and Stop work at once (Frage 249 A). Link lines: "Connected, maintenance mode, protocol 0, connector <version>" (Test connection, no Pair row), "Connector in maintenance mode" (a paired profile meets `Maintenance`), "This app is a member" (a refused owner call). The app sends no owner call in this slice. No operating tips. Layout and all other words: `ux-notes.md` sections 8 and 9.
- A27. Given a paired app from slice 2 after the upgrade, then the "This app" row shows "Member" (the record stays format 1, the role comes with every `Authenticated`); nothing is lost and no re-pairing is forced (the maker may pair again with `--owner`).

*Foreground mode, packaging, build*

- A28. Given the run flags of slices 2 and 4 (`--serial`, `--baud`, `--fake-controller`, `--require-local-start`, `--pair`), then they form foreground mode: held in memory, never written, with today's terminal behaviour (code on stdout, Enter, `pair`); refused with a message when `connector.toml` exists; never used by the unit file. **Decision (Proposal, from ADR 0071 decision 1 and 7):** the terminal Enter and `--require-local-start` stay as a documented legacy for foreground mode only, with a technical-debt entry for their removal once tests and demos use a configuration; the service never uses them.
- A29. Given tests and the demo without a Linux service box, then a connector started in a temporary state directory without foreground flags and with a fake controller in its applied configuration is the setup demo (no real serial port needed); `ListPorts` on a machine without serial devices returns an empty list.
- A30. Given `curvyo-connector-server/dist/curvyo-connector.service`, then it uses `DynamicUser=yes`, `StateDirectory=curvyo-connector` (`0700`), `SupplementaryGroups=dialout`, `Restart=on-failure`, `TimeoutStopSec=15`, `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`, `DevicePolicy=closed` with `DeviceAllow` for `char-ttyUSB` and `char-ttyACM`; `systemd-analyze verify` passes on it (customer check on the Linux box if not run in CI); `docs/guides/connector-service-linux.md` covers install, groups (Arch `uucp`) and `curvyoctl`.
- A31. Given `connector.toml` and the command line, then `--state-dir` names the directory (no file can), a value set in `connector.toml` wins over a flag, a value the file leaves out takes the flag, else the default; only `listen` overlaps (`--listen`), and stdout says when the flag was ignored. There is no `--config <file>` (ADR 0071 decision 9; architect addendum D8).
- A32. Given every new message (including `Authenticated`, `Welcome` with `mode`, `Status` with `requested_by`, and the codes `Maintenance`, `MaintenanceClosed`, `InvalidConfig`, `OwnerRequired`, `UnknownName`) and the config file, then each has a golden file in `tests/fixtures/` (JSON frames, TOML, `clients.json` formats 1 and 2, `admin.log`); protocol stays 0, additive only; an older connector's `UnknownType` is covered (A25).
- A33. Given the crates, then `curvyo-connector-core` still builds for `wasm32-unknown-unknown`, `#![forbid(unsafe_code)]`, no I/O dependency; `curvyo-connector-ctl` uses `anyhow`, the libraries use `thiserror`; `cargo deny` is green including the AGPL exception for the new crate; the quality gate of `CLAUDE.md` §7 is green.
- A34. Given `curvyoctl`'s output, then it follows `ux-notes.md` sections 1 to 6: results on stdout, every refusal one line `curvyoctl: <text>` on stderr, ASCII only, no colour, no operating tips, labelled lines and columns, times in UTC, the fingerprint in one form (eight groups of four), `<connector>` optional while one is known, the tool calls itself `curvyoctl@<host name>`, `--last <n>` (default 20) on `jobs` and `audit`, the texts of `login`, `logout` and `whoami` as in ux-notes section 4.13, `--help` text within 80 columns; tests assert the texts against the loopback connector, including `pair`'s "Pairing code: K7QM-4XTR, valid for 10 minutes" and `confirm-start`'s "Started sign.gcode, requested by Phone" ("Resumed ... at line n"). `confirm-start` asks nothing and sends `Start { job_id }` only when a request waits; with none it prints `no start is waiting on <name>`, exit 1. `stop` with no job prints "No job", exit 0.

*`curvyoctl login`, `logout`, `whoami` (a second computer; Frage 252 B)*

- A35. Given an owner ran `curvyoctl pair --name <name> --owner` and the code is valid, when a second computer runs `curvyoctl login <address> --code <code>` (the code with or without its hyphen, either case, checked for its shape locally so that a typing slip spends no attempt), then it pairs with the connector as the app does (SPAKE2 over the pinned-on-first-use TLS key, the existing client in `-io`; no new dependency), the connector records it as a client named `curvyoctl@<host name>` with role **owner**, and it prints "Logged in to workshop-laser. This computer is an owner." with Connector, Address, Name and the stored key. After it, `clients` on either computer lists both, and every owner command works from the second computer (a test runs `confirm-start` and `pair` from it). The role is the one the code was issued for: a code from `pair` without `--owner` stores nothing and prints `curvyoctl: the code was issued for a member`; the member client it created stays listed until revoked.
- A36. Given `login`, then it stores exactly what `apply-config --insecure` stores (A24): one entry per connector address (name, address, the full key fingerprint pinned from the TLS key the SPAKE2 run confirmed) in `curvyoctl.toml`, written with mode `0600` in a `0700` folder, and the token only in the credential store; the connector's identity key and the pairing code are never written. Given an address already in the list with the same key, then `login` replaces its token (a repeated login of one computer replaces its client, D7); given a different connector name already known at another address, then it is refused with `curvyoctl: a connector called workshop-laser is already known at 192.168.1.21:7447`. Tested with a temporary config directory and a test credential store, asserting the file mode and that the token is absent from every file.
- A37. Given a refused login, then nothing is stored and each case is one stderr line, exit 1: wrong code `wrong code, 2 attempts left` ("1 attempt left"), third wrong code `code spent`, expired code `code expired`, code already used or never issued `code not valid`, connector in maintenance mode `workshop-pi is in maintenance mode`. A man in the middle looks like a wrong code (ADR 0035 section 4); the stored key is never replaced by a login (A38).
- A38. Given a connector whose key differs from the stored entry of that address (after a reset on the host), then `login` is refused before any code is sent, as A23: `the key of workshop-laser changed` with `Stored key:` and `Presented key:`; the code keeps all its attempts. `curvyoctl logout [<connector>]` removes the entry and its token from this computer only (no message to the connector, so it works offline and with a lost connector), prints `Removed workshop-laser from this computer`, and the client stays in `clients` until an owner revokes it; a login after the logout then pins the new key. `curvyoctl whoami [<connector>]` connects with the stored token and prints Connector, Address, Name, Role and Key; its refusals are those of every owner command (ux-notes section 5).

## Tests without hardware

Core: pure tests of the config validation, the role-aware start rule and the state machine with the fake controller (goldens). `-io`: a loopback connector with the fake controller and a recording controller in a temporary state directory, a fake clock and injected peer addresses for the window; `curvyoctl` is run as a library against that loopback connector in tests (setup, pair, login of a second computer, member start, confirm, reset). A GRBL simulator is not needed. The customer check on the real Linux box is the systemd unit (A30) and one run against a real GRBL port (🟡 until done).

## Out of scope

- Enrolment through the relay (`curvyoctl enrol`), the machine room, admins of the maker's devices as owners, and sharing with a workspace (slices 6 and 7 of `0064`, ADR 0070).
- EST, a GPIO start button (Frage 244 B, not chosen), a boot-partition reset trigger, the web app served on the LAN, the direct channel of ADR 0070.
- Windows and macOS for `curvyoctl` (Linux first; builds elsewhere are not promised) and for the connector.
- Accounts, email, server-side identity, telemetry; changing the pairing code ceremony (SPAKE2) itself.
- Removing foreground mode (a debt entry only), multiple machines per connector, a graphical setup tool.

## Questions

**Decided by the customer (ADR 0071, 2026-11-18):** Frage 243 A, Frage 244 A, Frage 245 A, Frage 246 A.

**Decided by the customer (2026-11-18):**

- Frage 250 (CONN-CONFIRM-NOTICE) A: an owner learns of a waiting start only where the job shows (Connector row, Output line, `curvyoctl status`); a notice in any page is a later story (B).
- Frage 251 (CONN-CONFIRM-EXPIRY) A: a request waits until confirmed or stopped, no expiry; the waiting time is shown where the connector reports it (optional, not a criterion).
- Frage 252 (CONN-CTL-SECOND) **B**: a second computer manages the connector in this slice with `curvyoctl login <address> --code <code>`, using a code from `curvyoctl pair --owner` (A35 to A38; SPAKE2 client exists in `-io`, no new dependency).

**Applied by default, vetoable at the demo** (no answer needed to build):

- Frage 247 (CONN-FOREGROUND-ENTER) A: Enter and `--require-local-start` stay in foreground mode only, documented legacy (A28).
- Frage 248 (CONN-LAST-OWNER) A: the connector refuses to remove the last owner; the way out is reset (A11).
- Frage 249 (CONN-MEMBER-STOP) A: Pause and Stop work for every paired app at once; Start and Resume wait for an owner (A13).

## Done before Ready

- `ux-notes.md` (2026-11-18): P1 to P6 applied here (the last-owner text, lower-case refusals kept, the eight-group `--fingerprint`, the new app and `curvyoctl` texts, the way on after a host reset in A23, name limits in A16: 64, 64 and 32 instead of the proposed 40, since long names are cut with an ellipsis).
- Architect addendum D1 to D17 in `specs/0064-machine-connector/adrs.md` (2026-11-18). Its Frage 250 (CONN-CTL-SECOND) is Frage 252 here (numbers collided); its D7 wording "That is the last owner; reset the connector instead" is superseded by A11's text. Corrected in `adrs.md` (D7, D12 and its end: Frage 252) on 2026-11-18.
- Frage 252 B applied 2026-11-18: A35 to A38, `ux-notes.md` section 4.13, `adrs.md` D12.

## Depends on

Building this slice waits for the merge of PR #386 (`0127-connector-grbl-serial`: the controller fields, the SIGTERM stop that Reset reuses, `docs/guides/connector-serial-linux.md`). The spec and the UX and architect work may proceed before the merge.

## Links

Requirements: R-MFG-LASER-002, R-MFG-011, R-SYS-007
Depends on: `specs/0127-connector-grbl-serial/` (PR #386, not merged), `specs/0125-send-to-machine/`, `specs/0064-machine-connector/` slices 1 to 3
Related: ADR 0071 (Accepted, this slice's design), ADR 0070 (slices 6 and 7 follow), ADR 0035, ADR 0007, `specs/0118` (motion from a member waits)
PR: -
