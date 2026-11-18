# ADRs for the machine connector

Architect, 2026-11-02. Customer decisions: CONN-FIRST A (GRBL over serial, 2026-10-17); CONN-SEC B, TLS with key
pinning or an EST certificate, and CONN-BROWSER A and B (2026-11-02); CONN-PATH
default A (direct on the LAN first); CONN-SAFETY default A (local start as an option, off). Customer
input of 2026-11-02: at least the protocol between connector and workbench is next. Customer
decisions of 2026-11-17 (ADR 0070): CONN-RELAY A, CONN-SHARE B, CONN-LAN A (with the built-in
relay), CONN-REMOTE-START C.

- [ADR 0035](../../docs/adr/0035-machine-connector-protocol.md) (new, Accepted by the customer
  2026-11-02): crates `curvyo-connector-core` (messages, version check, job state machine,
  chunk assembly; wasm32), `curvyo-connector-io` and `curvyo-connector-server`; WebSocket over TLS 1.3
  (`rustls` with `ring`); connector key pinned at pairing (SPAKE2 over the code, SPKI bound) or a
  CA certificate via EST (config option); paired clients hold a token; JSON messages, protocol 0;
  job = GRBL G-code text; browser through the web app the connector serves, and the relay later. Covers criteria 2, 3, 6, 9 (protocol part), 10, 11.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §4, §5: the relay route
  and machine-resource leases, later.
- [ADR 0007](../../docs/adr/0007-asset-library-connector-and-credential-storage.md): the OS
  credential store holds the app's pairing token; the "asset connector" is a different thing.
- [ADR 0008](../../docs/adr/0008-end-to-end-encryption-of-sync-and-collaboration.md): the relay
  sees ciphertext only; how the TLS channel is carried end to end through it is the relay story's decision.
- [ADR 0010](../../docs/adr/0010-document-keyring-admins-and-revocation.md): not reused (ADR 0035
  option D).
- [ADR 0070](../../docs/adr/0070-connector-through-the-relay.md) (new, Accepted by the customer
  2026-11-17: Frage 218 A, 219 B, 220 A with "a stand-alone connector would then also include a
  relay server", 221 C): the connector is an enrolled member of its own machine room, reached by
  a Noise `KK` channel through the relay (relay protocol 4); shared with the members of an
  assigned workspace; a start from anyone but the owner's devices (admin role) waits for the
  press at the machine; `--host-relay` runs the `curvyo-sync-server` library inside the connector
  binary, so a workshop needs no other service; the LAN mode of this branch stays (PR #375).
  Covers criteria 7, 8 and the relay part of 2, 3, 4, 9.
- [ADR 0064](../../docs/adr/0064-laser-job-builder-and-output-writers.md) and
  [ADR 0065](../../docs/adr/0065-laser-output-settings-and-air-assist.md) (`0029`, accepted
  2026-11-17): the job text is `job::write(output_format, &job)` of `curvyo-laser-core`; the
  profile's `output_format` (`grbl` only) says which connector can run it.

## Feature-local decisions

- 2026-11-17 (after ADR 0070's acceptance, **Proposal**): **slice order for the rest of `0064`.**
  Each slice is demoable end to end and is one PR from `main` (slices 1 and 2 are this branch's
  PR #375). Order question: Frage 222 below; the list is its recommended answer A.
  - **Slice 3, Send to machine (LAN).** A "Send to machine" button beside "Export job file" in the
    Output group of a laser page whose machine profile has a paired connector sends the bytes of
    `job::write(output_format, &job)` as the job (name: the page's name), under the same checks
    as Export (a block disables both); the Connector row then shows Ready, Start and progress as
    in slice 2, and the fake controller runs the page's own job. Only `grbl` sends (a later format
    gets a check of its own until a connector speaks it). Whether an accepted send records
    Lasered like an export (`0029` criterion 28, the same reserved commit) is for the product
    owner; Proposal: yes, on `JobAccepted`. No crate, dependency, protocol or format change.
    Needs: `0029` (job, gate, writer; Done), `0026` (connector address; Done), slice 2.
  - **Slice 4, GRBL over serial** (criterion 5 without the frame). A `Controller` trait arrives
    with its second implementation (serial GRBL beside the fake controller, `CLAUDE.md` §5):
    character-counting flow control, `?` status polling with position in `Status`, `!` / `~` for
    pause and resume, feed hold then soft reset for stop, alarm text, `--serial <port>`. The
    serial crate (e.g. `serialport`, MPL-2.0, in `deny.toml`'s list) is decided in the story with
    `cargo deny`. Tests against a scripted GRBL double; the demo runs the exported job on a real
    laser. Needs: `0029` (Done). `0118` (`machine-control`: Home, jog, frame, focus, the slim
    panel) builds on this slice and owns its protocol additions.
  - **Slice 5, Enrol and reach through the relay (own devices).** ADR 0070 decisions 1 and 3 to 9:
    Noise `KK` in `crypto-core` (published vectors, crypto review), relay protocol 4 channel
    frames in `sync-core` and `sync-server`, enrolment with `curvyo-enrol-machine-v1`, machine
    room and record, the connector's relay link, `--relay`, `--enrol`, `--host-relay`; "Add
    machine" in My devices; the Connector row and Send to machine over the channel, in the desktop
    app and the browser. Demo: `curvyo-connector-server --fake-controller --relay ws://<lan
    ip>:8787 --host-relay`, enrol from one computer, run from a second paired computer and from
    the browser (the browser through a `wss://` relay). Every member is an admin here, so
    decision 2's rule is trivially met. Needs: `0096` (device group, devices list `machines`,
    `reconcile`, My devices), `0097` (removal with `NewEpoch`, last admin, leave), ADR 0031's
    "server before apps" release note; from `0116` nothing (the profile's link to its machine
    room stays per computer, ADR 0070 Concepts). UX notes: Add machine, machine rows, offline.
  - **Slice 6, Share with a workspace; remote start rule C.** ADR 0070 decisions 2 and 2a:
    assign a machine to a workspace, `reconcile` adds and removes its members as `member`, the
    connector holds every start (Start, Resume) from a non-admin for the press at the machine,
    revocation by removal from the workspace; `docs/technical-debt.md` gets the reconcile gap and
    the freshness gap. Demo: a second maker in a shared workspace sends and starts, the connector
    waits for Enter; the owner's device starts at once; removing the second maker from the
    workspace cuts their channel. Needs: `0095` (workspace list stream `machines`), `0097`
    (membership changes trigger `reconcile`), `0105` / ADR 0044.
  - **Later, each its own slice or spec:** frame, Home, jog and focus as `0118` (after slice 4;
    from a non-admin they are starts under ADR 0070 decision 2); EST certificates (ADR 0035 §3);
    the web app served by the connector on the LAN (Frage 121 B, less urgent since the browser
    reaches it through the relay); the configuration file (criterion 1); the direct channel on
    the LAN for an enrolled connector (ADR 0070 decision 8c), only if the customer asks.

Frage 222 (CONN-ORDER): Which comes first after "Send to machine", the real laser or the relay?
- A: The real laser first: slice 4 GRBL over serial, then 5 enrol and relay, then 6 sharing. A
  Curvyo job cuts on your laser soonest, and `0118` (Home, jog, frame) can start after slice 4.
- B: The relay first: 4 enrol and relay, 5 sharing, 6 GRBL over serial. Remote and browser use
  come first, but until slice 6 only the fake controller runs.
- C: GRBL over serial before "Send to machine" too (serial as slice 3).
- Recommendation: A. Default if unanswered: A.

- 2026-11-17 (slice review, **Proposal**): `Status` in `Ready` carries `job` (`job_id`, `line` 0,
  `lines`, `elapsed_ms` 0), so `Start` works from any paired app and after an app restart, like
  `Stop` does (P19, Q16). The wire shape is unchanged (`job` is already nullable); P20's list of
  states that carry progress gains `Ready`; a `status_ready.json` golden file is added; the app
  takes the id for `Start` from the `Status` of the connection it opens, not from memory.
  Protocol stays 0 (not released). Built in the same branch before the PR.
- 2026-11-16: `thiserror` is an edge of `curvyo-connector-core` besides the four ADR 0035 §1 names (`CLAUDE.md` §5: `thiserror` in libraries); `uuid` with `default-features = false, features = ["serde"]` (parse and print only, no generation, so no `getrandom` on wasm32).
- 2026-11-16: The first slice (protocol core) is Ready; invisible work, commits on `story/machine-connector`, no PR until the first demoable slice. No new ADR needed: ADR 0035 covers it.
- 2026-11-02: The first story builds only `curvyo-connector-core` with a fake controller test
  double; `-io` (WebSocket, TLS, pinning, pairing) and `-server` follow as one story; serial and GRBL
  streaming as the next.
- 2026-11-02: Criterion 4 (connector named in the machine profile, connection test) waits for
  `0026`; until then the connector address, its pinned key hash and the pairing token (in the OS credential store) are a per-user app setting beside
  `extensions.toml`, never in the project file (a shared project must not carry a workshop's
  address or keys).

## Addendum: second slice, the connector on the LAN (architect, 2026-11-17)

Covers criteria Q1 to Q22. No new ADR: ADR 0035 §1 to §7 decide crates, TLS, trust, pairing and
framing; this section fixes what ADR 0035 leaves to `-io`. Everything here is a **(Proposal)**
unless it quotes ADR 0035 or ADR 0007.

**A1. Crates and edges.** `curvyo-connector-io` (library, native only, `thiserror`) and
`curvyo-connector-server` (binary, `anyhow`), both AGPL-3.0-or-later, each with its
`[[licenses.exceptions]]` entry in `deny.toml` in the commit that creates it. `-io` depends on
`curvyo-connector-core` and no UI, document or storage crate; `curvyo-app` depends on `-io`
(desktop side) and keeps the credential store in `curvyo-storage-io`; `-core` is unchanged and
stays on its five dependencies. Dependencies stay in the crates' own manifests (one user each,
not `[workspace.dependencies]`). No async runtime: blocking `tungstenite` over
`rustls::StreamOwned` on std threads, one thread per connection; a 50 ms socket read timeout lets
the thread interleave reads with the status pushes it gets from a channel. The app side runs the
same blocking client on its own thread and reports to the webview through Tauri events.

**A2. Dependency set** (dry run of `cargo deny check licenses bans sources advisories` with the
repository's `deny.toml` on a scratch crate outside the workspace, 2026-11-17: all four green;
the real check runs again when `-io` is created).

| Crate | Version | Licence | Features | Why |
|---|---|---|---|---|
| `rustls` | 0.23.46 | Apache-2.0 OR ISC OR MIT | `default-features = false`, `ring`, `std` | TLS 1.3 (ADR 0035 §2) |
| `rustls-pki-types` | 1.16.0 | MIT OR Apache-2.0 | `std` | DER/PEM types, PEM reading of the key file |
| `ring` | 0.17.14 | Apache-2.0 AND ISC | default | CSPRNG (token, code), HMAC-SHA256 (confirmation), SHA-256 (SPKI, token hash), constant-time compare |
| `rcgen` | 0.14.10 | MIT OR Apache-2.0 | `default-features = false`, `crypto`, `ring`, `pem` | key and self-signed certificate (§3) |
| `tungstenite` | 0.29.0 | MIT OR Apache-2.0 | `default-features = false`, `handshake` | WebSocket (§5); 0.29 is the version `axum` already locks |
| `spake2` | 0.4.0 | MIT OR Apache-2.0 | default (`getrandom`) | pairing PAKE (§4); 0.5 is only a pre-release |
| `time` | 0.3.55 | MIT OR Apache-2.0 | `std`, `formatting` | `rcgen` validity takes it; RFC 3339 in `jobs.log` |
| `zeroize` | 1.9.1 | Apache-2.0 OR MIT | default | token and code buffers (ADR 0007 §4) |
| `dirs` (server only) | 7.0.0 | MIT OR Apache-2.0 | default | default state directory; already locked through Tauri |

New in `Cargo.lock`: `rustls`, `rustls-webpki` 0.103.17 (ISC), `rustls-pki-types`, `ring`,
`untrusted` (ISC), `rcgen`, `pem` (MIT), `yasna` (MIT OR Apache-2.0), `spake2`, and as second
versions `curve25519-dalek` 4.1.3 (BSD-3-Clause), `hkdf` 0.12.4 and `hmac` 0.12.1
(`multiple-versions = "warn"`, accepted until `spake2` 0.5 is released). `ring` builds C and
assembly with `cc` (no CMake, unlike `aws-lc-sys`). Not added: `tokio-rustls`, `clap`, `hex`,
`if-addrs`.

Also new in `Cargo.lock`, found at the slice review (2026-11-17): `fiat-crypto` 0.2.9 (MIT OR
Apache-2.0 OR BSD-1-Clause, through `curve25519-dalek` 4) and `windows-sys` 0.52.0 (MIT OR
Apache-2.0, through `ring`, Windows only) are built; `x509-parser` 0.18.1, `asn1-rs` 0.7.2,
`asn1-rs-derive`, `asn1-rs-impl`, `der-parser` 10, `oid-registry` 0.8.1, `rusticata-macros`,
`nom` 7.1.3, `minimal-lexical` and `synstructure` 0.13.2 (all MIT or MIT OR Apache-2.0) are lock
entries only: `rcgen`'s weak feature `x509-parser?/verify` records its optional `x509-parser` in
the lock, but no build of the workspace compiles it (`cargo tree --all-features --target all -i
x509-parser` prints nothing). `cargo deny check licenses bans sources` is green on the branch;
`advisories` fails on `chrono` (RUSTSEC-2026-0543) and `zip` (RUSTSEC-2026-0548), both locked
unchanged on `main` and not reached by the connector crates.

**A3. TLS.** Server: `ServerConfig::builder_with_protocol_versions(&[&TLS13])` on the `ring`
provider, no client auth, the one key and certificate. The version list is explicit so that
TLS 1.3 only holds even if feature unification ever turns `tls12` on. The WebSocket endpoint is
`wss://<host>:<port>/connector`; any other path gets HTTP 404, so the web app of ADR 0035 §8 can
later share the port. `tungstenite` `max_frame_size` and `max_message_size` are
`MAX_FRAME_BYTES` of core (1 MiB, exactly 1 MiB accepted, P5); the TLS handshake and the first
frame each have 10 s; at most 8 connections at once, the ninth is closed after accept. Q4 is
tested with a raw socket: plain TCP bytes, an HTTP request, and a committed TLS 1.2-only
ClientHello (`tests/fixtures/tls12_client_hello.bin`), so no `tls12` feature enters the tree.

**A4. Pinned-key verifier (app side).** One struct `PinVerifier` in `-io` implementing rustls'
`ServerCertVerifier` (an external trait, not a new one; §5 is not touched), with two modes in an
enum: `Expect(SpkiHash)` for every paired connection and `Capture` for the first contact while
pairing. `verify_server_cert` parses the end-entity certificate with
`rustls::server::ParsedCertificate`, hashes `subject_public_key_info()` (the full DER `SEQUENCE`,
the same bytes as `rcgen`'s `public_key_der()` and `openssl pkey -pubin -outform der`) with
SHA-256, records the hash it saw, and in `Expect` mode fails with
`CertificateError::ApplicationVerificationFailure` on a mismatch; the client maps that to
`KeyChanged { stored, presented }` (Q8), before any token is sent. The server name, chain and
validity dates are not checked: the pin is the trust. `verify_tls13_signature` delegates to
`rustls::crypto::verify_tls13_signature` with the `ring` algorithms, so the peer must hold the
private key of the pinned SPKI; `verify_tls12_signature` returns an error. `SpkiHash` is a
`[u8; 32]` newtype; the printed fingerprint is its first 16 bytes as 8 groups of 4 upper-case hex
characters (Q1, `ux-notes.md` section 5; corrected 2026-11-17 from "lower-case"); the full 32 bytes are what is pinned and compared.

**A5. Pairing code.** Crockford base32 (`0-9 A-Z` without `I L O U`, 32 symbols, so `O` never
meets `0` nor `I`/`L` meet `1`): 8 characters, 40 bits from `ring`'s CSPRNG, printed as
`XXXX-XXXX`. Input is normalised before use: case ignored, `-` and spaces dropped, `O` read as
`0`, `I` and `L` as `1`. One code at a time, held in memory only (never written to the state
directory or a log), valid 10 minutes, three wrong attempts. `pairing_code` is pure: every
function takes `now: Instant` from its caller (Q2's injected clock). A code is issued at start
when no client is paired yet or with `--pair`, and by the terminal line `pair` while running;
a new code replaces a valid one.

**A6. SPAKE2 transcript.** `spake2` 0.4, `Ed25519Group`. Password: the normalised 8 ASCII
characters. `id_a` = `b"curvyo-connector-app/0"`, `id_b` = the 32 raw bytes of the SPKI hash (the
app's of the key it saw, the connector's of its own key); `spake2` hashes both identities and both
messages into the session key `K` (32 bytes), so a relay with its own key yields different keys.
The app is side A, the connector side B. Confirmation: `HMAC-SHA256(K, "curvyo-connector pair
confirm app")` and `HMAC-SHA256(K, "curvyo-connector pair confirm connector")`, verified with
`ring::hmac::verify` (constant time). Order:

1. app: `{"type":"PairSpake","round":1,"client_name":"<name>","message":"<66 hex, 33 bytes>"}`
2. connector: `{"type":"PairSpake","round":1,"message":"<66 hex>"}` (or `PairRefused`, below)
3. app: `{"type":"PairSpake","round":2,"message":"<64 hex, app confirmation>"}`
4. connector, if it verifies: `{"type":"PairSpake","round":2,"message":"<64 hex>"}`, then core's
   `Paired { token }`; otherwise `PairRefused`.
5. The app verifies 4 before it pins the hash and stores the token; on failure it stores nothing.

`PairRefused { reason: WrongCode | CodeExpired | NoCode, attempts_left }` is a third `-io` frame
(beside the two `PairSpake` frames each way of ADR 0035 §6), because core's `ErrorCode` has no
pairing codes and Q6 needs the number. An attempt is counted when the connector answers round 1
and is wrong unless round 3 verifies (a connection closed or idle for 10 s in between counts as
wrong); the third wrong attempt answers `WrongCode` with `attempts_left: 0`, later tries get
`NoCode`. One exchange at a time; a second round 1 meanwhile gets `PairRefused { NoCode }`.
`client_name` is at most 64 characters, control characters refused. Frames are dispatched in
`-io`: core's frame-size check first, then a frame whose `type` is `PairSpake` decodes as the
`-io` type, everything else through `connector_core::codec::decode`. Core's `Pair { code }`
gets `Error { UnexpectedMessage }` in this slice (the code must not travel in clear to an
unpinned key; the browser story enables it). Golden files: the four `PairSpake` shapes and the
three `PairRefused` reasons in `curvyo-connector-io/tests/fixtures/`, and one known-answer run
with fixed scalars through `start_a_with_rng`/`start_b_with_rng` and a fixed test RNG.

**A7. Token and auth gate.** Token: 32 bytes from `ring`, sent as 64 lower-case hex in
`Paired`/`Auth`, held in `Zeroizing` with a redacted `Debug` (ADR 0007 §4). The session state
(unauthenticated, pairing, authenticated) lives in `-io`'s session, not in core; before a valid
`Auth` every message except `Hello`, `Pair`, `PairSpake` and `Auth` gets
`Error { PairingRequired }` and never reaches the `Machine` (Q9). A valid `Auth` is answered with
the current `Status`. The connector compares `SHA-256(token)` against `clients.json` in constant
time and re-reads that file on every `Auth`, so `--revoke` from a second process takes effect on
the next connection; an already authenticated connection is not cut (**Proposal**, ADR 0035 says
nothing; cutting it needs a signal between processes).

**A8. Credential-store record (app side, ADR 0007 §1, §3, §4).** One entry per machine profile id
in the app's existing `CredentialStore` (keychain, or the passphrase file the app already offers
when there is none; no new storage), name `connector/<profile id>`, value UTF-8 JSON:
`{"format":1,"spki_sha256":"<64 hex>","token":"<64 hex>","connector_name":"<name>"}`. No address
(it is the profile field of `0026`), no code, nothing in a profile record or the project file
(Q10). The webview only ever gets the fingerprint text and the connector name; pairing, the
token and the pin stay in Rust. Forget, a successful re-pairing and deleting the profile remove
or replace the entry: `CredentialStore` gains `remove(name)` in `curvyo-storage-io`. A "Not in
your profiles" copy has no id and no record. Golden file for the record JSON in
`curvyo-app` tests.

**A9. Fake controller.** `FakeController`, a plain struct in `-io` (no trait; the trait comes with
serial/GRBL, `CLAUDE.md` §5), pure: `new(lines_per_second, alarm_at_line)`,
`command(ControllerCommand, at)` and `advance(at) -> Vec<Event>`, where `at` is a `Duration`
since the server's start, given by the caller. It counts lines (a final line without `\n`
counts), emits core's `Progress`, `JobFinished` and `Alarm` events, reports
`ControllerInfo { name: "Fake controller", firmware: "curvyo-fake 0" }` and no position. One
streamer thread calls it every 50 ms; after `Alarm` the streamer stops on its own (follow-up 12)
and the connector stays in `Alarm` until restarted (no clear command in this slice). The status
limiter is a separate pure struct (`push(status, now)`, `due(now)`): newest pending status wins,
at most one send per 200 ms, `GetStatus` answered at once, so every change reaches the app
within 200 ms and never more than 5 per second (Q14).

**A10. State directory** (`--state-dir`, default `dirs::data_dir()/curvyo-connector`, on Linux
`~/.local/share/curvyo-connector`), created `0700`:

- `identity.pem`, `0600`: a `PRIVATE KEY` (PKCS#8, ECDSA P-256) and a `CERTIFICATE` block,
  written once (temp file and rename). On start: missing, a new pair is made; unreadable, not
  parseable, a key that does not match the certificate, or a mode wider than `0600` refuses the
  start with a message naming the file (Q3). Fixtures: one valid and one damaged file.
- `clients.json`, `0600`: `{"format":1,"clients":[{"name":"...","token_sha256":"<64 hex>","paired_at":"<RFC 3339 UTC>"}]}`,
  rewritten by temp file and rename; a re-pairing under the same name replaces that entry.
  Golden file. A pairing and a `--revoke` in the very same moment can lose one write (accepted).
- `jobs.log`, `0600`, append only, the line format of Q18; control characters in the job name
  become a space. Golden file.

Nothing else is written there or anywhere else.

**A11. Command line** (hand-parsed like `curvyo-sync-server`'s `config.rs`, no `clap`, no
environment variables, no config file in this slice):
`--listen <addr:port>` (default `0.0.0.0:7447`; the printout says all addresses when it is
unspecified, no interface lookup), `--state-dir <dir>`, `--fake-controller` (without it the state
is `NoController`, which Q13's "No controller" uses), `--fake-lines-per-second <n>` (default 20),
`--fake-alarm-at-line <n>`, `--require-local-start`, `--pair`, and the one-shot commands
`--list-clients` and `--revoke <name>` that work on the state directory and exit; `--help`. The
two `--fake-*` flags need `--fake-controller`. Exit codes 0, 1 (runtime), 2 (usage). Terminal
lines while running: an empty line is the local start press (only in `AwaitingLocalStart`),
`pair` issues a new code. The connector name in `Welcome` is the host name from
`/proc/sys/kernel/hostname`, else `curvyo-connector`.

**A12. Module split.** `-io`, one responsibility each: `spki` (`SpkiHash` and fingerprint text),
`identity` (key file), `server_tls` (server config), `pin_verifier` (A4), `frames` (`-io` frames
and dispatch), `spake` (A6 exchange and confirmation), `pairing_code` (A5), `clients`
(`clients.json`), `job_log`, `fake_controller`, `status_limit`, `hub` (shared `Machine`, code,
clients, fan-out under one `Mutex`), `session` (one connection's loop and auth gate), `server`
(listener, threads, `RunningServer` handle: address, fingerprint, code, local start, new code,
notices for the terminal, shutdown), `client` (the app side: connect, pair, auth, test, job,
controls, status). `-server`: `main` (wiring and printing), `args` (A11), `terminal` (stdin
lines to the handle). Q21's test drives `RunningServer` and `client` in one process.

**Spec checks against ADR 0035 and `CLAUDE.md` §5** (fixed in the spec, 2026-11-17):

- Q7 asked for a separate "Key mismatch during pairing" text; SPAKE2 cannot tell a relay from a
  wrong code (ADR 0035 §4: "the confirmation fails"), so Q7 now shows Q6's text and tests that
  nothing is stored and one attempt is counted.
- Q1 checked "no client code in the dependency tree", which `rustls` and `tungstenite` (and `-io`'s
  own app-side `client`, ADR 0035 §1) make impossible; Q1 now checks that the server never calls
  `client` and has no HTTP client crate.
- No other conflict. Deferred, not conflicting: ADR 0035 §1's config file and §3's trust-mode
  setting (pinned is the only mode), §8's `Pair { code }` path. ADR 0007 §1's "no 0600 plaintext
  store" is about client credentials; the connector's own key file is ADR 0035 §3's decision.

## Addendum: third slice, Send to machine (architect, 2026-11-17)

Covers `specs/0125-send-to-machine` criteria S1 to S18. No new ADR: ADR 0035 (protocol), ADR 0064
§11 (export effects) and addendum A1 to A12 above decide everything this slice reuses. Everything
here is a **(Proposal)** unless it quotes an ADR.

**B1. What does not change.** No new crate, no new external dependency, no new trait, no
protocol change (protocol 0, every golden file under `curvyo-connector-core/tests/fixtures/` and
`curvyo-connector-io/tests/fixtures/` byte-identical), no `format_version` change: the commit
writes the fixture jobs' Output, the stored numbers and `test_grid_lasered`, the fields
criterion 28 of `0029` already writes. No GRBL writer change and no golden change (B6).
What changes, all internal API: `curvyo-connector-core` gains one module (B4);
`curvyo-connector-io`'s `Connection::send_job` takes the job id from its caller
(`send_job(job_id, name, text, deadline)`, `job_send::new_job_id()` becomes `pub`) and `watch`
gets the retry rule of B5; `curvyo-ui-core` gains one module (B2); `curvyo-document-core`'s
`export_job` takes its commit label (B3); `curvyo-editor-wasm` gains, for the S17 test only,
`curvyo-connector-io` and `curvyo-connector-core` as `[target.'cfg(not(target_arch =
"wasm32"))'.dev-dependencies]` (internal path crates; no production edge, nothing in the wasm build).

**B2. Enabling rule: one function beside Export's gate** (S1 to S5, S8). `curvyo-ui-core/src/send_button.rs`
("decides whether Send to machine is enabled and why not"), pure, no connector types:
`send_button(gate: &Gate, format: OutputFormat, link: SendLink, sending: bool) -> SendButton
{ enabled, cause: Option<SendCause> }`. Causes in this order, the first wins: (1)
`gate.first_block()` (the very issue Export's tooltip shows, so S2 holds by construction:
Export and Send read one `Gate` in one frame); (2) `Browser`; (3) `FormatNotSpoken(format)` from
`connector_speaks(format)`, one `match` on the closed enum (`Grbl => true`; adding a format to
ADR 0064 §12 adds its arm there, the compiler asks); (4) `NotPaired`, `NoLongerPaired`,
`KeyChanged` from `link`; (5) `Sending`. `SendLink` is a ui-core enum `Browser | NotPaired |
NoLongerPaired | KeyChanged | Paired`. The session (`session/output/view_json.rs`) adds
`send: Option<SendView>` to `OutputGroupView`, `None` when the page copy's `connector_address`
is empty (S1), else `send_button(..)`; `sending` is the session's own pending send (B3); `link`
is set by the frontend through one wasm call `set_send_link(spelling)` from the link store (B5),
session state only, never in the document. The words: a gate cause keeps its check text
(`job_gate_text.rs`); the connector causes are serialised as tags and worded by
`frontend/src/lib/connectorText.ts`, where the Connector row's words already live. Unit tests in
ui-core: every cause, the order, and "Export disabled ⇒ Send disabled with the same text" for
each `Cause` that blocks.

**B3. The commit: one function, two callers** (S6, S10, S12). In `session/output/export.rs` the
hand-out and the commit are shared: `export_job` and a new `send_job` both call one private
`hand_out(kind)` (gate checked again at that moment, `write(format, &job)`, `export_effects`, a
token); `send_job` returns the same bytes, the **page's name** and the token, and `None` unless
`send_button(..).enabled`. `export_job_done` and a new `send_job_done(token, connector_name,
now_ms)` both call one private `commit(effects, label, now_ms)`: the history step through
`Document::export_job(label, ..)` (labels `export_job` "Export job file" and `send_job` "Send
job to machine" in `curvyo-ui-core/src/history_names.rs`; nothing committed when nothing differs,
so an unchanged second send adds no step, S12), then the reserved `mark_test_grid_lasered`
commit and the records as ADR 0064 §11 (2), (3); undo takes back the step only. Notices:
`send_notice(page, connector, n)` beside `export_notice`, sharing `fit` and the second line.
Tokens: the send token is held apart from export tokens (`Exports::send: Option<(u32,
ExportEffects)>`), so an export during a send does not make it stale; at most one pending send
per session (Send shows `Sending` on every laser page meanwhile); `send_job_failed(token)` drops
it. An unknown or used token commits nothing.

**B4. Commit at most once per job id, reconnect included** (S11, P19, P20). One pure struct in
`curvyo-connector-core/src/send_watch.rs` ("follows one sent job until the connector has
accepted or lost it"): `SendWatch::new(job_id)`, `transfer_ended()` and `observe(&Message) ->
Option<Resolution>`, `Resolution = Accepted | Rejected(RejectReason) | NotReceived`.
`JobAccepted`/`JobRejected` with its id decide at any time; a `Status` whose `job.job_id` is the
id in `Ready`, `AwaitingLocalStart`, `Running`, `Paused` or `Done` is `Accepted` (P20: these
carry the progress); while the transfer runs, any other `Status` is ignored (the watcher sees
`Receiving`); after `transfer_ended()`, the next `Status` without the id (`Idle`, `Receiving` left
by a dropped transfer, `NoController`, `Alarm`, another id) is `NotReceived`. After a
`Some(..)` it answers `None` for ever, so a replayed status never commits twice (unit test).
The app holds it: `ConnectorState` (`curvyo-app`) gets `pending: Mutex<Option<(profile id,
SendWatch)>>`; the send command (new module `curvyo-app/src/connector_send.rs`, since
`connector.rs` is past 500 lines) registers it before `SubmitJob`, feeds it the send's own
answer, and the watcher's thread feeds it every `Status`; whoever gets the `Some` takes the entry
under the mutex and reports it, the command in its return value, the watcher as one
`connector-event` `{kind: "sendResolved", jobId, resolution}`. A connect failure before
`SubmitJob` clears it ("Nothing answers at <address>"). A connection error after `SubmitJob`
calls `transfer_ended()`, then one fresh `open_paired` whose first `Status` resolves it; if
that fails too, the watcher's retries (B5) resolve it later in this run. The webview commits with
`send_job_done` only on `Accepted` for the job id it holds, and calls `send_job_failed` on the
others; the token is used once, a second guard. A send unresolved when the app quits commits
nothing (accepted gap: such a job runs, the page is not Lasered). Known edge, accepted: a job
accepted, then started from another app and in `Alarm` before the reconnect reads "Not received"
(`Alarm` carries no job id).

**B5. One status source** (S9, S13 to S15, Q14, Q16). Rust: one watcher per profile id
(`ConnectorState.watcher` becomes a map; `connector_unwatch(profile_id)`), the standing
connection of slice 2. It never sends `GetStatus` on a timer: the connector pushes on change
at most 5 per second (A9), the watcher forwards each push unchanged, the app adds no limiter.
`watch` retries every 5 s only while the last `Status` it saw was active (Receiving to Paused) or
a send is pending; otherwise a drop or a failed first read is reported once and the watcher
ends, and the next press or showing starts it again (no polling while idle). Frontend: one module
`frontend/src/lib/connectorLink.ts` listens to `connector-event` once and keeps, per profile id
and address, `{status, held, sending, lost, error}` with a consumer count: the first subscriber
calls `connector_watch`, the last `connector_unwatch`. `useConnector` (Connector row) and the
Output group's hook read the same entry, so both show equal values (S14 test on the store);
`held` (job id, name, lines, bytes) is set by either send, so the row shows the page's job Ready
with Start (S9). The status carries no job name (protocol 0), so a name is shown only for a job
this app sent in this run, matched by id (S15).

**B6. A non-ASCII page name in the header** (S7). Settled by ADR 0064 decision 2 and built:
`job_write_grbl.rs` `comment` writes every character that is not printable ASCII (`0x21` to
`0x7E`, or a space) as `_`, one per character, then cuts the line at 80 characters ("Küche" →
`; Curvyo job: K_che`); `acceptance_0029_tester_core.rs` `c24_long_and_non_ascii_names_stay_ascii_lines`
covers it. So the writer already meets P10 and Send does **not** sanitise: the bytes stay
byte-identical to Export's (S6). No writer change, no golden change. `SubmitJob.name` is a JSON
string and carries the page's name as it is ("Küche"); the connector checks only the text, and
`jobs.log` turns control characters into a space (A10). S7's test asserts the header line above
and an accepted send.

**B7. App and webview** (ADR 0035, A8). The webview holds the document and its wasm session, the
job bytes for the one call, the session token and job id of the pending send, statuses, names,
counts and fingerprints. Rust holds the record (pin, token) in the credential store and process
memory, every connection, the watcher and the `SendWatch`. The bytes reach Rust as the raw body of
one command, `connector_send_page_job(profile_id, address, name)` (`raw_ipc`, as
`write_job_export`); Rust never gets the document, the webview never gets the token. Browser
build: no host command is called, `SendLink::Browser`, Export downloads as before (S5).

**B8. Tests** (S16, S17). Unit: `send_button` (ui-core), `SendWatch` with replayed and
reordered statuses (connector-core), `hand_out`/`commit` (editor-wasm: Send's bytes equal
Export's; one step; Lasered kept by undo; no step for an unchanged second send). Integration:
`curvyo-editor-wasm/tests/send_to_machine.rs` drives a session with a test grid and a fixture
job, a `RunningServer` with the fake controller, pairing, `send_job` → `Connection::send_job`
→ `SendWatch` → `send_job_done`, Start, Pause, Resume, Stop, a second send, and a dropped
connection after the last chunk resolved by a fresh `Status`; waits by line counts and statuses,
never by sleeps. Project fixtures saved before and after compare byte-identical (S16).

**Spec checks against ADRs 0035, 0064 and slice 2** (fixed in `specs/0125-send-to-machine`, 2026-11-17):

- S7 proposed `?` for a non-ASCII name and said `0029` does not settle it; ADR 0064 decision 2
  already writes `_` and the writer is built that way (B6). S7 now says so.
- S10 said the notice shows 3 s, criterion 27 of `0029` (which it reuses) shows 5 s with the
  second line; S10 now says "3 s, 5 s with the second line".
- S11 named only `Idle` and another id for "Not received"; `Receiving` left by a dropped
  transfer, `NoController` and `Alarm` carry no id either (B4). S11 now lists them.
- S13 and S15 asked for the name of a job sent by another app; `Status` has no name and the spec
  forbids a protocol change. S15 now shows such a job's lines without the name and size (B5);
  S13's words stand for the app's own job.
