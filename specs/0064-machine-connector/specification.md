# Machine connector: a small program next to the machine that the app talks to, on the local network or through the sync service

Status: First slice (protocol core) Done (PR #375); second slice (device I/O on the LAN with a fake controller) Done (PR #375; Fragen 1 to 3 and CONN-HINT stand as Proposal defaults); slice 4 (GRBL over serial, `0127`) Done (PR #386); the spec as a whole stays open for the later slices (criteria 1 to 13 are not all covered; the rest stays a sketch)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "Machines are attached by a connector, which can be a very simple Rust service that compiles and runs from itself. It can then talk to a local instance in the network or via the sync service."

## User value

As a maker whose laser stands in a workshop and whose computer, or browser, is somewhere else, I want to run a small program on the machine that is plugged into the laser, and send a job to it from Curvyo and watch it run, so that I need neither a cable to my laptop nor a desktop-only app (`R-MFG-LASER-002`, `R-MFG-LASER-003`).

**What other tools do.** LightBurn: talks to the controller directly over USB serial or a network address from the same PC; "LightBurn Bridge" is a separate small program for a machine PC. OctoPrint (3D printers): a service next to the printer with a web interface and API; the slicer uploads to it over the network. Mainsail and FluidNC: the controller has its own web server. Carbide Motion and Easel: a local service in the browser's reach. Pattern: a small service near the machine, spoken to over HTTP or WebSocket; that is what the customer describes.

**Word clash.** ADR 0007 already has an "asset connector" (a client of a third-party asset library). This is the **machine connector**; the two are different things and use different words in code and UI.

## What it is (Proposal)

A single self-contained binary, written in Rust, that the maker downloads or builds (`cargo install`), starts on a computer next to the machine and leaves running. It holds the link to the controller, accepts jobs, runs them, reports status. The Curvyo app is a client of it. There are two ways to reach it:

- **Direct (local network):** the app connects to the connector's address on the LAN.
- **Through the sync service** (`curvyo-sync-server`, ADR 0004): the connector keeps an outbound connection to the sync service and the app reaches it from anywhere through the relay, end-to-end encrypted so the relay cannot read a job (ADR 0008).

## First slice: the protocol core (Ready, customer decision 2026-11-02)

**Invisible work.** This slice delivers a library, not something the maker can try. Per `CLAUDE.md` §4 it is commits on the branch `story/machine-connector` and has **no PR of its own**. The PR is opened with the first demoable slice (`-io`, `-server` and a fake-controller job visible in the app). **What changes in the demo: nothing.**

**Job text.** `0029` (G-code output) is not built. It does not block this slice: the protocol treats the job as opaque text of printable ASCII and newlines and never interprets it (ADR 0035 §7). Tests use hand-written text.

**Scope.** One new crate `curvyo-connector-core` (ADR 0035 §1): `#![forbid(unsafe_code)]`, no I/O, no clock, no threads; dependencies only `serde`, `serde_json`, `sha2`, `thiserror`, `uuid` (permissive, `cargo deny` clean; the PR of the demoable slice lists them). Its own `Millimetres` newtype for positions. Tested with a fake controller test double (a plain struct in the tests that stands for the machine side and feeds events to the state machine). Protocol version is the constant `0` (ADR 0035 §6).

Acceptance criteria of this slice (numbered `P1` to `P24` so that the later criteria 1 to 11 keep their numbers):

*Messages and encoding (ADR 0035 §5, §6)*

- P1. Given every message type of ADR 0035 §6 (`Hello`, `Pair`, `Auth`, `GetStatus`, `SubmitJob`, `JobChunk`, `Start`, `Pause`, `Resume`, `Stop`, `Welcome`, `Paired`, `JobAccepted`, `JobRejected`, `Status`, `Error`), when encoded, then it is one JSON object `{ "type": "<Name>", ... }`, and decoding the encoding returns an equal value (round trip).
- P2. Given a golden file per message type in `tests/fixtures/` (JSON text, committed), when a value is encoded, then the text equals the golden file byte for byte, and decoding the golden file returns that value. Field names and enum spellings (`Job`, `Frame`, `Busy`, `TooLarge`, `HashMismatch`, `NotAscii`, `NoController`, the `state` values and the `Done` results `Completed`, `Stopped`, `Failed`, the error codes) are fixed by these files.
- P3. Given `Status` with `job: null`, `position: null` and `alarm: null`, then the golden file shows them as JSON `null`, not omitted; a `Status` with a job and a position `[x, y, z]` in millimetres round-trips with the same numbers.
- P4. Given text that is not valid JSON, or JSON with an unknown `type`, or a known `type` with a missing or wrongly typed field, when decoded, then the result is a typed decode error (never a panic), and the caller can turn it into `Error { code, message }`; the codes for "malformed" and "unknown type" are distinct and fixed in a golden file (the connection stays open, ADR 0035 §6).
- P5. Given a frame (one message as text) longer than 1 MiB, when decoded, then it is refused with a "frame too large" error before parsing; a frame of exactly 1 MiB is accepted.

*Version check (ADR 0035 §6)*

- P6. Given `Hello { protocol, software, version }`, when `protocol` equals the constant, then the check passes; when it differs (higher or lower), then the check returns `Error { code: UnsupportedProtocol }` and the "close the connection" outcome, and no other message is accepted after it.
- P7. Given a message other than `Hello` as the first message, then the result is an `Error` (a fixed code, in a golden file) and the "close" outcome.

*Chunk assembly and job checks (ADR 0035 §5, §7)*

- P8. Given `SubmitJob { job_id, name, kind, bytes, sha256 }` followed by `JobChunk`s with indices `0, 1, 2, ...` whose texts together have exactly `bytes` bytes, when the last chunk arrives, then the assembled text equals the original, its SHA-256 matches `sha256` (lower-case hex), and the result is `JobAccepted { job_id }` with the state `Ready`. A job of `kind: Frame` is assembled the same way. A job announced with `bytes: 0` is complete on `SubmitJob` alone: no `JobChunk` is needed, and the empty text is checked against `sha256` at once (`JobAccepted` and `Ready`, or `HashMismatch` and `Idle`).
- P9. Given a job whose assembled text hashes to a different value than `sha256`, then the result is `JobRejected { reason: HashMismatch }` and the assembly is discarded (the state returns to `Idle`).
- P10. Given a chunk or a job text containing a byte outside printable ASCII (0x20 to 0x7E) and newline (`\n`, and `\r\n` counted as newline), then the result is `JobRejected { reason: NotAscii }`, discarded the same way.
- P11. Given `SubmitJob` with `bytes` above 64 MiB (67 108 864), then it is rejected with `TooLarge` at once, before any chunk; `bytes` of exactly 64 MiB is accepted. A chunk that would push the total above the announced `bytes`, or a chunk above 256 KiB (262 144 bytes), is rejected with `TooLarge` and the assembly discarded.
- P12. Given a `JobChunk` with an unexpected index (skipped, repeated or out of order) or with a `job_id` other than the job being assembled, then the job is rejected and discarded (the reason is `HashMismatch` for a gap or repeat; a chunk for an unknown `job_id` gets `Error`, the job in assembly is untouched). The chosen mapping is fixed by a golden file.
- P13. Given a job in any state other than `Idle` (assembling, ready, running, paused, awaiting local start), when another `SubmitJob` arrives, then it gets `JobRejected { reason: Busy }` and the held job is untouched. After a `Done` state the next `SubmitJob` is accepted.
- P14. Given the state `NoController`, when `SubmitJob` arrives, then it gets `JobRejected { reason: NoController }`.
- P15. Given the same job submitted twice in a row with the same `job_id` while the first is held, then the second gets `Busy` (no special case).

*Job state machine (ADR 0035 §6, §7)*

The state machine takes events (messages from the app, events from the controller double: connected, disconnected, started, finished, failed, alarm, local start pressed) and returns the new state and the messages to send. It has no clock; elapsed time and line counts come in with the events.

- P16. Given the transitions `NoController -> Idle` (controller connected), `Idle -> Receiving` (SubmitJob), `Receiving -> Ready` (assembly passed), `Receiving -> Idle` (rejected), `Ready -> Running` (Start, local start off), `Running -> Paused` (Pause), `Paused -> Running` (Resume), `Running -> Done { Completed }` (controller finished), `Running or Paused -> Done { Stopped }` (Stop), `Running -> Done { Failed }` (controller error), `any state with a controller -> Alarm` (alarm event, with its text), `controller lost -> NoController`; then a test walks each of them with the fake controller and checks the state, and a table-driven test asserts that every other (state, event) pair leaves the state unchanged and yields an `Error` or no message (the pair list is in the test).
- P17. Given `Stop` in `Idle`, `Ready`, `Done`, `Alarm` or `NoController`, or `Pause` while already `Paused`, or `Resume` while `Running`, then it is accepted without error and without a state change (idempotent, ADR 0035 §7); `Stop` in `Ready` or `AwaitingLocalStart` discards the held job and returns to `Idle`; `Stop` in `Running` or `Paused` ends in `Done { Stopped }` exactly once, and a second `Stop` changes nothing.
- P18. Given the option "require a local start" is **on** (CONN-SAFETY default A: an option, off by default), when `Start` arrives in `Ready`, then the state is `AwaitingLocalStart`, not `Running`; when the "local start pressed" event arrives, then it is `Running`; `Stop` in `AwaitingLocalStart` returns to `Idle`. With the option off, `Start` goes straight to `Running`. `Start` with a `job_id` that is not the held job gets `Error`.
- P19. Given a job running, when the app's connection is lost (a "client disconnected" event), then the state is unchanged and the job keeps running; a new connection that sends `GetStatus` gets the same `Status` (job id, line, lines), and a `Stop` from that new connection ends the job (ADR 0035 §7, criterion 6). A job still in `Receiving` is the exception: `ClientDisconnected` drops it and the state is `Idle` (Q16); `Ready`, `AwaitingLocalStart`, `Running` and `Paused` keep theirs.
- P20. Given `Status` output, then the state machine returns a `Status` message for every state change and for `GetStatus`; the `job` progress (`job_id`, `line`, `lines`, `elapsed_ms`) is carried only in `Ready` (line 0, elapsed 0), `Running`, `Paused`, `AwaitingLocalStart` and `Done`. Limiting the push rate to 5 per second is a timing matter of the transport and belongs to `-io`, not to core (core has no clock); this slice does not test it.

*Build and rules*

- P21. Given the crate, then `cargo build -p curvyo-connector-core --target wasm32-unknown-unknown` succeeds (added to the existing wasm32 check in CI, `CLAUDE.md` §6).
- P22. Given the crate, then it contains `#![forbid(unsafe_code)]`, depends only on `serde`, `serde_json`, `sha2`, `thiserror` and `uuid` and on no `document-core`, UI, platform or I/O crate (checked by the architect review and `cargo tree`), and has no `unwrap`/`expect` outside tests without an `// invariant:` comment.
- P23. Given the fake controller test double, then it is used by at least the tests of P16 to P20 (so the "test double" the rules allow exists and is used), and no trait is added to production code only for it.
- P24. Given the quality gate of `CLAUDE.md` §7, then it is green on the branch; the golden files are in `tests/fixtures/` of the crate.

**Out of scope for this slice:** sockets, WebSocket, TLS, serial, GRBL streaming, pairing logic (`Pair`, `Auth`, `Paired` exist as message types with golden files only; the code check, token storage and `PairSpake` are `-io`), the status rate limit, UI, the machine profile, any change to `src/` outside the new crate, workspace wiring beyond adding the crate and its CI wasm line.

**Follow-up slices.** Criteria 1 to 11 below stay as written, with their numbers. The next slice ("Second slice" below, criteria `Q1` to `Q22`: `-io` and `-server`, WebSocket over TLS, pinning, pairing, a fake controller job visible in the app) is the first with a PR. Criterion 11 is met by this slice for the message types (the connection code stays open).

## Second slice: the connector on the LAN with a fake controller (Ready, 2026-11-17)

Customer decision of 2026-11-02: after the protocol come device I/O, pairing and encryption, the relay later. This slice is the first one the maker can try end to end and carries the **PR of the whole branch** (`story/machine-connector`, with the protocol core commits).

**One-line meaning (Proposal, Frage 2):** *Send job* copies the text of a G-code file to the connector; the connector holds it and runs it only after *Start*. Curvyo generates nothing and changes nothing in the text (`0029` is not built).

**What changes in the demo.**
- In the Machine block of a laser manufacturing (`0026`), a **Connector** row appears under the connection fields: a connection-test symbol button with its result, a Pair field for the code, and, once paired, send, start, pause, resume and stop symbol buttons with a status line and a progress bar.
- On the maker's computer (or the same one in a second terminal) the maker runs `curvyo-connector-server --fake-controller`: it prints its address, a pairing code and the key fingerprint. The fake controller streams the job's lines at a steady, slow pace, so progress, pause and stop can be watched. **No laser is touched; nothing moves.**
- **Send job** takes any printable-ASCII text file; `tests/fixtures/connector/sample-job.gcode` (about 400 lines of a square spiral) is the file for the demo.
- Nothing else in the app changes. The project file does not change (no `format_version` bump).

**Boundary (Proposal).**
- In: crates `curvyo-connector-io` and `curvyo-connector-server` (ADR 0035 §1); WebSocket over TLS 1.3, self-signed key pinned at pairing, pairing code with SPAKE2, token; the desktop app's side (Rust in `-io`, Tauri commands, UI in the Machine block); a **fake controller** (a plain struct in `-io`, started by a flag, no trait: the trait comes with the second controller, serial/GRBL, `CLAUDE.md` §5); local start option; local job log; the status rate limit of P20.
- Out: serial and GRBL, the `Frame` job in the UI (the message type exists, no button), the config file (this slice has command line flags and a state directory), EST, the web app served by the connector, the relay, the browser build, discovery, G-code generation (`0029`).

**Where things are kept.** The connector address is the field of `0026` criterion 9 (no change there). The pinned key, the token and the connector's name (so the row can show it offline) are **per user on this computer**, kept in the OS credential store (ADR 0007, ADR 0035 §4, addendum A8), one record per machine profile id, never in a profile record and never in the project file. Forget pairing, a new pairing and deleting the user profile remove or replace its record (Q10a).

Acceptance criteria of this slice (prefix `Q`, so that the later criteria 1 to 13 and `P1` to `P24` keep their numbers). "Demo setup" below means: connector started with `--fake-controller` on `127.0.0.1` (or a LAN address) and the desktop app with a user profile whose connector address is that address.

*The connector program (criteria 1, 2, 3, 10)*

- Q1. Given `curvyo-connector-server --fake-controller [--listen <addr:port>] [--state-dir <dir>]` (default port 7447, Proposal; default state directory in the user's data directory), when started on Linux without a display and without an account, then on its first start (and on `--pair`) it prints its address, its pairing code (Q2) and the SHA-256 fingerprint of its key as 8 groups of 4 hex characters, and listens. `--help` lists every flag. It makes no outbound connection and sends nothing anywhere but to a paired client (`R-SYS-007`; checked by review: the server never calls the client module of `-io`, and its dependency tree has no HTTP client crate; `rustls` and `tungstenite` carry client code themselves, so "no client code in the tree" cannot be the check).
- Q2. Given the pairing code, then it has 8 characters from a 32-symbol alphabet (Crockford base32: `0`-`9` and `A`-`Z` without `I`, `L`, `O`, `U`; `0` and `1` are present). Typed input ignores case, `-` and spaces and reads `O` as `0` and `I`/`L` as `1`, is valid 10 minutes, works once, and is spent after three wrong attempts; `--pair` prints a new code. A second code request while one is valid replaces it. Tests drive the expiry with an injected clock value, not the wall clock.
- Q3. Given the first start, then an ECDSA P-256 key and a self-signed certificate (10 years) are made and saved in a `0600` file in the state directory; given a restart with the same directory, then the key is the same (same fingerprint) and a paired app connects without pairing again; given a state directory with a damaged key file, then the connector refuses to start with a message naming the file (no silent new key).
- Q4. Given a client that offers TLS 1.2 or lower, or plain TCP or plain HTTP, then the connection is refused and no message is read (test with a raw socket and a TLS 1.2-only client).

*Pairing and trust (criteria 2, 3)*

- Q5. Given an unpaired app and the right code, when the maker types it in the Pair field and confirms, then the app and connector run the SPAKE2 exchange of ADR 0035 §4 over the TLS connection (the two `PairSpake` frames, defined in `-io`, with golden files in `tests/fixtures/`), the row shows "Pairing" while it runs, the app pins the SPKI hash of the key it saw, stores the token and the connector name, and the row shows "Paired with <connector name>, key <fingerprint>". The code is spent; typing it again gets a visible "code not valid" in the row.
- Q6. Given a wrong code, then the row shows "Wrong code, <n> attempts left" (inline, no popup) and nothing is stored; after the third wrong attempt it shows "Code spent"; an expired code shows "Code expired".
- Q7. Given a man in the middle that terminates TLS with its own key and relays the frames (a test proxy in `-io` tests), then pairing fails, the app stores and pins nothing, and the connector counts one wrong attempt; the row shows the wrong-code text of Q6. SPAKE2 cannot tell a wrong code from a different key (both give a different session key, ADR 0035 §4), so no separate "key mismatch" text exists; the test asserts the failure, the empty credential store and the attempt count.
- Q8. Given a paired connector whose key changes (state directory replaced), when the app connects or tests, then it refuses with "This connector's key changed" and shows both fingerprints (stored and presented); it sends no token. The pair action is then offered again; accepting is only possible by pairing anew with a code. Nothing is accepted silently.
- Q9. Given a client that is not paired, then every message except `Hello`, `Pair`, `PairSpake` and `Auth` gets `Error { code: PairingRequired }` and no job state changes. `--list-clients` prints name and date of every paired client, `--revoke <name>` removes one; a revoked token gets `PairingRequired` and the app row shows "No longer paired" (the Pair row is the way on). The connector stores only a SHA-256 hash of each token (tested by reading the state directory).
- Q10. Given the pinned key, token and address, then **none** of them appears in the project file or in the machine-profile record (a test saves both and searches the bytes for the token and the fingerprint); the project file and the profile library are unchanged by this slice (`format_version` stays).
- Q10a. Given a paired profile, when the maker presses **Forget pairing** (armed action, second press acts), then the record `connector/<profile id>` is removed (`CredentialStore::remove(name)`, new in `curvyo-storage-io`, tested there with the in-memory and the file store), the row shows "Not paired" with the Pair row; given the maker deletes that user profile, then its record is removed too (tested: delete the profile, the record is gone); a "Not in your profiles" copy has no id and no record. The record keeps `spki_sha256`, `token` and `connector_name` (golden file, addendum A8). Given the store cannot remove the record, then the pairing stays shown and the row says "Could not forget the pairing" (Proposal text). Given this computer has no credential store, then the row says "No credential store on this computer" and the Pair row is not shown, as pairing cannot work (Proposal text). Given an error without a text of its own, then the row says "Connector error" and the detail goes to the log only (Proposal text).

*Connection test (criterion 4)*

- Q11. Given a user profile with a connector address, when the maker presses the **Test connection** symbol button, then within 5 s the row shows one result line, no popup, "Testing" while it runs: "Connected, controller answered: <controller name>, protocol 0, connector <version>" on success; otherwise exactly one of: "Not a valid address" (the text is none of `host`, `host:port`, `[v6]:port`; a missing port means 7447, the connector's default listen port), "Nothing answers at <address>" (refused, or a name that does not resolve; <address> as dialled, with the port), "Not paired" (with the Pair field), "No longer paired", "This connector's key changed" (Q8), "Protocol mismatch: app 0, connector <n>", "Timed out". While running, the button is disabled and shows a busy state. A saved profile with an empty connector address shows "No connector address" and Test connection is greyed (not hidden). The built-in profile and a "Not in your profiles" copy show the row unavailable with the reason, as `0026` does for the connection fields.
- Q12. Given a successful test, then the connection is closed afterwards (the test leaves nothing running on the connector), and the result is not stored.

*Job, status, control (criteria 6, 9, 10)*

- Q13. Given a paired, reachable connector in `Idle` and a profile with a connector address, when the maker chooses **Send job** and picks a text file (Frage 1), then the app sends `SubmitJob` and the chunks of P8, shows "Sending <name>", then "Ready: <name>, <n> lines, <size>". A file that is not printable ASCII, is over 64 MiB, or is refused otherwise shows the rejection in words ("Not plain ASCII text", "Larger than 64 MiB", "Hash mismatch", "Connector busy", "No controller"), nothing is held, and the connector's job log records the rejection (Q18). A file that cannot be read (missing, no permission) shows "Could not read the file", sends nothing and is not shown as a lost connection.
- Q14. Given a job in `Ready`, when the maker presses **Start**, then (local start off) the state is `Running` and the fake controller streams the lines at 20 lines per second (Proposal; `--fake-lines-per-second <n>` overrides it for tests). The row shows state in words, `line / lines`, elapsed time and a progress bar; the status is pushed on every change and at most 5 times per second (P20's rate limit, tested with a counted burst of fast lines). *(Amended by `0118`: the job buttons, the state line and the bar named "the row" here live in the Machine controls footer of the Operations tab; Test connection, Pair and Forget pairing stay in the Connector row.)*
- Q15. Given a running job, when the maker presses **Pause**, then the state is `Paused` and the line count stops; **Resume** continues from the same line; **Stop** ends it as `Done { Stopped }` and the row shows "Stopped at line <n>"; a command naming a job the connector does not hold shows "The connector holds another job". At the end of the file the row shows "Completed, <n> lines"; a job that ends in a controller error shows "Failed at line <n>" (log result `Failed`, Q18). Each button applies only in certain states and is a symbol button with a tooltip naming it (`docs/design-system.md`); a button that does not apply is greyed and keeps its place (Proposal, vetoable at the demo; UX notes sections 3 and 7), a press does nothing and sends nothing. *(Amended by `0118`: the job buttons, the state line and the bar named "the row" here live in the Machine controls footer of the Operations tab; Test connection, Pair and Forget pairing stay in the Connector row.)*
- Q16. Given a running job, when the app is closed or its connection cut, then the job keeps running on the connector; given the app started again and the connector's address in the profile, then **Test connection** or opening the Machine block reads `Status` and shows the job's progress, and **Stop** works from that fresh connection (P19, criterion 6; test: kill the app side, reconnect with a second client). While the block is shown and the connection drops during an active job (Receiving, Ready, AwaitingLocalStart, Running, Paused), the row shows "Connection lost", keeps the last values and the app retries every 5 s (retry period injected in tests); a button press opens a fresh connection first, so Stop works then. **A job still being received dies with its connection**: when the connection that sent the `SubmitJob` ends before the last chunk, the connector drops the half-received job and returns to `Idle` (core event `ClientDisconnected`; a job that is `Ready`, `AwaitingLocalStart`, `Running` or `Paused` keeps its state; test: cut the connection mid-transfer, a second `SubmitJob` is accepted). **Start works from a fresh connection too** (the job id comes from the `Status` in `Ready`; **Proposal**). *(Amended by `0118`: the job buttons, the state line and the bar named "the row" here live in the Machine controls footer of the Operations tab; Test connection, Pair and Forget pairing stay in the Connector row.)*
- Q17. Given `--require-local-start`, when the maker presses Start, then the row shows "Waiting for the start at the machine" (state `AwaitingLocalStart`), the connector prints "Press Enter to start <name>", and the job runs only after Enter on the connector's terminal; **Stop** in that state discards the job. Without the flag, Start runs at once. The setting is on the connector only, not in the profile (Proposal: it must not be changeable from the app, otherwise the safety is void). *(Amended by `0118`: the job buttons, the state line and the bar named "the row" here live in the Machine controls footer of the Operations tab; Test connection, Pair and Forget pairing stay in the Connector row.)*

*Log, alarm (criterion 10; follow-ups 12, 13)*

- Q18. Given any job outcome (completed, stopped, failed, alarm, or rejected on submit), then the connector appends one line to `jobs.log` in the state directory: `<UTC time RFC 3339> <TAB> <job name> <TAB> <result>` (`Completed`, `Stopped`, `Failed`, `Alarm: <text>`, `Rejected: <reason>`); a golden file in `tests/fixtures/` fixes the format. Nothing else is logged to a file, and the log never leaves the connector.
- Q19. Given `--fake-alarm-at-line <n>`, when the running job reaches line n, then the fake controller raises an alarm with a text, the streaming **stops** (follow-up 12: core only changes state, `-io` stops the stream), the row shows "Alarm: <text>" and the log has `Alarm: <text>`. Follow-up 13 is settled this way (Proposal): `SubmitJob` in `Alarm` keeps answering `Busy` (the protocol and its golden files stay as built) and the app explains it from the `Status` alarm text ("Machine in alarm: <text>"); a later slice may add a reason.

*Build and rules*

- Q20. Given the crates, then `curvyo-connector-io` and `curvyo-connector-server` follow ADR 0035 §1 and §2 (`rustls` with `ring`, TLS 1.3 only; `rcgen`; `tungstenite`; `spake2`; every licence in `deny.toml`, `cargo deny` green; each dependency listed with licence and reason in the PR), `-core` stays free of them and still builds for wasm32; `-io` has no UI crate, the server is thin (arguments, wiring, terminal input); `anyhow` only in the server; no trait for the fake controller (a concrete struct in `-io`).
- Q21. Given an integration test in `curvyo-connector-io/tests/` that starts the server on `127.0.0.1:0` with the fake controller and drives a client through pair, test, send, start, pause, resume, stop, reconnect and revoke, then it passes in the quality gate (the demo's end to end path without the UI). Timing is by line counts and injected values, never by wall-clock sleeps that decide the result.
- Q22. Given the quality gate of `CLAUDE.md` §7, then it is green, and the demo path (Demo setup, Q5, Q11, Q13 to Q15) was tried once in the running desktop app by the tester.

**Out of scope for this slice (also in the lists above):** serial, GRBL, `Frame` button, config file, EST, web app served by the connector, relay, browser build (the Connector row shows "Needs the desktop app" there and G-code export is unaffected), several connectors per profile, several jobs, discovery, G-code generation, any change to `format_version`.

**UX notes needed (not written here).** The `ux-engineer` adds `ux-notes.md` for: the Connector row inside the Machine block of the Laser tab (position, grouping with the connection fields of `0026`, widths in the narrow panel); the symbol buttons and glyphs for test, pair, send, start, pause, resume, stop, forget; every inline state text of Q5 to Q8, Q11, Q13 to Q17, Q19 (wording, colour use for error and alarm, where the key fingerprint and the two fingerprints of Q8 are shown); the progress bar and the elapsed time format; the file picker for Send job; disabled and busy states; keyboard use and tab order; the unavailable state in the browser; the demo-only fake-controller wording on the connector's terminal (not UI). No hint or tip labels (labels naming a control or state only).

**Questions (Proposal defaults, vetoable at the demo; none blocks).** Fragen 1 to 3 and CONN-HINT stand as the defaults below (built unless the customer vetoes at the demo): CONN-JOBSRC A, CONN-SEND-MEANING A, CONN-LOCALSTART-WHERE A, CONN-HINT A. The UX Proposal "job buttons, Test connection and Pair are greyed, not hidden" (UX notes section 3, a recorded exception to "Hidden, not disabled") is also vetoable at the demo.

Frage 1 (CONN-JOBSRC): Where does the job text come from before `0029` exists?
- A (default, recommended): the maker picks a text file (`.gcode`, `.nc`, `.txt`) with Send job; a small sample file is committed for the demo: `tests/fixtures/connector/sample-job.gcode`.
- B: a built-in sample job (a square) sent by the button, no file.
- C: both.
- Default if unanswered: A.

Frage 2 (CONN-SEND-MEANING): Is the one-line meaning above right, that Send job only copies the text to the connector and Start is a separate press?
- A (default, recommended): yes, two steps; the maker sees "Ready" before anything runs.
- B: Send job starts the job at once.
- Default if unanswered: A.

Frage 3 (CONN-LOCALSTART-WHERE): Where is "require a local start" set?
- A (default, recommended): on the connector only (`--require-local-start`); the app cannot change it, so a remote client cannot disarm it.
- B: a switch in the machine profile, sent to the connector.
- Default if unanswered: A.

Frage 4 (CONN-HINT): Q6 and Q9 first had operating tips after the state ("..., start the connector with --pair", "..., pair again"; customer rule of 2026-10-16: no hint labels).
- A (default, recommended, applied): the short state texts "Code spent" and "No longer paired".
- B: keep the longer sentences as a recorded exception.
- Default if unanswered: A.

**Done before Ready (2026-11-17):** (1) `ux-notes.md` by the ux-engineer; (2) an addendum to `adrs.md` by the architect: the pinned-key TLS verifier on the app side, the SPAKE2 transcript and frame definitions, the credential-store record per profile id (ADR 0007), the fake controller as a concrete struct, the state directory layout, CLI flags, the `-io` module split, `cargo deny` result for the new dependencies; (3) the defaults of Fragen 1 to 3 and CONN-HINT (above). No ADR is new: ADR 0035 already covers crates, transport, trust and pairing.

## Acceptance criteria of the later slices (as far as clear, not Ready)

1. Given the connector program, then it runs without the desktop app, without a display and without an account, on Linux first (a Raspberry Pi or a workshop PC), with a command line and a configuration file; it needs no installer.
2. Given its first start, then it prints a **pairing code** and its address. The app pairs once by typing the code; afterwards the app knows the connector's key (pinned) or trusts its CA certificate (EST), holds a pairing token, and the code is spent (CONN-SEC, ADR 0035). Nothing is sent to a connector that is not paired.
3. Given pairing, then the connection is encrypted on the LAN (no plain HTTP with a token) and the app remembers the connector by its key, so a changed key is shown as a changed key, not silently accepted.
4. Given the app, then the machine profile (`0026`) names the connector (direct address or relayed id) and the controller port; a connection test is a symbol button in the profile with a visible result (connected, controller answered, version), no popup.
5. Given the first connector (decided: GRBL over serial), then it speaks **GRBL over serial** to one controller: it reports status and position, accepts a G-code job, streams it with the controller's flow control, pauses, resumes and stops it, and can run the **frame** (outline at low power, `R-MFG-LASER-003`).
6. Given a job in progress, then the app shows progress and status from the connector, and a lost connection never leaves the job in an unknown state: the connector keeps running the job and the app reconnects and reads the state; stop works even from a fresh connection.
7. Given the browser build (`R-SYS-005`), then it reaches a connector the same way (network), because it has no serial access of its own; export of the G-code file stays always available.
8. Given the sync service as the path, then the connector's outbound link is the only open connection; no inbound port needs to be opened on the workshop network.
9. Given safety, then a machine can be set to **require a local start**: the job is accepted and shown at the connector (a button or a key on the connector's host), and runs only after that (CONN-SAFETY, Frage 2 below). The default is off; it is the maker's setting (permit over refuse).
10. Given logs, then the connector keeps a local log of jobs (time, name, result) and sends nothing else anywhere. No telemetry (`R-SYS-007`).
11. Given the code, then the message types (job, status, commands) are plain data in a `*-core` crate that builds for wasm32 (so the browser app and the connector share them); the connection code (serial, sockets, TLS) is in an `-io` crate; the connector binary is thin.
12. Follow-up from the architect review of the protocol slice: an `Alarm` while `Running` must make the streamer stop (core only changes the state), and the job log (criterion 10) needs a result for that job.
13. Follow-up from the architect review: `SubmitJob` in `Alarm` answers `Busy` although no job is held; decide the right reason when the `-io`/server slice is built.

## Out of scope

- Other controllers (Ruida, Trocen, Marlin, Smoothieware, plotters, embroidery machines): each becomes its own entry, one family at a time (`CLAUDE.md` §6). A connector for them may be an extension (`0065`).
- USB without serial emulation, camera feeds, firmware update, machine-side jog panel beyond frame, job queues for many users.
- An accounts system or a hosted connector service; the sync service already has the accounts it has.
- Discovery by multicast (Proposal for later: the pairing address is typed or scanned).

## Architect notes (2026-10-17)

- New crates need an ADR (`CLAUDE.md` §5): a protocol core crate, an `-io` crate for GRBL/serial and the transport, and a `-server`-style binary. Crate naming needs the exception of `CLAUDE.md` §8 extended (the connector is a binary like `curvyo-sync-server`). **`needs-customer`** (data leaves the machine over the relay; security model).
- Decided in ADR 0035: WebSocket over TLS, the job as G-code text, pairing by code; the document keyring of ADR 0010 is not reused.
- The connector holds no document and no user data beyond the job in flight.

## Decided (customer, 2026-11-02)

Customer (translated): "0064 also becomes relevant, at least the protocol between connector and workbench." Scope order, still a sketch:

1. **Next:** the protocol between connector and the Laser workbench (`0063`, `0093`): the message types (job, status, commands, frame) as plain data in a `*-core` crate that builds for wasm32 (criterion 11), with golden-file tests of their encoding. No sockets, no serial, no pairing yet.
2. **After:** device I/O (GRBL over serial, the thin connector binary), pairing and encryption, the relay path. These keep their criteria 1 to 10 and the open questions below.

The protocol slice is scheduled (2026-11-16) without waiting for `0029`: the job is opaque printable-ASCII text to the protocol. ADR 0035 is accepted. It is built as invisible commits on `story/machine-connector`, no PR of its own (see "First slice").

## Decided (customer, 2026-11-02, answers to ADR 0035)

- **CONN-SEC (Frage 120): B, TLS.** Customer (translated): "B, with public key pinning and pairing code, or an EST client, i.e. a valid certificate." The connection is TLS 1.3. Two ways to trust the connector: (1) its self-signed certificate's public key is pinned at pairing, and the pairing code authenticates that first contact; (2) the connector obtains a valid certificate from the maker's CA with an EST client (RFC 7030), validated normally. No Noise channel. ADR 0035 §2 to §4.
- **CONN-BROWSER (Frage 121): A and B.** The browser reaches a connector through the relay once the sync service exists, **and** the connector serves the Curvyo web app itself on the LAN, so the page talks to it same-origin. With a self-signed certificate the browser warns once and cannot pin; with an EST certificate from a CA on the maker's device it does not warn. ADR 0035 §8.

## Decided (customer, 2026-10-17)

- **CONN-FIRST: A.** The first connector talks GRBL over serial (USB), reached directly on the LAN first (the CONN-PATH default).

## Open questions (customer; each has a default)

Frage 1 (CONN-PATH): Which route comes first, direct on the LAN or through the sync service?
- A (default, recommended): direct on the LAN first; it needs no server and is testable at once. The relay route follows when the sync service exists.
- B: relay first, because the browser and remote use are the main reason.
- Default if unanswered: A.

Frage 2 (CONN-SAFETY): Is a local start confirmation (criterion 9) built in the first version?
- A (default): yes, as an option, off by default.
- B: always required for a remote start.
- C: not built yet.
- Default if unanswered: A.

## Links

Requirements: R-MFG-LASER-002, R-MFG-LASER-003 (existing); R-MFG-011 (new)
Depends on: `specs/0026` machine-profile (reserved), `specs/0029` laser job output (reserved, the G-code), ADR 0004 (sync service), ADR 0008 (end-to-end encryption), ADR 0010 (keyring)
Related: `specs/0063-manufacturing-workbenches/`, `specs/0065-extensions-sdk/`, ADR 0007 (the other "connector")
ADRs: ADR 0035 (protocol, security, pairing, crates), accepted by the customer 2026-11-02
PR: - (the first slice has none; the PR comes with the first demoable slice)
