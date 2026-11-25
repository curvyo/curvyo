# 0149 Path box: UX notes

ux-engineer, 2026-11-24, revision 3. Checked against `specification.md` revision 3 (criteria 1 to 12;
Fragen 364 to 366 at their defaults A), ADR 0083 revision 2 (§1 kind and label, §2 contract, source,
placement and checks, §3 classification, §4 one rule per span, §5 corner joints, §7 plates and hollow,
§8 parameters and layout, §11 warnings and marks), `adrs.md`, `specs/0147-rounded-box/ux-notes.md` revision 3 (the shared block; everything not
named here is as there), `specs/0141-parametric-generators/ux-notes.md` and `docs/design-system.md` (rows
"Pick mode", "Select bar layout", "Stroke to path", "Action notice", "Canvas notice slot", "Drag ghost",
"Modifier badge", "Hidden, not disabled", `--mixed-hatch`).

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
  before create).
- **Press:** one step "Add path box"; the box is built from the outline (section 4), selected, the Style
  tab shows (`0141` section 2), focus goes to the canvas. The source is deselected and untouched.
- **Refused** (Action notice under the button, the refusal look, `role="alert"`, 8 s; **only there**, not
  a second time in the canvas notice slot, **(Proposal)**, lead): "The outline
  crosses itself. Nothing was changed." / "The outline is too small for a box with 3 mm walls. Nothing
  was changed." (area below (4 t_w)², t_w the default Wall thickness). In slice 1 an outline with a
  curved cubic: "This outline has curves. Box from path builds straight outlines only so far. Nothing
  was changed." (ADR 0083 §2: the tests are one `generate` with the default values, run in ui-core
  before create).
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
    self-intersection, size and, in slice 1, no curved cubic; one `generate` per hovered object that
    passes the Select bar test, cached until it changes, ADR 0083 §11 A) draws it 2px `--hover-box`
    on its casing, with the corner marks and kink hatch of section 3 at the default Corner angle (25°). Cursor `pointer`;
    elsewhere, and over an ineligible object (a curved, crossing or too small outline included), the
    default arrow and no highlight.
  - A **click** on it builds the box from it: one step "Add path box", the box selected, back to Select
    (`0141` section 2). The source is untouched.
  - A click on an **ineligible** object (open path, group, text, crossing outline, too small, in slice 1
    an outline with curves): nothing is built, the refusal in the canvas notice slot (the refusal look,
    `role="alert"`, 8 s, as every text ending "Nothing was changed." there; warnings stay in the status
    look, section 6; **(Proposal)**, lead): "Select one
    closed outline. Nothing was changed."; for a crossing, too small or curved outline the texts of
    1.1, so the curves case reads "This outline has curves. Box from path builds straight outlines
    only so far. Nothing was changed.".
  - A click on **empty canvas** does nothing and says nothing (the Pick mode rule: a miss is not a
    request).
  - **Enter** builds from the current selection when it is one eligible outline (the keyboard way:
    select with Select, then the tool and Enter); otherwise the refusal "Select one closed outline.
    Nothing was changed." in the canvas notice slot, or the crossing, too small or curves text of 1.1
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

## 3. Corners and curves on the canvas (criteria 3, 5, 7, 12)

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
- **Bent span** (a `Bend` mark, ADR 0083 §11; always slit): **slice 1** for the filleted kinks, which
  are built in slice 1 (question 1 A); slice 2 adds the curved cubics. A band 6px wide on screen just
  inside the footprint line on the bent stretch, 45° stripes 1px wide every 4px in `--drag-ghost` (the
  `--mixed-hatch` geometry, an existing colour). A stretch whose radius is too small to bend (criterion 7) carries the stripes in
  `--field-invalid` instead, so the place the wood may crack stays visible while the box is selected,
  not only for the notice's few seconds. Straights carry nothing.
- **Piece numbers** (criterion 12): the wall pieces are numbered in walk order from the outline's start
  (ADR 0083 §8, the part keys `wall-1` to `wall-n`). Each number is a **badge** in the "Modifier badge"
  look of the Pick set row (16px circle, solid `--accent`, white 10px tabular digit, 1px white ring, DOM,
  `aria-hidden`): one at the middle of each wall piece in the layout, and one on the inner base 12px
  inside the middle of the stretch that piece stands on. So the maker matches piece 3 to its place on
  the base, and the notices can name pieces. A piece shorter than 24px on screen leaves its badges out.
  With no corner or one corner there is one piece and no badge.
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
  as section 1.2 ("Select one closed outline. Nothing was changed.", or the crossing, too small or
  curves text of 1.1), in the canvas notice slot in the refusal look; the pick stays armed.
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

## 5. The Generator block (criterion 2)

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
  is there without the canvas. A higher angle turns joints into curves; the hint of that is the overlay,
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

## 6. Notices that build anyway (criteria 5, 7)

The customer: "if the radius is too small: own fault, we just do it". Nothing is refused, nothing is
clamped, no dialog. The text names pieces, because pieces are what the badges show (section 3):

- Very sharp corner (criterion 5): "The corner between pieces 3 and 4 is very sharp: 22°." Several:
  "2 corners are very sharp: between pieces 3 and 4, and 6 and 1."
- Radius too small (criterion 7): "Piece 2 bends to 2.5 mm; below 6 mm the wall may crack." Several:
  "Pieces 2 and 5 bend below 6 mm; the wall may crack there." (6 mm = 2 x Wall thickness; concave:
  below Wall thickness.)
- Smallest size (`smallest_size`, a resize that stopped): "The box is at its smallest for 3 mm walls."
  (the current Wall thickness), in the canvas notice slot on release. **(Proposal)**
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

1. **Decided A (Proposal): kinks built in slice 1, only curved cubics refused.** **Curves and kinks in slice 1** (for the architect and the product owner; slice 1 builds straight
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

To go into `docs/design-system.md` when Done: the Box from path button and segment, the pick of a
generator source (and its Replace outline use), the corner cross and the bend hatch, the piece badges.
