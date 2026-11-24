# 0147 Rounded box: UX notes

ux-engineer, 2026-11-24, revision 4 (findings of the UX browser review of slice 1). Checked against
`specification.md` revision 4 (criteria 1 to 13 for slice 1, criterion 15 for slice 3; Fragen 349 B, 355 C,
360 B, 361 A, 362 B answered, 348, 350 to 353 at A, 363 at its default A), ADR 0082 revision 2 (§5 parts,
thicknesses, tabs and flat windows, clamps, layout; §6 Wave; §7 automatic values; §8 parameters; §10
Detach part), `adrs.md`, the exploded drawings 34 (stadium) and 35 (rounded square),
`specs/0141-parametric-generators/ux-notes.md` (sections 1 to 15; the Rounded box is a further generator
of the same contract, so everything not named here is as there) and `docs/design-system.md` (rows "Value
field", "Parameter handle", "Parameter handle layout", "Corner radius readout and chip texts", "Drag
ghost", "Live preview outline", "Hidden, not disabled").

Customer rules applied: no popups in the panel, no operating tips or hint labels, no notice after a
success, hidden not disabled. No new colour token, no new component. Everything here that the spec, ADR
0082 or a decided Frage does not fix (group order, labels, tracks, knob positions, clamp texts, the
automatic state, the hollow defaults, the ghost, where Detach part sits) is **(Proposal)**, vetoable at
the demo.

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
| | Top shape | choice strip **Flat / Curved** (Frage 353 A) | **slice 2 only**: not drawn in slice 1 (no single-cell or disabled strip) |
| Material | Wall thickness, Kerf compensation | length | always |
| Slits | Slit pattern | choice strip **Straight / Offset / Wave** (Frage 349 B), three cells of 80, 80, 84px | always |
| | Wall slits | choice strip **Corners only / All round** (criterion 3), 122px cells | always (also on a circle, where both give the same strip; see below) |
| | Slit width, Slit spacing | length, automatic by default | always |
| | Slit rows | count, automatic by default | Slit pattern **Offset** only (ADR 0082 §8 `ShownWhen::Is`; Straight and Wave have one slot per column) |
| Base | Inner base thickness | length, follows Wall thickness | always |
| | Base inset | length, default 0 (criterion 10) | always |
| | Base cover | switch, on (criterion 10) | always |
| | Cover thickness | length, follows Wall thickness | Base cover on |
| Lid | Lid | switch, on | always |
| | Lid thickness | length, follows Wall thickness | Lid on |
| | Lid seat | switch, on (criterion 12) | Lid on |
| | Ring thickness | length, follows Wall thickness | Lid on and Lid seat on |
| | Finger hollow | switch, on (criterion 13) | Lid on and Lid seat on |
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
  shows every part with its slits.

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

## 10. Top shape Curved (criterion 11, slice 2)

Slice 1 builds no part of this; the Top shape row is not drawn (section 3). For slice 2 **(Proposal, to
confirm when slice 2 is Ready)**, following ADR 0082 §5 "Curved top":

- Top shape Curved: the strip runs over the top and round the back arc; Corner radius is the radius of
  that arc (Frage 353 A, one radius field, label unchanged); the footprint corners are square.
- Rows hidden while Curved: the Lid group (the strip is the lid). Base, Material, Slits and Layout stay;
  Wall slits Corners only means the back arc only.
- The Corner radius knob moves to the end plate's back-top corner; the ghost and the layout follow
  sections 2 and 8 with the end plates and front in the second row.

## 11. Accessibility

Tab order is screen order: the Size fields (and Top shape in slice 2), Material, the two Slits strips
(one stop each), the slit fields, the Base fields and switch, the Lid switches and fields, Gap, then the
Style sections. A switch is one stop, Space toggles it; rows it shows appear after it in the order, rows
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

To go into `docs/design-system.md` when Done: the Rounded box segment and glyph, the automatic Value
field state for schema fields (with "follows <field>"), the schema-drawn switch row with shown-when rows
under it, the ghost without holes except a ring opening, "full round" in the radius readout.
