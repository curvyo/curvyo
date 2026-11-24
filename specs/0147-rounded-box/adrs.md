# ADRs for the rounded box

Status: Proposed (architect, 2026-11-24), revision 2 the same day with the customer's answers
(Frage 349 B, 355 C, 360 B, 361 A, 362 B) and spec revision 3, revision 3 with the lead's
decisions on the UX notes revision 3 (spec revision 5), revision 4 the same day after the second
review of `story/generators-flex-box` (decisions 9, 10). One new ADR (0082, Proposed, revision 5;
§10 `needs-customer`, file format, spec Frage 363 ROUND-DETACH, default A: slice 3). Decisions marked **(Proposal)** are the architect's and vetoable
at the demo.

- [ADR 0082](../../docs/adr/0082-generator-plate-joints-corner-rule-and-rounded-box.md) (new):
  - kind `rounded-box`, module `rounded_box/` in `curvyo-generators-core` (criteria 1, 2);
  - shared `fingers.rs`, `plate.rs`, `contour.rs` with the Box, and the corner rule (criterion 5);
    `footprint.rs`, `wall.rs`, `wall_plates.rs` at crate level for ADR 0083 (revision 2);
  - kerf and clearance semantics, miter joins at non-tangent corners (revision 2);
  - strip length on the neutral line at r + t/2 (criterion 3);
  - the part set of the drawings: strip, inner base, base cover, seat ring, lid with finger
    hollow, a thickness field per part, the extra four following Wall thickness (criteria 10, 12,
    13; Frage 355 C);
  - the tab-and-slot rule between inner plates or ring and strip (criteria 5, 10, 12);
  - slits as closed slots and edge notches that run out through both edges, never an uncut band
    across a bend (criterion 4);
  - the pattern as the pure, reusable `slit_bend.rs`, Straight, Offset and Wave as an open set of
    kinds (criteria 9, 14 / `0148`; Frage 349 B);
  - corner radius up to half the shorter side, an exact circle, tabs on curves in flat windows
    (criterion 2; Frage 360 B);
  - live per-part operations: part keys, `member.<node>#<part>`, Detach and Attach part writing
    a copy of the box's operation, the next `format_version` (criterion 15, slice 3; Frage 362 B,
    363 A; `needs-customer`);
  - a resize keeps the corner radius, also at full round (criterion 7; revision 3, Proposal);
  - automatic values: a schema marker plus the values used (criterion 2, Frage 350, UX notes §9
    item 5);
  - slider tracks in the schema, `Param.track` (revision 4; `0141` revision 2 criterion 9);
  - the curved top as a second slice (criterion 11, Frage 353 A);
  - clamps, tests and the 400-slot CPU test (criteria 4, 6).
- [ADR 0080](../../docs/adr/0080-generated-objects-extension-trait-and-parametric-instances.md)
  covers the generator contract, the stored outline and unknown kinds kept (criterion 8), and
  `Edit::Scale` and Convert to paths as the group of parts (criterion 7). The history labels come
  from the manifest label ("Add rounded box", criterion 1). Unchanged.
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) §5: parameter keys are file
  format.
- [ADR 0083](../../docs/adr/0083-box-from-a-closed-path.md) (new, the customer's path-box idea of
  2026-11-24): not part of this story; it reuses the modules ADR 0082 §4 lifts to crate level.

## Feature-local decisions

1. 2026-11-24: **`format_version`:** unchanged in slices 1 and 2 (criterion 8). The record already
   holds kind, params and parts; slots are holes or edge notches of the strip part. Slice 3
   (Detach part) bumps it (ADR 0082 §10).
2. 2026-11-24: **Name.** Kind id `rounded-box`, label "Rounded box", module `rounded_box/`, pattern
   module `slit_bend.rs` (after `0148`). The slug `0147-rounded-box` fits and stays. The kind id is
   file format, so it is fixed before the first merge. (Proposal)
3. 2026-11-24: **Parts** in layout order: strip, inner base, base cover, seat ring, lid. One part
   each; the strip's slots are reversed subpaths of the strip part.
4. 2026-11-24: **Seam** in the middle of the longer straight; plain butt ends, held by the inner
   plates' tabs. (Proposal)
5. 2026-11-24: **Slicing** (for the lead): slice 1 is the vertical-corner box with all five parts,
   Straight, Offset and Wave, the circle and stadium, a thickness per part, and the automatic
   values (revision 2), with no Detach part (Frage 363 A). Slice 2 is the curved top (criterion
   11), built from the same modules plus Box-style finger plates for base and front. Slice 3 is
   Detach part with the `format_version` bump (criterion 15, ADR 0082 §10).
6. 2026-11-24, revision 3: **Resize at full round** (lead decision, Proposal, UX question 9 A):
   the radius stays; a larger full-round box becomes a rounded rectangle, a smaller one is
   clamped and stays full round (ADR 0082 §5).
7. 2026-11-24, revision 3: **Detach part writes a copy** of the box's operation, "<operation> ·
   <part>", right after it (UX notes revision 3 §9; ADR 0082 §10; confirm when slice 3 is
   Ready).

8. 2026-11-24, ADR 0082 revision 4 (architect review of `story/generators-flex-box`): **at
   least two slit columns per bend** (implementer's Proposal, accepted; ADR 0082 §6), because one
   column leaves an uncut band at the bridge height (criterion 4). **`Param.track`** joins the
   contract change of ADR 0082 §7.
9. 2026-11-24, ADR 0082 revision 5 (second architect review of `story/generators-flex-box`):
   **`Edit::Draft`** (the ghost's strip is the plain rectangle), **`Generated.states`** with
   `RowView.state` ("full round" in the field and the knob readout) and
   **`Bound::DependentMinimumFor`** (the Wave spacing notice) join the contract change of ADR 0082
   §7. `format_version` unchanged.
10. 2026-11-24: **Gear resize readout and tooth-stepped preview** ("34 teeth · ⌀ 72 mm", `0141` UX
    notes §15.6) are shell code in `curvyo-ui-core` (`instance::resize_view`), not contract. They
    are a special case of the built-in gear until a second generator steps its resize; then the
    readout moves into the contract. Its natural home is `specs/0141-parametric-generators/adrs.md`.

## Spec issues (for the product owner)

1. Criterion 4 says slits "stop at least one thickness from the top and bottom edge". That
   cracks: an uncut band across a bend strains by t/2r, 10 % at 3 mm and r 15, where wood takes
   about 1 to 2 %. Alternate columns must run out through both edges (ADR 0082 §6); the strip
   holds through the bridges. Please change the wording and the test.
2. Criteria 5 and 9 ("fingered lid and slotted lid") predate the drawings. Joints are now
   tab-and-slot between inner plates or ring and strip. A slot at the very edge is a finger notch
   (`base_inset` 0). Please align the two criteria with criteria 10, 12 and 13.
3. Criterion 10 reads the customer's "base offset" as the cover. ADR 0082 keeps that as the
   default (`base_inset` 0, inner plate directly on the cover). It adds `base_inset` for the
   raised inner base of the customer's addition (1); between 0 and one wall thickness the value
   is clamped. Please add the slider, or drop it explicitly.
4. Frage 350 A "width = kerf + 0.1 mm": a slit is a closed slot drawn at its width minus the
   kerf. The automatic rule is therefore `kerf + 0.3 mm` (drawn 0.3 mm); please align the text.
5. Criterion 2 "at the maximum the footprint is a stadium or circle": superseded by Frage 360 B
   (issue 9).
6. Criterion 11 "the footprint corners may stay rounded": a strip cannot bend about two axes. A
   curved top with square footprint corners comes first; rounded corners plus a curved top come
   later, as two strips.
7. Criterion 2 "Wall slit rows": kept as an automatic Count, as the UX notes have it.
8. Frage numbers: ADR 0082 uses 360 to 362 and leaves 356 to 359 for this spec.

Against spec revision 3 (2026-11-24; ADR 0082 revision 2 answers what it left to the architect):

9. Criterion 5, circle: tabs on curves sit on flat windows (ADR 0082 §5), so the mating test
   on a curve allows the window sag l²/8r plus the kerf, not the kerf alone. Tab width is the
   plate's own thickness times 2, as the spec says (adopted).
10. Criterion 3, seam without a straight: the middle of the back, between two tabs (ADR 0082 §5).
11. Criterion 15, Detach part: built as ADR 0082 §10 Option C (part keys, `member.<node>#<part>`,
    one `format_version` bump), `needs-customer` through Frage 363. Please add to its option A
    that the file format changes (older Curvyo versions refuse such files), and note that "Ready
    for slice 1" now holds a `needs-customer` item. The UX notes revision 3 say which operation
    Detach part writes (a copy of the box's or a picked one). Fallback 363 B.
12. UX notes §10 items 6, 11 and 12 are answered by ADR 0082 §5 and §8 (cover and lid thickness
    fields, the tabbed lid takes Lid thickness, Slit rows shown when Offset).
13. Frage numbers: 363 is the spec's ROUND-DETACH, which ADR 0082 §10 uses; 356 to 359 stay unused.

## Note: box from a closed path

2026-11-24: the customer's path-box idea is spec `0149` (`specs/0149-path-box/adrs.md`, ADR 0083).
It reuses the modules ADR 0082 §4 lifts to crate level and the part keys of §10.
