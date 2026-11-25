# ADR 0035: Machine connector protocol

**Status:** Accepted (customer, 2026-11-02: Frage 120 CONN-SEC B with key pinning, pairing code or
an EST certificate; Frage 121 CONN-BROWSER A and B). `needs-customer` (data leaves the machine:
jobs travel over the local network; security model). Spec `0064` (decided: CONN-FIRST A, GRBL over
serial; CONN-PATH default A, direct on the LAN first). Scope: the protocol between the Laser
workbench and the connector, its security and the crates. Serial and GRBL streaming are deferred to
their own story; the relay route follows the sync service (ADR 0004 §5).

## Context

The connector is a small headless program next to the laser (spec `0064`). The app sends it a
job and watches it run. What must not be cheap to get wrong: the message types (both sides ship
separately, so a mismatch must be detected, not guessed), and the security of a channel that can
fire a laser. The job is G-code text produced by `curvyo-laser-core` (`0029`), so the connector
needs no CAM code and no document.

### Options: transport

- **A. WebSocket (chosen).** One long-lived, bidirectional connection; status is pushed; browsers
  speak it; the same transport later runs through the relay. Rust: `tungstenite` (MIT/Apache-2.0).
- **B. HTTP request/response plus polling.** Simple, but status needs polling and a stop must not
  wait behind a poll. Rejected.
- **C. A raw framed TCP protocol.** No browser path ever. Rejected.

### Options: security on the LAN

- **A. TLS 1.3 (`wss://`), connector key pinned at pairing or a CA-issued certificate (chosen by
  the customer, Frage 120 B).** Standard, inspectable with ordinary tools, and the only option a
  browser can speak (option B of Frage 121 needs it).
- **B. Pairing code, then a Noise channel over the WebSocket.** No certificates, but browsers
  cannot speak it and it is our own construction. Rejected by the customer.
- **C. Plain WebSocket with a token.** Sniffable on any shared workshop network; violates spec
  `0064` criterion 3. Rejected.
- **D. Reuse the document keyring of ADR 0010.** A machine is not a participant of a document and
  must not see its keys. Rejected.

### Options: how a client proves it is paired

- **A. A random token issued at pairing, sent inside TLS on every connection (chosen).** The server
  is already authenticated by TLS, so a bearer secret is enough; works the same in the desktop app
  and the browser.
- **B. Client certificates (mutual TLS).** Browsers cannot hold an app-made client certificate
  without the maker installing it by hand. Rejected.
- **C. A client key pair and a signed challenge.** Same guarantee as A behind authenticated TLS,
  more parts. Rejected.

## Decision

1. **Crates** (named here, created when their story starts, `CLAUDE.md` §5):
   - **`curvyo-connector-core`** (first, with the protocol story): message types, protocol version
     check, the job state machine (events in, state out) and job assembly from chunks with a hash
     check. Pure, `#![forbid(unsafe_code)]`, builds for wasm32 (the browser route uses it).
     Edges: external only (`serde`, `serde_json`, `sha2`, `uuid`); no `document-core`, so the
     connector binary carries no document model. Its own `Millimetres` newtype for positions.
   - **`curvyo-connector-io`** (with the transport story): WebSocket over TLS, certificate
     generation and pinning, pairing; later serial and GRBL streaming. Native only. Used by the
     connector binary and by `curvyo-app` (the desktop app opens the socket in Rust and hands
     messages to the webview through Tauri commands; the webview never holds the token).
   - **`curvyo-connector-server`**: the thin connector binary (suffix `-server` as ADR 0004
     names it). Config file and CLI, Linux first. Serves the web app (§7).
2. **TLS stack.** `rustls` 0.23 (Apache-2.0 OR ISC OR MIT) with the **`ring`** provider
   (Apache-2.0 AND ISC; `default-features = false`, features `ring`, `std`, `tls12` off, so TLS 1.3
   only); `rustls-webpki` (ISC) and `rustls-pki-types` (MIT OR Apache-2.0) come with it. Not the
   default `aws-lc-rs` provider: `aws-lc-sys` carries the OpenSSL licence, which is not in the
   `deny.toml` allow-list, and needs a C/CMake toolchain on every build host. The connector makes
   its self-signed certificate with `rcgen` (MIT OR Apache-2.0, `ring` backend). All licences are
   in the `deny.toml` allow-list; `cargo deny` confirms when `-io` is created. Only `-io` and
   `-server` depend on them; `connector-core` stays free of crypto beyond `sha2`.
3. **Two trust modes for the connector's identity**, chosen in the connector's config:
   - **Pinned (default).** On first start the connector generates an ECDSA P-256 key and a
     self-signed certificate valid for 10 years (Proposal) and keeps both in a `0600` file. The
     app pins the SHA-256 of the certificate's public key (SPKI), not the certificate, so a
     renewal with the same key changes nothing. A different key is shown as "this connector's key
     changed" and the connection is refused, never accepted silently (spec `0064` criterion 3);
     the maker re-pairs to accept it.
   - **CA-issued (EST, option behind config).** The connector obtains its certificate from the
     maker's CA with an EST client (RFC 7030: `simpleenroll`, `simplereenroll` before expiry). The
     app validates it normally against a CA certificate the maker imports into the app's settings
     (no OS trust store, no public CAs, Proposal); the host name or IP in the URL must match.
     No pin is needed. The EST client and its dependencies are decided in its own story; nothing
     is added before (`CLAUDE.md` §5).
4. **Pairing** authorises a client to command the connector. On first start, and on `--pair`, the
   connector prints its address and a code of 8 characters from a 32-character alphabet without
   look-alikes, valid 10 minutes, one use; three wrong attempts end it.
   - **Desktop app, pinned mode:** the first contact cannot check the certificate yet, so the code
     authenticates it: both sides run SPAKE2 (`spake2`, MIT/Apache-2.0) over the TLS connection
     with the code as password and the SPKI hash in the transcript (the app's hash of the key it
     saw, the connector's of its own key). A man in the middle presents a different key, the
     confirmation fails, and he gets one online guess per attempt. On success the app pins the
     SPKI hash.
   - **Desktop app in EST mode, and the browser:** TLS already authenticates the connector (or, in
     the browser, the browser does, §7), so the code is sent as `Pair { code, client_name }` inside
     TLS and compared in constant time.
   - Either way the connector answers `Paired { token }`: 32 random bytes, stored by the connector
     only as a SHA-256 hash with the client name, by the desktop app in the OS credential store
     (ADR 0007), by the browser in that origin's IndexedDB. Every later connection starts with
     `Auth { token }`; anything else before it gets `Error { code: PairingRequired }`.
     `curvyo-connector-server --list-clients` and `--revoke <name>` manage the list.
5. **Framing.** Every WebSocket text frame is one JSON message, `{ "type": "...", ... }` (serde,
   tagged enum). JSON over a binary format because messages are small, debugging matters, and the
   G-code text dominates the size anyway. Frame limit 1 MiB; a job travels in chunks of 256 KiB;
   job limit 64 MiB (Proposal).
6. **Messages, protocol version 0** (unstable, as ADR 0005 §3 for interfaces).
   - Both: `Hello { protocol, software, version }` first; a different `protocol` gets
     `Error { code: UnsupportedProtocol }` and the connection closes. Unknown `type`: `Error`, the
     connection stays.
   - App to connector: `Pair`, `Auth` (§4); `GetStatus`; `SubmitJob { job_id (UUID), name,
     kind: Job | Frame, bytes, sha256 }`, then `JobChunk { job_id, index, text }` up to `bytes`;
     `Start { job_id }`; `Pause`; `Resume`; `Stop`.
   - Connector to app: `Welcome { connector_name, controller: Option<ControllerInfo> }`;
     `Paired { token }`; `JobAccepted { job_id }` / `JobRejected { job_id, reason: Busy |
     TooLarge | HashMismatch | NotAscii | NoController }`; `Status { state, job: Option<Progress
     { job_id, line, lines, elapsed_ms }>, position: Option<[Millimetres; 3]>, alarm:
     Option<String> }`, pushed on every change and at most 5 times per second;
     `Error { code, message }`.
   - `state`: `NoController`, `Idle`, `Receiving`, `Ready`, `AwaitingLocalStart`, `Running`,
     `Paused`, `Done { result: Completed | Stopped | Failed }`, `Alarm`.
   - The SPAKE2 exchange of §4 is two `PairSpake { message }` frames each way, defined in
     `connector-io`, not in core (the browser never sends them).
7. **Job rules.** One job at a time per connector; a second `SubmitJob` while one is held gets
   `Busy` (amended 2026-11-23 by spec `0132`: it is added to the job list behind the current job
   and answered with `JobAccepted { position }`, `ListFull` beyond 20 jobs or 256 MiB; `Busy`
   stays for a frame, a homing and `Alarm`; one job still runs at a time). The connector keeps running a job when the app disconnects; any paired connection can
   read the state and send `Stop` at any time (spec `0064` criterion 6). `Stop` and `Pause` are
   idempotent. With "require a local start" on (CONN-SAFETY default A, off by default), `Start`
   leads to `AwaitingLocalStart` until the button on the connector's host. The job is GRBL-family
   G-code text, printable ASCII and newlines only; the connector checks that and the hash, and does
   not interpret it beyond line streaming.
8. **Reach** (Frage 121, A and B).
   - **Desktop app:** direct on the LAN, §3 and §4.
   - **Browser, on the LAN (B):** the connector serves the Curvyo web app build of its own release
     over HTTPS on its port; the page opens `wss://` to the same origin, so mixed-content and
     private-network-access rules do not apply. Trust is the browser's: with a **self-signed**
     certificate the browser warns once and the maker must accept the exception; the browser cannot
     pin, so a man in the middle at that moment is not detectable by Curvyo, and the pairing code
     only authorises the browser. With an **EST certificate** from a CA installed on the maker's
     device there is no warning and the connection is as strong as the desktop's. The Laser tab
     recommends EST mode for browser use (Proposal). The served app is a separate origin from a
     hosted Curvyo, so its local projects are its own; files are opened from disk. The static
     file server is chosen in that story.
   - **Browser, anywhere (A):** through the relay once the sync service exists. The relay must
     not see jobs (ADR 0008); how the TLS-protected channel is carried end to end through it is
     decided in the relay story.
   - G-code export stays available everywhere (spec `0064` criterion 7).
9. **No telemetry.** The connector logs jobs locally (time, name, result) and sends nothing
   anywhere else (R-SYS-007). EST talks only to the CA the maker configured.

## Consequences

- The message types are shared code, so app, browser and connector cannot drift silently; a
  version bump is one constant and a refusal, not a guess.
- New dependencies, all permissive and in the allow-list: `tungstenite`, `rustls` with `ring`,
  `rcgen`, `spake2`, later `serialport`; checked by `cargo deny` when `-io` is created. The EST
  client and the static file server are not decided yet.
- A self-signed connector opened from a browser is only as safe as the maker's click on the
  browser warning; recorded in `docs/technical-debt.md` once the browser-route story merges.
- Not decided: the GRBL streaming state machine (character counting, `ok`/`error` handling) and
  where its pure part lives; framing a job's outline; multi-user leases on the relay
  (ADR 0004 §4); end-to-end protection through the relay. Each is its story's decision.

## Amendment 2026-11-23: jog and "homed since connect" (spec `0118`, slice 2a)

Protocol 0, additive (section 6): the app sends `Jog { axis: "X" | "Y", direction: "Plus" | "Minus",
step: "0.1" | "1" | "10" | "50", hold? }`, `JogKeep` and `JogStop`; `State` gains `Jogging`; `Status`
gains `homed`, `machine_position` and `work_offset`, each left out when false or unknown, so every
earlier golden file stays byte-identical. `step` is a closed set, never a number, and no feed is on
the wire: the feeds are constants of `curvyo-connector-core` (3,000, 3,000, 1,000 and 300 mm/min),
capped by the controller's `$110` and `$111`. `JogStop` is not `Stop`: a release that arrives after
the jog ended does nothing, so it can never discard a `Ready` job. The dead-man (300 ms without a
`JogKeep`) counts on an injected clock: the hub passes the time since its start to the job machine
on every 50 ms tick and before each jog message, and core has no clock of its own. Details:
`specs/0118-machine-control/adrs.md`, "Second slice 2a (jog)", J1 to J9.

## Amendment 2026-11-23: the frame (spec `0118`, slice 2b)

Protocol 0, additive (section 6): `State` gains `Framing`, `RejectReason` gains `NotAFrame`, and
`Status` gains `frame_id`, left out when there is no frame, so every earlier golden file stays
byte-identical. `JobKind::Frame` (section 6, P2) now has its rules: a frame is assembled in a slot
of its own beside the held job (a `Ready` job stays held and is never listed), is at most one chunk
(`TooLarge` above), and the connector reads its words before anything moves: only `G21`, `G90`,
`G91`, `G17`, `M5`, `M9` and `G1` with `X`, `Y`, `F` above 0 and `S0`, `M5` and `G21` and a distance
mode before the first `G1`; anything else is `NotAFrame`. A frame can therefore never turn the laser
on, which is why `$32=0` does not refuse it. A homed machine's frame is checked against the travel
before the first byte (`Outside the travel by <n> mm`, Frage 334 A). A member's frame waits for an
owner, who confirms it with `Start { job_id: frame_id }` (`Confirms::Frame`, `admin.log` names Frame).
Details: `specs/0118-machine-control/adrs.md` F1 to F7.

## Amendment 2026-11-25: the focus point and its pulse (spec `0118`, slice 2d)

Protocol 0, additive (sections 6, 7): `GoToPoint { x, y, pulse? }` (an owner's motion; `x`, `y` in
job file coordinates within 10,000 mm, `pulse` `{ power_permille, length_ms }` left out when none),
`State` gains `Focusing` (the move) and `Pulsing` (the pulse), and `Status` gains `target`, the
connector's target clamped to the travel, left out otherwise; every earlier golden stays
byte-identical. The connector owns both caps: `power_permille` 1 to 50 and `length_ms` 50 to 2,000
are bounded types whose decode refuses anything outside (`Malformed`, nothing moves). The move is a
GRBL jog in absolute work coordinates after `M5`/`M9`; the pulse block `G1 M3 S<s>` / `G4 P<seconds>`
/ `M5 S0` is written in one write only after `Idle` within 0.05 mm of the target, with laser mode on
and `s = floor(power_permille x $30 / 1000) >= 1`. Stop during the move is the jog cancel, during the
pulse the stop of `0127` G12, as are a busy word, a pulse not ended within its length plus 1 s and
the connector's shutdown. `jobs.log` writes `Focus point: <client>` and `Focus pulse: <client>`.
Details: `specs/0118-machine-control/adrs.md` P1 to P12.
