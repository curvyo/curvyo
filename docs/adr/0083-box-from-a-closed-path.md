# ADR 0083: A box from any closed path

**Status:** Proposed by the architect 2026-11-24, revision 2 the same day with the lead's
decisions on spec `0149` revision 3 and its UX notes revision 1 (all Proposal, vetoable at the
demo). Not `needs-customer` as a whole: no new crate,
no `format_version` (a new generator kind is additive, ADR 0080 §5; the path is a parameter
value); one new crate edge (`curvyo-generators-core → curvyo-geometry-core`, allowed by ADR 0034
§2 and ADR 0080 §3 once a generator needs booleans); one source-breaking `0.x` contract change
(§2, §11). Revision 2 changes: the object is named "Path box", the command "Box from path" (§1,
§2); the layout lies right of the source, in the source's layer (§2); a straight default outline
and outline checks that never swap the maker's outline on a thickness or scale edit (§2); slice 1
builds kinks as bent spans (§3) and a curved cubic only from a newer file, as its chord (§2); the automatic
spacing reports its smallest gap (§4); notices name wall pieces and travel as structured warnings,
and the generator returns canvas marks (§4, §5, §7, §11); a seam on a straight loop (§6). The
customer questions are the spec's: Frage 364 (copy, decided here as A), 365 (the source
stays), 346 (Corner angle default 25°), 347 (answered by §4: no curve pattern is needed). Spec:
[`0149`](../../specs/0149-path-box/specification.md) (Ready for slice 1, straight outlines; slice 2 curves). Per-part operations
(Detach part) are ADR 0082 §10. Builds on [ADR 0082](0082-generator-plate-joints-corner-rule-and-rounded-box.md)
§1, §2, §4 to §7 and [ADR 0080](0080-generated-objects-extension-trait-and-parametric-instances.md)
§2, §4 to §6; their decision text is unchanged.

**In plain words (for the customer).** Draw any closed shape (a heart, a shield, a coffin), pick
"Box from path", and Curvyo builds a box with that shape as its floor, laid out flat to the right
of your drawing. Where the outline has a
real corner, the wall pieces meet in a finger joint; where it curves, the wall bends by slits.
The base, ring, lid and cover are the rounded box's, cut to your shape. The box keeps a copy of
your outline, so it stays a live object: change the height, the thickness or the slits and it is
rebuilt. Very tight curves are built anyway, with a notice that the wood may crack there. The
path is the inside of the wall, as with the rounded box.

## Context

- The rounded box (ADR 0082) is a wall strip round a footprint of lines and arcs, built by the
  crate-level `footprint.rs`, `wall.rs` and `wall_plates.rs`. A drawn path is cubic Béziers with
  corners of any angle, curves of any radius and concave parts.
- A generator is a pure function of its stored parameters (ADR 0080 §2). Document-core cannot
  call a generator (ADR 0080 Context), so nothing in the document can rerun one when another
  object changes.
- `contour.rs` offsets lines and arcs exactly (ADR 0082 §2); a cubic has no exact offset.
  `curvyo-geometry-core` has a deterministic boolean offset (`offset.rs`, spec `0038`) that
  handles collapse and self-overlap on any outline.
- A finger joint at a corner that is not a right angle cannot be flush on both faces with square
  laser cuts.

## Options

**How the box holds the path:**
- **A. A copy of the outline as a parameter value (chosen).** A new parameter kind `Path`; the
  copy lives in `generated.params` like every other value. The generator stays a pure function,
  the record stays one atomic value, copies and clones work, an older build keeps it as an
  unknown kind. Editing the drawn path afterwards does not change the box; "Replace outline"
  takes a new one (§2).
- **B. A live link to the document object.** The box would have to be regenerated when another
  object changes. That is a host-side dependency pass writing a second object in the same commit,
  a merge of one peer's path edit with another peer's stale box, a kept copy when the source is
  deleted (as ADR 0022 for text carriers), and a new reference register with a `format_version`
  bump. Rejected now; the copy can become a link later by the same register route as ADR 0022.
- **C. Turn the path node itself into the box.** Loses the drawing and gains nothing over A
  (spec Frage 365 A: the source stays untouched).

**How the geometry is computed:**
- **A. Fit the outline with lines and arcs once, then reuse the rounded box's modules (chosen).**
  Kerf, neutral-line lengths, tab places and slot sides stay exact (ADR 0082 §2); the only
  approximation is the fit, with an explicit tolerance. The offsets that can collapse (cover,
  ring opening) go through geometry-core.
- **B. Work on cubics throughout** (`path_measure` for positions, the boolean offset for every
  kerf). Every plate runs through the flattened boolean engine on every edit, tab positions are
  numeric, and the rounded box's code does not apply. Rejected.
- **C. Flatten to polylines.** Faceted plates, large outlines, slits laid on facets. Rejected.

**Where it lives:** **A. module `path_box/` in `curvyo-generators-core` (chosen)**, a fourth
kind beside the rounded box; B. a mode of the rounded box (its size fields, radius and handles do
not apply to a path; two kinds sharing modules is simpler); C. its own crate (same extension,
nothing gained).

## Decision

1. **Kind** `path-box` (stored `curvyo.generators/path-box`), label **"Path box"** (the object's
   name: Layers, "2 path boxes", the steps "Add path box" and "Convert path box to paths", the
   status bar), icon `Spline`, command **"Box from path"** (the segment and the Select bar
   button, §2 `source`). Revision 2, lead decision on the UX notes §4 (accepted): every other
   kind's label is a noun and the history steps are built from it, so "Box from path" as the
   label would give "2 box from paths". Module
   `path_box/`: `mod.rs` (values, clamps, layout), `schema.rs`, `classify.rs` (outline to
   footprint), `arc_fit.rs` (cubics to lines and arcs). It uses ADR 0082's `fingers.rs`,
   `contour.rs`, `slit_bend.rs`, `footprint.rs`, `wall.rs` and `wall_plates.rs`.
2. **Contract (source-breaking 0.x in `curvyo-extension-core`, CHANGELOG, TS schema reader).**
   - `ParamKind::Path` and `ParamValue::Path(ClosedPath)`, stored as the one-key map `{"path":
     [x0, y0, c1x, c1y, c2x, c2y, x, y, ...]}`: one closed subpath, frame-local mm, the encoding of
     a stored subpath (ADR 0080 §4). The block draws no field for it (UX: a row "Outline" with
     "Replace from selection", later per spec `0149`).
   - `GeneratorKind` gains `source: Option<Source>`, `Source { param: &'static str, command:
     &'static str }`: the key of a `Path` parameter filled from the selection, and the text of
     the segment and the Select bar button (`"outline"`, `"Box from path"`). `label` stays the
     object's name; its doc comment drops "and of the bar segment". The command text lives in
     `Source` because only a kind built from a selection has a verb (no free optional field).
     The registry orders segments by the text they show (command, else label), so the bar reads
     Box, Box from path, Gear, Rounded box (UX notes §1.2). A kind with a source builds only from
     exactly one closed outline (a path or a shape, after its transform, with one closed
     subpath); otherwise "Select one closed outline. Nothing was changed." (spec criterion 1).
     The segment is a pick mode and the Select bar has the button (UX notes §1).
   - When `ParamValue` moves to document-core with `0099` (ADR 0080 §2), `ClosedPath` moves with
     the `Path` arm.
   - Create: one commit `add_generated` ("Add path box"); `outline` is the selected outline in
     page mm, moved so its bounding-box centre is the origin; the source is not touched (spec
     Frage 365 A). **Placement (revision 2, lead decision, Proposal; spec criterion 1, UX
     question 2 B):** `generated_frame` is a pure translation that puts the flat layout to the
     right of the source: with S the source's geometric bounding box in page mm (its outline
     after its transform, stroke not counted) and L the generated parts' bounding box in the
     frame, the translation is `(S.max_x + gap - L.min_x, S.min_y - L.min_y)`, `gap` the box's
     own value. So the layout starts Gap right of the source, top edges aligned, and the source
     stays clickable. This holds at create only; later edits regenerate about the frame origin
     as for every generator. **Layer (revision 2, UX notes §4, accepted):** the box is written
     directly above the source in the source's parent (`drawing_above.rs`), not on top of the
     active layer as `0141`'s placed generators. A derived object follows its source, as clones,
     trace results and Stroke to path already do; the source was just selected or picked, so its
     layer is visible and unlocked. Replace outline: `Edit::Param("outline")` with the new copy,
     one "Change parameter" commit. `Edit::Scale`: the outline scales about the origin together
     with `inner_height`; thicknesses, kerf and slit values stay (spec criterion 8), and the
     factor stops at the size check below.
   - **Checks in two places** (extension-core has no geometry-core edge, and `Param::accept` sees
     one value). `Param::accept` checks the shape only: `2 + 6n` numbers, at most 2,000 cubics,
     every number finite within ±1e7; a value that fails reads as the default outline (a coffin
     of 100 x 60 mm, six straight edges, Proposal; revision 2: a heart cannot be built in slice 1),
     shown with a notice, never rewritten on read (ADR 0033 §3). The generator's dependent step
     in `path_box/` (one function, `path_box::check`) then normalises (clockwise on screen,
     starting at the first corner of §3 if any) and tests, in this order:
     - **No self-intersection** (geometry-core `curve_intersections`): a crossing outline is
       structurally unusable, so it is replaced by the default outline with the warning
       `outline_crosses` (§11). Only damage brings it here, since create refuses it.
     - **Size:** an area of at least (4 t_w)². This never replaces the outline. On create
       (`edit` `None`) and a thickness edit the wall thickness is clamped (Dependent, t_w ≤
       √area / 4, permit over refuse as `0147`'s knock-on clamps). On `Edit::Scale` the
       thickness stays (it is the sheet the maker has) and the factor is limited so the area
       keeps (4 t_w)², with the warning `smallest_size` (§11; a `Clamp` needs a parameter key,
       and no key is the scale).
     - **Curves in slice 1:** a cubic outside the 0.1 mm line test of §3 is built as its chord
       (so its ends become kinks or corners) with the warning `outline_curved`. The stored
       outline stays as it is, so a build with slice 2 rebuilds it exactly. Only a file from a
       newer build brings it here, since create refuses it. Slice 2 removes this rule.
     ui-core does not test the outline itself. Before create (and for each object the pick mode
     hovers that passes the cheap Select bar test of one closed subpath, cached until the object
     changes) it runs `generate` with the candidate outline and
     the default values and refuses, with nothing built, when the result holds the warning
     `outline_crosses` ("The outline crosses itself."), a Clamp on `thickness` ("The outline is
     too small for a box with 3 mm walls.") or, in slice 1, the warning `outline_curved` ("This
     outline has curves. Box from path builds straight outlines only so far."), each ending
     "Nothing was changed.". So the 0.1 mm tolerance and the tests live once, in `path_box/`.
3. **Classification (`classify.rs`), in this order:**
   - **Arc fit.** A cubic that stays within 0.1 mm of its chord is a line (spec `0149` criterion
     3). Every other cubic becomes arcs within 0.01 mm (biarcs, split at the middle until within
     tolerance); joins inside one cubic are tangent by construction. Slice 1 (straight outlines)
     needs only the line test; `arc_fit.rs` arrives with slice 2 (§2 for a curve in slice 1).
   - **Junctions.** At each join, the turn angle δ between the incoming and outgoing tangents.
     |δ| ≤ 1° is tangent (fitting noise). 1° < |δ| ≤ `corner_angle` is a **kink**: filleted on
     the inner face with radius 2 t_w, so it becomes part of a bent span (Proposal; the outline
     moves by at most 2 t_w (1/cos(δ/2) - 1) there, 0.07 t_w at 30°). |δ| > `corner_angle` is a
     **corner**, convex or concave. **Kinks are slice 1** (revision 2, lead decision, Proposal;
     UX question 1 A): a fillet is an exact arc between two lines, so it needs no arc fit, and
     the bent span it makes is the rounded box's (slits, flat windows, the seam of §6). A
     straight too short for the fillets at both its ends (each takes 2 t_w tan(|δ|/2)) gives
     both fillets the radius whose tangent points meet in its middle; the straight is dropped
     and the two arcs join into one bent span (ADR 0082 §5, degenerate straights). Such a
     radius may fall below the bending limit of §4, which then warns.
   - **Spans.** Between corners, consecutive arcs and fillets form one bent span; a line is a
     straight. The result is a `Footprint` (ADR 0082 §4) with corners marked.
   - `corner_angle`: an Angle parameter, default 25°, 5° to 90° (spec Frage 366 A).
4. **One rule per span:**

   | Span | Wall | Plates |
   |---|---|---|
   | Corner (turn above `corner_angle`) | the strip ends; a finger joint to the next piece (5) | no tab within the joint zone plus t_w |
   | Straight | solid (text or a hole may go on it) | tabs (ADR 0082 §5) |
   | Bent span, any radius, kinks included | always slit-bent | tabs only in flat windows, when the straights hold fewer than 4 (ADR 0082 §5) |

   Slits are laid on the developed, flat strip piece with ADR 0082 §6's straight `Band`, so no
   pattern along a curve is needed (spec Frage 367 is moot; `0148` alone needs the curved spine).
   Automatic spacing follows the local radius (spec criterion 6): from the start of the band,
   each next column lies 0.22 r further, r the inner radius at the current column, at least slit
   width + 1 mm, as long as the column still fits before the band's end; then all gaps are
   scaled by one factor (at least 1, since only columns that fit were placed) so the last column
   lies as far from the band's end as the first from its start. So no gap falls below the
   minimum. A typed spacing applies everywhere. **Reported value** (revision 2, UX notes §5,
   accepted): `Generated.automatic` carries one value per key, so `slit_spacing` reports the
   smallest gap after the scaling, over every bent span of every piece: the gap the minimum
   clamp guards and the place a crack starts, so a typed value never lies denser than the
   densest automatic place. With no bent span (a coffin at 25°) it reports the gap a kink would
   get, 0.22 x 2 t_w, at least slit width + 1 mm; the field is shown in slice 1 either way.
   Wall slits is not offered: straights are solid, bent spans always slit. A radius below 2 t_w
   (convex), or a concave radius below t_w (the outer face collapses), is built at the minimum
   spacing with the warning `tight_bend` naming the piece, the radius and that limit (§11; "Piece
   2 bends to 2.5 mm; below 6 mm the wall may crack.", UX notes §6); never refused.
5. **Corner joints** (`wall.rs`, `fingers.rs`; ADR 0082 §1):
   - The joint zone is the parallelogram where the two wall bands overlap. Along each wall's inner
     face it is z = t_w / sin|δ| long (t_w at 90°); `finger_count` splits the strip height into
     an odd number of pieces as on the Box.
   - **Flat length of a piece end.** At a convex corner the zone lies beyond the inner corner, so
     the piece extends z past it. At a concave corner (the heart's top) the zone lies inside both
     walls' own lengths, so the piece stops at the inner corner and the zone is its last z. A
     concave end extended by z would push into the box.
   - Finger tips are cut square at the middle of the zone's slanted end, so a tip lies off the
     mating outer face by at most t_w |cot δ| / 2: 0 at 90°, 0.29 t_w at 60° or 120°, 0.87 t_w at
     30°. Convex and concave corners follow the same rule with |δ|.
   - **Owner:** at each corner, the piece that arrives there in the clockwise walk from the
     outline's start owns the end pieces at the top and bottom edges; the next piece has notches
     there. With one corner, the one piece's end owns and its start has notches.
   - z + t_w is the solid margin between a corner and the first slit column (2 t_w at 90°, spec
     criterion 6).
   - |δ| above 150° (an interior angle below 30°, spec criterion 5): z is capped at 2 t_w and the
     warning `sharp_corner` names the two pieces and the interior angle (§11; "The corner between
     pieces 3 and 4 is very sharp: 22°.", UX notes §6); built anyway.
   - On a bent span next to a corner the zone follows the tangent at the corner. The plate meets
     the wall's inner face, which is outside every zone.
6. **Seam.** Only an outline with no corner has one, at the middle of the back (spec criterion
   9): the outline's point with the least screen y (ties: least x), with tabs laid out half a
   pitch either side of it (ADR 0082 §5). When the least y is held along a straight (a polygon
   whose kinks left no corner, its top edge level), the seam is that straight's middle and the
   straight takes the next even tab count, as the rounded box's back straight (revision 2). With
   corners every piece runs corner to corner and the joints hold it.
7. **Plates** (`wall_plates.rs`). Inner base, tabbed lid and the ring's outline are the footprint
   with tabs, through `contour.rs` (exact; chords at windows, miter joins at corners, ADR 0082 §2).
   The outer footprint (cover, seat lid) is the footprint offset outward by t_w + k/2 with Miter
   joins, so it covers the walls' outer corners; the ring opening is the footprint offset inward
   by 2 t_r + k/2 with Round joins. Both through geometry-core `offset`; its cubic result is the
   part's `ClosedPath` (or hole) directly. An empty ring opening leaves the ring as a full plate
   with the warning `ring_closed`. The finger hollow sits on the lid's longest straight that holds
   the width plus 2 t_w, else centred on its longest bent span whose smallest radius is at least
   the hollow width, else it is left out with the warning `hollow_left_out` (§11).
8. **Parameters** (keys are file format; Proposal): `outline` (Path), `inner_height`,
   `corner_angle`, then the rounded box's groups and keys without its sizes, corner radius and
   `wall_slits` (thicknesses, `kerf`, plates, slits, `gap`; ADR 0082 §8), built by one shared
   schema function. The outline is the inner face of the wall (spec criterion 10: cover and lid
   are the outline offset by the wall thickness). **Layout:** the strip pieces in the first row
   in walk order; the plates in the second row in thickness runs (ADR 0082 §5). Part keys
   `wall-1` to `wall-n` and the plate keys (ADR 0082 §10), so Detach part works as on the rounded
   box once `0147` slice 3 brings it; until then the pieces are numbered in the marks (§11) and
   nothing is stored per part. Wall keys are positions, so a change of the set of wall pieces (Replace outline, or a
   `corner_angle` change that merges or splits pieces) removes every `#wall-*` key in the same
   commit, with the notice "Detached wall pieces were attached again: the pieces changed". Plate
   keys keep their meaning and stay. No size handles; a proportional resize scales (§2).
9. **Format:** no `format_version` bump. The kind is additive (ADR 0080 §5); the path is a value
   inside `generated.params`, which document-core keeps opaque (checked: `generated_codec.rs`
   refuses only a `params` that is not a map, so `{"path": [...]}` opens in every build; its doc
   comment gains the `path` form); the parts are ordinary parts. Revision 2 changes nothing here:
   placement, layer, warnings and marks are not stored. A
   build without the kind draws and cuts the stored outline. When the parameter codec moves to
   document-core with `0099` (ADR 0080 §2), the `Path` arm moves with it; sketch clones never hold
   one. Each committed edit stores the path once more (a 40-cubic heart: 242 numbers, about 2 KB).
10. **Tests.** Arc fit against the cubic within 0.01 mm (sampled; explicit tolerance).
    Classification goldens: a heart (two corners: the bottom convex, the top concave; two bent
    spans), a shield, a coffin (corners only), a circle drawn as four cubics (no corner, one bent
    span). Corner joints at 60°, 90°, 120°, 135° and a concave 240° interior: the two pieces' fingers
    alternate, one owner per corner, tips within t_w |cot δ| / 2 of the outer face. Per bent span
    the uncut-band, slot-inside and spacing tests of ADR 0082 §9. The cover equals the outward
    offset within the kerf. The same outline rotated by 90° gives congruent parts. Golden
    `path_box_default.txt`. Revision 2: a 20-gon at 25° (kinks only: one strip, the seam in the
    top straight's middle, tabs in windows if the straights hold fewer than 4); a straight too
    short for its two fillets; a stored curved cubic in slice 1 built as its chord with
    `outline_curved` and the stored outline unchanged; a thickness raised past √area / 4 is
    clamped and the outline kept; a scale-down below the minimum keeps the thickness and stops
    at the smallest outline with `smallest_size`; the placement translation of §2 (layout left edge Gap right of
    the source, tops equal) on a rotated source; the reported `slit_spacing` equals the smallest
    gap. CPU (spec criterion 11): the coffin at most 100 ms and the heart with
    at least 400 slots at most 300 ms of thread CPU for the edit with its commit.

11. **Warnings and canvas marks (revision 2; contract, run time only, never stored).** The UX
    notes need generator knowledge the result does not carry: `Clamp` names one parameter, and
    the canvas needs the classification.
    - **Warnings.** `Generated` gains `warnings: Vec<Warning>`, `Warning { id: &'static str,
      pieces: Vec<u32>, value: Option<ParamValue>, limit: Option<ParamValue> }`. The shell
      writes the sentence from it, as it does for a `Clamp`, because the shell formats lengths
      in the document's units and merges warnings of one id into the plural sentence ("2 corners
      are very sharp: ..."). Ids of the path box: `sharp_corner` (the two pieces, the interior
      angle), `tight_bend` (the piece, its smallest radius, the limit 2 t_w or t_w), `ring_closed`,
      `hollow_left_out`, `smallest_size`, `outline_crosses`, `outline_curved` (slice 1). With one wall piece the
      text says "The wall" instead of "Piece 1" (no badges then, UX notes §3). Rejected:
      sentences from the generator (`Vec<String>`): core would format units it does not know.
      Placement in the UI (canvas notice slot after a build, under the field after an edit) is
      the UX notes' §6.
    - **Marks.** `Generated` gains `marks: Marks { footprint: Vec2, on_footprint: Vec<Mark>,
      on_parts: Vec<Mark> }`. `on_footprint` is in the stored outline's coordinates: `Corner
      { at, bisector, sharp }`, `Bend { start, segments: Vec<Cubic>, tight }` (one per bent span,
      kinks included; the stretch of the inner face, the band drawn on its inside, the right of
      the clockwise walk) and `Piece { number, at, inward, length }` (the middle of the stretch a
      piece stands on). `footprint` is the inner base's offset in the layout, so the shell draws
      `on_footprint` on the selected box's inner base and, unshifted, on the hovered source in
      the pick mode. `on_parts` holds the badge `Piece { number, at, .. }` at each wall piece's
      middle in the frame. The Corner angle value text counts the `Corner` marks.
    - Options for the hover: **A. `generate` on the candidate with the default values, cached
      per object (chosen)**: one route through the registry, and the same call gives the refusal
      of §2. B. A classify-only trait method: a second entry for one kind, before a measurement
      asks for it. C. ui-core calling `path_box` directly: bypasses the registry (ADR 0034).
      Selecting a stored box runs one `generate` for its marks (the stored outline is drawn
      without one), memoised with the instance (ADR 0080).
    - No `format_version`: `Generated` is run time; the record still holds kind, seed, params and
      parts only.

## Consequences

- One shape tool covers any footprint; the rounded box stays the quick one with size fields and
  handles. Fixes to `footprint.rs`, `wall.rs` or `wall_plates.rs` serve both.
- Off-right-angle finger joints are not flush; the notice and the bound in §5 say how much.
- The box does not follow later edits of the drawn path (Option B rejected); "Replace outline"
  does it in one step.
- `curvyo-generators-core` gains the geometry-core edge (boolean offset, intersection test). It is
  in CI's wasm32 list already.
- Very long strip pieces are not split across sheets; that is a later entry if a spec asks.
- Hovering an outline in the pick mode costs one generation per object (at most 100 ms for the
  coffin). If slice 2's heart makes the hover lag, a classify-only request (option B of §11) is
  the next step.
- `Generated` grows by `warnings` and `marks`; the other kinds return them empty.

## Customer questions

None of its own. The spec's Frage 364 (PATH-SOURCE, copy or link) is decided A by Option A above;
B is the later link (a format change). Frage 365 (the source stays) and 366 (Corner angle 25°) are
product choices this ADR follows at their defaults. Frage 367 (where the curve pattern is built)
falls away: the wall is slit on the flat developed strip (§4).
