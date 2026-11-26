# Plan for Generator joint options

## Slice 1 (tighter tab windows)

Implementer, 2026-11-26, branch `story/generator-joint-options`. Built under `specification.md` (slice 1, criteria 1 to 9)
and `adrs.md` (A0). No field, no key, no `format_version` change. Own choices are marked **(Proposal)**.

## Affected crates/modules

- **`curvyo-generators-core`:** `wall_bands.rs` (band ends at a window, columns laid from the band ends),
  `slit_bend.rs` (float tolerance at a window's edge); goldens `rounded_box_circle.txt`, `path_box_circle.txt`,
  `path_box_heart.txt`, `path_box_ellipse.txt` (`path_box_shield.txt` has no window on a bend and stays byte for byte);
  `tab_layout.rs` / `tab_runs.rs` and the plates are not touched (criterion 4). No new crate, no dependency.

## Tasks

- [x] 1. Test first: `tests/acceptance_0155.rs`, the flat across every tab window on a bend is at most l + 0.1 mm for
  Straight, Offset and Wave, kerf 0 and 0.2, on circle, short stadium, heart and ellipse (1).
- [x] 2. `wall_bands.rs`: a band beside a window starts one bridge from the widest tab slot (the window's span holds the
  slot in its middle), and its columns are laid from the band ends: the column next to a window half a slit width
  (plus a wave's swing) inside, the others half a step; steps never below the radius's gap, two columns at least,
  fewer only below the least gap on a path-box run (2, 3, 7). Bands with no window beside them are unchanged (5).
- [x] 3. Unit tests of the column layout in `wall_bands.rs`; pinned column count of the circle strip (2).
- [x] 4. Re-bless the four goldens; the gap test of the heart and ellipse knows the new window gap (6).
- [x] 5. Full gate (4, 5, 8, 9 by the existing tests: strip length, mating, no uncut band, CPU time).

## Validation

Property test of the flat, existing 0147/0149 acceptance tests (strip length, mating, uncut bands, 400-slot CPU time),
sha256 of every golden except the four re-blessed ones. Demo check (9) is the customer's cut.

## Own choices (Proposal)

- The kink of half a wall times the turn that a window's neutral length holds sits inside the window's span, so the
  band starts 0.3 mm inside the span at r 30 (bridge to the slot stays t_w); the flat on the strip is exactly l.
- Steps follow the existing floor rule (gap at least what the radius asks), which gives the 18 to 24 columns of the spec;
  the spec text "no larger than" would give 29.
- The test measures the cut flat, the drawn flat less the kerf (drawn slits are centre lines, each edge kerf / 2 inside).


## Slice 2 (Bend through tabs, criteria 10 to 17)

Implementer, 2026-11-26, branch `story/joint-bend-through-tabs`. ADR 0089 decision 2. Own choices are **(Proposal)**.

Affected: `curvyo-extension-core` (`Param.initial`, CHANGELOG), `curvyo-generators-core`: `rounded_box/schema.rs` (Choice
`bend_tabs`, Flat top only, path box too), `tab_layout.rs` / `tab_runs.rs` (a tab on a bend without splitting it),
`wall_plates.rs` (tab as a rectangle on the normal with a concentric tip), `wall.rs` + `slit_bend.rs` (Window with a height
span, columns clipped t_w from the slot), `rounded_box/mod.rs`, `path_box/build.rs` (read the key). No new crate, no
dependency, no `format_version` change.

- [x] 1. Test first: `tests/acceptance_0155_curve.rs`: schema and `initial` (10), strip length without window shortening (13),
  clipped columns keep t_w and no kerf collision (11, 12), uncut bands (12), mating at five thicknesses (14), curved
  top and straight-only boxes equal (15), window equals slice 1 (15), small radius never fails (16), CPU time (17).
- [x] 2. `Param.initial` + `bend_tabs` rows (10).
- [x] 3. `Window` rows and column clipping in `slit_bend.rs` / `wall.rs` (11, 12).
- [x] 4. Tabs on a bend: `tab_layout.rs`, `tab_runs.rs`, `wall_plates.rs` (11, 13, 14, 16).
- [x] 5. Golden `rounded_box_circle_curve.txt`, `path_box_heart_curve.txt`; full gate (15, 17).

Own choices (Proposal): a tab keeps the window layout's count and places (same tabs, no chord); a tab that does not lie wholly on one
arc, or a concave arc too small for the tip, stays a window on that stretch with a Dependent-free silent fallback, as a window
is the last resort of criterion 16; a wave slot clipped to a closed piece is drawn as a plain rectangle.


## Slice 3 (Joint layout for tabs, criteria 18 to 24)

Implementer, 2026-11-26, branch `story/joint-tab-layout` (stacked on `story/joint-bend-through-tabs`). ADR 0089 decision 3. Own
choices are **(Proposal)**.

Affected: `curvyo-generators-core`: new `joint_layout.rs` (pure 1-D layout, Even and Centred with Spread, the per-edge fit),
`tab_layout.rs` / `tab_runs.rs` (straight tabs by the layout, typed count on a loop, typed count suppresses windows),
`wall.rs` / `wall_bands.rs` (`Joint.half_width`), `wall_plates` callers, `rounded_box/mod.rs` + `schema.rs`, `path_box/build.rs`,
`mod.rs`, `schema.rs`, `values.rs` (`limit_count`); frontend: the Advanced group collapses, `joint_spread` shows "%".
No new crate, no dependency, no contract change, no `format_version` change.

- [x] 1. Test first: `joint_layout.rs` unit tests (Count 1 centre, no overlap, margins, Spread 100 % equals Even, count rules) (19, 21).
- [x] 2. Test first: `tests/acceptance_0155_layout.rs`: group and keys, defaults equal the goldens, mating and centres per pattern, seam
  straight even, clamps with notices, 100 joints per edge CPU time, key round trip (18 to 24).
- [x] 3. `joint_layout.rs` and `tab_layout.rs` / `tab_runs.rs` (19 to 22).
- [x] 4. `Joint.half_width`, plates and windows read the joint size (20).
- [x] 5. Schema group Advanced, reading, clamps and used values in the rounded box and the path box (18, 20, 23).
- [x] 6. Frontend: collapsed Advanced, "%" unit; tests (18, 19).
- [x] 7. Full gate; spec status (24).

Own choices (Proposal):
- Gap is edge to edge; Centred gaps are g, 2g, 3g outward from the middle; the least gap between two tabs is one joint size.
- Typed values are clamped once for the whole box to the tightest edge that carries tabs (notice "most this box allows"); an edge where
  not even one tab fits keeps none, as today. The muted Auto count shows the most tabs on one edge.
- Joint size typed: every plate's tab and the strip slot use it; Auto keeps 2 times each plate's own thickness.
- Joint gap and Joint spread show with Centred (no contract change: `ShownWhen` has no number test, so not "Count 2 or more").
- Curved top: Advanced is hidden there (its end-plate tabs are edge tabs, no span layout); slice 4 owns the curved top.
- The Box's leftover `joint` Choice is not removed here (removal of accepted behaviour, the lead decides).
- An unknown pattern id reads Even silently, as an unknown slit pattern does.


## Slice 4 (Joint layout for finger joints, criteria 25 to 29)

Implementer, 2026-11-26, branch `story/joint-layout-fingers` (stacked on `story/joint-tab-layout`). ADR 0089 decisions 3 and 4. Own
choices are **(Proposal)**.

Affected: `curvyo-generators-core`: `joint_layout.rs` (`finger_split`: the pieces of one finger joint from the same centres
as the tabs), `plate.rs` (`Fingers` may carry explicit piece widths), `boxes/mod.rs` + `boxes/schema.rs` (Advanced group,
Joint size is `finger_width`, the `joint` Choice goes), `rounded_box/curved_top/*` (clamp, plates, strip end, end-plate
tabs read the layout), `rounded_box/schema.rs` (Advanced rows on both tops). No new crate, no dependency, no
`format_version` change.

- [x] 1. Test first: `tests/acceptance_0155_fingers.rs`: group and keys of the Box (25), defaults byte for byte (25),
  centres agree per pattern and count, Count 1 centred (26), no contour closer than the kerf and every part closed
  per pattern (27), clamps with notices, key round trip, CPU time (28), the 120 x 80 x 50 demo box (29).
- [x] 2. `joint_layout::finger_split` + unit tests (26).
- [x] 3. `plate.rs` `Fingers` with explicit widths; Box reads the layout (25 to 27).
- [x] 4. Box schema: Advanced group with Finger width as Joint size; the `joint` Choice and its plain-plate path removed.
- [x] 5. Curved top: finger joints, strip end and (if cheap) end-plate tabs read the layout; Advanced rows on both tops (25).
- [x] 6. Full gate; spec status (28, 29).

Own choices (Proposal):
- A finger joint's count is the number of the mate's fingers (odd pieces); Even with Auto count and no typed size is today's
  equal-pieces rule (byte for byte), Even with a typed count puts fingers of the joint size at the even centres, Centred
  uses the tab centres of slice 3. The owner's pieces are the rest, at least one finger width at each end.
- Typed values are fitted once to the tightest finger joint of the box (as slice 3), only when a count is typed or the
  pattern is Centred, so a default box never gets a notice.
- The `joint` Choice is removed now (ADR 0089 decision 4, which names slice 3 but was left open; the Box is the first thing
  touched here): a stored `joint: none` reads as fingers, the key is dropped on the next write (customer: old files do not matter).
- The curved top's end-plate tabs and the strip's tab slots read the same layout (cheap: one `places` function shared by the
  fingers and the tabs); a typed Joint size also sets their width. The run they lie on is fitted with the finger joints.
- Box: Finger width keeps its own clamps (smallest inner side, kerf 0.4 times the width); the layout only adds count, gap
  and spread clamps. A stored file with the removed `joint` key opens with fingers and loses the key on its next write
  (`Values::finish` keeps schema keys only; ADR 0089 decision 4 said "kept and ignored", the host regenerates from the schema).
- Muted rows: Auto count shows the most fingers (or tabs) on one joint; the curved top shows its automatic Joint size
  (twice the thinner part) muted.
- `a_box_with_no_joint_is_plain_rectangles` (0141, `tests/generators.rs`) replaced by "a stored joint none reads as fingers"
  (0141 revision 2 had already removed the choice); the slice 3 test "rows hang on the flat top" now says both tops.
- `joint_layout.rs` is 522 lines with its unit tests; the code itself is under 430.
