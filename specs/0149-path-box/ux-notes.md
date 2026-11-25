# 0149 Path box: UX notes

ux-engineer, 2026-11-25, revision 4. Checked against `specification.md` revision 6 (criteria 1 to 16;
Fragen 364 to 366 at their defaults A), ADR 0083 revision 4 (§1 kind and label, §2 contract, source,
placement and checks, §3 classification, §4 one rule per span, §5 corner joints, §6 seam, §7 plates
and hollow, §8 parameters and layout, §11 warnings and marks, §13 curves), `adrs.md` revision 4, the
slice-1 overlay as built (`curvyo-ui-core/src/path_box_marks.rs`, `instance_warnings.rs`),
`specs/0147-rounded-box/ux-notes.md` revision 3 (the shared block; everything not
named here is as there), `specs/0141-parametric-generators/ux-notes.md` and `docs/design-system.md` (rows
"Pick mode", "Select bar layout", "Stroke to path", "Action notice", "Canvas notice slot", "Drag ghost",
"Modifier badge", "Hidden, not disabled", `--mixed-hatch`).

Revision 4 (slice 2, curves; all **(Proposal)** unless the spec or ADR 0083 §13 fixes it, vetoable at
the demo): the curved-outline refusal is gone from the Select bar, the pick, Replace outline and the
hover (sections 1, 4); the pick hover runs in Draft and draws the same marks as the built box (section
1.2); bend bands follow curves, red only on the stretch that is too tight (section 3); crosses at
cusps and at corners meeting a curve, with the solid flat left bare (section 3); a **seam tick** on an
outline with no corner (section 3, new); one piece means no badge and "the wall" in the notices
(sections 3, 6); the finger hollow on the lid's longest convex curve and the rounded-cover notice
(sections 5, 6); notices come from the full build only, never from a Draft (section 6). Items for the
architect and the product owner: section 10. Slice-1 decisions below are kept unless named here.

Revision 3 (as built in slice 1; lead decisions, all **(Proposal)**, vetoable at the demo): "Replace
outline" is in slice 1, a button in the Size group that arms the pick mode (section 4); refusals in the
canvas notice slot use the refusal look, and a Select bar refusal shows under the button only (sections
1.1, 1.2); the finger hollow on a bent span is out of slice 1, the lid is built without it (section 5).

Revision 2: the layout lies right of the source (section 4, question 2 B); kinks are built in slice 1
and carry the bend hatch (section 3, question 1 A); the curves refusal in the button and the pick
(section 1); the default outline is a 100 x 60 mm coffin (section 4); the names "Path box" and "Box
from path" as ADR 0083 §1 (section 4).

Customer rules applied: no popups, no operating tips or hint labels, no notice after a success, hidden
not disabled. No new colour token, no new component. Everything here that the spec, ADR 0083 or a
decided Frage does not fix is **(Proposal)**, vetoable at the demo.

## 1. Two ways in (criterion 1)

A path box needs an outline, so it cannot be placed at the pointer like Box, Gear and Rounded box. It is
offered where the outline is: on the selection, and as a pick on the canvas.

### 1.1 Select bar: **Box from path** (the main way)

- A text button **Box from path**, the look of "Object to path" and "Stroke to path" (28px,
  `rounded-md`, 14px label, no glyph, hover `--editor-accent-hover`, the focus ring), in a group of its
  own **last in the Select bar, after Linked clone** (a new control is appended so no earlier one
  moves).
- **Shown** when the selection is exactly one path or shape whose outline is one closed subpath (a
  path, a rectangle, an ellipse, a polygon or star, a compound path with one subpath), on a drawing or
  laser page. **Hidden** (not disabled) for anything else: nothing, several objects, a group, an open
  path, text, an image, a generated object, a sketch page (no generators there, `0141` section 2). The
  cheap tests run on the selection; self-intersection and size run on the press (ADR 0083 §2, ui-core
  before create). **Slice 2:** curves are no test any more; a circle, an ellipse, a heart, a shield or
  a D shows the button like a coffin does.
- **Press:** one step "Add path box"; the box is built from the outline (section 4), selected, the Style
  tab shows (`0141` section 2), focus goes to the canvas. The source is deselected and untouched.
- **Refused** (Action notice under the button, the refusal look, `role="alert"`, 8 s; **only there**, not
  a second time in the canvas notice slot, **(Proposal)**, lead): "The outline
  crosses itself. Nothing was changed." / "The outline is too small for a box with 3 mm walls. Nothing
  was changed." (area below (4 t_w)², t_w the default Wall thickness; from slice 2 the area of the
  curve, so a circle of radius r has π r², ADR 0083 §13.1). These two are the only refusals. Slice 1's
  "This outline has curves. ..." is removed with slice 2 (ADR 0083 §13.14), so is the stored-file
  warning "This outline has curves; they are built as straight edges.": a curved outline in any file is
  now built exactly and says nothing. (ADR 0083 §2: the tests are one `generate` with the default
  values, run in ui-core before create; §13.13: the test runs it as a Draft, the press builds in full.)
- No shortcut. **(Proposal)**

### 1.2 Generators bar: segment **Box from path**, a pick mode

- A segment of the `role="radiogroup"` "Generator", text "Box from path" with Lucide `Spline` 16px (a
  curve through nodes; present in lucide-react 1.50). By the registry rule the bar reads **Box, Box from
  path, Gear, Rounded box**; four segments fit from an 800px window and wrap whole by the bar overflow
  rule below that.
- **A segment is a mode, never a command.** Arrow keys move through a radiogroup and select as they go,
  so a segment that built on selection would build, or refuse, while the maker arrows past it. Choosing
  the segment builds nothing and shows nothing; it turns the tool's click into a pick, the "Pick mode"
  of `0080`:
  - The pointer over an **eligible** outline (the Select bar test of one closed subpath, then
    self-intersection and size; one `generate` per hovered object that
    passes the Select bar test, cached until it changes, ADR 0083 §11 A) draws it 2px `--hover-box`
    on its casing, with the corner crosses, bend hatch and seam tick of section 3 at the default Corner
    angle (25°); no badges on the hover (as built). Cursor `pointer`;
    elsewhere, and over an ineligible object (a crossing or too small outline included), the
    default arrow and no highlight. **Curves are eligible from slice 2**: a heart shows its two crosses
    and two hatched lobes, a circle its hatch all round and the seam tick.
  - **The hover runs in Draft** (ADR 0083 §13.13: no slits, no plate calls, at most 100 ms for the
    heart). Its crosses, red and seam are the built box's exactly (they come from the classification,
    which a Draft runs in full); a test compares them for the heart, the shield and the circle (**for
    the tester**). The bands may differ where the built box lowers its window count for the slits
    (ADR 0083 §13.8), so the hover's band gaps are only approximate (section 10, item 6). Highlight, marks and cursor appear
    together when the result is there, never the highlight first: a highlight that then turns out
    ineligible would flash a promise. The first hover of a big outline may take a frame longer, a
    second hover is the cache. No spinner, no wait cursor (100 ms is below what reads as waiting).
    **(Proposal)**
  - A **click** on it builds the box from it: one step "Add path box", the box selected, back to Select
    (`0141` section 2). The source is untouched.
  - A click on an **ineligible** object (open path, group, text, crossing outline, too small): nothing
    is built, the refusal in the canvas notice slot (the refusal look,
    `role="alert"`, 8 s, as every text ending "Nothing was changed." there; warnings stay in the status
    look, section 6; **(Proposal)**, lead): "Select one
    closed outline. Nothing was changed."; for a crossing or too small outline the texts of 1.1.
  - A click on **empty canvas** does nothing and says nothing (the Pick mode rule: a miss is not a
    request).
  - **Enter** builds from the current selection when it is one eligible outline (the keyboard way:
    select with Select, then the tool and Enter); otherwise the refusal "Select one closed outline.
    Nothing was changed." in the canvas notice slot, or the crossing or too small text of 1.1
    when the selection is one closed outline that fails those tests. This is where the spec's notice
    lives.
  - **Escape** leaves the tool for Select, as `0141`.
- The texts end with "Nothing was changed." as every refusal in the app (design system, "Action
  notice"); the spec's "status notice" becomes a refusal for that reason (**for the product owner**).

## 2. Ghost (criterion 1)

There is **no pointer ghost**: the box does not go where the pointer is but where the outline is (ADR
0083 §2), so a ghost that follows the pointer would show a place the click does not use. In the pick
mode the hovered outline with its corner marks is the preview: it shows the one thing the maker decides
by the pick, which corners become joints. The parts appear on the click, one undo away. The spec's
"live ghost of the corners" is these marks: on the hovered outline here, on the selected box in
section 3, live during a Corner angle drag.

## 3. Corners and curves on the canvas (criteria 3, 5, 7, 9, 12 to 15)

While **one path box is selected alone** (not during a Select-tool drag, back on release), and on the
hovered outline in the pick mode, an overlay shows how the outline was read. All of it is
`aria-hidden`, has no hit area, no cursor change and is never cut, exported or in the job.

- **Where.** On the **inner base**: its outline is the footprint (the path, ADR 0083 §3), so the marks
  sit on the shape the maker drew.
- **Corner** (turn above Corner angle; a finger joint): a **cross**, two 8px strokes of 1.5px `--accent`
  at ±45° to the corner's bisector, on the 1.5px `--selection-casing`, centred on the corner point. A
  cross is no handle silhouette (resize squares, rotate arrows, knobs are rings with a dot, nodes are
  squares, diamonds and triangles in the Node tool only), so nobody tries to drag it. Tangent joins get
  no mark. A kink (a join turning by more than 1° and at most Corner angle, filleted into a bent span,
  ADR 0083 §3) gets no cross but the hatch of a bent span below: cross for a joint, hatch for a bend.
- **Very sharp corner** (interior angle below 30°, criterion 5): the same cross in `--field-invalid`
  with a 12px `--field-invalid` ring round it, so it reads by shape as well as colour.
- **Corners on curves (slice 2).** A cross sits on every corner whatever meets there: two straights,
  a straight and a curve (the shield's top corners are straight to straight; a D's two corners are a
  line meeting a curve), two curves (the heart's notch and tip), and a **cusp** inside one cubic (ADR
  0083 §13.2, usually red). It is centred on the corner point the maker drew; its bisector is that of
  the joint as cut (the turn between the flats, §13.7), so the cross is square to the fingers the wood
  gets. The red ring follows the joint as cut too: a heart tip drawn just above 30° may be cut below it
  and then carries the ring and the notice, which is then true of the wood.
- **The flat beside a corner carries nothing**, like a straight: where a corner meets a curve, the
  first z + t_w of the curve is a solid flat (§13.7), and the hatch starts where the slits start. The
  bare gap between cross and hatch is the solid margin, visible without a label. **(Proposal)**
- **Bent span** (a `Bend` mark, ADR 0083 §11; always slit): **slice 1** for the filleted kinks, which
  are built in slice 1 (question 1 A); slice 2 adds the curved cubics. A band 6px wide on screen just
  inside the footprint line on the bent stretch, 45° stripes 1px wide every 4px in `--drag-ghost` (the
  `--mixed-hatch` geometry, an existing colour). A stretch whose radius is too small to bend (criterion 7) carries the stripes in
  `--field-invalid` instead, so the place the wood may crack stays visible while the box is selected,
  not only for the notice's few seconds. Straights carry nothing.
- **Bend bands on curves (slice 2).** One band per run (ADR 0083 §13.4: arcs and fillets with no
  straight, flat, window or corner between), drawn along the curve as built: the stripes stand at 45°
  to the **local** tangent (as `path_box_marks.rs` already draws them on kinks), so they read the same
  on a heart lobe as on a 20-gon fillet, and a circle is striped evenly all round. Two rules the curve
  adds **(Proposal)**: stripes are spaced **4px along the line** (by arc length, not by cubic
  parameter, which bunches them where a cubic is tight), and a join between two cubics of a run gets
  one stripe, not two. Windows (the plates' tabs, §13.8) carry no stripes, as on the rounded box: the
  gaps in the band are where the tabs sit (the band as the slit bands: section 10, item 9).
- **Red only where it is too tight.** A run is mostly fine and tight in one place (a heart lobe near
  the notch). The band is `--field-invalid` on the stretch whose radius is below the limit (2 t_w
  convex, t_w concave) and `--drag-ghost` on the rest of the same run, so the maker sees where, not
  only which piece (**for the architect**: `Mark::Bend.tight` is one flag per run today; it needs the
  tight stretches, e.g. one `Bend` per stretch with its own flag. Until then the whole run is red,
  the slice-1 rule). Built anyway, nothing clamped (criterion 7); the notice of section 6 names the
  piece.
- **Seam tick (slice 2, new; Proposal).** An outline with **no corner** is one strip with one seam
  (criterion 9; ADR 0083 §6, §13.11): at the middle of the longest straight if there is one (a
  rounded square, a stadium, the 20-gon), else at the point with the least y on the page, the back of
  the box (a circle, an ellipse). The seam is where the maker glues the strip shut, so it is marked: a
  **tick**, one 10px stroke of 2px `--accent` across the footprint line (square to it, centred on it),
  on the 1.5px `--selection-casing`. Cross for a joint, hatch for a bend, tick for the seam. The hatch
  leaves the seam's slit-free margin (2 t_w each side, criterion 9) bare, like the flat beside a
  corner. On the hover too, so the maker sees before the click where an ellipse will be cut. (**For the
  architect**: `Marks` has no seam; it needs `Mark::Seam { at, inward }` on the footprint, run time,
  as §11's other marks.)
- **Piece numbers** (criterion 12): the wall pieces are numbered in walk order from the outline's start
  (ADR 0083 §8, the part keys `wall-1` to `wall-n`). Each number is a **badge** in the "Modifier badge"
  look of the Pick set row (16px circle, solid `--accent`, white 10px tabular digit, 1px white ring, DOM,
  `aria-hidden`): one at the middle of each wall piece in the layout, and one on the inner base 12px
  inside the middle of the stretch that piece stands on. So the maker matches piece 3 to its place on
  the base, and the notices can name pieces. A piece shorter than 24px on screen leaves its badges out.
  With no corner or one corner there is one piece and no badge.
- **Numbering on curves (slice 2).** A piece runs corner to corner; tangent junctions (a line into a
  curve, two cubics of a lobe, a smooth node) add no piece (criterion 14), so the heart has 2 pieces
  and badges 1 and 2, the shield 2 (the top, and one piece of side, bottom curve and side, since the
  sides run into the curve without a corner; criterion 13's "three straight pieces" reads otherwise,
  section 10), a D 2. A run with no corner
  on either side is the whole outline: one strip (circle, ellipse, rounded square), no badge, the seam
  tick instead; a teardrop with one corner is one piece too, joined to itself at that corner, no badge.
  Numbering is walk order from the outline's start, as slice 1; on a single piece nothing is
  numbered and the notices say "the wall" (section 6). A Corner angle drag that turns a node of
  the drawing into a corner, or back (criterion 15; a smooth bend inside a cubic never becomes one),
  renumbers the pieces live; the numbers are the walk order,
  never kept per place.
- During a Corner angle drag the marks and numbers follow each regeneration, so the maker watches
  corners appear and vanish as the threshold passes them. **(Proposal)**
- The badges replace the spec's "labelled in the Generator block" (criterion 12): a number in the panel
  cannot be matched to a piece, a number on the piece can (**for the product owner**). They are not
  engraved; numbers burnt on the parts are a later entry if the customer wants them.

## 4. The box and its source (criteria 1, 8; Frage 364 A, 365 A; question 2 B)

- **The copy.** The box keeps a copy of the outline (ADR 0083 §2). The block draws no row for its
  value. A stored
  outline that fails the shape check (a damaged or hand-edited file) reads as the **default outline, a
  coffin of 100 x 60 mm** (six straight edges, ADR 0083 §2), shown with a notice and never rewritten.
- **Replace outline** (built in slice 1, **(Proposal)**, lead; it replaces ADR 0083 §2's "Replace from
  selection", which needs a second selected object while the block shows for one selected box only). A
  full-width text button **Replace outline** at the top of the Size group, above Inner height (the
  look of the block's buttons, the focus ring). The press arms the **pick mode of section 1.2** for this
  box: the Generators tool with the box's kind, hover highlight on an eligible outline, cursor
  `pointer`. A **click**, or **Enter** with the pointer over an outline, replaces the stored copy with
  that outline: one step **"Replace outline"**, the box keeps its place and its other values,
  regenerates, stays selected, back to Select. The picked outline (the source) is untouched. **Escape**
  or a **tool change** cancels; nothing changes. An ineligible pick is refused with the same sentences
  as section 1.2 ("Select one closed outline. Nothing was changed.", or the crossing or too small
  text of 1.1), in the canvas notice slot in the refusal look; the pick stays armed. From slice 2 a
  curved outline is eligible here too: a coffin box can take a heart, and back.
- **The source stays untouched:** same place, layer, style and operation, deselected after the build.
  Editing it later changes nothing in the box. On a laser page the source keeps its operation, so it is
  cut as well as the box's inner base until the maker deletes or unassigns it; nothing warns about that
  (no hint), it is the Frage 365 A choice.
- **Where the box lands** (ADR 0083 §2, question 2 B). The flat layout lies **to the right of the
  source's bounding box** (its outline after its transform, stroke not counted), **Gap apart, top
  edges aligned**: the layout's left edge is Gap right of the source's right edge, its top edge level
  with the source's top edge. Source and parts do not overlap, so the source stays visible and
  clickable. This holds at the build only; later edits regenerate about the box's own origin, as every
  generator. The box goes **in the source's layer, directly above the source** (not on top of the
  active layer, so the two stay together in Layers), as clones, trace results and Stroke to path do.
- **Names** (ADR 0083 §1). The object is a **"Path box"** (a noun): Layers glyph `Spline`, name "Path
  box"; subject line "Path box", "2 path boxes"; status bar "Path box · <page>"; steps "Add path box"
  and "Convert path box to paths". The command is **"Box from path"**: the Generators bar segment and
  the Select bar button.

## 5. The Generator block (criteria 2, 10, 16)

The block of `0147` revision 3, section 3, built by one shared schema function (ADR 0083 §8), with these
differences:

| Group | Row | Element | Shown when |
|---|---|---|---|
| Size | Inner height | length, as the rounded box | always |
| | **Corner angle** | angle, default 25° (Frage 366 A), 5° to 90° | always |
| Material, Slits, Base, Lid, Layout | as `0147` §3 | | as there |

- **Left out:** Inner width, Inner depth and Corner radius (the outline gives them), Top shape, and
  **Wall slits** (straights are solid, curves always slit). No Outline row (section 4).
- **"Inner height"**, not the spec's "Wall height": one schema function means one label, and the rounded
  box says "Inner height" for the same key `inner_height` (**for the product owner**).
- **Corner angle** sits where the rounded box has Corner radius, the row that shapes how the footprint
  is read. A Value field with "°" as its unit. Track 5° to 90°, linear, step 1° (Shift 5°, Ctrl 0.1°),
  whole degrees unless a fraction was typed or Ctrl-dragged. `aria-valuetext` "25 degrees, 6 corners"
  (the count from the classification), tooltip first line "Corner angle: 25°, 6 corners", so the count
  is there without the canvas. Curves give the other counts: "25 degrees, 1 corner", "25 degrees, no
  corners" (a circle at any angle). A higher angle turns joints into curves; the hint of that is the overlay,
  not a label.
- **Slit spacing automatic** follows the local radius (ADR 0083 §4), so no single value describes it.
  The field shows the **smallest spacing used**, muted, `aria-valuetext` "1.3 millimetres at the tightest
  curve, automatic"; a typed value applies everywhere (**for the architect**: the reported automatic
  value is the smallest column gap; accepted, ADR 0083 §4). **(Proposal)** The Slits rows stay shown in
  slice 1 too: a join of two straights that turns by less than Corner angle is a kink, filleted into a
  bent span with slits and built in slice 1 (ADR 0083 §3, question 1 A), so a straight outline can have
  slits.
- **No knobs** (ADR 0083 §8: no size handles). Proportional resize as `0147` §8: the outline scales with
  Inner height, the thicknesses stay; readout "W × H mm". The scale stops where 3 mm walls (the current
  Wall thickness) no longer fit (ADR 0083 §2, warning `smallest_size`), with the notice of section 6.
- Clamps as `0147` §5 where the field exists. Extra texts: "No place for the finger hollow on this lid."
  (ADR 0083 §7, warning `hollow_left_out`); "The ring has no opening at this ring thickness." (the ring
  a full plate).
- **Slice 1** (**(Proposal)**, lead): the finger hollow sits on the lid's longest straight that holds it;
  the hollow on a bent span (ADR 0083 §7, "else centred on its longest bent span") is not built. With no
  such straight the lid is built without the hollow and the `hollow_left_out` text above shows, as a
  warning of section 6.
- **Slice 2: the hollow on a curve** (criteria 10, 16; ADR 0083 §13.10). Order: the lid's longest
  straight that holds the hollow width plus 2 t_w; else the middle of the lid's **longest convex
  curved run** longer than that and convex across the whole width; else the next longest such run.
  So the heart's lid, the ellipse's and the circle's get the hollow on a lobe or at the front, the
  shield's on its top straight. The notch follows the curve's normal at the run's middle, so it points
  into the box as on the rounded box's front. Nothing on the canvas marks it beyond the cut itself.
  **Notices:** a lid with no straight and no convex run that holds it (a small or very wavy outline)
  is built without the hollow and says "No place for the finger hollow on this lid." (unchanged). A
  lid built by the rounded fallback (section 6) has no hollow for that reason and says the rounded
  sentence only, not this one as well: one cause, one sentence. The hollow exists only on the seat
  lid (Lid on and Lid seat on, `0147` §3), so with the seat off or Finger hollow off neither text
  mentions it. **(Proposal)**

## 6. Notices that build anyway (criteria 5, 7, 16)

The customer: "if the radius is too small: own fault, we just do it". Nothing is refused, nothing is
clamped, no dialog. The text names pieces, because pieces are what the badges show (section 3):

- Very sharp corner (criterion 5): "The corner between pieces 3 and 4 is very sharp: 22°." Several:
  "2 corners are very sharp: between pieces 3 and 4, and 6 and 1."
- Radius too small (criterion 7): "Piece 2 bends to 2.5 mm; below 6 mm the wall may crack." Several:
  "Pieces 2 and 5 bend below 6 mm; the wall may crack there." (6 mm = 2 x Wall thickness; concave:
  below Wall thickness.)
- **One piece** (no corner or one corner, section 3; ADR 0083 §11 "The wall"): the texts say "The
  wall" and never "Piece 1", since no badge shows a 1. "The wall bends to 2.5 mm; below 6 mm it may
  crack." / for one corner: "The corner of the wall is very sharp: 22°." (not "between pieces 1 and
  1"). Check that `instance_warnings.rs` does so for a single piece: its sentences format
  `pieces` as given and have no one-piece case (**for the implementer**). **(Proposal)**
- **Bending inward** (a concave stretch, limit t_w; ADR 0083 §13.12): the limit in the sentence is the
  one that stretch has, so a piece is never told "below 6 mm" for a concave 2 mm bend whose limit is
  3 mm. One sentence per limit: "Pieces 2 and 5 bend below 6 mm; the wall may crack there." and
  "Piece 3 bends inward to 2 mm; below 3 mm the wall may crack." ("bends inward" only for a concave
  stretch; plural "Pieces 3 and 4 bend inward below 3 mm; the wall may crack there."). Today the
  plural takes the first warning's limit for all (**for the implementer**). **(Proposal)**
- **The rounded cover** (criterion 16; ADR 0083 §13.9): where the exact outward offset collapses (a
  concave bend tighter than t_w) or crosses itself (the heart's notch narrower than 2 t_w after the
  offset, a peanut's waist), cover and seat lid are built by the rounded offset, without the hollow.
  Built anyway, one notice: "The notch is too narrow for the cover at 3 mm walls: the cover and lid
  are rounded there, without a finger hollow." (the current Wall thickness). Variants: Lid off or Lid
  seat off, "the cover is rounded there"; Finger hollow off, the sentence ends at "rounded there."
  It extends the spec's "The notch is too narrow for the cover at this wall thickness." (revision 6,
  criterion 16, which says neither what was built nor names the thickness) and, as there, replaces "No place for the finger hollow on
  this lid." (section 5). **(Proposal; for the architect and the product owner**, section 10.)
- Smallest size (`smallest_size`, a resize that stopped): "The box is at its smallest for 3 mm walls."
  (the current Wall thickness), in the canvas notice slot on release. **(Proposal)**
- **Only the full build speaks.** Refusals (crossing, too small) do come from the Draft check of the
  Select bar press, the pick click and Enter, and show as section 1 says. Build-anyway warnings come
  only from the full build: a Draft (the pick hover, the Select bar test, the ghost of a value
  drag, ADR 0083 §13.13) skips the ring opening and the cover test, so its warnings are never shown:
  no "The ring has no opening" flashes during a drag, no rounded-cover notice before release. The
  notices come from the build after the press, click, release or typed value. The red marks of
  section 3 do follow a drag, since they need no plate. **(Proposal)**
- **Where:** after a build, in the canvas notice slot (the button that caused it has gone with the
  selection change), the status look, `role="status"`, 5 s; after a parameter edit, under the edited
  field (`0141` section 6). These are warnings about the result, not success messages, so the "no
  notice after a success" rule does not remove them. The red marks of section 3 keep the state visible
  afterwards.
- These replace the spec's "Corner n is very sharp" and ADR 0083 §4's "near x, y" (coordinates mean
  nothing on a canvas) (**for the product owner and the architect**).

## 7. Layout, convert, files (criteria 8, 11, 12)

- Flat layout (ADR 0083 §8): the wall pieces in the first row in walk order, numbered as section 3; the
  plates in the second row in thickness runs as `0147` §8.
- Operations: one for the whole box, as `0147` §7; Detach part only with `0147` slice 3.
- Object to path: the group of the wall pieces and plates as paths, selected, one step "Convert path box
  to paths"; the badges go with the box (they were never geometry).
- Unknown kind: as `0141` section 12, the stored parts.

## 8. Accessibility

The Select bar button is one Tab stop in bar order. The segment is reached by the radiogroup's arrows;
the pick has the keyboard way of section 1.2 (select, choose the segment, Enter). The overlay is
`aria-hidden`; its content reaches a screen reader through the Corner angle value text and the notices'
live regions. Tokens only; nothing animates.

## 9. Questions

All three are decided (lead, 2026-11-24, **Proposal**, vetoable at the demo; spec revision 3, ADR 0083
revision 2). Kept for the record.

1. **Decided A (Proposal): kinks built in slice 1, only curved cubics refused; the curves refusal
   ended with slice 2 (revision 4).** **Curves and kinks in slice 1** (for the architect and the product owner; slice 1 builds straight
   edges only). Two cases: an outline with a curved cubic, and an all-straight outline whose joins
   turn by less than Corner angle (a 20-gon turns 18° per join; a coffin with Corner angle at 90°),
   which ADR 0083 §3 fillets into bent spans. A: slice 1 builds kinks (bent spans with slits, as the
   rounded box's arcs; a loop with no corner left gets the seam and tab windows) and refuses only
   curved cubics: "This outline has curves. Box from path builds straight outlines only so far.
   Nothing was changed." B: slice 1 refuses both; for kinks: "Corner 3 turns by 18°, below the Corner
   angle of 25°. Nothing was changed." (a refusal on the press only; a Corner angle drag over a corner
   would need a clamp instead). C: the button and the pick hide such outlines. Recommendation and
   default A: kinks need nothing slice 1 does not already take from `0147`, and the maker sees why a
   curve waits for slice 2.
2. **Decided B (Proposal): the layout right of the source, Gap apart, top edges aligned, in the source's
   layer directly above it** (sections 4 and 1). **Where the box lands** (for the architect and the product owner; criterion 1 says "over the
   outline's position"). A: as ADR 0083 §2, the layout centred on the outline, lying over the source.
   B: the layout to the right of the source's box, Gap apart, top edges aligned, so source and parts do
   not overlap and the source stays clickable. Recommendation B: two unrelated sets of lines on top of
   each other read as one broken drawing.
3. **Decided: aligned in spec revision 3.** **"Select one closed outline" as a refusal** with "Nothing was changed." (section 1.2), the piece
   numbers instead of the block labels (section 3), "Inner height" (section 5) and the reworded notices
   (section 6): the product owner aligns criteria 1, 2, 5, 7 and 12.

## 10. Slice 2: items for the architect, the product owner and the implementer

No customer question; Fragen 364 to 366 stay at A. Each item has a default that is built if nobody
objects.

1. **Seam mark** (architect): `Mark::Seam { at, inward }` on the footprint, run time, no format
   (section 3). Default: added with slice 2.
2. **Tight stretches** (architect): `Mark::Bend` carries where in the run it is tight, not one flag
   per run (section 3). Default: one `Bend` per tight or fine stretch of a run, the band drawn
   continuously across them.
3. **Rounded-cover warning** (architect): ADR 0083 §13.9 reports the fallback only as
   `hollow_left_out`, which tells a heart "no place for the finger hollow" when the cause is the notch.
   Default: a warning id `cover_rounded` (with the plates it applied to and whether a hollow was
   dropped) and the sentence of section 6; `hollow_left_out` is not sent with it.
4. **Criterion 16's sentence** (product owner): revision 6 has the round offset and "The notch is too
   narrow for the cover at this wall thickness."; section 6 adds what was built and the thickness.
   Default: the sentence of section 6.
5. **Criterion 13, the shield** (product owner): "three straight pieces ... and one bent bottom piece"
   counts spans as pieces; by criteria 6 and 14 the sides run into the bottom curve without a cut, so
   the shield has 2 pieces and 2 corners (section 3). Default: the golden records 2 pieces, the
   criterion says spans.
6. **Draft against the built box** (architect). (a) The ring in a value drag: `0147` §2 keeps the
   ring's opening in the ghost because a ring without it reads as a fifth plate; ADR 0083 §13.13
   skips it in every path-box Draft. Accepted as the ADR says (**Proposal**): during a drag the ring
   shows full, it gets its opening on release; the one difference from `0147` §2, and the hover draws
   no plates anyway. (b) The bands on the hover: if a Draft lays windows without the slit-driven
   lowering of §13.8, its band gaps differ from the built box's. Default: accepted, the hover is
   approximate there; crosses, red and seam must match.
7. **Sentences** (implementer, `instance_warnings.rs`): "The wall" for one piece, one sentence per
   limit with "bends inward" for concave stretches (section 6); the `outline_curved` arm and
   `SourceRefusal::Curved` go (ADR 0083 §13.14).
8. **Stripes** (implementer, `path_box_marks.rs`): 4px by arc length, one stripe at a join
   (section 3).
9. **What a band covers** (architect): no stripes on windows, on the flats beside a corner or in the
   seam margin (section 3) needs `Mark::Bend` to hold the slit bands, not the whole run; today it is
   "the stretch of the inner face" of the bent span (§11). Default: `Bend` segments are the slit
   bands, split at windows.

To go into `docs/design-system.md` when Done: the Box from path button and segment, the pick of a
generator source (and its Replace outline use), the corner cross, the bend hatch and the seam tick, the
piece badges.
