# Path offset: UX rework concept

Status: accepted by the customer on 2026-10-12 with all defaults; **reworked 2026-10-13** by
`ux-engineer` after the customer tried the build. `specification.md` wins where the two differ.
Mockup: `ux-rework-mockup.html` in this folder (open it in a browser).

## Customer decision 2026-10-13 (fourth round): Alt steps the corner style

Replaces the double-click on a corner handle and the "pending style for one drag" (sections 3 and
11); the specification (note at the top, criteria 13 and 56) has the criteria.

- While a corner handle is dragged, **each Alt press steps the corner style** Round, Miter, Bevel,
  Round. Preview and readout follow in the same frame.
- The **first drag ever starts with Round**; then the **last used style is remembered** (app
  setting like the remembered distance, document-independent, not in the file) and the next drag
  starts with it. The **chip's Corners control reads and writes the same value.**
- The double-click on a corner handle is gone. The center-handle double-click stays.
- Constraints: act on Alt **keydown** only; ignore `repeat`; no key-up needed (WebKitGTK may not
  deliver it); `preventDefault` so Alt does not focus a menu or menubar in the webview; the press
  starts **without** Alt (Alt held at the press is not used, so Linux window managers' Alt+drag
  does not matter; the step needs a fresh Alt keydown after the press). Alt outside a corner drag
  is not touched.
- Defaults pending customer: only a written drag updates the remembered style (Escape, the other
  button, release at d = 0 and a refused release leave it); Alt in the dead zone already steps; Alt
  works with Shift or Ctrl held; the hint is on the corner handle's hint chip, not the tool tooltip.

## Customer decision 2026-10-13 (third round)

Three decisions of the customer, after trying the entry chip; they override the sections below
where they differ (the sections are updated; the specification, note at the top, has the criteria):

1. **Mode mapping.** The default Mode is **Change original** (also the chip's initial Mode).
   **Ctrl = New object** (as Ctrl = copy in Move). Drag: none = Change original; **Shift** = Both
   with Change original; **Ctrl** = New object; **Ctrl+Shift** = New object and Both. The old chord
   "the original takes the dragged side, a copy for the other" is dropped. **Shift alone writes**
   (default pending customer): the original becomes the outset side, one new object is added for
   the inset side (section 4).
2. **Preview only while it is being judged:** live during a handle drag, and while the entry chip
   is open, showing the chip's values. **No standing preview**: not on tool choice, selection,
   after a release, Escape or closing the chip (section 3).
3. **Modifiers at the double-click on the center handle** preselect the chip: Shift = Direction
   Both; Ctrl = Mode New object; Ctrl+Shift = both; none = the remembered values (section 2.1).

## 2026-10-13: what changed and why

The customer tried the reworked slice on 2026-10-13 and decided (spec, note at the top):

- **No top bar, no Apply button.** The Offset bar (old section 2) is gone. Its values move into an
  **Offset entry chip** at the selection, opened like the Move entry chip from a **center handle**
  (section 2), or by **O** or **Enter**.
- **No status hint.** "Stays a rectangle" / "Becomes a path" is removed with no replacement.
- **No Ends, no open paths.** An open path is not offset and gets no handles (section 8).
- ~~**The preview stands.** With the tool active and a closed shape selected, the result for the
  remembered values is shown at once.~~ Replaced in the third round: no standing preview (section 3).
- **Modifiers while dragging a corner handle:** Shift = Both, Ctrl = New object, default Mode
  Change original (third round; the first 2026-10-13 version had Ctrl = change the original and a
  Shift+Ctrl chord "dragged side plus copy", both dropped) (section 4).
- ~~**A double-click on a corner handle steps the corner style** Round, Miter, Bevel, shown in the
  readout chip.~~ Replaced in the fourth round by Alt while dragging (section 3).

Everything below that the customer did not state is marked **default pending customer** and is
collected in section 11.

**The corner style (fourth round).** The earlier "pending style for one drag" is gone. There is one
**remembered corner style**; a drag starts with it, Alt presses step it during the drag, and a
written drag stores the result (section 3, "Corner style by Alt"). The chip's Corners control shows
and sets the same value.

## Reference tools (as far as I know them)

- **Inkscape:** Path > Outset / Inset step by a fixed amount; Dynamic Offset (Ctrl+J) turns the
  path into an offset object with one handle on the outline. We take the handle, not the conversion.
- **Illustrator:** Object > Path > Offset Path, a dialog (Offset, Joins, Miter limit, Preview).
  Adds a new path, keeps the original. We take "keep the original" as default.
- **LightBurn:** Offset Shapes, a dialog with Distance, Corner style, Direction (Outward, Inward,
  Both) and "Delete original". We take Both and the replace choice, on the canvas.
- **Figma / Affinity:** typed values sit next to the object, not in a dialog. The entry chip is
  Curvyo's version of that (the Move entry chip already works so).

## 1. Where the tool lives: the "Path tools" card

Unchanged since 2026-10-12. A toolbox card **"Path tools"** in rail column A, 8 px under the Group
card, with one 40 x 40 px button, the **Offset tool** (glyph: a solid rounded square inside a
dashed concentric outline). Tools, not commands: the button takes the active-tool look and shares
the one-active-tool rule with the Tools card. `role="toolbar"`, name "Path tools", one Tab stop,
the rail keyboard model; Tab order Tools, Group, Path tools, Path, Boolean. Key **O**, a tool
letter in every state (as B, N, E, P).

Tooltip "Offset tool (O)", muted second line **"Drag a corner handle. Shift: both sides. Ctrl:
new object."** (third round: Ctrl now means New object; the readout and the handle hint chip say
the rest).

## 2. Offset entry chip and center handle (new 2026-10-13)

### 2.1 The center handle

| | |
|---|---|
| Where | The center of the **operand box** (the box the corner handles use, section 3: one rotated object its own rotated box, otherwise the axis-aligned box around the closed operands). Upright, never rotated. |
| Look | The shell of the Move center handle (row "Transform center move handle"): 16 x 16 px screen-space rounded square, 3 px radius, white fill, 1 px `--accent` outline. **Glyph (default pending customer):** not the four-way arrow, because a drag here does not move anything; instead the Offset glyph in small: a 10 px square outline (1.5 px `--accent`, 2 px radius) around a 4 px solid `--accent` square. Alternative: the Move glyph unchanged. |
| States | Idle: as above. Hover: `--accent-hover` over the white fill. **Chip open:** solid `--accent` with a white glyph (the Move rule "the handle stays in its dragging look while the chip is open"). |
| When drawn | Whenever the corner handles are drawn (at least one closed operand selected), **at every box size**: the box is widened to 24 px per axis for placing handles, so the glyph keeps at least 13 px clear of every corner handle. Not drawn during a corner drag (the preview needs the room). Not drawn with nothing selected, a group alone, or only open paths. |
| Hit area | A circle of **12 px radius** around its center (24 px target), always, because in this tool no other handle lies inside the box. Corner handle hit areas (8 px radius, 12 px outside the corner) never overlap it: at the 24 px minimum box the two centers are 29 px apart. |
| Press | The hit area **claims the press**: no marquee, no selection change, no move, no drag. This matters for an unfilled shape, where the center is "empty canvas" and a press would otherwise clear the selection on the first click of the double-click. |
| Double-click | Two primary presses within the system double-click interval, both inside the hit area, neither moving past the 3 px dead zone: **opens the Offset entry chip**. **Modifiers held at the second press preselect options (third round, spec criterion 57):** Shift = Direction Both; Ctrl = Mode New object; Ctrl+Shift = both. A value not named by a held key shows the remembered value; with no modifier all values are the remembered ones. A single click does nothing. |
| Cursor | The built-in `pointer` (not `move`). |
| Hint chip | After 600 ms of rest (row "Transform handle hint chip"): one line, "Double-click, O or Enter: type a distance" (third round, default pending customer: a second muted line "Shift: both sides. Ctrl: new object." tells that the modifiers preselect). |

### 2.2 Opening the chip by keyboard

With the Offset tool active, the canvas focused and at least one closed operand selected:
**O** or **Enter** opens the chip (default pending customer: both keys; the customer named
"O/Enter") with the **remembered values**; modifier keys held with O or Enter preselect nothing
(only the double-click does, 2.1). It is placed exactly as if the center handle had been
double-clicked, with no modifiers. No preview is shown before the chip opens. With no operand
selected, O re-selects the Offset tool and Enter does nothing; no hint is shown (spec criterion 5).
Both keys are gated like every shortcut (not during a drag, not with a chip open, not on key
repeat). O in the Select tool is the tool letter: so from the Select tool with a selection,
**O, O** opens the chip.

### 2.3 Contents and layout

The "Transform entry chip" surface: `--toolbar-bg`, `--panel-elevation-shadow`, 8 px radius,
6 px padding. **168 x 108 px.** Three rows, 6 px apart:

| Row | Content | Size | Notes |
|---|---|---|---|
| 1 | Visible label **"Distance"** (12 px), then the **Distance field** with the fixed `mm` suffix | label box 52 px, 4 px gap, field 100 x 28 px | The field of the Move chip: white ground, 14 px tabular text, right-aligned, 32 px right padding for the suffix. Prefilled with the remembered distance, up to two decimals ("1", "2.5", "0.15"). |
| 2 | **Direction:** a three-segment pill **Out \| In \| Both** | 156 x 28 px, three 52 px segments, 12 px text | The Move mode pill's look (1.5 px `--toolbar-icon` ring; active segment `--toolbar-icon-active-bg` / `--toolbar-icon-active-fg`). |
| 3 | **Corners:** three icon segments **Round, Miter, Bevel**; then, right-aligned, **Mode:** two icon segments **New object, Change original** | Corners 84 x 28 px (three 28 px), Mode 56 x 28 px (two 28 px), 16 px between | Corner glyphs: the stroke Join glyphs (`JoinIcon` of `panel/StyleIcons.tsx`) at 16 px. Mode glyphs: Lucide **`Plus`** (New object) and **`RefreshCw`** (Change original: two arrows in a circle), 16 px, 1.5 px stroke. Not `RotateCcw`: that is the reset glyph of the panel's value fields. Icons only; each segment has a tooltip with its name. **Default segment: Change original** (third round; New object was the default before). **Corners (fourth round) reads the remembered corner style on opening and writes it when another segment is chosen**; it is the same value a corner drag starts with and stores. |

Invalid input and refusals use the Move chip's **message card** (a separate small card under the
chip, above it when there is no room; `CircleAlert` 12 px and one line of 12 px text; the body of
the chip never moves).

Why Corners is in the chip: the keyboard route needs it (Alt is a pointer-drag key), and the chip is where a maker looks for "the settings of this offset" (default pending
customer, spec open question 3).

### 2.4 Placement

The Move chip's rule, adapted so the chip does not cover the result:

1. **Anchor box:** the screen-space axis-aligned box around the operand box **and** the preview
   of the chip's values at the moment the chip opens (with Out or Both the preview is bigger than
   the object). The preview appears with the chip and goes with it (section 3).
2. **Left edge** 16 px right of the center glyph's center; **top edge 20 px below the anchor box's
   lower edge** (not 16: the bottom corner handles reach 13.5 px below the box corner).
3. **Flip:** to the left of the center (right edge 16 px left of it) when it would cross the
   canvas's right edge; **above the anchor box** (bottom edge 20 px above its upper edge) when it
   would cross the bottom edge. The Move chip flips to above the center glyph and covers the upper
   half of the box; the Offset chip does not, because the preview is what the maker is judging.
4. **Clamp** inside the canvas and right of the tool rail (`--rail-right` + 12 = 128 px), as the
   Move chip.
5. **Fixed while open.** Typing, arrow steps or a new Direction change the preview but do not move
   the chip; a bigger preview may then run under it. Pan, zoom and window resize recompute the
   placement from the anchor box at its new screen position.

At 800 x 600 the 168 x 108 px chip fits in every case (canvas about 324 px wide with the panel
open).

### 2.5 Focus and Tab order

- **On open:** focus in Distance, its text selected (so typing replaces it).
- **Tab order:** Distance, Direction, Corners, Mode, and back to Distance. The chip is a **closed
  loop** like the Move chip: Tab never leaves it (leaving would close it by blur). Shift+Tab goes
  backwards. Each radio group is one Tab stop (roving `tabindex`: the chosen item).
- **On close** (Enter or Escape): focus returns to the canvas container. The preview goes with the
  chip (spec criterion 52); nothing stands on the canvas after a write. **O, Enter** again offsets
  the new selection: it changes the same object once more with Mode Change original, and makes the
  next concentric copy with Mode New object.

### 2.6 Keys inside the chip

| Key | Where | Effect |
|---|---|---|
| Digits, `.` or `,` | Distance | Edit the text. `inputmode="decimal"`. No preview per keystroke (`0017` criterion 42). |
| Up / Down | Distance | Step the value by 0.1 mm; **Shift** 1 mm; **Ctrl** 0.01 mm (the panel field rule; a deliberate exception to the entry-chip rule "no stepping with arrow keys", spec criterion 25). The value is rounded to the step, clamped to 0.01 to 1000 mm, and the preview follows at once. |
| Left / Right, Home / End | Distance | Move the caret (the field is text). |
| Left / Right (also Up / Down) | a radio group | Move to the next or previous item and choose it; wraps (the radio group pattern). Home / End: first and last item. Space chooses the focused item. The preview follows at once. |
| Tab / Shift+Tab | anywhere | Next or previous stop in the loop. Leaving Distance with a valid value commits it to the preview. |
| **Enter** | anywhere | **Writes** (2.7). Also with an untouched Distance: the prefilled value is the value to use (a deliberate exception to the entry-chip rule "Enter on unedited text writes nothing", because **O, Enter** with the remembered distance is the fast keyboard route). |
| **Escape** | anywhere | Closes the chip, **writes nothing**. The values chosen in it become the remembered values (default pending customer); a Distance text that is not a valid number is dropped and the last valid value stays. Escape is the first step of the cascade: the selection stays. |
| Ctrl+Z | Distance | The field's own text undo; key events inside the chip never reach the app's undo or the canvas shortcuts (entry-chip rule). |

### 2.7 Enter: writing, and what refuses

- **Valid:** the result for the chip's values is written as **one commit** labelled "Offset"
  (spec criterion 16); the chip closes; the canvas notice of criterion 24 appears; the selection
  becomes what was produced or changed (criterion 23). Direction and Mode combine as the
  modifiers do (criterion 26): Both + Change original = the Shift result (the original becomes the
  outset, one new object for the inset); Both + New object = the Ctrl+Shift result (two new
  objects).
- **Not a number** (empty, "abc"): the chip stays open, Distance is marked (2 px `--field-invalid`,
  `aria-invalid`), its text re-selected, the message card says "Enter a number". The mark clears
  on the next keystroke.
- **Out of range** (below 0.01 or above 1000 mm): the field shows the clamped value, is marked,
  the message card says "From 0.01 to 1000 mm"; nothing is written by this Enter. The mark clears
  on the next keystroke; the next Enter writes the clamped value (spec criterion 6).
- **Refusals** (a group among the operands, criterion 32; an inset that leaves nothing, criterion
  29): nothing is written; the chip **stays open** and its message card shows the refusal text of
  the spec ("Offset does not work on groups. Ungroup first. Nothing was changed.", "Inset is
  empty: nothing is left at 10 mm. Nothing was changed."); the group is outlined (row "Refusal
  outline"). The maker can change a value or press Escape. The error is shown where the eye is,
  not in the bottom notice slot.

### 2.8 Other ways the chip closes

As the Move chip, and always without writing: a **press outside the chip** (the press is not
swallowed: a press on a corner handle starts that drag at once, a press on an object selects it),
a tool switch, a selection change (also by Ctrl+A or undo), the window losing focus. In each case
the chip's valid values become the remembered values, as with Escape (default pending customer).

### 2.9 The chip and a drag

- They never run at the same time. A press on a corner handle closes the chip first (2.8), so the
  drag starts from the values just set: a distance typed as 2 mm is the snap target of that drag
  (spec criterion 11).
- During a corner drag the chip cannot open: O and Enter are gated, and a double-click needs two
  clicks. Opening the chip shows the remembered corner style in Corners (section 3).
- A drag does not change the chip's Direction or Mode (spec criterion 10); its distance becomes
  the remembered distance on release (criterion 6), so the next chip opens with it.

### 2.10 Accessibility

- The chip: `role="group"`, `aria-label="Offset"`. Distance: `aria-label="Distance, millimetres"`,
  `aria-invalid` and `aria-describedby` (the message card) when marked. The message card is a
  polite `role="status"` that is always mounted, empty when there is nothing to say.
- Direction: `role="radiogroup"` named "Direction", radios named by their visible text "Out",
  "In", "Both"; `title` "Outset: grow", "Inset: shrink", "Both sides: one copy outside, one
  inside".
- Corners: `role="radiogroup"` named "Corners", radios `aria-label` "Round", "Miter", "Bevel".
- Mode: `role="radiogroup"` named "Mode", radios `aria-label` "New object" and "Change original";
  `title` "New object: the original stays" and "Change original: no copy is made". Icons
  `aria-hidden`.
- Focus ring: 2 px `--editor-accent`, as the Move chip. Contrast as the Move chip (text
  `--toolbar-icon` on `--toolbar-bg` and on white, active segments white on `--accent`).
- The canvas handles are pointer targets only; **O / Enter and the chip are their keyboard
  equivalent**, including the corner style. The readout and badges stay `aria-hidden`; what was
  written is announced by the notice (`role="status"`).

## 3. The handles, the preview rule and the drag

**Corner handles** (unchanged from 2026-10-12): four, one per corner of the operand box, each 12 px
outside the corner on the box's outward diagonal; the "Parameter handle" look (10 px circle,
1.5 px `--accent` ring, white ground, 4 px `--accent` dot); hover `--accent-hover`, being dragged
`--accent` ground; hit area 16 px. One rotated object: its own rotated box; several objects: the
axis-aligned box around them; an axis under 24 px on screen widened to 24 px for placing the
handles only. **Hint chip** after 600 ms of rest on a corner handle: "Drag: outset or inset" /
"Shift: both sides" / "Ctrl: make a new object" (or "Ctrl: change the original" while the chip's
Mode is New object) / "Alt: corner style" (fourth round; replaces "Double-click: next corner
style").

**Preview: only while it is judged** (third round, spec criterion 52; replaces the "standing
preview" of the first 2026-10-13 version). The "Live preview outline" (1.5 px hollow
`--preview-new`, white casing, no fill) is drawn in two situations only: (a) **live while a corner
handle is dragged**, from the moment the pointer leaves the 3 px dead zone to the release or
Escape; (b) **while the Offset entry chip is open**, for the chip's current values (Distance,
Direction, Corners, Mode; Corners is the remembered corner style). Not on tool choice, selection,
after a release, after Escape or after closing the chip. An Alt step changes the style only inside
a drag, so it needs no preview of its own. Over 2,000 operand nodes the grown box (dashed `--preview-new`) replaces the outline
in (a) and (b).

**The handles stay at the operand box** (spec criterion 9); a drag starts from d = 0 at the press
point (criterion 10). Snapping to the remembered distance within 6 px (criterion 11) lets the
maker drag back to exactly that value. Before the drag nothing but the handles is drawn.
(Default pending customer; alternative: a drag continues from the remembered distance.)

**The drag** (unchanged): d is the pointer's movement projected on the handle's outward diagonal,
`d = (dx * sx + dy * sy) / 2`; outward Outset, inward Inset, switching live through the press
point; snapping as in criterion 11; the inset stops where nothing is left. The dragged handle moves
with the grown box; the other three stay idle at the operand box. The center handle is hidden
during the drag.

**Readout chip** during a drag (the "Live transform readout", 12 px up and right of the pointer,
flip and clamp rules), amended 2026-10-13:

- **Line 1:** the side, the distance and **always the corner style**: "Outset 2.5 mm · Round",
  "Inset 0.8 mm · Miter", "Both ±2.5 mm · Bevel". Always, not only when it is not Round: Alt can
  change the style during the drag, so the maker must see which one the drag uses now; the text
  changes in the frame of the Alt press.
  (Default pending customer; changes spec criterion 15, see its 2026-10-13 remark.)
- **Line 2, muted, the effective Mode and Direction** (third round): "Changes the original" (no
  modifier, default), "Changes the original, adds the inner copy" (Shift), "New object" (Ctrl),
  "Two new objects" (Ctrl+Shift). It follows the effective result: with the chip's Mode remembered
  as New object, no modifier shows "New object" and Ctrl shows "Changes the original".
- **Line 3, only when it applies:** "Nothing is left" (`--field-invalid`), "1 of 3 shapes vanishes",
  "Preview off: large selection".
- Whenever at least one new object is made (Ctrl, Ctrl+Shift, and Shift alone with its inner
  copy), the copy-move **plus badge** sits 16 px left of and below the pointer (row "Modifier
  badge"); not with a single-side Change original (no modifier).
- No line ever says "Stays a rectangle" or "Becomes a path".

**Corner style by Alt** (fourth round, 2026-10-13; replaces the double-click on a corner handle,
which no longer exists):

- **Start:** a drag starts with the **remembered corner style**: Round on the very first drag, then
  the last style a written drag used. The press itself is made without Alt.
- **Step:** during the drag each **Alt keydown** moves one place in the cycle Round, Miter, Bevel,
  Round. Auto-repeat (`repeat`) is ignored, so holding Alt steps once; the key-up is not needed.
  The keydown is `preventDefault`-ed, so Alt does not focus a menu or menubar in the webview. Alt
  held at the press is not used (a fresh Alt keydown after the press is needed), which also keeps
  Linux window managers' Alt+drag out of the picture. Alt works with or without Shift and Ctrl.
  Alt outside a corner drag does nothing and is not intercepted.
- **Feedback:** in the same frame the preview (or the grown box) and line 1 of the readout show the
  new style ("Outset 2.5 mm · Miter"). There is no separate message for the step.
- **Store:** a **written** drag stores the style in use at the release as the remembered one. A
  drag that writes nothing (Escape, the other button, release at d = 0, a refusal) leaves the
  remembered style as it was (default pending customer). The chip's Corners control reads and writes
  the same value (section 2.3).
- **Hint:** the corner handle's hint chip (600 ms) ends with "Alt: corner style".
- **Two quick clicks on a corner handle** are two drags of d = 0; they write nothing and change
  nothing. The center handle's double-click still opens the chip.

**Release and Escape** (unchanged): release writes one commit "Offset"; Escape during the drag or
the other mouse button cancels. After the release the tool stays active and the result is selected
(spec criterion 23); no preview is shown.

## 4. Modifier keys (rewritten 2026-10-13, third round)

| Held during the drag | Direction | Result (default Mode: Change original) |
|---|---|---|
| none | from the drag (outward Out, inward In) | **the original is changed**, no new object |
| **Shift** | **Both** at the dragged distance | **the original becomes the outset side**, one new object is made for the inset side (default pending customer) |
| **Ctrl** | from the drag | a **new object**, the original unchanged |
| **Ctrl+Shift** | Both | **two new objects** (outer and inner), the original unchanged |

Ctrl switches to the other Mode than the remembered one; the table shows the default. The old
Shift+Ctrl chord "original takes the dragged side plus a copy for the other" is gone: Shift alone
now gives that result with the outset side fixed for the original (the result no longer depends on
which way the pointer went). Shift stands for the chip's Direction control (Both), Ctrl for its
Mode toggle; the same combinations apply in the chip (2.7). Keys may be
pressed before the press or during the drag; the state at release counts; the readout, the badge
and the preview follow in the frame a key changes; the release reads the modifiers from its own
event (WebKitGTK's late Shift key-up, `0014`). Shift and Ctrl in other tools mean "about the
centre / one axis" and "copy"; Ctrl = New object now agrees with Ctrl = copy in Move (customer,
third round). Shift is the symmetric choice; the readout says which result a release writes.

## 5. What is created, z-order, selection after

Unchanged. Each result copies its original's complete style; each object is offset on its own; an
outset copy goes directly below its original, an inset copy directly above; with Both the outer
below, the inner above. Change original keeps place, name and id. After a write the produced or
changed objects are selected, the unchanged originals are not, open paths that were ignored are
not.

## 6. Rectangles, circles, polygons, stars: stay primitives where exact

Unchanged rules (spec criterion 40). **2026-10-13:** no text anywhere says whether the result
stays a primitive or becomes a path; the old status line and readout line are removed.

## 7. Output quality

Unchanged: curves stay curves, exact to 0.01 mm (spec criteria 33 to 39).

## 8. Multi-selection, groups, open paths, large documents

- **Multi-selection:** one operand box, four corner handles, one center handle, one d for all.
- **Groups:** a group alone, nothing is drawn (no handles, no preview, no message). A group among
  closed operands: handles and preview for the operands; a release or Enter is refused with the
  spec's sentence and the group outlined. After a release the sentence is an alert in the canvas
  notice slot, as the Boolean refusals of `0016`; after Enter it is the chip's message card (2.7).
- **Open paths (2026-10-13):** never offset. Only open paths selected: no handles, no preview, no
  message. Mixed with closed operands: the operand box, the handles and the preview are for the
  closed ones only; the open paths stay drawn as selected and are left unchanged and unselected
  after a write; no message (default pending customer).
- **Selecting in the tool:** click, Shift+click, a press on empty canvas, the marquee, Ctrl+A, as
  in the Select tool. A drag on an object does not move it. A press on the center handle's hit area
  does nothing (2.1).
- **Large documents:** handles, readout and chip follow at frame rate; previews are computed after
  the frame, a stale one is dropped; over 2,000 operand nodes the grown box replaces the preview,
  while dragging and while the chip is open; a write computes the real result. A selection change
  from Ctrl+A on a large document never blocks the pointer (no preview is computed on selection).

## 9. Keyboard-only route (amended 2026-10-13)

1. **O** selects the Offset tool (or Tab to the "Path tools" toolbox, Space).
2. Select with Ctrl+A, or select in the Select tool first; the selection is kept. Only the handles
   are drawn, no preview.
3. **O** or **Enter** opens the chip with the remembered values, focus in Distance with its text
   selected; the preview of those values appears with the chip.
4. Type a distance or step it with Up / Down; Tab to Direction, Corners, Mode; arrows choose; the
   preview follows.
5. **Enter** writes one commit and returns focus to the canvas; **Escape** closes without writing.
   The preview goes with the chip.
6. **O, Enter** again offsets the result: with Mode New object it makes concentric copies by
   keyboard; with Change original (default) it changes the same object once more.

Arrow keys on the canvas nudge as in the Select tool and do not offset.

## 10. Deliberately not there

A top bar, an Apply button, a status hint for the result kind, Ends and offsetting open paths
(removed 2026-10-13); a linked live offset; "adjust the last offset"; miter limit and precision
fields; several copies at once; a variable distance; offsetting strokes or text; units other than
mm.

## 11. Defaults pending customer (2026-10-13)

1. The **center handle** is the double-click target for the chip (alternative: a double-click
   anywhere in the operand box). Its glyph is the Offset glyph in small, not the Move arrows.
2. **O and Enter** both open the chip.
3. **Corners is in the chip**, beside Distance, Direction and Mode.
4. **Escape, a press outside, a tool switch or a selection change** close the chip without
   writing and keep its values as the remembered ones.
5. The **chip is placed below the result**, not over the box (Move chip rule adapted, 2.4).
6. The **handles stay at the operand box** and a drag starts from d = 0 (alternative: continuing
   from the remembered distance).
7. ~~The pending corner style from a double-click is used by the next written drag only.~~
   **Replaced, fourth round (customer decision):** Alt while dragging steps the style; the last
   used style is remembered and shared with the chip's Corners; the readout always names it. Still
   **default pending customer:** only a written drag updates the remembered style (a cancelled or
   refused drag does not); Alt in the dead zone already steps; Alt works with Shift or Ctrl held;
   "Alt: corner style" is on the corner handle's hint chip, not the tool tooltip; whether the
   remembered style survives an app restart is as for the remembered distance.
8. **Open paths** in a mixed selection are ignored silently; only open paths or a group alone show
   nothing.
9. Tooltip second line: "Drag a corner handle. Shift: both sides. Ctrl: new object."
10. *Third round.* **Shift alone** = Both with Change original: the original becomes the outset
    side, one new object is made for the inset side (alternative: the original takes the inset
    side).
11. *Third round.* **Ctrl at the double-click** sets Mode New object whatever the remembered Mode
    is (the customer's words); in a drag Ctrl switches to the other Mode. They differ only when the
    remembered Mode is New object (alternative: the drag rule in the chip too).
12. *Third round.* The plus badge shows whenever a new object is made, so also with Shift alone.
13. *Third round.* The center handle's hint chip gets a second muted line "Shift: both sides. Ctrl:
    new object." so the maker learns that the double-click takes modifiers.

## 12. Questions for the architect

A1 and A2 of 2026-10-12 are answered in `geometry-rework.md` and `adrs.md`.
