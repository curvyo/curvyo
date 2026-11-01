# Plan for Tiling (0069)

Status: Draft plan (2026-11-01). Customer answers of 2026-11-01: TILE-LIVE **B**, TILE-HOME-UI A, TILE-SOURCE A, TILE-SPACING A, TILE-EXTRAS A, TILE-RESULT-GROUP A. Spec, ADRs and UX notes follow; the build starts after the customer releases the plan. Nothing is implemented.

**Consequence of TILE-LIVE B:** a tiling is a stored, reopenable object (source, options, seed): a new node kind, a `format_version` bump and a new ADR. Section 4 and the format line in section 3 change accordingly; the stored tiling is also the base `0071` pattern fill can build on.

`specification.md` is a sketch of 2026-10-17 with 11 criteria and one open question (TILE-LIVE). Since then linked clones (`0049`), clone overrides with own placement, own style and Spread colours (`0057`), linked text clones (`0080`), the Select bar with Align and Distribute (`0061`), click-to-create placement (`0052`), pages (`0059`) and the grid (`0090`) are built. This plan lists what the sketch already settles, what is missing, how the work is cut and what the customer decides first.

## 1. What exists and can be reused

- **Clones:** `Document::linked_clone(sources, conversions)` makes clones of paths and compound paths (a primitive is converted to a path first, silently, `0047`); chains up to 8; clones of texts are linked text clones (`0080`). Groups, images, sketches and orphan clones are **not clonable** (`CloneError::NotClonable`).
- **Overrides (`0057`):** own placement (a clone keeps its own matrix), own style per clone, and "Spread colours" (evenly spaced hues over selected clones, `SPREAD_COLOURS_COMMIT_LABEL`).
- **Select bar (`0061`):** the top bar with Align, Distribute and the Gap field while something is selected; the place where options of a selection live today.
- **Live preview pattern:** blue/black preview while adjusting, one commit on release/Enter, Escape cancels (`0009`, `0031`, `0052`); the history step label table in `curvyo-ui-core/src/history_names.rs`.
- **No random generator** exists in the workspace (the sketch solver is deterministic without one); a seeded generator is new.

## 2. What the sketch already settles

Grid and circle layouts (1), live preview only while adjusting (2), random ± variance for rotation, scale, offset and opacity, default 0 (3), a seed with a dice button and a portable generator (4), clones by default, copies as a switch (5, decided TILE-RESULT), colour variance through own style (6), "Keep source in place" (7), count limits 10,000 warn / 100,000 refuse (8), one history step "Tile" (10), nesting and clipping are other entries (11). Home: core (decided TILE-HOME).

## 3. What is missing or changed since the sketch

| Topic | Gap | Proposal / question |
|---|---|---|
| Where it lives | The sketch wants a tool in the rail with a card and a top bar; the app now keeps selection commands in the Select bar (`0061`). | Frage 19 (TILE-HOME-UI). |
| Re-open after commit | Sketch criterion 9 / Frage 1. | Frage 18 (TILE-LIVE). |
| Groups and several objects as source | Groups are not clonable; the sketch says "selected object(s)". | Frage 20 (TILE-SOURCE). Images and sketches tile as copies (Proposal). |
| Grid spacing | "Spacing (or total size)" is not defined: gap between boxes or centre-to-centre pitch. | Frage 21 (TILE-SPACING). |
| Offset rows (brick / honeycomb) | Not in the sketch; common for coasters and honeycomb fills. | Frage 22 (TILE-EXTRAS). |
| Variance kinds | Only random ±; Inkscape also has steps per row/column. | Frage 22 (TILE-EXTRAS). |
| Colour variance | Criterion 6 names it; Spread colours exists. | In scope: random or stepped hue through own style (Proposal, reuses `0057`). |
| Circle centre and radius | Not defined where the centre sits or how it is moved. | Centre on the source's centre shifted left by the radius, so the source is the first instance at 0°; a centre handle on the canvas; radius and centre also typed (Proposal). |
| Pages, grid, snapping | Not mentioned. | Instances ignore pages (they land where the layout puts them); the centre handle and a dragged grid origin snap like other handles (`0060`, `0090`) (Proposal). |
| Format | With TILE-LIVE A nothing new is stored: clones with own placement and own style exist. | No `format_version` change for A; B needs a node kind (ADR, `needs-customer`). |

## 4. Architecture (for the architect to confirm in `adrs.md`)

- **Layout, pure:** `tiling_layout(source_box, &TilingOptions, seed) -> Vec<Affine>` in `curvyo-ui-core` (new module `tiling/`): grid and circle placements, mirror rows/columns, offset rows, variance applied in a fixed order; deterministic, `f64` only.
- **Generator:** a hand-written PCG32 (O'Neill's `pcg32`, 64-bit state, fixed increment) in the same module with golden sequence tests, so every platform and the wasm build give the same numbers; no `rand` dependency.
- **Commit:** a `Document` command that creates the instances as clones (with own placement and, for colour variance, own style) or as copies, and groups them, in one commit "Tile"; refusal before the first write (count, non-clonable sources per Frage 20).
- **Preview:** the session computes the layout per change and draws instances as a light overlay from the source's outline (no document write), bounded by the count limits; a CPU-time test for 10,000 instances.
- **UI:** per Frage 19; options in the top bar, variance behind one button, seed field and dice, Result as switch.
- No new crate, no new dependency.

## 5. Slices

One PR (as the customer preferred for 0024), cut into commits: generator and layout (test-first, goldens of the seeded layouts), document command, session tool/preview, frontend bar, acceptance tests, budget. Before the build: spec rewrite with numbered criteria from the answers (PO), `adrs.md` (architect), UX notes (bar layout, handles, preview look).

## 6. Questions for the customer (each has a default)

Frage 18 (TILE-LIVE): Can a tiling be reopened and changed after it is committed?
- A: no, in this version; undo and tile again. Nothing new is stored.
- B: yes: a tiling is stored as its own object (source + options + seed) and can be reopened; also the base for `0071` pattern fill. A new node kind, a format change and an ADR.
- Recommendation and default: A.

Frage 19 (TILE-HOME-UI): Where is tiling started and set?
- A: a **Tile** button in the Select bar (next to Align/Distribute); it switches the top bar to the tiling options (Grid / Circle, numbers, variance, seed, Result as) with a live preview; Enter or a click on Apply commits, Escape cancels.
- B: its own tool in the left tool rail with the options in the top bar, as in the sketch.
- Recommendation and default: A.

Frage 20 (TILE-SOURCE): What happens with a group or several selected objects?
- A: every instance is a group of clones, one clone per object of the source (primitives become paths first, as for clones today); images and sketches inside become copies.
- B: only single paths, shapes and texts can be tiled; anything else is refused with a notice.
- Recommendation and default: A.

Frage 21 (TILE-SPACING): What does the grid spacing mean?
- A: the **gap** between the instances' boxes (0 = touching); a second field switches to pitch (centre to centre).
- B: only pitch, centre to centre.
- Recommendation and default: A (makers think in gaps and kerf).

Frage 22 (TILE-EXTRAS): Which extras go into this version?
- A: offset every second row or column by half a cell (brick / honeycomb) and stepped variance per row/column (in addition to random).
- B: only offset rows; variance stays random.
- C: none of them.
- Recommendation and default: A.

Frage 23 (TILE-RESULT-GROUP): How is the result organised?
- A: all instances (and the source, if it stays in place) in one new group "Tiling", in the source's layer.
- B: instances loose in the layer, next to the source.
- Recommendation and default: A.
