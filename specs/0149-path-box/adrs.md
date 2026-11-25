# ADRs for the path box

Status: Proposed (architect, 2026-11-24), revision 2 the same day with the lead's decisions and
the UX notes revision 1, revision 3 the same day with the lead's decisions from the slice-1 build
(spec revision 4). One new ADR (0083, Proposed, revision 3, no questions of its own); it
builds on ADR 0082 revision 3. Decisions marked **(Proposal)** are the architect's and
vetoable at the demo.

- [ADR 0083](../../docs/adr/0083-box-from-a-closed-path.md) (new):
  - kind `path-box`, label "Path box", command "Box from path" in `GeneratorKind.source`,
    module `path_box/` in `curvyo-generators-core` (criteria 1, 2; revision 2);
  - the layout right of the source's bounding box, Gap apart, tops aligned, in the source's
    layer directly above it (criterion 1; revision 2);
  - one outline check in `path_box/`, reached by ui-core through `generate` for create and the
    pick hover; on size a thickness clamp at create or a thickness edit, a stopped scale on a
    resize, never a swapped outline (criteria 1, 8; revision 2);
  - the outline as a copy in a new `Path` parameter, `GeneratorKind.source`, normalised on
    accept; no `format_version` bump (criteria 1, 8; Frage 364 A, 365 A);
  - classification: a cubic within 0.1 mm of its chord is a line, else arcs within 0.01 mm;
    corners above `corner_angle` (25°, 5° to 90°), kinks below it filleted (criteria 2, 3;
    Frage 366 A);
  - one rule per span: corners finger-jointed, straights solid, bent spans always slit, tight
    radii built with a warning naming the piece (criteria 4, 6, 7);
  - `Generated.warnings` (structured, the shell writes the sentence) and `Generated.marks`
    (corner crosses, bend bands, piece badges) for the overlay (criteria 5, 7, 12; revision 2);
  - corner joints at any angle: the overlap zone t / sin|δ|, tips within t |cot δ| / 2 of the
    face, one owner per corner, very sharp corners built with a notice (criteria 4, 5);
  - slits on the flat developed strip, spacing from the local radius, the smallest gap reported
    as the automatic value (criterion 6; Frage 367 falls away);
  - seam only without corners, at the middle of the longest straight, never in a bent span;
    the least-y point only without any straight (criterion 9; revision 3);
  - a kink's fillet radius raised so its arc holds two slit columns at the least gap
    (criterion 6; revision 3);
  - plates: tabbed plates, cover and lid by `contour.rs`'s exact offset (the hollow cut into a
    span), ring opening and crossing test through geometry-core; residual risk at a concave
    valley narrower than 2 t (criterion 10; revision 3);
  - the shipped contract (`Path`, `Source`, `warnings`, `marks`) and the new public
    `Document::create_generated_above` checked against the ADR (§12; revision 3);
  - tests and CPU budgets (criteria 3, 4, 6, 11).
- [ADR 0082](../../docs/adr/0082-generator-plate-joints-corner-rule-and-rounded-box.md)
  revision 3: the corner rule (§1), kerf with miter joins (§2), the shared `footprint.rs`,
  `wall.rs`, `wall_plates.rs`, `slit_bend.rs` (§4, §6), tabs and flat windows (§5), automatic
  values (§7), part keys and Detach part (§10, `needs-customer`, `0147` slice 3, criterion 11).
- [ADR 0080](../../docs/adr/0080-generated-objects-extension-trait-and-parametric-instances.md):
  the generator contract, stored outline, unknown kinds kept, `Edit::Scale`, Convert to paths
  (criteria 8, 11).
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) §2 (the geometry-core edge),
  §5 (parameter keys are file format).

## Feature-local decisions

1. 2026-11-24, revision 2: **`format_version`:** none of its own (criterion 8). Detach part brings
   ADR 0082 §10's bump, which lands with `0147` slice 3; until then nothing is stored per part.
   Revision 2 stores nothing new: placement, layer, warnings and marks are run time. Checked that
   `generated_codec.rs` keeps `params` opaque (a map), so the `path` value needs no bump.
2. 2026-11-24: **The outline is the inner face of the wall**, as criterion 10 implies (cover and
   lid are the outline offset by the wall thickness). (Proposal)
3. 2026-11-24, revision 2: **Slicing** (lead decision, Proposal, UX question 1 A): slice 1
   (straight outlines) builds corners as angled finger joints and kinks as filleted bent spans,
   so it already has bent spans with slits, the seam of a corner-free loop and tabs in flat
   windows (all from `0147` slice 1), and the cover and ring offsets through geometry-core. It
   refuses a curved cubic at create. Slice 2 adds only `arc_fit.rs` and accepts curved cubics.
4. 2026-11-24: **Names** (UX notes §4, accepted): object "Path box" (Layers, plural, history,
   status bar), command "Box from path" (segment, Select bar). The label builds the history
   steps and plurals, so the noun is the label; the verb is `Source.command` (ADR 0083 §1, §2).
5. 2026-11-24: **Automatic slit spacing** shows the smallest gap (UX notes §5, accepted): one
   value per key, and the smallest gap is the one the minimum clamp guards (ADR 0083 §4).
6. 2026-11-24: **Layer** (UX notes §4, accepted): the box goes directly above the source in its
   layer, not on top of the active layer as `0141`; a derived object follows its source, as
   clones, trace results and Stroke to path (ADR 0083 §2).
7. 2026-11-24: **Placement** (lead decision, Proposal, UX question 2 B): the layout's left edge
   Gap right of the source's bounding box, tops aligned; at create only (ADR 0083 §2).
8. 2026-11-24, revision 3: **Slice-1 build decisions** (lead, Proposal; ADR 0083 §3, §6, §7,
   §10, §12): the seam in the longest straight (the least-y seam cut the coffin's kinks at
   Corner angle 90° short of their slits); kink fillets of max(2 t, 2 g / |δ|) (an 18° kink at
   2 t gave a 1.9 mm arc); cover and lid by the exact contour offset so the hollow sits on a
   span, ring opening and crossing test by geometry-core; the heart's four curved cubics merge
   into two bent spans. `Mark::Piece.length` is to become a `Length` before the PR.

## Spec issues (for the product owner)

1. Frage 367 falls away: the wall piece is slit flat, as on the rounded box, so slice 2 needs no
   slit pattern along a curve (ADR 0083 §4). Please drop it or mark it answered by the ADR.
2. Criterion 4 "mitred to the corner's angle": fingers are cut square, so off a right angle a tip
   stands proud or short of the outer face by up to t |cot δ| / 2 (ADR 0083 §5). Please say so
   and test that bound, not "on one line within the kerf", at 60° and 135°.
3. Criterion 6 "a solid margin of 2 times the wall thickness": ADR 0083 §5 keeps z + t_w, which
   is 2 t_w at 90° and longer at sharper corners.
4. Criterion 10 "at slit-free spots": the tabs sit on flat windows (ADR 0082 §5); the mating test
   on a curve allows the window sag plus the kerf.
5. Criterion 1 names the segment "Box from path" and the step "Add path box": ADR 0083 revision 2
   uses both as written, with "Path box" as the label (object name) and "Box from path" as
   `Source.command`; the segment is a pick mode and the Select bar holds the command (UX notes §1).
6. Criteria 5 and 7 (revision 2): the notices name pieces, as the spec revision 3 already does;
   with one wall piece the sentence says "The wall" (ADR 0083 §11).
