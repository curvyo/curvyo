# UX notes: 0158 slice 1, Pressure Pencil

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
unchanged.
