# ADRs for "Machine connector"

Date: 2026-11-02. Crates, wire protocol, pairing, encryption and the stop path:
**[ADR 0032](../../docs/adr/0032-machine-connector.md), Proposed — `needs-customer`** (security
model on the LAN, a new program listening on the network, data leaving the computer, crypto
dependencies). CONN-FIRST A is decided (GRBL 1.1, USB serial, LAN first). Everything under
"Feature-local decisions" is **Proposal**, reversible inside the story.

## Depends on

- [ADR 0032](../../docs/adr/0032-machine-connector.md): the two crates, `curvyo-connector.1`
  WebSocket with Noise `XX` and JSON, SPAKE2 pairing, upload-then-stream, stop invariants,
  host files, dependencies, musl builds.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2/§3: extended by note with
  `curvyo-connector-core`, `curvyo-connector-io`, and the edge `editor-wasm` → `connector-core`.
- [ADR 0010](../../docs/adr/0010-document-keyring-admins-and-revocation.md) §1: the app's Noise
  static key is the device X25519 key, so slice 3 (relay) needs no second identity.
- [ADR 0008](../../docs/adr/0008-end-to-end-encryption-of-sync-and-collaboration.md): same
  primitive family (X25519, ChaCha20-Poly1305, RustCrypto); slice 3 relays Noise frames opaque.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §4: "leased by the
  server" superseded for connector machines (ADR 0032 Decision 9); §5 relay unchanged.
- [ADR 0007](../../docs/adr/0007-asset-library-connector-and-credential-storage.md): word clash
  only; code uses `connector` only inside the `curvyo-connector-*` crates, the asset side keeps
  `provider`/`library` names.

## Feature-local decisions (2026-11-02)

1. **Engine is a pure step function.** `Engine::step(Event, Now) -> Vec<Action>`; events are client
   messages, serial bytes, ticks; actions are bytes to write, messages to send, log lines. `Now`
   is a millisecond newtype passed in. The `-io` engine thread is a loop around it.
2. **Simulator speaks bytes.** `Simulator::feed(&[u8])`, `Simulator::advance(Millis) -> Vec<u8>`;
   the `--simulate` mode wires it behind the same byte pipe as the serial port. Fault injection
   is a plain `Faults` value, not a trait. The 128th-unacknowledged-byte check is in the simulator
   and fails the test (criterion 21).
3. **Frame is computed in the app**, sent as an ordinary small job (`kind: frame`), so stop and
   the one-job rule apply unchanged (criteria 16, 17). Jog bounds are checked in the app and again
   in the engine against the job's `work_area_mm`.
4. **Status poll** `?` every 250 ms from the engine tick; the app gets a `state` message on every
   change and at least once per second while running (criteria 11, 22).
5. **Local start** (criterion 29): `start` over the control socket or a key press in the
   connector's terminal; the 10-minute timeout is an engine rule on `Now`.
6. **Machine record stand-in** (spec, slice 1): stored in the app's local settings with name,
   work area, frame power, connector address and pinned connector public key; replaced by `0026`
   without migration.
7. **Limits as constants, not options:** 8 concurrent connections, 5 pairing attempts, code valid
   5 minutes, chunk 60 000 bytes. A job larger than free memory fails at upload. Listed in
   `docs/technical-debt.md` when the story lands (not a customer-set limit).
8. **Criterion 6 test, request to the product owner:** a dictionary run of all 10^8 codes against
   a SPAKE2 transcript is hours of curve operations per CI run. Proposal: the test runs the full
   attack loop over a reduced space (the true code among 10^4 candidates, the rest of the code
   fixed) and asserts no candidate verifies, plus a protocol test that a passive transcript has
   no code-dependent value outside the SPAKE2 element. The criterion text needs this change.
9. **Spike before the plan** (`spike/connector-channel`, never merged, result into ADR 0032):
   (a) `snow` and `spake2` with caller randomness on wasm32, size added to the editor wasm;
   (b) `ws://` to a LAN address from the Tauri webview on Linux, Windows, macOS; (c) the
   `aarch64-unknown-linux-musl` build with `serialport` without `libudev`.

## Questions (customer)

Frage 1 (CONN-SEC): Security model on the LAN.
- A (default, recommended): pairing code once, then keys; everything encrypted (Noise), no
  certificates; browser works over `http`/`localhost`, hosted `https` only via the relay later.
- B: no encryption, address only (like LightBurn Bridge); simpler, anyone on the network can run
  the laser.
- C: TLS with a certificate the maker obtains (hostname, renewal); hosted `https` works at once.
- Default if unanswered: A.

Frage 2 (CONN-BIN): A new program `curvyo-connector` that listens on a network port of the
machine PC and receives jobs from the LAN (encrypted, sends nothing elsewhere).
- A (default): yes, as a separate static Linux binary (x86_64, Raspberry Pi), started by hand.
- B: also a Windows build in slice 1 (needs a different local control path).
- Default if unanswered: A.

The spec's CONN-PATH, CONN-SAFETY, CONN-GCODE, CONN-LOSS, CONN-HW, CONN-BROWSER and CONN-UI
defaults fit ADR 0032 unchanged.
