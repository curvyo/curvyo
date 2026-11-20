# Plan for Connector API, `curvyoctl` and maintenance mode (0128, slice 5 of 0064)

Criteria A1 to A38. Design: ADR 0071 and the architect addendum D1 to D17 in
`specs/0064-machine-connector/adrs.md`; texts: `ux-notes.md`. Branch `story/connector-ctl` from
main. Built in three phases, each with its own tests, commits and gate run. No `format_version`
change of the project file; protocol stays 0, additive only.

## Affected crates/modules
- `curvyo-connector-core`: `message.rs` (new requests and answers, `Welcome.mode`,
  `Status.requested_by`, five error codes), `admin.rs` (`Role`, `LogName`, `ConnectorMode`,
  `ClientEntry`, `Secret`), `config.rs` (`ConnectorConfig` and its validation), `access.rs` (the
  D4 table), `machine/confirm.rs` (the D5 start rule, `Peer`, `Confirmation`, the job's people).
- `curvyo-connector-io`: feature `serial` (default on), `maintenance.rs` (`is_local`,
  `MaintenanceWindow`), `ports.rs`, `config_file.rs`, `clients.rs` (format 2), `admin_log.rs`,
  `log_tail.rs`, `job_log.rs` (three more fields), `state_dir.rs` (lock, reset, fingerprint),
  `hub/` (`mode.rs` setup and switch, `admin.rs` owner calls), `session.rs` and `session/`
  (`gate.rs` the admit and role, `owner.rs` owner calls), `supervisor.rs` (controller threads),
  `server.rs` (`start_service`, exit channel), `client.rs` (`Authenticated`, mode, `request`).
- `curvyo-connector-server`: `args.rs` (`--reset`, `--fingerprint`, foreground flags refused with
  `connector.toml`), `main.rs` (service start, exit channel, reset wipe), `dist/curvyo-connector.service`.
- `curvyo-connector-ctl` (new): lib with `run()`, bin `curvyoctl`.
- `curvyo-app`, `frontend`: role and owner label rows, Confirm start, waiting texts.
- Pairing codes (Proposal, vetoable at the demo): the connector holds one live code, so a newer code replaces an unused older one (single slot); the older code answers "wrong code" and counts one attempt of the newer one. Open question for the customer: telling a replaced code from a typing slip needs either a hint of the code on the wire (leaks bits, a security decision) or a text that names both causes ("wrong code, or replaced by a newer one") for every wrong code; neither is built. `curvyoctl login` and `apply-config --insecure` open the credential store before they send the code or the configuration, so a missing store changes nothing on the connector. `--reset` prints "Deleted the contents of <dir>".
- `deny.toml`, `docs/guides/connector-service-linux.md`, `docs/technical-debt.md`, design system, ADR 0011 note, `CLAUDE.md`/`AGENTS.md` line (Proposal).

## Tasks
### Phase 1: server side
- [x] 1. Core wire types, error codes and goldens (A2, A8, A10, A11, A17, A19, A32)
- [x] 2. Core `ConnectorConfig` with validation, JSON goldens and `config_failures.txt` (A16, A32)
- [x] 3. Core access table `admit` with its full-table test (A2, A3, A9, A13)
- [x] 4. Core start rule `OwnerConfirms`, `Peer`, confirmation output, people of the job (A12 to A15)
- [x] 5. `-io` feature `serial`; `is_local`, `MaintenanceWindow`, `ListPorts` (A2, A4, A5, A29)
- [x] 6. `-io` `connector.toml` file, `clients.json` format 2, `admin.log`, `jobs.log` fields, `GetLog` tail, lock and reset (A8, A15, A16, A18, A21, A31)
- [x] 7. `-io` hub modes, apply-config race, owner calls, revoke closes sessions, confirm flow, reset exit (A1 to A20)
- [x] 8. `-io` session gate, peer check, `Authenticated`, client `request` (A2 to A9, A26)
- [x] 9. Server flags, service start, unit file (A1, A19, A21, A28, A30, A31)
- [x] 10. Integration tests: loopback connector, recording controller, pty with zero bytes in maintenance (A3, A4, A6, A7, A29)
### Phase 2: `curvyoctl`
- [x] 11. Crate, deny exception, args and help (A22, A34)
- [x] 12. `curvyoctl.toml`, credential store, trust on first use (A23, A24, A36)
- [x] 13. Commands: identify, ports, apply-config, get-config, status, jobs, audit, confirm-start, stop, unlock, pair, clients, revoke, reset, login, logout, whoami, version (A22 to A25, A34 to A38)
- [x] 14. In-process tests against a real connector (A23 to A25, A34 to A38)
### Phase 3: app, frontend, docs
- [x] 15. App watcher forwards role and owner label; Confirm start; waiting texts; link lines (A26, A27)
- [x] 16. Frontend rows, Start slot names, tests; design-system rows (A26)
- [x] 17. Docs: service guide, debt entries, ADR 0011 note, `CLAUDE.md`/`AGENTS.md` line (A25, A28, A30)

## Validation
Unit tests for every pure part (table, config, window, local check, migration, log lines),
loopback-connector tests with the fake and a recording controller, a Linux pty test, `curvyoctl`
run in process against a real `RunningServer`; the full gate of `CLAUDE.md` §7 and `cargo deny`.

## Changes against the first plan (accepted by the lead, 2026-11-20)
- No `--config` flag: spec A31 and addendum D8 rule it out.
- A member's `Home` is refused with `OwnerRequired` under the owner rule: homing moves the machine
  and has no confirmation of its own (Proposal).
- `errors/codes.json` and `jobs.log` stay as they were; the new codes are in
  `errors/codes_0128.json`, the new `jobs.log` lines in `jobs_v2.log`, so the guards of earlier
  slices against changed fixtures hold.
- `Status` carries `job_name` and `sent_by` under the owner rule (configured mode only), so that
  `curvyoctl status` and `confirm-start` can name the job (UX notes 4.5, 4.6); additive, old goldens
  unchanged. The job's size is not on the wire: `status` prints "sign.gcode, 412 lines".
- `pair --name` decides the name a login is recorded under (addendum D7): a second computer is
  paired with `pair --name curvyoctl@<its host name> --owner`.
- `curvyo-app` takes `-io` without its `serial` feature.
