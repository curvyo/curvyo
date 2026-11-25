# 0154 Library into slots: UX notes

ux-engineer, 2026-11-25. Written from `specification.md` (Draft, criteria 1 to 16) at the defaults
of Fragen 377 to 379 and 385: **377 C slice 1** (a copy only), **378 B** (Fill empty slots in slice 1),
**379 A** (Fit, Alt keeps the real size), **385 C** (a filled slot refuses, an armed Replace).
Checked against `specs/0102-laser-fixtures/ux-notes.md` (slot look, fit cue 7.2, slot rows 4.2,
Layers 8), `specs/0113-fixture-sets-and-pins/ux-notes.md` (the Fixtures tab, sets and job
fixtures), `specs/0137-document-library/ux-notes.md` (page rows, page preview pane, drags 6.5 and
7.1, Came from), `specs/0152-library-browser/ux-notes.md` (press model 0.5 and 9, detail bar 8),
`docs/design-system.md` ("Drop badge", "Drag ghost", "Armed action", "Canvas notice slot",
"Action notice", "No notice after a success", "Hidden, not disabled", "Bring content in",
"Library tab", "Library browser") and the code: `DropBadgePill.tsx` (the `fit` kind with
`Magnet`), `lib/libraryDrag.tsx`, `lib/fixtureRows.ts` ("Fits Slot 2"). No new colour token, no
new component kind, no popup. **(Proposal)** items are vetoable at the demo; section 13 lists them
as V1 to V22, section 14 the spec deltas K1 to K10. No customer question: every open point has a
house rule or a safe default. Draft: `docs/design-drafts/2026-11-library-into-slots/`.

Terms: "unit" is what goes into a slot (criterion 2: a wrapped page, wrapped picks, or one
object). "Target slot" is a slot of an Active holder of the active laser page (eye off included,
damaged excluded). "Fit" is criterion 6. "Muted" is `--panel-muted-fg`.

## 1. Principle

The slot is the drop target. While a library item is dragged over a laser page, the slot under
the pointer lights and shows the result before the release: the unit already sits in the slot,
centred, scaled and turned as it will land, and the badge says the scale in words. Alt keeps the
real size, Shift fills every empty slot of that holder. Nothing the maker placed is ever
overwritten by a drop: a filled slot says so, and replacing is one deliberate armed press. Every
pointer route has a keyboard route through one inline list, **Into slot…**.

## 2. Sources and where a drag can start (criteria 1, 2, 13)

| Source | Unit | Drag starts on |
|---|---|---|
| Library tab page row (`0137` 7.1) | the whole page, wrapped, its frame as the box | the thumbnail or the name, 4 px |
| Page preview pane picks (`0137` 6.5) | the picks, wrapped (one pick: itself) | a picked object, 4 px |
| Browser tile: local page, one-page template | the whole page, wrapped | the tile, 4 px, split view |
| Browser tile or tab cell: an icon (V1, K1) | one object (the 24 mm icon path), not wrapped | the tile or cell, 4 px |
| Browser tile: a template or document with several pages | refused over a slot (section 3.4) | |

The unit's box is known before the release for every source (page size on the row, picks' joint
box, the icon's 24 mm box), so the fit is shown during the drag. A template that is not yet
downloaded is fitted by its stated page size; if it has none, the badge shows the slot name
without the scale and the notice after the drop says it (section 6).

## 3. The drag over a laser page (criteria 1, 5, 6, 9)

### 3.1 Over a target slot

- **Hit**: the slot whose outline contains the pointer's hotspot, with 4 device px outside the
  outline (12 on a coarse pointer, the `0102` D12 tolerance). Overlapping slots: the one drawn last
  (creation order). One slot at a time. A slot with content in it is still the target (the content
  is not).
- **Slot light**: the `0102` 7.2 fit look, unchanged: fill `--accent-hover`, 2 px `--accent`
  outline with `--selection-casing`. A slot of a holder whose eye is off is not drawn, so it does
  not light; the badge still names it (`0102` 7.2).
- **Fitted ghost** (V2): the pointer's ghost (the 70 % thumbnail of a page row, the picks' Drag
  ghost outlines) is hidden, and the unit is drawn **in the slot** at the matrix it will get: its
  box as the "Drag ghost" outline (1.5 px `--drag-ghost` over the white casing), inside it the
  content outlines for picks and icons, or the page thumbnail at 70 % for a page. Turned when the
  fit turns it. It is the result before the release, so no other cue is needed. Past 2,000 leaves
  the box alone (the `0100` rule). Leaving the slot brings the pointer ghost back in the same frame.
- **Badge** (the "Drop badge" look; glyph `Magnet`, the slot glyph of `0102` "Fits Slot 2"):

| Case | Words |
|---|---|
| Plain centre (size within 0.1 mm) | "Into Slot 2" |
| Scaled | "Into Slot 2 · 50 %" |
| Scaled and turned | "Into Slot 2 · 67 %, turned" |
| Turned only (0.1 mm match turned) | "Into Slot 2 · turned" |
| Alt, fits at real size | "Into Slot 2 · real size" |
| Alt, larger than the slot | "Into Slot 2 · real size, too large" |

- **Names** (V3, K2): the slot's own name, not "slot <name>" ("Into slot Slot 2" reads twice). When
  two target slots of the page share the name, the holder is added: "Into Slot 2 of Holder 1 ·
  50 %". The slot name is cut at 24 characters with an ellipsis.
- **Scale words** (V4): whole percent ("67 %"), one decimal under 10 % ("4.5 %"), up to any value
  when scaled up ("400 %"). A non-breaking space before "%", as the rest of the app.
- **Alt** (criterion 6 d) switches the ghost and the badge live in both directions; the release
  follows the modifier flags of the pointer-up (the `0100` rule); Alt key events are
  `preventDefault`ed during the drag (the `0100` rule; Option on macOS).
- **Shift**: section 5 (fill).
- **Ctrl** has no meaning over a slot in slice 1 (Frage 377 C slice 2 gives it the live placed
  page, `Frame` "Placed page into Slot 2", V5).
- **Status region** (sr-only, polite, once per change of slot or modifier): "Into Slot 2 of
  Holder 1, scaled to 50 percent." / "Fill 12 of 15 slots of Holder 1."
- **Release**: section 4.

### 3.2 Over no target slot (criterion 1)

As built, nothing changes: a page shows `Import` "New page" and is added as a page, picks show
`Copy` "Copy" and land at the drop point, an icon lands at the drop point. Over a sketch page the
`0099` refusal. Escape cancels anywhere, nothing is written.

### 3.3 Over a filled slot (criterion 7)

- The slot shows the 2 px `--accent` outline **without** the fill (lit but not taking), the
  fitted ghost is not drawn, the badge is `Ban` "Slot 2 is filled". The cursor stays `grabbing`
  (not `not-allowed`): the release is not a no-op, it offers Replace (section 7) (V6).
- With Shift the filled slot is simply skipped (section 5); no refusal.

### 3.4 Slots that are no target (criterion 9; K3)

| Case | What the maker sees |
|---|---|
| Holder Active off, set off this page, damaged slot | not drawn, so nothing to point at: no light, no badge, the drop of 3.2 |
| A slot of a laser page that is not the active page | no light; the badge of 3.2; the release does the 3.2 drop (K3: the PO confirms the activation reading of criterion 9) |
| A source with several pages | no light, `Ban` "Only one page goes into a slot.", the release does the 3.2 drop |
| Inside a holder's board, outside every slot | the drop of 3.2 (the board is not a target) |

## 4. The release into a slot (criteria 3, 4, 5)

- One undo step **"Put into slot"**. The unit is the selection (criterion 4), the Select tool is
  active, the Fixtures and Library tabs stay as they were; focus goes to the canvas (mouse).
- The 0137 rules hold: Came from on the top-level group, "Linked clones became paths" notice when
  it applies.
- **Notice** (section 6) only when the result is not a plain centre: the scale and the turn are a
  change the maker did not draw, so they are said once (criterion 5; the "partial result" exception
  of "No notice after a success", as the `0100` guides notice). A plain centre has no notice.
- A template that is downloaded on drop: the slot keeps its light and shows `LoaderCircle` 16 px at
  its centre after 150 ms until the unit lands; a failure is the `0137` 10 alert ("Box could not be
  imported: Offline. Nothing was changed.").

## 5. Fill empty slots (criterion 8; K4)

### 5.1 Where the command lives

The source is in the library and the targets are on the canvas, so the command lives where the two
meet (V7):

1. **Shift during the drag** over any slot of a holder: the gesture of the pointer route. Shift
   "extends" in every selection of the app; over a slot it extends the drop to every empty slot of
   that holder. Off slots Shift keeps its `0100` axis lock.
2. **The Fill row of the Into slot… list** (section 8): the keyboard and touch route, and the one a
   maker finds without knowing Shift. The list is opened from the Library tab row, the page preview
   footer and the browser detail bar (criterion 8's "detail bar" route is this list, K4).

Not in the Fixtures tab (holder row or a selected holder's context): that place cannot see which
library item is meant, and a command there would have to name a source the maker picked in another
tab (V8). The spec's "press on the holder's name chip while dragging" is dropped: a mouse drag
holds the button, so no second press is possible (K4).

### 5.2 The Shift drag

- Every **empty** target slot of the hovered slot's holder lights (the 3.1 light) and shows its own
  fitted ghost; filled slots of that holder show nothing; slots of other holders do not change.
- Badge `Grid2x2Plus` (V9): "Fill 12 of 15 slots" (12 empty of 15 target slots), "Fill 12 of 15
  slots · 50 %" when every one gets the same scale, "… · 48 to 100 %" when they differ, "… · real
  size" with Alt. No empty slot: `Ban` "No empty slot in Holder 1." (criterion 8's sentence, in the
  badge), and the release writes nothing and shows no notice.
- Release: one undo step **"Fill slots"**; every copy is selected (one selection box over them).
- Large holders: 50 ghosts are drawn as boxes plus thumbnails; past 2,000 leaves in all (unit
  leaves times slots) the boxes alone.

## 6. Notices (criteria 5, 7, 8; each at most 70 characters)

All in the canvas notice slot. Slot names are cut at 24 characters.

| Text | When | Role, time |
|---|---|---|
| "Fitted into Slot 2 at 50 %." | a scaled drop | status, 3 s |
| "Fitted into Slot 2 at 67 %, turned." | scaled and turned | status, 3 s |
| "Turned to fit Slot 2." | turned only | status, 3 s |
| "Real size: larger than Slot 2." | Alt, the unit is larger than the slot | status, 3 s |
| "Filled 12 slots at 50 %." / "Filled 12 slots at 48 to 100 %." | a fill that scaled | status, 3 s |
| "Filled 11 slots at 50 %; 4 were already filled." / "Filled 11 slots; 4 were already filled." | a fill that skipped | status, 3 s |
| "Slot 2 is filled." with **Replace** | a drop on a filled slot | alert, 8 s, section 7 |
| "Fitted into Slot 2 at 50 %. 2 linked clones became paths." | a slot drop that also turned clones into paths: one notice, the fit sentence first, then the short `0137` sentence (alone: "2 linked clones became paths.") | status, 5 s |

No notice for a plain centre, a real-size drop that fits, a fill without scale and without skips,
or a Replace (its result is visible). Rust formats the texts (`curvyo-ui-core`), as every notice.

## 7. Filled slot: the refusal and the armed Replace (criterion 7)

- **The notice with one control** (V10, recorded as an exception of "Canvas notice slot", like the
  Document preview banner's Back to live): the refusal look (validation chip, `role="alert"`), text
  "Slot 2 is filled.", 8 px, then **Replace**, an outline text button 24 px high, 12 px, 72 px wide,
  accessible name "Replace the content of Slot 2".
- **Armed** (the "Armed action" convention, `--destructive`: it removes what the maker placed):
  first press fills it `--destructive` with white text "Replace"; a second press at least 400 ms
  later removes the members whose centre lies in the slot (never fixtures; a placed page as a
  reference) and puts the unit in, as one undo step **"Replace slot content"**; the notice closes,
  the new unit is selected, and the fit notice of section 6 follows if it scaled. Disarm after 4 s,
  Escape, a press elsewhere, focus leaving. sr-only status on arming "Press again to replace the
  content of Slot 2."
- **Lifetime**: 8 s, held while the pointer is over the notice or focus is in it (WCAG 2.2.1). The
  notice and its Replace go at once on any document change, a new drag, a page switch or Escape on
  the canvas: Replace always acts on the drop it answers, never on a stale one.
- **Keyboard**: the notice follows the canvas in the DOM, so Tab from the canvas reaches Replace
  while it is shown. The list route (section 8) never needs the notice: a filled row arms in place.

## 8. Into slot… (criterion 10)

### 8.1 The button

| Surface | Button | Shown |
|---|---|---|
| Library tab page row | `Magnet` 16 px icon button 24 x 24, tooltip and name "Into slot…", left of Import page; the row's action slot grows from 28 to 56 px (the `0138` source row's 56 px slot) | on hover, focus within, the current row, always on a coarse pointer (the 0137 action rule) |
| Page preview footer | outline text button "Into slot…" 96 x 28, left of Import page; with picks it takes the picks, without them the page; the count ellipsises first | always while there is a target |
| Browser detail bar | the last stacked button "Into slot…" (112 x 28; narrow: 28 px `Magnet` icon button), under Pick elements… for a local page, alone for a one-page template or an icon, in the reserved column (a source never has Show pages, so the column holds at most two buttons) | for the current tile when it is a source (section 2) |

**Hidden, not disabled** (V11, K5): with no target slot on the active page (not a laser page, no
Active holder, no slot) the button is not in the tree. In the tab row and the detail bar the slot
is reserved, so nothing moves; in the footer the count takes the room. Criterion 10's "disabled
with the reason" is the house exception only where neighbours would move, which is not the case
here.

### 8.2 The inline list

Opens next to its button, inside the same region, pushing what follows, never over it (no popup):
under the page row in the Library tab (the tree's rows below are offset by the list's height; it is
not a tree row), as a band above the footer in the page preview pane (the body shrinks), as a band
above the detail bar in the browser (the grid shrinks; the band rule of the narrow facets). At most
50 % of the region's height, then it scrolls in itself (in the tab: at most 8 rows).

| Row | Height (coarse) | Anatomy |
|---|---|---|
| Caption | 28 | the unit, 12 px muted, ellipsis: "Lid · 60 × 40 mm" / "3 elements · 60 × 40 mm" / "Arrow left · 24 × 24 mm"; at the right **Real size**, a 28 px outline toggle (`aria-pressed`, pressed look `--toolbar-icon-active-bg`), the keyboard and touch form of Alt (V12); session only |
| Holder heading | 24 | "Holder 1" 12 px semibold muted, at the right "3 of 15 filled" 11 px muted tabular; " · hidden" after the name when its eye is off; not a row in the keyboard order (the `0137` heading rule) |
| Fill | 28 (36) | `Grid2x2Plus` 16, 4, "Fill empty slots" 13 px, at the right "12 of 15" 12 px muted tabular; not in the tree when the holder has no empty slot |
| Slot | 28 (36) | slot shape glyph 16 (`0102` 4.0), 4, name 13 px, 8, size 12 px muted tabular "30 × 30 mm" / "⌀ 25 mm", at the right the outcome 12 px muted: "50 %", "67 %, turned", "turned", nothing for a plain centre, "real size" / "too large" with Real size pressed, **"filled"** for a filled slot (V13) |

Order (criterion 10): the holders in the Fixtures tab order (sets, then job fixtures), each heading
followed by its Fill row and its slots in creation order. Damaged slots are not listed.

- **Keys**: one Tab stop for the rows (roving), Real size before it in the Tab order; Up, Down,
  Home, End, Page Up, Page Down, no wrap; Enter or Space takes the focused row; Escape closes and
  returns focus to the button. On open, focus goes to the first row; with a selected target slot
  (section 9) to that slot's row.
- **Take**: a slot row is the keyboard drop (section 4: "Put into slot"), a Fill row the fill
  (section 5: "Fill slots"); the list closes, the unit is selected, focus returns to the button,
  the polite region says "Put into Slot 2." / "Filled 12 slots." and the section 6 notice shows as
  for a drop.
- **A filled row** (V14): Enter, Space or a press arms it in place: its outcome cell becomes a
  72 px filled `--destructive` pill "Replace" with white text, the row takes `--destructive-soft`
  (the armed trash row look); the second press after 400 ms is "Replace slot content". Disarm as
  the convention; the first Escape disarms, the second closes.
- **Names**: listbox "Into slot"; options "Fill empty slots of Holder 1, 12 of 15 slots",
  "Slot 2, rectangle, 30 by 30 millimetres, scaled to 50 percent", "Slot 3, circle, 25
  millimetres, filled"; the Real size toggle "Real size" (`aria-pressed`).

## 9. Double-click and Enter with a selected slot (V15, K6)

- A slot is selected through its row in the holder's slot list (`0102` 4.2; the 2 px accent outline
  on the canvas). While a target slot is the canvas selection, the **load gestures** that would
  import a page or an item (double-click or Enter on a browser tile, Ctrl+Enter on a tab page row,
  double-click or Enter on an icon cell) put the source **into that slot** instead: the same fit,
  the same "Put into slot" step, the same notices; Alt held on the double-click keeps the real
  size. A filled selected slot gives the section 7 notice.
- The buttons that say **Import** (Import page, Import elements) keep importing: a labelled control
  does what its label says.
- It is not a mode: after the drop the unit is the selection and the slot is not, so the next
  double-click imports again. The selected slot's outline is the state the maker set; no label or
  tip is added.
- Veto point: if the customer wants double-click never to change, this section goes and the list
  is the keyboard route alone.

## 10. After the drop: canvas, Layers, Fixtures tab (criteria 2, 11)

- **Canvas** (V16): the content is ordinary artwork drawn over the slot's hole, in its own colours
  and styles (scaled strokes as `0010`). The slot keeps its 1 px ink outline under the content. A
  filled slot **does not draw its name** (its 11 px label would show through the artwork). No tag,
  no badge, no mark: the content in the hole is the cue. A unit that is larger than its slot (Alt)
  simply overlaps the board.
- **Layers row** (V17): the unit's row is the group named after the source page ("Coaster", the
  group glyph; one icon or one pick keeps its own name and glyph). On a laser page a top-level
  member whose box centre lies in a target slot carries " · Slot 2" in 12 px muted after its name
  (computed each frame for the rows on screen, never stored, criterion 14), so fifteen "Coaster"
  rows can be told apart. Tooltip line 2 the Came from line "From Box job › Coaster, 24 Nov 12:03".
  Accessible name "Coaster, group, in Slot 2 of Holder 1".
- **Fixtures tab** (V18): a slot row of the holder's slot list gains "filled" 11 px muted after its
  name (the "Outside the board" place); the holder row's line 2 reads "Holder · 15 slots · 3 filled"
  when any slot is filled. Computed, no step.
- **Moving the holder** does not move the content (criterion 11): no cue, as for any other item on
  the board; the slot rows then read empty and the Layers suffix goes.

## 11. Undo steps

"Put into slot" (a drop, the list's slot row, the selected-slot double-click), "Fill slots" (Shift
drop, the list's Fill row), "Replace slot content" (the armed Replace in the notice or the list).
Each restores the selection of before (criterion 4). No step for arming, the Real size toggle,
opening the list, or a refused drop.

## 12. Keyboard, touch, states

- **Keyboard**: every drop has the list route (section 8); Alt is the Real size toggle; Shift is
  the Fill row; the filled-slot Replace arms in the row. No new shortcut.
- **Touch** (coarse pointer): a 400 ms hold starts the drag (the `0137`, `0152` rule); no
  modifiers, so Real size and Fill go through the list; action slots always shown; rows 36 px.
- **Empty**: no target on the active page: no Into slot… button, the drag shows no light; nothing
  else, no text.
- **Errors**: a source that cannot be read keeps the `0137` and `0152` row and tile states and is
  no source; a core failure on drop is the `0137` 10 alert ("… could not be imported: <reason>.
  Nothing was changed.").
- **Large**: the light, the ghost and the badge are computed once per frame for the hovered slot
  (and its holder's empty slots with Shift); the list is windowed like the Fixtures rows (at most
  60 row elements); the Layers suffix only for rows on screen.

## 13. Proposals (vetoable at the demo)

| # | Proposal |
|---|---|
| V1 | Icons (browser tiles, Iconify cells of the tab) are sources: one object, not wrapped |
| V2 | Over a slot the unit is drawn fitted in the slot (box, outlines or the 70 % thumbnail); the pointer ghost hides |
| V3 | Badge and notice name the slot by its own name; the holder is added only when two target slots share the name |
| V4 | Scale in whole percent (one decimal under 10 %) |
| V5 | Ctrl over a slot is kept for the slice-2 live placed page |
| V6 | A filled slot: outline without fill, `Ban` "Slot 2 is filled", cursor stays `grabbing` |
| V7 | Fill by Shift over a slot (pointer) and by the list's Fill row (keyboard, touch) |
| V8 | No Fill command in the Fixtures tab or the holder's context |
| V9 | Fill glyph `Grid2x2Plus`; count "12 of 15" (empty of target slots) |
| V10 | The canvas notice "Slot 2 is filled." carries one control, the armed Replace; held while hovered or focused; gone on any change |
| V11 | Into slot… is hidden, not disabled, without a target |
| V12 | A Real size toggle in the list as the keyboard and touch form of Alt |
| V13 | List rows show the outcome per slot ("50 %", "turned", "filled") |
| V14 | A filled row of the list arms Replace in place |
| V15 | With a selected target slot, double-click and Enter on a library item put it into that slot; Import buttons keep importing |
| V16 | A filled slot hides its name label; no other mark |
| V17 | Layers rows of laser page members carry " · Slot 2" when their centre lies in a slot |
| V18 | Fixtures tab slot rows show "filled", the holder line 2 "· 3 filled" |
| V19 | Into slot… sits left of Import page in the tab row (56 px action slot) and the footer (96 px) |
| V20 | The list opens next to its button and pushes, in all three surfaces |
| V21 | One fit notice per drop; the `0137` clone sentence is appended, not a second notice |
| V22 | The selected slot's row is focused first when the list opens |

## 14. Spec deltas for the product owner

| # | Criterion | Delta |
|---|---|---|
| K1 | Terms, 1 | Icons of the browser and of the tab's Iconify cells are a library source (one object, not wrapped); a source with several pages is refused over a slot ("Only one page goes into a slot.") |
| K2 | 1 | Badge "Into <slot name>" plus the scale ("Into Slot 2 · 50 %"), the holder only when names repeat; Alt reads "· real size" |
| K3 | 9 | Inactive holders, sets off this page and damaged slots are not drawn, so they show no reason badge; their example "Fixture is not on the bed." falls away. A slot of a laser page that is not active: read as "no light, the drop of criterion 1"; the PO says whether the release should also activate that page |
| K4 | 8 | The holder name chip while dragging is replaced by Shift over a slot; the browser detail bar reaches Fill through Into slot… (no button of its own); "No empty slot in <holder>." is said by the badge, the release writes nothing |
| K5 | 10 | Into slot… is hidden, not disabled, without a target (house rule); the list has a Real size toggle; a filled row arms Replace in place |
| K6 | new | With a selected target slot, double-click and Enter on a library item put it into that slot (section 9); vetoable |
| K7 | 5 | Notice texts of section 6; no notice for a plain centre or a real-size drop that fits |
| K8 | 7 | The Replace notice holds while hovered or focused and goes on any document change |
| K9 | 11 | Layers rows and Fixtures slot rows show the computed "Slot 2" / "filled" (no stored link) |
| K10 | 4 | Undo names unchanged: "Put into slot", "Fill slots", "Replace slot content" |

## Decisions after the build (2026-11-26, Proposal)

- A slot of a laser page that is not active (3.4, K3): the light, the fitted ghost and the badge
  "Into Slot n · …" as on the active page; the release activates that page and puts into the slot
  in one undo step "Put into slot" (the activation is no step).
- Library tab page rows (8.1, V19): Into slot… shows only on hover, focus within or the current
  row, like the other row actions, and takes no room otherwise; the name keeps the full width (no
  56 px action column).
- The tab's Iconify cells take the selected-slot double-click and Enter of section 9 too.
- The Into slot list ignores a held Enter or Space (key repeat): the Enter that opened the list
  never runs on into its first row, the Fill row; Enter on a slot row only ever puts into that slot.

## Links

Spec: `specification.md`. Slots and fit cue: `specs/0102-laser-fixtures/ux-notes.md` 4.2, 5.2,
7.2, 8. Fixtures tab: `specs/0113-fixture-sets-and-pins/ux-notes.md` 3, 4.3. Library tab, page
preview, drags, Came from: `specs/0137-document-library/ux-notes.md` 4.2, 6, 7, 8, 10. Browser
press model and detail bar: `specs/0152-library-browser/ux-notes.md` 0.5, 8, 9. Drop badge, Drag
ghost, Armed action, Canvas notice slot: `docs/design-system.md`; section "Library into slots".
Draft: `docs/design-drafts/2026-11-library-into-slots/01-into-slots.html`.
