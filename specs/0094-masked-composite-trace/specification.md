# Masked and composite trace: trace only parts of a picture, each with its own settings, and join them

Status: Draft
Priority: Should
Origin: Customer (2026-11-03); everything marked Proposal is the product owner's

## User value
As a maker who traces a scan, a photo or a logo sheet I want to limit a trace to the inside of
ordinary paths I drew (or to everything outside them), trace different parts of one picture with
different settings, and get the parts back as one result, so that fine detail and large flat areas
each get their best setting instead of one compromise, and so that I never have to erase or crop
the picture first.

Customer wording (2026-11-03, German, paraphrased): a simple workflow to trace graphics only
partially, using the normal paths as masks (positive and negative), and partial composite tracing:
trace certain parts of an image with one setting, other parts with other settings, and at the end
join them into one.

Requirement: R-VEC-005 (docs/requirements.md, added on branch `chore/tracing-embroidery-backlog`,
PR #257). It concretises R-VEC-002 ("combine multiple trace passes into one result").

## Problem
Today Trace takes the whole picture with one set of values. A picture with a sharp logo and a rough
paper texture, or a drawing whose title block needs another Threshold than its sketch, forces one
compromise. The only way around it is to crop the image in another program, trace the crops one by
one and move and merge the results by hand; the pieces then do not meet exactly at the seams.

## Goal
1. Select one image and one or more ordinary paths, press Trace, choose Inside or Outside, and the
   trace covers only that part, with edges that follow the maker's path exactly (slice 1).
2. Keep several such parts in the Trace tool, each with its own settings, and let one Create paths
   join them into a single result in which no area is covered twice (slice 2).

Field notes (to be re-checked by the implementer against the current manuals): Inkscape's Trace
Bitmap traces the whole selected image; the usual way to trace a part is to crop or clip the bitmap
first, and its SIOX option takes a shape drawn over the image as a hint for the foreground, not as
a stencil. LightBurn's Trace Image traces the selected image as a whole, and Ink/Stitch does not
trace. None of them traces through a stencil, none keeps two parts with different settings, none
joins parts so that the seams close. Better here: the maker's own paths are the stencil, positive
or negative, the edge follows the path exactly, parts have their own settings and the join leaves
no seam line and no doubled area.

## Terms
- **Mask path**: an ordinary closed path, compound path, rectangle, ellipse, polygon, star or linked
  clone that is selected together with the image. Its fill and stroke do not matter; an open path is
  closed by a straight line (as the Boolean operations of 0054 do). A group counts as the paths in it.
- **Region**: the area of all mask paths of one part together (overlapping mask paths are one
  union; a hole in a compound path is a hole of the region), clipped to the image.
- **Inside / Outside**: the mask mode. Inside (positive mask) traces the region; Outside (negative
  mask) traces the image minus the region.
- **Part**: a region with its mode and a full set of Trace settings (Proposal, see Slice 2).
  A part with no mask path is the **whole image**; only the first part may have none.
- **Join**: Create paths with more than one part. The parts become one result: one group, no area
  covered twice, same-coloured touching shapes merged, seams closed.
- **Stencil edge**: the border of the region. Traced shapes end exactly on it (Proposal, Frage 134 A).
- **Mask view**: the Mask view of 0089, pixels the tracer turns into shapes; with a mask the pixels
  outside the region are white.

Proposal, the operation in one line (Frage 134): "Trace with mask limits a trace to the inside or the
outside of the selected paths, which act only as a stencil and are not traced themselves; Join
turns several such traces into one result in which no area is covered twice."

## Slices
1. **Slice 1, one mask region per trace (Must for this story).** Select an image and mask paths,
   a Mask row with Inside / Outside, the trace limited to the region with an exact stencil edge,
   Create paths as today, everything else of the Trace tool unchanged. Works end to end for one
   image and one region; no part list, no join.
2. **Slice 2, parts and Join.** A Parts row with one chip per part, Add part, each part with its
   own region, mode and settings, one Create paths that joins the parts (Frage 132), a preview of
   the joined result. Parts live while the Trace tool is open (Frage 133).
3. **Later, own stories:** parts stored in the document for re-tracing (Frage 133 B); a Join
   command for finished trace groups; soft or feathered masks.

Slice 1 is the MVP cut. Each slice is tried by the customer on his own images.

## Proposals (not customer requirements; vetoable at the demo)
- How a mask is chosen: Select image plus paths (Frage 130 A). In the Trace tool a Shift+click on a
  path adds it to or removes it from the mask paths, so the maker does not have to leave the tool.
- The Mask row (Inside / Outside) sits in the Advanced block and is drawn only while mask paths are
  selected. The default is Inside, so the Trace bar stays as simple as today with Advanced closed.
  The mode is not remembered: every entry into the tool starts with Inside.
- Methods and analysis (Remove background, Pick colours, Analyse) read only the pixels of the
  region, so a background colour found in one part does not leak from another.
- Text and shapes recognition (0091, 0088 Part B) belong to the part that holds the centre of a
  recognised line or shape; a line whose centre is outside the region is neither set as text nor
  traced (its pixels are outside the stencil).
- At most 8 parts; chips are numbered by creation order; the order cannot be changed.
- Mask paths stay unchanged by Create paths (Frage 131 A).

## Acceptance criteria

Reference fixtures (committed, synthetic, `curvyo-ui-core` or `editor-wasm` tests):
- **M1**: 1000 x 1000 px, placed 100 x 100 mm (10 px per mm). White background, a black disc of
  300 px diameter centred at (250, 500) px, a black square of 300 px side centred at (750, 500) px,
  a black bar y = 480 to 520 px across the full width (it joins disc and square), and a grey
  (#808080) 100 px square at (900, 100) px. Mask rectangle R1: x = 0 to 50 mm, y = 0 to 100 mm.
- **M2**: M1 with the right half's white replaced by #D8D8D8 paper grey (a different background in
  each half), for the region-only analysis.
- **M3**: the 2000 x 2000 px poster of 0091's reference set, for text and shapes.

Area comparisons use the geometry kernel with a tolerance of 0.01 mm; "symmetric difference" is the
area of the exclusive-or of two outline sets.

Slice 1
1. Given an image and one closed path selected, when the Trace tool opens, then the Trace bar shows
   the preview limited to the region, and Advanced shows a Mask row with Inside selected. Given
   only images selected, then there is no Mask row and the result is path-for-path identical to the
   result before this story (regression).
2. Given M1 with R1 Inside and Brightness Threshold 50, when Create paths runs, then no node of the
   result lies more than 0.01 mm outside R1, and the result has the disc and the left half of the
   bar and not the square.
3. Given M1 with R1 Outside, then no node lies more than 0.01 mm inside R1, and the result has the
   square and the right half of the bar and not the disc.
4. Given criteria 2 and 3 at the same settings, then the union of the two results differs from the
   unmasked trace by a symmetric difference of at most 0.1 % of the unmasked area (the stencil edge
   adds no gap and no overlap).
5. Given a mask that is a compound path (a ring with a hole), Inside, then the content in the hole
   is not traced. Given two overlapping mask paths, then the region is their union and no area of
   the result is covered twice (pairwise overlap of result shapes at most 0.01 mm^2).
6. Given an open path as mask, then it is closed by a straight line and used; no refusal.
7. Given a mask that reaches beyond the image, then the region is clipped to the image. Given a
   mask that does not touch the image, then the muted row says "0 paths: the mask is outside the
   image.", the Trace button is dimmed and creates nothing.
8. Given the image rotated by 30 degrees and scaled to 60 mm width, and R1 transformed the same
   way, then criteria 2 and 4 hold with the same numbers.
9. Given M2 with Remove background on the right half (a mask over the right half, Inside), then the
   background colour used is #D8D8D8, no shape comes from the paper, and the grey square is traced
   (region-only analysis). The same method with no mask uses the dominant colour of the whole
   image, as before.
10. Given the Mask view with a mask, then pixels outside the region are white (checked at the
    pixel centres (100, 500) px for R1 Outside and (900, 500) px for R1 Inside), and pixels inside
    show the mask of the tracer as today. In the Overlay view the outline is drawn only inside the
    region.
11. Given M3 with a mask over the top half and Recognise text on, then only lines whose box centre
    lies in the region become text objects, no shape of the lines outside is traced, and every
    other criterion of 0091 holds inside. The same rule decides Recognise shapes.
12. Given Create paths with a mask, then the result is one group "Trace of ...", the mask paths are
    unchanged (same ids, nodes, style; Frage 131 A), one undo restores the document, and no success
    notice appears.
13. Given the Trace tool in use, when Shift+click hits a closed path, then it is added to or removed
    from the mask paths and the preview updates; Shift+click on empty canvas changes nothing.
14. Given a selection of a text, a second image or a sketch with the image, then the muted row says
    "Use Object to path for text; one image at a time." and the Trace button is dimmed; the image
    alone still traces.
15. Given M1 and the same settings run twice, then the results are identical (determinism).
16. Given a 2000 x 2000 px image and a mask of 500 nodes, then the stencil clipping adds at most
    0.1 s of thread CPU time to the trace (Proposal; reported, not blocking, a miss goes to
    `docs/technical-debt.md`).
17. Given the Mask mode set to Outside, when the tool is left and entered again, then the mode is
    Inside again, and the other Trace settings are remembered as before.

Slice 2
18. Given one image and mask paths in the tool, then the Advanced block shows a Parts row with the
    chip "1" (active) and an "Add part" button. Given 8 parts, then Add part is dimmed.
19. Given Add part, then a chip "2" is added and active, with a copy of the settings of the part
    before and no mask path; it contributes nothing (the chip is muted) until paths are selected
    for it; selecting image plus paths assigns them to the active part. Clicking a chip makes it
    active, loads its settings into the bar and selects its mask paths with the image.
20. Given M1 with part 1 = whole image at Threshold 40 and part 2 = R1 Inside at Threshold 70, then
    the result inside R1 differs from a single-part trace at Threshold 70 clipped to R1 by a
    symmetric difference of at most 0.1 %, and the result outside R1 differs from a single-part
    trace at Threshold 40 minus R1 by at most 0.1 %.
21. Given overlapping regions (Frage 132 A, last on top), then inside the overlap only the later
    part's shapes exist, and the earlier part's shapes are cut at the later part's stencil edge. In
    multicolour, shapes of different fill colours overlap by at most 0.01 mm^2 in the result.
22. Given M1 with part 1 = whole image at Smooth 20 and part 2 = R1 Inside at Smooth 80, then the
    bar across the seam at x = 50 mm is one closed outline: the result has exactly 1 path for the
    bar, disc and square together (they are joined by the bar), and no stencil edge appears as an
    extra line (no two result segments overlap along the seam for a length above 0.01 mm).
23. Given Multicolour in both parts, then the result has one compound path per distinct fill colour
    over all parts (same RGB merges, a different RGB stays separate), in one group.
24. Given Create paths with more than one part, then there is one group "Trace of ...", one history
    step and no success notice; one undo restores the document and the image.
25. Given a change of one setting in the active part, then only that part is traced again (a
    trace-call counter of 1 in a test) and the others come from the cache; the preview shows the
    joined result.
26. Given a stop, a failure or a cancel during several parts, then the Trace dialog behaves as in
    0025 and 0091 (criterion 25 there): one progress from 0 to 100 % over the whole job, never
    running backwards, no document change on cancel.
27. Given a mask path deleted from the document while the tool is open, then its part loses the
    path; if the part has none left, its chip is muted (it contributes nothing), no error appears.
    Given the first part with no mask path left, then it covers the whole image again.
28. Given the trash on a chip, then the first press arms it in soft red and the second after 400 ms
    removes the part (the inline confirm of 0081); chips renumber.
29. Given the tool left (Escape, S, another tool) and entered again, then there is one part with no
    mask unless paths are selected (Frage 133 A); nothing about parts is stored in the document or
    the settings.
30. Given Recognise text on in part 1 and off in part 2, then a line whose centre lies in part 2's
    region stays as traced paths of part 2, a line in part 1's own area becomes text.
31. Given 4 non-overlapping parts of one quarter each on a 2000 x 2000 px image, then the total
    thread CPU time of the preview is at most 1.3 times that of one whole-image trace
    (Proposal; each part is traced from the crop of its region; reported, not blocking).
32. Given a screen reader, then a chip is named "Part 2 of 3, inside, 2 paths, active" and the Mask
    row's segments are named "Inside" and "Outside" (UX notes may refine).
33. Slice 1 criteria 1 to 17 hold with a single part.

## Out of scope
- Soft, feathered or grey-level masks; a mask taken from the picture's own colours (that is Pick
  colours of 0089).
- Text objects, images or sketches as mask paths (Object to path first).
- Parts stored in the document and re-tracing from them (Frage 133 B, later story); named,
  reordered or presetted parts.
- Keeping the parts as separate objects instead of joining them (Frage 132 C is a question, not a
  deliverable until answered).
- Several images in one masked trace; parts on different images.
- Automatic detection of regions (text areas, faces, "mixed templates" of 0089's Out of scope).
- Per-part machine role, layer or colour for output (0027, 0063).
- A Join command for already created trace groups.
- Clip of the image itself or of other objects (0067).
- Success notices and hint texts.

## Open questions for the customer

Frage 130 (MASK-PICK): How is a mask chosen?
- A: Select the image and the paths together (Select tool, or Shift+click in the Trace tool), then
  Trace. No new tool, works with marquee and lasso.
- B: A pick mode in the Trace bar: a pipette-like "Mask" button, then click each path on the canvas.
- C: Both.
- Recommendation: A. It reuses selection, keeps the bar simple and needs no new mode.
- Default if unanswered: A.

Frage 131 (MASK-KEEP): What happens to the mask paths after Create paths?
- A: They stay exactly as they are (visible, still ordinary output geometry).
- B: They become guides (green, dashed, not output, as the carrier of a text on a path in 0080).
- C: They are deleted.
- Recommendation: A. A mask path can be a real cut outline (a coaster's shape); silently making it a
  guide would drop it from the job. The Guide button of 0046 does B in one click when wanted.
- Default if unanswered: A (one undo restores either way).

Frage 132 (JOIN-OVERLAP): How does Join treat overlapping parts?
- A: Last on top: a later part replaces the earlier ones inside its region; the result has no
  doubled area (right for a laser: nothing is cut or engraved twice).
- B: Union per colour: all shapes are kept, same-coloured ones merge, different colours may overlap.
- C: No resolution: each part stays its own group and the maker resolves overlaps with the Boolean
  operations of 0016.
- Recommendation: A.
- Default if unanswered: A.

Frage 133 (PARTS-KEEP): Are parts remembered for re-tracing?
- A: Only while the Trace tool is open; Create paths is the end of the parts.
- B: Stored in the document (mask paths by id, mode and settings per part), so a later Trace of the
  result's image offers the same parts. Needs a document-model change and an ADR.
- C: A copy of the parts' settings only, without the paths, in the app settings.
- Recommendation: A for this story; B as its own story if the customer misses it after trying.
- Default if unanswered: A.

Frage 134 (MASK-MEANING): Is this the meaning of the operation?
"Trace with mask limits a trace to the inside or the outside of the selected paths, which act only
as a stencil; the traced shapes end exactly on the path. Join turns several such traces into one
result in which no area is covered twice."
- A: Yes, as written (exact stencil edge, geometric clip of the traced shapes).
- B: The paths only choose which pixels are traced; the edge follows the pixels and stays rough at
  the border.
- C: Another meaning (the customer says which).
- Recommendation: A. B is simpler but the border of a path the maker drew precisely would be
  jagged, and two parts could not close their seam.
- Default if unanswered: A.

## UX notes
(To be filled in by ux-engineer before Ready. Open points: the Mask row, the Parts row and chips,
how the region is shown in the Overlay view, the Select bar's Trace button rule below.)

## Changes to accepted text (for the build)
- `docs/design-system.md`, "Trace tool": the Select bar's Trace button is shown while the selection
  is nothing but images, and now also for one image plus mask paths. The muted rows gain the
  messages of criteria 7 and 14.
- `docs/tracing-images.md`: new sections "Mask" and "Parts"; Advanced rows list gains the Mask row
  and the Parts row.
- 0089's Out of scope "regional processing of mixed templates" means automatic regions and stays.

## Links
Requirements: R-VEC-005 (docs/requirements.md, branch `chore/tracing-embroidery-backlog`, PR #257),
concretises R-VEC-002  PR: none yet
Related: 0025 (Trace tool, presets), 0089 (methods, Mask view, multicolour), 0091 and 0088 Part B
(Recognise text and shapes), 0016 (Boolean kernel), 0046 (guides), 0081 (inline delete confirm),
0067 (clipping, different thing)
