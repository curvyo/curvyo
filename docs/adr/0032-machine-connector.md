# ADR 0032: Machine connector — crates, wire protocol, pairing and the stop path

**Status:** Proposed — `needs-customer` (security model on the LAN, a new program that listens on
the network, data leaving the computer, new cryptographic dependencies). Asked by
`specs/0064-machine-connector` (CONN-FIRST A: GRBL 1.1 over USB serial, LAN first). Supersedes one
sentence of ADR 0004 §4 (see Decision 9). Adds two crates to ADR 0011 §2/§3 by note.

**In plain words (for the customer).** A small program, `curvyo-connector`, runs on the PC or
Raspberry Pi at the laser. Curvyo talks to it over the local network. The first time, you type a
code it prints; after that the two recognise each other by key, and everything between them is
encrypted, also from the browser. The program takes the whole job first and then feeds it to the
laser by itself, so a Wi-Fi drop does not starve the machine. Stop always goes first. Nothing is
sent anywhere else.

## Context

The connector is a published interface between two programs that update separately (app and
connector), it writes files on the host (keys, paired devices, job log, config), and it moves a
laser. Those three are what is expensive to change later. Constraints: `*-core` is pure and builds
for wasm32 (CLAUDE.md §6); device communication lives in an `-io` crate; one machine family, no
universal machine abstraction; the browser cannot open a serial port, cannot use `ws://` from an
`https` page and cannot pin a self-signed certificate; ADR 0008/0010 give every device an X25519
key and use XChaCha20-Poly1305 and RustCrypto; nothing in `main` implements them yet
(`curvyo-crypto-core` does not exist).

## Options

| Topic | Chosen | Rejected, and why |
|---|---|---|
| Crates | **One core crate `curvyo-connector-core`, one `curvyo-connector-io` crate with the `curvyo-connector` binary target** | `curvyo-grbl-core` + `curvyo-connector-core`: a second crate split by "family vs. protocol" before a second family exists (§5). A separate binary crate: ~100 lines of `main`, and a new crate suffix for nothing. Putting it in `ui-core`/`storage-io`: the connector would pull Loro and the document model. |
| Secure channel | **Noise `XX` (25519, ChaChaPoly, BLAKE2s) via `snow`, over `ws://`, for every connection** | TLS with a self-signed certificate: browsers cannot pin it (criterion 38). TLS with a public certificate: needs a hostname and renewal (Question 6 B). Own handshake on `chacha20poly1305` + `x25519`: we would design a protocol; Noise is specified and analysed. Noise `KK`: a changed connector key shows only as "handshake failed", no new fingerprint (criterion 9). |
| Pairing | **SPAKE2 (`spake2`, RustCrypto) over the 8-digit code, bound to the Noise handshake hash** | Code as Noise PSK: safe against a passive listener only; an active fake connector brute-forces 10^8 codes offline in seconds. Short-code comparison (Bluetooth style): the spec says "type the code". OPAQUE: a server/password store we do not have. |
| Message encoding | **JSON (`serde_json`, already in the workspace) inside Noise transport messages** | CBOR: one more dependency for a few KB/s of status; G-code is text anyway. |
| Job transport | **Upload the whole job, then the connector streams it** | Stream-through from the app: link loss starves the planner mid-cut and criterion 30 becomes impossible. |
| Concurrency in the connector | **std threads + `std::sync::mpsc`, sync `tungstenite`** | `tokio` + `tokio-tungstenite` + `tokio-serial`: an async runtime for one port and a handful of clients; more code paths, larger binary. |
| Discovery | **Address typed or copied** (spec: out of scope) | mDNS (`mdns-sd`): later, when asked; a browser cannot do mDNS anyway. |

## Decision

1. **Crates** (ADR 0011 §2/§3 note). `curvyo-connector-core` → external only; `#![forbid(unsafe_code)]`,
   wasm32 in CI. Modules: `gcode` (line reader, bounding box incl. arcs, units, modes, line
   count), `grbl` (reply/status/alarm/error parser, realtime bytes, character-counting streamer),
   `simulator` (pure GRBL 1.1 model with virtual time), `engine` (the connector state machine:
   job, jog, frame, stop, link-loss policy, gates), `protocol` (messages, versions), `channel`
   (Noise session and SPAKE2 pairing as pure step functions; randomness passed in as bytes).
   `curvyo-connector-io` → `connector-core`: serial port, WebSocket listener, threads, clock, OS
   RNG, files, control socket; `src/main.rs` is the `curvyo-connector` binary (CLI, config,
   wiring; `anyhow` only there). `curvyo-editor-wasm` → `connector-core` (new edge): the app's
   client (channel, protocol, G-code reader for frame and send panel). No new crate suffix.
   GRBL types appear in `protocol` as they are; no machine trait until a second family (§6).
2. **Wire.** WebSocket, binary frames, subprotocol `curvyo-connector.1` (the major version; any
   other is refused at the HTTP upgrade). Every frame after the upgrade is one Noise message.
   Plaintext is one JSON object `{ "t": <type>, ... }`. Inside a major version changes are
   additive: unknown fields ignored, unknown `t` answered with `unsupported`. The first message
   after the handshake is `hello` with `{ protocol: [major, minor], connector_version }` both
   ways. Noise nonces make every replayed, reordered or altered frame fail; any failure closes
   the connection (criterion 8).
3. **Pairing and identity.** Every connection runs Noise `XX`; both sides learn the other's static
   X25519 key. A client whose key is not in the paired list may only send the pairing messages:
   SPAKE2 over the code with the Noise handshake hash as identity binding, then key confirmation;
   on success both store the other's key. A man in the middle sees two different handshake
   hashes, so the PAKE fails: one guess per attempt, 5 attempts per code (criterion 6). Unpaired
   clients receive nothing else (criterion 5); the connector's public key is the only thing an
   unpaired client learns. The app pins the connector key per machine record and compares it
   after `XX` message 2, so a changed key shows both fingerprints (criterion 9). The app's static
   key is the device X25519 key of ADR 0010 §1, so the relay slice adds no second identity.
   Unpair takes effect for open connections at the next message (the engine re-checks the list).
4. **Job transport.** `job_begin { name, machine: { id, name, work_area_mm }, lines, sha256 }`,
   `job_chunk` (≤ 60 000 bytes each, under Noise's 65 535 limit), `job_commit`. The connector
   checks count and hash, holds the job in memory only (never on disk, so nothing resumes after
   a restart, criterion 33) and streams it with character counting: at most 127 unacknowledged
   bytes in the controller (criterion 21). Progress is acknowledged lines. A job without
   `machine` is refused (criterion 19, `R-SYS-008`). One job per controller (criterion 28).
5. **Stop path, as invariants of `engine`** (all tested on the simulator with virtual time):
   (a) `stop` from any paired session is accepted in every state, needs no job ownership and no
   lock; (b) stop emits `!`, then `0x18` after hold-complete or 2 s, and the line queue is
   cleared in the same step, so no line byte is ever emitted after `0x18` (criterion 24);
   (c) `error:N` or `ALARM:N` halts the same way (criterion 27); (d) no job or jog while Alarm,
   unhomed or not Idle; jog cancel is `0x85`; (e) controller lost or connector restart ends the
   job as failed or interrupted and the app is told the position is unknown; (f) link-loss
   policy `continue` (default) or `hold` lives in `engine`. In `-io` one engine thread owns the
   serial writer and the state machine; client threads, the serial reader thread and a 250 ms
   tick send events into one channel; the engine never blocks except on that channel, so a stop
   waits only behind events that take microseconds. A stop is never queued behind a job upload:
   uploads are assembled in the client thread and handed over as one finished job.
6. **Host files** (formats we write; each TOML file carries `format_version = 1`):
   `curvyo-connector.toml` (serial device, baud 115200, listen address, port, `on_link_lost`,
   `require_local_start`); `identity.toml` (connector private key) and `paired.toml` (name,
   public key, paired time), mode 0600, never logged; `jobs.log`, one tab-separated line per job
   ended. Default location: the directory of the config file. Local commands (`pair`, `devices`,
   `unpair`, `start`) reach a running connector through a Unix domain socket in that directory,
   mode 0600; Windows hosts are out of scope for slice 1.
7. **Browser and desktop.** One client: the frontend opens the WebSocket, `connector-core` in
   the editor wasm does the crypto and protocol. Reachable from the desktop app and from a
   browser build served over `http` or `localhost`; an `https` app reaches machines only through
   the relay (slice 3), which will carry the same Noise frames end to end. The app's private
   key: a 0600 file in the app data directory (desktop), IndexedDB (browser); OS keychain is
   later. Open point for the spike: whether WebView2, WKWebView and WebKitGTK allow `ws://` to a
   LAN address from Tauri's origin; if one does not, the fallback is a byte pipe in `curvyo-app`
   (socket in Rust, crypto still in wasm), decided in `adrs.md`.
8. **Dependencies** (pinned exactly where they touch keys; `cargo deny` licences already allowed):
   core: `snow` (Apache-2.0 OR MIT, default features off, RustCrypto resolver features: X25519,
   ChaChaPoly, BLAKE2s), `spake2` (MIT OR Apache-2.0), `rand_chacha` (MIT OR Apache-2.0, expands
   the caller's 32 seed bytes; already locked), `zeroize`; `serde`, `serde_json`, `sha2` from the
   workspace. io: `serialport` (MPL-2.0, default features off: no `libudev`, no C library),
   `tungstenite` (MIT OR Apache-2.0, default features off, `handshake` only, no TLS),
   `getrandom` (already locked), `toml` (workspace), `anyhow` (binary). No `tokio`, no `clap`.
   The spike confirms `snow` and `spake2` take caller randomness and build for wasm32 without
   `getrandom`'s JS backend; if not, Decision 3 stays and the primitive crates replace them.
9. **Machine resources (supersedes "leased by the server" in ADR 0004 §4 for connector-attached
   machines).** The connector owns its serial port and its one running job; there is no server
   lease. ADR 0004 §4's lease remains for resources no connector owns.
10. **Build and test.** Release binaries for `x86_64-unknown-linux-musl` and
    `aarch64-unknown-linux-musl` (static, "runs from itself"; Raspberry Pi 4), built in CI on every
    PR that touches the connector crates (build only, no run). Tests: core on the simulator with
    explicit ticks, deterministic, in the ordinary nextest run; golden GRBL replies and G-code in
    `curvyo-connector-core/tests/fixtures/`; io integration tests over loopback with the
    simulator behind an in-memory byte pipe, asserting order and content, never wall-time
    durations. No timing test is needed; if one is, it measures thread CPU time (§5).

## Consequences

- The app and any connector of the same major version interoperate; a major bump breaks pairing
  with old connectors on purpose (the upgrade refuses).
- No TLS means no certificate handling for makers, and the browser works; the price is that an
  `https`-hosted Curvyo needs the relay (slice 3) to reach a machine.
- Two security crates (`snow`, `spake2`) enter the editor wasm and add to its size (reported by
  `metrics/wasm-size`, tracked in `docs/technical-debt.md`, no limit set). Neither has an
  independent audit we know of; both sit on RustCrypto primitives.
- Keys at rest are plain 0600 files on both sides until a keychain story exists (technical debt).
- The job is held in memory; a job larger than the host's free memory fails at upload with a
  clear error. No resume after restart, by construction.
- The Unix control socket leaves Windows hosts for a later slice.
- When a second machine family arrives, `grbl` and `simulator` move to a family crate by a new
  ADR; `protocol` then gains a family tag. Not before.
