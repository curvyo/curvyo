# ADR 0040: Own-devices sync: device group room, pairing by code, admission by reconciliation

**Status:** Accepted 2026-11-04 (customer; ADR-0040 A, BROWSER-SLICE A: the browser pairs as a
device in slice 1; Frage 145 A, 2026-11-04: passkey backup of a browser device, decision 13;
Frage 146 GROUP-DEADLOCK unanswered, default A, decision 8). Amended 2026-11-04 after the
architecture, crypto and UX reviews (decisions 1, 3, 5, 8, 9, 10, 12, 13). Asked by
`specs/0096-own-devices-sync`. Executes the customer answers of
2026-11-04: DEV-IDENTITY A (per-device keys linked by pairing), PAIR-METHOD A (code on the old
device, four check words), SYNC-SCOPE B (switch per workspace), DEV-ROLE A (the admitting
device's role), DEV-LOST A (no recovery code in slice 1), DEV-UI A (Share tab), DEV-BROWSER B
(the browser pairs as a device). Extends ADR 0031 (a pairing connection and backup connections,
relay protocol 3) and
ADR 0037 (synced workspaces are shared workspaces). Keeps ADR 0010 unchanged: no new keyring
payload. Amends ADR 0008 §9 by a note on acceptance: own-device sync uses per-device wraps in
workspace rooms, not a per-user sync key; the recovery key stays unbuilt (DEV-LOST A). Choices
marked (Proposal) are the architect's, vetoable at the demo.

## Context

A synced workspace is an ADR 0037 shared workspace whose members include every computer of the
maker. Three things are missing: (1) a durable, sealed place that says which devices belong
together and which workspaces they sync, readable by a device that was offline (criterion 11);
(2) a pairing that links a new device's own keys without a server account, without a private key
leaving its device (criterion 3) and without the relay learning the code; (3) a rule for who
writes the keyring entries that admit or remove a device in each workspace room. The relay stays
blind (WS-META) and is not an authority (ADR 0010 §10).

### Options: where the device group lives

**A. A device group room** (chosen). An ordinary ADR 0031 room with its own keyring (genesis by
the first device, every paired device a member) and one sealed stream: the devices list. A device
that was off reads it when it connects, like any room. No relay change, no new payload.
**B. Device certificates inside every workspace keyring** (a new `LinkDevice` payload). Rejected:
a new authorization rule in every keyring (ADR 0010 §3), shown to other makers in shared
workspaces, and it still gives an offline device no way to learn which rooms exist.
**C. No shared state; a per-device inbox room.** Each device announces new workspaces to every
other device's inbox. Rejected: N inboxes, N-squared messages, and removal must reach each inbox;
A is the same mechanism with one room.

### Options: pairing protocol

**A. A PAKE over the relay with the code as password, plus check words** (chosen). The code
authenticates the key exchange online only; an offline guess is impossible and the relay cannot
swap keys without failing the PAKE.
**B. Unauthenticated key exchange with commitment and check words only** (ZRTP style). Secure only
if the maker really compares the words; a relay that swaps keys wins whenever the maker skips that
step. Rejected as the only defence; its check words are kept on top of A.
**C. Key derived from the code (HMAC or AEAD keyed by the code).** Rejected: 40 bits can be guessed
offline from one captured transcript in hours, by the relay operator.
For A: **CPace on ristretto255** (CFRG-selected, draft test vectors) needs only `curve25519-dalek`
and `sha2`, which `crypto-core` already has. The `spake2` crate was rejected: it brings
`rand_core` into `crypto-core`, which takes entropy as parameters (ADR 0031 decision 13), and
SPAKE2 needs per-group constants.

### Options: browser key custody (DEV-BROWSER B)

Facts as of 2026: Ed25519 and X25519 are in WebCrypto in current Chromium, Firefox and Safari;
WebCrypto has no XChaCha20-Poly1305, so epoch keys must be raw bytes in wasm in any case. A
non-extractable `CryptoKey` survives in IndexedDB by structured clone, but the browser keeps its
bytes in the profile on disk: non-extractable stops JavaScript from reading the key, not a copy of
the profile. `navigator.storage.persist()` is granted silently by heuristics (Chromium), by prompt
(Firefox), and does not lift Safari's rule that deletes script-written storage of a site not
visited for 7 days (installed web apps excepted). "Clear site data" deletes keys and the browser's
copy of every workspace together. WebAuthn PRF (passkeys, Chromium, Safari 18, Firefox 135+,
provider-dependent) yields a stable 32-byte secret per passkey, which could wrap the keys, but the
wrapped blob must then live outside the browser's storage, i.e. on the server under the passkey's
credential id: a sign-in with a server-side record, an account without email or password.

**A. Keys in IndexedDB, wrapped by a non-extractable AES-GCM key, persistence requested**
(chosen for the first browser slice). No account. A browser that forgets is a lost
device: removed from another device, paired again.
**B. A plus a passkey-wrapped key backup on the server.** Restores a browser after its data was
cleared; reopens ADR 0007's no-accounts boundary (a persistent record the maker signs in to), and
a self-hosted server needs its own domain for passkeys. Compared below (Frage 145).
**C. A plus a passphrase-protected backup file the maker downloads** (the desktop key file of
ADR 0031 decision 12). No server record; one more file and passphrase to keep.
**D. Identity keys as non-extractable WebCrypto Ed25519/X25519 keys.** Rejected: signing and
unwrapping become asynchronous calls outside wasm, which needs a signer trait in the session and a
second code path, while epoch keys remain raw in wasm anyway; it protects one key out of many.

## Decision

1. **Device group room.** "Add my computer" on a device without a group mints a 16-byte group id
   and, when the maker presses Pair (not before, Proposal), writes `Genesis{document_id = group id,
   admins = [self]}`. Every paired device is added with `AddMember{role: admin, wraps: every epoch
   the writer holds}` (DEV-ROLE A: the first device is admin, so every device is). The room's
   sealed records carry one stream, the **devices list**: module `devices_list` in
   `curvyo-document-core`, a Loro map, no Loro type in its API, `format_version = 1`:
   `devices.<member id hex> = {label, browser?: bool}` and `workspaces.<workspace id hex> =
   {name, on: bool}` (registers, last writer wins). `browser` is optional and absent means false:
   a browser device marks its own entry, so other devices show it with the browser glyph and
   "Browser, " on its row. Readers ignore fields they do not know, so this needs no new
   `format_version`; a build without it drops the mark when it renames that device (amended
   2026-11-04, UX review 11). Snapshots and pruning as ADR 0031 decision 10. One group per
   device; "Join my devices" is unavailable while a device is in a group (merging groups is out of
   scope, Proposal).
2. **Local files.** Desktop: `devices/` in the app data directory with `group.json`
   `{format_version: 1, server, room, server_seq, synced_version, synced_folder, folders:
   {workspace id: path}}`, `keyring.log`, `devices.loro` (temp-then-rename). The group's epoch keys
   are one credential-store value in the ADR 0031 decision 12 room-keys encoding. Browser:
   decision 9. Never inside `.curvyo-workspace/` or a `.curvyo` file (criterion 31).
3. **Pairing connection** (relay protocol 3; protocols 1 and 2 still admitted). A pairing uses its
   own WebSocket whose first frame is not `Hello`. Frames: client → relay `PairOpen`,
   `PairJoin{slot u16}`, `PairMsg{bytes}`, `PairClose{reason}`; relay → client
   `PairSlot{slot, expires_in_s}`, `PairJoined{sid[16]}` (to the joiner and the opener),
   `PairMsg{bytes}`, `PairClose{reason}` (the other side's close forwarded, or `gone` when its
   connection ended), `Refused{unknown | expired | used | locked | busy}` (`busy`: the slot has a
   joiner already, or the relay holds its most slots; Proposal, implementer 2026-11-04). Close
   reasons: `wrong`, `cancelled`, `locked`, `done` (opener), `gone` (relay). The relay draws a
   free random slot below 32,768, a fresh random `sid` for every join on it (so no two `CPace`
   runs on one slot share a `sid`, as the CPace draft recommends; amended 2026-11-04), forwards `PairMsg` between the opener and the
   one current joiner, and keeps slots in memory only: never on disk, never logged. A room
   connection's `Hello` names protocol 3, so a relay that was not upgraded refuses every room
   connection of an upgraded app: **upgrade the server before the apps** (Proposal; the release
   notes say so). Limits (Proposal): slot lifetime 5 minutes by
   the relay's clock; one slot per connection; 5 joins per slot; 4,096 open slots per server; a
   slot whose pairing finished answers `used` until its lifetime ends, then `expired` for one
   more lifetime, then `unknown`; a pairing message is at most 4 KiB. Pure `PairSlots` in
   `curvyo-sync-core::relay`, clock passed in; the server is the I/O shell.
4. **Code and protocol.** The code is 8 Crockford base-32 characters (no I, L, O, U; input folds
   case, I/L to 1, O to 0), shown `XXXX-XXXX`: the first 3 characters are the relay's slot
   (15 bits), the last 5 a secret drawn by the opener (25 bits) that never reaches the relay.
   Messages inside `PairMsg`, joiner B and opener A:
   1. B → A: CPace share `Yb`; A → B: `Ya`. CPace-ristretto255-SHA512 with password = the
      25-bit secret, `sid` from this join's `PairJoined`, channel identifier
      `lv_cat("curvyo-pair-v1", server URL)` (the length-value concatenation of the draft).
      Module `pake` in `curvyo-crypto-core`, scalars from passed-in entropy, draft test vectors.
   2. From the shared key: HKDF-SHA-256 (salt `sid`, input ISK) to `confirm_a`, `confirm_b`,
      `seal_a`, `seal_b`, `words`; the confirmation tags are HKDF outputs with the transcript in
      their info, compared in constant time. B → A: `confirm_b`, then sealed (XChaCha20-Poly1305)
      `{B's identity (ed25519 + X25519 public), label, B's signature over the transcript}`.
   3. A checks the MAC. Wrong: `PairClose{wrong}` to B; after 5 wrong in a row A closes the slot
      as `locked` (criterion 4). Right: A shows B's label, fingerprint and the check words and
      sends `confirm_a` plus its own sealed identity, label and signature; B shows the same words
      and "Waiting for A to confirm". No group key or content has been sent yet (criterion 2).
   4. Pair on A: A writes the group genesis if needed, `AddMember{B, admin}` with wraps for every
      group epoch, then sends sealed `{group room id}` and closes the slot (`used`). Cancel sends
      `PairClose{cancelled}`. B connects to the group room with `Hello`; the relay admits it as a
      member (ADR 0031 decision 5). B's private keys never leave B (criterion 3).
   Check words: 4 words of 11 bits from the 2,048-word BIP-39 English list (Proposal; the
   implementer confirms its licence, else the EFF large list), derived from `words` over the
   transcript including both identities: up to 44 bits against a swapped key; an attacker who
   already guessed the code can grind its own identity to match words, so the words are a second
   check, not a bound on their own. A guesser, the relay operator included, has 5 tries per code
   at 2^25 each (5/2^25), and a failed try shows as a wrong code. Pairing state machines: module
   `pairing` in `curvyo-sync-core`.
5. **Admission by reconciliation.** One pure function `reconcile` in module `devices` of
   `curvyo-sync-core` takes the replayed group keyring, the set of workspace ids switched on, each
   such workspace's replayed keyring and the own identity, and returns the keyring entries this
   device should write. Each device runs it on start, after every group or workspace keyring change
   and after a switch. In each switched-on workspace where this device is admin:
   - a group member that is not a workspace member gets `AddMember{its identity, role: the lower
     of this device's role there and its group role, wraps: every epoch this device holds}` (full
     history, ADR 0010 §9; with DEV-ROLE A every group device is admin, so it is this device's
     role there). **Re-adding is limited** (amended 2026-11-04, author rule amended 2026-11-05):
     a group member that was removed from the workspace before is added again only when every
     removal of it since its last add was written by an identity that ever was a member of the
     group (group history, not the current members): a switch-off, its own leave, or a device
     the maker has removed from the group since, such as a stolen one that kicked the maker's
     other devices. A removal by an identity that never was in the group, an admin of another
     maker in a shared workspace, stands: reconcile leaves that device out and reports it in the
     plan (`withheld`), and `admit_device` refuses it (`may_readmit`, keyring causal order, the
     same on every device). ADR 0010 §8 itself allows re-adding; this client rule keeps a maker
     from undoing another maker's removal;
   - a workspace member that was a group member and is no longer one (removed or left) gets
     `RemoveMember`, then `NewEpoch` and a workspace snapshot as ADR 0031 decision 6a;
   - a current epoch wrapped to a non-member (a member left by itself) gets `NewEpoch` (criterion
     27, "Rotation pending" until written).
   In each workspace the devices list marks off, other group members get `RemoveMember` and one
   `NewEpoch` (decision 6).
   Where this device is not admin it writes nothing and reports the workspace (criteria 17, 26).
   Two devices reconciling at once write duplicate adds or concurrent rotations; replay merges
   them by ADR 0010 §5 and `RewrapEpoch` (exhaustive graph tests with three devices required).
   Other makers in a shared workspace see one `AddMember` by the maker's device and its sealed
   presence label (criterion 16); they are not in the group room.
6. **Switching a workspace on and off.** On: if not shared, Share as ADR 0037 decision 9; then
   `reconcile` adds every group member; only after that `workspaces.<id> = {name, on: true}` is
   written to the devices list, so a device that reads it is already a member (criterion 11).
   A device that sees `on` for a workspace it does not have creates
   `<synced_folder>/<name>` (" 2" on a clash, criterion 10), joins the room as an existing member
   and reads the snapshot; one it already holds by id is offered "Connect this folder" (criterion
   14). Off: `on: false` written, then `reconcile` removes the other group devices from that workspace
   (decision 5); they keep local files (criterion 13). A
   workspace absent from the list or off is never sent anywhere (criterion 12).
7. **Connections.** While the app runs, it keeps one connection to the group room and one per
   switched-on workspace, opened or not; each is driven as the open workspace in ADR 0037
   decision 8 (main session opens and seals, worker writes files). Nothing syncs while the app is
   closed (Proposal).
8. **Remove, lost device, leave.** Remove on a row (also for a device no longer at hand): in the
   group room `RemoveMember` and `NewEpoch`, so the removed device cannot read later group
   records; then `reconcile` in every workspace, with a count (criterion 23). Any other admin
   device that comes online completes what is left. "Leave my devices" on B: B writes its own
   `RemoveMember` in the group room and each synced workspace, forgets those epoch keys and
   `devices/`, keeps files (ADR 0010 §8); the next admin device rotates (decision 5). A removed
   device learns it from its group keyring or `Refused{removed}` and keeps its files as local
   (ADR 0037 decision 9).
   Consequence of DEV-ROLE A stated here: a stolen device that stays in the group is an admin and
   can remove the maker's other devices first. The defence is removing it promptly; ADR 0010 §5
   keeps at least one admin when two devices remove each other concurrently.
   **The frozen group (Frage 146 GROUP-DEADLOCK, default A, amended 2026-11-04).** When the only
   admins of a room remove each other at the same time (two devices, or a thief that races its
   own removal), ADR 0010 §5 drops both removals, the room's admin set counts as restored, and
   every later removal of either is dropped as well while they are the only admins. With
   DEV-ROLE A every device is admin, so after a two-device mutual removal **the stolen device can
   no longer be removed**: it keeps the group's and the workspaces' current keys and every
   later key. Pairing a third device does not undo it: the moment another admin exists both old
   removals take effect at once, so the thief **and the maker's own device** leave the group and
   the third device holds it alone. Both stay members of every workspace they were in, with its
   keys, and no device left in the group is a member there, so no group device can remove the
   thief from those workspaces any more.
   `reconcile` keeps adding new group devices and renewing keys in such a room, and plans no
   removal there (it could never settle). The only way out is to dissolve the group and set it
   up again: every device leaves, the maker stops syncing the workspaces on the device that
   keeps them, pairs again and switches them on (a new group, new workspace rooms). Debt
   "Mutual removal freezes a device group" tracks a keyring rule that would settle this; the
   keyring rule is unchanged here (default A).
   **What only the relay stops** (crypto review 6). Keyring validity (ADR 0010) checks who wrote
   an entry, not when: a stolen device can write entries whose parents lie before its removal
   (an `AddMember` of a puppet identity) or race a mutual removal as above; only the relay's
   refusal of a removed member's connection keeps such entries out. The My devices list
   therefore shows every participant of the group keyring, named or not ("A device"), so a
   puppet is visible and can be removed.
9. **Browser custody** (option A; a backup on top per Frage 145). An IndexedDB database
   `curvyo-keys` holds one non-extractable AES-GCM-256 `CryptoKey` and records
   `{format_version: 1, name, iv, ciphertext}` for the identity entropy, the group keys and each
   room's keys. Encodings: the identity entropy as its 64 bytes; a room's keys (the group's
   included) as the UTF-8 JSON `[[epoch, key hex], ...]` the desktop host command answers with,
   not the desktop's binary room-keys encoding (amended 2026-11-04; golden
   `frontend/tests/fixtures/key_record_v1.json`). The frontend decrypts them and hands them to
   the wasm session as the desktop host command does; signing stays in wasm. A record that is
   there but does not open (newer `format_version`, damaged, another wrap key) is a refusal
   ("the device keys are not available"), never taken for absent: the identity is only made
   when no record exists, written with IndexedDB `add` and read back, so of two tabs on first
   use the first one's identity wins and a real identity is never overwritten. **What custody
   A protects** (crypto review 9): script cannot export the key bytes, and a copied record
   alone does not open. It does not protect against script running in the page (an XSS or an
   extension can ask the browser to decrypt, as the page does), nor against theft of the
   browser profile, which holds the non-extractable key next to the records. At pairing the browser
   calls `navigator.storage.persist()` and shows the answer. The browser's group state and its
   workspace copies live in browser storage; that store is not decided here (ADR 0037 gives the
   browser no workspace) and needs its own spec. A browser joins a group but cannot start one
   (Proposal), so a group always began on a desktop that holds plaintext files (ADR 0010 §13). It
   needs `wss://` (ADR 0031 decision 11), so it cannot pair with the plain `ws://` LAN server.
   The browser pairs in slice 1 (BROWSER-SLICE A, customer 2026-11-04), so the browser
   workspace store and a `wss://` server behind TLS are part of slice 1. Without a passkey backup
   (decision 13) a browser that loses its data is a lost device and is paired again.
10. **What the relay sees and can link** (criterion 30). Pairing: a slot number, two connections,
    their timing, two group elements, sizes of sealed messages; not the secret part of the code,
    labels, words or identities. Group room: its keyring, i.e. every device's public keys in one
    room: the relay knows exactly which identities belong to one maker, more than the co-occurrence
    criterion 30 names, and from timing that a new member joins rooms right after pairing. Room ids
    of workspaces, names, labels and the on/off list are sealed. Passkey backups (decision 13): an
    opaque id, the write key, the sealed record's size and its store, read and delete times and
    IPs; not the credential id, the passkey, the identity, the group room or the device name.
    Everything else as ADR 0037 decision 11. **The links, spelled out** (amended 2026-11-04):
    - the group keyring is every identity of one maker; every workspace room's keyring lists the
      same identities, so the relay knows how many workspaces a maker syncs, their record sizes
      and times, and which other makers share a workspace with those devices;
    - switching a workspace off is visible as `RemoveMember` plus `NewEpoch` in that room;
      other makers in a shared workspace see each of the maker's devices added and removed;
    - pairing links the two connections' IP addresses by timing, and the new identity's first
      `Hello` in the group room right after it;
    - a backup's id, write key, size, store and read times and IPs, and a restore followed by
      the restored identity's `Hello` links the backup id to that identity.
    The Share tab says "The server can tell which devices belong together, not their names." at
    pairing and at the first switch-on; the backup confirm says the server sees when the backup
    is stored or read.
11. **Formats** (golden files each): relay protocol 3 frames (pairing and backup); pairing messages
    v1 (bound by `"curvyo-pair-v1"`); devices list `format_version` 1 (a snapshot or update that
    writes a newer one is refused and merges nothing; the device keeps its list, says so once
    and uploads no group snapshot in that session); `group.json` v1; browser key
    records v1; backup record v1 and the derived keys (`curvyo-crypto-core`); the relay's backup
    file v1 (`backups/<id hex>`: version, write key, record).
    No keyring payload, envelope, `share.json` or `.curvyo` `format_version` change.
12. **Crates and dependencies.** No new crate, no new crate in `Cargo.lock`; `subtle` (BSD-3-Clause,
    already in the tree through `curve25519-dalek`) becomes a direct dependency of `crypto-core`
    for the constant-time tag comparison. `crypto-core` + `pake`, `pair_code`, `pair_channel`;
    `sync-core` + pairing frames, `pairing`, `devices`, `relay::PairSlots`, `server_address`
    (moved from `ui-core::share_text`); `sync-server` + the
    pairing route; `document-core` + `devices_list`; `storage-io` + `devices`; `editor-wasm` and
    `curvyo-app` + glue; `ui-core` + texts. The word list is data embedded in `sync-core`. Browser
    custody uses WebCrypto and IndexedDB, no npm package; the passkey backup adds `crypto-core::
    backup`, `sync-core::backup` and `relay::Backups`, the server's `backups/`, and WebAuthn glue
    in the page, no WebAuthn library on either side.
13. **Passkey backup of a browser device** (Frage 145 A, customer 2026-11-04). On a paired browser
    "Back up with passkey" creates a discoverable passkey (user verification required) with the
    WebAuthn PRF extension, evaluated on the fixed input SHA-256(`"curvyo-backup-v1"`). The 32-byte
    output never leaves the tab; HKDF-SHA-256 (salt `"curvyo-backup-v1"`, infos `wrap`, `id`,
    `write`) gives the record's XChaCha20-Poly1305 key, a 16-byte record id and the entropy of an
    Ed25519 write key. **What is backed up:** the identity entropy and the group pointer (server,
    group room, device name), sealed as backup record v1 (version, 24-byte nonce, ciphertext; the
    version and the id bound as associated data). Epoch keys are not in it: a member gets every
    epoch wrapped to its identity back from the keyring on connecting (ADR 0010 §6), and the
    group's local files are rebuilt by the joiner's path of pairing, so the record is written
    once and never goes stale. **Relay:** a backup connection's first frame is `BackupGet{id}`
    (answered `BackupRecord{record}` or `Refused{unknown}`) or `BackupBegin{id}` (answered with a
    `Challenge{nonce}`), then `BackupPut{write key, signature, record}` or
    `BackupDelete{signature}`, answered `BackupDone`; one request per connection. The first put
    pins the write key; a later put or delete must be signed by it over
    `"curvyo-backup-write-v1"` ‖ op ‖ id ‖ nonce ‖ SHA-512(record) (never the relay's signing
    context), so a captured write cannot be replayed and nobody else can overwrite or delete a
    record; `Refused{forbidden}` otherwise. Records over 4 KiB (Proposal) are `too-large`. The
    frames are part of relay protocol 3, which was not released before them, so the protocol
    number is unchanged. **Restore:** on a browser without a group, "Restore with passkey" asks
    any passkey of the site, reads the record from the server address typed in the Share tab,
    saves the identity in the key store, and joins the group as a freshly paired device does:
    the same device (same fingerprint), no pairing, no other device online needed. **Remove
    backup** asks the same passkey again and deletes the record. **Without PRF** (the browser,
    the provider or the authenticator lacks it) the browser says so, keeps custody A, and asks
    the provider to forget the useless passkey where the browser offers that (also when a passkey
    that reported PRF gives no output on use). A passkey belongs to the web app's origin, not to
    the relay: each self-hosted web origin has its own passkeys. Accounts can still follow (the
    hybrid below) without changing the record. Amended 2026-11-04 (crypto review):
    - **A stolen passkey is this device.** Whoever can use the passkey (a synced passkey
      provider's account included) restores this identity: a full admin of the group and of every
      synced workspace. The only revocation is removing this device from the group on another
      device, which makes the backup useless. The backup confirm says so in one sentence.
    - **Restore checks first:** a browser that is in a group refuses a restore before anything
      is written ("This device is paired already. Nothing changed."), so a restore never
      overwrites a live identity.
    - **One passkey per device:** Back up again asks the passkey the mark names (same PRF, same
      record id, the record is replaced) instead of making a second one. Leave my devices deletes
      the record (the passkey is asked once more) and forgets the mark either way; a removed
      device forgets its mark (the record then holds an identity the group refuses).
    - **No cap on backups:** anyone who reaches the relay can store backups under new ids, 4 KiB
      each; debt "Relay quotas" covers it with the room quotas.

## Consequences

- No account, no new keyring payload, no relay authority: own-device sync is shared workspaces
  plus one more room per maker, admitted and removed by the same entries 0010 already validates.
- A device that was off joins every workspace switched on meanwhile without anyone online, since
  its public keys are known from pairing. Pairing itself needs both devices online (debt stays).
- `reconcile` is the new security-relevant code next to 0010 §5: a bug re-admits a removed
  device. Mitigation: pure function, exhaustive tests on three-device graphs with another
  maker's device in the workspace, invariants checked in restored rooms too.
- A mutual removal of the only admins freezes a room (decision 8); the group is then dissolved
  and set up again (Frage 146 default A, debt entry).
- CPace is hand-written on audited primitives; it must pass the draft's ristretto255 vectors, and
  the check words stay as a second, independent defence. Needs the focused crypto review ADR 0008
  asks for.
- The relay learns the device grouping exactly, stated in the confirm and the privacy statement.
- A running app holds one connection per synced workspace; fine for tens of workspaces.
- Browser: a browser with a passkey backup survives cleared data and Safari's 7-day rule as the
  same device; without PRF it stays a lost device after clearing. The browser workspace store and
  `wss://` are part of slice 1 (BROWSER-SLICE A). The relay now keeps one record per backed-up
  browser under an opaque id: anyone who reaches it can store small records (quota debt).

## Browser key backup: passkey backup vs accounts (answered A, decision 13)

The customer leans to Frage 142 B or real accounts. Fixed points: the relay is self-hosted (VM or
k8s) and sees only ciphertext (R-COLLAB-006); ADR 0007 keeps accounts and payment out; ADR 0004
puts no user records on the server. A backup only matters for a browser that loses its data or a
maker who uses the browser alone.

**B. Passkey-wrapped backup ("anonymous account").** At pairing the browser creates a
discoverable passkey with the WebAuthn PRF extension; the 32-byte PRF output derives (HKDF) a
wrap key, a record id and an Ed25519 write key. The browser seals its key records (decision 9)
and stores `{id, write public key, blob}` on the relay; writes and deletes are signed with the
write key, reads need the id. After clearing, "Restore with passkey" picks the passkey, rederives
all three, fetches and unwraps: same device, no re-pairing. The relay learns an opaque id, blob
size, store/fetch times and IPs; not the credential id, no name, no email, no WebAuthn data, so
it needs no WebAuthn library. PRF support 2026: Chrome/Edge with Google Password Manager,
Windows Hello (Windows 11) and FIDO2 keys (`hmac-secret`); Safari 18+ with iCloud Keychain;
Firefox 135+ desktop; third-party managers vary. Checked at creation; without PRF the browser
stays custody A and says so. The passkey is bound to the web app's domain, not the relay's, so
every self-hosted web origin has its own passkeys. Effort: a relay route with a small store,
WebAuthn glue, backup record v1 with golden files; about a fifth of slice 1.

**C2. Real accounts.** Sign-in by passkey and/or email+password, server-side user records,
sessions, email delivery (SMTP in the k8s instance, DKIM/SPF), password reset, account deletion,
an admin view for the instance. E2E means a password reset restores the login, never the keys:
recovery still needs B-style wrapping (or a password-derived wrap via OPAQUE/Argon2, or a
recovery code). Email makes the self-hoster a GDPR controller (privacy statement, deletion,
export, breach duty). Gains beyond B: finding a maker by email for sharing, billing and an asset
provider later (ADR 0007 questions). Effort: an own ADR revising 0007, a server database and
several slices; it would not fit slice 1.

**Hybrid.** Build B now; a later accounts ADR adds a login record that points to the same backup
id (proved by the write key) and more passkeys as further wraps. Device keys and the backup
format stay unchanged, so accounts can come without re-pairing or re-encryption.

**Recommendation: hybrid, B in slice 1.** Slice 1 then contains desktop pairing and the device
group, the browser paired as a device with custody A, the browser workspace store, `wss://`,
and the passkey backup with restore. A browser still joins a group and does not start one.

## Questions for the customer

Frage 142 (BROWSER-KEYS): replaced by Frage 145 (customer leans B or accounts, 2026-11-04).

Frage 143 (BROWSER-SLICE): answered A (customer, 2026-11-04): the browser pairs in slice 1.

Frage 146 (GROUP-DEADLOCK): asked 2026-11-04, unanswered, default A built (decision 8): the
ADR states that a group frozen by a mutual removal must be dissolved and set up again, a debt
entry tracks it, the keyring rule of ADR 0010 §5 is unchanged.

Frage 145 (BROWSER-ACCOUNT): answered A (customer, 2026-11-04), built as decision 13. The
question as asked: How does a browser get its keys back after its data was cleared?
- A: Passkey backup in slice 1: the keys, locked by the passkey, lie on your sync server under an
  anonymous number; no email, no password, the server sees nothing readable. Real accounts can
  follow later as their own decision, without re-pairing.
- B: Real accounts now (passkey or email+password, email delivery, password reset, GDPR duties
  on your server). Recovery of keys still needs the passkey lock of A. Own ADR, several slices;
  slice 1 ships without browser recovery.
- C: No backup in slice 1; a cleared browser is paired again from the desktop. Decide later.
- Recommendation: A. It gives recovery and browser-alone use now and keeps the way to accounts open.
- Default if unanswered: C (adds nothing on the server that would be hard to take back).
