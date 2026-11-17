# Plan for the machine connector, first slice (protocol core)

Invisible work: commits on `story/machine-connector`, no PR of its own (spec, "First slice").

## Affected crates/modules
- New `curvyo-connector-core` (ADR 0035 §1), added to the workspace members. Modules, one
  responsibility each: `millimetres`, `message` (types), `codec` (frames, errors), `handshake`
  (first message, version), `assembly` (chunks, hash, ASCII), `machine` (job state machine).
- Root `Cargo.toml`: workspace dependency `uuid` 1 (Apache-2.0 OR MIT, already in `Cargo.lock`; checked with `cargo deny`),
  `serde`, `serde_json`, `sha2` already there.
- CI: the `core-wasm32` job discovers `*-core` crates by name, so no CI edit is needed (P21).

## Tasks
- [x] 1. Crate skeleton, workspace wiring, `Millimetres`, message types with serde (fulfils P1, P3, P22)
- [x] 2. Golden files for every message type and error code; codec with typed errors and the 1 MiB
        frame limit, tests first (P1, P2, P3, P4, P5)
- [x] 3. Handshake: `Hello` first, version constant 0, close outcome (P6, P7)
- [x] 4. Job assembly: chunks, hash, ASCII, limits, mapping golden, tests first (P8 to P12)
- [x] 5. State machine with `Event`/`Output`, fake controller test double in `tests/common`,
        table-driven pair test (P13 to P20, P23); `ClientDisconnected` in `Receiving` drops
        the half-received job (P19, Q16)
- [x] 6. wasm32 build, `cargo deny`, full gate (P21, P22, P24)

## Validation
Golden files in `curvyo-connector-core/tests/fixtures/` (encoding, error codes, the chunk-failure
mapping); a table of every (state, event) pair; `cargo build --target wasm32-unknown-unknown`;
`cargo tree` shows only `serde`, `serde_json`, `sha2`, `uuid`, `thiserror`.

---

# Plan for the second slice (the connector on the LAN)

Criteria Q1 to Q22. This slice carries the PR of the whole branch. Design: the architect addendum
in `adrs.md` (A1 to A12); UI: `ux-notes.md`. Commits: up to ~8 this time (two new crates, the
app wiring and the frontend are each a logical step; `CLAUDE.md` §8 names 1-5 as the norm).

## Affected crates/modules
- `curvyo-storage-io`: `CredentialStore::remove` (A8).
- New `curvyo-connector-io` (library, modules per A12), new `curvyo-connector-server` (binary:
  `main`, `args`, `terminal`). Both in the workspace members, `deny.toml` exceptions.
- `curvyo-app`: `connector.rs` (Tauri commands over the `-io` client, the credential record per
  profile id, status events); `frontend/`: the Connector group of the Machine block.
- Deviations from the addendum are listed in the report: the connector sends its own `Hello`
  first (so the app learns protocol and version, Q11); the fingerprint is printed upper-case (UX
  notes section 5), stored as lower-case hex; WebSocket size limit is 2 MiB so that a frame
  between 1 and 2 MiB gets `FrameTooLarge` with the connection kept open (P5).

## Tasks
- [x] 1. `CredentialStore::remove(name)`, tests for the file and the in-memory path (Q10a)
- [x] 2. `-io` trust primitives: `spki`, `identity` (+ damaged-file fixtures), `server_tls`,
        `pairing_code` (injected `Instant`), `frames` + golden files, `spake` + known answer
        (Q1, Q2, Q3, Q5, Q20)
- [x] 3. `-io` state: `clients` (golden), `job_log` (golden), `fake_controller`, `status_limit`
        with a counted burst (Q9, Q14, Q18, Q19)
- [x] 4. `-io` server: `hub`, `session`, `server`; auth gate, TLS 1.3 only, raw-socket tests,
        local start, alarm stop, log lines (Q1, Q4, Q9, Q13 to Q19)
- [x] 5. `-io` app side: `pin_verifier`, `client`; key changed, wrong code, man-in-the-middle
        proxy, test connection, reconnect, a connection cut mid-transfer frees the connector
        (Q5 to Q8, Q11, Q12, Q16)
- [x] 6. `curvyo-connector-server`: `args`, `terminal`, `main`, `--help`, one-shot commands,
        process tests (Q1, Q9, Q17, Q18, Q19, Q20)
- [x] 7. `curvyo-app` commands and the credential record golden; project and profile bytes do
        not contain pin or token (Q10, Q10a, Q11 to Q16)
- [x] 8. Frontend Connector group: states, buttons, "Needs the desktop app" (Q5 to Q8, Q11 to
        Q17, Q19; UX notes 2 to 10)
- [x] 9. `curvyo-connector-io/tests/end_to_end.rs`: pair, test, send, start, pause, resume, stop,
        reconnect, revoke (Q21)
- [x] 10. Full gate, `cargo deny check`, wasm32 build of `-core` (Q20, Q22 minus the tester's run)

- Built with 10 commits (the norm is 1-5): two new crates, the app wiring and the frontend are each a
  logical step. More deviations: `Start` needs the
  job id, which core's `Status` does not carry in `Ready`, so the app keeps the id of the job it sent
  (a job sent from another computer can be stopped but not started here: follow-up for the architect);
  "No credential store on this computer" and "The connector holds another job" are new texts
  (Proposal); the host's event name lives in `main.rs` beside the command list because
  `tests/acceptance_0051_tester.rs` reads only `main.rs` and `project.rs` for emitted events.

## Validation
Golden files in `-io/tests/fixtures/` and `curvyo-app/tests/fixtures/`; raw-socket tests for Q4
(a committed TLS 1.2 ClientHello); a man-in-the-middle proxy test for Q7; the end-to-end test of
task 9; timing by line counts and injected `Instant` values only. The tester drives the running
app.
