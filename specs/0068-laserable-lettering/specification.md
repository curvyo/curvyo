# Weld: overlapping shapes become one object in which every visible line exists once

Status: Done (PR #169, merged 2026-10-23). Reworked after the customer rejected the design of PR #167; Questions 14, 15 and 16 decided 2026-10-20, Frage 24 WELD-STYLE 2026-10-23.
Priority: Should
Origin: Customer (request of 2026-10-17; correction of 2026-10-20: "Weld is a tool on the Boolean card, next to Union and Flatten. It does not make an outline, Union does that. It is more or less a Flatten, but you get ONE object. With Flatten you still have two objects, and the laser would cut the lines twice. This also handles different stroke widths, because the paths underneath no longer exist."). Items marked **(Proposal)** or **PO** are the product owner's and vetoable.
Depends on: `0037-fracture-and-flatten` (visible region, the Flatten fold), `0016-boolean-operations` (kernel, region, refusals, determinism, Boolean card), `0054-combine-open-paths` (compound paths with open outlines; open operands closed by a straight line), `0074-text-objects` (Object to path for text), `0023-groups`, `0020-undo-redo`.

## User value

As a maker who lasers, scores or plots drawings built from overlapping shapes and letters, I want to press one button and get one object that holds exactly the lines I see, each line once, so that the laser never cuts or scores a line twice, no hidden part of a lower shape is cut, and a thick stroke on a lower shape cannot show through.

## Field reference

| Tool | What it does | Where it falls short |
|---|---|---|
| LightBurn | Weld joins shapes into one outer outline (a multi-shape Union). Its optimizer has "Remove overlapping lines" at output time. | Weld throws away the inner lines; removing overlaps is an output setting, invisible on the canvas. |
| Inkscape | Path > Division or Flatten-like chains; extensions ("Remove duplicate lines") clean up coincident segments. | Several steps; the result is still several objects with shared edges. |
| Illustrator | Pathfinder Trim/Merge, then Outline. | Outline gives open segments without style; several objects. |
| Our Flatten (`0037`) | Trims each shape to what shows. | One object per shape; a trim edge is in both neighbours and is cut twice. |

**What we do better.** One press on the Boolean card, on shapes, texts and groups together. The result is visible on the canvas before any job is made: one object, each line once, hidden lines gone.

## Definition: Weld against Union and Flatten

**Visible region** of an operand O: the region of O minus the union of the regions of all operands above O (`0037` criterion 10).

**Weld lines**: every point that lies on the boundary of at least one visible region. Where the boundaries of two visible regions coincide (a trim edge, a shared edge), that line is in the result once.

| Question | Union (`0016`) | Flatten (`0037`) | **Weld** |
|---|---|---|---|
| Number of results | one object | one per visible operand | **one object** |
| Outer outline | yes | split over the objects | **yes, closed** |
| Upper outline lying over a lower shape | dissolved | kept, in two objects (cut twice) | **kept, once** |
| Hidden part of a lower outline | dissolved | removed | **removed** |
| Edge shared by two shapes that touch | dissolved | in both objects | **kept, once** |
| Completely hidden operand | absorbed | removed | **removed** |
| Groups | refused | refused | **accepted, any depth** |
| Text | converted first | not accepted | **converted first** |

Lead's reading checked against `0016`, `0037`, `0054`: correct, with five corrections.
1. The inner lines are **open outlines** of the compound path, not closed regions. A compound path of closed regions would hold every shared edge twice, which is what Weld exists to prevent (`0054` allows open outlines in a compound path; no format change).
2. Weld is enabled with **one** selected object too (a text, a group, a compound path), unlike the other Boolean buttons: one text is the normal case.
3. Open operands are closed by a straight line, as in Flatten (`0054` criteria 21, 22, 24). The stroke band of PR #167 is dropped.
4. Cutting the inner lines separates the parts on a laser. A name plate in one piece is Union's job.
5. Fill: see Question 14.

## Terms

- **Operand:** every leaf the selection holds: path, compound path, rectangle, ellipse, polygon, star, text, and every such leaf inside a selected group at any depth. Linked clones count as the path they show (`0049`).
- **Stacking order:** the document's, never the selection order (`0016` criterion 9). The letters of a text are stacked in the order Object to path gives them (`0074` criterion 22).
- **Region** of an operand: its fill area under the nonzero rule, whatever its paint or stroke (`0016` criteria 4, 5; `0054` criterion 21 for open outlines). Lines inside one operand's own region (a glyph whose contours overlap, a Combine of overlapping shapes) are not boundaries and are not in the result.
- **Base operand:** the bottom-most operand.
- Kernel tolerance 0.01 mm and grid 0.001 mm as `0016` criteria 25 and 39. Lines closer than one grid unit are coincident.

## Acceptance criteria

### Entry point

1. Given the Boolean card, then Weld is its eighth button, after Flatten, with the rules of `0016` criteria 1a, 2, 3 and 18 (command, not tool; roving focus; no shortcut; no dialog). It is enabled when the Select tool is active and **one or more** objects are selected, and dimmed otherwise with the card's dimmed look. Its tooltip names it "Weld" with a rule line chosen by the ux-engineer.
2. Given the app after this slice, then there is no Weld group, no Weld button and no Gap field in the Select bar, and no stored Gap setting.
3. Given a press, then nothing opens: one press, one result. Success and refusal notices are the Boolean card's (`0037` UX notes, "Interaction and notices").

### What the result holds

4. Given two squares of side 20 mm, the lower at (0, 0) to (20, 20), the upper at (10, 10) to (30, 30), when welded, then the result is one compound path of two outlines: a closed outline of 8 nodes enclosing 700 mm², and an open outline (10, 20), (10, 10), (20, 10). The total line length is 140 mm (Flatten gives 160 mm, the 20 mm trim edge twice).
5. Given a disc of radius 10 mm on top of a concentric disc of radius 20 mm, then the result has two closed outlines (radius 20 and radius 10); the inner one is wound as a shape, not a hole. Given the small disc below the large one, then the result is one closed path of radius 20 and the notice adds "1 hidden shape was removed."
6. Given two squares of side 20 mm that share a full edge, then the result is a closed 4-node rectangle 40 x 20 mm and one open 2-node outline along the shared edge (length 140 mm in total). Given two squares that touch only at a corner, then the result is two closed outlines and no open outline.
7. Given two identical squares on top of each other, then the result is one closed 4-node path.
8. Given a lower square with Stroke 5 mm and an upper square overlapping it with Stroke 0.1 mm, then no segment of the result lies inside the upper square's region (the hidden part of the lower outline is gone), and the result has one stroke width (criterion 18).
9. Given a ring (disc r = 20 mm minus a concentric disc r = 10 mm), a bar 50 x 4 mm on top across its outer part, and a disc r = 5 mm inside the hole, then the hole stays a hole, the disc stays an island, and the bar's outline over the ring is kept as open outlines.
10. Given one selected compound path whose outlines overlap, or a text in a script font whose glyphs overlap themselves, then the lines inside each operand's own region are not in the result (Terms, Region).
11. Given one selected text, then it is converted as by Object to path and welded in the same history step, without asking. Given texts, groups (nested) and other objects together, then all their leaves are operands.
12. Given one closed object that overlaps nothing, then Weld runs and the result is its outline(s) as a path. Nothing is refused for having nothing to weld.
13. Given an open operand, then it is closed by a straight line with the notice sentences of `0054` criteria 21 and 22. **Superseded by the customer rule of 2026-10-31 (docs/design-system.md, "Interaction conventions"): no success notice after Weld, so no open-outline sentences.**

### Exactly once

14. Given any result, then no two of its segments overlap over a length of more than 0.001 mm; every point of the result lies within 0.001 mm of a visible-region boundary; every point of a visible-region boundary lies within 0.001 mm of the result. Test: property test on at least 200 random shape sets, fixed seed.
15. Given the result, then the boundary of the union of all regions forms its closed outlines (outer, holes, islands; winding as `0016` criterion 41). Every other weld line is an open outline that starts and ends where it meets another line, or a closed outline wound as a shape when it meets no other line. The result is an ordinary closed path when it is one closed outline and nothing else; otherwise one compound path.
16. Given the result, then its nodes obey `0016` criteria 24, 24a and 25 (Corner nodes, straight segments within 0.01 mm of the exact lines, no near-duplicate or collinear nodes).

### The result

17. Given a success, then all operands are removed (a selected group with all its members), and one new object takes the place of the lowest selected object in the stacking order; inside an entered group it stays in that group. Unselected objects keep their ids and order.
18. Given a success, then the result's style is a copy of the base operand's style (customer, Frage 24 WELD-STYLE A, 2026-10-23: the rule of Union and Combine), except for Fill as Question 14 decides. Default (14 A): Fill is off; if the base had Stroke off, Stroke is on in the base's fill colour at the default stroke width of a new path.
19. Given a success, then the result is the only selected object, the tool stays Select, and the Properties panel reads "Compound path" or "Path".
20. Given a success, then exactly one commit `weld` is written, shown as "Weld" in History; Ctrl+Z restores texts (editable), groups and objects with their ids and order.
21. Given a success, then the notice reads "Weld: 3 objects became 1 compound path with 412.3 mm of line." (`<n>` counts the selected objects, a group once; "1 text became ..." when only texts were selected; "1 path" for a single closed outline; the length is the result's total line length in the display unit with the status bar's decimals; criterion 4 reads "with 140.0 mm of line"), followed when they apply, in this order, by: "1 hidden shape was removed." ("3 hidden shapes were removed."); the sentences of `0054` criteria 21 and 22; "It uses the style of the lowest object." when operand styles differed; under 14 A, "Fill was switched off." when the base had Fill on, and "Stroke was switched on." when the base had Stroke off (UX notes, "Notices"). **Superseded by the customer rule of 2026-10-31 (docs/design-system.md, "Interaction conventions"): no success notice after Weld.**
22. Given a welded result saved and reopened, then outlines, open or closed state, node order and style are unchanged. Under default 14 A there is no `format_version` change.

### Refusals

23. Given a selected text whose face is missing, then Weld is refused with the notice of `0074` criterion 29, the text outlined in red, nothing changed.
24. Given operands that together enclose no area, or a coordinate that is not finite or beyond `MAX_COORDINATE_MM`, then Weld is refused with the sentences of `0016` criteria 16 and 17a with "Weld" as the name, offenders outlined in red, nothing changed (`0016` criterion 18).

### Determinism, budgets

25. Given the same objects at the same stacking positions, then the result is node for node the same however they were selected, and golden files agree to 1e-6 mm on Linux, Windows, macOS and the browser build.
26. Given the code, then the visible regions come from the Flatten fold of `0037`; Weld adds no second boolean algorithm and no dependency without the architect's note in `adrs.md`.
27. Given the golden fixtures in `tests/fixtures/weld/`, then each welds in a release build on the CI runner (browser build up to twice as long) within budget and matches its golden result: (a) "Annabelle" in Great Vibes at Size 20 mm: under 300 ms; (b) 500 characters of Great Vibes at 10 mm: under 2 s; (c) the fixtures (a) to (i) of `0016` criterion 40: under 2 s each, a valid result or a refusal of criterion 24; (d) the 20 squares of `0037` criterion 17 (a): one closed 105 x 10 mm outline and 19 open 10 mm lines, 420 mm in total; (e) 2,000 such squares: under 2 s.

## Out of scope

- One outer outline without inner lines: that is Union (`0016`).
- Gap closing and a Gap field; the Select-bar placement of PR #167.
- Stroke bands of open lines; strokes as area (decided WELD-STROKE A stays: regions only).
- Fill None as "does not hide" (`0037` Question 2 A applies: every operand hides what is below).
- Joining near-coincident lines farther apart than 0.001 mm.
- Bridges, live weld on a text, kerf, cut order, lead-ins, keeping curves (refit is a `0016` follow-up).
- Paint for the regions between the lines (Question 14 B, not chosen).

## Needs from the architect (`adrs.md`)

1. The edge arrangement from the visible regions of `0037`: merging coincident boundary segments on the grid, chaining into closed and open outlines deterministically (criteria 14, 15).
2. Collection of operands through groups, clones and converted texts in one commit; placement when the lowest selected object is a group.
3. What can be removed from the PR #167 branch (gap closing, stroke band, Select-bar wiring, Gap setting).
4. Question 14 was answered A: no stored mark, no `format_version` bump.

## Decided by the product owner (vetoable at the demo)

- Weld runs on one object or more, on texts and on groups (permit over refuse).
- Inner lines are kept even though cutting them separates the parts; one piece is Union's job.

## Decided by the customer 2026-10-20

- Frage 15 WELD-DEF: the definition above is confirmed as written ("genau so"): the Boolean card, button 8; Flatten's visible regions as one object; every visible boundary line once; the union boundary closed; inner visible lines open; overlapping script letters are cut apart (one-piece plates are Union's job); the bottom-most operand's style.
- Frage 14 WELD-FILL: **A**. Line art: fill off, stroke on (if the bottom-most shape has no stroke, the stroke takes its fill colour); no format change.
- Frage 16 WELD-MINH: **A**. The Tauri window's minimum height rises from 600 to 640 px, so column B with eight Boolean buttons fits.

## Decided by the customer 2026-10-23

- Frage 24 WELD-STYLE: **A**. The result keeps the style of the lowest object, as Union and Combine; the notice says when styles differed.

## Open questions (customer; each has a default)

None.

## UX notes

By `ux-engineer`, 2026-10-20, for the reworked specification (Frage 15 WELD-DEF, Frage 14 WELD-FILL A).
These replace the UX notes of PR #167. Rows in `docs/design-system.md` that change with this slice:
"Boolean toolbox", "Tool rail architecture", "Command glyphs", "Boolean glyphs", "Boolean tooltip",
"Boolean preview", "Action notice", "Window minimum". The ux-engineer edits them on `story/weld-v2`.

### Placement

Button 8 of the Boolean card, appended after Flatten: **Union, Difference, Intersection, Exclusion,
Reverse difference, Fracture, Flatten, Weld**. No other button or card moves. The toolbar keeps the
name "Boolean operations". There is no Weld control anywhere else: no Select-bar group, no Gap field,
no menu entry, no shortcut (criteria 1, 2).

The card grows from 312 to 356 px (8 x 40 + 7 x 4 + 2 x 4). The rail now holds 22 buttons
(Tools 7, Group 2, Path tools 1, Path 4, Boolean 8), which is the cap of the row "Tool rail
architecture". The next command for the selection goes to the Select bar or the Properties panel.
The sentence in `0037` ("A later command ... (for example Weld of `0068`) goes to the Select bar")
is superseded by the customer's decision of 2026-10-20.

### Column B at the minimum window (the overflow)

Arithmetic from `main`: the smallest Tauri window (800 x 600) leaves a 546 px canvas viewport
(`ToolRail.tsx`). The rail root is inset 12 px at the top and 4 px at the bottom, so a column has
530 px. Column B becomes Path 180 + 8 + Boolean 356 = **544 px, 14 px too tall** (the bottom card
padding, the shadow and 10 px of the Weld button run under the status bar). The PO's "about 4 px"
was measured against the status bar top without the rail's 4 px bottom inset and with the 0037
figures; 14 px is the figure to plan with.

Moving cards cannot fix it. Column A holds 468 px and has 62 px free, which is too little for any
column B card (Path is 180 px, and Boolean is 356 px). Swapping cards breaks "cards keep their
place" and the rule that tools stay in column A.

**Decision: raise the window minimum height from 600 to 640 px** (`tauri.conf.json` `minHeight`;
the width stays 800). At 640 the viewport is 586 px, the column has 570 px, and column B ends with
26 px clear. Nothing on the rail moves, shrinks or scrolls, and every button stays a full 40 px
target. The browser build keeps the existing "Too short" rule: below the needed height the cards
under the Path card scroll inside column B, and a focused button scrolls into view.

Rejected:
- 4 px gaps between the cards of column B only. That leaves 10 px over, and the two columns would
  get two spacings.
- Card padding of 2 px. A global change that still misses by 6 px.
- 36 px buttons. A second button size on the rail.
- A scroll in Tauri at the minimum. Half of the Weld button would be hidden in the window the
  rail was designed for.
- Moving the Path card to column A. Column A would need 656 px.

Re-measure at the build: the Select bar at 800 x 640 (4 rows with a text) and the Properties
panel. Both only gain height.

### States

| Situation | Weld | Buttons 1 to 7 |
|---|---|---|
| No object selected, or a tool other than Select | dimmed | dimmed |
| **One** object selected (path, shape, compound path, text, group), Select tool | **enabled** | dimmed |
| Two or more objects, Select tool | enabled | enabled |
| Open outlines among the operands | enabled; closed with a straight line (`0054` criteria 21, 22, 24) | as built |
| A selected group | enabled; its leaves at any depth are operands | enabled; refused on press (as built) |
| A text whose face is missing | enabled; refused on press (criterion 23) | as built |
| Nothing with area, or out of range | enabled; refused on press (criterion 24) | as built |

Dimming is the rail's: `aria-disabled="true"`, 40 % opacity, still focusable, the tooltip stays open
on a press, and activation does nothing. With one object selected, Weld is the only bright button on
the card. That difference is intended and is not labelled.

### Glyph

"Flatten with the lines merged into one", drawn as line art, because the result is line art (fill
off). The glyph uses the squares of the row "Boolean glyphs". It has **no fill, no gap and no
second piece**: the outer silhouette of both squares as one 1.5 px outline, plus the upper square's
two edges that lie over the lower square, drawn once, as one open line. The lower square's edges
inside the overlap are not drawn, because they are hidden.

```tsx
/** The silhouette of both squares: outer corners rounded by 1, the two inner corners sharp. */
const WELD_SILHOUETTE =
  "M2.75 1.75H9.75A1 1 0 0 1 10.75 2.75V5.25H13.25A1 1 0 0 1 14.25 6.25V13.25A1 1 0 0 1 13.25 14.25H6.25A1 1 0 0 1 5.25 13.25V10.75H2.75A1 1 0 0 1 1.75 9.75V2.75A1 1 0 0 1 2.75 1.75Z";
/** The trim edge: the upper square's top and left edges over the lower square, once. */
const WELD_TRIM_LINE = "M10.75 5.25H6.25A1 1 0 0 0 5.25 6.25V10.75";
/** The two joints where the trim line meets the silhouette (added in the UX review). */
const WELD_JOINTS = [[10.75, 5.25], [5.25, 10.75]];

case "weld":
  return (
    <>
      <path d={WELD_SILHOUETTE} fill="none" {...STROKE} />
      <path d={WELD_TRIM_LINE} fill="none" strokeLinecap="round" {...STROKE} />
      {WELD_JOINTS.map(([cx, cy]) => <circle cx={cx} cy={cy} r={1.6} fill="currentColor" />)}
    </>
  );
```

**Joint dots (UX review, 2026-10-20):** without them the line art is Lucide `Copy`, the Duplicate
glyph of the Layers toolbar, at 1x and 2x. The two dots mark where the inner line meets the outline,
which Copy never has.

How it reads apart from its neighbours at 1x:
- **Union:** a solid silhouette. Weld is hollow, with an inner line.
- **Flatten:** two solid pieces with a 2 px gap. Weld has no fill, no gap and one continuous outline.
- **Intersection:** two whole outlines, which cross each other, and a solid overlap. Weld has only two
  small joint dots, and the lower square's corner inside the overlap is missing.
- **Exclusion:** two solid L pieces. Weld has no solid piece.

The rule "solid = result, outline = removed" of the row "Boolean glyphs" gets one stated exception:
**a glyph with no solid shape shows a line-art result** (Weld only). The clear interior of the
lower L is 3.5 units (4.4 px at 20 px, about 2.9 px between the strokes), which is above the 2 px
minimum of "Command glyphs".

The glyph sheet at 1x, 1.25x and 2x (greyscale) adds the pairs Weld / Union, Weld / Flatten and
Weld / Intersection. **Fallback** if Weld / Intersection does not read apart at 1x: the Union
silhouette solid, with the trim line knocked out as a 1.5 px line (one object with an inner cut
line). The button never grows and gets no text.

### Tooltip

The three lines of the Boolean buttons (row "Boolean tooltip"): name (semibold), rule, note.

- Name: **"Weld"**. This is also the accessible name.
- Rule: **"Keeps each visible line once, in one object."** That is 44 characters, the length of
  Difference's rule, so it fits 260 px without a wrap. It describes the command at name level, as
  "Trims each shape to what shows." does for Flatten. It is not an operating tip.
- Note, first match wins:
  - Nothing selected, or a tool other than Select: "Select one or more objects.", followed by
    "Use the Select tool." when the tool is not Select. These are Weld's own words, because its
    predicate differs (adrs decision 7).
  - Open outlines that a press closes: the shared "Closes 2 open outlines with a straight line."
  - Otherwise: "Replaces the selection."
- No preview line. Weld has no preview, as Fracture and Flatten (row "Boolean preview": add Weld to
  its "No preview" list).
- No sentence about text conversion, fill or hidden shapes. The notice reports them after the press
  (no hint labels, 2026-10-16).

Implementation: `weld` becomes a third `StackCommand`-like entry with its own `name`/`rule`, and
`tooltipNote` gets the one-or-more variant next to the existing one. That is a second predicate, not
a parameter on the card's (adrs decision 7).

### Interaction

One press, one result, nothing opens (criterion 3). This is a command with a visible trigger: no
dialog, no flyout, no chip.

- **Mouse press:** the focus returns to the canvas. **Key press** (Space or Enter): the focus stays
  on the button.
- **Keyboard:** the card stays one Tab stop. Down, Up, Home and End move the focus; End now lands on
  Weld. There is no shortcut, as for every Boolean button (`0020` Question 10).
- **Busy:** synchronous on the UI thread, like Flatten. `aria-busy` on the card, the pressed ground
  stays on Weld, and the cursor is `wait` over the window. The busy state is painted (two frames)
  before the call. Other activations are ignored.
- **After a success:** the result is the only selected object and the tool stays Select
  (criterion 19).

### The result on the canvas once selected

- The result is drawn in the base operand's stroke, **without fill**. Under 14 A, a base without a
  stroke gets one in its fill colour at the default width. The selection box is the single-object
  box (no member boxes) around the result's bounds; its handles are as for any path.
- Hit test: with fill off, the result is hit on its lines only (the existing rule for unfilled
  paths). Clicking inside an empty region selects what is below or nothing. This is intended for
  line art and is not labelled.
- The Properties panel reads "Compound path" (or "Path" for a single closed outline), with Fill
  None and Stroke on.
- Node tool: the existing compound-path rule applies ("Nodes of compound paths cannot be edited
  yet."). Nothing new.
- Large results (the 2,000 squares of fixture (e)): one object, so one box. The canvas draws one
  path with about 2,000 outlines, and the existing draw path handles it. There are no per-outline
  overlays.

### Notices

The Boolean card's notice (row "Action notice"): level with Union, success `role="status"` for
3 / 5 / 8 s by length (70 and 140 characters), refusal `role="alert"` for 8 s. Each sentence appears
only when it applies, in this order.

1. **Count and length:** `Weld: <n objects | n texts> became 1 <path | compound path> with <L> of
   line.`
   - `<n>` counts the **selected** objects (a group counts once). It uses "text" and "texts" when
     only texts were selected.
   - `<L>` is the result's total line length in the display unit, with the fixed decimals of the
     status bar (mm 1, cm 2, in 3).
   - Examples:
     - "Weld: 2 objects became 1 compound path with 140.0 mm of line."
     - "Weld: 1 text became 1 compound path with 1312.6 mm of line."
     - "Weld: 1 object became 1 path with 80.0 mm of line."
   - Why the length: it is the one number that shows on screen that every line is there once
     (Flatten of criterion 4 gives 160 mm, Weld gives 140 mm), and it is the cut length the maker
     plans with. "became 1" alone (the lead's draft) drops the kind of object, which Union's
     notice states. **PO: criterion 21 gains "with 140.0 mm of line"; criterion 4's figure becomes
     testable in the notice.**
2. "1 hidden shape was removed." / "3 hidden shapes were removed."
   - The word is **shape**, not "object": the hidden operands are leaves and glyphs, which are not
     selected objects (criterion 5 and 21 say "object"; PO to align).
3. The open-outline sentences of `0054` criteria 21 and 22, shared (`openOutlineSentences`).
4. "It uses the style of the lowest object." when the operand styles differed.
5. "Fill was switched off." when the base had Fill on. "Stroke was switched on." when the base had
   Stroke off (14 A: the stroke takes the fill colour, which the canvas shows).

Example: "Weld: 3 objects became 1 compound path with 412.3 mm of line. 1 hidden shape was removed.
It uses the style of the lowest object. Fill was switched off." That is 149 characters, so it shows
for 8 s and wraps to three lines at 360 px.

Edge case, no extra wording: Weld on an earlier Weld result has only open inner lines inside its
own region. The shared `0054` sentence reports them ("... enclosed no area and were removed.").

### Refusals

Nothing is changed. The notice is the refusal chip (`--field-invalid` ring and text, alert glyph,
8 s). Offenders are drawn with the row "Refusal outline": hollow 2 px `--field-invalid`, white
casing, not selectable, gone with the notice.

- **Missing face (criterion 23):** the notice of `0074` criterion 29, unchanged. The text object
  gets the red outline.
- **No area (criterion 24):** this applies only when no operand encloses an area (operands without
  area are otherwise dropped and reported by the `0054` sentence). The outline goes on the
  **offending leaves**, inside groups too, because those are what the maker fixes.
  - "Weld needs shapes that enclose an area. 3 of 3 selected objects have no area. Nothing was
    changed."
  - With one selected object, "1 of 1" reads wrong: "Weld needs shapes that enclose an area. The
    selected object has no area. Nothing was changed." `sharedRefusalText` gets this `of === 1`
    branch for every command.
- **Out of range:** "Weld works only within 10 km of the point 0, 0. 1 of 3 selected objects
  reaches further. Nothing was changed." With one selected object: "The selected object reaches
  further." Offending leaves are outlined.
- There is no "groups", "do not overlap" or "too many" refusal for Weld. A single object that
  overlaps nothing is welded (criterion 12).

### Undo

- One commit, `weld`, shown as **"Weld"** in History.
- Ctrl+Z restores every operand with its id, order and style: texts editable, groups with their
  members, and the selection as it was before the press. The notice clears on the key (a key is
  "the next action").
- Ctrl+Shift+Z (Ctrl+Y) welds again with the same result (criterion 25).
- Converting a text and welding it is the same step (criterion 11). There is no intermediate state
  in History.

### Accessibility and contrast

- Accessible name "Weld". The description is the rule plus the note, as on the other buttons.
- The notice is announced through the live region that is already in the tree.
- Counts and lengths are in the sentence, so colour is never the only cue.
- Contrast is the Boolean card's: refusal text 6.5:1, red outline 5.3:1 on the canvas, notice
  8.3:1.
- The line-art result on the canvas uses the operand's own colours. That is the maker's choice, not
  ours.

### Large documents

- Fixture (b), 500 characters of Great Vibes: under 2 s.
- Fixture (e), 2,000 squares: under 2 s, on the UI thread with the busy state above.
- No progress UI: as with Flatten, a miss is logged in `docs/technical-debt.md` (adrs decision 8)
  and is not covered over with a spinner.

## Links

Requirements: R-FONT-009; related R-EDIT-003, R-EDIT-023
Depends on: `specs/0037-fracture-and-flatten/`, `specs/0016-boolean-operations/`, `specs/0054-combine-open-paths/`, `specs/0074-text-objects/`, `specs/0023-groups/`
ADRs: ADR 0003 (kernel); none new expected under default 14 A
PR: https://github.com/curvyo/curvyo/pull/169 (PR #167, the rejected design, was closed)
