# Architect review, branch story/ellipse-arcs-and-shaping (17e74ef8)

Verdict: CHANGES REQUESTED (rewrite 7 old tests, renumber format to 16 after #150, one size note). No structural finding.

## Checked and fine
- Core purity: new modules `ring_curve.rs`, `ellipse_arc.rs`, `shape_arc.rs`, `shape_arc_codec.rs` are pure; no unsafe, no new trait, no generics, no new crate or dependency. Module docs are one sentence each. The only non-test `unwrap` sites carry `// invariant:` comments (`primitive_outline.rs:381-385`, `ring_curve.rs:374-376`).
- One shared evaluation: `Ring`, `effective_curve`, `curve_handle_position`, `curve_from_position`, `arc_outline`, `arc_bounds` live in document-core; ui-core (`shape_knobs.rs`, `shape_arc_edit.rs`) and render-core (`drawing_outline_of`) call them. `PrimitiveOutline { anchors, closed }` derefs to anchors, so callers stay small.
- Decision 9: `converts_on_stretch` keeps an ellipse with an arc (`conversion.rs:57-60`); `arc_after_linear_map` / `ellipse_after_linear_map` do the SVD with a round-trip test. Skew still converts. A curved ellipse converts unless the stretch runs along its axes, as the spec says.
- Registers `ellipse_arc` and `curve` are deleted at their defaults; validation is `Damaged`; there are golden and damaged fixtures; the LWW test for set against delete exists. `Document::resize_ellipse_with_arc` is additive.
- Units: `Curve` and `Angle` are newtypes. The tolerances I looked at are explicit.

## Must fix
1. Format version. Only one source literal exists, `document.rs` `CURRENT_FORMAT_VERSION = 15` with its doc paragraph (about line 180). It is in its own commit `209b0bfd`, but that commit also holds the 8 fixtures and the format test, so the renumber is a regeneration and not a one-line change. After #150 merges and this branch is rebased, change all of these to 16:
   - the const and its comment;
   - the 8 `*_v15.curvyo` fixtures, regenerated and renamed to `_v16` because the manifest inside the zip says 15;
   - `ARC_FORMAT_VERSION` and the golden names in `tests/arc_curve_format.rs`, and the three damaged names in `acceptance_0017_block1_tester.rs:544-546`;
   - the literals in `acceptance_0042_feedback_tester.rs` and `acceptance_0042_tester.rs` (document-core and editor-wasm), `acceptance_0047_part_a_tester.rs`, `history_versions.rs`, and the `rezip_manifest(.., 15)` newer-than-current tests;
   - the `specs/README.md` format table row (it will have one row for 15 from #150 and one for 16 here) and the `plan.md` header.
   Ask the implementer to keep the renumber as its own commit. A cheaper design: have those tests compare against `CURRENT_FORMAT_VERSION` where they only mean "unchanged by this story". Do that in the same renumber commit.
2. The 7 failing tests are all caused by the spec and must be rewritten, not deleted. Criteria 1, 22 and 40 replace criteria 6 to 8 of 0009, and the "polygon had no parameter handle" statement, for these cases:
   - `acceptance_unified_editing.rs` `ac07_ellipse_tiers...` (l.492) and `ac07_polygon_tiers...`: from 72 px an ellipse has the arc knobs and the curve knob, a polygon has the curve knob. Rename the tests and assert the parameter tier at 72 and none below. `ac07_star_tiers...`: from 72 px a star has the inner-radius knob and the curve knob, so the glyph count changes from one to two.
   - `acceptance_0005.rs` `ac1_polygon_and_star...` (l.430): the side middle of a polygon is now the curve knob (cursor "pointer"). Assert that, and probe "not a handle" at a quarter of the side (cursor "move").
   - `acceptance_polygon_star_box_refit.rs` `ac02_` (l.402) and `ac05_` (l.661): the decoration now contains the knob glyph vertices (blue and white). Filter the vertices of the curve knob, whose position is known from `curve_handle_position`, before comparing with the turned square. The box and the frame are what these tests protect.
   - `acceptance_unified_editing_pr2.rs` `ac32_` (l.1007): the ellipse probe point lies on a knob, and a double-click on a knob opens the entry chip (criterion 8). Move the ellipse probe point to an outline spot clear of every knob (for example the top). Add a case for the knob double-click in the new tests if not already there.
   Test intent that stays: a double-click on a primitive anywhere else gives the hint and writes nothing.

## Should fix
3. `curvyo-document-core/src/shapes.rs` is 541 non-test lines (501 on main). The ADR said new code goes to `shape_arc.rs`. Move what is arc-related out of `shapes.rs`, or give the reason in the PR. `primitive_model.rs` is 585 non-test lines (556 on main, +29): same note. Neither blocks.
4. `plan.md` "Notes from the build" says the failing tests are "left for the tester". Rule 1 of the PR is "gate green"; the rewrite above is part of this slice, so remove that sentence once done.

## Judgements
- Knob displacement of 28 px (two pitches) instead of 14: accept. The 14 px rule cannot clear a knob that sits exactly on a corner glyph (the glyphs are about 15 px apart in the worst case). Impact is cosmetic and the value follows the drawn position. Record it in the UX notes of the spec as a dated line ("Knob displacement: at most two pitches"), as it overrides "at most 14 px".
- Not building the vetoable proposal of criterion 56 (a rotated whole ellipse keeps its kind on stretch): accept. It changes 0047 criterion 7 and was explicitly vetoable and not asked for. Put it in `docs/technical-debt.md` or the backlog as an offered follow-up, so it is not lost.
