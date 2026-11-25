# Plan for Path box (slice 1)

Implementer, 2026-11-24, branch `story/path-box`. Built under `specification.md` (revision 3), `adrs.md`,
ADR 0083 (revision 2, Proposed), ADR 0082 and `ux-notes.md` (revision 2). Fragen 364 to 366 at their defaults A.
Own choices are marked **(Proposal)**.

## Affected crates/modules

- **`curvyo-extension-core`:** `param.rs` (`ParamKind::Path`, `ParamValue::Path`, stored `{"path": [...]}`),
  `manifest.rs` (`GeneratorKind.source`), `extension.rs` (`Generated.warnings`, `Generated.marks`),
  `CHANGELOG.md` (source-breaking 0.x, ADR 0083 §2, §11).
- **`curvyo-generators-core`:** new `path_box/` (`mod.rs`, `schema.rs`, `classify.rs`, `check.rs`, `corner.rs`,
  `pieces.rs`, `plates.rs`, `marks.rs`); `wall.rs` (finger ends of a piece, a margin to the first slit
  column), `footprint.rs`/`tab_layout.rs` (a footprint that starts at a corner, a tab margin), `manifest.rs`,
  `lib.rs`; new edge `curvyo-generators-core -> curvyo-geometry-core` (ADR 0083 §2, allowed by ADR 0034 §2);
  goldens in `tests/fixtures/`.
- **`curvyo-ui-core`:** create from a selected outline (`instance.rs`), the checks before create, the placement.
- **`curvyo-editor-wasm`, `frontend`:** Select bar button, Generators segment pick mode, marks, notices.
- No new crate, no new dependency, no `format_version` change (ADR 0083 §9).

## Tasks

- [x] 1. Contract: `Path` parameter (shape check, stored encoding, default outline), `GeneratorKind.source`,
  `Generated.warnings` and `marks`, CHANGELOG (1, 2, 8).
- [x] 2. `classify.rs`: outline to spans and joins (tangent, kink, corner), the 0.1 mm line test; fixtures coffin,
  shield, heart, circle (3). The heart reads 4 curved cubics and 2 merged bent spans (spec: "four curved spans",
  ADR 0083 §10: "two bent spans"; each assertion names its unit).
- [x] 3. `check.rs`: normalise (clockwise), self-intersection (geometry-core `cut_parameters`), curved cubic as
  chord, default coffin (1, 8). The size rule is in `mod.rs` (`measures`, `edited`).
- [x] 4. `corner.rs`, `wall.rs`: finger ends at any corner angle, owner rule, solid margin z + t (4, 5).
- [x] 5. `shape.rs`, `build.rs`: fillets for kinks, the footprint cut into wall pieces, tabs, seam for an outline
  without corner, slit spacing from the local radius, `tight_bend` and `sharp_corner` warnings (5, 6, 7, 9).
- [x] 6. `build.rs`: inner base, cover, ring (boolean inset), lid and finger hollow place (10).
- [x] 7. `path_box/mod.rs`, `schema.rs`: values, clamps, scale, layout, numbering, marks, manifest kind and goldens
  `path_box_default.txt`, `path_box_20gon.txt` (2, 8, 11, 12).
- [x] 8. `curvyo-ui-core`: create from one closed outline, refusals, placement right of the source, above it (1).
- [~] 9. Shell: Select bar button, Generators pick mode, refusal notices (1). Open: the canvas overlay (corner
  crosses, bend hatch, piece badges, hover highlight in the pick mode), the warning notices after a build and
  under a field, the Corner angle value text, the Replace outline row, the CPU-time test (3, 5, 7, 11, 12).
- [x] 10. Full gate: fmt, clippy, nextest (17427 passed), doc, tsc, npm test, oxlint (warnings as before).

## Validation

Property tests with explicit tolerances (finger and gap centres, tips within t |cot d| / 2 of the outer face,
strip length, no uncut band), classification goldens (coffin, 20-gon), golden `path_box_default.txt`, CPU-time
test (coffin at most 100 ms), full gate.

## Own choices (Proposal)

- A concave corner's notch is `z` deep and its finger ends at the inner corner (ADR 0083 §5); a convex finger
  tip stands `t (tan(d/2) + cot(d)/2)` past the inner corner (slant middle).
- Tabs keep `z_max + t` from a corner (the finger zone plus the solid margin).

## Function length (CLAUDE.md §5) and deviations

`build::strips` (about 70 lines: the joints, the height, one strip per piece) and `path_box::measures` (about 70
lines: the dependent clamps in the order they depend on each other) stay whole, as `0147`'s `generate`.

Deviations from ADR 0083 for the architect: the cover and the seat lid are the exact `Contour` offset (Miter, as
ADR 0082 §2), not the geometry-core offset of §7, because the finger hollow is cut into a footprint of spans; the
ring opening and the crossing test use geometry-core. A seam that falls inside a kink leaves that kink unslit
(see the report). `Document::create_generated_above` is new public document-core API (ADR 0083 §2 names the
placement, not the function).

## Open after the shell work (2026-11-24)

- Finger hollow on a bent span (ADR 0083 §7 "else centred on its longest bent span"): not built; with no
  straight that holds the hollow the lid is built without it and the warning `hollow_left_out` says so.
- Replace outline row: not built. The block shows for one selected box only, so "Replace from selection" has no
  second selected object to read; the way to pick the new outline is a decision for the lead.
- Piece number badges (on the base and on the pieces) and the red marks of sharp corners and tight bends are not
  drawn: the overlay draws corner crosses, bent stretches and the hover outline in the ghost look.
- `build::strips`, `path_box::measures` are split below 60 lines; `build.rs` is split into `build`, `plates`,
  `describe`.

## Shell, second pass (2026-11-24)

- Built: Replace outline (button, pick mode, one step "Replace outline", Escape cancels), pointer cursor in the
  pick mode, sharp-corner and tight-bend marks in the refusal colour, piece badges as stroke digits.
- The badges are strokes in the drag-ghost look, not 0121's digit table (it holds digits 1 to 4 only and is
  private to document-core); no new draw-list primitive was needed.
- The finger hollow on a bent span stays out of slice 1; `hollow_left_out` says so (decision of the lead).

# Plan for Path box, slice 2 (curves)

Implementer, 2026-11-25, same branch `story/path-box-curves`. Built under `specification.md` (revision 7), `adrs.md`
(revision 5), ADR 0083 revision 5 §13 and `ux-notes.md` (revision 4). Section numbers below are ADR 0083 §13.n.
No new crate, no new dependency, no `format_version` change (if one turns out to be needed: stop and report).
Own choices are marked **(Proposal)**.

## Affected files (§13.14)

- **`curvyo-generators-core`:** new `path_box/arc_fit.rs` (biarcs), `path_box/spans.rs` (the outline as lines and arcs
  with fillets, replaces the polygon route of `shape.rs`), `path_box/flats.rs` (flats beside corners), `path_box/hollow.rs`
  (hollow on a run); changed `check.rs` (curve, area, winding), `classify.rs` (nodes and cusps), `build.rs`, `describe.rs`,
  `plates.rs`, `mod.rs`; shared `wall.rs` (runs behind `Wall.runs`, integral spacing), `slit_bend.rs` (`slots` with column
  positions), `tab_layout.rs` (windows on runs, signed flat term), `contour.rs` (`line_circle`, `circle_circle` `pub(crate)`),
  `wall_plates.rs` (`bend_with_hollow` with a middle normal). Goldens `tests/fixtures/path_box_{heart,shield,ellipse,circle}.txt`;
  `rounded_box_*.txt`, `path_box_default.txt`, `path_box_20gon.txt` stay byte-identical (the guard of §13.4; only the 4 mm 20-gon unit test of `shape.rs` changes on purpose).
- **`curvyo-extension-core`:** `Mark::Seam`, doc of `Mark::Bend`, warning id list (`cover_rounded`, `outline_curved` out), CHANGELOG.
- **`curvyo-ui-core`:** `instance_source.rs` (`SourceRefusal::Curved` out), `instance_warnings.rs` (arms), `preview` and
  `hover_marks` build with `Edit::Draft`.
- **`curvyo-editor-wasm`, `frontend`:** CPU tests, `path_box_marks.rs` (seam, bend), `generatedView.test.ts` sentence.
- **Tester-owned** `curvyo-generators-core/tests/acceptance_0149.rs` and `curvyo-ui-core/tests/acceptance_0149.rs`: only what slice 2
  supersedes is adapted, each marked "Superseded by".

## Tasks (criteria in brackets, test first, one commit each)

- [x] 1. `arc_fit.rs`: biarcs within 0.01 mm, splits at inflections, cusps and 90 degrees, halving, exact G1 joins, 1e6 mm guard (14).
- [x] 2. `check.rs` reads the curve: area by geometry-core, winding, degenerate and crossing; D and lens valid (1, 13).
- [x] 3. `classify.rs`: corners at nodes and cusps only, tangent at 1 degree or less; `spans.rs`: lines, arcs, fillets beside arcs (3, 14, 15).
- [x] 4. Runs in `wall.rs`, `slit_bend::slots` with columns at equal steps of the integral of 1/g, `tab_layout` windows on runs, exact signed neutral length (6, 9, 14).
- [x] 5. `flats.rs`: flats beside corners with the bounds of 13.7, joint as cut, `sharp_corner` from the cut turn (6, 5).
- [x] 6. Plates on exact arcs, ring and cover test through geometry-core, fallback and `cover_rounded` (10, 16).
- [x] 7. Hollow on the lid's longest convex run (10, 16).
- [x] 8. `Mark::Seam`, `Mark::Bend` per stretch of one tightness, `tight_bend` from the arcs (7, 9, 12).
- [x] 9. Remove `outline_curved`, `SourceRefusal::Curved`; hover and Select bar in Draft; adapt superseded tests (1, 11). Open: the `cover_rounded` sentence has no variants for Lid or Finger hollow off, and "The wall" / "bends inward" (UX notes 6) are not worded.
- [x] 10. Goldens heart, shield, ellipse, circle; classification and piece counts; CPU budgets (13, 14, 15, 11). Open: the mating tests of `0147` on heart and shield (16).
- [ ] 11. Full gate once.

## Validation

Arc fit within 0.01 mm sampled both ways, tangency within 1e-9 rad; piece length against geometry-core `PathMeasure` within 0.1 mm;
spacing (N at least 2, no gap below `least`, the reported value the smallest gap); mating tests on heart and shield; peanut fallback;
CPU: heart edit at most 300 ms with at least 400 slots, Draft hover at most 100 ms (thread CPU); the slice-1 goldens byte-identical.

## Own choices of slice 2 (Proposal)

- The arc fit aims at 0.008 mm on its 16 samples so the 0.01 mm of the ADR holds between them.
- A kink's fillet takes its tangent points across several fitted arcs of one tangent chain (a fillet
  next to a 5 mm fitted arc would otherwise get a tiny radius); each side may use half a chain that a
  second kink shares (ADR 0083 13.3).
- Outlines of straight cubics only keep the slice-1 route (`shape.rs`) so the goldens stay byte-identical;
  every outline with a curved cubic goes through `spans.rs`.
- The fallback cover is the geometry-core round offset as a polyline of four chords per offset cubic.
- The golden heart has a notch of about 100 degrees: a narrower V (53 degrees) crosses itself in the exact
  cover with 3 mm walls and takes the fallback, as criterion 16 says it should.
- `Span.piece_start` marks where a wall piece starts so runs and windows stop at a corner.
