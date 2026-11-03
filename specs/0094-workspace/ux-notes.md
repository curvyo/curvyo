# 0094 Workspace: UX notes

Status: Draft for Ready (slice 1). Owner: ux-engineer, 2026-11-03. Authority: `specification.md` (criteria 1
to 48; the visible ones are 1, 5 to 14, 16 to 21, 25 to 28, 35 to 44), customer decisions WS-UI (the workspace
lives inside the File tab, no new tab), WS-DELETE A, WS-VISIBLE A, WS-DELETE-ALL A, and the customer decisions
of 2026-11-03: WS-SHARE-PLACE B (Share workspace, Invite, participants and Remove live in the Share tab, not in
the File tab) and WS-SEPARATE B (a document shared on its own that lies in or is added to a shared workspace is
taken into the workspace; there is no "shared separately" state). References: `docs/design-system.md` ("Properties panel: File tab", "Properties panel: Layers
tab" treegrid, "Inline confirm", "Canvas notice slot", "Interaction conventions"), `specs/0092-cooperative-editing/ux-notes.md`
(Share tab, participant row, invite block, join block, notices; its Share tab is extended here), `FileTab.tsx`, `useRowsFocus.ts`,
`EntryField.tsx`. **(Proposal)** marks reversible choices. Customer rule 2026-10-16: no operating tips or hint
labels; labels that name a control or a state stay. Texts with state or numbers are formatted in Rust
(`curvyo-ui-core`). Desktop app only (criterion scope); nothing here relies on hover alone.

## 1. Where it lives

- **File tab, new section "Workspace"**, appended below the Export SVG block and above the divider and Quit.
  Nothing existing moves ("append, never move"); the project block, the six file buttons, the unsaved-changes
  check and Quit are unchanged. No new tab, strip icon, pill, toolbar, popup or dialog of our own. Only the OS
  folder picker floats (Open workspace, choose the folder for a joined workspace).
- **Height:** the tab body scrolls as a whole, as today. The workspace **tree is the one inner scroll area**
  of the section: it fills the body height left under the file buttons and has a minimum of 176px (four
  document rows), so at 800 x 600 the body scrolls a little and the tree is never squeezed to nothing.
- **Window title:** "Box job / Sides / Lid — Curvyo" (workspace name, path inside it, criterion 11); a shared
  workspace document "Box job / Lid (shared) — Curvyo" as in `0092`; the dirty "*" stays in front; a document
  opened by plain Open keeps "Lid — Curvyo". The page title follows.
- **Share tab** (`0092`) gets the workspace controls (section 4, 9); the File tab holds no sharing controls, only
  the state line in the header. **Collaboration pill** (`0092`) exists unchanged; for a workspace document it
  shows the workspace state. One pill, aggregate state of all rooms of the workspace.

## 2. Section layout (244px content; 8px rows; 12px above the section)

| # | Block | Shown when |
|---|---|---|
| 1 | Header row: workspace name and state | always |
| 2 | Empty block: Open workspace, Recent | no workspace open |
| 3 | Tree (treegrid) | workspace open |
| 4 | Action slot (inline confirm, move notice, errors) | on demand, between tree and Deleted |
| 5 | Deleted disclosure | shared workspace |

Absent blocks are not in the tree (hidden, not disabled).

### 2.1 Header row (28px)

No workspace: the label "Workspace" (14px `--toolbar-icon`, semibold) alone.

Workspace open: the name (the folder name, 14px, ellipsis, full path in a tooltip) and under it a 12px
`--panel-muted-fg` line: "14 documents" / "Shared, 2 people" / "Offline" / "Receiving 7 of 20"
(`role="status"`, polite, throttled to once a second). Right end, four 28px icon buttons in reserved slots
(36px coarse; Lucide 16px, 1.5px stroke, tooltip = accessible name, no chord): **New document** `FilePlus`,
**New folder** `FolderPlus`, **Recent workspaces** `History` (toggle, `aria-pressed`; section 2.3) and **Close
workspace** `X`. Tab order: name (not a stop), the four buttons, then the tree.

### 2.2 Empty state (no workspace)

The File tab with no workspace is as today plus the section, which holds, in this order:

1. **Open workspace...** (244 x 28 outline, `FolderOpen` glyph). Opens the OS folder picker. A folder that is
   not writable: refusal line below the button, `role="alert"`, `--field-invalid`: "Cannot write to this
   folder. It was not opened as a workspace." (criterion 4). A folder with a higher workspace format: "This
   workspace was made by a newer Curvyo. Nothing was changed." No other state is stored.
2. **Recent** list (section 2.3), shown at once when any exists: up to 10 rows, so the common case is one
   click, not two. With none, no list and no caption.

Joining a shared workspace is not here: it lives in the Share tab (section 8), next to "Open shared document...".
No illustration, no explanation text.

### 2.3 Recent workspaces (criterion 20)

`ul` named "Recent workspaces", rows 40px (44 coarse): `Folder` glyph 16px, the folder name (14px, ellipsis),
under it the path (12px `--panel-muted-fg`, ellipsis in the middle, full path in the tooltip), a 28px
remove slot at the right with `X` ("Remove Box job from recent", removes the entry only, no file touched).
Enter or click opens it. A missing path: name and path at 55% opacity, the word "Not found" in place of the
path's tail line, still removable, Enter shows nothing and sets focus on the remove button. While a workspace
is open, the header's Recent toggle shows this list **in place of the tree** (same height); picking one runs
the unsaved-changes check, closes the open workspace and opens the picked one; the toggle or Escape returns to
the tree.

Opening a workspace while another is open (Open workspace..., a recent row) goes through the same check as
Close (criterion 21): inline three-button check of the File tab, "Poster.curvyo has unsaved changes. Save them
before you open another workspace?" (same words, same keys, same reserved height; it replaces the six file
buttons, as today). Closing a shared workspace: no extra question (it disconnects, nothing is lost; it can
be reopened, criterion 33).

## 3. Tree

`role="treegrid"` named "Workspace documents", one Tab stop, roving focus, virtualised exactly like the Layers
treegrid (`useRowsFocus`; at most 60 rows in the DOM, the focused row's element kept when scrolled out,
`aria-rowcount`, `aria-rowindex`, `aria-level`, `aria-posinset`, `aria-setsize`, `aria-expanded` on folders,
`aria-selected` on the open document). A folder expands and collapses without a stall (criterion 5); expanded
state is per workspace session. Sorting: folders first, then documents, both by name (natural order, case
insensitive); no other sort in slice 1.

Rows, indent 12px per level (to level 6; deeper levels clamp the indent, the path is in the row name):

- **Folder row**, 32px (36 coarse): chevron `ChevronRight`/`ChevronDown` 16px, `Folder` glyph, name 14px,
  ellipsis; at the right the count (12px muted, "6") and, only on the focused or hovered row, the slot of the
  New document button for that folder (`FilePlus` 24px, "New document in Sides").
- **Document row**, 44px (48 coarse): thumbnail 40 x 28px at the indent (criterion 6), then line 1 the name
  without extension (14px, ellipsis), line 2 (12px muted) the time of the last change ("12:03"; "2 Nov 12:03" before
  today). Thumbnails come from `thumbnail.png` of the container, load lazily for visible rows only and
  after 100ms of rest (as the Fonts grid), on `--canvas-bg` with a 1px `--toolbar-icon` at 25% border; the
  placeholder is `FileText` 16px centred on the same ground. The thumbnail is never colour-only information.
- **Selected / open row:** the open document's row has the Layers look for a selected row (`--editor-accent`
  tint, 3px bar at the left, semibold name) and `aria-selected`; the focused row has the panel's focus ring.
  Hover: `--editor-accent-hover`.
- **Row actions** (cells reached with Right, as in Layers): **Duplicate** (`Copy`) and **Delete** (`Trash2`),
  24px icon buttons in a 52px slot at the right end of line 1, visible on hover, on the focused row and on the
  open row (the slot is reserved so names do not shift), accessible names "Duplicate Lid", "Delete Lid".
  Rename is F2, double-click on the name, or Enter in the name cell edit mode: no button (a text-like action
  that frequent is a key, and the list stays calm).

Row states (a second 12px line in `--panel-muted-fg`, replacing the time, always text):

| State | Row text | Notes |
|---|---|---|
| Normal | time | |
| Cannot be read (criterion 8) | "Cannot be read: not a Curvyo file." / "...: damaged." | `TriangleAlert` 16px in place of the thumbnail; Enter selects it and the action slot shows the full reason, nothing opens |
| Not in the shared workspace (criterion 41) | "Not in the shared workspace" | row grows to 64px; third line, **Add to workspace** (24px outline text button, 12px label, `aria-label` "Add Lid to workspace"). Never added silently; a document with its own sharing is taken in after a confirm (section 6) |
| Missing (criterion 7, the open file removed) | "File missing" | row stays 60s then leaves; Save as stays possible (section 10) |
| Missing on this computer (criterion 41, shared workspace, the file removed by another program) | "Missing on this computer" **(Proposal)** | **Download again** (24px outline text button) in the row's action slot **(Proposal)**; the row keeps 44px |
| Cut | name at 55% opacity | pending move (3.2) |

### 3.1 Open and switch (criteria 9 to 12)

Click, or Enter on a row, opens the document in the window; with unsaved changes the three-button check of
the File tab comes first and focus goes to its Cancel. Exactly one document is open. Folder rows: Enter and
Space toggle. After a keyboard open focus stays in the tree on the row (the canvas takes it with Escape or by
clicking it); after a mouse open focus goes to the canvas, as for the file buttons. The previous document's
zoom, scroll and page layout are restored on return (criterion 10); no message.

### 3.2 Create, rename, move, duplicate (criteria 13 to 19)

- **New document / New folder:** a new row appears at the top of the target folder (the selected folder, else
  the root) with an inline name field in the row (`EntryField` behaviour: Enter commits, Escape cancels and
  removes the row, Tab commits; default text "Untitled" selected, 1 to 120 characters). For a document the
  file is written and opened on commit. A taken name gets " 2", " 3"; the maker is told by a status in
  the action slot, 3s: "Created Lid 2." (criterion 13). New document button in the header acts on the root or
  the selected folder row; the button on a folder row acts on that folder.
- **Rename** (F2, double-click on the name): the same inline field in place of the name, text selected
  without the extension. Enter commits, Escape restores. Refused inline (message line in `--field-invalid`
  under the field, `aria-invalid`, the old name stays): "A name cannot contain / or \." / "A file with this name
  exists." / "This name is not allowed on this system." / "Enter a name." Renaming the open document keeps it
  open, no reload, no loss.
- **Move by drag:** drag a row onto a folder row; the target folder row shows a 1.5px `--editor-accent`
  outline, the dragged row follows the pointer at 70% opacity; dropping on the header row's name moves to the
  root. A refused move (into itself, name exists) ends the drag with no change and the action slot shows an alert, 8s:
  "Cannot move Sides into itself." / "Sides already has a Lid." Auto-expand of a folder after 600ms hover; no
  other motion.
- **Move by keyboard:** Ctrl+X (Cmd+X on macOS) on a row marks it as cut (55% opacity), Ctrl+V on a folder row
  or the header row moves it there; Escape clears the mark. In the treegrid these chords belong to the tree
  and never reach the canvas. **(Proposal)**
- **After a move,** the action slot shows for 10s, `role="status"`: "Moved Lid to Sides." and an **Undo move**
  button (outline, 76 x 28, in the Tab order after the tree). A new move replaces it. It is not a history step
  of any document; Ctrl+Z on the canvas does not touch it.
- **Duplicate:** writes `Lid copy.curvyo` beside it, the row appears selected, name field open for a rename
  at once (Enter keeps it). Status 3s "Created Lid copy." Not opened.

### 3.3 Delete (criteria 17, 40)

Delete (key Delete, or the Trash button) opens the **Inline confirm** in the action slot (tree shrinks by its
height, the row stays selected and in view). Local workspace, sentence 12px, formatted in Rust:

- Document: "Move Lid to the trash?" Folder: "Move Sides and its 6 documents to the trash?"
- Buttons **Cancel** (outline, focused) and **Delete** (`--destructive`, 76 x 28; ignores presses for 400ms
  as the File tab's Discard). Escape and Cancel close it; focus returns to the row.
- No OS trash: nothing is deleted, `role="alert"`: "The system has no trash. Lid was not deleted."
- Deleting the open document: after the unsaved-changes check, the window shows an untitled empty document as
  after Close; the row is gone.

Shared workspace (WS-DELETE-ALL A), same slot, sentence: "Delete Lid for everyone? Every computer moves it to
its trash. You can restore it from Deleted." (folder: "Delete Sides and its 6 documents for everyone? ..."),
same buttons. Other devices: section 7.

## 4. Sharing in the Share tab (criteria 25, 26, 43, 44)

The File tab has no sharing block. All workspace sharing lives in the Share tab (WS-SHARE-PLACE B), as a
**Workspace block** that appears when a workspace is open. Layout of the tab with a workspace open, top to
bottom (the `0092` tab with one block added; nothing of `0092` moves):

1. Server and Name rows, exactly as `0092` (stored once per OS user, read-only while sharing; used by both the
   workspace and a document).
2. **Workspace block**, headed by a 14px semibold line with the workspace name and, at the right, the state
   "Not shared" / "2 people" (12px muted). Contents below. 1px `--toolbar-icon` at 25% divider under it.
3. **Document block**: the `0092` content for the open document (section 9).
4. "Open shared document..." and "Join shared workspace..." (section 8), as the last rows, as in `0092`.

Workspace block, same components as the `0092` document flow (one implementation, one set of texts, nouns
change from document to workspace):

- **Share workspace** (118 x 28 minimal; Apply look, `Share2`; `aria-disabled` with reason line "Set a server
  address." / "Cannot reach the server."; the workspace stays editable). Pressing it opens the Inline confirm:
  "Upload the workspace Box job (14 documents) to 192.168.1.20:8080? Names, folders and files leave this
  computer encrypted; the server cannot read them." Buttons **Cancel** (focused) and **Share**. This is the one
  place the privacy fact is said for the workspace (R-SYS-007). When documents of the workspace are shared on
  their own, the sentence is extended, same confirm, formatted in Rust: "Lid and 2 other documents are shared on
  their own. They become part of the workspace and their own links stop working." A document that cannot be taken
  in is named in the same confirm and stays out (section 6). Then "Sharing..." (`aria-busy`) with the count,
  "Uploading 3 of 14", `role="status"`. Failure: "Could not share: cannot reach the server." with the button
  back; nothing was written.
- **Shared:** line "Shared on 192.168.1.20:8080", the 12px connection line (as `0092` section 6, aggregate:
  the worst state of any room wins; "Syncing 3 of 50" while background rooms catch up, criterion 32). No
  "Stop sharing" (not in the criteria; as 0092 STOP-SHARING).
- **Invite:** as `0092` 2.3 (radiogroup "Valid for" 1 hour, 24 hours (selected), 7 days; Create link; Copy;
  Done), with the sentence of criterion 26 verbatim: "Anyone with this link can edit every document of
  this workspace once, until it expires." (the `2.` link text is the only artefact).
- **Participants:** the `0092` 2.4 list; the Remove sentence: "Remove Ben from Box job? Removing does not reach
  back: Ben keeps what is already on that computer and gets no later changes." Admin-only button, Cancel
  focused, offline refusal "Not connected. Try again when connected." During rotation of the keys (criterion 43)
  the row shows "Removing..." (`aria-busy`) and the row leaves on acknowledgement; no reload. A participant online
  during the rotation sees nothing.
- **On the removed device (criterion 44):** the File tab's Workspace section shows an alert block at the top,
  once, until dismissed with **OK** (**Proposal**: it is about the tree the person is looking at, and it is
  also announced as `role="alert"`): "You were removed from this workspace. Your copies stay on this computer."
  The workspace becomes a local workspace (no Users glyph, header line without people, the Workspace block of
  the Share tab says "Not shared" and Share workspace is available again), the open document stays open,
  nothing is deleted. A removal found on reconnect (criterion 45) shows the same block.

## 5. Deleted list (shared workspace, criterion 40)

A disclosure "Deleted" with the count at the right ("3"), collapsed by default, in the tree only in a shared
workspace. `ul` "Deleted documents", rows 44px: `Trash2` 16px, name (14px), line 2 12px muted "Deleted by Ben, 12:03"
("by you" for the own); at the right **Restore** (24px outline text button, accessible name "Restore Lid"). Any member
may restore; the file is written again at its old place from the last synced state, the row leaves the list
and appears in the tree selected; status 3s "Restored Lid." A name taken since gets " 2" and the status says
the final name. Restoring is not an undo and never changes the document. Empty list: the disclosure says
"0"; opened, one 12px muted line "No deleted documents." The list is capped at 200 rows with "and N more"
(virtualised past 60).

## 6. Not-in-workspace files and takeover (criteria 41, 42)

- A file added by another program to a shared workspace's folder shows the row state "Not in the shared
  workspace" with **Add to workspace** (section 3). Add writes it into the list; the row becomes normal;
  status 3s "Added Lid to the workspace." A document that already carries a room of a workspace elsewhere gets
  a new document id silently (criterion 41); no message.
- **Takeover (WS-SEPARATE B).** A document that is shared on its own (a `0092` room) and lies in a shared
  workspace's folder, or is added to it, is taken into the workspace; there is no "shared separately" state
  in the tree. Two routes, one rule:
  1. **Add to workspace** on such a row opens the Inline confirm in the action slot, Cancel focused: "Lid is
     shared on its own with 2 people. Take it into Box job? Its own link stops working. People of Box job can
     edit it." Buttons **Cancel** and **Take in** (76 x 28). Then status 3s "Added Lid to the workspace."
  2. **Share workspace** over a workspace that holds such documents: the sentence is part of the Share
     confirm (section 4); there is no second confirm.
  Participants of the document's own room who are not members of the workspace: the document is **refused**,
  not taken in; the row stays "Not in the shared workspace", the action slot shows `role="alert"`, 8s: "Lid was
  not added: it is shared with people who are not in Box job." and in the Share confirm the same document is
  named in a line "Not added: Lid (shared with people outside this workspace)". The exact rule (who counts as a
  member, what happens to the old room) comes from the architect's ADR; the texts are formatted in Rust and
  change with it, the places stay.
  After a takeover the document's Share tab block reads "Shared through workspace Box job" (section 9) and its
  collaboration pill shows the workspace state.
- In a **local** workspace a document shared on its own keeps working as `0092`; its tree row is a normal
  row. The takeover happens when the workspace is shared or the document is added to a shared workspace.

## 7. When other people change the tree (criteria 35 to 40)

No conflict screen (the model merges, ADR 0009). The tree updates within 2s. Notices go to the canvas notice
slot (`role="status"`, 5s, at most 70 characters; the participant's label cut to 16 characters, "Someone" when
unknown; names of items cut to 24 characters with the ellipsis in the middle, extension never shown here):

| Event | Notice | Tree |
|---|---|---|
| Ben creates a document | none | row appears; if not yet downloaded, row line 2 "Receiving..." and thumbnail placeholder, opens once present |
| Ben renames | "Ben renamed Lid to Cover." | row moves to its sorted place; if open, title changes, no reload |
| Ben moves | "Ben moved Lid to Sides." | row moves; a row being dragged by me and moved by Ben ends my drag with no change |
| Same document moved or renamed by two people | none | both converge on one result; no message (criteria 37, 38) |
| Folder-loop move skipped (criterion 39) | "Folder move skipped: it would loop." | tree unchanged for that move |
| Ben deletes | "Ben deleted Lid." (several: "Ben deleted 3 documents.") | row leaves; the Deleted disclosure count grows |
| Ben deletes the document I have open with unsaved changes | project block status becomes "Deleted by Ben. Save as to keep it." (replaces "Unsaved changes", `role="status"`) | row stays flagged "Deleted by Ben" until saved elsewhere or closed; the file is not trashed until then (criterion 40) |
| Ben deletes the document I have open, no unsaved changes | "Ben deleted Lid." and the window shows an untitled empty document | row leaves |
| Ben's change while I rename that row | my rename field closes without writing, a status line under it, 5s: "Ben changed this document's name." | row follows Ben's result |

Timing of the first appearance of a document on a joining device: tree count "Receiving 7 of 20" (header
line); rows appear as they arrive and open as soon as their own state is complete (criterion 28).

## 8. Join by a `2.` link (criteria 27, 28, 29)

Joining lives in the Share tab (**Proposal**: the File tab keeps the tree, Deleted and recents; every
invite-link action is in one place). The Share tab shows two outline buttons, "Open shared document..." and
"Join shared workspace..." (`LogIn`), without a workspace open also in the empty Share tab. Either one opens
the same join block in place (the fields are identical); the parser decides by the link, so a `2.` link pasted
under "Open shared document..." goes the workspace route and a `1.` link under "Join shared workspace..." the
document route. Steps, in one block, each shown as it is entered:

1. Text field "Invite link" (autofocus, mono 12px, one line; paste). Parser decides `1.` (document, the `0092`
   flow, opens in the window) or `2.` (workspace, below).
2. "Name shown to others" only when none is stored.
3. **Choose folder...** (outline, `FolderOpen`) opens the OS picker. The chosen folder is shown as a read-only
   path line. Refusals (message line, `role="alert"`): "Choose an empty folder." / "Choose a folder outside any
   workspace." / "Cannot write to this folder."
4. **Cancel** and **Join** (Apply look; Join is `aria-disabled` until a folder is set with the reason line "Choose
   a folder.").
5. With an open workspace or unsaved changes, the unsaved-changes check comes first (joining closes the open
   workspace).

Status rows, `role="status"`, in order: "Connecting..." / "Waiting for a member to let you in" (with **Cancel**,
which leaves the room) / "Receiving 7 of 20". On admission the File tab's Workspace section becomes the shared workspace (tree
filling, header "Shared, 2 people"); no success notice. No document opens by itself. Refusals: "This link was
already used." / "This link has expired. Ask for a new one." / "This link is not valid." / "Cannot reach the
server at 192.168.1.20:8080."; nothing of the workspace is shown, the field keeps its value.

## 9. The Share tab for a workspace document

The Workspace block (section 4) is shown whenever a workspace is open, whichever document is open. The
**Document block** below it depends on the open document:

| Open document | Document block shows |
|---|---|
| Not in a workspace, not shared | `0092` as today (Share, Join) |
| In a workspace, not shared, workspace not shared | `0092` as today; the document can be shared on its own and is taken in when the workspace is shared (section 6) |
| In a shared workspace, part of it | One line "Shared through workspace Box job" (14px) and the connection line. No Share, Invite, participants or Remove: they are in the Workspace block above. |
| In a shared workspace, "Not in the shared workspace" | `0092` as today under the subject line "Not in the shared workspace"; sharing it on its own is allowed, adding it takes it in (section 6) |
| Shared on its own, in a local workspace | `0092` as today with its own invite and participants |

A document shared on its own inside a shared workspace does not exist as a state (WS-SEPARATE B): it is taken
in or refused. The server setting rows stay as in `0092`. The collaboration pill (`0092` section 3) is shown
for a document that is part of a shared workspace and for one shared on its own, with the aggregate state
(section 4); accessible name for a workspace document "Collaboration, workspace Box job, 2 people, connected".
Opening a workspace document, switching or closing never shows the Share tab by itself.

## 10. States and errors

| Case | Behaviour |
|---|---|
| Large tree (500 documents) | Virtualised; the scan shows "Scanning..." (`role="status"`, header line) only past 300ms; the tree fills as rows arrive in sorted order; no layout jump (row heights fixed) |
| Empty workspace (criterion 1) | The tree shows one 12px muted line "No documents." and a **New document** button (118 x 28 outline) centred 16px below it; the header button does the same |
| External change (criterion 7) | Tree updates within 5s; rows move or leave without a notice; if the open file is removed: project block status "File missing", Save as stays |
| File changed by another program while open | Not specified by 0094; out of scope (existing File tab behaviour) |
| Open fails (damaged) | Existing error dialog "Can't open file" with the reason; the row gets "Cannot be read" |
| Server unreachable (shared workspace) | Header line "Offline", pill state `CloudOff`; local rows and file operations work; a rename, move or delete done offline is queued, header line "Offline. Changes are kept." |
| Long names | Ellipsis in the middle for items in text, ellipsis at the end in the tree; full name in the tooltip and in the row's accessible name |
| Path outside the workspace removed or renamed while open | Tree shows "Not found" in the header line, the open document stays; Close workspace works |
| Window below 244px content | Not supported (panel width is fixed) |
| Workspace names, labels and file names | User text, always plain text, never markup |

## 11. Keyboard and focus

Tab order in the tab: the six file buttons (or the check), Export SVG and its switches, then the Workspace
section: header buttons (New document, New folder, Recent, Close workspace), the tree (one stop), the action
slot's buttons (Undo move, or the confirm's Cancel and Delete, when shown), the Deleted disclosure and its
Restore buttons (when open), Quit. The Share tab's order: Server and Name rows, the Workspace block (Share
workspace or Invite, participants and their Remove buttons), the Document block, the two open and join buttons. A new action slot
appears with focus on its first control only for a confirm (Cancel) and an error; a status notice does not move focus.

| In the tree | Key |
|---|---|
| Move between rows | Up, Down, Home, End, PageUp, PageDown |
| Expand, collapse a folder; move to the row action cells | Right, Left (as Layers) |
| Open the document, toggle a folder | Enter (folders also Space) |
| Rename | F2 |
| Delete | Delete (opens the confirm) |
| Cut, paste into a folder | Ctrl+X, Ctrl+V (Cmd on macOS) |
| Cancel a cut, close a confirm or a name field | Escape |
| New document in the selected folder | Ctrl+Alt+N reserved for later; slice 1 has the header button only |

- Escape from the tree sets focus on the canvas only when nothing is open to cancel.
- After closing a confirm or an inline name field, focus returns to its row.
- After deleting, focus moves to the next row, else the previous, else the header's New document button.
- After Open workspace, focus goes to the tree's first row; with none, the New document button.
- Hit targets: icon buttons 28px (36 coarse); rows 32 and 44px (36 and 48 coarse); the 24px row actions are
  inside a row whose whole width is a target. Contrast of 12px muted text against `--panel-bg` as the rest of the panel.
- Live regions: one visually hidden `role="status"` region for tree announcements ("Moved Lid to Sides.",
  "Deleted Lid."), polite; refusals `role="alert"`. The row name for a screen reader: "Lid, document, changed
  12:03" / "Sides, folder, 6 documents, expanded".
- Reduced motion: no motion exists (no row animations; auto-expand is a state change).

## 12. Tokens and components

Reused unchanged: `--toolbar-bg`, `--toolbar-icon`, `--toolbar-icon-active-bg/-fg`, `--editor-accent`,
`--editor-accent-hover`, `--panel-muted-fg`, `--field-invalid`, `--canvas-bg`, `--destructive`, `--peer-1` to
`--peer-6` and the participant row, avatar disc, invite block, join block, Inline confirm (two and three
buttons), `PanelRadioGroup`, `EntryField`, `useRowsFocus`, the treegrid and row look of Layers, the canvas notice
slot, `Tooltip`.

New (to enter into `docs/design-system.md` when this ships): the **Workspace section** of the File tab, the **Workspace block** of the Share tab (headed block above the document block), the
**document row** (44px with 40 x 28 thumbnail), **folder row** (32px), **action slot** (reserved on demand
under the tree), **disclosure row** (Deleted; 28px, chevron, count at the right), **recent
workspace row**. No new colour or size token.

## 13. Review checklist

Open workspace on an unwritable folder; a folder with a newer format; 500 documents (scroll, expand,
thumbnails lazy); rename to a taken name, with a slash, empty; rename the open document; drag onto self, onto
a taken name; Ctrl+X, Ctrl+V; Undo move after 10s; Delete with the OS trash missing; delete a folder with 6
documents; delete the open dirty document; Deleted list restore with a taken name; two instances: Ben deletes
while I rename; a folder loop; unreadable file row; Not in the shared workspace and Add; takeover of a document
shared on its own (confirm, via Add and via Share workspace) and the refusal with outside participants; Share tab with a
workspace open (Workspace block above Document block; Share workspace, Invite, Remove only there; File tab has no sharing controls); join with
a `2.` link (from the Share tab, both buttons): used, expired, invalid, non-empty folder, folder inside a workspace, waiting, cancelled while
waiting, over unsaved changes; removal of a member and the removed device while offline; server restart (pill
goes through Reconnecting); keyboard-only run of every action; coarse targets; 800 x 600 body scroll.

## 14. Open UX questions (each with a default)

- **WS-SHARE-PLACE:** decided B (2026-11-03): all workspace sharing controls in the Share tab. Not open.
- **WS-JOIN-PLACE:** "Join shared workspace..." in the Share tab next to "Open shared document..." (this note) /
  also in the File tab's empty state. Default: Share tab only (one place for link actions). **(Proposal)**
- **WS-RECENT-PLACE:** recent workspaces listed in the empty state (this note, one click) / behind Open
  workspace... Default: in the empty state and behind the History button.
- **WS-MOVE-KEY:** Ctrl+X and Ctrl+V move rows in the tree. Default: yes (the tree has no other keyboard
  route for a move).
- **WS-THUMB:** thumbnails 40 x 28px in the rows (this note) / names only with larger thumbnails on hover.
  Default: 40 x 28; criterion 6 requires a thumbnail, the panel is 244px wide.
- **WS-UNDO-MOVE:** "Undo move" for 10s in the action slot (criterion 16 Proposal). Default: yes.
- **WS-NEW-CHORD:** no chord for New document or Open workspace in slice 1 (every frequent chord of the file
  buttons is already taken; the File tab buttons are the route). Default: none.
