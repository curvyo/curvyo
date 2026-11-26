# Plan for Generator joint options, slice 1 (tighter tab windows)

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
