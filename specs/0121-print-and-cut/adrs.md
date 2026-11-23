# ADRs for print and cut

Architect, 2026-11-23. Slice 1 (Fragen 328 to 332 at their defaults, A each).

- [ADR 0081](../../docs/adr/0081-print-and-cut-registration-marks-and-fit.md) (new, Proposed,
  `needs-customer` for the document model and file format; accepted with the PR as 0045 was):
  the `registration_mark` leaf kind and the tagged marks group (criteria 1 to 3, 6), never burns
  and always prints (4, 5), the measures register on the placement (7, 9, 19), the reading and
  its warnings in `curvyo-laser-core/src/print_and_cut.rs` (10, 13, 14, 18, 22), the fit in
  `curvyo-geometry-core/src/point_fit.rs` (11, 12, 15), Apply and Reset (16, 18), the head
  position read (8, 20, 21), format 55 and goldens (17, 19), seams for slices 2 to 4.
- [ADR 0045](../../docs/adr/0045-clones-of-groups-texts-images-and-movable-placed-pages.md) §7:
  unchanged; Apply is one `placement_link` write and keeps the off-page refusal. A group clone
  draws marks (§1); a mark is no clone origin of its own.
- [ADR 0027](../../docs/adr/0027-linked-dimension-objects.md) §7: the precedent for a leaf that
  never burns; a mark differs in that it is exported (SVG export, page picture).
- [ADR 0064](../../docs/adr/0064-laser-job-builder-and-output-writers.md) §1: the job needs no new
  field; `leaf_geometry` gains the mark arm returning nothing. `Frame::machine_point` converts
  measures.
- [ADR 0065](../../docs/adr/0065-laser-output-settings-and-air-assist.md): Start is read only for
  the Head position warning; no new setting.
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §7: format 55 (next
  free at merge, main at 54).
- [ADR 0071](../../docs/adr/0071-connector-api-ctl-and-maintenance-mode.md): unchanged; the
  existing `Status.position` (work position) is the read, no new message.

## Open question

**Frage 333 (PNC-MODEL): What is a registration mark in the file?**
- A: its own object kind (centre, number, size, line width), drawn by Curvyo, inside an ordinary
  group; never burnt, always printed. Files with marks need the new app version.
- B: ordinary shapes (circle, cross, number text) grouped and tagged as a mark; you could edit
  them like any shape, but then the centre is no longer certain, and every laser and export path
  must look for the tag.
- Recommendation: A (the centre is exact by construction; "never burns" is one rule). Default if
  unanswered: A.

## Notes for the PO (spec changes, ADR 0081 Consequences)

1. Criterion 8: the head position is the *work* position (the frame the job's absolute moves
   use), not the machine position of `0140` criterion 10; disabled while Running *or Homing*.
2. New warnings (Proposal): "Start is Head position: the cut will not follow the sheet"; "The fit
   puts the sheet off the laser page." (Apply refused); "The fit is not usable: scale 31.00 %."
   (outside 50 to 200 %, Apply not offered); "The stored measures could not be read."
3. "An older build shows the marks as plain shapes" becomes "an older build refuses the file".
4. Slice 3: the projective correction is the camera's pixel-to-bed map (`0120`); the sheet fit
   stays a similarity. Frage 331 C needs no placement change (`placement_link` is a full matrix).
5. "Moved by X and Y" is the machine position of the sheet's page origin (Proposal).
6. A copy of a placed sheet does not copy its measures (Proposal).
