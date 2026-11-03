# 0092 Cooperative editing: UX notes

Status: Draft for Ready (slice 1, which includes removing a participant, customer decision COOP-SLICE B, 2026-11-03). Owner: ux-engineer,
2026-11-02. Authority: `specification.md` (criteria 1 to 5, 14 to 17, 21 to 23, 31 are the visible ones),
`adrs.md`, ADR 0031. References: `docs/design-system.md` ("Properties panel: tabs", "File tab", "Inline
confirm", "Canvas notice slot", "Interaction conventions", "Color tokens", the Document section),
`PanelStrip.tsx`, `CanvasNotice.tsx`, `EntryField.tsx`, `ValueField.tsx`, `FormatList.tsx`. **(Proposal)**
marks reversible choices. Customer goal: nice, consistent, above all working, no new toolbars. Customer
rule 2026-10-16: no operating tips or hint labels; labels that name a control or a state stay. Texts with
state or numbers are formatted in Rust (`curvyo-ui-core`), as for the other panels. Slice 1 is the desktop
app; nothing here uses a hover-only cue without a keyboard route.

## 1. Where it lives

- **Share tab** in the Properties panel strip, label "Share", Lucide `Users` 16px, appended after the last
  tab that exists when this ships (the order never changes). Tooltip "Share", no chord in slice 1 (every
  letter chord with Shift+Ctrl is taken or reserved by Chrome; the pill and the strip reach it). It holds
  the server setting, the name, Share, Invite, Join and the participant list. It never activates by itself.
- **Collaboration pill**, floating at the canvas's bottom right in the shared **status dock** (also used by
  0064's Machine pill): inset 12px, a column with 8px gaps, collaboration pill above, `z-40`. It exists
  only while the open document is shared. It shows who is here and the connection state, and opens the
  Share tab on click.
- **Canvas overlay**: the other participants' pointers and selections (section 5).
- **Window title**: "<file> (shared) — Curvyo" for a shared document (criterion 1d), the page title in
  the same form; the dirty "*" stays in front as today.

No toolbar, no rail card, no popup, no dialog of our own. The only floating things are the pill and the
canvas overlay. File tab unchanged.

## 2. Share tab layout (244px content, 8px rows, 12px between blocks)

Header row: "Share" at the left, subject line right-aligned: "Not shared" / "Shared, 2 people" / "Offline".
Blocks, top to bottom; absent blocks are not in the tree (hidden, not disabled), except the one reason line:

| # | Block | Shown when |
|---|---|---|
| 1 | Settings: "Server" and "Name shown to others" | always (the Server field read-only while shared) |
| 2 | Share / status | not shared: the Share button; shared: status line |
| 3 | Invite | shared |
| 4 | Participants | shared |
| 5 | Join | not shared |

### 2.1 Settings

Two rows, each a 12px label above a 28px text field (the `EntryField` behaviour: Enter or Tab commits,
Escape restores, invalid gets `--field-invalid`, message line, `aria-invalid`).

- **Server** (the self-hosted demo setting, criterion 1 and ADR 0004 §13): placeholder "Server address",
  accepts `host:port`, `ws://host:port`, `wss://host:port` and stores the normalised form. Per OS user,
  application setting, never in the project file. While not shared it is editable; while shared it shows
  as a read-only line (a shared document's room is on one server). Refusal: "Not a server address."
  A status line under the field (12px, `role="status"`, polite, changes only on a result): "Reaching..." /
  "Reachable" / "Cannot reach 192.168.1.20:8080." A check runs after a commit and when the tab is shown with
  a server set; never in the background otherwise. Empty server: Share is `aria-disabled` with the reason
  (2.2).
- **Name shown to others** (the device label of ADR 0010, shared with 0064): required once, default the OS
  host name, 1 to 24 characters, editable at any time; a change is announced to the room within a second.
  Refusal: "Enter a name." No further text; the label says who sees it.

### 2.2 Share

Not shared, the button **Share** (118 x 28, Apply look, `Share2` glyph), `aria-disabled` at 40% with its
Tab stop and tooltip when it cannot work, and a reason line under it (12px muted, `aria-describedby`):
"Save the document first." / "Set a server address." / "Cannot reach the server." (criterion 1: the
document stays editable, the reason is named).

Pressing Share replaces the button in place by the **Inline confirm** (two buttons, Apply look, not the
destructive look because nothing is lost): "Upload this document to 192.168.1.20:8080? It leaves this computer
encrypted; the server cannot read it." Buttons **Cancel** (focused) and **Share**. This is the one place
the privacy fact is said (data leaves the machine, `R-SYS-007`); no other caption in the feature. Escape
and Cancel close it without writing. On Share: the button reads "Sharing..." (`aria-busy`), then the block
becomes the shared state. A failure: the sentence "Could not share: cannot reach the server." with the
button back (`role="alert"`, nothing was written).

Shared, the status line (14px): "Shared on 192.168.1.20:8080" and under it 12px muted the connection text of
section 6. No "Stop sharing" in slice 1 (not in the criteria; question STOP-SHARING).

### 2.3 Invite

Button **Invite** (118 x 28 outline, `UserPlus` glyph). It opens, in place, the Invite block:

1. A `role="radiogroup"` "Valid for" of three items **1 hour, 24 hours, 7 days** (`PanelRadioGroup`
   look; 24 hours selected; **Proposal**, criterion 2). Not shown again after the link exists.
2. **Create link** (118 x 28, Apply look). It writes the pending invite (ADR 0010 `AddPendingInvite`) only
   when pressed, so looking at the block creates nothing.
3. After creation: the sentence of criterion 2, verbatim, 12px: "Anyone with this link can edit this document
   once, until it expires." and the expiry in the same line group: "Expires Tue 14:32." Then a read-only
   text field with the link (mono 12px, `<input readonly>`, selects all on focus, one horizontal line, no
   wrapping) and **Copy** (`Copy` glyph, 76 x 28, Apply look). After Copy the label reads "Copied" for 2s
   (`role="status"`, polite) and the text is on the clipboard; if the clipboard is refused the field is
   selected and the label reads "Select and copy". **Done** closes the block (focus back to Invite).
   Leaving the tab keeps the block; Escape closes it; the link stays valid (the server expires it).

Raw key material is never shown or typed (criterion 2). The link text is the only artefact; the secret is
after `#`.

### 2.4 Participants

A list, `ul` named "Participants", rows 40px (44 with coarse). Row: a 24px disc in the
participant's colour with the initials (section 4), the label (14px `--toolbar-icon`, ellipsis) and the muted
"this device" after the own label, and under it the fingerprint (12px mono `--panel-muted-fg`, 16 base32
characters in four groups of four, the same helper as 0064's connector key). Right end: "offline" in 12px
muted for a participant without presence, nothing for online ones; an offline row's disc is at 55% opacity
(text and shape carry it: the word "offline"). Own row first, then by join order. Criteria 3: both lists show
both entries with label, colour and fingerprint.

**Remove (slice 1, criteria 32 and 33).** A row action slot of 28px sits at the right end of every row (the
slot is reserved on all rows so labels do not shift). Only an admin sees a button in it, and only on the
rows of other participants: none on the own row, none for a member. The button is `UserMinus` 16px, outline
look without border, tooltip "Remove", accessible name "Remove Ben"; it is a Tab stop and the only one in the
list. Pressing it replaces the row's lower part in place by the **Inline confirm** (two buttons, destructive
look on the action, **Cancel** focused, Escape and Cancel close it and return focus to the Remove button), with
the sentence, 12px, formatted in Rust: "Remove Ben? Removing does not reach back: Ben keeps what is already on
that computer and gets no later changes." (label cut at 24 characters; "This participant" when empty). This is
the one place the limit of removal is said. The action button **Remove** writes the removal; the row leaves
the list when the removal is acknowledged. Offline, the removal is refused before anything is written: a
`role="alert"` line under the row in `--field-invalid`, "Not connected. Try again when connected.", the
confirm stays open for a retry.

**On the removed device:** the canvas notice slot shows once, as an alert: "You were removed from this
document. Your copy stays on this computer." The document stays open as a plain local file (not shared, no
pill, Share tab in its not-shared state with Share and Join available); nothing on the device is deleted and
no later changes arrive.

### 2.5 Join

Button **Open shared document...** (244 x 28 outline, `LogIn` glyph, label "Open shared document..."),
last in the tab, in the tree only when the open document is not shared. It replaces itself in place by the
Join block:

1. A text field "Invite link" (autofocus, paste; `curvyo://` or `https://…#…` shapes are accepted by the
   parser, the field shows what was pasted, mono 12px, one line).
2. "Name shown to others" appears here only if none is stored (the same field as 2.1; once set it is not
   asked again).
3. Buttons **Cancel** and **Join** (Apply look). Enter in the link field joins.
4. If the open document has unsaved changes, the existing **three-button inline check** of the File tab
   replaces the block first ("Poster.curvyo has unsaved changes. Save them before you open another
   project?"), because joining replaces the open document. Same words, same keys.

While joining: a status row in the block, `role="status"`, in this order as each phase is entered:
"Connecting..." / "Waiting for a member of this document to let you in" (until a member's `RewrapEpoch`
arrives, ADR 0031 option A; with **Cancel**, which leaves the room) / "Downloading..." The document then
opens, the tab switches to the shared state, the subject line says "Shared, 2 people". No success
notice (the document appearing is the result).

Refusals (`role="alert"`, message line in `--field-invalid` under the link field; nothing of the document
is shown, criterion 4): "This link was already used." / "This link has expired. Ask for a new one." / "This link is not
valid." (malformed) / "Cannot reach the server at 192.168.1.20:8080." The link field keeps its value for
editing.

A joined document opens as **Untitled, shared**, dirty, so closing asks to Save, and the first Save
chooses a place (criterion 5 then holds on reopening). **(Proposal, question JOIN-SAVE.)**

## 3. Collaboration pill

Look: the contextual bars' (`--toolbar-bg`, `--panel-elevation-shadow`, `rounded-md`), height 32, padding
4 6, 12px text, `role="group"` `aria-label="Collaboration"`. Contents, left to right: the connection glyph
(16px), up to four avatar discs (24px, overlapped by 6px, 2px `--toolbar-bg` ring), then "+2" (12px) for more,
and for the abnormal connection states the short text of section 6. It is one `button` named
"Collaboration, 2 people, connected"; it opens the Share tab (shows the body if hidden) with the participant
list focused-in-view. Tooltip: the labels of all participants, one per line, own marked.

Online avatars only; offline participants do not appear in the pill (they remain in the list).

## 4. Participant colours and avatars

Colour is derived from the fingerprint (ADR 0031 decision 5), so every device shows the same colour for the
same person without storing it. Six tokens, picked so none collides with the shell's semantic colours
(`--editor-accent` blue, `--marquee-touch` green, `--marquee-contain` red, `--clone-link` violet, grey chrome),
each with white text at 4.5:1 or better and 3:1 or better against `--canvas-bg` and white (to be measured in
review):

| Token | Value | Name (for tests only; never shown) |
|---|---|---|
| `--peer-1` | `#C2410C` | orange |
| `--peer-2` | `#0F766E` | teal |
| `--peer-3` | `#BE185D` | pink |
| `--peer-4` | `#4D7C0F` | olive |
| `--peer-5` | `#854D0E` | ochre |
| `--peer-6` | `#A21CAF` | magenta |

More than six participants repeat colours; the label always travels with the colour, so colour is never the
only cue. A disc shows the initials: first letters of the first two words of the label, else the first two
letters, uppercase, 11px bold white. Own colour is shown in the list and the pill for consistency with what
others see; nothing on the own canvas is drawn in it (own selection stays `--editor-accent`).

## 5. On the canvas

Everything here is presence (criterion 16): drawn in the overlay, never in the document, in no history step,
no undo. Sizes are screen-space and do not scale with zoom.

- **Remote pointer:** a 16px arrow in the participant's colour with 1px `--selection-casing` casing, hotspot
  at its tip, and below-right of it a **label chip**: the label (12px bold, white) on the participant's
  colour, 2px 6px padding, `rounded-[4px]`, at most 16 characters then an ellipsis; criterion 16's tool icon
  (14px Lucide glyph of the tool, white) follows the label inside the chip only while the tool differs from
  Select (**Proposal**). The pointer disappears 5s after the last presence message or at once on disconnect
  (criterion 14). No motion smoothing: it jumps with the data (nothing to switch off for reduced motion).
  Hidden while the participant's pointer is outside the viewport; no edge indicators in slice 1.
- **Remote selection:** the union bounding box of the participant's selected objects, 1.5px solid in the
  participant's colour over a 1px casing, no handles, no fill, 2px outside the box. The chip of the label
  sits at its top-left corner outside the box (the same chip, one per selection). Several participants on
  one object: the boxes are offset by 3px each. Own selection drawing is unchanged and wins hit-testing; the
  remote box never intercepts the pointer.
- **Live drag** (criterion 7): the object itself moves, drawn normally, the remote box follows it. Nothing
  else marks the drag.
- **Contested object** (criterion 15): when the local pointer hovers over, or presses on, an object another
  participant has selected or is dragging, the label chip of that selection expands in place to "Ben is
  editing this" (the chip grows to the right, same colour, maximum 36 characters of text with the label cut
  first). It is a state of the chip, not a new element. It does not block: no lock, no modal. Keyboard route:
  when the local selection gains an object that is in a remote selection, the canvas notice slot shows the
  status "Ben is editing this." once per such selection (3s). Both routes are shown before the local edit.
- **Text editing by another:** the other's caret is not shown (presence shares the pointer and selection
  only); the chip reads "is editing this" while the text object is selected by them.

## 6. Connection state

States, glyph in the pill and one 12px line in the Share tab status block (under "Shared on ..."), all
`role="status"` polite except where marked:

| State | Pill glyph and text | Share tab line | Notes |
|---|---|---|---|
| Connected | `Radio`, no text | "Connected" | quiet: avatars only |
| Connecting / reconnecting | `RefreshCw`, "Reconnecting..." | "Reconnecting..." | rotates once a second; static under reduced motion. From the first lost connection until the server answers; backoff is invisible |
| Offline | `CloudOff`, "Offline, changes are kept" | "Offline. Changes are kept and merge when the connection returns." | criterion 17, within 5 s of the loss; editing, saving, exporting unaffected; remote pointers vanish |
| Syncing | `RefreshCw`, "Syncing..." | "Syncing..." | between reconnect and the last acknowledgement; skipped if under 1 s |
| Resyncing | `RefreshCw`, "Resyncing..." | "Resyncing from the server's snapshot." | after a rejected update; `assertive` once (7.3) |
| Not connected | `CloudOff`, "Not connected" | "Not connected to 192.168.1.20:8080." | shared document and the server cannot be reached at open; same editing |

After an offline period longer than 5 seconds the canvas notice slot says once "Back online." (status, 3s)
when the merge has finished. Briefer drops are silent. No state ever blocks a tool or shows a modal.

## 7. When things disagree

There is no conflict screen and none is designed: the model merges (ADR 0009). The visible cases:

1. **A contested scalar** (criterion 9): no message; one value wins on both sides. The marker of section 5
   is the mitigation.
2. **Someone deletes what I am editing.** The delete wins, nothing is resurrected, nothing crashes
   (criterion 11):
   - *Dragging it:* the drag shows the in-flight ghost until the release, which writes nothing (ADR 0031
     decision 3). Canvas notice, status, 5s: "Ben deleted the object you were moving."
   - *Typing in it:* the text edit ends in the same frame, the caret goes, the selection is empty. Notice
     (status, 5s): "Ben deleted this text while you were editing." What I typed is part of the deleted object.
   - *Selected only:* the object leaves my selection silently when it is one of several; when the selection
     becomes empty by a remote delete, notice (status, 3s): "Ben deleted the selected object."
   - *Panel open on it:* the panel follows the existing rule (non-empty to empty while Style becomes
     Document); an entry chip open on it closes without writing.
   One notice per batch of remote deletes ("Ben deleted 3 selected objects."). The label is the participant's;
   when unknown it reads "Someone".
3. **A rejected update** (criterion 25): one `role="alert"` notice, 8s: "A received update was rejected."
   then the Resyncing state of section 6. Never a dialog; never repeated for the same cause.
4. **Resize and fit with another person connected** (criterion 31): the existing refusal path of the
   Document section (the `FitResult` "blocked" and the size field's message line): "Another person is
   editing." The controls are not dimmed (the refusal-on-use rule, not hidden/disabled).
5. **Same group move** (criterion 12): the object ends in one group; no message.

## 8. Undo messaging

Ctrl+Z and Ctrl+Shift+Z behave as in a document that is not shared (criteria 21, 23): own steps only; the
History tab's button tooltips and canvas notices keep today's texts. Specific to sharing:

- **Skipped step** (criterion 22): canvas notice, status, 5s, at most 70 characters: "Skipped: Ben deleted
  that object." (label cut to 16 characters; "Someone" when unknown). The undo stack keeps the skipped
  step's neighbours; the next Ctrl+Z reverts the previous own step.
- Nothing says "undoes only your changes" anywhere (no operating tip).
- **History tab:** a step of another participant shows that participant's disc (16px) in front of its
  name, and "Another session" when no hello matches (ADR 0031 decision 4). Undo/redo buttons and Ctrl+U act
  on own steps only; a step of another participant has no undo row action.

## 9. Privacy facts that must be visible

- At the moment data leaves the computer: the Inline confirm sentence of 2.2, with the server's address.
- The invite sentence of 2.3 (what a link grants).
- The Share tab lists exactly who can read the document: the participant list with fingerprints.
- The server address is shown read-only while shared.
- The name the maker sets is visible to every participant; the field label says "shown to others".
- Presence is sealed like edits; nothing in the UI claims more or less (no padlock icon, no "secure" badge:
  the claims are in the text above).
- Removal (slice 1): the Inline confirm sentence of 2.4, "Removing does not reach back: Ben keeps what is
  already on that computer and gets no later changes." (ADR 0010 §8), and the notice on the removed device.

## 10. Tokens and components

Reused unchanged: `--toolbar-bg`, `--toolbar-icon`, `--toolbar-icon-active-bg/-fg`, `--editor-accent`,
`--panel-muted-fg`, `--field-invalid`, `--selection-casing`, `--panel-elevation-shadow`. Components reused:
`PanelStrip` tab, `EntryField`, `PanelRadioGroup`, Inline confirm (two- and three-button), the outline text
button, `Tooltip`, the canvas notice slot, the History tab's row look, `FitResult` refusal.

New (to be entered into `docs/design-system.md` when this ships; contrast measured in review):
`--peer-1` to `--peer-6` (section 4); components: **status dock** (shared with 0064), **Collaboration pill**,
**avatar disc**, **remote pointer and label chip**, **remote selection box**, **participant row**. No new
modal, no new toolbar.

## 11. Keyboard and accessibility

| Action | Key |
|---|---|
| Open the Share tab | Tab to the strip, or Tab to the pill and Enter; no chord in slice 1 |
| Share, Invite, Create link, Copy, Done, Join, Cancel, Remove (admin) | Tab and Enter or Space; Enter in the link or name field submits; Escape closes an open block or the removal confirm |
| Undo, Redo | unchanged |

- Tab order in the tab follows the screen order: Server, Name, Share/Invite, link field, Copy, Done, Remove buttons, Join.
  The participant list is read by browsing; its only Tab stops are the Remove buttons of an admin, one per
  other participant, in row order. Remove opens the confirm with focus on Cancel.
- Pill: one Tab stop after the canvas (below the Machine pill's when both exist in DOM order: collaboration
  first, then machine), named with its full state. Disc, glyph and text: no information is colour only.
- Live regions: polite for connection and join phases and the notices; assertive for refusals and the
  rejected update. Remote pointer movement is never announced. The notice for a remote deletion is
  announced politely, once per batch.
- Targets: 28px buttons (Remove included; 36 coarse), discs 24px (not interactive), pill 32px high.
- Reduced motion: the `RefreshCw` glyph is static; no other motion exists.
- A participant's label is user text and is always rendered as plain text (never as markup) and cut at 24
  characters; the colour never carries the identity alone.

## 12. Empty and error states (checklist for the review)

Remove while offline; removing a participant who is offline; the removed device with unsaved edits; No server set; server unreachable at Share; unsaved document; Share failed after the confirm; link used,
expired, invalid; join waiting for a member, cancelled while waiting; join over unsaved changes; two
participants with one colour; label of 24 characters and an emoji; participant disconnecting mid-drag;
remote delete during a drag, a text edit, a Node edit, with an entry chip open; server restart while open
(criterion 20, pill goes through Reconnecting and back with no notice if under 5 s); 1,000 objects and 4
participants (the overlay is drawn from presence data only, 4 pointers and 4 boxes per frame at most).

## 13. Open UX questions (each with a default)

- **SHARE-PLACE:** a Share tab with a pill (this note) / a block in the File tab. Default: tab and pill; the
  File tab would grow past the 800 x 600 fold and its "append, never move" rule would move Quit.
- **STOP-SHARING:** a "Stop sharing" action. Not in the criteria; default none in slice 1 (removing another participant is in slice 1,
  2.4; leaving is slice 2).
- **INVITE-REVOKE:** revoke a link before it expires. Default none in slice 1; the expiry choice (1 h, 24 h,
  7 days) is the control.
- **JOIN-SAVE:** a joined document opens Untitled and dirty (this note) / asks for a place at once. Default:
  Untitled and dirty, no dialog in the middle of a join.
- **NAME-SHARED:** one "Name shown to others" for sharing and for pairing machines (0064). Default: one, in
  the Share tab.
- **LABEL-ALWAYS:** pointer labels always visible while the participant is online (this note) / only while it
  moves. Default: always; 2 people is the demo.
- **BACK-ONLINE:** the "Back online." notice after more than 5 s offline. Default: yes.
- **EXPIRY-CHOICES:** 1 hour, 24 hours, 7 days. Default 24 hours (criterion 2 Proposal).
- **FOLLOW:** following another participant's viewport is out of scope (spec); not designed, no cue reserved.
