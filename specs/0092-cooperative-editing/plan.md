# Plan for cooperative editing, slice 1

Implementer, 2026-11-03, on `story/0092-cooperative-editing` (rebased onto `main` `49716eba`,
format 33). Built against `specification.md` slice 1 (criteria 1 to 40), `adrs.md` decisions 1
to 10, ADR 0031 (accepted) and `ux-notes.md`. Everything marked **(Proposal)** is mine and
vetoable at the demo. One PR at the end (M7), when the customer can try two instances against a
local server; M1 to M6 are commits on this branch.

## Affected crates/modules

- **`curvyo-document-core`**: `pages.rs` and `document_size.rs` (page frame register, ADR 0031
  9a), `page_frame.rs` (new: codec, migration read, damage check), `container.rs` and
  `container_share.rs` (new: `share.json` and `keyring.log` members), `document.rs`
  (`CURRENT_FORMAT_VERSION` 34, `DocumentVersion::to_text`/`from_text`), `images.rs`
  (`insert_asset`), goldens `page_frame_v34.curvyo`, `shared_v34.curvyo` and damage fixtures.
- **`curvyo-crypto-core`** (new crate, slot of ADR 0008 §4 / 0011): envelope, `NonceStream`,
  identity and invite keys, wraps, fingerprints, `key_id`, keyring entry codec, replay and fork
  rules, "who lacks a wrap" query.
- **`curvyo-sync-core`** (new crate, ADR 0031 decision 1): frame codec, `ClientSession`,
  `RoomLog`; `tests/support/sim.rs` with real `Document` replicas (dev-dependency only).
- **`curvyo-sync-server`** (new binary crate): axum/tokio shell around `RoomLog`, file store,
  `/healthz`, flags/env, shutdown, `Dockerfile`.
- **`curvyo-ui-core`**: presence payload types, participant colour from fingerprint, the
  "B is editing this" marker, History label of a live peer.
- **`curvyo-editor-wasm`**: `wasm_sync.rs` / `session/sync.rs` binding of `ClientSession`, remote
  import at frame start, presence out and in, removal and refusal events; CPU budgets in
  `tests/cpu_time/`.
- **`curvyo-storage-io`**: credential store module (`keyring` crate, encrypted-file fallback).
  **`curvyo-app`**: one command to read and write a document's keys; CSP `connect-src`.
- **`frontend`**: WebSocket transport and timer tick, Share tab, collaboration pill, presence
  layer, removal and invite dialogs, notices.
- Docs and CI: `docs/technical-debt.md`, `docs/design-system.md` (Share tab), wasm build and
  clippy list plus ban list (`getrandom`) for the two new core crates, `sync-core` edge check,
  `deny.toml`, licence list.

## Milestones and tasks

### M1 document-core: page frame, format 34, share container, version text, assets

1. [x] `DocumentVersion::to_text`/`from_text` on the ADR 0014 §7 encoding; round trip, `never()`,
   malformed text refused (criteria 18, 19 via `synced_version`).
2. [x] `Document::insert_asset(bytes) -> AssetHash`, idempotent, dropped on save while no node
   references it (criteria 3, 18; ADR 0031 decision 3).
3. [x] Page frame register `page.<id>.frame` `{x, y, width, height}`, written whole by add, move,
   resize, fit, drawing pages and new documents; read falls back to the version 33 registers;
   resize keeps the frame centre and moves no object; fit writes the content bounds as the frame;
   damage rule at open (criterion 31). Un-ignore `concurrent_resizes_of_different_axes_keep_the_centre`.
   Readers that assumed the first page at (0, 0) read its frame (session resize/fit stop panning
   the view because nothing moves; tests that read stored positions after a resize read them
   relative to the frame origin).
4. [x] Container members `share.json` (`{format_version: 1, server, room, server_seq,
   synced_version}`) and `keyring.log` (opaque bytes); a file with both opens as shared, one
   without the other is damaged, a `share.json` version above 1 is too new (criteria 5, 19, 26).
5. [x] `CURRENT_FORMAT_VERSION` 33 to 34 with its paragraph; goldens `page_frame_v34.curvyo`,
   `shared_v34.curvyo`, `page_frame_damaged_v34.curvyo`; every v33 golden still opens and reads
   its frames from the old registers (criteria 26, 31).

### M2 curvyo-crypto-core

6. [x] Crate skeleton (`#![forbid(unsafe_code)]`, no `getrandom`, wasm32), dependencies of ADR
   0031 decision 13 with `default-features = false`; `cargo deny`.
7. [x] Envelope seal/open with the cleartext header as associated data, kinds, signed flag;
   `NonceStream`; XChaCha20-Poly1305 vectors; golden envelopes; proptest: flipped byte, other
   room, epoch, `key_id`, kind refused (criteria 24, 25, 27, 28).
8. [x] Identity keys (X25519 + Ed25519 from passed entropy), fingerprints, epoch `key_id`, wraps
   with AD `document_id ‖ epoch ‖ key_id ‖ recipient`; RFC 7748 / 8032 vectors (criteria 1, 3).
9. [x] Keyring entry codec (all eight ADR 0010 §3 payloads), signatures, causal replay, fork rules,
   invite key derivation and `ClaimInvite` proof, "who lacks a wrap I can give"; exhaustive graphs
   up to three members incl. remove plus rotate, concurrent rotations, rewrap by a removed member
   (criteria 2, 3, 4, 32 to 34).

### M3 curvyo-sync-core

10. [x] Frame codec for every client and relay frame of ADR 0031 decision 5; golden frames;
    64 MiB limit (criterion 25 `too-large`).
11. [x] `RoomLog`: gapless sequence, admission (member, valid claim, genesis), keyring before
    records, removal closes connections, snapshot replace and prune (criteria 4, 27, 32, 33, 38, 40).
12. [x] `ClientSession`: hello/proof, cursor, outbox with `client_id`s and acks, sealing at send
    time with the current epoch, ping/offline/reconnect timing with jitter from passed entropy,
    snapshot trigger (500 records / 4 MiB), presence throttle, heartbeat and expiry, removal
    (criteria 6, 14, 17, 20, 33, 34).
13. [x] Seeded simulator with two or three real `Document` replicas: convergence (8 to 12),
    offline 50 + 50 and byte-identical SVG of one version (18), close and reopen through
    `pack`/`unpack` (19), relay restart (20), undo with a peer (21 to 23), uninvited key (27),
    one edit in 1,000 objects at most 10 kB (13), removal (32 to 34), concurrent resizes (31),
    two rooms of one maker (40).

### M4 curvyo-sync-server

14. [x] Flags and environment, flag wins, defaults (criterion 35).
15. [x] File store per room (`keyring.log`, `log` with fsync, `snapshot` by rename), startup scan
    (criteria 20, 38).
16. [x] axum `/` WebSocket and `/healthz` 503 then 200; SIGTERM/SIGINT closes with 1012 and exits
    within 10 s; no IPs or bodies in logs (criteria 36, 37).
17. [x] Real-socket test on `127.0.0.1:0`: golden sealed frames with `ZQX-MARKER-7731` and
    `#13a7c9`, restart, byte search of the data directory and logs (criteria 24, 20, 37).
18. [ ] `Dockerfile` (musl static, distroless `static-debian12:nonroot`, `/data`, 8787), built by
    hand once (criterion 36). Written; the image build is open (no Docker or Podman on the build
    VM); the musl static binary inside it was built and run.

### M5 editor-wasm session, transport, Share tab, presence

19. [x] Credential store in `storage-io` and the key command in `curvyo-app` (ADR 0031 decision 12).
20. [x] `session/sync.rs`: drive `ClientSession`, import remote updates at frame start (one
    redraw), export after each commit, assets as records, `share.json` kept in the session
    (criteria 6 to 13, 17 to 20).
21. [x] Frontend WebSocket transport and tick; server address setting; Tauri CSP (criterion 39).
22. [x] Share tab: Share, Invite (24 h, sentence of criterion 2), Join ("Open shared document",
    waiting text, refusals), participant list with names, colours, fingerprints, epoch
    (criteria 1 to 5).
23. [x] Presence: pointers with labels, selection outlines, tool icon, in-flight drag geometry,
    "B is editing this" marker, expiry (criteria 7, 14 to 16); collaboration pill and connection
    states (criteria 1, 17, 20).
24. [x] Peer-scoped undo messages (criterion 22), History labels of live peers, versions for
    export and jobs (criteria 29, 30). Missing-font guard of ADR 0009 §5 if not present.

### M6 removal, invite expiry, notices

25. [x] Remove participant (admin only): dialog sentence, `RemoveMember` + `NewEpoch` + signed
    snapshot, epoch shown, removed device's notice and plain local file (criteria 32 to 34).
26. [x] Invite expiry and used/expired refusals, "A received update was rejected" once, then the
    resync from the room's snapshot (criteria 3, 4, 25; adrs.md decision 2).
26a. [x] M5 gaps: pictures added offline before a reopen go out on connect (criterion 19, ADR 0031
    decision 9); labels of other participants kept between runs (adrs.md decision 5).

### M7 journey, verification, gate

27. [x] Browser journey with two app instances and a local server (criteria 1 to 3, 6, 14, 17,
    18, 32); CPU budgets (criterion 13 and adrs decision 9).
28. [x] Docs: `docs/technical-debt.md` entries of `adrs.md`, design system, licences; CI lists.
29. [ ] Tester pass, UX review, architect review; full gate (`CLAUDE.md` §7); PR.

## Validation

Test-first per module: unit tests in each new module, golden files for every format (container
members, envelopes, keyring logs, frames), proptest for the envelope, exhaustive keyring graphs,
the seeded simulator for convergence and offline behaviour, one real-socket server test, thread
CPU budgets, and the browser journey. Not in the gate: the 95th-percentile latencies of criteria
6 and 14 and the three-OS webview reach of a LAN `ws://` relay (the tester measures them on the
LAN demo).

## Dependencies (ADR 0031 decision 13)

`chacha20poly1305`, `chacha20`, `hkdf`, `sha2`, `zeroize` (MIT/Apache-2.0), `ed25519-dalek`,
`x25519-dalek` (BSD-3-Clause), `axum`, `tokio` (MIT), `anyhow` (MIT/Apache-2.0, already in the
workspace), `keyring` (MIT/Apache-2.0); dev: `proptest` (MIT/Apache-2.0, if not present),
`tokio-tungstenite` (MIT) for the server test client (Proposal). All checked with `cargo deny`.

## Results

### M1 (2026-11-03)

- Page frame register `page.<id>.frame` in `page_frame.rs`; every page edit (add, move, resize,
  fit, drawing pages) writes it whole; reads fall back to the version 33 registers.
  **(Proposal)** A new document's setup change still writes the four version 33 registers for the
  first page (read as its frame) and its first edit writes the frame: the setup change, and with it
  every node id in the existing goldens, stays as before; convergence is unaffected because every
  writer writes the whole frame.
- `Document::resize_page(id, size)` lost its `objects` argument (resize moves no object);
  `Document::fit_page_to_content(id, box)` is new (the multi-page fit of the session);
  `Document::resize_shift` and `fit_shift` are gone, and the session no longer pans the view on a
  resize or fit (nothing moves). `Viewport::pan_by_document_offset` was then used only by tests;
  deleted in the review fixes with its tests (criterion 20 stays covered on the session).
- Single-page fit now always fits all objects (the "first page not at (0, 0)" branch to the
  active-page fit is gone, since after any resize the first page is not at (0, 0)).
- Tests that read stored positions after a resize or fit now read them relative to the first
  page's frame origin (helpers `sig_of`, `rect_origin`, `content_bounds`, `first_path_points` in
  the 0015 tester files; single lines in 0090, 0059, 0060, clone, compound path tests); version
  pins in the 0069 tester test follow the 0071 precedent; the four damage fixtures joined the 0017
  tester list. The ignored two-resizes test of 0015 is un-ignored and passes.

### M2 (2026-11-03)

- `curvyo-crypto-core`: `envelope` (39-byte header as associated data, kinds update, asset,
  snapshot, presence; signed flag with signer key and signature after the ciphertext; a snapshot
  must be signed), `NonceStream` (nonce *n* = first 24 bytes of the ChaCha20 keystream under the
  seed with *n* as IV), `IdentityKeys::from_entropy(&[u8; 64])` (ed25519 seed ‖ X25519 secret),
  fingerprints, `EpochKey::key_id`, `wrap_epoch_key`/`unwrap_epoch_key`, `InviteKey` and the claim
  proof, `keyring` (codec of all eight payloads, `keyring.log`, causal replay, fork rules,
  `missing_wraps`). Goldens: four envelopes and `keyring_v1.log` in `tests/fixtures/`
  (`CURVYO_UPDATE_GOLDEN=1` rewrites them).
- **(Proposal)** X25519 through `curve25519-dalek` 5 (`MontgomeryPoint::mul_clamped`) instead of
  `x25519-dalek`: `x25519-dalek` 3 depends on `rand_core` unconditionally, which ADR 0031 decision
  13 rules out; `curve25519-dalek` is the code it wraps and is already pulled by `ed25519-dalek`.
  The current major lines are used (`chacha20poly1305` 0.11, `chacha20` 0.10, `hkdf` 0.13, `sha2`
  0.11, `ed25519-dalek` 3, `curve25519-dalek` 5), so the crate has its own `sha2` 0.11 next to the
  workspace's 0.10 (duplicate-version warning in `cargo deny`).
- **(Proposal)** Wrap layout `key_id ‖ ephemeral public ‖ sealed key ‖ tag` (88 bytes): the
  `key_id` is public so replay and recipients know which key of an epoch number a wrap holds
  (needed for `RewrapEpoch` after concurrent rotations) without a payload change. Wrap AD binds the
  recipient's whole identity (both public keys).
- **(Proposal)** Identity references inside payloads (`RemoveMember`, `SetRole`, `RewrapEpoch`,
  wrap recipients) are the ed25519 key; `Genesis`, `AddMember` and `ClaimInvite` carry the whole
  identity. Invite `expires` is seconds since the Unix epoch.
- **(Proposal)** Validity rules beyond ADR 0010's text, all fail closed: `NewEpoch` must be the
  highest number + 1, wrap exactly the current members, use one fresh `key_id`; `ClaimInvite`'s
  author must be the claimed identity; `RewrapEpoch` needs author and recipient to be members and the
  author to hold that key; a genesis must wrap epoch 0 to exactly its admins.
- **(Proposal)** Deny wins over the epoch tiebreak: an epoch key wrapped to someone who is not a
  member of the merged state (a rotation concurrent with that member's removal) is not chosen as
  current while an epoch held only by members exists. Without it, ADR 0010 §5's "lowest hash" could
  pick a rotation that still includes the removed member.
- `missing_wraps(me, now)` follows ADR 0031 decision 8 literally: a claimant holding no key is
  not completed once the invite's expiry is not after `now`, so a joiner whose claim the relay
  accepted in time but whom no member saw before expiry is not admitted (criterion 3 "keeps
  waiting" vs. the 24 h link). For the lead.
- Long functions, justified: `write_payload` (73 lines), `read_payload` (60), `state::collect`
  (67) and `validity::is_authorized` (68) are one `match` arm per ADR 0010 §3 payload; split, the
  rule table would be scattered.
- CI: the `core-wasm32` ban step also refuses `getrandom` and `rand_core` in the trees of
  `curvyo-crypto-core` and `curvyo-sync-core`. `[profile.dev.package.curve25519-dalek]` gets
  `opt-level = 3` (the exhaustive keyring tests verify thousands of signatures).

### M3 (2026-11-03)

- `curvyo-sync-core` (deps: `curvyo-crypto-core`, `thiserror`; `curvyo-document-core` as a
  dev-dependency only): `frame` (every client and relay frame of decision 5, tag byte plus
  little-endian fields, the last bytes field runs to the end of the message, 64 MiB limit;
  golden `tests/fixtures/frames_v1.bin`), `relay` (`Relay`: handshake and routing over many
  rooms; `RoomLog`: gapless sequence, keyring before records, envelope checks, removal closes
  connections, snapshot replace and prune, `StoredRoom` restore; outputs are sends, closes and
  `StoreOp`s for the server), `client` (`ClientSession`: hello/proof, cursor, outbox with
  `client_id`s and acks, sealing at send time under the current epoch, ping 2 s / offline 4 s /
  reconnect 0.5 to 10 s ±20 %, presence 50 ms / 2 s / 5 s, snapshot after 500 records or 4 MiB,
  share, join, invite, remove with rotation, automatic `RewrapEpoch`, invite expiry while
  waiting). Clock and entropy are passed in (`Millis`, two 32-byte seeds).
- Tests: `frames` (golden, layout, round trip, malformed, too large), `relay_rules` (admission,
  refusals, sequence, envelope checks, snapshots, presence, restore), `session_timing` (ping,
  backoff, presence throttle and expiry, snapshot trigger, acks), and the simulator
  `tests/support/sim.rs` with real `Document` replicas: `sim_join` (criteria 1 to 4, 27, 40),
  `sim_editing` (6, 8 to 13, 21 to 23, 31), `sim_offline` (17 to 20, 25), `sim_removal`
  (32 to 34).
- **(Proposal)** Joining: a proven non-member's first `Keyring` frame may be a `ClaimInvite`
  without parents (a probe). The relay checks its invite proof, use and expiry and answers with
  the keyring entries only; the joiner then writes its real claim on the heads. Nothing reaches a
  connection that proved neither membership nor a valid invite (criterion 27), and a used or
  expired link is refused before it sees even the keyring (criterion 4).
- **(Proposal)** The snapshot reaches a client as `Record{seq: covers_seq, kind: snapshot}`
  (decision 5 lists no relay snapshot frame); a cursor of 0 or one before the snapshot gets it
  first, then the records after it.
- **(Proposal)** The end of the admission catch-up is the `Pong` to a `Ping` the client sends
  after `Welcome` (the relay answers in order); only then the outbox goes out, rewraps are written
  and a share's first snapshot is uploaded.
- **(Proposal)** A removed identity's later `Hello` is refused with `removed`, not
  `not-a-member` (decision 6a's text), so the device can show the removal notice even when it was
  offline during the removal (criterion 34); it learns nothing else.
- **(Proposal)** `Hello` carries the ed25519 member id (32 bytes); `Peer` reports it too.
  Keyring entries the replay refuses are dropped by the relay, not stored. Invite and remove need
  the connection (`SessionError::Offline`). Any `Refused` ends the session.
- **(Proposal)** Secrets and jitter come from a second caller seed through
  `NonceStream::nonce_at` (a `ChaCha20` keystream), so `sync-core` needs no RNG crate.
- Criterion 18's "byte-identical SVG of one version" is checked here as equal version text and
  equal document JSON (the SVG writer lives in `svg-core`/`ui-core`, not a dependency of this
  crate); the SVG bytes are for the M5/M7 tests. Criterion 18's 5 s is measured from the
  reconnect: after a long outage the backoff (up to 10 s +20 %) can delay the reconnect itself.
  For the lead: a "network is back" input that retries at once would close that gap.
- Long function, justified: `ClientFrame::encode` (65 lines) is one arm per frame.

### M4 (2026-11-03)

- `curvyo-sync-server` (lib for the in-process tests, thin `main.rs`): `config` (hand-parsed
  `--listen`/`--data`, also `--flag=value`, over `CURVYO_SYNC_LISTEN`/`CURVYO_SYNC_DATA`, over the
  defaults), `store` (per room `keyring.log` in the client's `keyring.log` framing, `log` of
  `seq u64, kind u8, len u32, envelope`, `snapshot` as `covers_seq u64` and envelope; `fsync` per
  append, write-temp-then-rename for snapshot and pruned log, directory syncs), `hub` (the one
  thread owning `Relay` and `Store`; outputs carried out in order, a store write synced before
  the sends after it), `server` (axum `/` and `/healthz`, startup scan, shutdown), `health`,
  `log`. Deps: `axum` 0.8 (`ws`, `http1`, `tokio`), `tokio` 1, `anyhow` 1, `getrandom` 0.3
  (challenge nonces; already locked), `thiserror`; dev: `tokio-tungstenite` 0.29 (the version axum
  uses), `futures-util`, `tempfile`, `curvyo-document-core` (dev only). `tokio` 1.53.1 to 1.54.1
  for RUSTSEC `AsyncFd` (cargo deny).
- Tests: `config` and `store` unit tests (round trip, snapshot prune, cut-short ends truncated,
  covered records after a crash skipped, damaged log refused, foreign directories ignored),
  `health` (503 before the scan), `tests/relay_socket.rs` (two real `ClientSession`s over
  WebSockets with real documents holding `ZQX-MARKER-7731` and `#13a7c9`: share, invite, join,
  edits, restart on the same port and directory with an offline edit in between, reconnect and
  convergence within 30 s, close code 1012 seen, stored files only grow, then the byte search of
  the data directory and captured log for the marker, colour, `<svg`, a JSON key and every 16-byte
  fragment of every plaintext and of both documents' JSON; `/healthz` 200, 503 on a read-only
  data directory), `tests/binary.rs` (unix: flags beat environment through the real binary;
  SIGTERM closes two sockets with 1012 and the process exits 0 within 10 s; no client address in
  stdout). Run guide: `docs/guides/sync-server.md`.
- **(Proposal)** Fail-stop: a failed store write or a missing entropy source stops the server
  (logged, non-zero exit); the next start restores from disk and clients resend what was not
  acknowledged. A damaged store file (not merely cut short at its end) refuses the start.
- **(Proposal)** A record or keyring entry cut short at a file's end (a crash mid-append, never
  acknowledged) is truncated by the startup scan; directories not named as a room id (e.g.
  `lost+found` on a PVC) are ignored. `/healthz` checks writability by creating and removing
  `.healthz` in the data directory.
- **(Proposal)** WebSocket text messages close the connection; the axum message limit is
  `MAX_MESSAGE_LEN + 1`, so a message one over the limit still gets `Refused{too-large}`.
- **(Proposal)** The image's `/data` belongs to uid 65532, so a fresh named volume is writable;
  a bind mount needs `chown 65532` (in the guide). Log lines: listening address, room count after
  the scan, shutdown, errors; no client address, no message bytes.
- For the lead (M7 CI): the server's tests run on unix only where they need signals or file
  modes; `ci.yml` does not yet list the server (ubuntu only per 0011 §7) or the edge check
  "server depends on `crypto-core` and `sync-core` only".

### M5 (2026-11-03)

- `curvyo-sync-core`: `ClientSession::network_back` (the browser `online` event) retries a
  waiting reconnect at once from the first delay, so criterion 18's 5 s holds after long outages
  (`session_timing`: merged within 1 s of the network coming back after 40 s offline).
  `InviteLink` writes and reads `curvyo://join?server=…#1.<base64url(room ‖ seed)>`; an
  `https://…?server=…#1.…` shape is read too (UX notes §2.5).
- `curvyo-ui-core`: `presence` (the payload: label, Loro peer id as text, pointer, selection,
  tool, move offset; checks before drawing), `share_text` (colour from the fingerprint by FNV-1a,
  initials, marker, skip and connection texts, server address normalisation, expiry text), and
  `PanelTab::Share` appended last. The strip lists of the tester files of 0043, 0050, 0051 and
  `history_tab_session` gain "share", as they gained Grid and Sketches before (for the lead).
- `curvyo-storage-io`: `credentials` (OS keychain through `keyring` 3 with Secret Service over
  libdbus, macOS Keychain, Windows Credential Manager; else an encrypted key file under the
  maker's passphrase, Argon2id + XChaCha20-Poly1305; ADR 0007 §1). One slot id per data
  directory, so two instances with their own data directories are two devices. `curvyo-app`:
  `read_device_keys(room, passphrase?)` and `write_document_keys(room, keys)`; the title says
  "(shared)" (`set_shared`); CSP `connect-src` allows `ws:` `wss:`. Deps: `keyring` 3.6,
  `argon2` 0.6 (MIT OR Apache-2.0), `chacha20poly1305`, `zeroize`, `getrandom` (already locked);
  dev `tempfile` in the app. `cargo deny` ok.
- `curvyo-editor-wasm`: `session/sync.rs` (the link: share, join, resume, transport inputs,
  invite, share record on save), `sync_events.rs` (session events), `sync_records.rs` (export
  after commits with pictures first, import at frame start, a joiner's snapshot replaces its
  blank document, a member's snapshot merges), `sync_presence.rs` (presence out and in, remote
  move drags drawn live, marks for the overlay, the "is editing this" notice, peer-named undo
  skips, the export's version note), `sync_view.rs` (the Share tab's record), `wasm_sync.rs`.
  Tests `session/sync_tests.rs`: two and three sessions against an in-process `Relay` with the
  host's transport loop (criteria 1, 3, 4, 5, 6 to 8, 14, 15, 17, 18, 21, 22) and the frame-start
  budget of adrs.md decision 9 (one remote edit into 1,000 objects: 2.5 ms in release, asserted
  at 8 ms; idle frame start under 1 µs).
- Frontend: `lib/syncTransport.ts` (+ tests), `hooks/useCollaboration.ts` (50 ms timer, the
  `online` event, settings in `localStorage`, keys from the host, share/join/resume/invite),
  `ShareTab`, `CollaborationPill`, `PresenceOverlay`, `--peer-1` to `--peer-6`; remote records
  are imported in the render loop before `render()`.
- **(Proposal)** Every presence message carries the hello (label and peer id), so no separate
  hello message exists. In-flight geometry (criterion 7) is the move offset of a Select drag;
  resize and rotate drags of a peer show at release.
- **(Proposal)** Server address and name live in the webview's `localStorage` (per data
  directory); the name has no default (no host name in the browser) and is asked once.
- **(Proposal)** Join over unsaved changes raises the File tab's existing check ("…before you
  open another project?"), then a blank project is joined; the Share tab is not given its own
  copy of the check.
- **(Proposal)** Share needs a saved document and a reachable server; once connected the file
  is saved at once, so its `share.json` and `keyring.log` exist (criteria 5, 26). A share that
  never reached the server is left (nothing kept).
- **(Proposal)** Participants are listed own row first, then by member id (the keyring does not
  expose join order); labels of others are learned from presence this run (not persisted).
- The missing-font guard (ADR 0009 §5) is present: Object to path refuses a text with a missing
  face, and measured layout is never stored.
- Open for M6/M7: removal UI (task 25), "Resyncing" after a rejected update (the update is
  dropped and the notice shown once, no resync from the snapshot yet), pictures added offline
  before a reopen are not pushed (only pictures added in the running session go out), the
  browser journey and the other CPU budgets of decision 9.

### M6 (2026-11-03)

- `curvyo-sync-core`: `ClientSession::record_rejected(seq)` and `SessionEvent::Resyncing`. A
  record that does not open (here) or does not import (reported by the caller) is shown once and
  the session reconnects with cursor 0, so the room's snapshot and every record after it arrive
  again (adrs.md decision 2, UX notes §6 "Resyncing"). **(Proposal)** A rejected record up to the
  room's head at the last resync starts no new resync, so a record that stays damaged is skipped,
  not looped on. Frames arriving before `connected` (left over from a closed socket) are ignored.
  Tests: `sim_offline` (altered records: one notice, one resync, the snapshot arrives again;
  caller-side rejection resyncs once and converges).
- `curvyo-ui-core` `share_text`: `ConnectionState::Resyncing` ("Resyncing...", "Resyncing from the
  server's snapshot."), `removal_confirm_text`, `REMOVED_NOTICE`; the expired-link refusal says
  "This link has expired. Ask for a new one." (criterion 3's "ask for a new one").
  **(Proposal)** removal sentence: "Remove Ben? Removing does not reach back: Ben keeps what is
  already on that computer and gets no later changes."
- `curvyo-editor-wasm`: `sync_members.rs` (`remove_participant(member hex)`, `share_labels`,
  `remember_share_labels`); the participant rows carry `member` and `removable` (admin and not
  the own row); import or snapshot failures report the record's sequence to the client session;
  the link shows Resyncing until it is online again. `resume_shared` pushes the referenced
  pictures again when the file holds unsynced edits (**Proposal**, debt entry "A reopened shared
  file resends its pictures after offline edits"). The test network moved to
  `sync_test_support.rs`; `sync_removal_tests.rs` covers criteria 19 (picture added offline),
  25 (once, Resyncing, reconnect, converged), 32 to 34 (Remove only for an admin and not the own
  row, epoch 1, list without B, B told and plain, nothing travels, C across the rotation; removal
  while B offline; Remove refused offline) and the label cache.
- Frontend: Remove (`UserMinus`, 28px row action slot) with the in-place confirm (Cancel focused,
  Remove in the destructive look), its refusal line; `remove` in `useCollaboration`; other
  participants' labels kept in `localStorage` (`curvyo.share-labels`, per data directory) and
  handed to the session on resume and join; the pill spins while resyncing.
- Invite expiry was already complete in M3/M5 (1 h / 24 h / 7 days, relay refusal of used and
  expired links, expiry while waiting); M6 only changed the expired text.

### M7 (2026-11-03)

- Browser journey `frontend/tests/browser/cooperative-editing.cjs` (README entry): the real
  `curvyo-sync-server` on `127.0.0.1:0` with a fresh data directory and two browser contexts on
  the production preview. Covers criteria 1 (share, one row with fingerprint, pill, `share.json` and
  `keyring.log` in the saved file), 2 (24 h link, sentence, secret after `#`), 3 (paste, name,
  join, both lists named), 6 (both ways), 7 (live drag in B, one Move step), 14, 15 (marker), 17
  (pill text within 5 s, B's pointer gone in A), 18 (3 + 3 edits merged), 32 (400 ms guard, epoch
  1, B told, no later change) and a byte search of the relay's files and log (24). Observed wall
  times on the build VM (not gate figures): join 1.3 s, edit visible in the peer about 0.4 s
  (includes a save per poll), presence at once, offline shown after 3.5 s, merge 1.2 to 1.7 s.
- The journey found two defects, fixed with tests: the transport still served the freed session
  after a join replaced it ("null pointer passed to rust", the join never connected), now each
  command attaches the session it acted on; and the room's snapshot opened the joiner on the
  Document tab, now the Share tab stays shown (`a_join_keeps_the_share_tab_shown`; the frontend
  presses Share after the blank project too).
- UX review fix: Remove in the participant confirm ignores presses for 400 ms (design system
  "Inline confirm").
- CPU budgets `curvyo-editor-wasm/tests/sync_cpu_budget.rs` (adrs.md decision 9, asserted at
  four times in release): seal 10 kB 16 µs, open 10 kB 13 µs, presence encode and seal 4 µs, full
  join of 1,000 objects 11 ms; with the frame-start test (2.2 ms) in the release budget job.
- CI: job `dependency-edges` (curvyo dependencies of crypto-core, sync-core, sync-server from
  `cargo metadata`; normal and build edges); the nightly windows and macos host gate excludes
  `curvyo-sync-server` (0011 §7); the release budget job runs the sync budgets.
- Docs: `docs/technical-debt.md` (resize entry replaced by the narrower page-move entry; contested
  scalar mitigated; font guard noted; relay without TLS and quotas, one replica, browser key custody,
  joining needs a member online), `docs/design-system.md` (`--peer-1` to `--peer-6`, Share tab and
  collaboration section), ADR index rows of 0010 and 0011. The licence exceptions of the three new
  crates were already in `deny.toml`.
- Still open: the Docker image build (task 18, no container runtime on the build VM); reviews,
  tester pass and PR (task 29, the lead).

### Review fixes (2026-11-03)

- Key file `device-keys.bin` versioned as ADR 0031 decision 12 describes: format 1 and the pinned
  Argon2id version and costs in the authenticated header, read back on open; room-key values start
  with `version u8 = 1` (`curvyo-storage-io/src/room_keys.rs`). Goldens
  `curvyo-storage-io/tests/fixtures/device_keys_v1.bin` (passphrase in
  `tests/key_file_golden.rs`) and `room_keys_v1.bin`.
- Relay snapshots are packed with `pack_snapshot`, without `share.json` and `keyring.log`.
- Splits: `sync.rs` gives sharing, joining and resuming to `sync_start.rs`; `share_view_json`
  hands rows and the join phase to `participant_rows` and `join_json`; `sync_apply_remote` hands
  one record to `apply_record` and the selection clean-up to `drop_gone_from_selection`;
  `peer_marks` hands the box to `selection_box`; `pages_of_root` hands its key pass to `page_keys`
  and the pre-pages fallback to `first_page_of_old_file`. Each is now under 60 lines.
- `Viewport::pan_by_document_offset` deleted with its unit tests and the ui-core `ac20_*` tester
  tests of 0015; criterion 20 is covered by the session's `ac20_*` tests.
