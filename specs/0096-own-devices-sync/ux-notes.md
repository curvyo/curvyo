# 0096 Own-devices sync: UX notes

Status: Draft for Ready (slice 1). Owner: ux-engineer, 2026-11-04. Authority: `specification.md` (visible criteria: 1 to 3, 5, 6, 8 to 10, 13, 14, 16, 17, 22 to 28), customer decisions of 2026-11-04: DEV-UI A (Share tab: a "My devices" block plus a per-workspace switch in the Workspace block), PAIR-METHOD A (one-time code typed on the new device, four check words compared on both screens), SYNC-SCOPE B (switch per workspace; existing ones preselected at pairing, new ones off), DEV-ROLE A, DEV-LOST A (no recovery code; warn while only one device is paired), DEV-BROWSER B (a browser pairs as a device; clearing site data loses that device and the UI says so). References: `docs/design-system.md` ("Properties panel: Share tab and collaboration", "shared workspace", "Inline confirm", "Canvas notice slot", "Interaction conventions"), `specs/0095-workspace/ux-notes.md` (Workspace block, Join block, removed device), `specs/0092-cooperative-editing/ux-notes.md` (participant row, Remove confirm, connection states), `ShareTab.tsx`, `WorkspaceShareBlock.tsx`, `components/ui/switch.tsx`. **(Proposal)** marks reversible choices. Customer rule 2026-10-16: no operating tips or hint labels; labels that name a control or a state stay, and so do the sentences that state a consequence (what leaves the machine, what removal does not undo). Texts with state or numbers are formatted in Rust (`curvyo-ui-core`).

**Noun (Proposal):** the UI says "device", not "computer", because a browser is a device too (DEV-BROWSER B). "This device", "Add a device", "Join my devices", "Remove". The product owner's criteria say "computer"; the texts here win for the UI and the noun is one Rust parameter ("device" / "browser" where a sentence is about a browser). Slice 1 is the desktop app; the browser parts (section 9) ship with the browser slice.

## 1. Where it lives

- **Share tab only** (DEV-UI A). No new tab, pill, toolbar, dialog or Settings page. The OS folder picker is the only floating surface (Synced workspaces folder). Order of the tab, top to bottom: Server and Name rows (as `0092`, unchanged) / **My devices block** (new) / Workspace block (`0095`, gains one switch row) / Document block / the two open and join buttons. Nothing of `0092` and `0095` moves.
- The My devices block is device-level: it is shown whether or not a workspace or document is open and whether or not anything is shared.
- The File tab gets no sync controls. Its Workspace header line and the window title show the sync state of the open workspace (section 5.3); the collaboration pill keeps its meaning (section 8).
- Block heading: "My devices" 14px semibold at the left; at the right, 12px `--panel-muted-fg`: "Only this device" / "2 devices". Divider under the block as the Workspace block has (1px `--toolbar-icon` at 25%).

## 2. My devices block

| # | Part | Shown when |
|---|---|---|
| 1 | Alert: "This device was removed from your devices. ..." with **OK** | this device was removed (section 7.2) |
| 2 | Device list | always (own row at least) |
| 3 | Warning line (only one device) | a workspace is synced and this is the only paired device |
| 4 | Decision rows ("Connect this folder") | a copy of a synced workspace exists here (criterion 14) |
| 5 | Pairing buttons or the open pairing block | not paired / paired; sections 3 and 4 |

Absent parts are not in the tree (hidden, not disabled), except the reason line of a disabled button.

### 2.1 Device list

`ul` named "My devices". Own row first, then by pairing order. One row is a 56px block (60 coarse), three lines, left to right: device glyph 16px (`Monitor` desktop, `Globe` browser; the kind is also text on line 3 for a browser), a text column, and a 28px action slot at the right (reserved on every row so text does not shift):

- Line 1, 14px `--toolbar-icon`, ellipsis: the device name; on the own row followed by the muted "this device".
- Line 2, 12px mono `--panel-muted-fg`: the fingerprint, 16 base32 characters in four groups (the helper of the participant row). While this device has no keys yet (nothing paired or synced), the own row shows no fingerprint and line 2 is absent; the row is 40px then.
- Line 3, 12px `--panel-muted-fg`, always text: other device: "Online" / "Last seen 12:03" / "Last seen 2 Nov 12:03" (before today) / "Not seen yet" (paired, never connected since this device knows it; **Proposal**, implementer 2026-11-04); a browser adds "Browser, " in front (a browser marks itself in the devices list, ADR 0040 decision 1). A device the group's keyring lists but the devices list names not (yet) is shown as "A device", so no participant is hidden (ADR 0040 decision 8). Own row: the aggregate state of its synced workspaces (table in 5.2), or nothing when none is synced.
- **Details.** A row with at least one synced workspace is a `button` (the text column) with `aria-expanded` and a `ChevronRight`/`ChevronDown` 16px before the glyph; Enter or Space toggles. Expanded, under the row: one 24px line per synced workspace, name left (14px, ellipsis), its state right (12px muted): criterion 22. Collapsed by default. A row without synced workspaces has no chevron (the slot is reserved) and is not a Tab stop.
- **Rename (own row):** F2, or Enter on the name in edit mode via double-click, opens the `EntryField` in place of line 1 (1 to 24 characters, default the OS host name, Enter commits, Escape restores, "Enter a name." on empty). The name is announced to the other devices within a second and is the second half of what other makers see: "Marc (Laptop)" (criterion 16). No separate Settings field.
- **Row action:** other devices: **Remove** (`Unlink` 16px, border-less outline, tooltip "Remove", accessible name "Remove Laptop"); own row: **Leave my devices** (`LogOut`, tooltip "Leave my devices", accessible name "Leave my devices"). Both are Tab stops after their row's details button. One route for a lost device: the Remove of its row (an offline device shows "Last seen ..."); there is no separate "lost device" command.
- Live regions: line 3 and the detail lines are `role="status"`, polite, throttled to once a second (the same rule as the File tab header line); no per-document announcements.

### 2.2 Warning: only one device (DEV-LOST A, criterion 28)

While at least one workspace is synced and the own row is the only row, a persistent line under the list: `TriangleAlert` 16px (`--toolbar-icon`) and 12px text, `role="status"`: "Only this device is paired. If you lose it, the copy on the server cannot be opened by anyone. Pair a second device as a backup." It is a state, not a tip: it is shown exactly while the risk exists and leaves with the second device or the last switch-off. The first switch-on carries the same first two sentences in its confirm (section 6.2); no separate notice. Not a block: nothing is disabled.

### 2.3 Decision rows (criterion 14)

Shown under the list for each synced workspace whose copy exists here already (same workspace id) and has not been answered. One row, `role="group"` named "Box job", 12px: "Box job is already in ~/Work/Box job. Connecting merges both copies; nothing is lost." Buttons **Connect this folder** (Apply look, 148 x 28) and **Make a second copy** (outline). Neither is focused on appearance and focus does not move (a status line, not an error). Until answered, that workspace's detail line says "Waiting for your answer". Connect runs the offline merge; "second copy" creates `Box job 2` in the Synced workspaces folder. Escape does nothing here.

### 2.4 Empty and not-paired state

Own row, then the two buttons (4.1). No illustration, no explanation. Server not set: both buttons `aria-disabled` at 40% with their Tab stop and the reason line "Set a server address."; unreachable: "Cannot reach the server."; no name: "Enter a name." (as Share). The app stays fully usable offline (criterion 5); the reason is the only difference.

## 3. Pairing, the old device (shows the code)

Pressing **Add a device** (148 x 28, Apply look, `MonitorSmartphone`) replaces the buttons in place by the **pairing block**, a column with 8px gaps, focus on the block's first element (the code group, `tabindex="-1"`, so a screen reader reads it). Escape or **Cancel** at any step before Pair ends the attempt, the code dies, focus returns to Add a device.

1. **Code.** Label "Pairing code" 12px. The code in two groups of four, 24px mono semibold, 0.08em tracking, `--toolbar-icon`, on a 1px bordered `rounded-[5px]` field surface (`--toolbar-icon` at 25%), centred, `user-select: all`, `aria-label` with the characters spelled ("K, 7, Q, 2, 9, M, 4, X"). Under it 12px muted: "Server 192.168.1.20:8080" and "Valid for 4:52" (counts down visibly per second, `aria-hidden`; one polite status at "1 minute left" and at "expired"). Then the sentence of criterion 1, 12px `--toolbar-icon`: "The device that enters this code can sync the workspaces you choose, and you can remove it again. The server can tell which devices belong together, not their names." Then 12px status "Waiting for the other device." and **Cancel**. No Copy button: the code is typed, not sent (nothing passes through a messenger).
   - Wrong tries seen by the server: status "Someone entered a wrong code 2 times." (polite).
   - Locked or expired: replaced by an alert (`role="alert"`, `--field-invalid`): "Too many wrong tries. This code no longer works." / "This code has expired." with **New code** (Apply). A used code or a wrong code the relay ended: "Pairing stopped. The code no longer works." A lost connection: "Lost the connection. Pairing stopped. Nothing was shared." with **New code**.
2. **Compare.** A device entered the code and its keys arrived: the group is replaced by: 14px "Laptop wants to join" (user text, ellipsis), its fingerprint (12px mono muted), then the **four check words**: a 2 x 2 grid, 8px gaps, each word 16px semibold on the same bordered surface, `--toolbar-icon`, left-aligned; label above 12px: "Check words". Under it the sentence "They must match on Laptop." (the one fact the screen needs: which side to look at). Buttons **Pair** (Apply, 76 x 28) and **Words differ** (outline), in that Tab order. Focus goes to the group (named "Laptop wants to join. Check words: maple, otter, fjord, lamp."), never to Pair, so Enter cannot pair by reflex. **Words differ** ends the attempt, burns the code and shows the alert "Pairing stopped. The code no longer works." Escape = Words differ.
3. **Pairing.** After Pair: "Pairing Laptop..." (`aria-busy`, no buttons). Failure: alert "Could not pair: cannot reach the server." with **New code**; nothing was written. Done: the new row appears in the list (focus stays in the block), the block continues with 3.1 at once; no success notice (the row is the result).
4. **Choose workspaces** (3.1) then the block closes.

### 3.1 Choosing workspaces at pairing (SYNC-SCOPE B, criterion 8)

Shown on the confirming device (A, or B when C paired through B) right after Pair, in the same block. Heading line 14px semibold: "Workspaces for Laptop". Content:

- A `ul` named "Workspaces to sync" with a check box row each (32px; the whole row is the label and the target; native check box 16px + name 14px ellipsis + 12px muted "14 documents" at the right), in this order: open workspace first, then recent ones, newest first, only folders that exist (a "Not found" recent is not offered). **All boxes are checked** (Proposal of the spec). At most five rows high (160px) with an inner scroll beyond; no virtualisation (10 recents at most).
- A workspace in which this device is only a member: unchecked, check box `aria-disabled`, line 2 "Not an admin here" (criterion 17); not an error.
- Workspaces already synced are not rows: one muted line above the list, formatted in Rust: "Already synced, they arrive on Laptop too: Box job, Poster." Absent when none.
- Sentence 12px under the list, once: "Names, folders and files leave this device encrypted, to your own devices only. The server can tell which devices belong together, not their names." (criterion 9 and 30; this is the confirm, so no second one follows.)
- Buttons **Not now** (outline) and **Sync these** (Apply look; `aria-disabled` with the reason line "Choose a workspace." while none is checked). Focus on the first check box. Escape = Not now. Not now syncs nothing and the maker finds the switch later (section 6).
- After Sync these: the block shows `role="status"` "Syncing 2 workspaces: uploading 3 of 14" (the 0095 progress text, aggregated), then closes. Failure per workspace: alert "Could not sync Box job: cannot reach the server." naming it; the others continue; the switch of that workspace is off.

## 4. Pairing, the new device (types the code)

### 4.1 Buttons

**Join my devices** (outline, 148 x 28, `LogIn`) sits next to Add a device. Shown only while this device has no pairing and no synced workspace; otherwise it is absent (hidden, not disabled), because joining another set would silently leave the current one **(Proposal)**. Disabled with the same reasons as Add a device. It opens the **join block** in place.

### 4.2 Join block (step 1, "Connect")

In order: field "Pairing code" (autofocus, mono 16px, one line, `autocomplete="off"`, `spellcheck="false"`, `autocapitalize="characters"`; input is upper-cased, spaces and dashes ignored, paste works, shown as two groups of four; Enter submits); read-only line "Server 192.168.1.20:8080" (12px muted; the Server field of the tab is the source); field "Device name" (prefilled with the OS host name, the rename `EntryField` rules; Enter in it moves to Connect); the **Synced workspaces folder** row: path line `~/Curvyo` (12px, ellipsis in the middle, full path in the tooltip) and **Change folder...** (outline, `FolderOpen`); refusals as `0095` section 8: "Choose a folder outside any workspace." / "Cannot write to this folder." The default folder is created when absent and the line says nothing about it. "Name shown to others" appears only when none is stored (the `0092` field). Buttons **Cancel** and **Connect** (Apply; `aria-disabled` with the reason line "Enter the code." until 8 characters are present). While busy the form is read-only.

### 4.3 Waiting and compare (step 2)

After Connect: status "Connecting..." then "Waiting for Desktop to confirm" (`role="status"`) and the **check words** exactly as on the old device (same grid, same label "Check words", same sentence "They must match on Desktop."), focus on the group. Buttons **Cancel** and **Words differ** (outline both; B never pairs by itself). B shows no content, no workspace names, no list before the old device presses Pair. Closing the old device does not end the wait; B keeps waiting until the code expires (criterion 5).

### 4.4 Done (step 3)

On Pair: the block closes, the list shows both devices (own row "this device"), focus on the first row's details button (or the Add a device button when no row is expandable). Workspaces arrive without input: canvas notice `role="status"`, 5s, "Workspace Box job arrived." (at most 70 characters, name cut to 24); the own row's line 3 reads "Receiving 7 of 20" and the arrived workspaces are listed under **Recent workspaces** of the File tab; none opens by itself (`0095`). A name taken in the folder gets " 2" and the notice says the final name. A file name the OS refuses (criterion 15): one notice "Renamed a:b to a-b on this device." plus the row in the details; the document id is kept.

### 4.5 Errors on the new device

All `role="alert"`, `--field-invalid`, message line under the code field (the field keeps its value except where noted), `aria-invalid` on the field. Nothing of the other device is shown.

| Case (criterion 4) | Text |
|---|---|
| Wrong code | "That code does not match. 3 tries left." (count shown from the first wrong try; field text kept, selected) |
| Locked (5 wrong in a row) | "Too many wrong tries. Ask for a new code." Field read-only and cleared; Connect `aria-disabled` |
| Expired | "This code has expired. Ask for a new one." |
| Already used | "This code was already used." |
| Server not reachable | "Cannot reach the server at 192.168.1.20:8080." |
| Expired while waiting at the compare step | "The code expired before Desktop confirmed. Ask for a new code." (back to step 1, field cleared) |
| Old device cancelled or pressed Words differ | "Pairing was stopped on Desktop. Nothing was shared." (back to step 1) |
| This side pressed Words differ | The block closes with "Pairing stopped. Nothing was shared." (status) |
| Connection lost while waiting | "Lost the connection. Pairing stopped. Nothing was shared." with **Try again** (back to step 1) |

The old device shows the matching alert for the case it learns of (3 step 1 and 2): "The words did not match on Laptop. Pairing stopped." when the other side pressed Words differ.

## 5. Sync states

### 5.1 Words (all `role="status"`, polite, throttled to 1 per second)

| State | Text | Shown |
|---|---|---|
| Current | "In sync" | |
| Catching up | "Syncing 3 of 14 documents" | `RefreshCw`, skipped if under 1s |
| Offline | "Offline, 2 changes waiting" | `CloudOff`; the changes are kept (R-SYS-002) |
| First copy not delivered yet | "Waiting for first copy" | on the owner's side for a device that has not connected since the switch-on |
| First copy arriving | "Receiving 7 of 20" | on the receiving device, as `0095` |
| Waiting for an answer | "Waiting for your answer" | decision row open (2.3) |
| Not an admin | "Not an admin here" | |
| Removal incomplete | "Removal incomplete, 1 of 2 workspaces" | row of a device that is still member of a workspace in which this device is not admin |

A device that is not connected is not "Offline" from another device's view: it shows "Last seen 12:03" (line 3). "Offline" is only ever the state of this device. Static glyphs under reduced motion.

### 5.2 Where they show

Own row line 3 (the worst state wins: Offline over Syncing over In sync), the detail lines per workspace (own row: own states; other row: "In sync" / "Syncing 3 of 14 documents" / "Waiting for first copy" / "Last seen 12:03"), the Workspace block's sync line (6.1), the File tab header line and the title bar (5.3).

### 5.3 Header and title

File tab header line of a synced workspace: "Synced, 2 devices" / "Syncing 3 of 14" / "Offline, 2 changes waiting" / "Receiving 7 of 20" (one line replaces `0095`'s "Shared, 2 people" when every member is an own device; with other makers it reads "Shared, 3 people" and the sync state stays in the Share tab). Window title unchanged (`0095`).

## 6. The switch "Sync to my devices"

### 6.1 Placement and states

First row of the Workspace block, under the headed line, above Share workspace and above everything the block holds. A 28px row (36 coarse): the label **Sync to my devices** (14px) left, the `Switch` (`components/ui/switch.tsx`, `role="switch"`, the look of the Export SVG switches) right; the label is the switch's accessible name. Under it, 12px muted, one line:

| Switch | Line under it |
|---|---|
| Off, can be turned on | none |
| Off, cannot | reason (`aria-disabled`, Tab stop kept): "Set a server address." / "Cannot reach the server." / "Enter a name." / "You are not an admin of Box job. Ask an admin to add this device." (criterion 17; no error, nothing syncs) |
| Turning on | "Uploading 3 of 14" (`aria-busy`, switch inert) |
| On | the state line of 5.1 for this workspace plus the number: "In sync with 2 devices"; with no other device paired: "In sync. Only this device." |
| Turning off | "Stopping..." (`aria-busy`), with a count "2 of 2 devices" when keys are renewed |

The Workspace block's header-right text reads "Synced, 2 devices" while every member is an own device, else the `0095` text ("Shared, 3 people"). When the switch is on the workspace is a shared workspace: the block shows the `0095` shared state below the switch (server line, Invite for admins, participants). **Share workspace is absent while the switch is on** (it would be a second route to the same state); with the switch off and the workspace shared with others it is not shown either (it is already shared). In the participant list own devices carry the muted "my device" after their label and have no Remove button there (one route: My devices), so there is one place that removes a device.

### 6.2 Turn on (criterion 9)

The switch does not move on press. The Inline confirm (two buttons, Apply look, **Cancel** focused) replaces the switch row in place, formatted in Rust: "Sync Box job (14 documents) to your devices? Names, folders and files leave this device encrypted, to your own devices only. The server can tell which devices belong together, not their names." Extensions in the same sentence block, as `0095`: documents shared on their own that are taken in ("Lid and 2 other documents are shared on their own. They become part of the workspace and their own links stop working."), "Not added: Lid (shared with people outside this workspace)."; and, only while no other device is paired: "If you lose this device, the copy on the server cannot be opened by anyone. Pair a second device to have a backup." Buttons **Cancel** and **Sync**. Escape and Cancel close it, focus returns to the switch, nothing was written. On Sync the switch shows on in `aria-busy`, with "Uploading 3 of 14". A failure: the switch returns to off, alert under it "Could not sync: cannot reach the server." Workspaces created, opened or copied later start off and no prompt appears (SYNC-SCOPE B).

### 6.3 Turn off (criterion 13)

Inline confirm, **Cancel** focused, action **Stop syncing** in the destructive look (400ms guard, as every destructive confirm): "Stop syncing Box job? Laptop and Phone keep their copies as local files and get no later changes. They are removed from this workspace." Where others share it: add "Box job stays shared with Ben." Offline: refused before anything is written, `role="alert"`: "Not connected. Try again when connected." and the confirm stays. The switch works the same on every device: the off switch of one device ends it for all of them (the sentence names them). Where this device is not an admin of a workspace the other devices belong to, the switch is off-limited with the reason of 6.1.

### 6.4 Devices that stop receiving

On a device whose workspace was switched off: canvas notice `role="status"` 5s "Box job is no longer synced. Your copy stays here." The folder, its documents and its row in Recent stay; the workspace becomes local (Share tab: "Not shared"). Turning the switch on again later: the decision row of 2.3 appears on the device that still holds the copy.

## 7. Removing a device

### 7.1 Remove, and Leave

- **Remove** opens the Inline confirm below the row (destructive look, Cancel focused, 400ms guard, Escape and Cancel return focus to the row's Remove): "Remove Laptop? It stops syncing. Removing does not reach back: the copies already on Laptop stay there as local files." For a browser: "...the copies already in that browser stay there." Buttons **Cancel** and **Remove**. Action: "Removing..." on the row (`aria-busy`, line 3 "Renewing keys, 1 of 2 workspaces"), the row leaves on acknowledgement; a canvas notice "Laptop was removed." (status, 5s). The row's progress is the criterion 23 count.
- Not connected: refused before writing, `role="alert"` under the row, "Not connected. Try again when connected.", the confirm stays.
- Partly done (criterion 26): the row stays with line 3 "Removal incomplete, 1 of 2 workspaces" and, under the row, `role="alert"` that stays until resolved: "Laptop is still in Box job. Ask an admin of Box job to remove it." The other workspaces are done.
- **Leave my devices** (own row): the same confirm shape: "Leave your devices? This device stops syncing and keeps its copies as local files. Your other devices renew the keys the next time they connect." Buttons **Cancel** and **Leave** (destructive). The list shrinks to the own row, the pairing is forgotten, Join my devices appears again (4.1). On the other devices the header says "Key renewal pending for 2 workspaces" (12px, `role="status"`) until done (criterion 27); no prompt there.
- Lists change when another device pairs or leaves: a canvas notice "Phone was added to your devices." / "Phone left your devices." (5s, polite, device name cut to 16 characters).

### 7.2 On the device that was removed (criteria 24, 25)

Shown once, as an alert block at the **top of the My devices block** and, so it is not missed when the maker is on another tab, as the same alert in the File tab's Workspace section where `0095`'s removed-device block would be (one block, one text, one **OK**; dismissing either dismisses both; never one per workspace): `role="alert"`, "This device was removed from your devices. Your copies stay here as local files." The same words go to the canvas notice slot as an alert. The workspaces become local workspaces (Share tab "Not shared", every switch off), the open document stays open, nothing is deleted, no later changes arrive. After OK the block is the not-paired state (2.4) with Join my devices available. A removal found on reconnect after being offline shows the identical block; the offline edits are not merged and the text does not mention them (the files are intact).

## 8. Collaboration pill, presence and notices

- A synced workspace is a shared workspace, so the pill (`0092`) exists for its documents: other devices appear as discs with the initials of the device name; pointers on the canvas carry the label "Marc (Laptop)" cut to 16 characters. The pill's accessible name uses the Rust text: "Collaboration, workspace Box job, 2 devices, connected" while only own devices are present, else "... 3 people ...". No new colours: participant colours stay derived from the fingerprint, so each of my own devices has its own colour.
- Notices in the canvas slot (status, 5s, at most 70 characters): "Workspace Box job arrived." / "Box job is no longer synced. Your copy stays here." / "Laptop was added to your devices." / "Laptop was removed." / "Laptop left your devices." Alerts (8s): the removed-device sentence of 7.2.

## 9. The browser as a device (DEV-BROWSER B; ships with the browser slice)

Same flows, differences only:

- No Synced workspaces folder step and no Change folder row in the join block; workspaces arrive in the browser's own storage (needs the browser workspace of `0095`'s successor and the key custody ADR). The kind is text: line 3 of the browser's row reads "Browser, ..." on every device; glyph `Globe`.
- **Honest loss text** (a consequence, not a tip), at two places: in the join block on a browser, between the folder-less fields and the buttons, 12px with `TriangleAlert`: "This browser keeps its keys in site data. If you clear site data, this device is lost and has to be paired again." And permanently on the browser's own row as a fourth muted line (12px): "Clearing site data removes this device." When the browser refuses persistent storage (**Proposal**, depends on the ADR): the own row says "This browser may delete this device's data when space runs low." The warning of 2.2 uses "browser" in its noun slot when that is the only device.
- A cleared browser is a new unpaired browser. On the other devices the old row stays as "Last seen ..." until removed; its Remove sentence names "that browser".
- The old-device flow works on a browser too (it can show the code and press Pair) once it has keys.

### 9.1 Passkey backup (Frage 145 A, ADR 0040 decision 13; implementer, 2026-11-04, Proposal)

- **Where:** on a paired browser, one outline button under the device list (after the one-device warning), 176px, `KeyRound` 16px: "Back up with passkey", or "Remove backup" once a backup exists. Not on a desktop. While a pairing block is open it is hidden.
- **Back up:** the button opens an Inline confirm in its place (Apply look, **Cancel** focused, Escape and Cancel return), formatted in Rust with the server: "This device's keys are locked by a passkey and stored on wss://sync.example.org. The server cannot open them. Restoring needs the same passkey and this server. The server sees when the backup is stored or read. Whoever has this passkey can act as this device until you remove it from your devices." Buttons **Cancel** and **Back up**. The browser's passkey dialog follows; under the button a `role="status"` line "Waiting for the passkey..." then "Talking to the server...", then "Keys backed up with a passkey." (status). The own row's loss line "Clearing site data removes this device." becomes "Keys backed up with a passkey. Clearing site data does not lose this device." The loss sentence of the join block stays as it is: at joining, nothing is backed up yet.
- **Without PRF** (the browser or the passkey cannot): an alert line "This browser or passkey cannot lock keys. The keys stay in this browser only." and nothing changes; a page that cannot ask for passkeys at all (no secure context) shows that sentence in place of the button. Closing the passkey dialog: "No passkey was used. Nothing changed." (alert). Server unreachable: "Could not reach the server. Nothing changed."
- **Remove backup:** destructive Inline confirm (400ms guard): "Remove the backup? The server deletes it. Clearing site data then loses this device." Buttons **Cancel** and **Remove**; the same passkey is asked again; "Backup removed from the server." (status) and the loss line returns.
- **Restore** is refused on a browser that is in a group, before anything is written: "This device is paired already. Nothing changed." (alert). **Leave my devices** on a backed-up browser asks the passkey once more and deletes the backup.
- **Restore:** on a browser without a group, a second row under Add a device / Join my devices: outline "Restore with passkey" (176px, `KeyRound`), disabled with the same reason as Join (no server, no `wss://`, another tab). It asks for any passkey of this site and reads from the server address in the Share tab. Success: "This device was restored from its passkey backup." (status); the list shows the group's devices with this device's old name and fingerprint, and synced workspaces arrive as after a pairing. "This passkey has no backup on this server." when the server has none; "The backup on the server does not open with this passkey." for a damaged record.

## 10. Keyboard and focus

| Action | Key |
|---|---|
| Move through the block | Tab in screen order; rows have two stops (details, action) |
| Expand details; open Remove or Leave | Enter or Space on the details button; Enter or Space on the action button |
| Rename the own device | F2 on the own row (Enter commits, Escape restores) |
| Submit the code | Enter in the code field |
| Pair, Sync, Remove, Leave | Enter or Space on the button; none of them is the default button of a screen |
| Cancel a pairing, close a confirm, Not now | Escape |
| Toggle the sync switch | Space or Enter (opens the confirm; the switch itself moves only after it) |

- Tab order of the tab: Server and Name, **My devices** (first row details, its action, ... then the warning line is not a stop, then decision-row buttons, then Add a device and Join my devices or the open block's controls), **Workspace block** (switch row, then `0095` order), Document block, the two open and join buttons.
- Focus rules: opening a block puts focus on its first element: the code group (3.1), the compare group, the first check box, the code field (4.2). A confirm opens with focus on Cancel. Closing or cancelling returns focus to the control that opened it; after a successful Remove focus goes to the next row's details button, else the previous, else Add a device; after pairing completes on the new device focus goes to the first list row. The switch keeps focus after a turn-on or turn-off.
- Check words and the code are never colour-only, selectable text, not in an image. Words are read once as part of the group name; they do not change while shown.
- Hit targets 28px (36 coarse); check box rows 32px (36 coarse) and a whole-row label; detail lines are not interactive.
- Live regions: pairing phases, sync states, counts: polite; refusals, lost connection, locked or expired, removed device: assertive (`role="alert"`) once each. The countdown is not announced per second. Remote and own state changes are throttled to one per second.
- Reduced motion: none exists beyond the `RefreshCw` glyph, which is static.
- User text (device names, workspace names) is plain text, cut at 24 characters in the lists and with the middle ellipsis in sentences; the full name is in the tooltip and the accessible name.

## 11. Tokens and components

Reused unchanged: `--toolbar-icon`, `--toolbar-icon-active-bg/-fg`, `--editor-accent`, `--editor-accent-hover`, `--panel-muted-fg`, `--field-invalid`, `--destructive`, `--peer-1` to `--peer-6`, the participant row look, Inline confirm (two buttons), `EntryField`, `Switch`, `Tooltip`, canvas notice slot, outline and Apply text buttons, `useRowsFocus` not needed (at most ten rows). New (to enter into `docs/design-system.md` when this ships, no new token): **device row** (56px, three lines, details disclosure and a 28px action), **pairing block** with the **code display** and the **check-words grid**, **workspace pick list** (check box rows), **decision row**, **sync switch row** (switch row of the Workspace block).

## 12. Review checklist

Not paired, no server; server set and unreachable; Add a device: code shown, countdown, expiry, lock after wrong tries, Cancel and Escape, New code; Join my devices: lowercase and spaced and pasted code, wrong (tries left), used, expired, locked, server lost, A closed while B waits, A cancels, Words differ on either side; check words identical on both screens and readable by a screen reader; no content visible on the new device before Pair; the default folder created, a folder inside a workspace refused; choosing workspaces: all checked, a member-only workspace unchecked and disabled, already-synced line, none checked (Sync these disabled), Not now; the arrival notice, name collision, a refused file name; third device paired through the second one; the switch: off with each reason, turn-on confirm (with takeover lines, one-device sentence), Cancel leaves it off, upload progress, failure, turn-off confirm naming the devices, offline refusal; sync lines: syncing, offline with a count, waiting for first copy; a device removed while online and while offline (alert block once, File tab and Share tab, OK dismisses both); Remove with a device that is offline, with an incomplete removal, with a workspace in which this device is not admin; Leave and "Key renewal pending" on the other device; only-one-device warning appearing and going away; decision row (Connect, second copy); keyboard-only run of every step; 36px targets; 800 x 600; a 20-document workspace of 1,000 objects arriving while the tab is open (no jank, lines throttled); browser variant (honest text in both places) when it ships.

## 13. Open UX questions (each with a default)

- **DEV-NOUN:** "device" in the UI (this note) / "computer" (the spec's wording). Default: device, one Rust parameter. **(Proposal)**
- **DEV-NAME:** the device name (own row) is separate from "Name shown to others" (the person), other makers see "Marc (Laptop)". Default: separate, because criterion 16 needs both. **(Proposal)**
- **DEV-WARN:** the one-device warning is a persistent line while the risk exists plus the sentence in the first confirm (this note) / a one-time notice (criterion 28 as written). Default: persistent, since DEV-LOST A says to warn while only one device is paired.
- **DEV-JOIN-AGAIN:** Join my devices hidden once this device has a pairing or a synced workspace (this note) / allowed with a "leaves your current devices" confirm. Default: hidden.
- **DEV-STOP-ANY:** the off switch on any device stops sync for all devices of that workspace (this note) / only the device that turned it on. Default: for all, the confirm names them.
- **DEV-BROWSER-SLICE:** the browser texts of section 9 are specified now but ship with the browser slice (the spec's slice 4; needs a browser workspace and the key custody ADR). Default: as the spec.

## 14. Amendment: armed actions, the switch and Try again (customer feedback on PR #281, 2026-11-09)

- **Armed instead of confirms.** Remove (7), Leave my devices (7.1), Remove backup (9.1) and turning the switch off (6.3) are the **armed action** of `docs/design-system.md` (customer feedback on PR #281, 2026-11-09): the first press arms the button (filled `--destructive`, white glyph; Make admin keeps the Apply look) and shows the sentence as a polite status line under the row, a press within 400 ms is ignored, the second press acts; it disarms after 4 s, on Escape, on a press elsewhere or when the focus leaves the row. No Cancel button and no confirm group; a refusal shows as `role="alert"` under the row and the button disarms. For the switch the switch itself is the armed control: the first press keeps it on with a `--destructive` track and shows the 6.3 sentence under it, the second press turns it off. Turning it on keeps its Apply inline confirm (nothing is lost).
- **Leave my devices on the last device** is allowed (customer 2026-11-09, ADR 0040 decision 8): the sentence reads "Leave your devices? This is your last device: your devices end here. Synced workspaces stay on this device as local files." and, for each synced workspace whose room keeps other people but no admin afterwards, "Box job stays here as local files; its other participants keep it, but nobody there can add or remove people any more." The refusal "This device is the only one that can manage your devices. Pair another device first." is gone.
- **The switch is always operable.** While a switch-on runs (line "Sharing..." or "Uploading 3 of 14", the block header "Sharing", never "Not shared"), a press turns it back at once without asking: the Share stops, a room it made is left, nothing is written and the document comes back. A switch-off that has not gone out turns back the same way. A switch or a Share that makes no progress for 30 s fails: `role="alert"` "Could not share: the server did not answer in time." (or the Share's own reason, or "Could not sync: cannot reach the server.") with a 28px **Try again** icon button (`RefreshCw`, tooltip and name "Try again") beside it, which switches on again without the confirm.
- **Try again on the Server field.** While the reachability line reads "Cannot reach <server>.", the same **Try again** button stands at its right: it checks the server again and reconnects every waiting link (the open document or workspace, the device group, the synced workspaces in the background) at once.

## 15. Amendment: no sentences (customer feedback on PR #281, 2026-11-09)

- The armed Remove, Leave my devices, Remove backup and switch-off of section 14 show **no sentence**. Switching on asks nothing (no Apply confirm); Back up with passkey asks nothing beyond the passkey prompt.
- Gone as hints: the pairing sentence (3), the privacy and loss sentences of the workspace choice and the browser join (3.1, 9), the one-device warning (2.2) and the browser's loss lines on the own row (9). The storage line, states and real errors stay; the frozen-room warning stays with the workspace choice.
