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
   `Busy`. The connector keeps running a job when the app disconnects; any paired connection can
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
