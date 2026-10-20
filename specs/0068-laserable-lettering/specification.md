# Weld: overlapping letters and shapes become one cuttable outline

Status: Ready (2026-10-20: `adrs.md` and the UX notes are in; Question 10 decided by the customer 2026-10-20 (A); the UX notes' amendments to criteria 1, 2, 5 and Terms are applied; builds right after `0074-text-objects`, customer decision; in progress on `story/weld`)
Priority: Should
Origin: Customer (request of 2026-10-17: "a tool to make overlapping lettering laserable"; decisions WELD-SCOPE and WELD-VS-FLATTEN A of 2026-10-17; build order of 2026-10-20: right after `0074`, so that laser name plates work). Items marked **(Proposal)** are the product owner's and not accepted until the customer says so.
Depends on: `0074-text-objects` (text node, Object to path, criterion 24), `0016-boolean-operations` (kernel, region, refusals, determinism), `0037-fracture-and-flatten` (shared engine, customer decision WELD-VS-FLATTEN A), `0054-combine-open-paths` (open outlines in path commands), `0023-groups`, `0020-undo-redo`.

## User value

As a maker who cuts name plates, door signs and cake toppers from wood or acrylic on a laser, I want to select my script lettering (as text, as converted letters or mixed with a few shapes) and press one button, so that the letters become one outline in which every overlap and every line inside an overlap is gone, the laser cuts each edge once, and the name comes out of the sheet in one piece.

## Field reference

| Tool | What it does | Where it falls short |
|---|---|---|
| LightBurn | **Weld** (Ctrl/Cmd+W): joins any number of closed shapes into one outline; "works similarly to Boolean Union" (which takes two). Open shapes grey the tools out. The docs warn that welding can make content inside another shape disappear ("weld text inside a ring, and the text disappears"; fix: group, then Boolean Union). Text has a live "Welded" option in the text bar. No gap tolerance. | Islands inside holes can vanish; open lines are not accepted; no tolerance. |
| Inkscape | Path > Union (Ctrl++) on text after Object to Path; Path > Stroke to Path for single-line strokes. | Two steps for text, no tolerance; a near-touching join stays a hairline gap. |
| Illustrator | Pathfinder Unite or the Shape Builder after Create Outlines. | Text must be outlined by hand first. |
| Cricut Design Space, Silhouette Studio | Weld is the standard script-font workflow, one button. | Not a laser tool; no tolerance. |
| xTool Creative Space | Weld in the top toolbar merges overlapping characters of a text; its docs warn against changing the size after welding. | Text only; letters must be moved to overlap by hand. |
| Glowforge app | No weld; the community sends makers to Inkscape; Premium "Outline" is a workaround. | Nothing built in. |

**What we do better.** One button on text, converted letters, groups and paths together, without converting first. Islands inside holes are kept (no vanishing text inside a ring). Near-touching joins are closed by a gap tolerance that is a visible number, and the result never shrinks and never grows by more than half of it. The notice says how many separate pieces the result has, so the maker sees a loose i-dot before cutting. Open stroked lines weld as the band the canvas draws.

## Definition: Weld against Union and Flatten

**Weld region** = the gap closing (by tolerance g, see Terms) of the union of the **contributions** of all operands. The result is **one** object holding that region.

| Question | Union (`0016`) | Flatten (`0037`) | **Weld** |
|---|---|---|---|
| Overlap between operands | dissolved | lower object trimmed, upper kept, edge between them stays (two objects) | **dissolved: no edge of the result lies inside the region** |
| Number of results | one object | one per visible operand | **one object** |
| Holes inside a letter (o, e, a, B) | kept | kept | **kept** |
| Areas enclosed between letters (a loop closed by the next letter) | kept as holes | not a hole (no shape there) | **kept as holes** |
| Islands inside holes (text inside a ring) | kept | kept | **kept** (LightBurn drops them) |
| Letters sharing an edge exactly | merged | stay two objects | **merged into one outline** |
| Letters touching at one point | two outlines | stay two | **two pieces at g = 0; one piece for g > 0** |
| Letters with a gap narrower than g | two outlines | two | **one outline** |
| i-dots, umlaut dots, separate letters | separate outlines | separate | **separate pieces of the one object, counted in the notice; never connected** |
| Number of objects needed | two or more | two or more | **one or more** (one text, one group, one compound path is enough) |
| Text | converted first (`0074` criterion 24) | converted first | **converted first** |
| Groups | refused | refused | **welded: every leaf inside counts** |
| Open outline | closed by a straight line (`0054`) | same | **its stroke band, as drawn** (criterion 9) |
| Stroke of a closed shape | ignored (region only) | ignored | **ignored** (Question 10, default A) |
| Paint (Fill None) | ignored | ignored | ignored for closed outlines |
| Style and place of the result | bottom-most operand | per operand | **bottom-most operand** |

The customer's decision of 2026-10-17 ("stacking order decides, covered parts of lower objects go, uncovered parts stay, one new object", WELD-VS-FLATTEN A) is read this way: Flatten's visible regions are joined into one object, and **joining dissolves every edge where two pieces meet**, because a kept edge would be cut twice, which is what Weld exists to prevent. The union of Flatten's visible regions is the Union region, so with g = 0 and only closed operands Weld gives Union's region. Weld adds no boolean algorithm of its own (criterion 30).

## Terms

- **Operand:** every leaf the selection holds: a selected path, compound path, rectangle, ellipse, polygon, star or text, and every such leaf inside a selected group at any depth. Linked clones count as the path they show (`0049`).
- **Contribution** of an operand: for a closed outline, its region under the nonzero rule whatever the paint (`0016` criterion 4); for an open outline, criterion 9; for a text, the contributions of the letters Object to path makes of it (`0074` criterion 22).
- **Gap tolerance g** ("Gap"): a length in mm, 0 to 1 mm, default 0.1 mm. **Gap closing** by g: grow the region by g/2 with round corners, then shrink it by g/2 with round corners (a morphological closing). It fills every gap, slit, notch and hole narrower than g, never removes area, and never adds area farther than g/2 from the union.
- **Piece:** one outline of even depth of the result with the holes directly inside it. An island inside a hole is a piece of its own, since it falls out of the sheet as a separate part (`adrs.md`, "Flagged to the PO", default taken 2026-10-20).
- **Base operand:** the bottom-most operand in stacking order (never selection order, `0016` criterion 9). For a selected group, its lowest leaf.
- Kernel tolerance 0.01 mm and grid 0.001 mm as `0016` criteria 25 and 39. Notices are anchored 4px under the Weld button (the Select bar rule of the Action notice), with the timing of the Boolean card's notices (`0016` criterion 29, `0054` durations).

## Acceptance criteria

### Entry point

1. Given the Select tool and one or more selected objects, then a **Weld** button is enabled; with nothing selected it is shown dimmed and focusable as the Flip buttons are, and activating it then does nothing; with another tool active the Select bar is not shown, and with it the Weld button is absent. Its place is the Select bar, in the always-shown zone after Flip and Guide (column B of the rail is full, `0037` UX notes); it has no keyboard shortcut (`0016` criterion 3; Ctrl+W closes the tab in the browser build). Its tooltip names it "Weld" with the rule line "Joins overlapping shapes into one outline.".
2. Given the Weld button, then a **Gap** field sits with it: a bar number field in mm, range 0 to 1 mm, default 0.1 mm, Up and Down step 0.01 mm, Shift 0.1 mm, typed units as `0052`. Enter and blur commit the typed value; Escape restores the shown value. A typed value outside the range is set to the nearest limit with the notice "Gap set to 1 mm, the largest gap." (or "0 mm, the smallest gap."). The value is a per-user setting kept across sessions; it is not stored in the project and changing it writes no history step.
3. Given a Weld, then nothing opens: no dialog, no panel section, no preview. One press, one result.

### What is welded

4. Given two squares of side 20 mm, the second offset by (10, 10) mm, when the maker presses Weld at g = 0, then the result is one ordinary closed path of 8 nodes and area 700 mm² (the result of `0016` criterion 10).
5. Given a ring (a compound path: disc of radius 20 mm minus a concentric disc of radius 10 mm) and a bar 50 x 4 mm laid across the ring's outer part but not into the hole, when welded, then the result is one compound path whose hole is the full disc of radius 10 mm (holes of letters are kept). Given a disc of radius 5 mm inside the hole as a third operand, then it stays an island inside the hole (three outlines; LightBurn loses it), and the notice reports 2 pieces (an island is a piece, Terms).
6. Given four bars 40 x 4 mm laid as a frame (a "#" whose ends overlap) so that they enclose a free square of 20 x 20 mm, when welded, then the result is one compound path of two outlines, the inner square a hole (areas enclosed between letters are kept).
7. Given two squares of side 20 mm that share a full edge, then at g = 0 the result is one 4-node rectangle. Given two squares that touch only at one corner, then at g = 0 the result has 2 pieces, and at g = 0.1 mm it has 1 piece.
8. Given two squares of side 20 mm with a gap between them of 0.05 mm, then at g = 0.1 mm the result has 1 piece; with a gap of 0.15 mm it has 2 pieces. At g = 0 both give 2 pieces.
9. Given an open outline (an open path, or an open outline of a compound path), then its contribution is what the canvas draws for it: with Stroke on, the band of its stroke (its width as drawn, its join, cap and miter limit; dashes ignored, so a dashed line welds as solid), within 0.01 mm of the drawn stroke; with Fill on, additionally its fill area closed by a straight line (`0054` criterion 21); with Stroke and Fill both off, its fill area closed by a straight line, as `0054` criterion 21. Test: an open straight line from (0, 0) to (40, 0) mm with Stroke 2 mm, round caps, welded with a square of side 10 mm centred on (20, 0): one outline, area 40 x 2 + π · 1² + 100 − 10 x 2 mm² within the bound of `0016` criterion 14.
10. Given a dashed open outline was welded, then the notice adds "Dashes were welded as solid lines." Given an open outline was closed by a straight line (criterion 9, Stroke off), then the notice adds the sentence of `0054` criterion 21; an open outline that so encloses no area adds the sentence of `0054` criterion 22.
11. Given a closed outline with Stroke on (any width), then only its region contributes; its stroke does not (Question 10, default A). Test: a square of side 20 mm with a 5 mm stroke welds to an outline of 20 x 20 mm, not 25 x 25 mm.
12. Given one selected text, when the maker presses Weld, then the text is converted as by Object to path (`0074` criterion 22) and its letters are welded, in the same history step, without asking. Given several texts and other objects, then all letters of all texts and all other operands weld into the one result.
13. Given a selected group (also nested, also the group Object to path makes of a text), then every leaf inside it is an operand. Given a single selected compound path whose outlines overlap (a Combine of overlapping shapes, `0035`), then its overlapping outlines are welded into its region.
14. Given one selected closed object that overlaps nothing (a rectangle), then Weld still runs: the result is its region as a path, and the notice reports 1 piece. Nothing is refused for having nothing to weld.

### Gap tolerance: exact bounds

15. Given any g, then the Union region of the contributions lies inside the result, and every point of the result lies within g/2 + 0.01 mm of that Union region. Test (property, at least 200 random shape sets, fixed seed, g in {0, 0.05, 0.1, 0.5, 1} mm): both inclusions hold.
16. Given a convex operand alone (a square of side 20 mm, a disc of radius 10 mm), then for every g the result lies within 0.01 mm of the operand's outline (gap closing does not round convex corners and does not change size).
17. Given a hole narrower than g (a 20 mm square with a slot 0.08 mm wide and 5 mm long cut into it, at g = 0.1 mm), then the slot is filled. Given a hole wider than g (0.2 mm), then it stays.
18. Given g = 0, then the result region equals the Union region of the contributions within the bound of `0016` criterion 14.

### The result

19. Given a successful Weld, then all operands are removed (a selected group with all its members) and exactly one new object takes their place: an ordinary closed path when the result is one outline without holes, else one compound path (`0016` criterion 20). Objects that were not selected keep their ids and their order.
20. Given a successful Weld, then the result's complete style is a copy of the base operand's style (for a text: its fill and stroke) and it sits at the place in the stacking order of the lowest selected object; inside an entered group it stays in that group.
21. Given a successful Weld, then the result is the only selected object, the tool stays the Select tool, and the Properties panel describes it.
22. Given a successful Weld, then exactly one commit is written, stored as `weld`, shown as "Weld" in the History tab, atomic as `0016` criterion 28; Ctrl+Z restores the texts, groups and objects exactly (texts editable again).
23. Given a successful Weld, then a status notice names the counts and the pieces: "Weld: 3 objects became 1 compound path, 4 pieces." ("1 path, 1 piece."; "Weld: 1 text became 1 compound path, 3 pieces." when the selection was texts only). The pieces are counted by the definition in Terms; holes are not pieces.
24. Given the result, then its outlines obey `0016` criteria 24, 24a, 41 and 42 (Corner nodes, no near-duplicate or collinear nodes, no crossing outlines, fixed winding). Outlines are straight segments within 0.01 mm of the exact result, except that the architect may keep the original curves of an outline that met no other contribution and was not changed by the gap closing (`adrs.md`).
25. Given the result, then the Layers row shows it with its kind's glyph; when the base operand was a text, the row is named by that text's first 24 characters (Proposal).
26. Given a project with a welded result saved and reopened, then it has the same outlines, node order and style. Weld writes only existing kinds; there is no `format_version` change.

### Refusals

27. Given a selected text whose face is missing, then Weld is refused with the notice of `0074` criterion 29 ("Font Source Serif 4 Bold is missing. Nothing was changed."; "2 texts use missing fonts. Nothing was changed." for several), the text outlined in red, and nothing changes (all or nothing).
28. Given the contributions together enclose no area (a single open line with Stroke and Fill off), or a coordinate that is not finite or lies beyond `MAX_COORDINATE_MM`, then Weld is refused with the sentences of `0016` criteria 16 and 17a with "Weld" as the operation name, offenders outlined in red, nothing changed (`0016` criterion 18).

### Determinism, robustness, budgets

29. Given the same objects at the same stacking positions and the same g, then the result is node for node the same however they were selected, and the golden files agree to 1e-6 mm on Linux, Windows, macOS and the browser build (`0016` criterion 43).
30. Given the code, then Weld calls the boolean kernel of `0016` and `0037` for the union and the offset routine already in the project for the gap closing; it brings no second boolean algorithm (WELD-VS-FLATTEN A) and no new dependency without the architect's note in `adrs.md`.
31. Given the golden fixtures in `tests/fixtures/weld/`, then each welds in a release build on the CI runner (native; the browser build may take twice as long) within its budget and matches its golden result: (a) "Annabelle" in Great Vibes at Size 20 mm, g = 0.1 mm: under 300 ms; the golden file records its piece count; (b) a text of 500 characters in Great Vibes at 10 mm: under 2 s; (c) the fixtures (a) to (i) of `0016` criterion 40: under 2 s each, a valid result or a refusal of criterion 28; (d) the fixture of criterion 9 (a stroked open line and a square).
32. Given fixture 31 (a), then no segment of the result lies inside the region of the result (no double cut line): every point of the result's outline is on the boundary of the Weld region within 0.001 mm.

## Out of scope

- **Bridges for islands** (stencils): the old Question 2, default A taken: no bridges in this slice.
- **Stroke to path as its own command:** the old Question 3, default A stands (it becomes its own button), but as a later story; this slice builds the stroke band only inside Weld (criterion 9), and the later command reuses it.
- **Live weld on a text** (LightBurn's "Welded" text option: the text stays editable and is drawn welded). Proposal for a later story; it needs a text-node register and a `format_version` bump.
- **Connecting separate pieces** (i-dots, umlaut dots, words with spaces): Weld never adds material between pieces farther apart than g; the notice counts them.
- Moving letters so they overlap (letter spacing, kerning): `0074` out of scope; single-line fonts with open glyphs (`0074` out of scope).
- Cuttable detection of fonts (`0062` Question 2).
- Kerf compensation, cut order, lead-ins (laser workbench, `0063`, `0029`).
- Refitting curves to the result (follow-up of `0016`).
- Strokes of closed shapes as area (Question 10, option B), dash patterns and stroke markers as geometry.
- A hover or live preview of the result; a keyboard shortcut.

## Needs from the architect (`adrs.md`)

1. **Shared engine** (customer WELD-VS-FLATTEN A): Weld's region is the Union region of the contributions, which equals the union of Flatten's visible regions. Decide which fold of `curvyo-geometry-core` Weld calls (the union fold of `0016` or the visible-region fold of `0037` with edges dissolved) so that there is one algorithm, and record why.
2. **Gap closing:** morphological closing by g with the `i_overlay` offset already used by `0038` (round, g/2 out, g/2 in), run on the union and before cleanup; how criteria 15 and 16 (inclusion bounds, convex corners unchanged) are met on the grid; what g = 0 skips.
3. **Stroke band of open outlines** (criterion 9): whether `i_overlay`'s stroke builder or our own routine gives the band of width, join, cap and miter limit within 0.01 mm of what `render-core` draws; the module that later serves Stroke to path.
4. **Collection of operands:** leaves through nested groups, linked clones, texts converted by the `0074` conversion inside the same commit, transforms applied; where the result goes when the lowest selected object is a group or the selection is inside an entered group (criterion 20).
5. **Curves of untouched outlines** (criterion 24): keep or flatten; either is allowed, the choice must be deterministic.
6. **Piece count** (Terms, criterion 23): a pure function in `curvyo-ui-core` from the result's outline nesting (`outline_nesting` of `0035`).
7. **Gap setting** (criterion 2): where the per-user value lives (as the remembered corner style of `0038`), no project key.
8. **Budgets and thread:** whether fixture 31 (b) fits the UI thread or needs the busy state of the Boolean card; any operand or node limit with its refusal text.
9. **Placement:** modules in `curvyo-geometry-core`, `curvyo-ui-core`, `curvyo-editor-wasm`; confirm no new crate, no new ADR, no `format_version` bump.

## Decided by the product owner (vetoable at the demo)

- Weld works on one object or more, on texts and on groups (permit over refuse; one text is the normal case for a name plate).
- Gap default 0.1 mm, range 0 to 1 mm, per-user setting (the draft's value).
- Open outlines weld as drawn (stroke band), not closed by a straight line as in Union: for a single-line stroke, the band is what the laser must cut around.
- Style and place from the bottom-most operand, as Union.
- No bridges; Stroke to path as its own command later.

## Decided by the customer 2026-10-20

- Question 10 WELD-STROKE: **A**. For closed shapes only the area counts, the stroke does not (as in Union); a 20 mm text stays exactly 20 mm high after Weld. Open lines weld as their drawn stroke band.
- Old Questions 2 (bridges) and 3 (Stroke to path) of the draft: defaults A taken (no bridges in this slice; Stroke to path is a later command of its own).

## Open questions (customer; each has a default)

None. Frage 10 (WELD-STROKE) was decided by the customer on 2026-10-20 (A, see above).

Customer decisions of 2026-10-17 (WELD-SCOPE, WELD-VS-FLATTEN A) are applied above. The old questions 2 (bridges) and 3 (stroke to path) were not answered; their defaults A are taken (Out of scope).

## UX notes

ux-engineer, 2026-10-20. Read against `docs/design-system.md` on `story/text-objects`
(rows "Select bar layout", "Flip buttons", "Guide (`0046`)", "Select bar Text group",
"Bar number field", "Size field", "Action notice", "Refusal outline", "Command glyphs",
"Boolean toolbox", "Hidden, not disabled") and `frontend/src/components/SelectToolbar.tsx`.
Takes the architect's flag: islands count as pieces.

### 1. Where Weld lives: a Weld group in the Select bar

Column B of the rail is full (row "Tool rail architecture", budget), and column A is full
with Text (`0074`, row "Tool rail budget with Text"). Weld is a command on the selection,
so it goes in the Select bar, like Flip, Guide, Object to path and Linked clone.

**Order of the Select bar after this slice:** settings (the two switches) | Flip, Guide |
**Weld group** | kind groups (rectangle, star, ellipse, curve) | Text group | Object to path |
Linked clone.

- The Weld group is the third zone: the first group after Flip and Guide, behind a divider
  (1px, 20px, 25 %, as between all groups).
- **Why here and not appended last:** Weld is always shown, as Flip and Guide are. The
  always-shown controls sit before the kind groups, so they never move when the selection
  changes kind. Appended last, Weld would be the only always-shown control that jumps
  with every selection. The kind groups, the Text group, Object to path and Linked clone
  come and go anyway, so moving them to the right on a wide bar breaks no muscle memory.
  The rule "a new control is appended so no earlier one moves" applies to the zone of
  shown-by-kind controls. The always-shown zone grows at its own end, after Guide.
- The group is `role="group"`, accessible name "Weld". It holds the Weld button and,
  4px on, the Gap field: one row, 28 + 4 + 104 = 136px.
- **Only in the Select tool.** The Select bar exists only there, so with another tool
  the Weld button is absent, not dimmed. This amends criterion 1 ("with another tool
  active it is shown dimmed" becomes "with another tool active the Select bar, and with
  it the Weld button, is not shown"). The Text bar of the Text tool gets no Weld: from
  editing, Escape twice gives the Select tool with the text selected, and Weld is there.

### 2. The Weld button

- **Look:** a 28 x 28px icon button, `rounded-md`, no text label. Glyph 16px, 1.5px
  stroke, `currentColor` (`--toolbar-icon`). Hover `--editor-accent-hover`. Focus-visible
  2px `--editor-accent` ring with 1px `--toolbar-bg` offset. A plain command button: no
  `aria-pressed`, never the active ground. This is the Flip button style.
- **Glyph `WeldGlyph`** (custom SVG, 16 x 16 viewBox, row "Command glyphs"): three discs
  of radius 3.25 centred on (4, 8), (8, 8) and (12, 8), drawn as **one** solid silhouette
  path with a 1.5px stroke and round joins. There are no inner arcs, so it reads as joined
  letters with one cut line. At most one shape: no modifier mark.
  - **Must read apart at 1x from Union** (two squares, rail) and from **Combine** (a
    square with a square hole, rail). The circles against squares carry the difference.
  - **Fallback** if the three discs read as a blob at 1x: two discs of radius 4 on
    (5, 8) and (11, 8).
  - Approved on the glyph sheet at 1x, 1.25x and 2x, as the other command glyphs.
  - UX review 2026-10-20: the three discs read as a capsule at 1x; the fallback is built.
- **Accessible name** "Weld". No shortcut (criterion 1). Ctrl+W closes the tab in the
  browser build; LightBurn's key is not copied.
- **Tooltip** (Radix, 400 ms, `side="bottom"`, max 260px; the Flip tooltip surface):
  - name line, semibold: "Weld";
  - rule line, `--panel-muted-fg`: "Joins overlapping shapes into one outline." (42
    characters, does not wrap);
  - a third muted line only while dimmed: "Select an object first." (the Flip wording).
    It names the state, so it is not a hint.
  - The tooltip is also shown on keyboard focus.
- **Enabled:** the Select tool and one or more objects selected, of any kind (texts,
  groups, clones, open paths, guides). The selection's content never dims the button.
  A selection that will be refused, such as a missing face or no area, is refused on the
  press with a reason. Permit over refuse: the button does not guess.
- **Dimmed** (nothing selected):
  - `aria-disabled="true"`, not `disabled`; glyph at 40 %; no hover ground; default
    cursor;
  - keeps its Tab stop and tooltip; a press does nothing and leaves the tooltip open;
  - this is the fourth recorded exception of "Hidden, not disabled", for the reason of
    Flip and Guide: the always-shown zone must not move.
- **Busy:** Weld runs on the UI thread, and fixture 31 (b) may take up to 2 s. On a
  press, as the Boolean toolbox does:
  - the `:active` ground stays on the button;
  - the group gets `aria-busy`;
  - the window cursor is `wait`;
  - these are painted (two frames) before the kernel call;
  - further presses and keys are ignored until it returns.
  - There is no "working…" notice: that notice is only for a call that runs off the UI
    thread.
- **Focus after the press:** a mouse press returns focus to the canvas (`onReturnFocus`).
  Enter or Space leaves focus on the button. This holds after a success and after a
  refusal.
- **Refused state:** the button has no refused look of its own. A refusal shows as a
  refusal notice under the button and as the Refusal outline on the canvas (section 5).
  The button stays enabled, so the maker can fix the selection and press again.

### 3. The Gap field

- **Look:** a bar number field (row "Bar number field"; the Size field's construction):
  - 104 x 28px, white ground, 14px tabular value, right-aligned;
  - "Gap" inside the left edge, `--toolbar-icon`;
  - "mm" inside the right edge.
  - **Always mm, whatever the display unit**, like Size. The bounds of the spec are
    millimetres, and 0.004 in is not a number anyone types.
  - Width check at build: "Gap", "0.05" and "mm" must show without clipping. If they
    clip, widen in steps of 8px. Never narrower than 104px.
- **Accessible name** "Weld gap". `title` and tooltip: the name line "Weld gap" and the
  rule line "Closes gaps narrower than this." (the form of a Boolean rule line). Nothing
  else.
- **Value:**
  - shown with up to two decimals, trailing zeros trimmed: "0", "0.05", "0.1", "1";
  - range 0 to 1 mm, default 0.1 mm;
  - Up and Down step 0.01 mm, Shift+Up and Shift+Down 0.1 mm;
  - a step stops at the limit without a notice;
  - typed units are those of `0052` ("0.004 in" becomes 0.1 mm, shown "0.1").
- **Commit:**
  - Enter commits. Escape restores the shown value and returns focus to the canvas.
  - **Blur commits too.** This deviates from the bar number field's "a press elsewhere
    restores". The Gap is a setting, not a document edit, and the next press is usually
    the Weld button beside it. Restoring on that press would weld with the old gap, a
    silent trap.
  - Invalid text gives the "Enter a number" rule (2px `--field-invalid`, a 12px message
    line under the field, `aria-invalid`). The stored value is kept, and Weld uses it.
- **Clamp notice** (criterion 2): an Action notice (status, 3 s), 4px under the field
  and left-aligned to it: "Gap set to 1 mm, the largest gap." / "Gap set to 0 mm, the
  smallest gap."
- **Never dimmed, never hidden** while the Select bar shows. It is a setting, like the
  two switches: it can be set before anything is selected.
- **No history step, no project key, no dirty mark.**
  - The value is stored in `localStorage` key `curvyo.weld-gap-mm` (`adrs.md` decision
    7), read once at start.
  - Every access is in `try/catch`; a failed read gives 0.1 mm, and a failed write keeps
    the value for the session.
- **No drag-scrub and no fill bar**, as Size. No live preview: Weld shows nothing before
  the press (criterion 3).

### 4. Notices (success)

- **Place:** an Action notice (status), 4px under the Weld button and left-aligned to it.
  This is the Flip and Linked clone rule for Select bar commands, and it amends Terms
  ("in the place of the Boolean card's notices": the Boolean card's place is beside the
  rail, which is far from this button).
  - Shifted to stay right of `--rail-right` + 12 and inside the window.
  - At most 360px wide, wrapping.
- **Duration:** the Action notice rule: 3 s, 5 s over 70 characters, 8 s over 140, or
  until the next action. The Weld group's polite live region carries the same text.
- **Sentence 1, the count:**
  `Weld: <N> <noun> became 1 <kind>, <P> <piece|pieces>.`
  - `<N>` counts the selected top-level objects. A group is one object.
  - `<noun>` is "text" or "texts" when every selected object is a text, else "object" or
    "objects".
  - `<kind>` is "path" or "compound path".
  - Pieces are counted with islands as pieces: every outline of even depth, with the
    holes directly inside it.
  - Examples:
    - "Weld: 1 text became 1 compound path, 3 pieces." ("Annabelle", with the i-dots
      loose);
    - "Weld: 3 objects became 1 compound path, 4 pieces.";
    - "Weld: 1 object became 1 path, 1 piece." (criterion 14);
    - "Weld: 3 objects became 1 compound path, 2 pieces." (criterion 5, the disc inside
      the ring).
- **Then, in this order, only when they apply** (the `0054` order: what changes the
  picture first):
  1. "Dashes were welded as solid lines." (criterion 10)
  2. the sentence of `0054` criterion 21 ("1 open outline was closed with a straight
     line." / plural);
  3. the sentence of `0054` criterion 22 ("1 open outline enclosed no area and was
     removed." / plural);
  4. "It uses the style of the lowest object." when the operands' styles differed (the
     Combine sentence; with texts of different fills this explains the result's colour).
- No sentence about the Gap value: the field beside the button shows it.

### 5. Refusals and the Refusal outline

- **Place and look:** the refusal Action notice (`--popover` ground, 1px
  `--field-invalid` ring, `--field-invalid` text, 12px alert glyph, `role="alert"`,
  8 s or until the next action), 4px under the Weld button, left-aligned. Nothing
  changes, and the selection stays as it was.
- **Texts:**
  - Missing face (criterion 27): "Font Source Serif 4 Bold is missing. Nothing was
    changed." / "2 texts use missing fonts. Nothing was changed." Each such text is
    outlined.
  - No area (criterion 28): "Weld needs an area. The selected objects enclose none.
    Nothing was changed." (the `0016` criterion 16 sentence with "Weld"; the
    implementer takes the exact `0016` text and only swaps the operation name). Every
    operand is outlined.
  - Out of range: "Weld works only within 10 km of the point 0, 0. 1 of 3 selected
    objects reaches further. Nothing was changed." (plural as Union). The offenders are
    outlined at their own coordinates.
- **Refusal outline** (row "Refusal outline", unchanged look): a hollow 2px
  `--field-invalid` outline with the white casing, for as long as the notice shows.
  - A text with a missing face is outlined by its glyph outlines in the fallback face
    that is drawn.
  - A group offender is outlined by its offending leaves.
  - At most 200 are drawn; the count in the sentence stays exact.

### 6. Undo

- One press writes one step: History label "Weld", kind glyph `Replace` (the five kinds
  of "Kind glyphs"). There are no intermediate steps for the text conversion or the
  ungrouping.
- Ctrl+Z (or the History tab's Undo) shows "Undid: Weld" and restores the texts
  (editable), groups and objects with their ids and order. It also restores the
  selection to the operands as they were before the press (the `0020` rule).
- Redo ("Redid: Weld") gives the same result node for node and selects it.
- A Gap change is not a step, so undo never touches it.

### 7. After a success

- The result is the only selected object: the Select tool with its box and handles. The
  Properties panel describes it ("Compound path" or "Path" subject line).
- The Layers row takes the kind glyph of the result; a text base names the row by the
  first 24 characters (criterion 25).
- The Select bar re-forms for the new selection: the Text group leaves, and Weld stays
  where it was.

### 8. Fit at 800 x 600

The bar is 324px wide (panel open, rulers on, two rail columns); rows are 36px.

- **Row 2:** 8 + Flip and Guide 92 + 12 + divider 1 + 12 + Weld group 136 + 8 = 269px.
  This leaves 39px, so no later group shares row 2 any more.
- **Row counts:**

| Selection | Before | After | Rows after |
|---|---|---|---|
| Nothing selected | 2 (72px) | 2 (72px) | switches; Flip, Guide, Weld, Gap |
| One path | 2 (72px) | **3 (108px)** | switches; Flip, Guide, Weld, Gap; Linked clone |
| One text (`0074`) | 4 (144px) | 4 (144px) | switches; Flip, Guide, Weld, Gap; Text group (300px); Object to path, Linked clone |
| One rectangle | 5 (172px) | 5 (172px) | as before, with Weld and Gap on Flip's row |
| One ellipse | 4 (144px) | **5 (180px)**, measured at the UX review | the arc kind group (145px) moves from Flip's row to its own |
| Only linked clones | 1 | 1 | no switches: Flip, Guide, Weld, Gap (269px) |

- The one cost is one path at the minimum window: 36px more bar, over the pasteboard
  and not over the panel. The bar is in the overlay row and does not shrink the canvas.
  This is accepted. The alternative, Weld appended last, would put the always-shown
  button at a different place for every kind.
- **Wide window** (1280 x 800): the Weld group follows Guide on row 1. The kind groups
  shift 161px to the right.
- **Browser build at the minimum:** the same as Tauri; the bar is not in the rail's
  column.
- Measure at build (panel open, rulers on): nothing selected, one text, one rectangle.
  Record the results in the row "Select bar layout".

### 9. Large documents

- A selection of thousands of objects changes nothing in the bar.
- The press shows the busy state above.
- The result is one object, so the Layers list shrinks. The Refusal outline caps at 200.

### 10. Changes to the criteria (for the PO)

- **Criterion 1:** "with nothing selected it is shown dimmed and focusable as the Flip
  buttons; with another tool active the Select bar is not shown". The tooltip rule line
  is "Joins overlapping shapes into one outline."
- **Criterion 2:** add "Blur commits the typed value; Escape restores".
- **Terms, notices:** "anchored 4px under the Weld button (Select bar rule)" in place of
  "the place of the Boolean card's notices". The timing is unchanged.
- **Terms, piece:** "one outline of even depth with the holes directly inside it"
  (architect's flag, default taken). Criterion 5 then reports 2 pieces.

The design-system rows of this slice are in `docs/design-system.md` (rows "Weld (`0068`)", "Weld gap field", "Weld notices" and the amended rows they name). Section 10's changes are applied to criteria 1, 2 and 5 and to Terms above.

## Links

Requirements: R-FONT-009; related R-EDIT-003, R-EDIT-023, R-EDIT-048, R-FONT-002
Depends on: `specs/0074-text-objects/`, `specs/0016-boolean-operations/`, `specs/0037-fracture-and-flatten/`, `specs/0054-combine-open-paths/`, `specs/0023-groups/`
Related: `specs/0062-fonts/` (cuttable group), `specs/0038-path-offset/` (offset routine), `specs/0035-combine-and-break-apart/` (outline nesting), `specs/0063-manufacturing-workbenches/`
Field sources: [LightBurn Boolean tools](https://docs.lightburnsoftware.com/2.1/Reference/BooleanTools/), [LightBurn combining shapes](https://docs.lightburnsoftware.com/Tools/CombiningShapes.html), [xTool text welding](https://support.xtool.com/article/676), [Glowforge community: welding](https://community.glowforge.com/t/welding-in-glowforge/124132)
ADRs: ADR 0003 (kernel); none new expected
PR: -
