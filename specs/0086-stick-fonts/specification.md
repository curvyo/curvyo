# Stick fonts: OTF and TTF single-line fonts with a centre line

Status: Draft (parked 2026-10-26: customer wants to refine it first)
Priority: Should
Origin: Customer. Backlog note of 2026-10-23 ("OTF/TTF stick fonts ... parked for refinement"), split from `0076` Frage 19 and `0062` stage 2b.

## User value

As a maker who letters single-line signs with a font that is shipped as an outline file, I want its strokes read as one centre line each, so that the laser cuts each line once and not twice.

No acceptance criteria yet. The customer wants to refine this spec before it is scheduled.

## Summary of what is known

- **The problem.** Many "single line" OpenType and TrueType fonts (OTF "stick" fonts) draw each stroke as a thin closed outline that runs out along one side and back along the other. A cutter that follows every outline cuts each stroke twice. LightBurn and Fusion users report this for OneLineFonts and similar fonts (sources in `0076`, Research table).
- **What `0076` decided.** SVG fonts (Hershey Text v3, EMS) are read as open centre lines and are the first single-line fonts. OTF/TTF stick fonts with doubled-back closed outlines were a "next slice" with a de-doubling heuristic (Frage 19 SL-STICK, decided A, 2026-10-23). They are not in the first slice (`0076` out of scope).
- **The heuristic.** De-doubling finds the centre line of a thin closed contour (two sides that run close and parallel, joined at the ends). Wrong results are possible. It needs its own criteria and its own golden fixtures in `tests/fixtures/` before it is built (`0076` Research table).
- **Placement, customer decision (Frage 19, 2026-10-23).** Stick fonts do **not** appear in the normal font selection. The customer wants them only in a dedicated workbench, later, with `0063` (manufacturing workbenches). The lead noted that "stick" may mean embroidery (German "Sticken"); to be clarified when that workbench is specified. This is an open question for the refinement.
- **Scope of the epic.** Removed from `0062` (customer, 2026-10-26) and parked here.

## Open questions for the refinement (no defaults decided)

1. Where do stick fonts live: the dedicated workbench of `0063` only, or also the Fonts tab behind a tag?
2. Is "stick" the single-line sense (as here) or the embroidery sense ("Sticken")?
3. What does de-doubling do on a font whose outline is not doubled (a true single line)? Keep it unchanged, or always ask?

## Acceptance criteria

None yet.

## Out of scope

- Any de-doubling or import of OTF/TTF stick fonts until the customer schedules this spec.
- SHX and Hershey JHF files (`0076` out of scope).
- OpenType-SVG open-path layers (`0076` out of scope).
- Embroidery stitch fonts (`0063` territory).

## UX notes

None yet. Placement (question 1 above) decides where any UX is specified.

## Links

Requirements: R-FONT-002 (single-line group)
Depends on: `specs/0076-single-line-fonts/` (single-line text, the Single line tag), `specs/0074-text-objects/` (text objects, Object to path)
Related: `specs/0062-fonts/` (the epic it left), `specs/0063-manufacturing-workbenches/` (the likely home, Frage 19)
ADRs: none yet; a de-doubling heuristic in a `*-core` crate may need one (architect, when scheduled)
PR: none
