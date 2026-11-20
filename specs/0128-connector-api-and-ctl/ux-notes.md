# 0128 Connector API, `curvyoctl` and maintenance mode: UX notes

ux-engineer, 2026-11-18. Covers criteria A1 to A38 of `specification.md` (mainly A1, A5 to A7, A10, A11, A13, A14, A18, A19, A21 to A27) and its list "Needed before Ready" for the ux-engineer. Checked against ADR 0071, `specs/0064-machine-connector/ux-notes.md` (Connector group, link line texts, fingerprint, job buttons per state, terminal wording), `specs/0125-send-to-machine/ux-notes.md` (Output group state line), the built `frontend/src/components/workbench/ConnectorGroup.tsx`, `curvyo-connector-server/src/main.rs` and `args.rs` (today's terminal lines and help text), `curvyo-connector-io/src/spki.rs` (fingerprint form), and `docs/design-system.md` (rows "Connector group (`0064`)", "Connector state line (`0125`)", "Hidden, not disabled"; Interaction conventions "No notice after a success", "No operating tips by default"). Customer rules applied: no popups, no dialog, symbol buttons with tooltip and accessible name, labels that name a control or a state only, no operating tips (also in `curvyoctl` output and the journal: texts name states, not instructions). Choices marked (Proposal) are vetoable at the demo; section 12 lists them as V1 to V16.

Example values used throughout: connector `workshop-laser` (host name `workshop-pi` before setup), owner label `Marc`, address `192.168.1.20:7447`, apps `Laptop` (owner) and `Phone` (member), job `sign.gcode`, fingerprint `3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27`.

## 1. `curvyoctl` output rules (all commands)

- **Streams.** Results on stdout; every refusal and failure on stderr. An error is one line `curvyoctl: <text>` (lower case after the prefix, no full stop, as `curvyo-connector-server` does), optionally followed by labelled detail lines (the two keys of a key mismatch). Nothing else goes to stderr: no progress, no spinner, no "connecting..." line.
- **No colour, no emoji, no box drawing**, ASCII only, so output reads the same in a journal, a pipe and a dumb terminal. No pager.
- **No operating tips (V1).** No "Try --help", no "run X first", no "restart the service". A text names what is (a state, a refusal, a value). `--help` is the one place that describes commands, because the maker asked for it.
- **Labelled lines** for one record (`identify`, `status`, `pair`, `apply-config`): `Label:` then the value, the value column starting one space after the longest label of that output, so values line up within one command's output. Labels are nouns in sentence case.
- **Columns** for lists (`jobs`, `audit`, `clients`): a header line, then one row per entry, columns separated by at least two spaces and padded to the widest cell, the last column free (it may be long). `ports` is one column: one path per line, no header. An empty list prints one line naming it ("No jobs", "No entries", "No serial ports") on stdout, exit 0.
- **Times in UTC (V2)**, as the connector's journal: labelled values "14:32 UTC", columns headed "Time (UTC)" with cells "2026-11-18 14:06:11". Durations as the app: "m:ss" under an hour, "h:mm:ss" from an hour; window times as "28 min" (rounded down, "under 1 min" below 60 s).
- **Numbers and sizes** as the app (`0064` 8): thousands comma ("5,000 lines"), binary sizes ("9.2 KiB").
- **Roles** are lower case words in `curvyoctl` ("owner", "member") and capitalised values in the app ("Owner", "Member").
- **Exit codes** (A22): 0 done; 1 the connector refused, did not answer, or a local store or file failed (V3: local failures count as 1, not a new code); 2 usage (unknown command or option, missing value, bad fingerprint form, unknown connector name, `--confirm` mismatch). Usage errors print the command's usage line after the error line (section 5).
- **Line length:** every fixed text fits 80 columns; only values (paths, names) may run longer, never wrapped by `curvyoctl`.

## 2. Key fingerprint

- **One form everywhere:** 8 groups of 4 upper-case hex characters separated by single spaces (`spki.rs`), 39 characters: `3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27`. `curvyoctl` and the journal print it on one line; the app shows the same groups as two lines of 4 (`0064` 5), the first app line being groups 1 to 4. No short form, no colon form, no partial fingerprint anywhere.
- **Where the maker compares it:** the journal line `Key fingerprint: …` printed at every start in every mode (A1), `curvyo-connector-server --fingerprint --state-dir <dir>` on the host (A21), `curvyoctl identify`, and the app's trust block. Comparing is group by group; the identical grouping is the whole aid (no checksum word, no picture: YAGNI).
- **`--fingerprint <key>` argument:** the 32 hex characters, upper or lower case, with spaces (quoted), hyphens or nothing between the groups: `--fingerprint 3F2A-9C10-77B4-E2D1-0A5C-61F8-9B3E-4D27`. Anything else is a usage error (exit 2): `curvyoctl: --fingerprint needs 32 hex characters`. A prefix is never accepted (V4: a prefix check is a weaker check that looks like the real one).
- **Key labels**, as the app: "Key" (the presented key, nothing to compare), "Stored key" (in `curvyoctl`'s list), "Expected key" (from `--fingerprint`), "Presented key" (what the connector sent).

## 3. Naming a connector

- `curvyoctl` remembers each set-up connector under its configured `name` (A24). Commands after setup take `<connector>` as their first argument.
- **`<connector>` may be left out while the list holds exactly one connector (V5)**, as the spec's demo writes `curvyoctl pair --name Laptop --owner` and `curvyoctl confirm-start`. With two or more and none given: `curvyoctl: more than one connector is known: garage-plotter, workshop-laser` (exit 2). With none known: `curvyoctl: no connector is known` (exit 2). An unknown name: `curvyoctl: no connector is called workshop-lasr` (exit 2).
- **The name `curvyoctl` gives itself** in `ApplyConfig { client_name }` (shown in `clients`, `audit`, the journal): `curvyoctl@<host name>` of the maker's computer, e.g. `curvyoctl@marc-laptop` (V6). No option to change it.

## 4. Commands and their output

### 4.1 `identify`

`curvyoctl identify --insecure <address>` (any connector, no stored key needed) or `curvyoctl identify [<connector>]` (checks the stored key first). Reads `Hello` only; stores nothing.

Maintenance mode, window open:

```text
Name:     workshop-pi
Software: Curvyo connector 0.2.0, protocol 0
Mode:     Maintenance mode, closes in 28 min (14:32 UTC)
Setup:    open to the local network
Key:      3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27
```

- "Setup: open to the local network" is the state that carries the risk of decision 2 (anyone on the workshop network may apply a configuration now). It names the state; it does not warn or instruct (V7).
- Window closed: `Mode: Maintenance mode, closed` and `Setup: closed`. No reopening hint.
- Configured mode: `Name: workshop-laser`, `Belongs to: Marc` (absent without an owner label), `Software: …`, `Mode: Configured`, `Key: …`. No Setup line.
- `identify --insecure` at an address whose stored entry has another key: the output above, then on stderr `curvyoctl: the key of workshop-laser changed` with `Stored key:` and `Presented key:` lines, exit 1.

### 4.2 `ports`

`curvyoctl ports --insecure <address>` (maintenance) or `curvyoctl ports [<connector>]` (owner). One path per line in the order the connector returns them (by-id names first), e.g. `/dev/serial/by-id/usb-1a86_USB2.0-Serial-if00-port0`, `/dev/ttyUSB0`. None: `No serial ports`.

### 4.3 `apply-config`

`curvyoctl apply-config --insecure <address> [--fingerprint <key>] -f <file>` (setup) or `curvyoctl apply-config [<connector>] -f <file>` (owner, a change).

- **Before anything is sent** `curvyoctl` reads the file and validates it with core's `ConnectorConfig` (A16); a fault stops here with the connector's own wording (section 5) and exit 1, nothing sent, the window untouched.
- **Setup success** (A6):

```text
Configured. You are the owner.
Connector:  workshop-laser
Controller: GRBL on /dev/ttyUSB0, 115200 baud
Key:        3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27
```

  "Configured. You are the owner." is the spec's line. `Connector:` is the name to type in later commands. `Controller:` reads "GRBL on <port>, <baud> baud" or "Fake controller: nothing moves". Without `--fingerprint` the key line reads `Stored key:` (trust on first use, A23: the key is now remembered and printed); with a matching `--fingerprint` it reads `Key:`.
- **Change success** (owner, configured mode): `Configuration changed.` and the `Controller:` line when the controller changed. Nothing else.
- Refusals in section 5.

### 4.4 `get-config`

Prints the stored configuration as TOML exactly as `connector.toml` would hold it, nothing before or after, so `curvyoctl get-config > connector.toml` and a later `apply-config -f connector.toml` round-trip. No token or key in it.

### 4.5 `status`

```text
Connector:  workshop-laser, belongs to Marc
Controller: GRBL on /dev/ttyUSB0
State:      Start requested by Phone
Job:        sign.gcode, 412 lines, 9.2 KiB
Sent by:    Phone
```

`State:` uses the owner app's words (section 8.3, `curvyoctl` is always an owner): "Idle", "Receiving sign.gcode", "Ready", "Start requested by Phone" / "Resume requested by Phone at line 120" with ", waiting 3:12" appended when the connector reports the waiting time (open point A2), "Running, line 120 / 412, 0:42", "Paused at line 120, 0:42", "Completed, 412 lines", "Stopped at line 120", "Failed at line 120", "Alarm: <text>", "No controller". `Job:` and `Sent by:` are absent with no job.

### 4.6 `confirm-start`

`curvyoctl confirm-start [<connector>] [<job name>]`. Reads `Status`; when a start or resume is requested it sends `Start { job_id }` at once, without a question (V8: the command is the deliberate act, and it must work from a script).

- Done: `Started sign.gcode, requested by Phone` (or `Resumed sign.gcode at line 120, requested by Phone`). The line names what ran, so a confirm that raced with another send is visible at once.
- `<job name>` given and the waiting job has another name: `curvyoctl: workshop-laser waits for cut.gcode, not sign.gcode`, nothing sent, exit 1.
- Nothing requested: `curvyoctl: no start is waiting on workshop-laser`, exit 1 (also when a job is Ready but nobody pressed Start: `confirm-start` confirms, it does not start).

### 4.7 `stop`, `unlock`

- `stop`: `Stopped sign.gcode at line 120` (running or paused), `Discarded sign.gcode` (Ready or a waiting request), `No job` when nothing is held (exit 0, V9: Stop is the safety command; "nothing to stop" is its goal state, not a refusal).
- `unlock`: `Alarm cleared` / `No alarm` (exit 0).

### 4.8 `jobs` and `audit`

`curvyoctl jobs [<connector>] [--last <n>]`, `curvyoctl audit [<connector>] [--last <n>]`, default 20 lines, newest last (V10; `GetLog` already carries `last`).

```text
Time (UTC)           Job         Sent by  Started by  Result
2026-11-18 14:20:03  sign.gcode  Phone    Laptop      Completed, 412 lines
2026-11-18 14:41:10  cut.gcode   Laptop   Laptop      Stopped at line 87
```

```text
Time (UTC)           By                     Event
2026-11-18 14:06:11  curvyoctl@marc-laptop  configured from 192.168.1.30
2026-11-18 14:08:40  curvyoctl@marc-laptop  code issued for Laptop, owner
2026-11-18 14:09:02  Laptop                 paired as owner
2026-11-18 14:12:20  curvyoctl@marc-laptop  code issued for Phone, member
2026-11-18 14:13:05  Phone                  paired as member
2026-11-18 14:20:01  Laptop                 start confirmed: sign.gcode, requested by Phone
2026-11-18 14:31:44  curvyoctl@marc-laptop  revoked Phone
2026-11-18 14:35:00  Laptop                 unlocked
2026-11-18 15:00:00  curvyoctl@marc-laptop  configuration changed
```

- "Started by" is empty for a job that never started (discarded); "Result" uses the job log's result words (`0064` Q18) plus "Discarded".
- The event words are the A18 list in plain lower case. The architect's `admin.log` line format (open point A3) should carry exactly these fields so `curvyoctl` only aligns columns and never rewords.
- The reset itself never shows in `audit`: the log is gone with it.

### 4.9 `pair`

`curvyoctl pair [<connector>] --name <name> [--owner]`:

```text
Pairing code: K7QM-4XTR, valid for 10 minutes
For:          Phone, member
```

The code line is the connector terminal's line of `0064` 11 (two groups of 4 with a hyphen; the app's Code field ignores it). `curvyoctl` exits at once; it does not wait for the app (the app's trust block appearing is the result; `audit` records "paired"). The code is printed only here, never stored by `curvyoctl` (A10).

### 4.10 `clients`, `revoke`

```text
Name                   Role    Paired (UTC)
curvyoctl@marc-laptop  owner   2026-11-18 14:06  (this computer)
Laptop                 owner   2026-11-18 14:09
Phone                  member  2026-11-18 14:13
```

- "(this computer)" marks the row whose token `curvyoctl` uses (a state label).
- `revoke Phone`: `Revoked Phone` (the server's precedent). Revoking this computer's own entry while another owner remains: `Revoked curvyoctl@marc-laptop` and `Removed workshop-laser from this computer` (its token is useless now).

### 4.11 `reset`

`curvyoctl reset [<connector>] --confirm <connector name>`:

```text
Reset workshop-laser: stopped, configuration, key, clients and logs deleted
Removed workshop-laser from this computer
```

- A running job is stopped first (A19); a line `Stopped sign.gcode at line 120` then comes before the two above.
- `--confirm` missing: usage error `curvyoctl: reset needs --confirm <connector name>` (exit 2). Mismatch: `curvyoctl: --confirm workshop-lasr does not match workshop-laser` (exit 2, checked locally, nothing sent; the connector checks again, A19).
- No question, no countdown: the typed name is the confirmation (decision 8).

### 4.12 `version`

`curvyoctl 0.2.0, protocol 0` (the form of the connector's first line).

### 4.13 `login`, `logout`, `whoami` (a second computer, Frage 252 B)

`curvyoctl login <address> --code <code>`. The code is the one `pair --owner` printed, with or without the hyphen, either case. A code that is not 8 characters of the code alphabet is a usage error before anything is sent (`curvyoctl: --code is not a pairing code`, exit 2), so a typing slip spends no attempt.

```text
Logged in to workshop-laser. This computer is an owner.
Connector:  workshop-laser
Address:    192.168.1.20:7447
Name:       curvyoctl@marc-laptop
Stored key: 3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27
```

`Name:` is the computer's name in `clients` and `audit`.

`curvyoctl logout [<connector>]`: `Removed workshop-laser from this computer`. Local only; the connector is not asked and its client list is unchanged.

`curvyoctl whoami [<connector>]`:

```text
Connector: workshop-laser
Address:   192.168.1.20:7447
Name:      curvyoctl@marc-laptop
Role:      owner
Key:       3F2A 9C10 77B4 E2D1 0A5C 61F8 9B3E 4D27
```

Refusals of `login` (stderr, exit 1, nothing stored):

| Case | Text after `curvyoctl: ` |
|---|---|
| Wrong code | `wrong code, 2 attempts left` ("1 attempt left") |
| Third wrong code | `code spent` |
| Expired | `code expired` |
| Used already, or never issued | `code not valid` |
| Code issued for a member | `the code was issued for a member` |
| Connector in maintenance | `workshop-pi is in maintenance mode` |
| Stored key of that address differs | `the key of workshop-laser changed`, then `Stored key:` and `Presented key:` lines; the code is not sent |
| Name known at another address | `a connector called workshop-laser is already known at 192.168.1.21:7447` |

The other refusals of section 5 (nothing listens, no answer, too old) apply as written. Texts reuse the app's pairing words (`0064` ux-notes, Q5 to Q7), lower case after the prefix.

## 5. Refusals and failures (stderr, one line, then optional detail)

| Case | Text after `curvyoctl: ` | Exit |
|---|---|---|
| Nothing listens | `nothing answers at 192.168.1.20:7447` (as the app, address as dialled, 7447 when none typed) | 1 |
| No answer in 5 s | `no answer from workshop-laser` (or the address) | 1 |
| Connection closed by the connector after accept (public peer, A4) | `192.168.1.20:7447 closed the connection` (V11: `curvyoctl` cannot know why; no guess) | 1 |
| `--fingerprint` differs (A23) | `the key at 192.168.1.20:7447 differs from --fingerprint`, then `Expected key:` and `Presented key:` lines; nothing sent | 1 |
| Stored key differs (A23) | `the key of workshop-laser changed`, then `Stored key:` and `Presented key:` lines; nothing sent | 1 |
| Already configured (A7, `OwnerRequired` on an unauthenticated `ApplyConfig`) | `workshop-laser is already configured` | 1 |
| Window closed (A5, `MaintenanceClosed`) | `the setup window of workshop-pi is closed` | 1 |
| Invalid configuration, local or `InvalidConfig` (A6) | `connector.toml: controller.baud: 0 is not from 1 to 4,000,000` (file, field path, value and the rule as a fact); a TOML syntax fault: `connector.toml, line 4: <parser text>` | 1 |
| File unreadable | `cannot read connector.toml: <OS text>` | 1 |
| Controller change while a job runs (A16, `Busy`) | `workshop-laser is running a job; configuration not changed` | 1 |
| Command needs configured mode, connector in maintenance (`Maintenance`) | `workshop-pi is in maintenance mode` | 1 |
| This computer is not an owner (`OwnerRequired`) | `this computer is not an owner of workshop-laser` | 1 |
| Token refused (revoked, or the connector was reset at the host) | `workshop-laser does not know this computer` | 1 |
| No token in the credential store | `no token for workshop-laser in the credential store` | 1 |
| No credential store on this computer | `no credential store on this computer` (the app's words) | 1 |
| Older connector (A25, `UnknownType`) | `connector too old (version 0.1.0)` | 1 |
| Last owner (A11, Frage 248 A) | `Laptop is the last owner of workshop-laser` | 1 |
| Unknown client on revoke | `workshop-laser has no client called Phon` | 1 |
| Name taken on pair (open point A4) | `workshop-laser already has a client called Phone` | 1 |
| Unknown command or option | `unknown command frob` (no usage line: there is no command to show it for) / `unknown option --forse`, then the usage line | 2 |
| Missing value | `--name needs a value`, then the usage line of that command | 2 |

Any other connector error code: `workshop-laser refused: <ErrorCode name>` (V12, the CLI may name the code; the app keeps "Connector error").

## 6. `--help` wording

`curvyoctl --help` (also `-h` and `curvyoctl help`), plain text, the server's `HELP` layout:

```text
curvyoctl: set up and manage Curvyo connectors from this computer.

Usage: curvyoctl <command> [<connector>] [options]

Setup of a connector in maintenance mode:
  identify --insecure <addr>         name, software, mode and key
  ports --insecure <addr>            serial ports of the connector's host
  apply-config --insecure <addr> -f <file> [--fingerprint <key>]
                                     configure it; this computer becomes owner

A configured connector (owner):
  identify                           name, software, mode and key
  ports                              serial ports of the connector's host
  apply-config -f <file>             change the configuration
  get-config                         print the configuration as TOML
  status                             connector, controller and job
  confirm-start [<job>]              run the job whose start a member requested
  stop                               stop or discard the job
  unlock                             clear the controller's alarm
  jobs [--last <n>]                  the job log (default 20 lines)
  audit [--last <n>]                 the admin log (default 20 lines)
  pair --name <name> [--owner]       pairing code for an app, member or owner
  clients                            the paired apps and their roles
  revoke <name>                      unpair a client
  reset --confirm <name>             delete all its state and stop it

A second computer:
  login <addr> --code <code>         become an owner with a code from pair --owner
  logout                             remove the connector from this computer
  whoami                             this computer's name, role and the key

Other:
  version                            this program's version and protocol
  --help                             this text; after a command, its own text

<connector>  a name from this computer's list; optional while it holds one
<key>        a key fingerprint: 32 hex characters, spaces or hyphens allowed
Exit codes: 0 done, 1 refused or not reachable, 2 usage.
```

- `curvyoctl <command> --help`: the command's usage line, one sentence saying what it does (the right column above, as a sentence), its options one per line, nothing else. Example: `Usage: curvyoctl pair [<connector>] --name <name> [--owner]` / `Print a pairing code for an app; the app becomes a member, or an owner with --owner.` / `--name <name>   the app's name in clients and logs` / `--owner   the app becomes an owner`.
- Descriptions say what a command does, never when or why to use it.

## 7. Connector journal (stdout of `curvyo-connector-server`)

Kept from `0064` 11: the version line, "Listening on …", "Key fingerprint: …" at every start. New lines (plain, one event each, never a code or token, A10):

| When | Line |
|---|---|
| Start, maintenance (A1, spec text) | `Maintenance mode until 14:32 UTC: waiting for curvyoctl apply-config --insecure` |
| Window ends (A5) | `Maintenance window closed at 14:32 UTC` |
| Setup done (A6) | `Configured by curvyoctl@marc-laptop from 192.168.1.30` and the controller line (`Controller: GRBL on /dev/ttyUSB0, 115200 baud` / the fake's line) |
| Second setup attempt (A7) | `Setup from 192.168.1.44 refused: configured by curvyoctl@marc-laptop from 192.168.1.30` |
| Start, configured mode | `Configured: workshop-laser, belongs to Marc`, then the controller line |
| Member start (A13) | `Start of sign.gcode requested by Phone` |
| Confirmation (A14) | `Started sign.gcode, confirmed by Laptop for Phone` |
| Reset (A19) | `Reset by curvyoctl@marc-laptop: state directory deleted` |
| Host `--reset` done (A21) | `Deleted the contents of /var/lib/curvyo-connector` (the directory itself stays) |
| Host `--reset` while running (A21) | stderr `curvyo-connector-server: a running connector holds /var/lib/curvyo-connector/lock`, exit 1 |
| Foreground flag with `connector.toml` present (A28) | stderr `curvyo-connector-server: --serial cannot be used: /var/lib/curvyo-connector/connector.toml exists`, exit 2 |

The maintenance line's "waiting for curvyoctl apply-config --insecure" is fixed by A1 and names what the connector waits for; it stays.

## 8. The app: Connector row

### 8.1 Trust block (paired)

The `0064` trust block gains two rows on the same 80px label grid, between "Paired with" and "Key" (V13):

| Label | Value |
|---|---|
| Paired with | connector name, Forget pairing at the right (as built) |
| Belongs to | the connector's owner label (`Welcome.owner`, A17), 12px `--toolbar-icon`, one line, end ellipsis, full text in `title` and in the block's accessible text; the row is absent when the label is empty |
| This app | "Owner" or "Member", 12px `--toolbar-icon` |
| Key | fingerprint, two lines (as built) |

- "Belongs to", not "Owner", for the label row: "Owner" is the role word one row below, and "Belongs to: Marc" reads right for a person and for "Marc's workshop". `curvyoctl` uses the same words (sections 4.1, 4.5). The config field stays `owner`.
- Accessible text of the block: "Paired with workshop-laser, belongs to Marc, this app is a member, key 3F2A 9C10 …".
- The role and owner label come from the connector at every login and are kept with the connector's name in the credential record (open point A1), so the rows show offline too; before the first login of a record that has none (a slice-2 record), the "This app" row is absent rather than guessed. A role that changes (the app was paired again with `--owner`) updates the row on the next read; no notice.
- Upgraded slice-2 pairing (A27): "This app: Member". No hint about `--owner`.
- Narrow panel: 244px content, value column 160px; "Member" and "Owner" never wrap; the owner label cuts with an ellipsis. Coarse pointer: no change (text rows, no targets).

### 8.2 Start slot: Start, Resume, Confirm start, Confirm resume

No new button (V14). The confirmation is `Start { job_id }` (decision 7), so it lives in the existing Start / Resume slot: the toolbar stays four buttons, 124px, and Stop never moves. Glyph `Play` in every case; the name and tooltip say what the press does.

| State | This app | Start slot | Pause | Stop |
|---|---|---|---|---|
| Ready | owner or member | on, "Start" | off | on (discards) |
| Paused | owner or member | on, "Resume" | off | on |
| AwaitingLocalStart, requested by a member, line 0 | owner | on, "Confirm start" | off | on (discards) |
| AwaitingLocalStart, requested by a member, line > 0 | owner | on, "Confirm resume" | off | on |
| AwaitingLocalStart | member | off, "Start" | off | on (discards, Frage 249 A) |
| AwaitingLocalStart, no `requested_by` (foreground `--require-local-start`) | either | off, "Start" | off | on |
| Running | either | off | on (any paired app, Frage 249 A) | on |

- Name, tooltip and accessible name are equal ("Confirm start"); `aria-describedby` points to the job block's state line, so a screen reader hears "Confirm start, Start requested by Phone".
- Not armed (V15): Start was never armed, and the state line names requester and job before the press. Stop stays never armed.
- Busy: from the press until the connector's `Status` reads Running (or 5 s, "Timed out"), as `0064` 3. No optimistic change.
- A member's press of Start or Resume is not a failure: the state moves to AwaitingLocalStart and the line says so (8.3). No alert.

### 8.3 Job block state line

| State | Owner app | Member app |
|---|---|---|
| AwaitingLocalStart, line 0 | "Start requested by Phone" | own request: "Waiting for Marc to confirm"; another member's: "Phone waits for Marc to confirm" |
| AwaitingLocalStart, line > 0 | "Resume requested by Phone at line 120" | as at line 0 |
| AwaitingLocalStart, no `requested_by` | "Waiting for the start at the machine" (as built) | the same |

- Without an owner label, "Marc" reads "an owner": "Waiting for an owner to confirm", "Phone waits for an owner to confirm".
- Every text stays within 70 characters; names are cut at their end with an ellipsis if a long requester or owner label would pass it (the `0125` U5 rule); the live region carries the full text.
- "Own request" needs the app's own client name from the login answer (open point A1). Without it, every member sees "Waiting for Marc to confirm".
- The progress bar shows at 0 (or held at the line for a resume), as built. The elapsed slot at the right of the state line shows the **waiting time** "m:ss" (accessible name "Waiting <text>") when the connector reports it (open point A2); otherwise it is empty.
- **When confirmation never comes:** the line stays as it is, for as long as the request waits; there is no timeout, reminder, notice or sound in this slice (Frage 251 A, decided: no expiry). Stop stays on and withdraws the request (A14). "Connection lost" and the 5 s retry work as built; a connector restart drops the job and the block goes (Idle).
- **A request discarded by someone else** (an owner's Stop or `curvyoctl stop`): the member app, which last saw AwaitingLocalStart for that job and did not press Stop itself, shows "Not started: sign.gcode" (muted) in the state line until the next Send, test or state change, so the job does not vanish without a word (V16). Its own Stop gives no line, as built.
- Live region: the state words as built (polite, once per change). The owner app's "Start requested by Phone" is polite, not an alert: it answers no press of this app.

### 8.4 States and errors in the app

New link line texts (`0064` 4 look and rules):

| When | Link line | Look | Rows shown |
|---|---|---|---|
| Test connection, maintenance mode | "Connected, maintenance mode, protocol 0, connector <version>" | neutral | the same as without the line (no Pair row: pairing is impossible) |
| Paired profile, connector in maintenance (`Maintenance` on login; e.g. `connector.toml` removed on the host with the key kept) | "Connector in maintenance mode" | failure, alert only after a press | trust block (stale); no Pair row, no job rows |
| An owner call or a confirm refused (`OwnerRequired`, the app thought it was an owner) | "This app is a member" | failure, alert (answers a press) | the role row turns to "Member" |
| Start with a stale job id (A14 `Error`) | "The connector holds another job" (as built) | failure | as built |
| Reset connector (new key) | "This connector's key changed" (as built, `0064` Q8) | failure | key changed block, Pair row |
| Token gone after a host-side reset that kept the key, or a revoke | "No longer paired" (as built) | neutral | Pair row |

- The app never shows the setup window, "already configured" or "window closed": setup is `curvyoctl`'s alone. The app sends no `ApplyConfig`, `IssueCode`, `ListClients`, `GetLog` or `Reset` in this slice.

## 9. Output group state line (`0125`)

The Output group's line reads the same status object (S14) and takes the same words as 8.3: "Start requested by Phone", "Resume requested by Phone at line 120", "Waiting for Marc to confirm", "Phone waits for Marc to confirm", "Not started: sign.gcode"; "Waiting for the start at the machine" stays for foreground mode. No button there (S15): an owner confirms in the Connector row or with `curvyoctl`.

## 10. Keyboard and focus

- No new Tab stop: "Belongs to" and "This app" are text. Confirm start is the Start slot inside the Job toolbar's roving focus (Left, Right, Home, End; Enter or Space presses).
- After Confirm start the slot greys (Running) and keeps focus, as built.
- No shortcut and no global chord for Confirm start (a laser must not start from a stray key, `0064` 3).
- Tooltips on keyboard focus as on hover.

## 11. Checks for the review

- **Contrast:** existing tokens only (`--toolbar-icon` 8.2:1, `--panel-muted-fg` 5.0:1, `--destructive` 4.8:1 on `--panel-bg`).
- **States to try** (demo of the spec, fake controller): owner app and member app side by side on one connector; member Start, owner Confirm start, member Stop of a waiting job, owner Stop of a member's waiting job (member sees "Not started"), member Resume after Pause (owner sees "Resume requested … at line n"), a slice-2 `clients.json` (Member), a connector without owner label, maintenance mode on Test connection, `curvyoctl reset` then the app (key changed).
- **`curvyoctl`:** every text of sections 4 and 5 against a loopback connector, `| cat` (no colour codes, same text), exit codes with `echo $?`, `--help` of every command fits 80 columns.
- **No hint texts** in the app, `curvyoctl` and the journal beyond the spec-fixed maintenance line.
- **Undo:** no connector action is a History step.

## 12. Decisions for veto (Proposal)

- **V1** No operating tips in `curvyoctl` errors (no "Try --help"), unlike today's server.
- **V2** All `curvyoctl` times in UTC, as the journal (no time zone dependency).
- **V3** Local failures (file, credential store) exit 1, no fourth code.
- **V4** `--fingerprint` needs all 32 hex characters; no prefix.
- **V5** `<connector>` may be left out while one connector is known.
- **V6** `curvyoctl` calls itself `curvyoctl@<host name>`.
- **V7** `identify` prints "Setup: open to the local network" during the window.
- **V8** `confirm-start` asks nothing and prints what it started; `<job name>` is an optional check.
- **V9** `stop` with no job exits 0 with "No job".
- **V10** `jobs` and `audit` default to 20 lines, `--last <n>`.
- **V11** A connection closed after accept is reported without a reason.
- **V12** Unknown connector error codes are named in `curvyoctl`.
- **V13** Trust block rows "Belongs to" and "This app", between "Paired with" and "Key".
- **V14** Confirm start and Confirm resume use the Start slot; no fifth button.
- **V15** Confirm start is not an armed action.
- **V16** "Not started: <name>" when someone else discards a member's waiting request.

## 13. Mismatches and open points

**For the product owner.**
- P1. A11's "That is the last owner; reset the connector instead" carries an operating tip; these notes use "Laptop is the last owner of workshop-laser". *Resolved in the spec (2026-11-18): A11 prints `Laptop is the last owner of workshop-laser` with `WrongState`, no operating tip.*
- P2. A23's "The key of <name> changed" and the other quoted refusals are lower case after the `curvyoctl: ` prefix; the words are kept. *Resolved in the spec (2026-11-18): A23 keeps the quoted refusals in lower case after the `curvyoctl: ` prefix.*
- P3. The demo's `--fingerprint XXXX-XXXX` reads as two groups; the argument takes all eight (V4). *Resolved in the spec (2026-11-18): A23 takes all 32 hex characters (spaces or hyphens allowed), A34 prints eight groups of four.*
- P4. Not in the spec yet: an owner app's "Start requested by <name>" and "Confirm start"/"Confirm resume" (A26 names "a Start button that confirms it"), "Not started: <name>", "Connector in maintenance mode", "This app is a member", the link line in maintenance mode, `jobs`/`audit --last`. *Resolved in the spec (2026-11-18): A26 and A34 carry these texts and `--last <n>`.*
- P5. `apply-config --insecure` to an address whose stored entry holds another key (after a host-side `--reset`) is refused by A23 with no way on short of editing `curvyoctl.toml`. Proposal: a `--fingerprint` that matches the presented key replaces the stored entry; a `forget` command is the alternative. PO and architect decide. *Resolved in the spec (2026-11-18): A23 replaces the stored entry when `--fingerprint` matches the presented key; no `forget` command.*
- P6. Length limits for `name`, the `owner` label and client names (A16): proposed 40, 40 and 32 characters so the 70-character lines hold. *Resolved in the spec (2026-11-18): A16 uses 1 to 64 characters for `name` and the `owner` label, 1 to 32 for client names.*

**For the architect (addendum).**
- A1. The login answer carries the client's own name and role, and the owner label; the app keeps role and owner label beside the connector name in the credential record.
- A2. During AwaitingLocalStart, `Status` reports the waiting time (e.g. `elapsed_ms` from the request); optional for the UX (8.3 works without it).
- A3. `admin.log` and `jobs.log` lines hold the fields of 4.8 in that order, so `curvyoctl` only aligns them.
- A4. A pairing name that is already taken: refuse (`curvyoctl` text in section 5) or replace.
- A5. In foreground mode `requested_by` is absent for the Enter wait and present for an owner wait, so the app picks the right words.

## 14. Design-system rows to add after acceptance

- **"Connector group (`0064`)"**: add the trust block rows "Belongs to" and "This app" (8.1), the Start slot's names "Confirm start" / "Confirm resume" for an owner, the AwaitingLocalStart words of 8.3 and the new link line texts of 8.4.
- **"Connector state line (`0125`)"**: the words of section 9.
- **New row "Command-line output (`curvyoctl`, `0128`)"** in a new short section "Command-line tools": section 1 (stdout and stderr, `curvyoctl: ` prefix, no colour, labelled lines and columns, UTC, exit codes 0/1/2, no tips), section 2 (one fingerprint form), and section 6's help layout as the pattern for every later CLI (the server's help follows it).
- "Keyboard shortcuts established so far": none added.
- No new token, no new glyph.

## 15. Questions for the customer

Fragen 250 (CONN-CONFIRM-NOTICE) A and 251 (CONN-CONFIRM-EXPIRY) A are decided by the customer (2026-11-18) as written in sections 8.3 and 9; Frage 252 (CONN-CTL-SECOND) B is decided and specified in section 4.13.

## Links

Spec: `specs/0128-connector-api-and-ctl/specification.md`. ADR 0071. Built notes: `specs/0064-machine-connector/ux-notes.md`, `specs/0125-send-to-machine/ux-notes.md`. Code: `frontend/src/components/workbench/ConnectorGroup.tsx`, `curvyo-connector-server/src/main.rs`, `curvyo-connector-server/src/args.rs`, `curvyo-connector-io/src/spki.rs`.
