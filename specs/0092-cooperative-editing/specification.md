# Cooperative editing: several people in one document at the same time

Status: Done (PR #253, PR (this one)): slice 1; single-document sharing removed and criteria 1 to 5, 33, 40 to 45 superseded by `0105`; criteria 6 to 31 and 35 to 39 hold through a shared workspace
Priority: Must
Origin: Customer (request of 2026-11-02, translated): "Start with cooperative work at the same time on several clients: spec, plan and ADR if needed." The requirement itself is older and already accepted: R-COLLAB-001 to -013 and ADR 0004 (2026-10-02): "not just sync, cooperative work from the start, similar to the Zed editor, but for vectors. Two people on two instances must be able to work in the same file at the same time."

Slices 3 and 4 and everything marked Proposal are the product owner's proposals until the customer accepts them. Slice 1 is the customer's request, cut thin. Customer decisions of 2026-11-03 are recorded below under "Customer decisions (2026-11-03)" and are applied in the criteria.

## User value

As a maker I want to open a document together with another maker on another computer and both of us draw in it at the same time, seeing each other's changes and pointers live, so that a joint job (one draws the artwork, the other prepares the cut or the stitching) needs no file ping-pong, no "who has the latest version" and no merge by hand. When one of us loses the network, we keep working and the work merges when the connection comes back.

What other tools do:

- **Figma**: the reference for live vector co-editing. Shared file, coloured cursors with names, followed viewports, comments. Server-side, unencrypted, account required, no offline merge to speak of. Undo is per person. We follow the per-person undo and the presence; we differ by encrypting end to end, needing no account and working offline.
- **Zed**: the customer's named model. Peers hold replicas, edits are local and immediate, the server relays. Same shape as ADR 0004 option A.
- **Excalidraw**: live rooms by link, the key in the URL fragment so the server cannot read the scene, no accounts. The closest to our invitation and encryption idea, but a whiteboard: no per-participant removal, no offline merge into a saved file.
- **Inkscape**: none. Files only; "collaboration" means a shared folder and a lost-update risk. Its SVG remains our exchange format (R-SYS-006).
- **Affinity**: none. Single-user documents; cloud storage of files, no simultaneous editing.
- **LightBurn, Ink/Stitch, Fusion/Easel**: no simultaneous editing of a job file either. Easel has cloud projects, one editor at a time.

What we do better: a vector editor for makers where two people can edit the same file offline or online, the relay can read nothing (ADR 0008), the people in a document are known and removable (ADR 0010), and the self-hosted server is the same server as the hosted one (ADR 0004 §13).

## Words

- **Shared document**: a document that has a keyring (ADR 0010) and a room on a sync server. One file on each participant's disk plus the replicated state.
- **Participant**: one device with its identity keys (ADR 0010 §1), shown under a label the maker chooses. A device, not a person.
- **Admin / member**: the two roles of ADR 0010 §12. The maker who shares a document is its first admin.
- **Room**: the place on the server where the sealed updates of one shared document are relayed and its newest sealed snapshot is kept (ADR 0004 §5).
- **Presence**: pointer position, selection, active tool and the live in-flight geometry of a drag of the other participants (ADR 0009 §2). Never document content, never saved, never undoable.
- **Sync server**: the open-source blind relay (`curvyo-sync-server`, ADR 0004 §5). It stores and forwards ciphertext.
- **Invite link**: a one-time, expiring link that admits one participant (ADR 0010 §7).
- **Version**: an immutable state of the document (ADR 0009 §4); exports and jobs are made from a version, never from the sliding live state.

## Slicing

| Slice | What the customer can try | Priority |
|---|---|---|
| 1 | Two instances, one self-hosted server (a local VM for testing, deployable later to Kubernetes without code change): share, join by invite link, draw together, see presence, work offline and merge, local undo, remove a participant with key rotation. Everything sealed. | Must |
| 2 | Rest of the membership lifecycle: second admin, leave, re-add, verified-fingerprint flow, concurrent removal. | Must (R-COLLAB-009 to -012) |
| 3 | Cloud sync of projects across the maker's own computers on the same server (R-SYS-004, R-COLLAB-003). Own spec when slice 1 is accepted. | Must |
| 4 | History view and "revert this change" (ADR 0009 §1 option C), compaction of the log, folder sync, forward secrecy. Own specs. | Should |

This spec's acceptance criteria cover slice 1 in full (including removal with key rotation, customer decision COOP-SLICE B) and slice 2 in outline (marked "Slice 2"). Slices 3 and 4 are named only so that slice 1 does not rule them out.

## Acceptance criteria

All criteria assume two instances of the desktop app, A and B, each with its own data directory and credential store (so they behave as two different devices on one machine as well as on one LAN), and one `curvyo-sync-server` started from the repository with one command (in the test setup in a local VM) and a server address setting in the app (ADR 0004 §13). "Edit" means one history step as today.

### Sharing and joining (slice 1)

1. Given a saved document open in A and a reachable server URL in A's settings, when the maker chooses Share in the Share tab of the Properties panel, then A (a) writes a genesis keyring entry with A as the only admin (ADR 0010 §3), (b) uploads the sealed state, (c) shows the participant list with one entry "this device" with a fingerprint, and (d) marks the document as shared in the title bar. A document without a reachable server stays fully editable and shows "Not connected" instead; Share is then unavailable with the reason named.
2. Given a shared document, when A chooses Invite, then A shows an invite link with a validity of 24 hours (Proposal, adjustable in the dialog) and one sentence stating what the link grants ("Anyone with this link can edit this document once, until it expires"). The link holds only a one-time secret, carried only in the part after `#`, so the server never receives it (ADR 0008 option A, ADR 0010 §7); it does not contain the document key. No raw key material is shown or typed. The room id in the link identifies one room; nothing in the link or the server assumes one document per server or per user.
3. Given the invite link, when B opens it (pasted into B's "Open shared document" entry, or opened as a `curvyo://` link where the platform allows, Proposal), then B asks for the one setting "Name shown to others" if B has none, and shows "Waiting for a member to let you in" until an online member (here A) admits B automatically, with no confirmation click by A (customer decision COOP-JOIN A). If no member is online, B keeps waiting and joins as soon as one comes online, as long as the invite has not expired by the admitting member's clock (ADR 0031 decision 8); past expiry B is told the link expired and to ask for a new one (Proposal, 2026-11-03); B never sees document content before admission. After admission B downloads the document and shows it within 5 s for a document of 1,000 objects on a LAN server, with A online at the time of opening. The joined document opens as an unsaved "Untitled" document (Proposal). The participant list in both A and B now has two entries with their names, colours and fingerprints.
4. Given a link already used once, or one past its expiry, when a third instance C opens it, then C is refused with a message that names the reason (used, expired) and sees no document content. The server's stored data for the room gained no readable content from the attempt.
5. Given B has joined and the joined document is still unsaved "Untitled", when B closes it, then B is asked for a save location (Proposal) and can choose to save or discard. Given B saved it, when B closes and reopens the document from its disk, then it opens as a shared document and reconnects to the room without a new invite.

### Live editing (slice 1)

6. Given A and B connected to one room, when A draws a rectangle, then it appears in B within 300 ms at the 95th percentile over 100 consecutive edits on a LAN server (measured from the commit in A to the rendered frame in B), and the same in the other direction.
7. Given A and B connected, when A drags an object, then B sees the object move live during the drag (presence geometry, ADR 0009 §2), and the document in B gains exactly one history entry for that drag, after A releases the mouse. No operation per input event is created.
8. Given A and B edit different objects, or different anchors of one path, or different fields of one object (fill in A, stroke width in B) at the same time, then after both commits both changes are present in both instances (ADR 0009 §3).
9. Given A and B set the same scalar field of the same object to different values at the same time, then both instances converge to the same one of the two values within 2 s, and no error is shown. (Documented behaviour, ADR 0009 §3: one value wins.)
10. Given A and B type into the same text object at different positions at the same time, then both insertions are present and both instances show the same string (ADR 0009 §3).
11. Given a path that A deletes while B edits one of its anchors, then both instances converge to the same state within 2 s, with no crash and no half-deleted object.
12. Given A moves an object into a group while B moves the same object into another group at the same time, then after convergence the object exists exactly once, in the same group, in both instances (movable tree, ADR 0004 option A).
13. Given a document of 1,000 objects, when A makes a single edit, then the bytes sent to the server are at most 10 kB (Proposal: updates are deltas, not snapshots) and A's frame time stays within the figures of the performance criteria of the editor (no frame over 16 ms of thread CPU caused by sync; measured with the thread-CPU method of CLAUDE.md §5).

### Presence (slice 1)

14. Given A and B connected, then each sees the other's pointer as a coloured pointer with the participant's label, and the other's current selection outlined in that participant's colour, updated within 150 ms at the 95th percentile on a LAN. Pointer and selection of a participant who disconnects disappear within 5 s.
15. Given B has selected or is dragging an object, when A hovers or presses on that object, then A sees a marker on it that names B ("B is editing this") before A changes it. Nothing prevents A from editing it (no locks, ADR 0004 option C rejected). This is the mitigation required by `docs/technical-debt.md` ("A contested scalar loses one side's value").
16. Given presence data, then it is sealed like edits (ADR 0009 consequences), is not written into the saved file, is not in any history step and is not undoable. Switching the active tool in B is visible to A only as a small tool icon at B's pointer (Proposal); no toolbar state is mirrored.

### Offline and reconnecting (slice 1)

17. Given A and B connected, when B's network connection is cut, then B shows "Offline, changes are kept" within 5 s, can keep editing, saving and exporting with full single-user function (R-COLLAB-005), and A sees B's presence disappear.
18. Given B made 50 edits offline and A made 50 edits to other objects meanwhile, when B's connection returns, then within 5 s both instances hold all 100 edits, and an SVG export of the same version from A and from B is byte-identical (ADR 0009 §4).
19. Given B closes the app while offline, edits are saved to B's file; when B reopens it with the network back, then the edits merge as in criterion 18 without any user action.
20. Given the server is restarted while A and B are connected, then both reconnect without user action within 30 s, lose no edit, and the server's stored data is unchanged apart from new sealed entries.

### Undo (slice 1)

21. Given A and B have both made edits, when A presses Ctrl+Z, then A's last own edit is reverted and B's edits are untouched; B's undo stack is unchanged by A's edit and by A's undo (ADR 0009 §1).
22. Given A's last own edit created an object that B has since deleted, when A presses Ctrl+Z, then that entry is skipped with a message of at most 70 characters, and the object is not resurrected.
23. Given nobody else is connected, then undo and redo behave exactly as in a document that is not shared.

### Encryption and what the server sees (slice 1)

24. Given a shared document containing a text object with the marker string `ZQX-MARKER-7731` and a stroke colour `#13a7c9`, when the server's entire stored data for the room (database files, logs, request bodies captured by a test proxy) is searched, then neither the marker, the colour, nor any SVG or JSON fragment of the document is found. (ADR 0008 §1.)
25. Given a test sync server whose stored updates are altered by one byte, or moved into another room or another epoch, when A and B receive them, then they are refused and not merged, and the user sees "A received update was rejected" once, not a crash (ADR 0008 §3).
26. Given the server is stopped for good, then A and B still hold the complete document in their own `.curvyo` files, openable and editable (R-SYS-002, ADR 0004 §1, ADR 0008 §7).
27. Given a fresh device that was never invited, when it connects to the room with only the room id, then it receives only ciphertext it cannot open, and the participant list of A and B does not change.
28. Given any code path, then there is no setting, flag or environment variable that disables sealing (ADR 0008 §1).

### Output stays reproducible (slice 1)

29. Given A is generating a job or exporting while B keeps editing, then the result is made from the version taken when A started and does not contain B's later edits; A's export shows the version it was made from (ADR 0009 §4).
30. Given A's document is shared, then a job, export or preview in A never changes because a remote edit arrived in the middle of it.

### Page resize and fit across peers (slice 1; customer decision COOP-RESIZE B)

31. Given A and B connected (or both offline and later merged as in criterion 18) on a page holding objects, when A and B resize the same page at the same time to different sizes, then within 2 s both instances show the same page frame (x, y, width, height), equal to one of the two whole frames and never a mix of A's width with B's height or origin; no resize is refused and no error is shown. Each resize keeps the frame's centre. No object changes its stored position through a resize, and the position of every object read relative to the frame origin (rulers, position fields, SVG export) is the same in A and in B and shows what a single-user resize shows today. Fit (page to content bounds) behaves the same: it writes the content bounds as the whole frame and moves no object. When A and B resize different pages at the same time, both resizes are present in both instances. Page frame as one register: ADR 0031 decision 9a. Resolves the entry "Resize and fit merge per field across peers" in `docs/technical-debt.md`, except the case named in Out of scope.

### Removing a participant (slice 1; customer decision COOP-SLICE B)

32. Given A (admin) and B (member) in a shared document, when A removes B in the participant list, then the removal dialog says in one sentence that removal does not reach back, and A's device writes, in this order, a `RemoveMember` entry for B, a `NewEpoch` entry (previous highest epoch + 1) with a fresh key wrapped to every remaining participant and not to B, and uploads a fresh signed snapshot sealed under the new epoch (ADR 0031 decision 6a). Afterwards the participant list in A shows the new key epoch number and no entry for B. A member who is not an admin sees no Remove action.
33. Given B was removed, then (a) the server closes B's open connections with the reason "removed" and refuses B's later connection attempts; (b) every envelope written after the removal carries the new epoch's `key_id` in its cleartext header, and B, holding only older keys, cannot open any of them (test: B's replica receives no record sealed under the new epoch); (c) a push from B is not merged by A, and A shows no error. B shows "You were removed from this document. Your copy stays on this computer." and keeps the last state B had synced as a normal editable local file, no longer syncing (R-COLLAB-010).
34. Given a removal while B is offline, when B reconnects, then B learns of the removal from the keyring, is disconnected, cannot read the new epoch, and edits B made offline are not merged into the shared document. Given C, a member who was offline during the rotation, when C reconnects, then C's edits made before it saw the rotation are sealed under the new epoch at send time and arrive at A. A third participant C, still a member, who was online continues editing with A across the rotation without user action and loses no edit. Records sealed under the older epoch stay readable to members who held that key.

### Hosting and server operation (slice 1; customer decision COOP-HOST)

35. Given the repository, then `curvyo-sync-server` is one binary configured only by flags or environment variables, the flag winning: `--listen` / `CURVYO_SYNC_LISTEN` (default `0.0.0.0:8787`) and `--data` / `CURVYO_SYNC_DATA` (default `/data`). It needs no config file and no other service (ADR 0031 decision 11). Test: a flag overrides the environment variable.
36. Given the repository, then a Dockerfile in the server crate directory builds a static musl binary into a distroless nonroot image (`gcr.io/distroless/static-debian12:nonroot`), with `/data` as the volume and port 8787; a container started from it with one volume mounted on `/data` serves the room of criterion 1; no code change is needed to run it in Kubernetes. The server logs to stdout and writes no IP addresses or request bodies to its logs. The image is built by hand for the demo; a CI image build is not part of slice 1.
37. Given a running server, then `GET /healthz` answers 200 once its startup scan is done and the data directory is writable, and 503 otherwise, and reveals no room data (usable as a Kubernetes probe). Given the server receives SIGTERM or SIGINT, then it stops accepting, closes every socket with WebSocket close code 1012, and exits within 10 s of the signal; clients reconnect as in criterion 20.
38. Given a server, then all its stored data (rooms, sealed entries, invitation state) is in the one data directory; when the server (or container) is stopped and started again on the same directory, criterion 20 holds. One server process owns the directory (one replica only; recorded as debt).
39. Given the app's settings, then there is one server address setting; changing it to the address of the VM server or of a later Kubernetes instance needs no other change. Test setup: the server runs in a local VM and both desktop app instances connect to it from outside the VM over plain `ws://` on the LAN (no TLS, no self-signed certificate in the demo). The browser build joins only once the server sits behind a TLS-terminating ingress offering `wss://` (later Kubernetes step, not slice 1).
40. Given a room, then room ids and invites identify one room each and are not derived from a single document per user or server: a server can hold several rooms of one maker, verified by a test with two shared documents of A on one server that stay separate (own invites, own participant lists, no cross-talk). The app in slice 1 shares one document per room (see Out of scope).

### Membership lifecycle (slice 2, outline)

41. Slice 2: Given a document with two admins, then either can invite, and the document's participant list shows each participant's role; the first sharing dialog nudges toward a second admin (ADR 0010 §13, R-COLLAB-012).
42. Slice 2: Given B leaves, then B stops syncing, the remaining admins are told a key rotation is pending, and B's wording never claims B lost access to what B can still read (ADR 0010 §8).
43. Slice 2: Given an admin re-adds a removed participant, then the admin chooses "full history" or "from now on" (ADR 0010 §9), and the returning participant sees exactly that history.
44. Slice 2: Given the verified flow, then the invitee shows a fingerprint, the admin confirms it by comparing it, and the participant is added without an invite link (ADR 0010 §7).
45. Slice 2: Given two admins remove each other at the same time, then one admin set survives and the UI reports it (ADR 0010 §5). Fork rules are covered by exhaustive unit tests in the crypto crate.

## Out of scope

- Accounts, email, passwords, payment, server-side recovery (ADR 0007, ADR 0004 §10).
- Cloud sync of the maker's own project files and libraries across their own computers (slice 3, own spec).
- Folder sync, history view, "revert this change", log compaction (slice 4, own specs).
- Read-only participants and per-object permissions (cannot be enforced, ADR 0010 §12).
- Comments, chat, voice, following another participant's viewport.
- Editing with no connection to the server between two instances directly (peer to peer over LAN without a server).
- Browser and mobile clients; slice 1 is the desktop app (CLAUDE.md platform order). The core and the protocol stay wasm-buildable.
- Real-time sharing of machine resources and leases (ADR 0004 §4, `0064`).
- Library records (materials, machines) as collaborative documents (ADR 0004 §6: one file per record).
- Forward secrecy (named follow-up in ADR 0008).
- Operating and publishing a hosted instance; slice 1 needs only the server runnable by the maker. Hosting, privacy policy and breach process are a separate decision (ADR 0004 consequences).
- Old-file compatibility (customer, 2026-10-30).
- Sharing several documents at once (a folder or project shared as a whole): the customer wants it, but it needs a future workspace feature (own spec). Slice 1: one document is one room. Design note: room ids and invites must not assume a single document per server or per user, so a workspace can later share several documents (criterion 40).
- Operating the Kubernetes instance (manifests, ingress, TLS, backups): the customer deploys it himself later; slice 1 only ensures no code change is needed (criteria 35 to 39).
- Showing a pending-admission prompt to the admitting member, or manual approval of a joiner (decision COOP-JOIN A: admission is automatic).
- Rest of the membership lifecycle (second admin, leave, re-add, verified flow): slice 2.
- A page move concurrent with a resize of the same page: the result keeps one whole frame and the move's object shift, not both intents. Recorded as narrow debt; see `adrs.md` (technical debt, "A page move concurrent with a resize") and ADR 0031 decision 9a.
- TLS in the server binary, room quotas and rate limits, more than one server replica, browser key custody: debts recorded in `adrs.md`.

## UX notes

Decisions already taken (the ux-engineer's notes in `ux-notes.md` build on these):

- Share is its own tab in the Properties panel (not a dialog or menu).
- A joined document opens as an unsaved "Untitled" document and asks for a save location when it is closed (Proposal).
- There is one setting "Name shown to others" (replaces "device label"); it is asked once, on first share or join.

## Architect notes

Decisions already taken and binding for this feature; nothing here is new:

- ADR 0004: CRDT document plus blind relay, Loro, server crate `curvyo-sync-server` (ADR 0004 and 0011 write `vecmanf-*`, since renamed by ADR 0013), no accounts (§10), server open source and self-hostable (§13), locks only for machine resources.
- ADR 0008: everything sealed from the first collaboration story, XChaCha20-Poly1305, snapshots made and signed by clients, key custody in the credential store, no plaintext flag.
- ADR 0009: peer-scoped undo, presence as a separate sealed awareness channel, per-field merge, version-snapshot reads, missing font rule (a substituting peer never writes layout back).
- ADR 0010: keyring log, device identities, two roles, deny wins, admin set never empty, remove = rotate, invite and verified flows.
- ADR 0011: crates `curvyo-crypto-core`, `curvyo-sync-server`, relay socket and credential store in `curvyo-storage-io`. `curvyo-crypto-core` and `curvyo-sync-server` do not exist yet in the workspace; the ADR authorizes them, so no new crate ADR is needed. Today `curvyo-document-core` holds Loro but exposes no export/import of updates, and no relay or keyring exists.
- Open for the architect (ADR needed, not decided by the PO): how local edits are exported as sealed updates and remote ones imported without a Loro type in the public API (ADR 0004 §3); the awareness channel format; the relay wire protocol (ADR 0004 §5 names only axum and WebSocket); how the 1:N reconnect and snapshot upload by a client are triggered (ADR 0008 §6); the invite-link format. `docs/technical-debt.md` entries to resolve here: resize/fit across peers, undo cannot reach a collaborator, contested scalar, missing font.
- Customer-accepted in ADR 0008: per-participant key wrap and rotation are "in the first collaboration story, not after it". With customer decision COOP-SLICE B, slice 1 includes the wrap, the signed keyring, removal and rotation, as ADR 0008 says.
- COOP-RESIZE B is a document-model change: page origin and size become single registers per page (last writer wins). The architect decides the model and the format change (ADR 0031 or its own ADR); objects keep their positions, no refusal path exists.
- COOP-JOIN A: the invite link holds only a one-time secret; admission is done automatically by an online member, so the join protocol needs a pending-join record at the relay (sealed) that a member answers; the architect specifies it.
- COOP-HOST: the server is configured by env/args, one data directory, health endpoint, Dockerfile; no hosted-instance code paths.
- Room ids and invites carry no "one document per server or user" assumption (criterion 40).

## Customer decisions (2026-11-03)

- COOP-SLICE: B. Slice 1 also includes removing a participant with key rotation (criteria 32 to 34). Leave, re-add, second admin, verified flow and concurrent removal stay in slice 2.
- COOP-HOST: first a sync server in a local VM for testing; later the customer deploys an instance to his own Kubernetes cluster. Hence one binary configured by env/args, a Dockerfile, a health endpoint, one data directory (volume), and a server address setting in the app (criteria 35 to 39).
- COOP-SCOPE: the customer wants even several documents shared, but that needs a future workspace feature. Slice 1: one document = one room, all its pages. Room ids and invites must not assume a single document per server or user (criterion 40; Out of scope).
- COOP-RESIZE: B. Everything happens live; page resize is a core feature and must work concurrently. Page origin and size are one frame register per page, last writer wins, no refusal, objects never move (criterion 31, ADR 0031 decision 9a).
- COOP-JOIN: A. The invite link holds only a one-time secret; an online member admits the joiner automatically; the join waits until a member is online (criteria 2 and 3).
- UX: Share is its own Properties tab; a joined document opens as unsaved "Untitled" and asks for a save location on close (Proposal); one setting "Name shown to others" (UX notes above).

## Open questions

Frage 3 (COOP-INVITE): How does a second maker get into a document in slice 1?
- A: Invite link only (one-time, expiring, secret after `#`); the verified fingerprint flow comes in slice 2.
- B: Short invite code to type in (words and digits), no link.
- C: Both link and code.
- Recommendation: A for slice 1; a link is what Figma and Excalidraw users know, and ADR 0010 §7 describes it. A code is a presentation of the same secret and can be added in slice 2 if you want it.
- Default if unanswered: A.

Not asked, because already decided: device keys rather than accounts (ADR 0010, answered 2026-10-02); encryption in the first story (ADR 0008); peer-scoped Ctrl+Z (ADR 0009); self-hostable open-source server (ADR 0004 §13).

## Links

Requirements: R-COLLAB-001 to -013, R-SYS-002, R-SYS-004, R-SYS-007 in `docs/requirements.md`; ADR 0004, 0008, 0009, 0010, 0011 in `docs/adr/`; `docs/technical-debt.md` entries named above.
PR: none yet.
