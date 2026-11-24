# ADR 0082: Shared plate joints in the generators, the corner rule and the rounded box

**Status:** Proposed by the architect 2026-11-24, revision 2 the same day with the customer's
answers (Frage 349 B, 355 C, 360 B, 361 A, 362 B; spec `0147` revision 3), revision 3 the same
day with the lead's decisions on the UX notes revision 3 (spec `0147` revision 5), revision 4 the
same day from the architect review of `story/generators-flex-box` (§6 at least two columns per
bend, §7 `Param.track`), revision 5 the same day from the second review of that branch (§7
`Edit::Draft`, `Generated.states`, `Bound::DependentMinimumFor`). **§10 is
`needs-customer`** (file format: per-part operations change `format_version`; Frage 363
ROUND-DETACH of spec `0147`, default A: not in slice 1, built as slice 3). The rest needs no customer decision: no new crate or
dependency, and the new kind is additive (ADR 0080 §5); one source-breaking `0.x` contract change
(§7 and §10, as ADR 0080 §2's additions). Revision 2
changes: Wave slits and the pattern as an open set of kinds (§6, Frage 349 B); a thickness field
per part (§5, §8, Frage 355 C); an exact circle, with tabs on curved spans in flat windows (§1,
§5, Frage 360 B); miter corners in `contour.rs` (§2); the footprint and wall modules lifted to
crate level for [ADR 0083](0083-box-from-a-closed-path.md) (§4); live per-part operations with
Detach part (§10, Frage 362 B, spec `0147` criterion 15). Revision 3 changes: a resize keeps the
corner radius, also at full round (§5); the Corner radius knob listed with the size handles (§5);
"follows wall thickness" (§5); Detach part writes a copy of the box's operation and is slice 3
(§10). Specs:
[`0141`](../../specs/0141-parametric-generators/specification.md) revision 2 (corner fix, Joint
choice gone), [`0147`](../../specs/0147-rounded-box/specification.md) (rounded box, with the
customer's additions and exploded drawings of 2026-11-24), `0148` `slit-bend-pattern` (idea, later).
Builds on [ADR 0080](0080-generated-objects-extension-trait-and-parametric-instances.md) §2, §3,
§5, §6; its decision text is unchanged.

**In plain words (for the customer).** The box floor lost its corners because each corner square
touched the floor at a single point. Now the floor and the lid own the corner square together with
the finger next to it, so the corner is one solid tab at kerf 0. The rounded box is a third
generator built from the parts of your drawings: one slit wall strip, an inner base plate with tabs
in the wall's slots (it can sit higher than the bottom edge), a plain cover plate under it, a frame
ring with tabs as the lid seat, and the lid with a finger hollow. Every part has its own thickness,
which follows the wall thickness until you type one. The corner radius goes up to half the shorter
side, so a square box with full corners is an exact circle; where there is no straight side, a tab
sits on a short flat spot of the curve. Slits come as Straight, Offset or Wave, and more patterns
can be added one at a time. All parts cut with the box's one operation. In a later slice, Detach
part gives one part its own operation while it stays part of the live box; for that the file
format changes once, so older Curvyo versions will not open such a file.

## Context

- **The corner bug (0141 revision 2).** `boxes/mod.rs` `Sizes::horizontal()` gives the bottom and
  the finger lid `Corner::Owned` on all four corners and edge strips `even_solid: false`. With an
  odd piece count, pieces 0 and n-1 are notches, so the owned t x t corner square has a notch on
  both sides and touches the plate only at its inner vertex: `tests/fixtures/box_default.txt`
  part 0 visits (-252, -40) twice, before and after the square. A raised kerf hides it only
  because `plate.rs` `offset()` turns that vertex into a neck about one kerf wide. The mating test
  passed because it compares step positions only.
- **Joint choice removed** (customer 2026-11-24): every box is finger-jointed.
- **Rounded box** (spec `0147`): a separate generator in the same extension; rounded corners or a
  curved top that bend by slits; the exploded drawings (stadium, rounded square) fix the part set.
- The contract carries closed outlines only (`Part { paths: Vec<ClosedPath> }`; stored subpaths of
  `2 + 6n` numbers, closed by definition). The gear already cuts holes as reversed subpaths of one
  part (`gear/web.rs`).
- **Physics of slit bending.** A strip of thickness t bent to radius R strains its faces by
  t / 2R: 10 % at t = 3, R = 15, far beyond wood. Bending works only because the ribs between slit
  columns twist. Any band of material that runs uncut across a bend (for example a solid edge
  margin) has to take the full strain and cracks. The converse: a stretch of strip with no slit
  column stays flat, so a plate edge that meets it must be straight there too.
- **Operations** (ADR 0046 §7, ADR 0056 §1): a leaf runs with the operations of the nearest key on
  its chain that has any membership. A generated object is one leaf (ADR 0080 §4).

## Options

**Slits** (the deciding point for spec `0147` criterion 8):
- **A. Narrow closed slots, holes of the strip part, and narrow notches where a slot runs out
  through an edge (chosen; Frage 361 A).** No contract or format change. Render, hit test, SVG
  export and the laser job already read compound parts with holes (the gear). Cost: a slot of
  length l and width w is cut along 2(l + w), so slit cutting takes about twice as long as with
  single lines.
- **B. Single open cut lines.** Faster to cut. But `Part` needs open paths and the stored record
  needs a new marker. A build without it would read an open subpath as closed, or drop it, and cut
  a different part. So the file needs a `format_version` bump and every outline consumer changes.
  Rejected (Frage 361 A).

**Where the rounded box lives:** **A. module `rounded_box/` in `curvyo-generators-core` (chosen)**;
B. a mode of the Box (rejected by the customer); C. its own crate (same extension and edges,
nothing gained).

**Shared joint code:** **A. move the finger split, the plate walker and the kerf offset to crate
level for both boxes (chosen)**; B. copy them (two copies of the corner rule, the rule that just
broke); C. a `Joint` trait (§5: finger, tab and slot are runs of one split, not types).

**Automatic slit values:** **A. a stored 0 means automatic, the schema marks such parameters and
the generator reports the values it used (chosen; the bore = 0 and Line spacing Auto
precedents)**; B. the generator rewrites the stored value when a driving value changes
(`Edit::Param`), so an own value is lost on the next radius change and the stored number hides
whether it was typed; C. an Auto/Custom choice row per value.

**Tabs where there is no straight** (Frage 360 B):
- **A. A flat window: the tab's stretch of the curve becomes a short chord on the plate, and the
  strip has no slit column there (chosen).** Mating is exact, because the strip really is flat
  where it has no slit. The plate leaves the true curve by the chord's sag, at most l²/8r for a
  window of length l on radius r (below).
- **B. The tab stays on the arc, with radial sides.** The strip is flat across the slot but the
  plate is curved, so the two disagree by the same sag and the gap is hidden in the joint.
  Rejected: an untested gap.
- **C. No tabs on curves; a glued butt joint.** Rejected: the customer asked for a circle that
  assembles like the other boxes.

**Per-part operations** (Frage 362 B; spec `0147` criterion 15: by default all parts share one
operation; Detach part gives one part its own and it "still belongs together": it regenerates,
moves with the box and keeps its layout place):
- **A. Convert to paths, then assign the one part.** ADR 0080 §6 already turns the box into a
  group of plate paths with the operation on the group; ADR 0056 §1 and the nearest rule let one
  plate take its own. No format change, but no part is live any more. This is spec Frage 363 B;
  it stays the fallback.
- **B. Detach one part as a path and keep the rest live.** The record needs a list of omitted
  parts; the detached part goes stale on the next edit and no longer mates. Rejected.
- **C. Live per-part membership keys (chosen, `needs-customer`).** `Part` gains a stable key, the
  record stores the keys, and an operation may hold `member.<node>#<part>`. The nearest rule takes
  one more step. One record, one leaf, nothing else in the document model changes. A build
  without it would read the key as a dangling or legacy key (ADR 0056 §2) and cut the detached
  part with the box's operation, which is wrong laser output, so it needs a `format_version`
  bump (ADR 0033 §7).
- **D. The generated object as a real group of part leaves.** Splits the one atomic record over
  several nodes, so a concurrent edit can pair one peer's parameters with another's part (ADR
  0080 option C), and every consumer has to treat the group specially. Rejected.

## Decision

1. **The corner rule (all generators).** Every corner where joints meet has exactly one owner;
   the owner's joint pieces next to that corner are solid and the mating plates have gaps there.
   A plate's material is never joined through a single point. The rule lives once, in
   `fingers.rs`.
   - **Box:** the bottom and the lid own the corner cubes; the walls' vertical joints keep today's
     rule (front and back own the end pieces). **Fix:** `horizontal()` edges `even_solid: true`;
     the wall's horizontal edges (bottom, and top when capped) `even_solid: false`. Step positions
     do not change.
   - **Rounded box:** the wall strip owns the bent spans. A tab never lies in the first or last
     piece of a straight, and on a bent span only in a flat window (§5). A tangent join is not a
     corner. The strip has no joint at its seam: the ends butt between two tabs (§5).
2. **Kerf and clearance, one meaning each** (the doc comments say exactly this):
   - **Kerf k** (default 0, unchanged). Every contour moves k/2 towards the cut-away side: outer
     contours outward, holes inward. So a finger or tab grows by k, and a notch, slot or hole
     shrinks by k. Outer contours run clockwise on screen and holes counter-clockwise, so the
     offset is always k/2 to the left of the walk. Lines move along their normal. Arcs change
     radius by k/2, outward or inward by their turn. **Joins:** a tangent join stays tangent; at
     any other corner the two offset pieces are extended or trimmed to their intersection
     (closed form for lines and arcs; revision 2, needed by the chord windows, the hollow on a
     curved edge and ADR 0083's corners). A miter longer than 2k from the corner is cut square.
     A concave arc of radius below k/2 is clamped by the kerf clamp (§5).
   - **Clearance c** (default 0; derived 0 in the rounded box, spec `0147` criterion 2). It is the
     total play of one engaged finger or tab: every notch or slot widens by c/4 at each side, the
     mating plate's notches do the same, so a finger narrows by c/2 and its notch widens by c/2.
     The side of an end piece that lies on the outer face is not widened.
3. **Joint choice gone.** Remove the `joint` key from the box schema. Remove `Sizes::fingers`,
   `plain()` for walls, the `side = depth - 2t` branch and the `("fingers", false)` lid arm;
   `plain()` stays for the flat lid. An old file with `joint: none` opens as stored (ADR 0080 §5).
   Its first parameter edit regenerates it with fingers. `Values::read` builds the stored map from
   the schema, so the stale key goes there.
4. **Shared modules** in `curvyo-generators-core/src/`. The first commit moves the Box's code;
   apart from rule 1 the move preserves behaviour. Each has two users once ADR 0083 lands; in
   slice 1 the rounded box is the only user of the last three, and their inputs are the general
   shapes below so the path box adds no second copy.
   - `fingers.rs`: the 1-D split of a span into an odd number of pieces, the owner's end pieces,
     clearance widening, merged runs `(from, to, inset)`; `finger_count`; and `tabs(span, count,
     width)` for tab-and-slot joints.
   - `plate.rs` (from `boxes/plate.rs`): the walk round a plate whose four edges carry run lists
     and whose corners are `Owned`, `Open`, `Notched` or `Round(radius)`. A tab is a run of
     negative inset; a strip edge concatenates several spans.
   - `contour.rs`: a closed contour of lines and arcs, the kerf offset of rule 2, and the
     conversion to a `ClosedPath` through `outline::Builder`. It replaces `plate.rs` `offset()`
     and `simplify()`.
   - `slit_bend.rs`: the slit pattern as pure geometry (§6).
   - `footprint.rs` (revision 2): the plan outline of a wall, a closed loop of lines and arcs on
     the wall's inner face with each junction marked tangent or corner (turn angle). It gives the
     spans (straight, bent), their neutral-line lengths, the tab places with their flat windows
     and the seam. `Footprint::rounded_rect(w, d, r)` is the rounded box's.
   - `wall.rs` (revision 2): the developed strip pieces of a footprint: the outline, the slit
     bands with their windows, the tab slots, and, at corners (ADR 0083), the finger joint between
     two pieces through `fingers.rs`.
   - `wall_plates.rs` (revision 2): the plates that sit inside or on a footprint: a tabbed plate
     (inner base, tabbed lid, ring outline), a plain plate (cover, seat lid) and the ring's
     opening. For the rounded rectangle every offset is exact in `contour.rs`; ADR 0083 adds the
     general offset.
5. **The rounded box generator**: kind `rounded-box` (stored `curvyo.generators/rounded-box`),
   label "Rounded box", module `rounded_box/` with `mod.rs` (values, clamps, layout, handles) and
   `schema.rs`; the footprint, strip and plates come from §4.
   - **Footprint.** Inner width w, depth d and corner radius r describe the inside of the wall.
     The strip is developed on the neutral line at r + t_w/2, so its length is `2(w - 2r) + 2(d -
     2r) + pi(2r + t_w)`, less what the chord windows save (below). A straight has the same length
     on every face, so each straight's joints are laid out in its own coordinate and cannot drift.
     Each arc spans `pi/2 (r + t_w/2)` on the strip.
   - **Degenerate straights (Frage 360 B).** r runs up to min(w, d) / 2. A straight shorter than
     0.01 mm is no straight: the value r is snapped to min(w, d) / 2 when it lies within 0.01 mm
     of it, the zero straight is dropped and the two arcs beside it join into one bent span (one
     column layout, no doubled column at the junction). w = d with r = w/2 is an exact circle: one
     closed bent span, no straight. `footprint.rs` and `contour.rs` never emit a zero-length piece.
   - **Parts**, in layout order, each with its own thickness (Frage 355 C; t_w is `thickness`):
     - *Wall strip* (`thickness`, t_w): the developed outline. It carries slit slots and edge
       notches on the bent spans, and one row of tab slots per inner plate.
     - *Inner base* (`plate_thickness`, t_p): the inner footprint with tabs of length t_w. They
       reach through the wall and are flush outside.
     - *Base cover* (`cover_thickness`, t_c; `base_cover`, on by default): the outer footprint
       (radius r + t_w), plain, under the wall's bottom edge.
     - *Seat ring* (`ring_thickness`, t_r; with a lid on a seat): the inner footprint with the
       same tabs, and a hole that is the inner footprint offset inward by the ring width (2 t_r).
       The offset of a rounded rectangle is exact: radius max(r - 2 t_r, 0).
     - *Lid* (`lid_thickness`, t_l): on a seat, the outer footprint, plain, lying on the strip's
       top edge and the ring (spec `0147` criterion 13, the drawings). Without a seat, the lid is
       a second inner plate with tabs (the inner base mirrored) and thickness t_l (answers the UX
       notes §10 item 11). The finger hollow is an optional U notch, centred in the lid's front
       straight: two straight sides at right angles to the edge and a half circle of the hollow
       width, depth at least half the width. With no front straight that holds it (the circle),
       it is centred on the front point of the curved edge, its sides parallel to the front
       direction; the miter joins of rule 2 take the corners.
   - **Thickness fields** (Frage 355 C): `plate_thickness`, `cover_thickness`, `ring_thickness`
     and `lid_thickness` are automatic Lengths (§7): a stored 0 follows `thickness` and the block
     shows the used value muted ("follows wall thickness", UX notes revision 3 §3; the field's label is
     "Wall thickness", its key stays `thickness`). Typing one keeps it through a
     later change of `thickness`.
   - **Tab and slot rule (inner plates and ring to strip).**
     - Tabs: width 2 t (the plate's own thickness, spec `0147` criterion 5), length t_w (through
       the wall, flush outside).
     - On straights: per straight `n = max(1, round(straight / 30 mm))`, centred in n equal
       sub-spans, each keeping at least t_w from the straight's ends (rule 1); a straight shorter
       than 4 t_w has none. The straight that holds the seam takes the next even n (at least 2),
       so the seam falls between two tabs.
     - **On bent spans (Frage 360 B):** only when the straights hold fewer than 4 tabs in total.
       Then every bent span of inner arc length L ≥ 4 t_w takes `max(1, round(L / 30 mm))` tabs,
       and a footprint with no straight at all takes `max(4, round(perimeter / 30 mm))`, reduced (down to 1) while a curved stretch between two windows would keep fewer than two slit columns plus their bridges (review 2026-11-24), spaced
       evenly by arc length and placed half a pitch either side of the seam.
     - **Flat window.** A tab on a bent span sits on a window of length l = 2 t + 2 t_w (the tab
       and a bridge of t_w either side). On every plate of the footprint the window's stretch of the arc
       is replaced by its chord; the tab stands on the chord at right angles. On the strip the
       window is a flat stretch of length l on every face (the inner chord), with no slit column;
       the kink of turn θ/2 at each end (θ the arc's angle over the window) belongs to the
       neighbouring bent span's neutral length;
       neighbouring columns keep one bridge (t_w) from the window. The chord ends are corners of
       turn l / r or less, which the miter joins of rule 2 take and the slits bend. The plate
       leaves the true arc by at most the sag l²/8r: 0.45 mm at t = t_w = 3, r 40; 0.9 mm at r 20.
       Test tolerance for "plate on the footprint": that sag plus the kerf.
     - The strip's slot for a plate of thickness t whose outer face lies e from the strip's edge
       spans `[e, e + t]` across the strip, widened by c/4 per side along it. e = 0 makes the slot
       an edge notch (a finger joint); for e > 0 the bridge to the edge must be at least t_w, so
       a value between 0 and t_w is clamped to t_w with a notice.
     - Base: e = `base_inset`, default 0: the inner plate lies directly on the cover, so the
       cover is the "base offset" of spec `0147` criterion 10. A larger inset raises the inner
       plate further (the customer's "base offset upward"); the cover stays under the strip.
     - Ring: e = 0 at the top edge, so the ring is flush with the edge the lid lies on.
     - Strip height: `base_inset + t_p + inner_height + t_r` with a seat, `base_inset + t_p +
       inner_height + t_l` with a tabbed lid, `base_inset + t_p + inner_height` without a lid.
       `inner_height` is the clear height between the inner plates; cover and seat lid lie outside.
   - **Seam.** In the middle of the back straight (the longer side, the back when equal). With no
     back straight of at least 8 t_w, at the back's middle point anyway: on a stadium or circle
     that is the point between the two back arcs. Tabs are laid out so it falls half a pitch from
     the nearest tab, and no slot ever crosses a strip end. The slit columns keep one bridge (t_w)
     from each strip end.
   - **Slits** on the bent spans outside the windows (all arcs; with Wall slits All round, the
     whole strip except the tab slot spans and one bridge round them; on a circle both choices are
     the same). No band may run uncut across a bend (Context), so the pattern runs out through
     both edges (§6). Edge notches stay clear of the slot spans and windows.
   - **Clamps** (`Clamp`, permit over refuse; spec `0147` criterion 4):
     - r ≥ 2 t_w; r ≤ min(w, d) / 2 (revision 2: the old "longer straight ≥ 8 t_w" bound is gone).
     - spacing ≥ slit width + 1 mm; for Wave, spacing ≥ (slit width + 1 mm) / 0.88 (§6).
     - slit width ≥ kerf + 0.2 mm, so the drawn slot is at least 0.2 mm.
     - kerf ≤ min(t_w, t_p, t_c, t_r, t_l) / 2, using the thicknesses the box uses.
     - ring: the opening keeps at least 2 t_w across, so t_r ≤ (min(w, d) - 2 t_w) / 4 (Dependent).
     - Hollow width ≤ the front straight (w − 2 r, the arcs subtracted once), or ≤ r on a circle; hollow depth ≤ 2 t_r + t_l
       (it ends over the ring).
   - **Layout and handles.** The strip is in the first row. The plates follow in a second row in
     runs of equal thickness value (two parts with typed equal thicknesses share a run), runs in
     the order of their first part in cover, lid, inner base, ring, `gap + kerf` apart,
     everything centred on the origin. Three size handles as on the Box: width and depth on the
     inner base, height on the strip; and a Corner radius handle on the inner base's top-right
     corner, along the inward diagonal (anchors per UX notes revision 3 §6, also for a footprint
     without straights). A scale changes only the three inner sizes, so the slit count follows
     the new perimeter. **The corner radius stays on a scale, also at full round** (revision 3,
     lead decision, Proposal; spec `0147` criterion 7, UX question 9 A): a larger full-round box
     becomes a rounded rectangle; a smaller one is clamped to min(w, d) / 2 (Dependent, the
     notice on release) and so stays full round. One rule for every radius; scaling r with the
     box at full round would be a special case that the stored value cannot show.
   - **Curved top** (criterion 11, a second slice; Proposal). The same parts in the elevation
     plane. The two end plates (inner plate, optional cover) carry the profile: depth x height,
     back-top corner rounded with the corner radius (Frage 353 A). The strip runs from the lid's
     free front edge over the top, round the back arc (the slit hinge) and down the back. Inner
     end plates hold it with tabs on the back straight only. The base and the front are Box-style
     finger-jointed plates (`plate.rs`, rule 1), and the end plates' and front's top edges are the
     lid seat. The footprint corners are square in that mode: a strip cannot bend about two axes.
     A rounded footprint with a curved top is two strips in one generator, later.
6. **The slit pattern, `slit_bend.rs`** (pure; reused by ADR 0083 and `0148`).
   - Input: a `Band { origin, along: Vec2, length, width, keep_clear: Vec<Window> }`, a straight
     region of a flat part with the bend axis across it, and `SlitPattern { kind, slit_width,
     spacing, bridge, rows }`. A `Window` is a stretch `[from, to]` along the band that holds no
     column (a tab window, a slot span).
   - Output: `Vec<Slot>`, each a spine of line and arc pieces (the `contour.rs` piece type), a
     width, and whether it runs out through the band's start or end edge.
   - The caller turns slots into holes or edge notches with `contour.rs`: the slot outline is the
     spine offset by half the width either side, ends cut square to the spine. Columns: `m =
     max(2, round(length / spacing))` per band (each bent span, or the whole strip with Wall
     slits All round), spaced `length / m`, centred; a column inside a window or within one bridge
     of it is dropped. Revision 4 (Proposal of the implementer, accepted): one column leaves an
     uncut band at the bridge height (Straight: at the edge it does not reach; Offset: an even
     column has bridges at both edges), so a bend needs two.
   - **An open set of kinds (Frage 349 B).** `enum SlitKind { Straight, Offset, Wave }`. Each kind
     is a column layout and a spine shape, one private function per kind, matched in one place:
     - *Straight* (meander, straight spine): each column is one slot that runs out through
       alternately the start and the end edge, leaving one bridge at the other edge.
     - *Offset* (lattice, straight spine): even columns hold `rows` slots with bridges between
       them and at both edges. Odd columns are shifted by half a period, so their end slots run
       out through both edges.
     - *Wave* (meander, wave spine): the Straight layout with each spine a chain of tangent arcs
       that swings either side of the column line: amplitude spacing / 8, crest to trough one
       spacing along the slot, arc radius 17/16 spacing; every column in phase. The steepest
       point leans 28°, so two neighbouring slots are at least 0.88 spacing apart (hence the
       Wave clamp in §5). Arcs, not a sine, so the slot's sides are exact offsets (rule 2).
     - **Adding a kind** is one enum arm, its function, one `Choice` option in each schema that
       offers the pattern, and one golden. The option ids are file format (ADR 0034 §5):
       `straight`, `offset`, `wave`. A build that does not know a stored id reads the default
       (Straight), draws the stored outline unchanged, and on the next parameter edit writes
       Straight with a Dependent clamp notice ("Slit pattern set to Straight, the pattern of this
       file is not known here"). Kinds need no new parameters unless a story says so; Wave's
       amplitude and wavelength are derived (Proposal).
   - Bridge = t_w. `rows` automatic is `max(1, round(width / (10 t_w)))`.
   - The module's types use only document-core units and points, not extension-core types. If
     `0148` makes the pattern a fill or a command below `generators-core`, the move is mechanical.
     `0148` adds a curved spine (slot positions by arc length along a path). The column and row
     logic stays.
7. **Contract (source-breaking 0.x in `curvyo-extension-core`, CHANGELOG; the serialised manifest
   the front end reads gains the field, so the TS schema reader changes in the same slice).**
   - `ParamKind::Length` and `ParamKind::Count` gain `automatic: bool`: a stored 0 means
     automatic, which answers the UX notes §9 item 5 and `0141` §15.3. `Param::accept` passes 0
     through unchanged when `automatic` is set; any other value is clamped to `min..=max` as
     today, so the track keeps its UX minimum (0.1 mm, 1 row) and 0 is never reported as a
     clamp.
   - `Generated` gains `automatic: Vec<(&'static str, ParamValue)>`, the values the generator
     used, which the block shows muted.
   - Revision 4: `Param` gains `track: Option<Track { from, to, curve: Linear | Log }>`, the span
     of a number's slider (`0141` UX notes §15.3, which asked for the schema to carry it; `0141`
     revision 2 criterion 9). `None` is the whole range, linear; a drag past either end goes on up
     to the parameter's own bounds (the overshoot rule). `from < to` lie within `min..=max`
     (`curvyo-generators-core/tests/generators.rs` checks every generator number). `from` and `to`
     are plain numbers in the kind's own unit (mm for a Length, whole numbers for a Count, degrees
     for an Angle), as `step` already is: one field serves three kinds, and the manifest is run
     time only (never written to a document or the history), so a typed form stays a cheap `0.x`
     change. Rejected: a typed track inside each `ParamKind` arm (three copies of one rule for no
     check the bounds test does not already make).
   - Revision 5 (second review of `story/generators-flex-box`), three additive items:
     - `Edit::Draft`: the host asks for a ghost under the pointer; a generator may leave out fine
       detail (the rounded box draws its strip as the plain rectangle, no slits and no tab slots;
       the plates stay as they are). The result is never stored: placing generates afresh with no
       edit. Rejected: a `GenerateRequest.draft: bool` (one more field on every request, and a
       draft never combines with `Param` or `Scale`); stripping detail in the host (the slits are
       notches in the strip's outer contour, which only the generator can tell apart). The doc of
       `Edit` should say "what the maker did, or what the result is for".
     - `Generated.states: Vec<(&'static str, &'static str)>`, a word for a value at a limit that
       is a shape ("full round" for `corner_radius` at half the shorter side), never written to the
       record. The shell's `RowView.state` carries it to the field, and the TS view reader gains
       `state` in the same slice.
     - `Bound::DependentMinimumFor(&'static str)`: a lower bound a choice sets, with that choice's
       name for the notice ("the least the Wave pattern allows", §6 Wave spacing). `Bound` now
       serialises that one variant as a map (`{"dependent_minimum_for": ...}`); the front end reads
       no `Bound`, the sentence is built in `curvyo-ui-core`.
     - No `format_version` change: none of the three is written to a document or the history.
   - Automatic rules (Frage 350 A): spacing `0.22 r`; slit width `kerf + 0.3 mm`, so the drawn slot
     is 0.3 mm. Spec `0147` asks for kerf + 0.1 mm, but a closed slot is drawn at its width minus
     the kerf, so the drawn slot would be only 0.1 mm wide. Each extra thickness: `thickness`.
   - No document-core change and no `format_version` bump for this section: kind and params are
     data, and the slots are holes or notches of the strip part (§10 has its own bump).
8. **Parameters of the rounded box** (keys are file format, ADR 0034 §5; Proposal against spec
   `0147` criterion 2):
   - Sizes: `inner_width`, `inner_depth`, `inner_height`, `corner_radius`.
   - Thicknesses (Frage 355 C): `thickness` (wall strip), `plate_thickness` (inner base),
     `cover_thickness` (base cover), `ring_thickness` (seat ring), `lid_thickness` (lid, seat or
     tabbed); the last four automatic, following `thickness`.
   - Cut: `kerf`.
   - Plates: `base_inset` (default 0), `base_cover` (Switch, on), `lid` (Switch, on),
     `lid_seat` (Switch, on: ring and loose lid; off: tabbed lid; ring width derived 2 t_r, no
     field), `finger_hollow` (Switch, on), `hollow_width`, `hollow_depth`.
   - Slits: `slit_pattern` (`straight` | `offset` | `wave`), `wall_slits` (`corners` | `all`),
     `slit_width`, `slit_spacing`, `slit_rows` (the last three automatic; `slit_rows` shown when
     `slit_pattern` is `offset`, `ShownWhen::Is`, answers the UX notes §10 item 12).
   - Layout: `gap`.
   - Later: `top_shape` (`flat` | `curved`, slice 2).
   - Finger width and clearance are derived (2 t_w, 0).
9. **Tests.**
   - **Box:**
     - Every plate outline is a simple polygon (no repeated vertex, no self-intersection) at kerf
       0 and 0.2, for every lid and piece counts 1 to 9.
     - Occupancy: along each of the 12 edges, every piece and corner cube belongs to exactly one
       plate.
     - The existing mating test.
     - Re-bless `box_default.txt` and any document-core golden that regenerates a box.
   - **Rounded box:**
     - Strip length within 0.1 mm against an independent sum: the inner perimeter with the
       windows' chords, plus t_w / 2 times the total turn (pi t_w for a convex footprint). Not
       through the strip's own mapping, which the tab-to-slot test uses.
     - Every slot inside the strip; no two contours closer than the kerf.
     - On every bent span outside the windows, every line along the bend direction (constant
       height across the strip) meets at least one slot or edge notch, so no uncut band.
     - Tab centres mapped onto the strip meet slot centres within c/2, and the base slots lie
       `base_inset` from the edge within the kerf (criterion 10).
     - The ring's top is flush with the strip's top edge and its opening is the footprint offset
       by 2 t_r, both within the kerf (criterion 12).
     - The hollow lies inside the lid outline and leaves it in one piece (criterion 13).
     - Contours closed, holes reversed, no zero-length piece.
     - **Circle and stadium** (Frage 360 B): w = d = 2r gives one strip with no straight, at least
       4 tabs in windows, the seam half a pitch from a tab; every plate lies on the circle within
       the window sag plus the kerf; the same for w > d = 2r.
     - **Patterns:** each of Straight, Offset and Wave passes the uncut-band and spacing tests;
       Wave slot sides are exact offsets of the spine (radii differ by the width within 1e-9 mm).
     - **Thicknesses:** five distinct thicknesses give the strip height formulas above and five
       layout runs; all automatic gives the revision-1 output.
     - Goldens `rounded_box_default.txt` and `rounded_box_circle.txt`.
   - **CPU:** `curvyo-editor-wasm/tests/generators_cpu_time.rs` gains a rounded box of at least
     400 slots: at most 100 ms of thread CPU for the parameter edit with its commit (criterion 6).
10. **Per-part operations, live (Frage 362 B, spec Frage 363 A; `needs-customer`, file format).**
    - **Part keys** (contract, 0.x): `Part` gains `key: Cow<'static, str>` (ADR 0083's wall
      pieces are numbered at run time), unique within one result
      and stable across regenerations: rounded box `strip`, `base`, `cover`, `ring`, `lid`; Box
      `bottom`, `front`, `back`, `left`, `right`, `lid`; gear `gear`; path box (ADR 0083)
      `wall-1` to `wall-n` in walk order from the outline's start, then the rounded box's plate
      keys. The record gains `part_keys`, a list of strings parallel to `parts`, written whole
      with them. Damage: not a list of strings, a length other than the parts', a duplicate.
      Absent (an older file) reads as no keys: the object cannot detach until its next edit writes
      them.
    - **Membership** (amends ADR 0046 §2 and §7 by one key form): an operation may hold
      `member.<node>#<part key>`. The nearest rule evaluates a generated leaf per part: the chain
      of part P of node N starts with `N#P`, then N and N's chain. A part with any own membership
      runs only with its own operations (it is "detached"); the others run with the box's. Key
      grammar checked: node ids are `peer:counter` (`history/lineage.rs`), so `#` never occurs in
      one; today's `decode_path` returns `None` for `12:34#lid`, so a build without this rule
      ignores the key silently and never sweeps it (hence the bump). The
      rule stays pure in `curvyo-laser-core` (`operation_group.rs` neighbour); document-core
      parses the key form and gives the job the outlines of a generated leaf per part key.
    - **Slice.** Slice 3 of spec `0147` (Frage 363 A); slices 1 and 2 write no part keys and no
      `#` membership, and keep `format_version`. `Part.key` may enter the contract earlier, as it
      is run time only; the record's `part_keys` and the membership form land with slice 3.
    - **Commands** (ui-core, one commit each): *Detach part* writes the part's own key on **a new
      copy of the box's operation**, named "<operation> · <part name>" ("Cut 3 mm · Lid") and
      placed right after it in the Operations list, so Detach alone never changes the job (UX
      notes revision 3 §9, Proposal to confirm when slice 3 is Ready; picking another operation is
      the copy's own operation row);
      *Attach part* removes every `N#P` key of that part. A group assignment of the box (ADR 0056
      §1) writes `N#P` for each detached part as for an inner node with its own key.
    - **Lifecycle.** A key whose part does not exist at the moment (Lid off) is kept, saved back
      unchanged and ignored; it applies again when the part returns. The delete sweep (ADR 0046
      §3) removes `N#*` with N. Convert to paths rewrites `N#P` to the new path of P in the same
      commit. A clone of the box runs with the clone's operations; part keys do not reach clones
      (as clone members, ADR 0056 §2).
    - **`format_version`**: the next free number at merge. Migration: none; a file without part
      keys opens unchanged and nothing is written on open. Goldens in
      `curvyo-document-core/tests/fixtures/`: `generated_part_ops_v<N>.curvyo` (a rounded box
      with the lid detached into a second operation, and a dangling `#ring` key with Lid seat
      off); `generated_part_keys_damaged_v<N>.curvyo`.
    - **Tests** (spec `0147` criterion 15): a new box has one operation for all parts; after
      Detach part on the lid the job cuts the lid with its own operation and the rest with the
      box's; a thickness change regenerates the lid and keeps its key; moving the box moves it;
      save and reopen keep both.
    - **Fallback** if the customer declines the format change at slice 3 (Frage 363 C): Option A,
      no record or key change, criterion 15 reads "after Object to path".

## Consequences

- The default rounded box (120 x 80 x 50, r 15, t 3) has about 50 slots. 400 slots are about
  1,600 cubics, some 10,400 numbers, about 100 KB per committed edit in the file and the history
  (gear with 200 teeth: 60 KB). A Wave slot has about four times the cubics of a Straight one. A
  compact encoding stays a later, migratable change.
- Slit cutting takes about twice as long as with single lines (Frage 361 A).
- Per-part operations cost one `format_version` bump, one key form and a per-part step in the
  nearest rule and the job (§10); a detached part stays live. Parts of different materials are
  cut from different sheets: the layout runs by thickness keep each material together.
- The corner rule is written once and used by every plate generator.
- A new slit kind is one enum arm, one option and one golden. A rolling lid, a curved front or a
  rounded footprint with a curved top are further strips and plates in `rounded_box/`, not new
  contract. A box from any closed path (ADR 0083) reuses `footprint.rs`, `wall.rs` and
  `wall_plates.rs` with a footprint built from the path.
- A plate on a circle is a polygon-like curve with short flats at its tabs (the window sag).

## Customer questions

**Frage 360 (ROUND-CIRCLE): a fully round box.** **Answered B, 2026-11-24:** an exact circle
(a square with full corners) and any stadium; the tabs sit on short flat windows of the curve
(§5).

**Frage 361 (ROUND-SLOT): slits as narrow slots or as single lines.** **Answered A, 2026-11-24.**

**Frage 362 (ROUND-SHEETS): parts of different materials.** **Answered B, 2026-11-24**, with the
clarification that all parts share one operation by default and a detached part keeps its own
operation and still belongs to the box; built as §10 (Option C).

**Frage 363 (ROUND-DETACH)** is asked by spec `0147` revision 5 (A: slice 1 without Detach part,
all parts on the box's one operation, Object to path for separate sheets; Detach part as slice 3
with the format change, default; B: Detach part in slice 1 with the format change now; C: Object
to path only, never a format change). For the customer, from this ADR: the format change makes
older Curvyo versions refuse a file with a detached part. Recommendation and default: **A**; the
format change is accepted with the customer's merge of slice 3, as ADR 0056 was.
