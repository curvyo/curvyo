# Sharing concept: the workspace is the only unit of sharing

Status: Ready
Priority: Must
Origin: Customer (DOC-SHARE-REMOVE B, 2026-11-06: "wir gehen immer auf Workspace"; the other decisions below, 2026-11-09). The wording of the texts is a Proposal until the demo.

## User value

As a maker who draws on several devices and works with other makers I want one model for sharing and syncing, so that I always know what leaves my computer, who sees it and where I do it: a workspace is the only thing that is synced or shared, and everyone in it sees and edits every document.

## The model

The workspace (a folder of `.curvyo` documents) is the only unit that is synced or shared. A workspace is Local (this device only), Synced (on the server and all my devices) or Shared (Synced, and another person is in it). To share one drawing, the maker puts it in a workspace. Word for a computer or browser: "device". The server stores and forwards sealed data it cannot read; the maker never sees the word "room".

## Customer decisions (final)

- DOC-SHARE-REMOVE: B. Sharing a single document is removed everywhere.
- SHARE-UNIT: A. "Make a workspace for it" for a file outside a workspace.
- ONE-SWITCH: A. One Sync switch per workspace; Invite turns it on.
- PEOPLE-VIEW: A. Participants grouped by person for display; roles and Remove stay per device.
- MEMBER-DEVICES: C. As built (`0096` criterion 17): a workspace I was invited into does not reach my other devices by itself.
- JOIN-FLOW: A. One Join field.
- STOP-SHARING: B. Remove one by one; the sole remaining participant gets an armed "Stop sharing" (built in PR #294, `0097` criterion 43). No "Remove everyone".
- DEVICE-WORD: A. "device" everywhere.
- BROWSER-PEOPLE: A. The browser takes part only as one of my own devices (it cannot be invited by a person or invite one).
- LEGACY-SHARE: A. Files that were shared on their own open as local documents.

## Acceptance criteria

Setup: desktop instances A and B (each its own data directory and credential store), A2 paired with A (`0096`), one `curvyo-sync-server`. "Within N s" means a LAN server with both online. UI is the Share tab; texts come from `curvyo-ui-core`.

### One unit

1. Given any document or workspace state, then the Share tab has no Document block with Share, no document participant list, no Leave or Add back of a document, and no "Open shared document…". Every Share, Invite and Join control acts on a workspace.
2. Given a document inside a Synced or Shared workspace W, then the File tab shows "Shared through workspace W" and the document has no share state of its own.
3. Given a `curvyo://join?…#1.…` link (single document), when it is pasted into the Join field, then the app shows "This link is for a single shared document. Documents are now shared in workspaces: ask for a workspace invite." and sends no network request (checked with the server stopped and a request log on the test server).

### Old shared files (LEGACY-SHARE A)

4. Given the golden files `shared_v34.curvyo` (share record and keyring) and `ended_v36.curvyo` (ended record), when each is opened, then it is an editable local document: no "(shared)" in the title, no connection attempt, no key read, and one notice "This document was shared on its own. Documents are now shared in a workspace; it opens as a local copy." The file on disk is byte-identical after opening (`open_writes_nothing`).
5. Given such a file, when it is saved, then the saved file holds neither `share.json`, `keyring.log` nor `share.ended.json` and opens without the notice.
6. Given a workspace folder whose documents carry `share.json` v2, and a marker with `share.json` (v1 shape), `keyring.log` and `share.ended.json`, then they open exactly as before (golden `workspace_marker_share_v1.json`, `workspace_shared_v35.curvyo`).

### Make a workspace for it (SHARE-UNIT A)

7. Given a file F outside any workspace, then the Share tab shows one line "Sharing works on workspaces." and one button "Make a workspace for this file".
8. Given the button, then an inline confirm names the new folder `<F's parent>/<F's name>/` and says the file moves there; if that folder exists the name gets " 2", " 3"; it is never F's current parent folder. Given Confirm, then the folder is created, F (saved first if it had unsaved changes, with no change lost) is moved into it, the folder opens as a Local workspace with F still open at its new path, and the Workspace block offers Sync and Invite person. Given Cancel, nothing changes on disk.
9. Given the browser, then the button is not offered and the line says that workspaces are made in the desktop app.

### One Sync switch (ONE-SWITCH A)

10. Given a new or opened Local workspace, then it shows "Local" and one switch Sync (off). Switching it on shows one confirm saying that names, folders and files leave this computer encrypted; after Confirm the workspace shows "Synced" and appears on A2 within 30 s for 20 documents of 1,000 objects (as `0096` criterion 10).
11. Given a Local workspace, when the maker chooses Invite person and confirms, then Sync is turned on in the same confirm (one dialog, not two) and the invite link is shown. The invite is valid 24 hours; no validity choice is offered.
12. Given a Shared workspace (another person is in it), then the Sync switch is greyed with the reason "Other people are in this workspace"; given only the maker's own devices, the switch can be turned off and the workspace is Local again on every device that had it.
13. Given the Share tab, then there is no separate "Share workspace", "Sync to my devices" or "Stop syncing" control.

### People (PEOPLE-VIEW A, STOP-SHARING B)

14. Given a Shared workspace in which Marc has two devices and Lena one, then the list shows "Marc (you) · Admin · 2 devices" and "Lena · Member · 1 device"; a person row expands to one row per device. Role changes, Remove and Leave act on one device, as `0097` criterion 2. Rows are grouped by the label the device sealed; a device with no label shows "A device".
15. Given an admin, then there is no "Remove everyone"; people are removed one by one. Given one participant is left (every other left or was removed), then that row offers an armed "Stop sharing" (`0097` criterion 43).

### Join (JOIN-FLOW A, DEVICE-WORD A, MEMBER-DEVICES C)

16. Given the Share tab, then there is one entry "Join…" with one field. Pasting a person's workspace link (`2.`) lets the device in automatically with no check words (COOP-JOIN A), once a member is online. Entering a code from the maker's own device or from an admin shows four check words on both screens and joins only after the existing device or admin confirms.
17. Given B joins A's workspace W by link, then W is on B's device only; it does not appear on B's second device B2 without B doing something (`0096` criterion 17).
18. Given every user-visible text of the Share tab and the share flows in `curvyo-ui-core`, then none contains "computer" or "this browser" for a device, and the add action reads "Add device" (a test over the texts).

### Browser (BROWSER-PEOPLE A)

19. Given the browser, then it can be added as one of the maker's own devices and shows that maker's synced workspaces (`0096` criteria 16 and 38). It cannot accept a person's workspace link or create an invite: the Join field there accepts a device code only, and a `2.` link shows "Open this link in the desktop app." (Proposal wording).

### Share tab stays; server reachability

20. Given the Share tab is shown, when the maker makes a workspace for a file, switches Sync on, invites a person, joins, or adds a device, then the selected tab is the Share tab before, during and after; the Document tab is never shown in between (a browser journey records every tab selection).
21. Given a server address is set and the Share tab is shown, then the reachability line re-checks about every 5 s. Given the test server is stopped, then the line says the server cannot be reached within 10 s; given it is started again, the line says "Connected" within 10 s, without the maker pressing anything. Given no server address, no check is made. The check stops when the Share tab is not shown.
22. Given a failure line such as "Could not sync Box job: cannot reach the server.", then it disappears as soon as it stops being true.

### Nothing else breaks

23. Given a Shared workspace of A and B, then live editing, presence, offline merge, undo, sealing and the server behave as `0092` criteria 6 to 31 and 35 to 39; a browser journey (`cooperative-editing.cjs`) covers criteria 6, 7, 14, 15, 17, 18 and 32 on a shared workspace.
24. Given an unshared document, workspace or a Local workspace, then the app makes no network request, except the reachability check of criterion 21 (a probe that sends nothing) while the Share tab is shown and a server address is set (R-SYS-002, R-SYS-007).
25. Given Linux, then the criteria are verified there; Windows and macOS run the same code in the nightly.

## Superseded by this spec

- `0092`: criteria 1 to 5, 33 (the document's own room), 40 to 45. Criteria 6 to 31 and 35 to 39 stay and hold through a workspace's open document.
- `0095`: criteria 12, 42, 49 (a separately shared file, take-in, `Moved`), the version `1.` exception in 27, the single-document rooms in 47, WS-SEPARATE B.
- `0096`: criterion 9 ("Share workspace" button) merges into Sync; criterion 13 ("Stop syncing" with others in the workspace) cannot exist any more (criterion 12 here).
- `0097`: every "for D" half; criteria 18 and 22 (History choice); document part of 33 (Frage 160, READD-DEFAULT). The amendments of PR #281 and #294 stay.

## Out of scope

- "Remove everyone", deleting the server copy (STOP-SHARING A and C).
- Inviting the browser into a workspace, or the browser inviting (a later story if the customer asks).
- Members adding their own devices without an admin (MEMBER-DEVICES A and B), and the verified join of `0097` slice 2.
- Roles per person; keys and roles stay per device (ADR 0010 decision 1). Grouping is display only; other people cannot verify it.
- Read-only roles, sharing part of a workspace, copying instead of moving the file at "Make a workspace for it".
- Offering to move a legacy shared file into a workspace on open.
- Deleting old document rooms on the server or old epoch keys on disk (tracked in `docs/technical-debt.md` by the architect).
- Phone companion.

## UX notes

Layout and wording: `docs/design-system.md` and the Share tab as built. Proposal for the ux-engineer to confirm at review: Workspace block states "Local", "Synced", "Shared", "Ended. Your copy stays here."; "Marc on Workshop PC" line with the server address and reachability; My devices as a short list below the workspace; buttons size to their label and wrap, never clipped; no explanatory sentences on armed actions.

## Done at merge

This slice also closes the whole sharing feature. When it merges, `0092`, `0095`, `0096`, `0097` and `0105` get `Status: Done (PR #n)` in the same PR.

## Links

Requirements: R-COLLAB-001, R-COLLAB-007 to R-COLLAB-013, R-SYS-002, R-SYS-007 in `docs/requirements.md` (wording "shared document" becomes "shared workspace" with this slice).
ADRs: `adrs.md` in this folder (architect). Related: `specs/0092-cooperative-editing/`, `specs/0095-workspace/`, `specs/0096-own-devices-sync/`, `specs/0097-membership-lifecycle/`.
PR: none yet (earlier work: #281, #294).
