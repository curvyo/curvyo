# Workspace: several documents together, shared as a whole

Status: Done (PR #264, PR #298): slice 1; take-in, `Moved` and WS-SEPARATE B superseded by `0105`
Priority: Should
Origin: Customer (request of 2026-11-03, translated): a workspace feature, several documents together. Earlier customer words (COOP-SCOPE, 2026-11-03): "even several documents shared, but we do not have several documents yet, that needs a workspace feature." Customer decisions of 2026-11-03 are recorded below and applied. What is marked Proposal is the product owner's until the customer accepts it.

## User value

As a maker with a job made of several drawings (a box: lid, sides, engraving; a sign: letters, frame, backing) I want to open one workspace that holds all of those documents, switch between them, create, rename and move them, and share the whole set with another maker in one step, so that I stop hunting through file dialogs and my partner sees every document, and every new one, without file ping-pong.

## Problem today

- One window holds one document. Open and Save as are file dialogs (File tab, `0051`); no list of "my documents", no recent files (Out of scope in `0051`), no way to see that three files belong together.
- Sharing (`0092` slice 1) is one document per room; invite link version `1.` means one document (ADR 0031 decision 8). Several documents need one invite, one membership and a shared list of documents.
- Cloud sync across the maker's own computers (R-SYS-004, R-COLLAB-003, `0092` slice 3) needs a unit to sync: the workspace.
- The material library (R-MAT-005, `0028`) needs a place that is not one document.

What other tools do:

- **LightBurn**: one project file with layers, no multi-file project; libraries global per machine, copied by hand.
- **Inkscape, Ink/Stitch, Affinity**: single files; a "project" is a folder by convention; Inkscape pages are inside one file (we have that, `0059`).
- **VS Code / Zed**: a workspace is a folder; the sidebar is its tree; files stay ordinary files. Zed shares the whole project with one invite. This is the model we copy and it matches what the customer said of Zed in `0092`.
- **Figma / Easel**: a project is a server-side group of files; needs an account, server reads everything.

What we do better: the workspace is an ordinary folder usable without Curvyo (R-SYS-002); every document stays a normal `.curvyo` file; sharing the whole set is sealed end to end, including file names and the folder tree (ADR 0008), with no account.

## Words

- **Workspace**: a folder the maker opened as a workspace, with a marker directory `.curvyo-workspace/` inside. Everything below it is the workspace.
- **Workspace document**: a `.curvyo` file (ADR 0004 §1) below the workspace folder. Format unchanged.
- **Workspace id**: a random id in `workspace.toml`; the room id of the workspace room.
- **Workspace room**: the one room (ADR 0004 §5) of a shared workspace. It carries the sealed document list (folders, document ids, names, places, deletions) and every document of the workspace as an encrypted stream inside its sealed records (ADR 0037). Merged like any document (ADR 0009). There is no room per document; the server cannot tell how many documents there are.
- **Single-document room**: the room of one document shared on its own, as in `0092` (room id = document id).
- **Shared workspace**: a workspace with a keyring (ADR 0010) and a workspace room. All its documents are shared through it.
- **Take-in**: moving a document that is shared on its own into the shared workspace (criterion 42).
- **Member, admin, participant, invite link, presence**: as in `0092` "Words".

## Slicing

| Slice | What the customer can try | Priority |
|---|---|---|
| 1 | Local workspace in the File tab (open folder, tree, open and switch, create, rename, move, duplicate, delete, recent workspaces) and sharing the whole workspace from the Share tab (invite version `2.`, one workspace room, joining, live document list, removal of a member). Two instances, one server, as in `0092`. | Should |
| 2 | Cloud sync of the workspace across the maker's own computers (R-SYS-004, R-COLLAB-003, `0092` slice 3). Own spec. | Must (R-SYS-004) |
| 3 | Workspace library: material records and shared assets (R-MAT-005); needs `0028`. Own spec. | Should |
| 4 | Several documents open at once, tabs or windows, objects between documents. Own spec. | Could |

Slice 1 is large. Proposed build order inside it, each step demoable (Proposal, the lead may cut differently): (a) local workspace, criteria 1 to 24; (b) share and join, 25 to 34; (c) live document list and conflicts, 35 to 42 and 49; (d) removal and server, 43 to 48.

## Acceptance criteria (slice 1)

Desktop app only (a folder needs a filesystem). The mechanisms are decided in ADR 0037 (see "Decided in ADR 0037"); the criteria state the behaviour. Where the UI is named: the tree, Deleted list and recent workspaces are in the File tab's Workspace section; sharing, invite, participants, Remove and "Join shared workspace..." are in the Share tab (`ux-notes.md`). Sharing criteria assume the setup of `0092` "Acceptance criteria" (two instances A and B, one `curvyo-sync-server`, server address setting).

### Opening and creating a workspace

1. Given no workspace open, when the maker chooses "Open workspace…" in the File tab's Workspace section and picks a folder, then the app (a) creates `.curvyo-workspace/workspace.toml` if absent, holding a random workspace id and a format number, (b) shows the folder's `.curvyo` files, recursively, as a tree in the Workspace section, and (c) changes no existing file. A folder with no documents shows an empty list with a "New document" button.
2. Given a folder that already is a workspace, when it is opened again, the same workspace id is kept. A workspace folder copied to another computer opens there with the same id.
3. Given the maker opens a plain `.curvyo` file with Open (not Open workspace), then it opens exactly as today, with no Workspace section content and no folder created beside it. Every workspace document opens that way too; a document of a shared workspace opened by Open outside its workspace opens as a local file.
4. Given a folder that cannot be written, then Open workspace says so and does not open it as a workspace.

### Document list

5. Given a workspace of 500 documents in nested folders, when it opens, then the tree is shown within 1 s of thread CPU time for the scan (names and sizes only, no document opened), and a folder expands or collapses without a visible stall. The list is virtualised (as `0050`).
6. Given a document in the list, then its row shows the name without extension, a thumbnail read from `thumbnail.png` of the container (placeholder when absent) and the time of its last change; thumbnails load lazily for visible rows only.
7. Given files or folders are added, removed or renamed in the folder by another program while the workspace is open, then the tree shows it within 5 s, and the open document is not affected unless its own file was removed (then the title shows "File missing" and Save as stays possible).
8. Given a file that is not a `.curvyo` container or is damaged, then its row shows "Cannot be read" with the reason; nothing else breaks and no file is changed.

### Open and switch

9. Given a workspace, when the maker clicks a document row (or presses Enter on it), then it opens in the window; with unsaved changes the inline check of `0051` comes first (Save, Discard, Cancel). Exactly one document is open at a time.
10. Given the maker switches from A to B and back, then A shows the same content, page layout, zoom and scroll as when left (view state per document for the session; Proposal), and A's history is the one in A's file.
11. Given a document is open, then the title bar shows "workspace name / path inside the workspace" and the tree highlights its row.
12. Given a document shared on its own (`0092`) lies in a local workspace folder, when it is opened by a click, then it reconnects to its own room as when opened by Open; switching away disconnects it as Close does.

### Create, rename, move, delete (local workspace)

13. Given "New document" (a button in the Workspace section, or on a folder row), when it is named in an inline field, then an empty document with the default page (as New in `0001`) is written as `<name>.curvyo` in that folder (the root when none is selected) and opened. A name that exists gets " 2", " 3" appended; the maker is told the final name.
14. Given a document row, when the maker renames it (F2 or double click on the name) to a name without path separators or characters the OS refuses, then the file is renamed and the row updates. A bad name is refused inline with the reason and the old name stays. Renaming the open document keeps it open with no reload and no loss of unsaved changes.
15. Given a document that is shared (on its own, or through a shared workspace), when it is renamed or moved, then it keeps its identity and the same participants: its document id is stored in the file, never derived from the name or path.
16. Given a document or folder row, when the maker drags it onto another folder row, then the file moves there. A move into itself or onto an existing name is refused with the reason and changes nothing. Moving the open document keeps it open. A move is reversible with an "Undo move" notice for 10 s (Proposal); it is not a history step of any document.
17. Given Delete on a document or folder row, then the item goes to the OS trash after one inline confirmation that names the item and, for a folder, the number of documents in it (customer, WS-DELETE A). Where the OS has no trash, nothing is deleted and the reason is shown. Deleting the open document closes it after the unsaved-changes check. In a shared workspace, Delete is a shared deletion: criterion 40.
18. Given New folder, then an empty folder is created and can be renamed. No operation in criteria 13 to 17 writes anything but the named file or folder.
19. Given Duplicate on a document, then `<name> copy.curvyo` is written beside it with a new document id and no keyring (a copy is not a member of the original's room; same rule as Save as in `0092`). In a shared workspace the copy is a new document of the workspace (criterion 36).

### Recent workspaces and closing

20. Given workspaces opened before, when the maker opens "Open workspace…", then up to 10 recent workspaces are listed by folder name and path, most recent first, each removable. The list is kept in the per-user app data (as `font-library.toml`, `0075`) and never sent anywhere; a missing path is greyed "Not found". Starting the app opens nothing on its own (Proposal).
21. Given a workspace open, when the maker chooses "Close workspace", then the document goes through the unsaved-changes check, the Workspace section returns to its empty state and the app is as after start. Closing a shared workspace disconnects the workspace room.
22. Given the local criteria, then no document content, file name or path leaves the machine and no network is used; only criteria 25 onwards use the server, and only after the maker chose Share (R-SYS-007, R-SYS-002).
23. Given `workspace.toml`, then it holds the workspace id and a format number; a reader that meets a higher format number refuses to open the workspace with the reason, without writing. It holds no per-document state, so any single file stays usable alone. The keyring and the cached document list of a shared workspace live beside it in `.curvyo-workspace/` (ADR 0037 decisions 1 and 4).
24. Given two workspaces opened one after the other, then their ids differ and nothing of the first shows in the second.

### Sharing the whole workspace and joining

25. Given an open workspace, a reachable server and the document list, when the maker chooses Share workspace in the Workspace block of the Share tab, then, after one confirm that says names, folders and files leave the computer encrypted, A (a) writes a genesis keyring entry with A as the only admin (ADR 0010 §3), (b) creates the workspace room, (c) uploads one sealed workspace snapshot holding the document list and every document, (d) shows the participant list with "this device" and a fingerprint, and (e) marks the workspace as shared in the Share tab, the File tab's header line and the title bar. Without a reachable server Share workspace is unavailable with the reason named; the workspace stays fully editable.
26. Given a shared workspace, when A chooses Invite in the Share tab, then A shows an invite link `curvyo://join?server=<url>#2.<base64url(workspace id ‖ invite seed)>` (version `2.`; version `1.` stays one document, ADR 0031 decision 8) valid 24 hours (Proposal, adjustable), with one sentence stating what it grants ("Anyone with this link can edit every document of this workspace once, until it expires"). The secret is carried only after `#`, never reaches the server, and holds no key.
27. Given the link, when B opens it (pasted into "Join shared workspace..." in the Share tab, or as a `curvyo://` link where the platform allows), then B is asked for "Name shown to others" if none is set, is asked to choose an empty folder for the workspace (B chooses where it lands; never inside another workspace), sees "Waiting for a member to let you in" until an online member admits B automatically (`0092` COOP-JOIN A), and sees no content before admission. A version `1.` link still opens one document as in `0092`.
28. Given B is admitted, then B receives the document list and every document, written as `.curvyo` files in the chosen folder in the same tree, within 30 s for a workspace of 20 documents of 1,000 objects on a LAN server (Proposal). The tree shows documents as they arrive with a progress count; a document that has not arrived can be opened as soon as it has. A and B then show two participants with names and fingerprints.
29. Given a link used once or past its expiry, when a third instance C opens it, then C is refused with the reason (used, expired) and sees no content; the server's stored data gained no readable content from the attempt (`0092` criterion 4).
30. Given B joined, then every document of the workspace opens in B as a shared document in the same session as in `0092` criteria 6 to 23 (live edits, presence, offline merge, peer-scoped undo), through the workspace's membership: no further invite per document.
31. Given the workspace is shared (one connection per workspace), then its documents are edited by both makers at once (A in one document, B in another, or both in one) with the guarantees of `0092`: criterion 6 of `0092` (300 ms p95) holds for the open document and for the document list, with 50 documents in the workspace, and changes of the list (criteria 35 to 40) do not slow it; frame time not above 16 ms of thread CPU caused by sync, as `0092` criterion 13.
32. Given a shared workspace with 50 documents, when A opens only one and B edits each of the other 49 once per second for 60 s, then there is still one connection, A's open document and the list are applied at once, the other documents are written to A's disk by a background writer without blocking the open document (Proposal: each at most every 5 s), A's frame time stays within criterion 31, and no document waits for "its turn" (no limit on rooms; there is one room).
33. Given B closes the app and reopens the workspace folder later (offline or online), then B's workspace opens as a shared workspace and reconnects without a new invite; changes made offline merge as in `0092` criteria 17 to 19 for the list and for every document.
34. Given the server's stored data for the workspace, when it is searched (database files, logs, captured request bodies), then none of the following is found: a document name `ZQX-DOC-7731`, a folder name `ZQX-FOLDER-7731`, a text marker inside a document, a stroke colour, the number of folders, the number of documents. Sealing cannot be disabled (ADR 0008). The server sees one room per workspace, each record's sequence, size and time, the count and total size of the snapshot parts, and which member identities connect when; that is all (ADR 0037 decision 11, towards the customer's WS-META: nothing but ciphertext). Test: two workspaces of the same total size and the same edit pattern, one of 3 documents and one of 30, give server data (files, records, headers, logs, captured bodies) of the same structure, in which the number of documents cannot be read.

### Live document list (shared workspace)

35. Given A and B in a shared workspace, when A creates a document, then its row appears in B's tree within 2 s on a LAN and the document is downloaded as in criterion 28.
36. Given A renames a document, then B's file is renamed on B's disk and B's tree updates within 2 s; if B has it open, it stays open and keeps unsaved changes (criterion 14); room, participants and history do not change.
37. Given A moves a document or folder into another folder, then B's file moves the same way within 2 s. Given A and B move the same document into different folders at the same time, then both converge within 2 s to the same one folder, the document exists once, and no error is shown (one wins, as `0092` criterion 12).
38. Given A and B create a document of the same name in the same folder at the same time, then both documents exist in both instances, the later one by the same rule on both sides gets " 2", and neither is lost. Given A and B rename the same document at the same time, then both converge to the same one name.
39. Given A moves a folder into a folder that B concurrently moves into the first (a loop), then both converge to a loop-free tree without losing any document (the one move that would close the loop is not applied; the rule is the same on both sides); the notice names it in at most 70 characters.
40. Given A deletes a document or a folder (criterion 17), then B sees a notice naming the document and who deleted it, B's local file goes to B's OS trash (never erased), B's tree drops the row within 2 s, and the document's state stays in the workspace room and in every later workspace snapshot; the deleted item can be restored by any member from the "Deleted" list of the File tab's Workspace section (WS-DELETE-ALL A). Restoring writes the file again from the last synced state, with the same document id. If B has the document open with unsaved changes, B's file is not trashed until B saved or discarded; B is told "Deleted by A" and can use Save as.
41. Given a file is added to the folder by another program while the workspace is shared, then it is not added to the workspace automatically: the row shows "Not in the shared workspace" with a button "Add to workspace" (Proposal: avoids sharing an unsuspected file). A document added this way gets its own new document id if it had none or its id is already in the workspace.
42. Given a document that is shared on its own (a `0092` room with its own keyring) and lies in the workspace folder when the workspace is shared, or is added by "Add to workspace" afterwards, when every current member of the document's own keyring is a current member of the workspace (Proposal, ADR 0037 decision 10), then it is taken into the workspace in one step by a device that is a member of both (no admin right needed): its merged state becomes a document of the workspace room (same history, later merges work), the old room is marked as moved so that no device syncs it or admits anyone there again, and the file now belongs to the workspace. The Share confirm or the Add confirm says beforehand that the document's own link stops working and that the people of the workspace can edit it (`ux-notes.md` section 6). There is no "shared separately" state in the tree (customer, WS-SEPARATE B).

### Removing a participant, and server

43. Given A (admin) and B (member) in a shared workspace, when A removes B in the workspace's participant list, then A's device writes, in this order, a `RemoveMember` entry, a `NewEpoch` entry with a fresh key wrapped to every remaining participant, and one fresh signed workspace snapshot, holding the list and every document, sealed under the new epoch (ADR 0031 decision 6a; one epoch sequence for the whole workspace, ADR 0037 decision 9). The dialog (in the Share tab) says in one sentence that removal does not reach back. A member who is not an admin sees no Remove action.
44. Given B was removed, then (a) the server closes B's connection to the workspace room with the reason "removed" and refuses new ones; (b) after the removal no envelope in the workspace room can be opened with B's keys; (c) B shows "You were removed from this workspace. Your copies stay on this computer." and keeps all documents as local editable files, no longer syncing and no longer a shared workspace (R-COLLAB-010). A third participant C who was online continues editing across the rotation without user action and loses no edit; C offline during the rotation seals its offline edits under the new epoch on reconnect.
45. Given a removal while B is offline, when B reconnects, then B learns of it from the keyring, is disconnected from the workspace room, cannot read the new epoch, and B's offline edits and offline renames are not merged into the workspace.
46. Given the workspace is shared, then the invitation of a new member after a removal reaches the whole workspace (the workspace room), with the history choice of `0092` slice 2 not part of slice 1 (every new member receives the current state only; Proposal).
47. Given the server, then no new setting or file is needed beyond `0092` criteria 35 to 40: a server holds several workspaces and several single documents of one maker as separate rooms (one room per workspace), verified by a test with two workspaces of A on one server (own invites, own participant lists, no cross-talk); data of all rooms stays in the one data directory.
48. Given the server is restarted while A and B are connected in a workspace, then both reconnect to the workspace room without user action within 30 s and lose no edit, including edits to the document list (`0092` criterion 20).
49. Given a document shared on its own whose own keyring has a member who is not a member of the workspace (Share of the workspace, or "Add to workspace"), then it is refused: it is not taken in, its row stays "Not in the shared workspace", the action slot says "Lid was not added: it is shared with people who are not in Box job." (names formatted by the app; the Share confirm names it as "Not added"), and it keeps syncing on its own room as in `0092`. Given a document that was taken in (criterion 42), when someone opens its old invite link, then nobody is admitted to the old room, and a member of the workspace who still holds the old copy is told where the document now is and that its old copy is a local file.

## Decided in ADR 0037

Mechanisms for the criteria are in `docs/adr/0037-shared-workspace-list-rooms-and-keys.md` and `adrs.md` (accepted 2026-11-03): marker directory and `share.json` v2, `keyring.log`, `list.loro` (1, 2, 23, 24); one room per workspace with every document as an encrypted stream, multi-part workspace snapshots (25, 28, 31, 32, 34, 43); invite `2.` and join into an empty folder (26, 27, 29); a background writer for documents that are not open (32); the Loro movable tree with loop rule, name registers and tombstones (35 to 40); take-in and `Moved` (42, 49); what the relay sees (34). The `.curvyo` format version is unchanged. Debts named there: changes pushed to the old room across the take-in instant stay local; the old room's data stays on the relay.

## Slice 2 outline: sync across the maker's own computers (Proposal)

- The same workspace opens on a second computer of the maker without an invite: pairing between the maker's own devices by the verified flow (ADR 0010 §7), the same rooms on the same server.
- Works offline and merges as in `0092` criteria 17 to 19; folder sync (Nextcloud) stays a separate option (R-SYS-004).
- Own spec after slice 1 is accepted.

## Slice 3 outline: workspace library (Proposal)

- Material records and shared assets (fonts, images, palettes) that all documents of a workspace see; ADR 0004 §6 (one file per record) makes it a folder of record files that syncs through the workspace room. Needs `0028`. Scope is the customer's answer WS-LIBRARY A: not before then; slice 1 reserves no library folder.

## Out of scope

- Sync across the maker's own computers (slice 2), library (slice 3), several documents open at once and objects between documents (slice 4), linked clones across files.
- Read-only members, per-document permissions or a per-document opt-out (cannot be enforced, ADR 0010 §12; WS-VISIBLE A: every member sees every document).
- Rest of the membership lifecycle (second admin, leave, re-add with history choice, verified flow, concurrent removal): the slice 2 of `0092` applies to workspaces once built; this spec only needs invite, admit, remove.
- Browser and mobile: no folder access in the browser; a browser workspace is a later decision.
- Search inside documents, tags, favourites, sorting beyond name and last change.
- Project metadata (client, notes), a job list per workspace (`0063`).
- Version history of files, a trash inside the app (OS trash only; the "Deleted" list of criterion 40 lists shared deletions, not an app trash), conflict handling for a folder synced by another tool.
- Importing a folder of SVG files as documents (`0024`).
- Templates and a start screen (`0051`).
- Comments, chat, following a viewport (as `0092`).
- Hosted instance, TLS in the server, quotas, replicas (debts of `0092`).
- Old-file compatibility (customer, 2026-10-30).

## UX notes

Full notes in `ux-notes.md`. Decided by the customer: the tree, Deleted list and recent workspaces live in a Workspace section of the existing File tab (WS-UI); sharing, Invite, participants, Remove and "Join shared workspace..." live in the Share tab (WS-SHARE-PLACE B). The Share tab shows a document that belongs to a shared workspace as "Shared through workspace X", with no separate invite. Row states: "Not in the shared workspace"; there is no "shared separately" state.

## Customer decisions (2026-11-03)

- WS-DISK: A. A workspace is a folder; documents stay ordinary `.curvyo` files; a marker directory `.curvyo-workspace/` holds the workspace id.
- WS-FILES: A. Any folder with `.curvyo` files opens as a workspace without conversion; Open of a single file still works and sees no workspace.
- WS-SHARE: B. Sharing a whole workspace is in slice 1: invite version `2.`, one workspace room carrying every document as an encrypted stream (ADR 0037; the first answer said "one room per document", revised by WS-META), joining a workspace, documents added, renamed, moved and deleted by one member appear for all, removal of a member from the workspace. The old slices 2 to 5 moved down by one.
- WS-LIBRARY: A. No library in slice 1; later slice, after `0028`.
- WS-UI: The workspace lives inside the existing File tab, not in a new tab.
- WS-DELETE: A. Delete goes to the OS trash with one inline confirmation; in a shared workspace a deletion reaches every participant (criterion 40).
- WS-VISIBLE: A. Every member of a shared workspace sees and edits every document; one keyring per workspace; no per-document privacy in slice 1.
- WS-DELETE-ALL: A. Any member may delete a document for all, with the confirmation; every device moves it to its OS trash and any member can restore it from the Deleted list (criterion 40).
- WS-SEPARATE: B. A document that is shared on its own is taken into the workspace (criterion 42). Refused where its own members are not all workspace members (criterion 49, Proposal of ADR 0037 decision 10).
- WS-SHARE-PLACE: B. Sharing a workspace, Invite, participants, Remove and "Join shared workspace..." live in the Share tab; the File tab keeps the tree, Deleted and recent workspaces.
- WS-META: "the server sees nothing but ciphertext, towards B": the relay sees one room per workspace and sizes and times, not the number of documents, names or folders (criterion 34). Led to ADR 0037: one workspace room with documents as encrypted streams.
- ADR 0037 accepted (2026-11-03).
- WS-JOIN-PLACE (UX notes, default recorded, not asked): "Join shared workspace..." only in the Share tab, not also in the File tab's empty state.

## Open questions

None. Every question has a customer answer or a recorded default.

Not asked, because a safe Proposal is marked in the criteria: view state per document (10), "Undo move" (16), no auto reopen of the last workspace (20), invite validity 24 hours (26), background writer at most every 5 s per document (32), take-in only when the old members are all workspace members (42, 49), "Add to workspace" for outside files (41), no history choice for a new member in slice 1 (46), Join only in the Share tab (WS-JOIN-PLACE).

## Links

Requirements: R-SYS-002, R-SYS-004, R-SYS-007, R-COLLAB-001, R-COLLAB-003, R-COLLAB-005, R-COLLAB-008, R-COLLAB-010, R-MAT-005 in `docs/requirements.md` (a requirement for the workspace itself is to be added); ADR 0004 §1, §5, §6, 0008, 0009, 0010, 0031 decisions 6a and 8, 0037 in `docs/adr/`; `specs/0095-workspace/adrs.md`, `specs/0095-workspace/ux-notes.md`; `specs/0092-cooperative-editing/`, `specs/0051-sidebar-and-no-menu/`, `specs/0001-project-file-foundation/`.
PR: none yet.
