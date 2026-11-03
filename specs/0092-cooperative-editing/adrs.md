# ADRs for cooperative editing

Architect, 2026-11-02; customer decisions applied 2026-11-03 (COOP-SLICE B, COOP-HOST VM then
Kubernetes, COOP-SCOPE A, COOP-RESIZE B, COOP-JOIN A, COOP-INVITE A). Covers slice 1, which now
includes removal with key rotation (ADR 0031 decision 6a). Leave, re-add with a history choice,
second admin and the verified flow stay slice 2 and need no new ADR (ADR 0010 §5, §7 to §9).
Constants marked Proposal are vetoable at the demo. Criterion numbers below are those of
`specification.md` as updated 2026-11-03 (31 is the concurrent resize, 32 to 34 removal, 35 to 40
hosting and rooms).

**One new ADR (0031, Accepted), one new crate (`curvyo-sync-core`), two crates first built
(`curvyo-crypto-core`, `curvyo-sync-server`), one new binary with a Dockerfile, new crypto and
server dependencies, one document-model change (page frame as one register) and one container
member, together `format_version` 34.**

## Depends on

- [ADR 0031](../../docs/adr/0031-sync-engine-relay-protocol-and-transport.md) (new, Accepted
  2026-11-03): sans-I/O sync engine with the WebSocket in the frontend, `curvyo-sync-core`, rooms as
  opaque ids, the relay frames, the envelope layout with `key_id`, removal with rotation and repair
  by `RewrapEpoch` (6a), presence frames, invite link and join by `RewrapEpoch`, `share.json`, the
  page frame register (9a), snapshots, the server's configuration, health, shutdown, container and
  file store, dependencies.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §1, §3, §5, §13: one file,
  no Loro type in the API, blind relay, self-hostable server. Unchanged.
- [ADR 0008](../../docs/adr/0008-end-to-end-encryption-of-sync-and-collaboration.md) §1 to §7: no
  plaintext path (criterion 28 is a code review item plus a grep test for flags), XChaCha20, AD
  binding (criterion 25), keys as parameters, client-made snapshots, key custody. Unchanged.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §1 to §5: peer-scoped undo
  (21 to 23), presence channel (7, 14 to 16), per-field merge (8 to 12), version reads (29, 30),
  the missing-font rule. Unchanged.
- [ADR 0010](../../docs/adr/0010-document-keyring-admins-and-revocation.md) §1 to §8, §10, §14:
  device keys, keyring log, all of `Genesis`, `AddPendingInvite`, `ClaimInvite`, `RemoveMember`,
  `NewEpoch`, `RewrapEpoch` written in slice 1, causal replay and fork rules, relay membership
  check. Amended by status note (COOP-JOIN A): §7's joiner gets the key from a member's
  `RewrapEpoch`, not from the link; §6's wrap binds the epoch `key_id`.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2 to §4, §6: crate slots for
  crypto-core and sync-server; status note of 2026-11-03 for `sync-core`, the server's second core
  edge, the unbuilt relay socket module in `storage-io`, and `getrandom` on the wasm ban list.
- [ADR 0030](../../docs/adr/0030-stored-tiling-nodes.md): took format 33 on `main`; this story
  takes 34 (provisional, renumbered on rebase if another format change merges first).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) §1, §4, §9, §12: restore-engine undo
  is already peer-safe; the history epoch is compared on import; no document wipe while shared
  (Q3 A); imports never meet a checkout guard.
- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md) §2: assets travel as sealed
  asset records and inside snapshots.

## Feature-local decisions

1. **Where each piece goes.**
   - `curvyo-crypto-core`: envelope seal/open/header, `NonceStream`, identity and invite key
     derivation, wrap/unwrap, fingerprints (first 10 bytes of SHA-256 over both public keys, base32
     in groups of four), epoch `key_id`, keyring entry codec, hashing, signatures, causal replay
     and fork rules, the "who lacks a wrap I can give" query of 0031 6a. Slice 1 writes `Genesis`,
     `AddPendingInvite`, `ClaimInvite`, `RemoveMember`, `NewEpoch`, `RewrapEpoch`; the codec and
     replay know all eight payloads of ADR 0010 §3, since the format is permanent.
   - `curvyo-sync-core`: frame codec, `ClientSession` (connect, auth, cursor, outbox of
     `client_id`s, acks, ping and reconnect timing, snapshot trigger, presence throttle and expiry),
     `RoomLog` (sequence, append, admission rule, removal closes connections, snapshot replace and
     prune).
   - `curvyo-sync-server` (criteria 35 to 40): `axum` routes (`/` WebSocket, `/healthz`), flag/env configuration,
     `tokio` tasks per connection, the per-room file store around `RoomLog`, startup scan to
     recover `head_seq`, SIGTERM/SIGINT shutdown, `Dockerfile` (musl static, distroless
     `static-debian12:nonroot`, `/data` volume, port 8787).
   - `curvyo-editor-wasm`: binding only: feeds frames and time into `ClientSession`, calls
     `export_updates`/`import_updates`/`pack`/`unpack`/`insert_asset`, hands presence to `ui-core`.
   - `curvyo-ui-core`: presence payload types; the "B is editing this" marker (criterion 15).
     No refusal of resize or fit (criterion 31 is the converging frame, COOP-RESIZE B).
   - `curvyo-storage-io`: credential store module (`keyring` crate, encrypted-file fallback).
     `curvyo-app`: one command to read and write a document's keys.
   - Frontend: `WebSocket` open/send/receive/close and a timer tick, nothing else; Tauri CSP
     `connect-src` covers the configured relay URL.
2. **Document-core additions, and only these:** `DocumentVersion::to_text`/`from_text` (ADR 0014
   §7 encoding, already in `history/wipe.rs` and `history/lineage.rs` as crate-private helpers),
   `Document::insert_asset(bytes) -> AssetHash` (stores received bytes under their own hash; an
   asset no node references is dropped on save, as today), the page frame register of ADR 0031 9a
   (`pages.rs`, `document_size.rs`: resize and fit write the frame and move no object; the
   ignored two-resizes test in `tests/acceptance_0015_pr1_tester.rs` is un-ignored and must pass),
   and the container member `share.json` in `container.rs`. One `CURRENT_FORMAT_VERSION` bump,
   33 to 34, with its doc paragraph, migration (old registers read when `frame` is absent),
   damage rules and goldens `page_frame_v34.curvyo` and `shared_v34.curvyo`.
   Also accepted (architect review, 2026-11-03), each a direct consequence of the above:
   `Document::share`/`set_share` and `ShareRecord` (the share record kept with the document and
   written by `pack`, like its assets), `CURRENT_SHARE_FORMAT_VERSION`,
   `Document::fit_page_to_content` (the multi-page fit as one frame write), and
   `Document::resize_page` without its `objects` argument (a resize moves no object).
   `import_updates` stays as it is; its `Pending` and `EpochMismatch` errors make the session show
   "A received update was rejected" once and resync from the snapshot.
3. **Applying remote updates.** Imported at the start of the next frame; one redraw per frame
   whatever the number of records. A drag in progress keeps its in-flight geometry; if the dragged
   object was deleted remotely, the gesture ends on release with the 0009 §2 lazy-resolve rule
   (missing ids are dropped), no error.
4. **History tab labels.** A step whose peer id matches a live participant's hello shows that
   participant's label; otherwise "Another session" (ADR 0014 Q5 A extended; no names in the file).
5. **Participant list** = keyring replay (members, roles, fingerprints) joined with the local
   label cache and the online set from presence. Colour = derived from the fingerprint, so it is
   the same on every device without being stored.
6. **Removal UX facts for the UX notes.** "Remove" is one admin action that writes
   `RemoveMember` and `NewEpoch` and uploads a fresh snapshot; the dialog says in one sentence that
   removal does not reach back (ADR 0010 §8); the participant list shows the new key epoch; the
   removed device shows "You were removed from this document. Your copy stays on this computer."
   and keeps a plain local file. A member who is not an admin sees no Remove action.
7. **Invite UX facts for the UX notes.** Validity 24 h (Proposal); the sentence of criterion 2;
   on the joiner, "Waiting for a member of this document to let you in" until a `RewrapEpoch`
   arrives; refusal texts for `used` and `expired` (criterion 4). The link is pasted into "Open
   shared document"; OS registration of `curvyo://` is not in slice 1 (would add
   `tauri-plugin-deep-link`, MIT/Apache-2.0).
8. **Constants (Proposal):** presence at most every 50 ms, heartbeat 2 s, expiry 5 s; ping 2 s,
   offline after 4 s; reconnect 0.5 s doubling to 10 s with ±20 % jitter (criterion 20's 30 s
   holds); snapshot after 500 records or 4 MiB; maximum message 64 MiB.
9. **Testing.**
   - `crypto-core`: XChaCha20-Poly1305, X25519 (RFC 7748), Ed25519 (RFC 8032) vectors; golden
     envelopes and keyring logs in `tests/fixtures/`; `proptest`: any flipped byte, other room,
     other epoch, other kind is refused (criterion 25); exhaustive keyring graphs up to three
     members for claim, double claim, expired claim, rewrap by a non-member, remove plus rotate,
     remove concurrent with a claim, two concurrent rotations (same epoch number, distinct
     `key_id`), rewrap by a removed member refused.
   - `sync-core`: golden frames; `ClientSession` and `RoomLog` unit tests with a virtual clock;
     `tests/support/sim.rs`, a seeded simulator (latency, drops and reconnects, interleaving
     across connections but in order within one, as TCP gives; partition, relay restart) running two or three real `Document` replicas and one
     `RoomLog` in process. It covers criteria 6 to 12 (convergence), 17 to 20 (offline, 50 + 50
     edits, close and reopen through `pack`/`unpack`, restart), 18 (byte-identical SVG of one
     version), 21 to 23 (undo with a peer), 27 (an uninvited key gets only `Refused` and opaque
     bytes), 13's byte figure (one edit in 1,000 objects pushes at most 10 kB), removal (criteria 32 to 34: the
     removed replica receives no record under the new epoch and its connection is closed; an
     offline member's edits made before it saw the rotation go out under the new key) and
     concurrent resizes (criterion 31: two replicas resize the same page offline; both converge to one whole
     frame and unchanged object positions).
   - `sync-server/tests/`: real WebSocket on `127.0.0.1:0` with a temp data directory: two clients
     replaying golden sealed frames made from a document containing `ZQX-MARKER-7731` and
     `#13a7c9`, a server restart, then a byte search of the data directory and captured logs
     (criterion 24); `/healthz` 503 then 200 (criterion 37); SIGTERM closes sockets with 1012 and exits; flags
     override environment variables.
   - Thread-CPU budgets in `curvyo-editor-wasm/tests/cpu_time/`, asserted at four times: import of
     one remote edit into 1,000 objects 2 ms, seal or open of 10 kB 0.2 ms, one presence message
     encode and seal 0.1 ms, a full join of 1,000 objects (unwrap, open snapshot, `unpack`)
     500 ms (criterion 3's 5 s leaves the rest to the network).
   - Not in the gate: latency at the 95th percentile (criteria 6, 14) and the three-OS webview
     reach of a LAN `ws://` relay; the tester measures them on the LAN demo setup.
10. **Gate.** `curvyo-crypto-core` and `curvyo-sync-core` join the wasm32 build and clippy list of
   `CLAUDE.md` §7 and the 0011 §6 ban list (plus `getrandom`); CI asserts `sync-core`'s only
   `curvyo-*` dependency is `crypto-core` and the server's are those two. The server runs on ubuntu
   only (0011 §7). The Docker image is built by hand for the demo; a CI image build is not part
   of slice 1 (Proposal).

## Technical debt to record when the slice merges

- The relay has no TLS and accepts room creation from anyone who reaches it (LAN `ws://` only;
  quotas, rate limits and the TLS ingress come with the Kubernetes step). The browser build cannot
  join until then.
- The relay's file store allows one replica only.
- Browser key custody undecided (wasm build connects but holds no persistent keys).
- Joining needs a member online (COOP-JOIN A).
- A page move concurrent with a resize of the same page keeps one frame and the move's object
  shift (ADR 0031 9a).
- Resolves: "Resize and fit merge per field across peers" (replaced by the narrower entry above),
  "A contested scalar loses one side's value" (mitigated by criterion 15, entry stays as
  documented behaviour). "Undo cannot reach a collaborator's change" and "A collaborator without
  the font sees substituted text" stay open; the latter's guard (ADR 0009 §5) must exist before
  the demo: a task in `plan.md` if it does not.

## Customer decisions (2026-11-03)

- COOP-SLICE B: removal with key rotation in slice 1 (ADR 0031 6a).
- COOP-HOST: LAN VM first, Kubernetes later; one binary, env/flags, Dockerfile, `/healthz`, one
  data directory, graceful shutdown; plain `ws://` on the LAN for the desktop demo, no self-signed
  certificate; the browser needs `wss://` behind the later ingress (ADR 0031 decision 11).
- COOP-SCOPE A: one document = one room; room ids are opaque, nothing assumes one per server or
  user (ADR 0031 decisions 5 and 8).
- COOP-RESIZE B: page frame as one atomic register, format 34 (ADR 0031 9a).
- COOP-JOIN A: ADR 0010 §7 amended by status note.
- COOP-INVITE A: link only in slice 1.
