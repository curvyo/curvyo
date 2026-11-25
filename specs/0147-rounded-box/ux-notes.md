# 0147 Rounded box: UX notes

ux-engineer, 2026-11-25, revision 6 (slice 2, the curved top, section 10). Checked against
`specification.md` revision 8 (criteria 1 to 13 for slice 1, criteria 11 and 16 to 24 for slice 2,
criterion 15 for slice 3; Fragen 349 B, 355 C, 360 B, 361 A, 362 B, 370 A answered, 348, 350 to 353 and 371
at A, 363 at its default A), ADR 0082 revision 6 as amended the same day (§5 parts, thicknesses, tabs and flat windows, clamps,
layout; §6 Wave; §7 automatic values; §8 parameters; §10 Detach part; §11 the curved top), `adrs.md`, the exploded drawings 34 (stadium) and 35 (rounded square),
`specs/0141-parametric-generators/ux-notes.md` (sections 1 to 15; the Rounded box is a further generator
of the same contract, so everything not named here is as there) and `docs/design-system.md` (rows "Value
field", "Parameter handle", "Parameter handle layout", "Corner radius readout and chip texts", "Drag
ghost", "Live preview outline", "Hidden, not disabled").

Customer rules applied: no popups in the panel, no operating tips or hint labels, no notice after a
success, hidden not disabled. No new colour token, no new component. Everything here that the spec, ADR
0082 or a decided Frage does not fix (group order, labels, tracks, knob positions, clamp texts, the
automatic state, the hollow defaults, the ghost, where Detach part sits) is **(Proposal)**, vetoable at
the demo.

**Revision 6 changes** (ADR 0082 §11 amended): the Curved clamp numbers in 10.10 follow the amended
bounds (at the defaults the most Corner radius is 32 mm, the least Inner height 24 mm, the hollow width
at most w - 2 t_e); 10.3 names the thickness keys; the hollow in the front's top edge and the part order
are decided; section 12 items 11 to 16 are resolved, item 17 stays open.

**Revision 5 changes** (slice 2 Ready): section 10 is rewritten for the curved top: the Top shape strip
in Size, the rows Curved hides (kept, restored on Flat), End plate thickness and Front thickness in
Material, the finger hollow in the front's top edge, the drag preview, the knobs, the part badges 1 to 5,
the bend band on the back arc, the clamp texts, the lid that rests without a catch (Frage 371 A); sections
3, 6, 11 and 12 follow. The look of the Generator block as a whole is revisited by `0151`
(section 10.12).

**Revision 4 changes** (the UX browser review of slice 1): the hollow width limit subtracts the corner
arcs once (section 5); the Wave spacing notice says "the least the Wave pattern allows" (section 5); a
Generators bar segment is as wide as its name, 84px at least (section 1); tooltip and `aria-valuetext`
end ", follows wall thickness" (section 3); the block's Convert to paths button waits for a later slice,
Object to path does the job in slice 1 (sections 3, 8).

**Revision 3 changes:** Wave is the third cell of Slit pattern; one thickness field per part (Inner base,
Cover, Ring, Lid), each following Wall thickness until typed; the corner radius runs to half the shorter
side (an exact circle or a stadium) with the readout "full round" at that limit and knob anchors for a
footprint without straights; layout runs by thickness value; no Detach part in slice 1, its place in
slice 3 is sketched in section 9; section 11's questions are mostly resolved.

## 1. The entry in the Generators bar (criterion 1)

- **A further segment** of the Generators bar's `role="radiogroup"` "Generator" (`0141` section 1): text
  "Rounded box" with Lucide `SquareRoundCorner` 16px (in lucide-react 1.50, the installed version)
  **(Proposal)**: a square with one round corner says "the box with round corners" next to Box's cube;
  the rail button and the tool are unchanged.
- **Order.** The registry rule of `0065` criterion 3 (built-ins by name) stays: **Box, Gear, Rounded box**
  (with `0149`: Box, Box from path, Gear, Rounded box). The two boxes do not sit side by side; the name
  rule wins over grouping, because a third-party generator must find its place by the same rule.
- **Segment width.** A segment is as wide as its name needs on one line (text plus glyph and padding),
  84px at least **(Proposal)**: at a fixed 84px "Rounded box" wrapped to two lines and made its segment
  taller than the others. Short names (Box, Gear) stay 84px. Four segments fit the bar from an 800px
  window; a narrower bar wraps whole segments by the bar overflow rule.
- First use of the session is still Box; the last choice is kept for the session (`0141`).

## 2. Ghost and placing (criterion 1)

- **Ghost.** The rounded box at its defaults in the "Drag ghost" look and its flat layout (section 8),
  following the pointer, centred on its layout box, snapping as a move (`0141` section 2). It draws the
  **outer outline of every part** (strip with its edge notches, inner base with its tabs, cover, ring,
  lid with its finger hollow) **and the ring's opening**, but **no slits and no tab slots** **(Proposal)**:
  several hundred slot outlines at a low zoom read as grey noise and cost a frame, while the ring without
  its opening would read as a fifth plate. The placed object draws everything at once. Wave changes
  nothing here (slits are not in the ghost).
- Click or press-drag-release places; one step "Add rounded box"; selected; back to Select; the Style tab
  shows. Enter and Escape as `0141`.
- Layers: glyph `SquareRoundCorner`, name "Rounded box"; subject line "Rounded box", "2 rounded boxes";
  status bar "Rounded box · <page>". Not enterable in slice 1 (one object, as the Box; section 9 for
  slice 3).

## 3. The Generator block (criteria 2 to 5, 9, 10, 12, 13)

The `0141` block (section 3, revision 2 section 15): Style tab, first block, a heading per group, every
number a Value field 244 x 28px, choices as choice strips, on/off as a `PanelSwitch` 28px row with the
label left (`0141` section 4, the Keyway precedent), nothing after the last group. Order is the schema
order; ADR 0082 §8 fixes the keys, the schema follows this order **(Proposal)**:

| Group | Row | Element | Shown when |
|---|---|---|---|
| Size | Inner width, Inner depth, Inner height | length | always |
| | Corner radius | length; default 15 mm (Frage 348 A), up to half the shorter side (Frage 360 B) | always |
| | Top shape | choice strip **Flat / Curved** (Frage 353 A), two cells of 122px | always from slice 2 (section 10.2); not drawn in slice 1 |
| Material | Wall thickness | length | always |
| | End plate thickness, Front thickness | length, follows Wall thickness | Top shape **Curved** (slice 2, section 10.3) |
| | Kerf compensation | length | always |
| Slits | Slit pattern | choice strip **Straight / Offset / Wave** (Frage 349 B), three cells of 80, 80, 84px | always |
| | Wall slits | choice strip **Corners only / All round** (criterion 3), 122px cells | Top shape Flat (also on a circle, where both give the same strip; see below) |
| | Slit width, Slit spacing | length, automatic by default | always |
| | Slit rows | count, automatic by default | Slit pattern **Offset** only (ADR 0082 §8 `ShownWhen::Is`; Straight and Wave have one slot per column) |
| Base | Inner base thickness | length, follows Wall thickness | always |
| | Base inset | length, default 0 (criterion 10) | Top shape Flat |
| | Base cover | switch, on (criterion 10) | Top shape Flat |
| | Cover thickness | length, follows Wall thickness | Top shape Flat and Base cover on |
| Lid | Lid | switch, on | Top shape Flat |
| | Lid thickness | length, follows Wall thickness | Top shape Flat and Lid on |
| | Lid seat | switch, on (criterion 12) | Top shape Flat and Lid on |
| | Ring thickness | length, follows Wall thickness | Top shape Flat and Lid on and Lid seat on |
| | Finger hollow | switch, on (criterion 13) | (Top shape Flat and Lid on and Lid seat on) or Top shape Curved |
| | Hollow width, Hollow depth | length | Finger hollow shown and on |
| Layout | Gap | length | always |

- **What the switches make** (ADR 0082 §5): Lid off is an open box (strip, inner base, cover). Lid on
  with Lid seat on is the drawings: the seat ring with tabs and the plain lid with its finger hollow
  lying on it. Lid on with Lid seat off is a second inner plate with tabs as the lid, Lid thickness
  thick; no hollow, since a notch in an inner plate opens into the box. Base cover off leaves the inner
  base flush at the bottom.
- **Each part's thickness sits with the part** (Frage 355 C): Inner base thickness heads Base, Cover
  thickness sits right under Base cover, Lid thickness right under Lid, Ring thickness right under Lid
  seat. A switch shows or hides the rows right below it and nothing elsewhere moves. Only Wall thickness
  stays in Material, because the strip has no switch and every other thickness follows it. The label
  is **"Wall thickness"** (the spec's word), not "Thickness": with five thickness fields a bare
  "Thickness" says nothing. The schema key stays `thickness`.
- **No Convert to paths button in slice 1** (lead decision, **Proposal**): `0141` section 11's full-width
  button last in the block is deferred to a later slice. In slice 1 the Select bar's **Object to path**
  gives the separate plates (section 8), so nothing is missing; the block ends with Layout.
- **No field** for Ring width (2 x Ring thickness, ADR 0082 §8), Finger width or Clearance (derived; they
  join a Joints group if the customer asks).
- **Corner radius in Size**, because it shapes the footprint; at its maximum the footprint is an exact
  circle (equal sides) or a stadium, no special case, no notice (Frage 360 B; section 6 for the readout).
- **Wall slits on a circle** stays shown: a circle has no straight, so both cells give the same strip.
  Hiding it would need a "shown when" on a computed value (a straight exists), which the schema does not
  carry, and a strip that comes and goes while the radius slider passes the limit would jump the rows
  below it. **(Proposal)**
- **Strips and switches name themselves.** Accessible names "Slit pattern", "Wall slits", "Top shape";
  switch names are their labels. No captions.
- **Automatic values** (Slit width, Slit spacing, Slit rows; Frage 350 A; ADR 0082 §7: a stored 0 means
  automatic and the generator reports the value it used): the Line spacing Auto precedent of the Value
  field row. While automatic, value and unit are drawn in `--panel-muted-fg`, `aria-valuetext` ends
  ", automatic" ("0.3 millimetres, automatic"), the reset icon is hidden. A drag, click, arrow or typed
  value makes it the maker's own (drawn normal, reset icon shown); the reset slot or Ctrl+Backspace
  returns it to automatic (step "Rounded box: Automatic slit width"). An own value is kept when radius or
  thickness change, clamped if it no longer fits.
- **The four part thicknesses follow Wall thickness** (ADR 0082 §5, a stored 0): the same muted state,
  `aria-valuetext` "3 millimetres, follows wall thickness", reset icon hidden; tooltip first line "Lid
  thickness: 3 mm, follows wall thickness". Tooltip and `aria-valuetext` end in the same suffix,
  **", follows wall thickness"** (lower case, a phrase, not the field label) **(Proposal)**; an automatic
  slit value ends ", automatic" in both. Typing, dragging or an arrow key makes the value the part's
  own; reset returns it, one step each: "Rounded box: Inner base thickness follows wall thickness",
  "... Cover thickness ...", "... Lid thickness ...", "... Ring thickness ...". Changing Wall thickness
  moves every following field in the same frame and leaves own values alone. A following value that a
  dependent clamp limits (Ring thickness on a small box) shows the used value, still muted and still
  following.

## 4. Tracks (as `0141` section 15.3)

Lengths show two decimals unless named; Kerf three, as the Box. `t_w` is Wall thickness.

| Field | Track | Curve | Step (Shift, Ctrl) | Note |
|---|---|---|---|---|
| Inner width, Inner depth, Inner height | 10 to 500 mm, to the schema max | log | 1 (10, 0.1) | as the Box |
| Corner radius | 1 to 100 mm, to the schema max | log | 0.5 (5, 0.05) | dependent: 2 t_w up to min(width, depth) / 2 by clamp; within 0.01 mm of the top it snaps to it (ADR 0082 §5) |
| Wall thickness, Inner base thickness, Cover thickness, Lid thickness, Ring thickness | 0.1 to 12 mm | linear | 0.1 (1, 0.01) | as the Box; the last four follow Wall thickness |
| Kerf compensation | 0 to 0.5 mm | linear | 0.01 (0.1, 0.001) | as the Box; dependent max half the thinnest part |
| Slit width | 0.2 to 2 mm | linear | 0.05 (0.5, 0.005) | automatic (kerf + 0.3 mm); three decimals |
| Slit spacing | 0.5 to 20 mm | log | 0.1 (1, 0.01) | automatic (0.22 x corner radius); Wave raises the minimum (section 5) |
| Slit rows | 1 to 10, to the schema max | linear | 1 (10, -) | automatic; the height limits it by clamp |
| Base inset | 0 to 50 mm | linear | 0.5 (5, 0.05) | between 0 and t_w snaps to t_w, by clamp |
| Hollow width | 5 to 60 mm | linear | 1 (10, 0.1) | default 16 mm |
| Hollow depth | 2 to 30 mm | linear | 0.5 (5, 0.05) | default 8 mm; at least half the width, at most 2 x Ring thickness + Lid thickness |
| Gap | 0 to 50 mm | linear | 1 (10, 0.1) | as the Box |

The hollow defaults fit the default box: the depth may reach 9 mm (2 x 3 + 3), so a half circle of
16 mm (depth 8) is the widest common size that ends over the ring, about one finger tip.

## 5. Clamping (criteria 4, 6)

`0141` sections 6 and 15.2: applied clamped, the notice under the field, no dialog. Texts (at the
defaults):

- "Corner radius set to 6 mm, the least this box allows." (2 x Wall thickness)
- "Corner radius set to 40 mm, the most this box allows." (half the shorter side; the box is then full
  round, section 6)
- "Slit rows set to 3, the most this box allows."
- "Slit width set to 0.2 mm, the least this box allows." (kerf + 0.2 mm)
- "Slit spacing set to 1.3 mm, the least this box allows." (slit width + 1 mm)
- "Slit spacing set to 1.48 mm, the least the Wave pattern allows." ((slit width + 1 mm) / 0.88, ADR 0082
  §5; also as the knock-on sentence under Slit pattern when Wave is chosen with an own spacing below it).
  The wording is exactly this: "the Wave pattern", not "this box", because the pattern sets the limit.
- "Kerf compensation set to 1.5 mm, the most this box allows." (half the thinnest part)
- "Base inset set to 3 mm, the least a raised base allows." (a value between 0 and Wall thickness)
- "Ring thickness set to 18.5 mm, the most this box allows." ((min(width, depth) - 2 t_w) / 4, so the
  opening stays at least two wall thicknesses across)
- "Hollow width set to 90 mm, the most this box allows." (the front straight, w - 2 r: 120 - 30 = 90 at
  the defaults; the corner arcs are subtracted once, not again from the straight. Above the track's
  60 mm, so at the defaults only a typed value reaches it; on a circle, with no straight, r)
- "Hollow depth set to 9 mm, the most this box allows." / "Hollow depth set to 8 mm, the least this
  box allows." (2 x Ring thickness + Lid thickness; half the width)

Knock-on in the same step, the sentence under the edited field names the other field: a thicker wall
or a smaller footprint clamps the corner radius ("Corner radius set to 8 mm, the least this box
allows."); a thinner ring or lid clamps the hollow depth; a wider hollow raises the depth; a smaller
footprint clamps the ring thickness.

## 6. Knobs on the canvas (criterion 7)

The `0141` section 9 "Parameter handle" (10px knob, 12px hit, hover dimension in the "Vector dimension
look", readout chip, Shift whole mm, double-click entry chip, one step per drag, Escape restores). The
generator names each anchor (`0141` section 4, Handle anchor):

| Handle | Position (object frame) | Drag | Readout |
|---|---|---|---|
| Inner width | inner base, on the footprint line (the tab root, not a tab tip) at the right side's middle: the midpoint of the right straight, or the arc's rightmost point when that side has no straight (stadium, circle) | along its normal | "Inner width 120.0 mm" |
| Inner depth | inner base, the same rule at the top side | along its normal | "Inner depth 80.0 mm" |
| Inner height | strip, top edge, the solid point nearest the middle of the first straight after the seam; with no straight, the middle of the first tab window after the seam, on the bridge beside the ring's notch (windows carry no slits) | along its normal | "Inner height 50.0 mm" |
| Corner radius | inner base, top-right corner, on the inward diagonal by the "Parameter handle layout" rectangle rule | along the diagonal | "r 15.0 mm"; at the lower bound "r 6.0 mm min"; **at the upper bound "r 40.0 mm full round"** |

- **"full round"** replaces "max" at the radius's upper bound, because that limit is not a stop but a
  shape (Frage 360 B): every corner is a half circle, an exact circle on equal sides and a stadium
  otherwise. It is a state name, not a hint. The same words end the field's `aria-valuetext` ("40
  millimetres, full round") and the first tooltip line ("Corner radius: 40 mm, full round"). The Value
  field itself draws nothing extra: its fill already stands at the track's end. **(Proposal)**
- No knobs for thicknesses, Kerf, slit values, Base inset, the hollow and Gap (too small to aim at, a
  count, or fields only). Collisions as `0141` section 9. On a circle the width and depth knobs sit a
  quarter turn apart on the arc and the radius knob at its rectangle-rule place on the diagonal; at the
  default sizes they are more than 40px apart at 100 % zoom.
- A knob drag regenerates the strip, so the slit count follows the new perimeter; the blue live preview
  shows every part with its slits, in Curved too (section 10.6).

## 7. Operations in slice 1 (criteria 7, 15 first half; Frage 363 A)

- The box is one object for `0101`: assigning it to an operation assigns every part; the Operations
  panel lists it once, by its name. There is no per-part control anywhere in slice 1: no Detach part, no
  part rows, no part selection.
- To cut parts of different thickness on different sheets or with different power, the maker uses
  **Object to path** (section 8) and assigns the plates. Nothing in the block points there (no hint
  label); the thickness runs of the layout make each sheet's parts one row.

## 8. Layout, resize, convert, files (criteria 7, 8)

- **Flat layout** (ADR 0082 §5): the strip alone in the first row; the plates in the second row in runs
  of equal thickness value, the runs in the order of their first part in **cover, lid, inner base,
  ring**, `gap + kerf` apart, everything centred on the origin. With every thickness following Wall
  thickness that is one run: cover, lid, inner base, ring. A typed Lid thickness of 4 mm makes the lid its
  own run after the 3 mm one: cover, inner base, ring, then lid. Two parts with equal typed values share
  a run. Parts that a switch turns off leave no hole in the row.
- **Resize:** proportional only, corner handles, the factor on Inner width, depth and height (the Box
  rule of `0141` section 8); readout the outer size of the layout ("W × H mm"). Corner radius,
  thicknesses, slit values and the hollow stay; a value that no longer fits is clamped and the sentence
  goes to the canvas notice slot on release. A full-round box made larger keeps its radius and so is no
  longer full round (question 9).
- **Object to path** (Select bar) gives the group of the shown parts (three to five) as separate plates,
  selected, one step "Convert rounded box to paths". In slice 1 this is the only way to convert (section 3:
  the block's Convert to paths button comes in a later slice).
- Unknown kind: as `0141` section 12.

## 9. Detach part (criterion 15, slice 3; not built in slice 1)

Slice 1 has no trace of it (section 7). When slice 3 is Ready, these notes get a revision; the
direction, so slice 1 leaves room **(Proposal, to confirm when slice 3 is Ready)**:

- **Where.** A **Parts** group last in the Generator block, after Layout: one row per shown part
  (Strip, Inner base, Cover, Ring, Lid), the part's name and its operation's name muted ("Cut 3 mm"), and
  at the right a text button **Detach** (or **Attach** once detached). Hovering a row draws that part in
  the `--hover-box` look on the canvas. The rows are the way in, so the box needs no "enter" mode and the
  canvas hit test stays one object (slice 1's "not enterable" still holds). The part's context-menu entry
  the spec names needs part hit testing inside the box; it is left out unless the customer asks.
- **Which operation Detach writes:** a **new copy of the box's operation**, named "<operation> · Lid",
  placed right after it in the Operations list. The job is the same until the maker changes the copy, so
  Detach itself never changes the cut; picking another existing operation is the copy's own operation
  row (`0101`), not a second path in Detach. One step "Detach part"; Attach part one step "Attach part".
- **Format.** A detached part bumps `format_version` (ADR 0082 §10); that is the customer's acceptance of
  slice 3, not a UI matter.

## 10. Top shape Curved (criteria 11, 16 to 24; slice 2)

Slice 1 builds no part of this (section 3). Slice 2 follows spec revision 8 and ADR 0082 §11 (revision
6, amended); the two agree on everything these notes depend on (section 12). Everything here is **(Proposal)** unless a Frage is named.

**10.1 What the maker gets.** The strip runs from the lid's free front edge over the top, round the back
arc (the slit hinge, radius = Corner radius, Frage 353 A, one radius field, label unchanged) and down the
back; two end plates carry the profile; the front and the base are finger-jointed plates. The footprint
corners are square. The strip's front run is the lid: it rests on the seat and opens on the hinge.

**10.2 Top shape in the block.** A choice strip **Flat / Curved** (Frage 353 A), two cells of 122px (the
Wall slits geometry), accessible name "Top shape", in **Size right after Corner radius** (ADR 0082 §11
key order). Everything Top shape shows or hides lies below it, so nothing above it moves. One Tab stop,
arrows move within it (the strip pattern of Slit pattern). One step, **"Rounded box: Top shape"** (the
`0141` step-name rule "<kind>: <field>", spec criterion 11).
Focus stays on the strip. The layout is rebuilt round the same centre, so the box does not jump away
from where the maker put it.

**10.3 Rows in Curved.**

| Group | Shown in Curved | Hidden in Curved |
|---|---|---|
| Size | Inner width, Inner depth, Inner height, Corner radius, Top shape | |
| Material | Wall thickness, **End plate thickness**, **Front thickness**, Kerf compensation | |
| Slits | Slit pattern, Slit width, Slit spacing, Slit rows (Offset only) | Wall slits (the arc is the only bend) |
| Base | Inner base thickness (the base) | Base inset, Base cover, Cover thickness |
| Lid | Finger hollow, Hollow width, Hollow depth | Lid, Lid thickness, Lid seat, Ring thickness |
| Layout | Gap | |

- **Hidden, not cleared.** The eight hidden rows keep their stored values, are saved back unread and
  come back exactly as they were when the maker returns to Flat (ADR 0082 §11). Hidden as the design
  system's "Hidden, not disabled": out of the tree, no greyed row, no reserved space.
- **A value both modes show is re-clamped on the switch and stays clamped.** Corner radius, Inner
  height, the hollow sizes and Kerf are visible in both modes, so the clamp writes them: a Flat box at r
  40 switched to Curved is r 32, and switched back it is still r 32, not 40 (one rule, the stored value
  is the shown value; the undo of the switch restores all of it in one step). The sentence sits under the
  Top shape strip and names the field (section 5, knock-on rule): "Corner radius set to 32 mm, the most
  this box allows."
- **End plate thickness and Front thickness** (new keys `end_plate_thickness` and `front_thickness`,
  Frage 355 C, spec criterion 21, ADR 0082 §11):
  Lengths that **follow Wall thickness** exactly as the four part thicknesses of section 3 (stored 0,
  muted, reset slot hidden, `aria-valuetext` "3 millimetres, follows wall thickness", tooltip "End plate
  thickness: 3 mm, follows wall thickness", reset steps "Rounded box: End plate thickness follows wall
  thickness" and "... Front thickness ..."). Track as the other thicknesses (0.1 to 12 mm, linear, 0.1
  (1, 0.01)). They sit in **Material right under Wall thickness**: the end plates and the front have no
  switch, and section 3's rule puts a thickness without a switch next to the one it follows. Shown only
  in Curved; in Flat they are hidden and kept.
- **The base keeps "Inner base thickness"** (key `plate_thickness`, one static label per key, ADR 0082
  §11; spec criterion 21).
  In Curved it is the only row of Base, so the label still finds its part. `0151` may rename it (10.12).

**10.4 Finger hollow.** A U notch (Width, Depth; the same keys and the stored on/off of Flat, so a
hollow turned off in Flat is off in Curved too) **in the front's top edge, centred across the width,
right under the lid's free edge** (decided: spec criterion 21, ADR 0082 §11). The lid lies over the
front's top edge, so a notch in the front leaves a gap under the lid's edge for a finger tip; a notch in
the lid's own edge would show the front's edge through it and leave nothing to lift from below.
The rows stay under the **Lid** heading, because the hollow serves the lid; in Curved that heading holds
only Finger hollow and its two fields (no Lid switch). Hollow width and depth shown while Finger hollow
is on, as in Flat.

**10.5 Lid closure.** The lid **rests on the seat** (the end plates' and the front's top edges) and opens
on the hinge; no stop, no catch, no magnet pocket (Frage 371 A, **Proposal**). Nothing in the block
speaks of it: no Closure row, no hint. Springback is checked at the demo; a catch (Frage 371 B, C) would
be a later row in the Lid group.

**10.6 Ghost and preview.**

- **Placing:** a new rounded box is always Flat at its defaults, so the placement ghost is section 2's,
  unchanged. The session does not remember Top shape (it remembers only the segment, section 1).
- **A ghost of a curved box** (any host request for `Edit::Draft`): the five parts without slits, the
  strip as its plain L by W rectangle and the plates as built with their fingers, tabs, arc and hollow
  (ADR 0082 §11; spec criterion 23).
- **A value or knob drag:** the blue live preview shows every part with its slits, as Flat (section 6).

**10.7 Knobs** (the section 6 "Parameter handle" behaviour).

| Handle | Position (object frame) | Drag | Readout |
|---|---|---|---|
| Inner width | base (part 5), the middle of its right edge, on the finger root line | along its normal | "Inner width 120.0 mm" |
| Inner depth | base, the middle of its back edge (the edge that meets the strip), on the root line | along its normal | "Inner depth 80.0 mm" |
| Inner height | front (part 4), top edge, midway between the hollow's right side and the edge's right end (the edge's middle with the hollow off) | along its normal | "Inner height 50.0 mm" |
| Corner radius | **on the back arc** of end plate 2: the arc's midpoint | along the inward diagonal (towards the arc's centre grows r); the knob stays on the arc | "r 15.0 mm"; "r 6.0 mm min"; "r 32.0 mm max" |

- **No "full round" in Curved** (ADR 0082 §11): the upper bound is a stop, so it reads "max" in the chip,
  the tooltip ("Corner radius: 32 mm, max") and `aria-valuetext`, as any other bound.
- The height knob keeps clear of the hollow, so it never sits in the notch; it shifts when the hollow is
  switched or resized.
- No knob for the new thicknesses (section 6 rule).

**10.8 Part numbers.** The parts are numbered by spec criterion 16: **1 strip, 2 and 3 end plates (left,
right; one outline, the second flipped), 4 front, 5 base**. While the box is selected alone, each part
carries its number as a **badge** in the "Modifier badge" look of `0149` section 3 (16px circle, solid
`--accent`, white 10px tabular digit, 1px white ring, DOM, `aria-hidden`, no hit area) at its middle;
on the strip at the middle of the lid run (solid, no slits). A part under 24px on screen leaves its
badge out. Badges are not engraved and go with Object to path (never geometry). The numbers help where
two plates look alike (front and base are both fingered rectangles, the end plates are a pair). In
slice 2 no notice names a part (every Curved notice is a clamp that names a field, 10.10); the numbers
are the names slice 3's Parts rows will use ("2 End plate", section 9). Flat gets no badges in slice 2.

**10.9 Marks: the bend band on the back arc.** While the box is selected alone, the arc of **each end
plate** (2 and 3) carries the `0149` section 3 bend band: 6px on screen just inside the arc line, 45°
stripes 1px every 4px by arc length in `--drag-ghost`, `aria-hidden`, no hit area, never cut, exported
or in the job. It says "the strip bends here" on the plates the maker assembles it to; the strip itself
already shows its slits. **Never red**: the Corner radius is clamped up to a bendable radius (criterion
20), so no stretch is too tight. The radius knob draws above the band. Flat draws no band (slice 1
emits no marks; section 12).

**10.10 Notices** (section 5 rules: applied clamped, the sentence under the field, no dialog; numbers at
the defaults 120 x 80 x 50, all thicknesses 3 mm, by ADR 0082 §11, revision 6 as amended):

- "Corner radius set to 6 mm, the least this box allows." (2 x Wall thickness, or more where the arc
  needs it for two slit columns)
- "Corner radius set to 32 mm, the most this box allows." (the smaller of r ≤ d - 2 t_w, the lid's flat
  run over the front, 74 mm, and r ≤ h - 2 t_e - 4 t_w, the back straight for one tab, 32 mm)
- "Inner height set to 24 mm, the least this box allows." (2 t_e + 6 t_w, so the radius range is never
  empty)
- "Hollow width set to <limit> mm, the most this rounded box allows." (limit min(w - 2 t_e, h): clear of the front's side fingers, and no deeper than the front is high; browser review 2026-11-25)
- "Hollow depth set to 25 mm, the most this box allows." (h / 2) / "Hollow depth set to 8 mm, the least
  this box allows." (half the width)
- Kerf compensation and Wall thickness as section 5 (half the thinnest part; a sixth of the smaller
  inside side). Slit rows, width and spacing as section 5.
- Every Curved clamp uses section 5's one form ("set to <value>, the most/least this box allows"),
  as spec criterion 20 now does; no sentence explains the cause (no-hint rule).

**10.11 Layout, resize, convert, files.**

- **Flat layout:** the strip alone in the first row, lying with its free front edge at the left and the
  back run ending at the right, the order it wraps; the second row in number order, **end plate 2, end
  plate 3, front 4, base 5**, `gap + kerf` apart, everything centred on the origin. The end plates show
  the profile with the front at the left and the arc at the top right. With typed thicknesses the
  section 8 run rule applies in number order: Front thickness 4 mm gives end plates, base, then front.
- **Resize:** proportional only, Inner width, depth and height; Corner radius, thicknesses, slit values
  and the hollow stay; a value that no longer fits is clamped, its sentence in the canvas notice slot on
  release (section 8).
- **Object to path:** the five plates as separate paths (both end plates), selected, one step "Convert
  rounded box to paths".
- **Files:** a slice-1 file has no `top_shape` and opens Flat; a curved box opens exactly as stored
  (spec criterion 24). Nothing in the UI marks the mode beyond the Top shape strip.

**10.12 Look of the block: revisited by `0151` generator-ui-polish.** Slice 2 adds only what Curved
needs and keeps the slice-1 block and canvas aids as they are. The planned spec `0151`
generator-ui-polish revisits the look of the generator blocks and their canvas aids as a whole, across
Box, Gear, Path box and Rounded box: a group that shrinks to one row (Base in Curved), a group heading
without its switch (Lid in Curved), the label "Inner base thickness" on a base that is not inner, part
badges and bend bands in Curved only (not in Flat), the deferred Convert to paths button (section 3) and
the segment widths of section 1. Nothing in slice 2 waits for it; whatever `0151` changes here is its
own customer-visible slice.

## 11. Accessibility

Tab order is screen order: the Size fields and Top shape (slice 2), Material (in Curved with End plate
and Front thickness after Wall thickness), the Slits strips (one stop each), the slit fields, the Base
fields and switch, the Lid switches and fields (in Curved only the hollow), Gap, then the Style sections. A switch is one stop, Space toggles it; rows it shows appear after it in the order, rows
it hides leave the order. Every control has its visible label; following and automatic values say so in
`aria-valuetext`. Tokens only; nothing animates.

## 12. For the product owner and the architect

Resolved since revision 2:

1. **Slit width default:** kerf + 0.3 mm (ADR 0082 §7, spec criterion 4).
2. **Base and lid joints:** tab and slot (ADR 0082 §5); no joint control; the lid is the Lid and Lid
   seat switches (spec criteria 5, 9, 13).
3. **Cover and Lid thickness fields:** own fields, following Wall thickness (Frage 355 C, ADR 0082 §5).
4. **Ring width:** derived 2 x Ring thickness, no field (ADR 0082 §8).
5. **Base inset:** in the spec (criterion 10).
6. **Thickness follows:** automatic Lengths, a stored 0 (ADR 0082 §5, §7).
7. **Tabbed lid thickness:** Lid thickness (ADR 0082 §5).
8. **Slit rows shown when Offset:** `ShownWhen::Is` (ADR 0082 §8).

Questions (the lead numbers them; options, recommendation, default):

9. **Resize of a full-round box** (criterion 7 keeps the corner radius on a resize). A: as the spec: the
   radius stays, so a larger full-round box becomes a rounded rectangle and a smaller one stays full
   round by the clamp. B: a box at full round stays full round on a resize (the radius scales with it).
   Recommendation and default A: the spec's rule, one rule for every radius; the maker drags the radius
   back to the end of its track.
10. **Label "Wall thickness"** instead of revision 2's "Thickness" (section 3): the product owner confirms
    against criterion 2, which already says "Wall thickness (strip)".

Slice 2, the curved top (revision 5 listed the differences between spec revision 7, ADR 0082 revision 6
and these notes; revision 6 marks what spec revision 8 and the amended ADR 0082 §11 settle):

11. **Part order: resolved.** 1 strip, 2 and 3 end plates, 4 front, 5 base, in numbering and layout
    order (spec criteria 16 and 23, ADR 0082 §11 amended; part keys `strip`, `left`, `right`, `front`,
    `base` for slice 3).
12. **Thickness fields: resolved.** ADR 0082 §11 amended (Frage 355 C): `end_plate_thickness` and
    `front_thickness`, automatic Lengths following `thickness` (stored 0), shown with `Is { top_shape,
    curved }`; the base on `plate_thickness`, label "Inner base thickness"; W = w + 2 t_e, D = d + t_f +
    t_w, H = h + t_b + t_w, finger width 2 x the thinner part; no format change.
13. **Finger hollow place: resolved.** In the front's top edge, centred, under the lid's free edge
    (spec criterion 21 revision 8, ADR 0082 §11; 10.4).
14. **Corner radius bounds: resolved.** r ≥ 2 t_w (and the two-column minimum), r ≤ d - 2 t_w, r ≤ h -
    2 t_e - 4 t_w; Inner height ≥ 2 t_e + 6 t_w (spec criterion 20, ADR 0082 §11 amended). At the
    defaults 6 to 32 mm, least height 24 mm (10.10). For the product owner: criterion 20's example
    "set to 38 mm" is not the default box's number; a value at the defaults (32 mm) would read better.
15. **Clamp wording: resolved.** Spec criterion 20 uses section 5's one form.
16. **Undo step: resolved.** "Rounded box: Top shape" (spec criterion 11).
17. **Marks (open, for the architect).** ADR 0082 §11 does not settle them. The rounded box emits `Marks::default()` today; the bend band of 10.9 needs a `Mark::Bend`
    per end-plate arc from `rounded_box/curved_top.rs` (run time, never `tight`), for the architect.
    The badges of 10.8 need the part centres and numbers (as `0149`'s wall pieces).
18. **Revision 4 text withdrawn:** section 10's "Wall slits Corners only means the back arc only" (Wall
    slits is hidden in Curved, spec revision 7) and "the Lid group is hidden" (the hollow stays under
    the Lid heading).

To go into `docs/design-system.md` when Done: the Rounded box segment and glyph, the automatic Value
field state for schema fields (with "follows <field>"), the schema-drawn switch row with shown-when rows
under it, the ghost without holes except a ring opening, "full round" in the radius readout; with slice
2 the part badges on a generated box and the bend band on a profile plate.
