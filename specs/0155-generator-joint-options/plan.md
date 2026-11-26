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
