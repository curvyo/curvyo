# Laserable lettering: weld overlapping letters so that the laser cuts each line once

Status: Draft (a sketch for the customer; analysis first, then the smallest new tool; not Ready: text objects do not exist yet; Weld is a new command, decided 2026-10-17)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "We need a tool to make overlapping lettering 'laserable'. I do not know yet whether that is possible with existing tools. It is for normal vector editing and also for laser cutting."

## User value

As a maker who cuts or engraves names and slogans in a script or monoline font, I want letters that overlap or touch to become one clean outline, so that the laser does not cut the same line twice (burnt edges, double power), does not leave tiny islands that fall out, and the cut result holds together.

**What other tools do.** Inkscape: Path > Union on converted text, Path > Stroke to Path for single-line fonts, Path > Union welds; no tolerance for near-touching letters. LightBurn: Weld (Ctrl+W) joins overlapping shapes into one outline; "Convert to path"; "Close path with tolerance" (Optimization, auto-join). Cricut Design Space: Weld (the standard script-font workflow) and Attach. Silhouette Studio: Weld. Glowforge: nothing automatic. In these tools welding is one button; the maker's tolerance is a number in the optimisation settings.

## Findings: do the existing and specified tools already do it?

Analysed 2026-10-17 against the specs on `main` and the drafts. A letter that "overlaps" can mean four different things; each is a row.

| Case | Tool that could do it | Result | Verdict |
|---|---|---|---|
| 1. Closed glyph outlines overlap (script font: the ends of letters lie over each other) | Union of `0016` after the text is a path | Region union under the nonzero rule at kernel tolerance 0.01 mm gives one outline per connected word, holes kept | **Yes**, but only once text becomes paths (`0062` does not exist; Object to path is `R-EDIT-004` for text, unbuilt). It must be many objects selected, one click. |
| 2. Overlapping contours inside one glyph (variable and some script fonts) | Union, or Combine (`0035`) which uses the nonzero rule | Same region, but Combine keeps the extra contours as separate outlines (still double cut lines) | **Union yes; Combine no.** |
| 3. Single-line (monoline) fonts: open strokes that cross or overlap | `0016` refuses open paths. `0054` combines open paths into a compound path without welding. `0038` Offset does not offset open paths. There is **no stroke-to-path** spec (0016 names it as "its own operation") | Nothing joins them. The laser would cut a crossing twice | **No. Gap 1: stroke to path** (turn a stroked open path into the closed outline of its stroke) is missing. With it, the path becomes case 1. Note: for engraving, overlapping lines are often wanted as lines and need no weld; only cutting needs it. |
| 4. Letters that nearly touch (gap or sliver below about 0.1 to 0.2 mm) | Offset `0038` outset by g/2 then inset by g/2 closes the gap (a morphological closing). Union alone does not: a gap above 0.01 mm stays | Possible by hand in two drags with typed distances; corners round off unless Miter is chosen; slivers of hairline width remain as two outlines | **Partly.** Gap 2: no one-step "weld within a tolerance". |
| 5. Islands (the inside of O, e, a, 8, A, B, D, P, R; stencil use) | Difference/Union can add a bridge by hand (a small rectangle, then Union) | One manual rectangle per island, positioned by eye | **No automatic way. Gap 3: bridges for islands** (for stencils, so the inside is not a loose piece). Needed only for cutting a stencil, not for cutting letters out of a sheet (there the island is the waste piece). |
| 6. Cut order and double lines after welding | `0029` job preview | Not part of this entry | Out of scope |

Summary: **yes for closed overlapping outlines (cases 1 and 2); no for monoline fonts; no tolerance; no bridges.** The existing tools cover the heart of it; three gaps remain, in this order of value: stroke to path, weld with tolerance, bridges.

## The smallest new tool or workflow (Proposal)

1. **Weld**, one symbol button in the Select bar and in the Boolean toolbox group (next to Union), working on a selection of any number of closed and open objects: it first turns open strokes into outlines (using the stroke width, the fix for Gap 1), then unions everything, and closes gaps and slivers smaller than the tolerance field in the top bar (default 0.1 mm; fixed per use, not stored in the project). One history step "Weld". Result: one compound path with the outer outlines and the holes, selected, in the style of the lowest object.
2. **Bridges** (Gap 3): a second tool, **Bridge islands**: the Bridge tool shows each island of the selection in the marking style; a click on an island adds a bridge of the width set in the top bar at the nearest point of the outer outline (draggable before committing, as the Offset handle is); the Enter key commits (a bridge is a subtracted gap in the island's outline, so the island stays attached to the stencil). No automatic placement in the first version (Question 2).
3. Everything is a pure geometry function over paths, so it belongs in `geometry-core` or the Boolean crate and builds for `wasm32`. Cuttable tagging of fonts (`0062` criterion 12) can later use "Weld gives at most N pieces" as its test.

## Acceptance criteria (as far as clear)

1. Given a selection of closed paths that overlap or touch, then Weld gives the same region as Union (`0016`) and is not a second algorithm.
2. Given open paths with a stroke width, then Weld first expands them to closed outlines (round joins and caps by default, the stroke's own joins and caps honoured) and then welds; the stroke becomes part of the result; the fill of the result is the lowest object's fill.
3. Given a tolerance of t, then two outlines closer than t (and slivers narrower than t) become one region; the result differs from a plain Union by no more than t/2 anywhere on the outline except at the closed gap.
4. Given a tolerance of 0, then Weld equals Union plus stroke expansion.
5. Given the same input twice, then the same output node for node (as `0016` criterion 43).
6. Given a result with N separate outlines, then the tool shows "N pieces" quietly in the top bar so the maker sees whether the word is connected; no popup.
7. Given Bridge islands, then a bridge never crosses another outline and its width is a length with unit; undo is one step per bridge.
8. Given the options, then all of them are in the top bar (the tolerance, the bridge width); nothing is added to the right panel.
9. Given a selection of text objects (after `0062`), then Weld applies Object to path first, per object, without asking.

## Out of scope

- Detecting which fonts are cuttable (`0062` criterion 12).
- Kerning or joining letters by moving them (the maker spaces letters with the Text tool).
- Automatic bridge placement and stencil fonts; text on a path.
- Cut order, lead-in and kerf compensation (laser workbench, `0063`).

## Decided (customer, 2026-10-17)

- **WELD-SCOPE: a new command.** Weld is its own command with the definition in "The smallest new tool" (stacking order decides, covered parts of lower objects are removed, uncovered parts stay, one new object). The comparison with Flatten is Frage 1.

## Open questions (customer; each has a default)

Frage 1 (WELD-VS-FLATTEN): Is Weld a second algorithm next to Flatten (`0037`) and Union (`0016`)?
Finding: no. Flatten already does "trim every shape to its visible part and remove hidden shapes" by stacking order. Weld's trimming is the same computation. Weld differs in three places only: (1) it returns **one** compound object instead of one object per shape (each Flatten result keeps its own style and place in the stack); (2) it also accepts open and stroked paths (Flatten refuses open paths) and, later, text, by expanding strokes first; (3) it closes gaps smaller than a tolerance first. Against Union the difference is real: Union keeps only the outer outline of the combined area, Weld keeps the upper object's whole contour.
- A (default, recommended): Weld is a thin command on the **shared Flatten engine** (one kernel routine, the Fracture/Flatten code of `0037`), adding the join into one object, stroke expansion and the tolerance. Two buttons, one algorithm; `0037` is not changed, the extras live in the new spec.
- B: no Weld button; extend Flatten itself with the three extras (a "Join" switch in the bar). One command less, but Flatten's tooltip and its refusal of open paths change in an accepted spec.
- C: Weld as an independent implementation. Not recommended (two algorithms that must agree).
- Default if unanswered: A.

Frage 2 (WELD-BRIDGE): Bridges at all in the first version?
- A (default): no; first Weld, bridges follow when the customer cuts a stencil. B: manual Bridge tool (above). C: automatic bridges with a rule (minimum island size).
- Default if unanswered: A.

Frage 3 (WELD-STROKE): Is stroke to path wanted as its own command too?
- A (default): yes, a symbol button in the Select bar, because it is useful alone (SVG export of strokes to outlines for cutters that need fills). B: only inside Weld.
- Default if unanswered: A.

## Links

Requirements: R-FONT-009 (new); relates to R-FONT-002, R-EDIT-003, R-EDIT-007
Depends on: `0016-boolean-operations`, `0038-path-offset`, `0054-combine-open-paths`; text objects of `0062-fonts`
Depends on (engine): `0037-fracture-and-flatten`. Related: `0035-combine-and-break-apart`, `0029` (laser output)
ADRs: none new; geometry stays in the ADR 0003 kernel
PR: -
