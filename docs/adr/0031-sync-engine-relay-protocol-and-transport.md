# ADR 0031: Sync engine, relay protocol and transport for cooperative editing

**Status:** Accepted (customer, 2026-11-03: COOP-SLICE B, COOP-HOST local VM first then
Kubernetes, COOP-SCOPE A, COOP-RESIZE B, COOP-JOIN A, COOP-INVITE A). Was `needs-customer`
(persistence/sync, document model). Asked by `specs/0092-cooperative-editing`. Amends ADR 0010 §6
(epoch key id in the wrap's associated data), §7 (the last sentence of the link flow) and §5 (deny
wins over the epoch tiebreak, stricter validity rules) by status notes there; amends ADR 0011 §2/§3/§4/§6 by a note there. Everything not listed in "What is new for
the customer" executes ADRs 0004, 0008, 0009, 0010 and 0014 as accepted. Architect note
2026-11-03: decisions 1, 5, 6, 6a, 12 and 13 match the built protocol of `0092` (join probe,
snapshot as `Record`, `Pong` ends the catch-up, `removed` refusal, wrap layout, key file format,
dependencies).

**Superseded in part by [ADR 0044](0044-workspace-only-unit-of-sharing.md)** (customer, 2026-11-06 DOC-SHARE-REMOVE B, accepted 2026-11-09): decision 2 (one connection per synced workspace, not per open shared document), decision 5's "one document per room" (rooms are workspace rooms and the device group room), decision 8 (`1.` links are refused on the device) and decision 9 (container `share.json` v1 and `keyring.log` are no longer written and read as a local document). Protocol, envelope, relay, snapshots and keys are unchanged.

## Context

ADRs 0004/0008/0009/0010 fix *what* cooperative editing is: Loro replicas, a blind relay that
stores client-sealed snapshots and a sealed update log, XChaCha20-Poly1305 envelopes, a signed
keyring log, peer-scoped undo, a separate sealed presence channel. They leave open how bytes move:
the update API, the wire protocol, the transport, the invite format and the offline path
(spec 0092, "Architect notes").

Three facts from `main` shape the answer:

- **The document lives in the webview, not in the host.** `curvyo-app` holds no `Document`
  (its `Cargo.toml` says so); the frontend's `WasmSession` owns it, in the desktop app and in the
  browser alike. ADR 0011 put the "relay socket" in `curvyo-storage-io` before that was true.
- **The update seam exists.** `Document::export_updates(since)` / `import_updates(bytes)`
  (`history/updates.rs`) already exchange framed Loro updates without a Loro type in the API, and
  compare the ADR 0014 §9 history epoch first.
- **Assets are outside the Loro log** (ADR 0017): an image's bytes live in the container's
  `assets/`, only the hash is in the document. A collaborator needs the bytes too.

### Options — where the socket and the sync logic live

**A. Sans-I/O sync engine in Rust, WebSocket in the webview** (chosen). A pure crate holds the
wire frames and the client's connection state machine (bytes and time in, bytes and events out);
the editor wasm module drives it; the frontend owns only a `WebSocket` object that hands binary
messages across. The same code path serves the desktop app and the browser build. All protocol
logic stays in Rust and in the Rust gate; the TypeScript part is open/send/receive/close.

**B. Socket in the host (`curvyo-storage-io`, `tokio-tungstenite`), bytes relayed to the webview
over Tauri events.** What ADR 0011 wrote down. Rejected: every update crosses IPC twice more, the
browser build needs option A anyway, so there would be two transports to test, and it pulls
`tokio` into the app for one socket.

**C. Loro's own sync protocol / a third-party relay (y-websocket style).** Rejected: they assume
the relay can read and merge state, which ADR 0008 §6 forbids.

### Options — where the wire types live

The server may depend on `curvyo-crypto-core` and nothing else from core (ADR 0008 §5, 0011 §4).
The frames must be shared by server and client, or they drift.

**A. A new crate `curvyo-sync-core`** that depends only on `curvyo-crypto-core` (chosen). It holds
no document type, so 0008 §5's reason holds: a document-model change cannot force a server deploy.
**B. Put the frames into `curvyo-crypto-core`.** Rejected: that crate is in the path of every byte,
needs a focused security review (ADR 0008 consequences) and must stay small; a connection state
machine is not cryptography. **C. Duplicate the frames in server and client.** Rejected for the
reason 0008 §5 gives.

### Options — how an invite link admits a joiner

**A. The link carries only an invite secret; an online member completes the join** (chosen,
customer 2026-11-03, COOP-JOIN A). The joiner writes `ClaimInvite`; any current member online writes `RewrapEpoch`
to the joiner's key — both payloads exist in ADR 0010 §3, no new payload. A link that leaks after
use, or a used link plus everything the relay stores, opens nothing. Cost: a join completes only
while a member is online (the spec's demo has A online).
**B. As ADR 0010 §7 literally says: the joiner "derives the epoch key from the invite".** Joining
works with nobody online, but the link then is the epoch key: a used or expired link plus the
relay's data reads the document, and "one-time" is enforced only by the server, which ADR 0010 §10
says is not the security boundary.

### Options — page resize and fit while others are connected (COOP-RESIZE)

Today a resize or fit is one commit that writes the size registers and shifts every contained
object (`pages.rs`, `document_size.rs`); across peers Loro merges each register alone, so two
concurrent resizes leave a size from both and positions from one (`docs/technical-debt.md`).

**A. Refuse resize and fit while another participant is connected.** No format change, but the
offline-merge case stays broken. Rejected by the customer (2026-11-03).
**B. A page's frame is one atomic register; a resize moves the frame, not the objects** (chosen,
COOP-RESIZE B). Concurrent resizes converge to one whole frame (last writer wins, Loro's map
register rule) and no object moves, so frame and content never disagree. Format change.
**B'. Two registers, origin and size.** Rejected: a resize must write both to keep content centred,
so a concurrent move plus resize would merge to an origin from one and a size from the other.

## Decision

1. **Crates and edges.** `curvyo-crypto-core` (authorized by 0008 §4) and `curvyo-sync-server`
   (0004 §5) are created; **`curvyo-sync-core` is new** (authorized here, `CLAUDE.md` §5). Its
   responsibility: *it defines the relay protocol as pure frames and state machines over bytes and
   passed-in time.* Edges: `sync-core → crypto-core` only (invariant, like 0011 §4);
   `sync-server → crypto-core, sync-core`; `editor-wasm → crypto-core, sync-core`;
   `storage-io` has no `curvyo-*` edge for the credential store: its key file uses
   `chacha20poly1305` and `argon2` directly (note 2026-11-03, decision 12). `ui-core` gains nothing but the presence
   payload types 0011 already gave it. Amends 0011 §3/§4 by note: the server's second core
   dependency is `sync-core`, and only it.
2. **Transport: WebSocket in the frontend** (option A), one connection per open shared document,
   binary messages, one frame per message. The relay socket module of 0011 §2 in `storage-io` is
   not built. Desktop and browser share the path; the browser additionally needs `wss://`.
3. **Document API: the existing seam, plus two small additions.** Sync calls
   `export_updates(since)` and `import_updates(bytes)` unchanged; `DocumentVersion` gains a public
   text encoding (the ADR 0014 §7 frontier encoding); `Document` gains `insert_asset(bytes)` that
   checks the bytes against their hash. The page frame of decision 9a is the only change to the
   document model. Snapshots are `container::pack`/`unpack` bytes, so a joiner
   receives assets with them. No Loro type crosses the API (0004 §3). Remote updates are imported by
   the session between calls on the single wasm thread, so no import meets a checkout guard
   (0014 §12), a batch, or an export or job read (0009 §4). `SyncError::Pending` is impossible
   with an honest relay (decision 5) and is handled as a rejected update plus a resync from the
   snapshot.
4. **Peer ids and history.** The Loro peer id stays a fresh random `u64` per session (0014 §4),
   minted by the frontend. It is the envelope's sender peer id (0008 §3) and is announced, sealed,
   in the presence hello, so the History tab can name a live collaborator's steps. Undo stays the
   0014 restore engine: remote steps never enter my stacks; a step whose objects a peer deleted is
   skipped (0009 §1). Document wipe stays unavailable in a shared document (0014 Q3 A); the 0014
   §9 history epoch stays in the sealed payload and is compared on import.
5. **Relay protocol.** A room is an opaque 16-byte id; the relay knows rooms, never documents,
   and holds any number of them for any number of clients. In slice 1 one document is one room and
   its room id is the document id from `Genesis` (COOP-SCOPE A); nothing in the protocol, the
   server's store, the invite link or the client's credential store may assume one room per server
   or per user, so a later workspace share (several documents) is a new link version and new rooms,
   not a protocol change. The
   relay assigns each stored record a gapless `u64` sequence per room. A pushed update depends only
   on records its client already received from this log and on its own earlier pushes, which the
   relay stored first, so log order is a causal order and Loro never sees missing dependencies. Frames (one tag byte, little-endian
   fields, hand-written codec, golden files):
   - client → relay: `Hello{protocol, room, since_seq, identity_pub}` (the 32-byte ed25519
     member id; `Peer` reports the same id), `Proof{signature}`,
     `Keyring{entry}`, `Push{client_id, kind, envelope}` (kind: update or asset),
     `Snapshot{covers_seq, envelope}`, `Awareness{envelope}`, `Ping`;
   - relay → client: `Challenge{nonce}`, `Welcome{head_seq, snapshot_seq, connection}`,
     `Keyring{entry}`, `Record{seq, kind, envelope}` (kind: update, asset or snapshot; the room's
     snapshot reaches a client as `Record{seq: covers_seq, kind: snapshot}` before the records
     after it), `Ack{client_id, seq}`,
     `Peer{connection, identity_pub | left}`, `Awareness{connection, envelope}`,
     `Refused{reason}`, `Pong`.
   Auth: the client signs `"curvyo-relay-v1" ‖ room ‖ nonce` with its ed25519 key; the relay admits
   it if the keyring it holds lists that key as a current member (0010 §10, defense in depth), or
   if the connection's first `Keyring` frame is a valid, unexpired `ClaimInvite` (relay clock), or a
   valid `Genesis` for an unknown room. A proven non-member's `ClaimInvite` without parents is a
   probe: the relay checks its invite proof, use and expiry, answers with the keyring entries only
   and stores nothing; the joiner then writes its real claim on the keyring's heads. `Refused`
   carries `used`, `expired`, `not-a-member`, `removed`, `too-large`, `protocol`; any `Refused`
   ends the client's session. Maximum message 64 MiB (Proposal, above the image import ceiling).
   After `Welcome` the relay sends every keyring entry the client lacks before any `Record`, so a
   client always seals with the current epoch (decision 6a). The relay handles a connection's
   frames in order, so the `Pong` to a `Ping` the client sends after `Welcome` marks the end of
   the catch-up; only then does the client send its outbox, write rewraps or upload a share's
   first snapshot.
6. **Envelope** (in `crypto-core`, 0008 §2–§3): cleartext header `version u8, kind u8, flags u8,
   epoch u32, key_id [8], document_id [16], sender_peer u64`, all of it associated data; `nonce [24]`;
   ciphertext. Kinds: update, asset, snapshot, presence, so a presence message cannot be replayed
   as an update. Snapshots set the signed flag and carry the author's ed25519 signature
   (0010 §11). Nonces come from a per-session ChaCha20 stream seeded with 32 bytes of browser
   entropy (`NonceStream` in `crypto-core`, a pure function of seed and counter); other entropy
   (identity keys, wrap ephemerals, invite seeds) is passed in the same way. `crypto-core` never
   draws randomness or reads a clock. `key_id` is the first 8 bytes of
   SHA-256(`"curvyo-epoch-id-v1"` ‖ epoch key): two concurrent rotations both create epoch *n+1*
   (0010 §5), and the id tells their keys apart without a payload change. A wrap's associated data
   binds `document_id ‖ epoch ‖ key_id ‖ recipient` (amends 0010 §6), the recipient being both
   public keys of its identity. A wrap is 88 bytes, `key_id[8] ‖ ephemeral X25519 public[32] ‖
   sealed epoch key[32] ‖ tag[16]`; the `key_id` is in clear (as in the envelope header), so
   replay and recipients know which key of an epoch number a wrap holds.
6a. **Removal with key rotation is in slice 1** (COOP-SLICE B; 0010 §8 executed as written).
   - *Writing.* An admin's "Remove" pushes two keyring entries in one `Keyring` frame sequence:
     `RemoveMember{identity}`, then `NewEpoch{epoch: highest known + 1, wraps}` with the first as
     parent, wrapping a fresh key (entropy passed in) to every remaining member's X25519 key and
     not the removed one. Then it seals a signed snapshot under the new epoch and uploads it
     (decision 10, regardless of the thresholds), so the relay prunes the old log.
   - *Sealing.* A client seals each push at send time with the current epoch of its replayed
     keyring, so edits made offline before it learned of a rotation go out under the new key.
     Records under an older epoch stay accepted and readable (0010 §9): a member who was offline
     legitimately holds that key.
   - *Relay.* When it appends a valid `RemoveMember`, it closes that identity's open connections
     with `Refused{removed}` and refuses its later `Hello` with `removed` as well, so a device that
     was offline during its removal still shows the removal notice (criterion 34); it learns
     nothing else. Defence in depth only (0010 §10).
   - *Repair by `RewrapEpoch`.* A member whose replay shows another current member without a wrap
     for an epoch the first holds writes `RewrapEpoch{epoch, identity, wrap}` for it. This one rule
     covers a join completing concurrently with a rotation, two admins rotating at once (the losing
     key stays readable, the surviving one is rewrapped), and the join of decision 8. Duplicate
     rewraps from two members are harmless; replay keeps the first by entry hash.
   - *Removed device.* Its client sees its own `RemoveMember` (or `Refused{removed}`), stops
     syncing and keeps the file as a plain local document (R-COLLAB-010). Leave, re-add with a
     history choice, second admin and the verified flow stay in slice 2.
   - *Tests.* Exhaustive keyring graphs up to three members add remove, remove plus concurrent
     claim, concurrent rotations and rewrap by a removed member; the simulator shows the removed
     replica receives no record sealed under the new epoch.7. **Presence** (0009 §2): sealed `Awareness` frames, relayed to the room's other connections,
   never stored, never logged. Each message is the full current state (pointer, selection ids,
   active tool, in-flight transform of a drag), sent at most every 50 ms while it changes and every
   2 s as a heartbeat; a peer silent for 5 s, or reported `left`, disappears. The first message
   after connecting is a hello with label, Loro peer id and identity; a hello whose identity differs
   from the relay's `Peer` report is ignored. Payload types live in `ui-core`, JSON-encoded inside
   the envelope (not persisted, so its format can change freely behind its version byte). **Device
   labels never travel unsealed**: they are presence data, cached per device outside the document.
8. **Invite link** (option A; link only, COOP-INVITE A): `curvyo://join?server=<url-encoded relay
   URL>#1.<base64url(room_id ‖ invite_seed[32])>`. Version `1.` means "one document, room id = its
   document id"; a workspace invite will be a new version. The seed derives (HKDF-SHA-256) an ed25519 invite key whose public half is
   the `invite_id` of `AddPendingInvite{invite_id, role: member, expires}`. `ClaimInvite.proof` is
   the invite key's signature over `document_id ‖ invite_id ‖ identity`. The keyring replay accepts
   the first claim per invite (concurrent claims: lowest entry hash wins, 0010 §5 style); the relay
   refuses expired ones; a completing member checks expiry with its own clock before it writes
   `RewrapEpoch` for every epoch it holds (full history, 0010 §9 default; the rule of 6a). Slice 1
   pastes the link;
   registering `curvyo://` with the OS is a later choice (a Tauri deep-link plugin).
9. **Offline and reconnect.** There is no separate queue: the Loro log is the queue. The container
   gains a member `share.json` `{format_version, server, room, server_seq, synced_version}`, plain
   on local disk like everything else (0008 §7); with `keyring.log` (0004 §1) it makes a file
   "shared". On connect the client receives records after `server_seq`, then pushes
   `export_updates(synced_version)` and any asset not yet acknowledged; an `Ack` advances
   `synced_version`. A stale value only resends bytes Loro ignores as known. Liveness: ping every
   2 s, offline after 4 s without a pong; reconnect after 0.5 s doubling to 10 s, jittered.
   `share.json` and decision 9a share **one** `format_version` bump, to **34** (main is at 33 after
   tiling, ADR 0030; provisional by the renumbering rule in `document.rs`), so an older build
   refuses a shared file instead of saving it without its keyring. `share.json`'s own
   `format_version` field starts at 1.
9a. **Page frame as one register** (COOP-RESIZE B; document model). Each page's `x`, `y`, `width`,
   `height` registers (the first page's root `width_mm`/`height_mm` among them) are replaced by one
   register `page.<id>.frame` holding the map `{x, y, width, height}` in mm, always written whole
   (like ADR 0030's `tiling`). Resize keeps the frame's centre and writes the new frame; fit writes
   the content bounds as the frame; neither moves an object. `move_page` writes the frame and
   translates the page's objects in the same commit. Positions shown to the user, rulers, grids,
   SVG export and jobs read objects relative to the frame's origin, so a resize looks as it does
   today in page coordinates; every reader that assumes the first page sits at (0, 0) reads its
   frame instead. Concurrent resizes and fits converge to one whole frame. A page move concurrent
   with a resize of the same page keeps one frame and the move's object shift; that residue is
   recorded as debt (narrowed from today's entry). Migration from 33: a missing `frame` is read
   from the four old registers; nothing is rewritten until the page is next edited. Damage: a frame
   that is not a map of four finite numbers or has a side outside the document size range.
   Golden: `page_frame_v34.curvyo`. Spec criterion 31 (converging frame) replaces the draft's refusal.
10. **Snapshots** (0008 §6). After `Welcome` or a push, a client whose room log since the last
    snapshot exceeds 500 records or 4 MiB (Proposal) seals a signed snapshot of its replica
    (which holds every record up to its cursor) and uploads it with `covers_seq`; the relay keeps the newest and,
    because a Loro snapshot holds the full log, prunes records up to `covers_seq`. Duplicate uploads
    from two clients are harmless.
11. **Server** (COOP-HOST: a VM on the maker's LAN first, Kubernetes later). `curvyo-sync-server`,
    one binary: `axum` with WebSockets on `tokio`.
    - *Configuration* by flag or environment, flag wins, hand-parsed: `--listen` /
      `CURVYO_SYNC_LISTEN` (default `0.0.0.0:8787`), `--data` / `CURVYO_SYNC_DATA` (default
      `/data`). Nothing else; no config file.
    - *Storage*: everything under the one data directory (a PVC later), one subdirectory per room
      (hex room id): `keyring.log`, an append-only `log` (seq, kind, length, envelope; `fsync` per
      append) and `snapshot` (write-temp-then-rename). A startup scan recovers each `head_seq`. One
      process owns the directory: one replica, no shared state.
    - *`GET /healthz`*: 200 once the startup scan is done and the data directory is writable, 503
      otherwise; it reveals no room data.
    - *Graceful shutdown* on SIGTERM and SIGINT: stop accepting, close every socket with WebSocket
      close code 1012 (clients reconnect with their normal backoff), finish in-flight appends, exit
      within 10 s.
    - *Container*: a multi-stage `Dockerfile` in the crate directory, building a static
      `x86_64-unknown-linux-musl` binary and copying it into `gcr.io/distroless/static-debian12:nonroot`;
      `/data` is the volume, port 8787. Logs go to stdout.
    - *Privacy*: it decrypts nothing, parses no document, writes no IP addresses or request bodies
      to its logs.
    - *TLS*: none in the binary. The slice 1 demo runs plain `ws://` on the LAN for the desktop app;
      a self-signed certificate is not used, because every webview and browser would have to be
      told to trust it. The browser build needs `wss://` (mixed content) and so connects only once
      the server sits behind a TLS-terminating ingress or reverse proxy (the Kubernetes step).
    - Any client may create a room on it.
12. **Keys on the desktop.** Device identity keys and epoch keys live in the credential store of
    ADR 0007 §1, built now in `storage-io` (`keyring` crate, native backends; else an encrypted
    key file), and are handed to the wasm session through one host command when a shared document
    opens. Browser key custody is decided with the browser story.
    *Key file* (note 2026-11-03; the fallback is our own format, not the `keyring` crate's):
    `device-keys.bin` in the data directory starts with `CVYKEYS1`, then a header of
    `format u8 = 1`, the Argon2id parameters `version u32 = 0x13, m_cost u32 (KiB), t_cost u32,
    p_cost u32` and `salt[16]`, then `nonce[24]` and the XChaCha20-Poly1305 ciphertext and tag of
    the entries. The parameters are fixed constants of this build and are read back from the
    header, never from the crate's defaults, so a dependency update cannot lock a maker out. Each
    room's epoch keys are one stored value starting with a `version u8 = 1`, followed by
    `epoch u32 ‖ key[32]` repeated. Golden files in `curvyo-storage-io/tests/fixtures/`.
13. **Dependencies** (all pass the current `deny.toml` allow list). `crypto-core`:
    `chacha20poly1305` (+`chacha20`), `hkdf`, `sha2`, `zeroize` (MIT/Apache-2.0), `ed25519-dalek`,
    `curve25519-dalek` for X25519 (BSD-3-Clause; implementation note 2026-11-03: `x25519-dalek` 3 pulls
    `rand_core` unconditionally, so the Montgomery ladder of `curve25519-dalek` is used directly), all
    `default-features = false`, no `rand_core`, no `getrandom`.
    `sync-core`: none beyond `crypto-core` and `thiserror`. Server: `axum` (`ws`), `tokio`, `anyhow`
    (MIT), `getrandom` (challenge nonces). `storage-io`: `keyring` (MIT/Apache-2.0), `argon2`,
    `chacha20poly1305`, `zeroize`, `getrandom` (MIT/Apache-2.0) for the key file. The current
    RustCrypto lines bring `sha2` 0.11 next to the workspace's 0.10 (debt entry). CI adds `getrandom` to the 0011 §6 wasm ban
    list for `crypto-core` and `sync-core`.
14. **Testing without sockets first.** Crypto by published vectors, golden envelopes and
    exhaustive small keyring graphs; the relay's room rules as a pure `RoomLog` in `sync-core`, so the
    server is an I/O shell around it; a deterministic, seeded network simulator in `sync-core/tests/`
    driving real `Document` replicas (`document-core` as a dev-dependency only); one real-socket test
    in the server crate; thread-CPU budgets in `curvyo-editor-wasm/tests/cpu_time/`. Details and
    figures: `specs/0092-cooperative-editing/adrs.md`.

## What is new for the customer

Already accepted: a server exists, is open source and self-hostable (0004 §5, §13); edits,
snapshots and presence leave the machine sealed (0008); per-device keys and the metadata the relay
sees (0010). Decided 2026-11-03: the maker runs `curvyo-sync-server` on a LAN VM, as a binary or
container, later on Kubernetes behind an ingress (COOP-HOST); removal with key rotation is in slice
1 (COOP-SLICE B); one document per room for now (COOP-SCOPE A); page resize and fit work live and
store the page frame as one register, with `share.json`, in format 34 (COOP-RESIZE B); joining
needs a member online (COOP-JOIN A); invite by link only (COOP-INVITE A). Images in a shared
document are uploaded sealed as asset records.

## Consequences

- One code path for desktop and browser, all protocol logic testable without sockets, and the
  server stays document-blind. The WebSocket and its reconnect timer are TypeScript, outside the
  Rust gate; they hold no protocol logic, and the simulator covers the state machine they drive.
- The webview holds private keys in wasm memory while a shared document is open. On the desktop
  that is our own code in our own process; the browser story must decide custody before it ships.
- A Tauri webview must be allowed to reach the configured relay (`connect-src`); mixed-content
  rules make a browser build need `wss://`, so the browser cannot join the LAN `ws://` demo.
  Checked by hand on all three OSes in the slice's demo.
- Resize no longer shifts objects; the page frame moves around them. Same picture in page
  coordinates, one register per page, and a format bump with a read-only fallback for version 33.
- No one-time invite works while every member is offline (option A). The joiner sees "Waiting for
  a member of this document to let you in".
- Assets travel whole in the log; a large image costs its size once per room. Asset dedup and
  pruning ride on snapshots.
- The relay accepts room creation from anyone who can reach it. Fine on a LAN; a hosted instance
  needs quotas and rate limits first. Recorded in `docs/technical-debt.md` with the absent TLS and
  the single-replica file store.
- `sync-core` is one more crate and a second core dependency of the server. Its invariant
  (only `crypto-core`) is asserted in CI the same way as 0011 §4.
