# UX notes: 0158 pen display (slice 1 Pressure Pencil, slice 2a Pen window, slice 2b Pen layout and Follow view)

UX engineer, 2026-11-27. Slice 1 only (criteria 1 to 17 of `specification.md`, Frage 414 A). Checked
against `frontend/src/components/ToolRail.tsx`, `frontend/src/workbenches/builtin.tsx`,
`frontend/src/components/PenModeBar.tsx`, `frontend/src/App.tsx` (overlay row), the width editing
rows of `0110`/`0115` and `docs/design-system.md`. The design-system rows "Pencil tool", "Pencil bar",
"Pencil eraser" and "Tool rail architecture" are updated to match these notes. Own choices are
marked **(Proposal)**; the customer can veto them at the demo.

## 1. Rail slot, glyph, key, tooltip

- **Slot: the eighth button of the Tools card, after Text.** The 0032 rows placed the Pencil seventh,
  after Polygon/Star; Text (`0074`) took that slot in ship order, and its row already says "the Pencil
  takes the slot after it". A new creation tool appends at the bottom of the Tools card; no built
  button moves. Not in the "More tools" card (Offset, Measure, Test grid, Generators): those are path
  and job tools, the Pencil is a creation tool beside the Pen.
- **Glyph:** Lucide `PencilLine`, 20px, as the other tools (`Pencil` is the History tab's Change kind,
  `PenTool` is the Pen, `LineSquiggle` is the Pen bar's Freehand mode).
- **Key `P`:** free. `letter_action` in `curvyo-editor-wasm/src/session/key_table.rs` binds b, n, e,
  o, t, i, `*`, r, s, m, k, h, v; nothing in the frontend reads P; the sketch rail has no letters. A
  tool letter in every state, like `B`, `N`, `E`, `T` (no selection entry behind it). Ignored under the
  general letter gate (drag, chip, focused control, unfinished Pen path, modifiers, repeat).
- **Tooltip and name:** "Pencil tool (P)" in every state (P has no Select-tool meaning, so no "Esc, P"
  variant; the old row's "Esc, P" is dropped). No hint line (customer rule, no operating tips).
- **Workbenches:** drawing and output pages (`vectorRail` serves both). Not on sketch pages (own rail).
- **Rail height.** The Tools card becomes 8 x 44 + 4 = 356px, ending 368px below the viewport top; the
  Group, More tools and Import cards move down 44px (the announced exception (2) of "Tool rail
  architecture"). Column A then ends 624px below the top off laser pages (580 today) and 668 / 712px on
  a laser page with Test grid / Generators, past the 582px of the 640px minimum window. Today column A
  has no scrolling (no `overflow-y` in `ToolRail.tsx`), so at the minimum window Import is cut off.
  **(Proposal)** build the existing "Too short" rule for column A with this slice: the Tools card stays
  fixed, the cards below it scroll inside the column (`overflow-y: auto`, thin scrollbar, a focused
  button scrolled into view). This is the rail rework of `docs/technical-debt.md` ("Rails scroll and a
  single column on small screens", customer 2026-11-23) reduced to column A; the alternative is to
  accept the overshoot as Frage 403 did for column B. **Needs the lead** (see report).
- **Keyboard on the rail:** unchanged model; the Tools card keeps one Tab stop per tool (8 stops), the
  roving-toolbar change stays a separate decision.

## 2. Pencil bar

Shown whenever the Pencil is the active tool, for a mouse too (the 0032 rule "absent until a pen was
seen" is withdrawn: a mouse needs Smoothing and Simulate pressure). The pressure readout of 0032 is
gone (the Pen probe of slice 0 shows it). `role="toolbar"` "Pencil options", the look of the Pen bar
(pill, `--toolbar-bg`, panel shadow, rows of 36px, 28px controls, `text-sm`, groups 25px apart with the
1px 20px 25% divider). **Placement:** the Pen bar's place (`PEN_BAR_PLACE`: centred from a 1100px wide
canvas, right-aligned below it, so it clears the page name label).

Order, left to right **(Proposal)**: the control every stroke uses first, then the switch, then the
group the switch governs, so turning the switch never moves anything left of the pointer.

| # | Control | Type (design system) | Default | Range, steps | Accessible name |
|---|---|---|---|---|---|
| 1 | Smoothing | value field of the Pen bar's Freehand Smoothing (`TraceField` / `ValueField`, label inside left, `%` inside right, 150px) | 40 % | 0 to 100, arrows 1, Shift 10 | "Pencil smoothing" |
| 2 | Simulate pressure | `ToolbarSwitch` (label then switch, the whole row is the target) | off | | "Simulate pressure" |
| 3 | Min, Max | two value fields as Smoothing, 112px each, 4px apart, labels "Min" and "Max" inside left, `mm` inside right, two decimals | 0.2 mm, 2 mm | 0.05 to 20 and 0.1 to 50; arrows 0.1, Shift 1, Ctrl 0.01 | "Minimum width", "Maximum width" |
| 4 | Curve | segmented text group (`ToggleGroup` bar items, 3 x 52px): Soft, Linear, Firm | Linear | | `role="radiogroup"` "Pressure curve" |
| 5 | (state) "No pen pressure" + "Pen probe..." | muted state label, then a 28px text button in the Edit width button look | not shown | see section 4 | label `role="status"` |

- **Min and Max.** Max is never below Min: a Max typed or dragged below Min is raised to Min and the
  field shows the raised value (0032); a Min above Max raises Max to the same value, so the value the
  maker just set is kept **(Proposal)**. Typed values: Enter commits, Escape restores, an invalid entry
  outlines the field with the field's own message, as every value field.
- **Smoothing** is the Pencil's own session value, not shared with the Pen's Freehand Smoothing
  **(Proposal)**: Frage 414 A keeps Freehand untouched, and a shared value would make one tool change
  the other.
- **When Curve, Min and Max are dimmed.** Dimmed (40 %, `aria-disabled="true"`, still focusable and in
  the Tab order, no hover ground, no edit, value still shown), not hidden, so the bar never changes width
  under the pointer. Dimmed exactly when the next stroke cannot use them: Simulate pressure is off **and**
  either no pen has been seen in this session (mouse only) or the last pen stroke had no pressure.
  Otherwise active (a pen seen and not yet shown to lack pressure, a pen stroke with pressure, or
  Simulate pressure on). Their tooltip gets the unavailable line "Needs pen pressure or Simulate
  pressure." (the reason line of a dimmed rail button, "Laser pages only.", not an operating tip).
  Smoothing and Simulate pressure are never dimmed. With a pen that has pressure, Simulate pressure
  stays operable and is ignored (criterion 8); it is not dimmed, because the same session can switch to
  a mouse.
- **When a change applies:** from the next stroke (criteria 4, 9); a change during a stroke is not
  possible because the bar takes no focus and no pointer while a stroke runs.
- **Lifetime:** session state, never in the document; kept across New and Open for the app run
  **(Proposal: these settings belong to the hand, not to the drawing)**; back to the defaults at the
  next app start.
- **800 x 600 (overlay row about 356px, panel open).** Wraps by whole groups: row 1 Smoothing and
  Simulate pressure (150 + 25 + about 160 + padding, about 351px), row 2 Min and Max (228px), row 3
  Curve; 108px. Row 4 holds the state group when it shows (144px). Measured at build in the browser
  build; if row 1 does not hold both, the Smoothing field shrinks to 136px (not the Pen bar's). The
  wrap is accepted, as for the 0032 bar: the bar stays inside the overlay row and never runs under the
  panel. At 1440 x 1000 it is one row of about 790px (about 1010px with the state group).

## 3. Drawing: cursor, preview, release, Escape

- **Cursor:** `crosshair` on the canvas while the Pencil is active, for mouse and pen, hovering or in
  contact (as the 0032 row). The arrow over the bar and the rail. A hovering pen moves the pointer as a
  mouse does and starts nothing (criterion 11); the Pencil draws no hover highlight on objects (it does
  not pick them).
- **Live preview (criterion 10), row "Live preview outline":** with pressure, or with Simulate pressure
  on, the hollow 1.5px `--preview-new` outline of the variable-width stroke a release would commit,
  white casing over filled artwork, no fill; without pressure and without simulation the 1.5px blue
  line of the centre line. The preview turns from line to outline in the frame the first usable pressure
  arrives (the first sample that is neither 0 nor 0.5; with simulation, from the second sample, when a
  speed exists). At least one update per animation frame up to 2000 samples.
- **Release:** in one frame the preview goes and the black object appears within 0.15 mm of it, selected
  as a finished Pen path is; the tool stays the Pencil, so the next stroke starts at once. Undo step
  "Pencil stroke".
- **Escape** during a stroke: the preview goes, nothing is written, the rest of the contact is ignored
  until the pen lifts (no new stroke starts mid-contact). The first step of the Escape cascade; a second
  Escape leaves to the Select tool, as for the Pen.
- **Hidden or locked target layer:** on the press nothing starts and the Pen's notice shows ("Layer Cut
  is locked. Unlock it or choose another layer." / "... is hidden. Show it ..."), no preview.
- **Too short** (fewer than 2 samples or under 0.1 mm, criterion 15): nothing is created, no notice
  **(Proposal)**: a tap of the pen is common and a notice per tap would nag. The selection is unchanged.
- **Right-click** (often the barrel button in the driver) does nothing in the Pencil **(Proposal)**;
  barrel bits are carried and unused (criterion 11).

## 4. States and messages

- **No pen pressure.** After a pen stroke (pointer type pen) that had no pressure (criterion 7), the bar
  shows its last group: "No pen pressure" (14px, `--panel-muted-fg`, a state label, `role="status"` so it
  is announced once) and a 28px text button **"Pen probe..."** that opens the probe page of slice 0
  (`openPenProbe`, the same page as the File tab's button). The group stays while the state lasts (for
  the session) and goes after the next pen stroke with pressure. No alert, no dialog, nothing on the
  canvas. The stroke itself is the uniform stroke of criterion 7 (or the simulated one).
- If the platform reports the pen as a mouse (WebKitGTK without the native route, `adrs.md` note), the
  app cannot tell a pen is there; the stroke is a mouse stroke and no state shows. The Pen probe stays
  reachable from the File tab.
- **No other new messages.** Erasing nothing writes nothing and says nothing.

## 5. Eraser end (criterion 12)

- **Cursor:** while the eraser end is in range over the canvas, the eraser cursor: a 16px circle (the 8px
  hit reach as a radius) in 1px black with the white halo of the Pen cursors, hotspot at its centre, and
  a 12px Lucide `Eraser` glyph at its lower right, so it is not read as a crosshair **(Proposal)**;
  `crosshair` where custom cursor images are ignored. Back to the crosshair when the tip end returns.
- **Before contact:** the object the eraser would delete (8px rule of `0014`, topmost hit, hidden and
  locked objects never) shows the Select tool's hover box (`--hover-box`), as the criterion says.
- **During contact:** every object the eraser path has come within 8px of keeps the hover box, the set
  grows as it moves; no trail is drawn **(Proposal)** (the highlighted objects are the answer to "what
  goes", and a trail would be a new overlay kind). The render side needs the hover box for a set, not
  one object.
- **Lift:** the highlighted objects are deleted in one step "Erase"; they leave the selection, the rest of
  the selection stays **(Proposal)**. Nothing touched: no commit. **Escape** during contact drops the
  highlight and deletes nothing.
- Only with the Pencil active and only on the canvas; over the bar or the rail the eraser end presses like
  the tip.

## 6. Pen ergonomics in this slice

- **Targets:** the bar's 28px controls in 36px rows and the 40px rail buttons are above the 24px
  minimum; on the Kamvas 13 (about 166 ppi) a 28px control is about 4.3 mm, enough for a pen. No pen-size
  variant in slice 1 (that is the Pen layout of slice 2).
- **No hover-only affordances:** every bar control is named on the control; tooltips repeat names and the
  dimmed reason only. Nothing is revealed by hover.
- **No double-click needed:** drawing is press, move, lift; a tap on a value field types, a drag changes
  it; the value fields already take the 8px slop for any pointer that is not a mouse
  (`useSliderPointer`), so a pen tap does not nudge a value. On Linux a pen the webview reports as
  "mouse" gets the 3px slop; accepted for slice 1.
- **The bar takes no focus** from a press on the canvas; while a stroke runs the bar does not react to the
  pointer.

## 7. Accessibility and keyboard

- Tool: `P`; rail button `aria-pressed`, name "Pencil tool (P)", focus ring of the rail. Drawing is
  pointer-only, like the Pen's freehand; nothing else in the tool needs a key except Escape.
- Bar: Tab order after the canvas, as every bar; inside, one Tab stop per control: Smoothing, Simulate
  pressure (Space), Min, Max, Curve (one stop, Left and Right choose, no wrap, Home, End), Pen probe...
  (Enter, Space). Value fields: arrows (Shift 10x, Ctrl 1/10), Home, End, Enter, F2 or a digit to type,
  as the Pen bar's Smoothing. A mouse press on a bar control hands focus back to the canvas,
  as the Pen bar does; a key press keeps it.
- Dimmed fields: `aria-disabled` plus the reason in the tooltip and `aria-description`. Contrast as built
  components (no new colour; the eraser highlight reuses `--hover-box`).
- Undo names "Pencil stroke" and "Erase" in the History tab.

## 8. Forward look: the fixed Pen layout (slice 2)

The Pen layout (Frage 413 A, fixed) will show a few tools and a slim property strip with large targets.
So that slice 1 does not paint us into a corner: the Pencil settings live in the session, not in the bar
component, so a second bar or strip can show the same values; the bar controls are the shared
components (`ValueField`, `ToolbarSwitch`, `ToggleGroup`) that the strip can render at a 40px target size
instead of copying them; the bar order (Smoothing first, width settings after the switch) is the order
the strip keeps; the eraser needs no UI at all, which suits a canvas-only toggle. An **Eraser tool
button** in the Pen layout (the spec's list) would set the Pencil's eraser behaviour for the tip, for
pens without an eraser end. The "No pen pressure" state and the Pen probe link move into the strip
unchanged. *(Superseded for slice 2 by sections 9 and 10: the strip renders the shared components
at 48px, not 40px.)*

# Slice 2: Pen window

UX engineer, 2026-11-27. Parts 2a (criteria 1 to 20) and 2b (criteria 1 to 9) of `specification.md`,
ADR 0096, Fragen 409 A, 413 A, 415 A. Checked against `frontend/src/App.tsx` (overlay row, no menu),
`ToolRail.tsx`, `workbenches/builtin.tsx`, `PencilBar.tsx`, `NodeToolbar`, `panel/FileTab.tsx` and
`PenProbe.tsx` (the diagnostics slot), `lib/viewTool.ts` (the unused Pan and Zoom state of `0093`),
`session/navigation.rs`, `session/pages.rs` and `session/layers.rs` (active page and active layer are
session state, so per window), and `docs/design-system.md`. Design-system rows added or amended:
"Pen display section", "Pen window", "Pen rail", "Pen window states", "Pen window steps", "Pen
layout", "Pen strip", "Pen layer pick", "Canvas only", "Follow view", "Pen window frame", token
`--pen-view-frame`. Own choices are **(Proposal)**. Customer facts this depends on: handedness (the
rail side) and what the Kamvas pen's barrel buttons send (section 9.6).

## 9. Part 2a: Pen window

### 9.1 "Open pen window" in the main window

- **Place: the File tab, a "Pen display" section in its diagnostics slot**, above the existing
  **Pen probe...** button (end of the tab, after Quit and Extensions) **(Proposal)**. Reason: there is
  no menu since `0051`, the rail holds tools, and the File tab already holds the other pen-display
  command. Desktop only; the browser build draws neither the section nor the button (criterion 1).
  No key, no chord (**Proposal**: once per session; a chord can come with the shortcut review).
- **Look:** the File tab's outline text button, 244 x 28px (36px coarse), then the Pen probe
  button 8px below as today. `section` named "Pen display", no heading text.
- **States of the one button (label is the state):**

| State | Label | Behaviour |
|---|---|---|
| Closed | **Open pen window** | Creates the window labelled `pen` |
| Opening (press to the pen window's first frame) | **Opening pen window...** | `aria-busy`, presses ignored, not dimmed |
| Open | **Show pen window** | Raises, un-minimises and focuses the pen window; no second window (criterion 1) |
| Failed (no first frame within 10 s, or the window could not be made) | back to **Open pen window** | Canvas notice, alert, 8 s: "The pen window could not be opened." |

- A close of the pen window by the window manager or a crash (criterion 10) turns the label back to
  **Open pen window** in the frame the main window learns of it; no notice.
- Close of the project (criterion 12) closes the pen window; New and Open keep it, it reloads.

### 9.2 What the pen window shows (2a)

- **Canvas** filling the window, the default tool **Pencil** (**Proposal**; also 2b's edge case),
  the view on open: **Fit page** of the main window's active page (criterion 12 for New and Open;
  **Proposal** for the first open: the open handshake must carry the main window's active page id,
  architect). No rulers, no status bar, no Properties panel, no page list, no collaboration pill,
  no presence pointers.
- **The pen rail** (left, design-system row "Pen rail") built at its final 48px size already in 2a
  **(Proposal: building it at 40px first and resizing in 2b is wasted work)**. Cards, top to bottom:
  1. **Tools:** Pencil (`PencilLine`, "Pencil tool (P)"), Select (`MousePointer`, "Select tool (S or
     Esc)"), Node (`MousePointer2`, "Node tool (N)"). Radio, `aria-pressed`, the rail look.
  2. **Edit:** Undo (`Undo2`), Redo (`Redo2`), Delete (`Trash2`). Commands, not tools. Undo and Redo
     act on this window's own steps (criterion 6); tooltip "Undo: Pencil stroke (Ctrl+Z)", "Redo:
     Erase (Ctrl+Shift+Z)"; dimmed (`aria-disabled`, 40 %, still named) when this window has nothing
     to undo or redo. The "Undid: ..." notice of `0020` shows in the pen window's notice slot.
     **Delete** does what the Delete key does in the active tool: Select and Pencil delete the
     selected objects (the Pencil leaves its last stroke selected, so "draw, Delete" removes it),
     Node deletes the selected nodes or the selected width point; dimmed when there is nothing to
     delete; one step "Delete" as the key.
  3. **View:** Zoom in (`ZoomIn`), Zoom out (`ZoomOut`), Fit page (`Fullscreen`), Pan (`Hand`). See
     9.3.
- **Why Delete and the View card (contradiction with the spec's list "Select, Node, Pencil, Undo,
  Redo"):** with a pen alone there is no Delete key, no wheel and no Space. Without Delete an
  older stroke can only go by Undo of everything after it; without zoom and pan the window shows
  the fitted page only and fixing a node at 10 mm scale is not possible. Both are needed in 2a for
  "draw and fix a stroke with the pen alone" **(Proposal; the PO adds them to 2a criterion 1 or a
  new criterion)**.
- **The active tool's bar**, in the main window's overlay row place (top, right of the rail):
  - Pencil: the Pencil bar of slice 1 unchanged (28px controls; 48px comes with the strip in 2b).
  - Node: the Node bar without the Centre lines button (it opens a main-window bar with an info
    page) **(Proposal)**; Insert, Delete, the convert buttons, Line, Curve, Join, Split, Close path
    and the width edit toggle stay. Insert (between selected nodes) is the pen route to a new node,
    no double-click needed.
  - Select: **no bar** in the pen window **(Proposal)**. The Select bar holds align, operation kinds,
    Trace, Recognise, tiling, place-on and text controls that open main-window bars and panels;
    the pen window's Select is for picking and moving strokes. Contradiction with "the active
    tool's own bar" of 2a for Select only.
- **Keys** while the pen window has focus: the shared ones work (Ctrl+Z, Ctrl+Shift+Z, Delete,
  Escape, Space-pan, P, S, N, Ctrl+S which saves in the main window, criterion 9). Letters of tools
  that are not in the pen window (B, R, E, T, I, O, M, ...) and Ctrl+O, Ctrl+N, Ctrl+I do nothing, no
  notice (the `0093` precedent for foreign tool keys) **(Proposal)**.
- **Layer and page the pen draws into:** the pen session's active layer (front-most until a
  selection sets it, as `session/layers.rs` does; the shared selection therefore sets it from the
  main window too) and its active page (a tap on another page's frame activates it, as today).
  2a shows neither; 2b adds the layer (section 10.3).
- **Sketch pages** in the pen window: shown, but the pen tools act on drawing and output pages
  only; a press on a sketch page shows the notice "Sketch pages are edited in the main window."
  **(Proposal)**.

### 9.3 Pan and zoom with a pen alone

- **Zoom in / Zoom out:** one tap zooms by a factor of 1.414 (two wheel notches; four taps = 4x)
  about the centre of the free canvas (right of the rail, below the bar), range 2 % to 8000 % as
  the wheel; dimmed at the end of the range **(Proposal)**.
- **Fit page:** the active page whole in the free canvas, centred, with 24px margin; the session's
  page-and-guides fit (`show_page_and_guides`) without its "only when outside" condition
  **(Proposal)**.
- **Pan (Hand)** is a latching toggle (the `lib/viewTool.ts` "pan" state, now per window): while
  pressed, a tip drag pans in any tool and the tool's bar and state stay; cursor `grab` / `grabbing`
  (row "Pan cursor"). It releases on a second tap and when a tool is chosen, so the next tip touch
  draws as the maker expects **(Proposal)**. A pen hovering does not pan.
- **Barrel button:** no new binding in slice 2 (out of scope). If the driver sends the middle mouse
  button, a barrel-held drag already pans (the existing middle-button pan); a right button does
  nothing in the Pencil (slice 1). The Hand toggle is the route that works with every driver.
- No zoom readout in the pen window (the canvas is the product; the status bar stays in the main
  window).

### 9.4 Title, selection, notices

- **Window title:** "Pen window — Poster.curvyo — Curvyo" ("Pen window — Untitled — Curvyo", in a
  workspace "Pen window — Box job / Sides / Lid — Curvyo"), never the "*" dirty marker: the main
  window owns the unsaved state (criterion 9) **(Proposal)**.
- **The other window changed the selection:** no notice, no flash, no animation. The selection box
  and handles appear in the frame the selection arrives, as for a local selection, and the main
  window's Properties panel shows the new selection; that is the cue (a notice per stroke would nag:
  every Pencil stroke selects itself). **(Proposal)** One guard: while a value field in the
  receiving window is being typed in or dragged, or a chip is open, the arriving selection waits
  until that field commits or is cancelled, so a typed value never lands on another object; this
  holds the selection for longer than "a frame later" of criterion 4 (PO to confirm).
- **Notices** in the pen window use its own canvas notice slot (centred in the free canvas, 12px
  above the bottom). Texts: criterion 15 "The text was deleted in the other window.", the Pen's
  locked or hidden layer notices, "Undid: ...", errors of criterion 13 (never a dialog).

### 9.5 "Waiting for the main window" and "Out of sync, reloading" (row "Pen window states")

Both use the Document preview's pause pattern without its blue frame (it means "preview"):

- the rail and the bar at 40 % and `inert`; the canvas keeps drawing the document as it is; the
  arrow cursor; pen samples and presses on the canvas are ignored; a stroke or drag in progress is
  cancelled as by Escape (nothing written, criterion 8);
- the persistent banner in the pen window's notice slot, `role="status"`, announced once:
  **"Waiting for the main window"** (criterion 13) or **"Out of sync, reloading..."** (criterion 18);
  no button (the dialog is on the other screen, the pen cannot reach it);
- ends in the frame the main window's dialog closes, or the full project has been shown; after a
  reload the tool and view stay, the selection is empty, and this window's undo starts empty (the
  main window keeps every step); one notice "Reloaded from the main window." **(Proposal)**.
- Canvas-only (2b) does not hide the banner.

### 9.6 History tab, first run, placement

- **History tab (main window):** a step of this device's pen window shows **"Pen window"** in the
  author place of line 2 (where "You" stands), glyph and operation name unchanged ("Pencil stroke,
  1 path, Pen window, 12:03"); a collaborator's pen window step shows **"Pen window of Marc"**
  (criterion 17). The main window's Undo and Redo (History tab and keys) act on its own steps only,
  so their tooltips name its own last step. The pen window has no History tab.
- **First-run hint "drag this window onto your tablet": none** (customer rule of 2026-10-16, no
  operating tips). The window opens at the remembered place, or where the compositor puts it; the
  maker drags it once and it comes back there (criterion 16). Nothing explains the remembered
  placement: it is consistent, so it needs no words. If the customer wants a hint, that is their
  call (question in the report).
- **Remembered placement:** size always, position and screen where honoured, and the **maximised
  state** (an additive field of `pen_window.json`; on a 1920 x 1080 pen display the maker will
  maximise it) **(Proposal, criterion 16 lists size and position only)**. On Wayland the window
  opens where the compositor puts it every time; no message.
- **Default size:** 1200 x 720 instead of "about 960 x 600" **(Proposal)**: the 2b rail needs 640px
  of height in one column (section 10.2); 720 also fits inside a 1366 x 768 screen.
- **Closing the pen window** is the window manager's close button (the pen can tap it); no in-app
  Close button **(Proposal)**.

## 10. Part 2b: Pen layout and Follow view

### 10.1 The fixed layout (Frage 413 A)

One layout, no setting, no customise button, nothing stored about it. Every control is a 48 x 48px
target (`size-12`), 4px inside a card, cards 56px wide, 8px apart, inset 12px from the window's left
and top edge (the rail's `--rail-inset`). At about 166 ppi (Kamvas 13, scale 1.0) 48px is about
7.3 mm. Chrome sits at the top and left only; the bottom and the right edge, where a right hand
rests on a tilted display, carry nothing interactive (only the notice banner, which takes no
input). The 12px inset keeps targets off the bezel edge, where pen accuracy is worst, and off the
desktop's hot corner (GNOME Activities at the top-left screen corner).

| Place | Content |
|---|---|
| Rail, left column | Tools: **Pencil, Eraser, Select, Node** (200px). Edit: **Undo, Redo, Delete** (152px). View: **Zoom in, Zoom out, Fit page, Pan, Follow view** (248px) |
| Strip, top, right of the rail | The Pen strip (10.3), 56px high, left-aligned 12px right of the rail |
| Top-right corner | The **Canvas only** toggle, its own 56 x 56px card, 12px from the right and top edges |
| Bottom centre | The notice slot (no controls) |

- **Order (Proposal):** by frequency for drawing, Pencil first (the main window keeps Select first;
  the pen window is its own curated layout). Eraser next to the Pencil because the two alternate.
- **Height:** 12 + 200 + 8 + 152 + 8 + 248 = 628px plus a 12px bottom inset: 640px. At 1920 x 1080
  and at 1366 x 768 (about 730px inside the title bar) one column holds it. Below 640px the View card moves to a second column beside the first, top-aligned (the
  main rail's two-column model, not a setting) **(Proposal)**.
- **Width at 1366 x 768:** rail 80px, strip about 560px, corner toggle 68px: about 660px stay clear
  in the top row; the canvas runs under the floating chrome as in the main window, so it keeps 94 %
  of the window width (criterion 8's 60 % holds by a wide margin; the criterion is better read as
  "no control clipped or overlapped").
- **Handedness: the rail stays left for everyone, no mirror switch** (a switch is the configuration
  the customer declined). Reasons: the main window's rail is left (one model for the maker's eye);
  the Kamvas 13's express keys sit on its left bezel in the default orientation, where a right-hander's off hand already rests;
  the drawing hand of a right-hander covers the bottom-right, which stays empty. If the customer is
  left-handed, the fixed layout is mirrored once, for everyone, as a product decision (a fact to ask,
  not a setting) **(Proposal)**.
- **No hover-only affordance:** every control is named by its glyph and accessible name; tooltips
  (pen hover shows them after 400ms, `side="right"` for the rail, `bottom` for the strip) repeat the
  name and the key only. **No double-click:** every function is a tap or a drag; Insert node,
  Delete and Fit page replace the double-click and key routes; double-clicks the shared Select tool
  understands (enter a group, edit a text) may still work, nothing depends on them.
- **Palm and edges:** the Kamvas 13 has no touch as far as known, so a resting palm triggers
  nothing; a touch pointer is ignored by the Pencil (slice 1 criterion 11) and by every pen-layout
  control (`pointerType === "touch"` presses do nothing) **(Proposal)**, so a later touch model does
  not need a rework.

### 10.2 Rail controls added in 2b

- **Eraser** (`Eraser`, "Eraser"): a radio item of the Tools card. On: the tip erases as the eraser
  end does (criterion 2, row "Pencil eraser": cursor, hover box set, "Erase" on lift); the strip stays
  the Pencil's, so nothing jumps between the two. A second tap on the pressed Eraser returns to the
  Pencil **(Proposal)**. Not remembered: a new pen window starts with the Pencil. The pen's real
  eraser end works in the Pencil and in the Eraser.
- **Follow view** (`Link`, "Follow view"), a toggle at the end of the View card, `aria-pressed`, the
  active-tool look when on. Off by default. Tap on: this window takes the main window's centre and
  zoom at once; afterwards every pan, zoom, Fit page and Zoom button in either window moves the other
  (criterion 5). Tap off: both keep their views. Remembered in `pen_window.json`. The main window has
  no Follow control **(Proposal)**; the frame (10.5) shows it.

### 10.3 The Pen strip (challenge of the PO's colour, Max width, Smoothing)

- **Proposal: Layer, Max width, Smoothing; colour leaves the strip.** In Curvyo the colour is not the
  machine instruction (`0101`, customer 2026-11-10: "away with the colours"); a new stroke takes the
  operation of its layer or page (slice 1 criterion 13), so the layer decides what the laser or plotter
  does with the stroke. Without a layer control the pen window draws into whatever layer the last
  selection made active, which the maker cannot see on the tablet. Colour stays the current stroke
  style; it is changed in the main window's Style panel. (PO to decide; if colour stays, it is a
  fourth item: a 48px swatch button that opens the Style panel's colour picker as a popover below it.)
- **Look:** a pill as the Pencil bar (`--toolbar-bg`, panel shadow), 56px high, items 48px high,
  groups 25px apart with the 1px 20px divider, `role="toolbar"` "Pen options". Order: Layer, Max
  width, Smoothing, then the "No pen pressure" state of slice 1 (state label and a 48px "Pen probe..."
  button). Shown with the Pencil and the Eraser; with Node the Node bar's pen subset (below); with
  Select nothing.
- **Layer** ("Pen layer pick" row): a 48px button, 180px wide, the layer's name (ellipsis) and a
  chevron; a tap opens a list of the active page's layers, front-most first, 48px rows, the active
  one with the 3px accent bar of the Layers list, hidden and locked layers with their glyph and
  chooseable (drawing on them shows the Pen's notice). Choosing sets the pen window's active layer
  (session state, no step, no document change); a tap outside or Escape closes the list. A page
  without layers: the button reads "Page", dimmed. A selection made in either window moves it, as
  the active layer follows the selection today.
- **Max width and Smoothing:** the slice-1 value fields at 48px height, 150px wide, the same params,
  ranges and defaults (criterion 3); drag to change (8px slop for a pen), tap to type with a
  keyboard. Min, Curve and Simulate pressure keep the window's values and are not in the strip.
- **Node with the pen layout:** the Node bar rendered at 48px with Insert, Make corner, Make smooth,
  Make line, Make curve, Join, Split and the width edit toggle (about 440px); Delete is the rail's;
  Close path and Centre lines are not offered **(Proposal)**.

### 10.4 Canvas only

- The corner toggle (`Maximize2`, "Canvas only"; on: `Minimize2`, "Show tools", `aria-pressed`)
  stays in the **same place** in both states, so the pen goes back to where it pressed. On: rail and
  strip are not drawn, the canvas fills the window, the pen draws with the current tool; off: the
  layout comes back unchanged. Remembered in `pen_window.json` (criterion 4). Undo with the tools
  hidden: one tap on the toggle, then Undo (Ctrl+Z still works on a keyboard).

### 10.5 The pen window frame in the main window

- **Look (row "Pen window frame"):** four corner brackets, 2px `--pen-view-frame` (an alias of
  `--toolbar-icon`, ink: 9:1 on `--canvas-bg`, 5.6:1 on `--pasteboard-bg`), legs 16px, joined by a
  1px solid line in the same ink at 40 %, the white 1px `--selection-casing` on both, and a label tab
  "Pen window" (12px, `--toolbar-bg` pill, ink text) outside above the top-left bracket. Not dashed
  (selection is dashed), not accent (selection, preview), not filled and no shadow (a page),
  not a peer colour (presence). Screen-space, constant at every zoom.
- Overlay only: never in the document, export, hit tests, snapping or machine output; no hover, no
  cursor change; the parts outside the main canvas are clipped, nothing marks an off-screen frame
  **(Proposal)**. Updated in the frame the pen window's view message arrives (criterion 6); a
  minimised pen window keeps its last frame.
- **Toggle "Show pen view":** there is no View menu (`0051`), so it is a switch in the File tab's
  Pen display section, 8px under the **Show pen window** button (the Export options switch look),
  in the tree only while the pen window is open, on each time it opens **(Proposal; contradicts 2b
  criterion 6's "View-menu toggle")**.

## 11. Forward look: where later slices attach (not built)

The radial pop-up palette (later list) opens at the pen tip in the pen window only, from a barrel
button once bindings exist, and holds the Tools card's items plus Undo; it adds no rail card. The
shape builder's separate mode (slice 5, Frage 411 C) is a toggle in the Pen strip after Smoothing
("Shapes"), not a fifth rail tool, so the rail does not grow; the hold-still trigger needs no control,
only the conversion cue on the canvas. The handwriting box (slice 6) is a rail tool "Write" in the
Tools card that docks its box under the strip at the top, away from the resting hand. Each keeps the
48px target, the top-and-left rule and "nothing configurable".
