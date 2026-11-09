# 0097 Membership lifecycle: UX notes

Status: Draft for Ready, aligned with ADR 0041 (slice 1; slice 2 in outline, section 11). Owner: ux-engineer, 2026-11-05. Authority: `specification.md` (visible criteria: 1 to 5, 7 to 14, 16 to 21, 23, 25 to 30, 35; slice 2: 37 to 41), customer decisions of 2026-11-05: two roles; any admin promotes and demotes anyone with a confirm; second-admin nudge; the last admin cannot leave until a successor is admin; Leave states what the leaver keeps (documents and workspaces); add back: "Full history" (preselected) / "From now on" for documents, no choice for workspaces; the verified join reuses the `0096` code and four check words and complements the invite link; roles per device, leave of a synced workspace per person; one in-app line and a mark on the Share tab icon for admin-set changes; a removed person presses "Check whether I was added back" (no background traffic); the notice for two admins who removed each other, with the warning when an admin is added later. References: `docs/design-system.md` ("Properties panel: Share tab and collaboration", "Inline confirm", "Canvas notice slot", "Interaction conventions"), `specs/0096-own-devices-sync/ux-notes.md`, `specs/0095-workspace/ux-notes.md`, `specs/0092-cooperative-editing/ux-notes.md` (2.4 participants, Remove, removed device), `ShareTab.tsx` (`ParticipantRow`), `WorkspaceShareBlock.tsx`, `MyDevicesBlock.tsx`. **(Proposal)** marks reversible choices. Customer rule 2026-10-16: no operating tips or hint labels; labels that name a control or a state stay, and so do sentences that state a consequence (what the leaver or the removed person keeps, what a choice withholds). Texts with names, state or numbers are formatted in Rust (`curvyo-ui-core`); the wording test of criterion 34 runs over them ("revoked", "lost access" never appear).

**Noun (Proposal):** "device", as `0096` DEV-NOUN (the spec says "computer"); one Rust parameter. User text (labels, document and workspace names) is cut at 24 characters in lists and with a middle ellipsis in sentences; the full text is in the tooltip and the accessible name. A label that is empty reads "A device" (criterion 9).

## 1. Where it lives

- **Share tab only.** No new tab, dialog, banner or Settings page. Everything is an Inline confirm or a block in place. For a document inside a workspace the tab shows "Shared through workspace Box job" and none of the blocks below (criterion 35); the Workspace block of the File tab and the Share tab show the same participant list for a workspace (one component, `0095`).
- Order of the Share tab when shared, top to bottom: header (subject line, then the **admin-change line**, 2.4) / Server and Name rows (`0092`, unchanged) / **Notices block** (section 4; absent when none) / Share status / Invite (admins) / **Participants** / **Removed or left** (admins, collapsed) / the second-admin line sits at the top of Participants (section 3). Not shared and removed or left: the **Removed block** (section 7) sits under the settings and above Share. Nothing of `0092`, `0095`, `0096` moves.
- The My devices block (`0096`) stays the one place that removes a device of mine. In the participant list my own devices keep "my device" and no Remove (`0096` 6.1); their role actions exist but Leave is per person (section 5.3).

## 2. The Share tab mark and the admin-change line (criterion 30, Frage 156 A)

1. **Mark.** An 8px dot, `--editor-accent`, at the top right of the `Users` icon in the tab strip, with a 2px ring in the strip background. Shown while an unread notice exists; the tab's accessible name becomes "Share, new notice" (the dot is `aria-hidden`; text carries it). It clears when the Share tab body has been visible for 1 s, never earlier, never by hovering. Switching to the tab by the collaboration pill counts. Notices caused by this device's own actions never set the mark.
2. **What sets it:** an admin-set change by someone else (promoted, demoted, left, removed an admin), a "keys not changed yet" notice for an admin, and a mutual-removal notice. A plain member join or removal sets nothing.
3. **Admin-change line.** Directly under the header subject line, one line, 12px `--toolbar-icon`, `role="status"`, polite, with the time muted at the right ("12:03"): "Ben is now an admin" / "Ben is no longer an admin" / "Ben left" / "Ben was removed" (admins only). One line only, the newest replaces the older one. It stays until the tab has been shown for 1 s and a later change arrives, or the document closes; it is not a list (the activity list is slice 3). Several changes at once: "Ben is now an admin" plus the muted "and 2 more changes" in the same line.
4. **Canvas notice (5 s, status, at most 70 characters)** only for changes to me: "You are now an admin." / "You are no longer an admin." / "Keys changed after Ben left." No OS notification, nothing leaves the app (R-SYS-007).
5. Nothing here is a toast that steals focus; no notice moves focus.

## 3. Participants list

`ul` named "Participants", `tabindex="-1"` (the focus anchor of section 9). The list shows every participant: roles are visible to everybody (criterion 1).

### 3.1 Row (56px, 60 coarse)

Left to right: 24px disc (as `0092`), a text column, a **role slot** 28px and an **action slot** 28px (both reserved on every row so text does not shift).

- Line 1, 14px `--toolbar-icon`, ellipsis: the label; own row followed by "this device"; my other devices by "my device".
- Line 2, 12px mono muted: the fingerprint (as `0092`).
- Line 3, 12px: the **role word** "Admin" in `--toolbar-icon` or "Member" in `--panel-muted-fg`, then ", offline" (muted) when the participant has no presence. This replaces the right-aligned "offline" of `0092` 2.4 (the right end now holds two slots; design system to be updated). The row's disc stays at 55% when offline. Role is text, never colour or an icon alone.
- **Role slot:** for an admin on every other row either **Make admin** (`ShieldPlus` 16px, outline without border, tooltip "Make admin", name "Make Ben an admin") on a member or **Make member** (`ShieldMinus`, name "Make Ben a member") on an admin. On the own row of an admin: **Step down** (`ShieldMinus`, name "Step down as admin"). Empty for members.
- **Action slot:** admin, other row: **Remove** (`UserMinus`, as `0092`). Every participant, own row: **Leave** (`LogOut`, tooltip "Leave", name "Leave Poster" / "Leave Box job"). Empty otherwise.
- All are Tab stops in this order: role slot, action slot, then the next row. Members therefore have exactly one stop in the whole list (their Leave). Hit targets 28px (36 coarse).
- **In flight.** After a confirm the row shows `aria-busy` and line 3 reads "Making admin..." / "Making member..." / "Removing..." / "Leaving..." until the keyring entry is acknowledged; nothing is modal and the canvas stays live. Role changes of others arrive within 2 s and update the row in place, no animation.

### 3.2 Make admin (criterion 3)

Inline confirm under the row (Apply look, not destructive; nothing is lost; role-group named by the sentence; **Cancel** focused), 12px, Rust: "Make Ben an admin? Ben can invite people, remove people, change roles and change keys. You can make Ben a member again." When another row carries the same label (self-asserted, so information only, Frage 155 A) add: "Ben's other device, Phone, keeps its role." Buttons **Cancel** and **Make admin**.

**While the room is frozen (held-back removals stand, criterion 28; ADR 0041 decision 4):** the sentence is replaced by the warning, inside the same confirm and not a second one (MEM-WARN-ONE, decided). Mutual removal: "Ana and Chris removed each other earlier. If you make another admin, both removals apply and both leave." One earlier removal or step down that was refused because it would have left no admin: "Ana's earlier removal was not applied because nobody else was an admin. If you make another admin, it applies and Ana leaves." The action reads **Make admin anyway** in the destructive look (400 ms guard), Cancel focused. The same warning sentence is appended in the `0096` confirm that pairs a new own device into a frozen workspace. Switching "Sync to my devices" on needs no warning: `reconcile` adds those devices as members there, so a frozen room is never released by an automatic write; they become admins only through Make admin.

### 3.3 Make member and Step down (criteria 4, 5)

- **Make member** (another admin): confirm, destructive look (400 ms guard), Cancel focused: "Make Ben a member? Ben can no longer invite, remove or change roles; Ben keeps editing." Buttons **Cancel** and **Make member**. Any admin may do this to any admin (customer decision); deny wins when two admins act at once and the list simply shows the end state.
- **Step down** (own row): "Step down as admin? You keep editing. You can no longer invite, remove or change roles." Buttons **Cancel** and **Step down** (destructive look). The row loses its admin actions in place; the canvas notice "You are no longer an admin." (status).
- **Only admin:** Step down (whenever this device is the room's only admin) and Leave (the only admin among several participants) are greyed: `aria-disabled` (40%, Tab stop kept), the reason "You are the only admin" as tooltip and `aria-description`. A press does nothing: no alert, no armed state (customer 2026-11-09; the sentence "You are the only admin. Make someone else an admin first." is gone).
- Pressing Make member or Remove on an admin where the client knows replay would drop it (the stale-removal case, section 6) opens the confirm as usual; the result is the dropped line of section 6, not an error.

### 3.4 Remove (unchanged from `0092` 2.4 and `0095` 8, with one addition)

Sentence, destructive look, Cancel focused, 400 ms guard: as shipped. For an admin target the sentence starts "Remove Ben, an admin?" and nothing else changes. Offline: refused before writing, `role="alert"` under the row "Not connected. Try again when connected.", confirm stays.

### 3.5 The second-admin line (criteria 7, 8; DEV-LOST A pattern of `0096` 2.2)

A persistent state line at the top of Participants, `TriangleAlert` 16px and 12px text, `role="status"`, with a 28px **Dismiss** (`X`, name "Dismiss") at the right. Two tiers, one line each, shown exactly while the condition holds:

| Condition | Text |
|---|---|
| Exactly one admin device | "Only this device is an admin. If you lose it, nobody can add or remove people. Make someone else an admin." |
| Admins exist but all are my own devices | "All admins are your own devices. If you lose them all, nobody can add or remove people. Make another person an admin." |

Admin devices of the own group are counted by the share dialog (criterion 8). Seen by admins only (a member cannot act on it). The buttons are the role slots of the member rows, not a button in the line; focus does not move to the line. **Dismiss** hides the line for this document or workspace on this device only (kept in the local app settings, never sent); a dismissal of tier 1 does not hide tier 2 **(Proposal)**. Not a block: nothing is disabled and no sharing step waits for it. It is a state line, not an operating tip.

## 4. Notices block (criteria 11, 12, 27, 28, 29)

Under Server and Name, above Invite; absent when there is nothing to say. One notice per kind, each a `role="group"` with a 12px sentence and at most one button, stacked with 8px gaps. State-derived notices (they come from the keyring, so they survive restart); the first two live until their cause is gone.

| Notice | Text | Button | Role |
|---|---|---|---|
| Rotation pending, admin | "Ben left. The keys have not been changed yet." (several: "Ben and 2 others left. ...") | **Change keys now** (Apply look, 140 x 28) | `status` |
| Rotation pending, member | "Ben left. The keys have not been changed yet. Waiting for an admin to change the keys." | none | `status` |
| Mutual removal (criterion 27, 29) | "Ana and Chris removed each other at the same time. Nobody was removed." For the two involved: "You and Chris removed each other at the same time. Nobody was removed." | **OK** (outline) | `status`, then stays |
| Removal dropped (criterion 28) | "Removal of Chris was not applied: you are the only admins." | none | `status`, second line inside the mutual-removal notice |

- **Change keys now** (criterion 12): no confirm (nothing is lost, it is the safe act). Busy: label "Changing keys..." (`aria-busy`, inert). Done: the notice leaves for everybody, canvas notice "Keys changed." (status). Offline: refused first, `role="alert"` under the notice, "Not connected. Try again when connected.", the notice and button stay. A `reconcile` that already rotated shows only the canvas notice "Keys changed after Ben left." and no notice block.
- **Mutual removal** stays until **OK** on this device (local, per document or workspace); OK hides it here only, the dropped-removal line goes with it. After OK, the warning of 3.2 still appears, because it comes from the keyring state, not from the notice. The notice sets the tab mark until read, like every notice. In a workspace the same notice appears in the Workspace block and the participants stay members (criterion 29).
- A user who is not an admin sees the mutual-removal notice too (every participant, criterion 27).

## 5. Leave (criteria 10 to 16, 25)

### 5.1 The confirm (every participant, own row)

Inline confirm in place under the own row, destructive look on the action (400 ms guard), **Cancel** focused. 12px, Rust, says what leaves and what stays (Frage 150 default):

- Document: "Leave Poster? Syncing stops on this device and the copy stays here. Until an admin changes the keys, this device still holds the key to what was shared so far."
- Workspace: "Leave Box job? The 17 documents stay in this folder as ordinary files and syncing stops here. Until an admin changes the keys, this device still holds the key to what was shared so far."
- Workspace that is synced to my other devices (criterion 16, per person): add "Box job is synced to your other devices. Leaving removes all of them: Laptop, Phone." (names cut at 16, "and 2 more" after three).
- Where the leaver is an admin and others are admins: nothing added; where it is the only admin: no confirm opens (3.3).

Buttons **Cancel** and **Leave**. Escape and Cancel close the confirm and return focus to the row's Leave button. Offline: refused before writing, `role="alert"` under the row "Not connected. Try again when connected." and the confirm stays. Nothing in the flow says "access lost" or "revoked".

### 5.2 After leaving, on the leaver

The row shows "Leaving..." then the tab becomes the **Removed block** in its "left" state (section 7). A `role="status"` canvas notice "You left Poster. Your copy stays on this device." (5 s). The pill goes away, the document stays open as an ordinary local file (not shared); a workspace becomes a local workspace, its marker share files are removed, no document is deleted anywhere (criterion 15). Focus goes to the block's heading line (7.2).

### 5.3 On the others (criterion 11)

The row leaves the list within 2 s, the admin-change line says "Ben left", the Notices block shows "rotation pending" (section 4). When Ben was the last other admin and the viewer is an admin, the second-admin line (3.5) appears again. In a workspace synced to Ben's other devices those devices leave one by one as their own self-removals; the rows go within the same few seconds, the line says "Ben left" once (device rows of one leave arriving together are one line, **Proposal**).

### 5.4 Person leave in a synced workspace: the `left` mark (criterion 16, ADR 0041 decision 3)

A workspace that is synced to my own devices is left per person. Leaving writes a `left` mark that all my devices read, so every device of the group leaves, one entry each.

- **My devices block** (`MyDevicesBlock`): the workspace's "Sync to my devices" switch shows off, with a 12px state line under it, `role="status"`: "You left Box job." While the mark is being written and acknowledged: "Leaving..." (`aria-busy`, switch inert). Nothing else on the row changes; the mark is not a separate control.
- **Clearing is a person's act, two ways:** **Sync again** in the add-back prompt (7.3) and the switch turned on. Both rewrite the entry without the mark and wait for the server's acknowledgement before connecting; the busy label is "Switching on..." or "Syncing...". The state line then goes. No confirm.
- **On my other devices** the workspace shows the Removed block in its "left" state ("You left Box job", 7.1) as soon as they read the mark, whichever way the removal reached them; a device that is offline shows it after its next start. Until then it keeps the workspace on and syncing.
- Switching the workspace off without leaving stays a plain switch-off: no mark, no state line.

## 6. Mutual removal, the consequence

While the two admins' stale removals stand (criterion 28) the list shows both as admins and the notice of section 4. A further Remove or Make member between them ends in the dropped line of section 4 (not an alert, not red). Adding a third admin first shows the warning of 3.2. This is the only place the keyring's replay rule becomes visible; no tooltip or help text explains it further.

## 7. Removed and left, on the person who is out

### 7.1 States of the Share tab (criteria 13, 20, 21, 25, 26, Frage 158 B)

A file that was removed from or left a share keeps its room reference (`share.ended.json`; nothing here depends on it being visible) and shows, under Server and Name and above Share (not shared state otherwise as `0092` 2.2), the **Removed block**:

1. Heading line, 14px: "You were removed from Poster" / "You left Poster". For a workspace: "... from Box job". Persistent while the reference exists. The first time after a removal, the `0092` alert ("You were removed from this document. Your copy stays on this device.", `role="alert"`, canvas notice as alert, File tab's Workspace section for workspaces, `0095` 8) comes with an **OK** that only dismisses the alert text; the block stays.
2. **Check whether I was added back** (244 x 28 outline, `UserCheck` 16px). Under it, 12px muted, one line: "Contacts 192.168.1.20:8080." (what leaves the machine when it is pressed; shown always while the button exists). No server in settings and the remembered server unreachable are handled as below. Nothing is sent before the button is pressed; no timer, no check at start or on opening (criterion 33).
3. The Share and Open shared document buttons below stay available; they start something new and say so as before.

**The check:**

| State | What the maker sees |
|---|---|
| Pressed | Button label "Checking..." (`aria-busy`, inert); focus stays on it |
| Not a participant, or the server denies the connection | `role="status"` under the button, "Not added back yet." Button back, focus on it. A denial and "not yet" read the same: the server can refuse a return but cannot fake one |
| Cannot reach the server | `role="alert"`, "Cannot reach the server at 192.168.1.20:8080." Button back; the Removed block is unchanged |
| The answer does not verify | `role="alert"`, "The server's answer could not be checked. Nothing was changed." Button back (the app decides from the signed entries itself; an answer that does not prove an add back is never shown as one) |
| Room not found or entry not readable | "Nothing to find here. This file stays local." and the Removed block ends (Proposal; the reference is dropped) |
| Added back (the app's own check of the entries says so) | The **add-back prompt** (7.3) replaces the button in place |

### 7.2 Focus rules of the block

On arrival of the block after Leave: focus on the heading line (`tabindex="-1"`, so a screen reader reads "You left Poster"). After Check without a result: back on Check. After OK on the alert: focus to the heading line. After the prompt resolves: see 7.3. The block never takes focus when the tab is only shown, and the check result never steals it from another control.

### 7.3 Add-back prompt (criteria 20, 21, 26)

`role="group"`, in place of the Check button, 12px: "Ana added you back to Poster. Sync again?" For a workspace: "Ana added you back to Box job. Sync again?" Under it the merge sentence: "Changes you made while you were away will be merged in." (both documents and workspaces; for a workspace with a kept folder it is the same: the `0096` "Connect this folder" decision is this prompt). Under it the consequence of the other button: "Keep as it is leaves Poster again. The admins will see that you left." (Frage 162, default A of ADR 0041.) Buttons **Sync again** (Apply look, 100 x 28) and **Keep as it is** (outline). Focus goes to the group (named by the first sentence), never to a button, so Enter cannot sync by reflex. Escape does nothing (neither answer is the safe default). Nothing syncs before the maker chooses (criterion 25).

- **Sync again:** buttons gone, `role="status"` "Syncing..." (`aria-busy`), then the tab becomes the shared state with the pill, "Shared, 3 people" in the header; the document keeps its edits. No success notice (the shared state is the result). Failure: `role="alert"` "Could not sync: cannot reach the server." and the prompt returns, nothing was written; focus to the group. After success focus goes to the Participants list (`tabindex="-1"`).
- **Keep as it is** (Frage 162 default A): the device leaves again as in 5.2: "Leaving..." (`aria-busy`), then the Removed block in its "left" state with Check available, focus on the heading line. Offline: refused first, `role="alert"` "Not connected. Try again when connected.", the prompt stays. If the customer answers B (nothing is sent), the block leaves instead and the tab is the plain not-shared state with focus on the Share button. A workspace marked `left` stays marked.

### 7.4 The person was added back while already syncing

Not possible: a removed or left file does not connect. A person with the app closed sees the Removed block at the next start, the same as now.

## 8. Adding back, on the admin (criteria 17 to 19, 23, 24)

### 8.1 Removed or left section (admins only)

Under Participants, a disclosure button **Removed or left (2)** (full width, 28px, `ChevronRight`/`ChevronDown`, `aria-expanded`, collapsed by default), not in the tree for members and when the count is zero. Expanded, a `ul` named "Removed or left" with 56px rows (60 coarse): glyph-free; line 1 the last known label or "A device" (the label is local to this device and may be missing), line 2 the fingerprint (mono), line 3 muted "Removed" or "Left" when the keyring tells, else nothing **(Proposal)**; one 28px action: **Add back** (`UserPlus`, tooltip "Add back", name "Add back Ben") and nothing else, no Remove, no role actions. Other admins' removed rows are visible too. Empty: the section is absent, no empty text.

### 8.2 The dialog

Inline confirm under the row (Apply look, nothing is lost), Rust text. **Document:** a `role="radiogroup"` "History" (the `PanelRadioGroup` look, two items, **Full history preselected**, customer decision) with a consequence line under each, 12px:

- **Full history** "Ben gets the keys of every key period you hold, so the document's history comes along."
- **From now on** "The app does not give Ben the keys of the key periods that began while Ben was away. Someone who already holds them could pass them on outside the app. Ben still holds the keys Ben had before." (ADR 0041 decision 7: a rule of this app, not a lock; the text never says "cannot read".)

Then one sentence for both: "Ben comes back as a member and keeps what Ben had. Nothing else changes for anybody." (criterion 18; no extra snapshot option, Frage 152 is obsolete). Buttons **Cancel** and **Add back** (Apply look). Focus on the selected radio; arrow keys change the choice, Enter on a radio does not confirm. Escape and Cancel close and return focus to the row's Add back.

**Workspace (customer decision: no choice):** no radio group. One sentence: "Add Ben back to Box job? Ben gets every key you hold and the current state of all 17 documents. Ben comes back as a member and keeps what Ben had." Buttons **Cancel** and **Add back**. Focus on Cancel.

**Never added silently:** the dialog says nothing about reaching Ben, because Ben decides (criterion 25); a leaver is not worded differently from a removed person. Offline: refused before writing, `role="alert"` under the row "Not connected. Try again when connected." and the dialog stays.

### 8.3 After the add back

The row shows "Adding back..." (`aria-busy`, inert), then leaves the section and appears in Participants as Member (within 2 s); canvas notice "Ben was added back." (status). Focus: the next row's Add back, else the previous one, else the Participants list (`tabindex="-1"`) when the section is gone. If Ben's other devices stay in "Removed or left" as their own rows (criterion 24), they are separate rows and nothing links them. Make admin for Ben is a separate action in the list (3.2).

## 9. Keyboard and focus

| Action | Key |
|---|---|
| Move through the list | Tab in screen order: per row role slot then action slot; no roving; one stop per member for their Leave |
| Open a confirm or dialog | Enter or Space on the button |
| Close a confirm, Cancel | Escape; focus returns to the button that opened it |
| Choose History | Arrow keys on the radio group; Enter does not confirm |
| Expand Removed or left | Enter or Space on the disclosure |
| Change keys now, Check, Sync again, Add back, OK | Enter or Space; none is a default button |

Focus rules (the `0096` review found returns missing; every confirm here has all four exits):

| Confirm | Opens on | Cancel or Escape | Success | Failure or refusal |
|---|---|---|---|---|
| Make admin, Make member | Cancel | the row's role button | the same role slot (its button now shows the opposite action, same position); announced by the row's line 3 | confirm stays, focus on Cancel; alert under the row |
| Step down | Cancel | Step down button | the same role slot (now empty): focus to the row's Leave button | as above |
| Remove | Cancel | the row's Remove | next row's role slot, else the previous row's, else the Participants list | as above |
| Leave | Cancel | the row's Leave | the Removed block heading (7.2) | as above |
| Add back (document) | the selected radio | the row's Add back | next row's Add back, else previous, else the Participants list | as above |
| Add back (workspace) | Cancel | the row's Add back | as above | as above |
| Change keys now | no confirm; stays on the button | n/a | the Participants list | stays on the button; alert under the notice |
| Check, Sync again, Keep as it is | the group | n/a | Sync again: Participants list; Keep: Share button; no result: Check | as 7.1 and 7.3 |

- Disabled controls keep their Tab stop (`aria-disabled`, reason announced). Alerts are `role="alert"`, once; progress and results are polite status lines; remote changes are throttled to one announcement per second; the row's line 3 carries the busy state. Counts and the admin-change line do not announce per second.
- A tab strip mark and a notice never move focus. When a row disappears while it holds focus (someone else removed it), focus goes to the Participants list, never to `body`.

## 10. States: empty, error, offline, large

- **Only me:** the list is the own row with Leave not offered (a sole participant has no Leave; Step down not offered; nothing is disabled and nothing explains why, criterion 14). The Removed or left section is absent.
- **Offline (this device):** every membership action is refused before writing with "Not connected. Try again when connected." under the row or notice; confirms stay open for the retry; the list is the last known state and the header shows the connection text of `0092` 6. Role changes, leave and add back never queue offline.
- **Server error mid-action:** the row's busy state ends, `role="alert"` "Could not finish: cannot reach the server. Nothing was changed." (Proposal: "Nothing was changed" only where the entry was not written; otherwise "Waiting to be confirmed" with the row staying busy).
- **Conflicts (criterion 6):** a concurrent removal beats a promotion, the row leaves, no error. A concurrent demotion beats a promotion, the row shows Member. No notice for either except the admin-change lines.
- **Large rooms and documents:** actions never block the canvas or edits; lists with 20 or more rows scroll with the tab (no virtualisation, no roving focus) and Removed or left is collapsed by default. The notices are derived from the keyring, so a 1,000-object document or a 20-document workspace does not change their cost.
- **Narrow panel and coarse pointer:** rows 60px, targets 36px; at 244px the label ellipsises first, the role word and action slots never.

## 11. Slice 2: verified join (outline, criteria 37 to 41; direction confirmed by ADR 0041 decision 8, text final when Frage 153 and 154 are answered)

Customer decisions: reuse the `0096` code and four check words; the link stays, complementing it.

- **Invite block** gains a first button **Add by comparing words** (118 x 28, `UserCheck`) before **Invite** (link); no sentence explaining the difference (customer rule). The admin side follows `0096` 3: the admin's device shows the code in two groups of four and "Valid for 4:52", the invitee types it in a **Join with a code** block (next to Open shared document..., the fields of `0096` 4.2 with the server and the name), both screens then show the four **check words** in the 2 x 2 grid, label "Check words", and the sentence "They must match on Ben's device." / "They must match on Ana's device." The admin sees the invitee's name and fingerprint above the words.
- The admin's buttons are **Add** (Apply) and **Words differ** (outline), focus on the compare group, never on a button; Escape equals Words differ. For documents the History radio group of 8.2 sits in the same screen (preselected Full history); for workspaces there is none. All error texts and the countdown are the `0096` 3 and 4.5 ones, with "Add" for "Pair".
- A maker's new device in a workspace where the maker is only a member (criterion 41): the device appears to admins as a request row in the Notices block ("Marc's new device wants to join Box job" with **Compare**), which opens the same compare screen; it never appears as a participant before an admin adds it.
- Focus: opening puts focus on the code group (admin) or the code field (invitee); resolving returns focus to the **Add by comparing words** button; after Add focus goes to the Participants list.

## 12. Tokens and components

No new token. Reused: `--toolbar-icon`, `--panel-muted-fg`, `--editor-accent` (the tab mark), `--field-invalid`, `--destructive`, `--peer-1` to `--peer-6`, Inline confirm (two buttons), `PanelRadioGroup`, `EntryField`, outline and Apply text buttons, canvas notice slot, participant row. New (to enter into `docs/design-system.md` when this ships): **participant row with role line and two action slots** (replaces the right-aligned "offline"), **role words** (Admin / Member), **Share tab mark**, **admin-change line**, **notice group** (sentence plus one button), **state line with Dismiss** (the second-admin line), **Removed or left row and disclosure**, **Removed block** and **add-back prompt**.

## 13. Review checklist

Participants as admin, member, own row, my other device, unknown label ("A device"); Make admin confirm with and without a same-label device; Make member of an admin; Step down and Leave as the only admin (reason alert, focus on a member's role slot); sole participant (no Leave); second-admin line in both tiers, Dismiss, reappearing per document only; Leave of a document, of a workspace (document count), of a workspace synced to my other devices; Leave offline; rotation pending as admin (Change keys now, busy, done, offline) and as member; reconcile already rotated; mutual removal notice (involved, not involved, OK, dropped-removal line, warning when adding an admin, `0096` confirms with the warning); concurrent removal vs promotion; person leave of a synced workspace (state line on the switch, `left` cleared by Sync again and by the switch, my other devices show "left"); frozen room: Make admin warns inside the one confirm, mutual and single-refusal wording; check with a denial, an unreachable server and an answer that does not verify; Removed or left (collapsed, expanded, missing label, Removed vs Left); add back of a document (radio preselected, both choices) and of a workspace (no choice); add back offline; removed device: alert once, Removed block, Check (not added, unreachable, added back); Sync again and Keep as it is; the check sends nothing until pressed (no timer); tab mark appears for an admin-set change and clears after 1 s; nothing for own actions; focus return table, every row; screen reader names of all icon buttons; 36px targets; 800 x 600; 244px label ellipsis; light and dark; a 20-document workspace of 1,000 objects while notices change; no text says "revoked", "lost access" or "can no longer read" without what stays.

## 14. Open UX questions (each with a default)

- **MEM-ICONS:** decided (default): per-row icon buttons, no overflow menu; the design system has no menu component and every action keeps a visible Tab stop.
- **MEM-WARN-ONE:** decided (default): the frozen-room warning sits inside the one Make admin confirm, destructive look.
- **MEM-NUDGE-DISMISS:** decided (default): both second-admin tiers can be dismissed, per tier, per document on this device.
- **MEM-NOUN:** "device" in all texts (this note) / "computer" (spec). Default: device.
- **MEM-HISTORY-LINE:** closed: Frage 152 is obsolete; the dialog says "keeps what it had" and, for Full history, that the history comes along (8.2).
- **MEM-VERIFIED-DIRECTION:** decided: the admin's device shows the code, the invitee types it, as `0096`; ADR 0041 decision 8 confirms. The spec outline (criterion 37) is to follow.
- **MEM-KEEP (Frage 162):** "Keep as it is" leaves again (ADR 0041 default A, 7.3); answer B removes the second leave entry and ends the block instead.
- **MEM-LINE-VERIFIED:** the spec's line "Verified: you compare a fingerprint, no link can be forwarded" is dropped (customer rule on hints); the button label "Add by comparing words" names the control. Default: dropped.

## 12. Amendment: armed actions (customer feedback on PR #281, 2026-11-09)

Every act of sections 3.2 to 3.4 and 5.1 (Make admin, Make member, Step down, Remove, Leave) is the **armed action** of `docs/design-system.md` (customer feedback on PR #281, 2026-11-09): the first press arms the button (filled `--destructive`, white glyph; Make admin keeps the Apply look) and shows the sentence as a polite status line under the row, a press within 400 ms is ignored, the second press acts; it disarms after 4 s, on Escape, on a press elsewhere or when the focus leaves the row. No Cancel button and no confirm group; a refusal shows as `role="alert"` under the row and the button disarms. The sentences of those sections stay word for word; they are the status line of the armed button instead of the text of an inline confirm, and "Buttons **Cancel** and ..." no longer applies. Focus stays on the pressed button while armed; after the act the focus rules of section 10 apply (Step down to the row's Leave, Remove to the next row, Leave to the Removed block heading). The Add back dialog (8.2) keeps its inline confirm: it holds a choice.

## 13. Amendment: no sentences, no add back (customer feedback on PR #281, 2026-11-09)

- The armed acts of section 12 show **no sentence**; the sentences of sections 3.2 to 3.4 and 5.1 are gone, and so is the second-admin line (3.5). Only labels, states and real errors stay in the Share tab.
- The "Removed or left" list and Add back (section 8) are dropped, and with them Check, the add-back prompt, Sync again and Keep as it is (section 7). The Removed block is its heading.
- A removed device joins again with a new invite: Open shared document... or Join shared workspace... with the new link, as a first join (a workspace into a folder it chooses).

## 14. Amendment: the only admin alone (customer, 2026-11-09)

- A device whose session carries a shared workspace's link while no workspace is open in the tree shows the workspace's block anyway (name, people, Shared on, notices, Invite for an admin, Participants), so an only admin left alone can still invite or stop sharing.
- Stop sharing as `0092` UX notes section 14.
