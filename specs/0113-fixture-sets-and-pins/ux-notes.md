# 0113 Fixture sets and pins: UX notes

ux-engineer, 2026-11-12. Written from `specification.md` (criteria 1 to 43), the accepted drafts `docs/design-drafts/2026-11-laser-redesign/` (README P14 to P18; screens 06 to 09: "die screens sehen gut aus"), the customer decisions of 2026-11-12 (Fragen 207 B, 208 A, 209 A, 210, **211 B**: a fixture inside a set keeps its own "On the bed" check; **P16 reduced**: the screw keeps its current Head and Thread presentation, no fields), `docs/design-system.md` (rows "Select bar layout", "Live preview outline", "Parameter handle", "Transform entry chip", "Lock cursor", "Kind chip places", "Action notice"; "Interaction conventions") and the built notes `specs/0102-laser-fixtures/ux-notes.md`, `specs/0103-fixture-jobs/ux-notes.md`, `specs/0060-snapping/ux-notes.md` (the marker), `specs/0111-laser-operation-tools/ux-notes.md`. Customer rules applied: symbols with tooltip and accessible name in the add bars, in-place rename, the armed trash, arrows and drag for order, no invented colours, no tips, no success notice, no popups in the panel, hidden not disabled. Items marked (Proposal) are vetoable at the demo; section 13 lists them as F1 to F16.

Terminology: "set" is a machine set, "set fixture" a fixture inside a set, "job fixture" a fixture of this page only, "grid of pins" Columns and Rows of one pin or screw (never the engraved bed Grid kind). "Ink" is `--toolbar-icon`.

Screens 06 and 07 B are superseded in two places: set fixture rows carry the 0102 "On the bed" check (Frage 211 B), and a screw row has no Head and Thread fields (criterion 20).

## 0. What is reused

| Part | Reused from (code) | Change in this slice |
|---|---|---|
| Fixture rows, rename, Active check, eye, armed trash, block fields | `FixturesBlock.tsx`, `useFixtures`, `fixtureRows.ts`, `fixturesView.ts` (`0102` sections 3, 4) | move into the Fixtures tab, inside sets and Job fixtures; Lock in job rows |
| Kind glyphs | `FixtureGlyph` (`FixtureGlyphs.tsx`, `0102` 4.0) | a Pin glyph |
| Handles, chips, readouts | the rectangle model of `0102` section 0 (`TransformEntryChip.tsx`, `MoveEntryChip.tsx`), Parameter handle | pin and screw grids: Diameter, Columns and Rows knobs |
| Fixture jobs | `FixtureJobsSection.tsx`, `FixtureJobRow.tsx`, `FixtureJobParts.tsx`, `useFixtureJobs` | two fixed jobs per pin and screw; "not on this page" |
| Snapping | `useSnapping` (`0060` Objects target and marker) | centres for job fixtures |
| Armed trash, windowing | `useDeleteArm`, `useScrollWindow` | set rows |
| Tab strip | `PanelStrip.tsx` (`GLYPHS`) | `fixtures`: the Holder glyph, wrapped to the strip's `Glyph` props |
| Machine block | `MachineBlock.tsx` (`0102` 9.1) | "Fixture sets in profile: N" |
| Select bar | `SelectToolbar.tsx` | the Lock group for a job fixture |
| Notices | `CanvasNotice.tsx`, `useActionNotice` | section 10 |

## 1. Principle

The bed belongs to the machine, the jig to the job. The machine's fixtures come in named sets, and a page says with one check box which sets lie on its bed today. Fixtures that only this job needs are placed on the page itself, by dropping their centre on a hole of the drawing, and locked so a nudge cannot move them. Marks and circles to drill or align by are jobs of the pin, off until the maker turns them on. Nothing opens a dialog or a popup, and nothing is mirrored.

## 2. Tokens and symbols (no new colour)

| Use | Token |
|---|---|
| Fixture ink: pin outline, screw rings, every fixture glyph | `--toolbar-icon` (9.25:1 on `--canvas-bg`) |
| Pin ground (a solid dowel) | `--toolbar-bg`, opaque, as the holder board (F1) |
| Casing under every fixture line | `--selection-casing` |
| Centre mark and Diameters with Output on | the Score look of `0101` (dotted ink, 1.5px) |
| Muted names and line 2 ("not on this page", "not on the bed") | `--panel-muted-fg` (5.0:1) |
| Selected row, unfolded row | `--value-fill`, 3px `--accent` bar |
| Indent guide of an unfolded set | 1px `--toolbar-icon` at 25 % |
| Pressed toggles (Centre mark, Diameters, Lock in the bar, an armed kind) | `--toolbar-icon-active-bg` fill, `--toolbar-icon-active-fg` glyph and word |
| Box, knobs, centre move | `--accent`, `--shape-handle-fill`, `--shape-handle-stroke` (Parameter handle) |
| Fixture hanging at the pointer, Apply preview | `--preview-new` 1.5px, dashed 4 on 3 (F2) |
| Snap marker | the marker of `0060` (unchanged) |
| Lock badge | chip `--toolbar-bg`, 1px `--lane-line` border, `Lock` 12px ink |
| Insertion line of a set drag | 2px `--accent` |
| Armed rows | `--destructive-soft`, `--destructive` |

Symbols (16px, 1.5px stroke, `currentColor`):

| Control | Symbol |
|---|---|
| Fixtures tab | custom Holder glyph (`FixtureGlyph kind="holder"`) |
| Grid, Holder, Clamp, Stop, Screw | custom `FixtureGlyph` (unchanged) |
| Pin | custom: a filled circle of radius 4.5 on the 16 grid (screen 06) |
| Set disclosure | `ChevronRight` / `ChevronDown` 12px |
| + Set | `Plus` |
| Lock toggle | `Lock` (locked, ink), `LockOpen` (unlocked, `--panel-muted-fg`) |
| Centre mark | `Crosshair` |
| Diameters | `Diameter` |
| Apply fixtures from page | `CopyPlus` |
| Order | `ChevronUp`, `ChevronDown` |

## 3. The Fixtures tab (criteria 1, 11)

- Tab strip: after Operations and Materials, the Holder glyph, name and tooltip "Fixtures"; only on laser pages. The Document tab loses its Fixtures block (Pages, Unit, Machine block, Place page, Background remain).
- Header: "Fixtures" and the Laser kind chip; the subject line names the selected fixture ("Pins 1", screen 07).
- Two sections, 244px column, the tab scrolls as one column: **Machine sets**, then a 1px divider (ink at 25 %), **Job fixtures**.
- Section heading 24px: the word 11px semibold `--panel-muted-fg`; Machine sets carries at the right "2 of 3 on this page" 11px tabular muted, only when the page has a set.
- Empty page (no set, no job fixture): "Machine sets" with "+ Set"; "Job fixtures" with its add bar. Nothing else, no text.

## 4. Machine sets (criteria 2 to 10)

### 4.1 Set row (40px, 48 coarse)

**Line 1**, left to right:

| Slot | Width | Content |
|---|---|---|
| Disclosure | 16px | `ChevronRight` folded, `ChevronDown` unfolded, 12px; `aria-expanded` on the row's disclosure button |
| Gap | 4px | |
| Name | flexible | 13px ink, ellipsis, tooltip; muted when off |
| On this page | 24px | `.cb` check box 16px, name "<set> on this page", `aria-checked` |
| Trash | 24px | armed trash |
| Group gap | 8px | |
| Up, Down | 24px each | hidden at the ends, slot kept |

**Line 2** (11px muted): the kinds in creation order, comma-separated, " · ", then "2 fixtures" ("1 fixture"); off: the kinds, " · not on this page"; empty set: "No fixtures" (F3).

- A press on the chevron, the name or line 2 folds or unfolds; not a step; session state. Enter or Space on the focused row does the same. F2 or a double-click on the name renames (1 to 40 characters; the row's controls leave the tree while the field is open; Enter or leaving commits; Escape cancels); step "Rename fixture set".
- On this page: one step "Set fixture set on page" / "Set fixture set off page". Off: the set's fixtures are not drawn, not hit, not snap targets, not selectable, run no job (criterion 2). A selected fixture of a set turned off is deselected.
- Armed trash (`0081`): status on arming "Press again to delete Coaster jig. Its 2 fixtures go with it."; second press one step "Remove fixture set".
- Order: arrows, drag of the row after 4px (2px `--accent` insertion line, the source row with a dashed `--drag-ghost` outline, a 60 % ghost following vertically, as `0101` 4.6), Alt+Up, Alt+Down, Alt+Home, Alt+End; one step "Move fixture set". A drag never starts on the check box, trash or arrows.

### 4.2 + Set

- Outline text button 244 x 28px, `Plus` and "Set", under the last set. A press adds "Set <N>" (lowest unused N), empty, On this page on, unfolded, with focus on its row; one step "Add fixture set". Hidden at 16 sets (criterion 17, no notice).

### 4.3 The unfolded set (criteria 7, 8)

- Body indented 16px with the indent guide at its left (screen 06 B).
- **Add bar:** `role="toolbar"` "Add fixture to <set>", one Tab stop, arrows move: six 28 x 28px buttons, glyph only, in the order Grid, Holder, Clamp, Stop, Screw, Pin; tooltip and name the kind word ("Pin"). Grid is absent while the set has one. A press adds the fixture with the sizes of `0102` criterion 2 (Pin: 6 mm), centred on the work area, Active on, selected, its row unfolded when it has fields; focus stays on the button; one step "Add fixture".
- **Fixture rows:** the `0102` 3.3 row: Active check ("On the bed"), glyph, name, eye, armed trash; line 2 kind word and the facts of 6.1, then " · not on the bed" when its own check is off, " · not on this page" when its set is off (that one wins). Both switches are shown and work independently (Frage 211 B): the fixture lies on the bed only when both are on. No arrows (creation order, criterion 16).
- A press on a fixture row selects the fixture on the canvas (only while its set is on this page; otherwise the row is selected in the tab only) and unfolds its fields (`0102` 4.2 for Grid and Holder, section 6 here for Pin and Screw). One fixture row unfolded at a time in the whole tab.

### 4.4 Profile (criteria 9, 10)

- Machine block (`0102` 9.1): "Fixture sets in profile: N" 12px muted; Save as profile, Update profile (armed), Reset from profile as built; Differs compares the sets (criterion 10). Picking a profile copies its sets, all On this page, in the pick's step; job fixtures stay.

## 5. Job fixtures (criteria 12 to 17)

### 5.1 Add bar and placement (criterion 13)

- `role="toolbar"` "Add job fixture": five 28 x 28px glyph buttons, Pin, Screw, Holder, Clamp, Stop; tooltip and name the kind word. Hidden at 64 job fixtures.
- A press **arms placement** (P17): the button takes the pressed look (`aria-pressed="true"`); the fixture hangs at the pointer by its centre, outline in `--preview-new` dashed; not drawn while the pointer is off the canvas. A click on the canvas drops it: one step "Add job fixture", named "<Kind> <N>", Active on, selected. **Placement stays armed** for the next one (screen 08: four pins in a row, F4); Escape, a right click, a second press on the pressed button or another tool ends it. While armed a click selects nothing.
- Keyboard: Enter or Space on a kind button arms; with no pointer over the canvas the fixture appears at the work area's centre; arrows move it 1 mm (Shift 10 mm), with centre snapping; Enter drops; Escape ends (F5).
- The button's pressed state is the only cue (no hint line, no tip); the sr-only status says "Placing Pin." once on arming.

### 5.2 Job fixture row (40px, 48 coarse)

**Line 1**: Active check ("On the bed"), 8px, glyph, 4px, name (flexible, ellipsis, rename by double-click or F2), **Lock** 24px, eye 24px, armed trash 24px. No arrows (creation order).

**Line 2** (11px muted): kind · count (only above 1, "2 × 2") · size ("⌀ 6 mm" for a pin; "M6" for a screw with a whole-millimetre thread, else "⌀ 6.5 mm", F6) · "locked" when locked · the jobs that are on ("Centre mark", "Centre mark, Diameters"). "not on the bed" replaces the jobs part when Active is off. Ellipsis cuts the end; the tooltip and the accessible name carry it all. Examples: "Pin · 2 × 2 · ⌀ 6 mm · locked · Centre mark", "Pin · ⌀ 8 mm", "Screw · 4 × 1 · M6 · Centre mark, Diameters".

- Lock: `Lock` ink when locked, `LockOpen` muted when not; name "Lock <name>", `aria-pressed`; tooltip "Lock" or "Unlock" by what a press does. One step "Lock fixture" / "Unlock fixture".
- The trash works when locked (criterion 32).

### 5.3 Apply fixtures from page (criteria 33 to 37)

- Outline text button 244 x 28px under the last job fixture: `CopyPlus` and "Apply fixtures from page…". Hidden while no other laser page has a job fixture.
- A press unfolds in place, under the button (which shows the pressed look, `aria-expanded`): a `role="radiogroup"` "From page", rows 28px (page name 13px, "4 job fixtures" 12px muted tabular at the right), then a button row: **Apply <n> fixtures** (Primary, 140px, only once a page is picked, hidden before) and **Cancel** (outline, 96px).
- Picking a page draws its job fixtures on this page as the preview (`--preview-new` dashed, no fill, no handles, not hit). Cancel, Escape or a second press on the button folds the list and drops the preview; no step.
- Apply: one step "Apply fixtures from page"; the copies keep positions, names, kinds, sizes, grids, Lock and eye; their jobs Output off; they are added after the page's own; the list folds, the copies are not selected (F7), focus returns to the button.
- Copies partly outside the work area: notice (section 10). Never mirrored.

## 6. Pin and screw (criteria 18 to 24)

### 6.1 Unfolded row body (indented 16px)

| Row | Pin | Screw |
|---|---|---|
| Size | **Diameter** field 104px, "mm", 0.5 to 1,000 | none (Head and Thread stay on the canvas knobs, `0102` 4.3) |
| Columns unit | **Columns** 104px (1 to 50) and **Spacing** 104px ("mm", 1 to 1,000); Spacing absent at 1 | same |
| Rows unit | **Rows** and **Spacing** as above | same |
| Jobs heading | "Jobs" 12px semibold muted | same |
| Centre mark unit | toggle 28px high: `Crosshair` and "Centre mark" 13px; while on, **Size** 88px ("mm", 0.5 to 50, default 4) beside it | same |
| Diameters unit | toggle: `Diameter` and "Diameters" | same |

- Each count unit and the Centre mark unit wrap as a whole (`0111` draft 07); in the 212px body each unit fits one line, so the units stack as in screen 07.
- Field commits: one step "Resize fixture" (Diameter, counts, spacings); Size: "Edit fixture job" (F8). Out of range: set to the nearest limit with the Action notice under the field (`0102` 4.1 pattern, section 10). A count whose product would pass 400: set to the largest allowed count with the notice.
- Jobs toggles: `aria-pressed`, name "Centre mark" / "Diameters", description "<fixture name>"; each press is the job's Output switch, one step "Output on" / "Output off" (`0103` 4.4). The same state shows on the job row in the Operations tab.

### 6.2 On the canvas

- **Pin:** a circle at its diameter, `--toolbar-bg` opaque ground, 1.5px ink outline with casing (F1). A grid draws every pin. Not a snap target, not a fit target (criterion 23).
- **Centre mark** (Output on): at every centre, a circle of Size diameter and a cross whose lines span it, in the Score look; Output off: in fixture ink, 1px. **Diameters**: the pin circle, or the screw's head and thread rings, in the Score look over the fixture's own line; off: not drawn twice.
- **Selected pin or screw grid:** the dashed `--accent` box around every pin; the centre move (M: Move chip); a **Diameter** knob on the first pin's circle at 0° (pin only; a screw keeps its Head knob at 0° and Thread knob at 180° on the first screw); a **Columns** knob at the centre of the last pin of row 1; a **Rows** knob at the centre of the last pin of column 1. Parameter handle look, 12px hit radius. Knobs shown when the first pin is at least 72px across on screen (Diameter, Head, Thread) and when their count is above 1 (Columns, Rows, F9).
- Drag of a Columns or Rows knob changes the spacing (the count stays), readout "Spacing 120.0 mm" at the pointer; at a limit "Spacing 1.0 mm min". Double-click opens the Transform entry chip with **Columns** and **Spacing** (or Rows and Spacing); Enter commits once, Escape cancels. Each commit one step "Resize fixture".
- No rotate glyph, no R, no skew (criterion 21).
- Tag (`0102` 6.3): glyph and "Pin", "Screw".

### 6.3 The two fixed jobs (criterion 27)

- In the Operations tab's Fixture jobs section, a pin's or screw's group lists "Centre mark" and "Diameters" as job rows with Show, Output, line 2 (`0103` 4.1). Their trash is absent (hidden, not disabled) and they are not renamed (F10): they live and die with the fixture.
- Their Parts list shows the fixed parts with a static `Check` 16px glyph instead of a check box (`aria-readonly`): Centre mark "Marks"; Diameters "Circle" (pin) or "Head", "Thread" (screw). No "Select" route beyond the fixture.
- With a pin or screw selected, the "Add fixture job" toolbar of `0103` 3.3 is empty (no further jobs, criterion 27).
- Line 2 priority (criterion 28): "not on this page", "not on the bed", "Output off", the part count.

## 7. Centre snapping and Lock (criteria 30 to 32)

- **Snapping:** while a job fixture hangs at the pointer or is dragged, each of its centres snaps to the Objects target's centres of `0060` (circle, ellipse, polygon, box of any other shape) with its capture, release, hysteresis and Alt rules; the marker shows (red lines along the matched axes, a cross at the point, no text). Only while the Objects target is on. Set fixtures do not centre-snap.
- **Select bar of one selected job fixture:** one group, the **Lock** toggle: 28px high text toggle, `Lock` 16px and "Lock" 13px; pressed look while locked; name "Lock", `aria-pressed`. Nothing else in the bar: the settings switches, Flip and every other group leave the tree (criterion 31, section 14 M7). Set fixtures keep no Select bar (`0102` 6.4).
- **Locked:** no drag from the outline or the box, no nudge, no squircles, knobs, centre move, Move chip or S chip; the box is drawn dashed without handles; a drag on it does nothing and starts no marquee; the pointer over it shows the "Lock cursor" (arrow with padlock). Still selectable, shown, listed, switched by eye and check, runs its jobs, removable by the trash.
- **Lock badge:** a 16px chip at the upper right of the fixture's box (4px outside, screen-constant), `aria-hidden`, `pointer-events: none`; drawn whether selected or not; one per fixture (a grid of pins: one badge on its box).
- Lock and Unlock: one step each; row and bar show the same state.

## 8. Layers tree and Operations tab

- Layers Fixtures row: children in the order sets (in order, each with its fixtures in creation order), then job fixtures (criterion 16). Fixtures of a set that is off: listed muted with "not on this page", not pressable. Job fixtures: no lock glyph of their own (the Layers lock column stays the layer lock; Lock is shown in the Fixtures tab and on the canvas).
- Operations tab: Fixture jobs grouped in the same order.

## 9. Keyboard routes

| Action | Route |
|---|---|
| Sections | Tab: Machine sets list (one stop), + Set, the open set's add bar, its fixture rows, Job fixtures add bar, job rows (one stop), Apply button, the Apply list |
| Set rows | Up, Down, Home, End; Enter or Space folds or unfolds; F2 renames; Delete arms; Alt+arrows reorder; Left and Right reach check, trash, arrows |
| On this page, On the bed, Lock, eye | Left and Right to the control, Space or Enter |
| Add bars | one Tab stop each, arrows move, Enter or Space presses (job kinds arm placement) |
| Placement by keyboard | arrows move the hanging fixture (Shift 10 mm), Enter drops, Escape ends |
| Job toggles, fields | Space toggles; Enter commits, Escape reverts, Up and Down step |
| Canvas knobs | the chip by double-click; S opens no chip for a pin grid (F11); M the Move chip |
| Lock in the bar | Tab to it, Space |
| Apply list | arrows choose a page; Tab to Apply and Cancel; Escape folds |

## 10. Notices and status (each at most 70 characters)

| Text | Where | Role, time |
|---|---|---|
| "Diameter is 0.5 to 1,000 mm. Set to 1,000 mm." | Action notice under the field or chip | status, 3 s |
| "Columns are 1 to 50. Set to 50." / "Rows are 1 to 50. Set to 1." | same | status, 3 s |
| "Spacing is 1 to 1,000 mm. Set to 1 mm." / "Size is 0.5 to 50 mm. Set to 50 mm." | same | status, 3 s |
| "At most 400 pins. Columns set to 20." (or "screws", "Rows") | same | status, 3 s |
| "4 fixtures lie outside the work area." | canvas notice slot, after Apply | status, 5 s |
| "2 values damaged in Set 1." / "2 values damaged in Job fixtures." | canvas notice slot, on opening the file | status, 5 s |
| "Press again to delete Coaster jig. Its 2 fixtures go with it." | sr-only status on arming | polite |
| "Press again to delete Pins 1." / "Pins 1 removed." | sr-only status | polite |

No notice after an add, a drop, a lock, an Apply inside the work area, a check or a rename. No notice when a limit hides an add control (criterion 17).

## 11. Undo steps

"Add fixture set", "Rename fixture set", "Remove fixture set", "Move fixture set", "Set fixture set on page", "Set fixture set off page", "Add fixture" (in a set), "Add job fixture", "Rename fixture", "Remove fixture", "Set fixture active", "Set fixture inactive", "Show or hide fixture", "Move fixture", "Resize fixture", "Lock fixture", "Unlock fixture", "Output on", "Output off", "Edit fixture job" (Centre mark Size), "Apply fixtures from page", and the profile steps of `0102`. No step for folding, arming placement, picking a page in the Apply list, or a selection.

## 12. Empty, error, locked, mixed and large states

- **Generic laser, no set:** "Machine sets" with "+ Set" only; no count.
- **Set with no fixture:** line 2 "No fixtures"; unfolded, its add bar alone.
- **No job fixture:** the add bar; Apply hidden unless another page has job fixtures.
- **Set off:** muted row, "not on this page", nothing on the canvas, its fixture rows listed muted with the same words, its jobs "not on this page".
- **Fixture off inside a set that is on:** "not on the bed" (`0102`).
- **Locked:** section 7.
- **Mixed:** a fixture cannot be multi-selected (`0102` 6.1); no mixed state.
- **Errors:** damaged values read as defaults with one notice per set (criterion 40); a damaged profile record is skipped with its notice (`0102`); a newer record as `0026`.
- **Old files:** page fixtures read as "Set 1", on; nothing shown, nothing written.
- **Large:** 16 sets and 64 job fixtures: set rows and job rows are windowed (fixed rows, the unfolded set and the unfolded fixture are variable items, at most 60 row elements); first rows within 100 ms; a 400-pin grid with both jobs draws as one batch per kind; hit within 2 ms, draw list within 16 ms, a snapping drag move within 16 ms, Apply of 64 within 100 ms (criterion 42).

## 13. Decisions for veto (Proposal)

- **F1** A pin has an opaque `--toolbar-bg` ground (screen 07 C), as a solid dowel.
- **F2** The hanging fixture and the Apply preview are `--preview-new` dashed 4 on 3.
- **F3** An empty set reads "No fixtures".
- **F4** Placement stays armed after a drop; Escape ends it.
- **F5** Keyboard placement starts at the work area's centre.
- **F6** Screw size in line 2: "M6" for a whole-millimetre thread, else "⌀ 6.5 mm".
- **F7** Applied copies are not selected.
- **F8** A Centre mark Size commit is "Edit fixture job".
- **F9** Columns and Rows knobs only when their count is above 1.
- **F10** The two fixed jobs have no trash and no rename.
- **F11** S opens no chip for a pin or screw grid (no single size to type).
- **F12** Line 2 of a job fixture appends the jobs that are on.
- **F13** One unfolded fixture row in the whole tab.
- **F14** Layers shows no Lock for job fixtures.
- **F15** Apply's button is hidden until a page is picked.
- **F16** The tab's subject line names the selected fixture.

## 14. Mismatches for the product owner

- **M1** Criterion 7 names the add step "Add fixture"; `0102` built "Add <kind>" ("Add screw"). The notes use the spec's names ("Add fixture", "Add job fixture"); the built label changes.
- **M2** Criterion 12's line 2 lists kind, count, size and "locked"; screen 06 also lists the jobs that are on. The notes append them last (F12).
- **M3** Criterion 13 does not say whether placement stays armed after a drop; screen 08 places four pins in a row. F4.
- **M4** Resolved: criterion 21 hides the Columns and Rows knobs at count 1 (F9).
- **M5** Criterion 25 shows the Jobs toggles "in its row (and its unfolded body)"; the 40px row has no room; the notes show them in the unfolded body and name the jobs that are on in line 2.
- **M6** Criterion 27 does not say whether the fixed jobs can be renamed; F10.
- **M7** Criterion 31 makes Lock the only control of the Select bar; "Select bar layout" says the settings switches are always shown. A design-system exception, like the entered group of `0111`.
- **M8** Criterion 34 asks for a "dashed blue live preview"; the design system's live preview outline is solid. F2 adds the dashed variant.
- **M9** Criterion 18 asks only for an outline; screen 07 C fills the pin. F1.
- **M10** Screens 06 and 07 B are superseded by Frage 211 B (the On the bed check in set fixture rows) and the reduced P16 (no Head and Thread fields).
- Design system rows to add at acceptance: "Fixtures tab (`0113`)", "Machine set row (`0113`)", "Job fixture row and Lock (`0113`)", "Placement by centre (`0113`)", "Pin and screw grid handles (`0113`)", "Apply fixtures from page (`0113`)"; move the "Fixtures block" rows of `0102` to the Fixtures tab. No token.

## 15. Implementer checklist

- Tab: Holder glyph after Operations and Materials, laser pages only; Document tab without the Fixtures block; two sections with the count heading.
- Sets: folded rows with chevron, name, line 2, On this page, armed trash, arrows, drag with the insertion line; rename in place; + Set; unfolded add bar (Grid hidden when present) and `0102` rows with their own On the bed check; the both-switches rule on canvas, hit, snap and jobs.
- Job fixtures: symbol add bar that arms placement, hanging dashed outline, click drops, stays armed, Escape ends, keyboard placement; rows with check, Lock, eye, trash, line 2; limits hide the add controls silently.
- Pin and screw: Diameter (pin only), Columns and Rows units with Spacing hidden at 1, clamping notices, 400 limit; Jobs toggles as the jobs' Output with Size; canvas look, knobs, chips, readouts, no rotate.
- Fixed jobs in the Operations tab: no trash, no rename, static part ticks, "not on this page" first in line 2.
- Centre snapping with the `0060` marker; Lock in row and bar, locked look, Lock cursor, badge, trash still works.
- Apply: in place list, preview, Apply <n>, Cancel and Escape, one step, Output off on copies, outside notice, never mirrored.
- Machine block row "Fixture sets in profile: N"; profile pick and Reset keep job fixtures.
- Steps of section 11; texts of section 10 at 70 characters at most; no success notice; no popup; no new colour; windowing; focus never to `body`.

## Links

Spec: `specs/0113-fixture-sets-and-pins/specification.md`. Drafts: `docs/design-drafts/2026-11-laser-redesign/` screens 06 to 09, README P14 to P18. Built notes: `specs/0102-laser-fixtures/ux-notes.md`, `specs/0103-fixture-jobs/ux-notes.md`, `specs/0060-snapping/ux-notes.md`. Before: `specs/0111-laser-operation-tools/ux-notes.md`, `specs/0112-test-grid-and-materials/ux-notes.md`.
