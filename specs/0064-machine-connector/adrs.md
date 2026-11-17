# ADRs for the machine connector

Architect, 2026-11-02. Customer decisions: CONN-FIRST A (GRBL over serial, 2026-10-17); CONN-SEC B, TLS with key
pinning or an EST certificate, and CONN-BROWSER A and B (2026-11-02); CONN-PATH
default A (direct on the LAN first); CONN-SAFETY default A (local start as an option, off). Customer
input of 2026-11-02: at least the protocol between connector and workbench is next.

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
- [ADR 0066](../../docs/adr/0066-connector-through-the-relay.md) (draft, Proposed 2026-11-17,
  needs-customer, Frage 218 to 221): the relay route after the customer's feedback of
  2026-11-17; recommends the connector as an enrolled member of its own machine room with an
  end-to-end channel through the relay; the LAN mode of this branch stays unchanged (PR #375).

## Feature-local decisions

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
