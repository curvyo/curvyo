# ADR 0043: Sketch pages: content by ownership, a fixed sketch, Convert and Delete

**Status:** Accepted 2026-11-06 (customer accepted PR #287; document model; Frage 157 A, 158 A). Design by the customer (concept A of
`specs/0063-manufacturing-workbenches/concept-output-pages.md` §2.1, §2.7, §9); Frage 157 A and
158 A are assumed (their defaults); the read and write rules below are the architect's
**Proposal**. Amends, by notes on their Status lines: [ADR 0042](0042-page-kinds.md) §1, §3
(`kind` written a second time by Convert, Sketch to Drawing only) and §4 (a sketch page's
content); [ADR 0024](0024-spatial-pages.md) §1, §5 (membership and frame move of a sketch
page); [ADR 0026](0026-parametric-sketch-nodes-and-workbench.md) "Page ownership revision" (page
deletion deletes the owned sketch) and "Original transforms" (the original is never transformed
as a whole). Spec: `specs/0099-sketch-pages`.

## Context

ADR 0042 made Sketch a fixed page kind that owns at most one sketch through the claim of ADR
0026. Membership is still by position for every page (ADR 0024 §1): a rectangle drawn inside a
sketch page's frame is that page's content, a frame move carries it, and the sketch itself can be
selected and transformed on a drawing page it overlaps. The customer wants a sketch page to hold
only its sketch, the sketch fixed to its page, and the sketch used elsewhere only as linked
clones. Five questions: how membership reads, who may move the sketch, what Convert to paths does
to the kind, what deleting the page does to clones, and whether the file changes.

### Options: a sketch page's members

- **A. One kind-aware membership read: a sketch page's only member is its live owning sketch; that
  sketch is a member of no other page; every other object ignores sketch frames (chosen).**
  Derived on read, nothing stored, one function every consumer calls.
- **B. Position membership as now, and each consumer (frame move, export scope, placement
  members, fit, picking) filters sketch pages itself.** Five copies of one rule that drift apart;
  today they already compute overlap in four places. Rejected.
- **C. A stored "pasteboard" flag on objects over sketch pages.** State that goes stale on every
  move and merges badly. Rejected.

### Options: keeping the sketch fixed

- **A. The selection is the gate: the owned sketch is not pickable, not in a marquee or Select
  all, and not selectable from Layers; only the frame move (and ADR 0042 §9) writes its placement
  (chosen).** One predicate, read in the session where selections are built.
- **B. A refusal in every document-core transform command.** Move, rotate, scale, skew, flip,
  align, distribute, nudge and the fields each take node lists from callers and would each need a
  page read. A placement written by an older build or a merge is harmless (the sketch is drawn
  where its placement says). Rejected.

### Options: Convert to paths on a sketch page (Frage 157)

- **A. Rewrite `kind` from `"sketch"` to `"drawing"` in the conversion commit (chosen, 157 A).**
  The page stays where and what size it is; the node keeps its id, so clones follow the paths.
- **B. A new drawing page receives the paths (157 B).** No kind write; the sketch moves by the
  origin difference with clones held back (the ADR 0042 §9 transform), the sketch page keeps an
  empty sketch. Needs no amendment of ADR 0042 §3.
- **C. No Convert on the sketch page (157 C).** Only Unlink clone makes paths; the existing
  conversion stays for ownership-conflict objects. Needs no amendment either.

### Options: deleting a sketch page with clones (Frage 158)

- **A. The page and its sketch go in one commit; the sketch's removal runs the `0049` criterion 31
  clone repair (chosen, 158 A).** No new repair path.
- **B. Refused while clones exist** (`PageError::SketchInUse`, a new error, no format change).
- **C. The page goes, the sketch becomes paths where it lay (the existing conversion), clones stay
  linked.** Leaves paths on the pasteboard with no page.

## Decision

1. **Membership** (amends ADR 0024 §1, ADR 0042 §4). One pure function in `curvyo-ui-core`
   (`page_membership.rs`) answers "the members of page P" and "the pages of object O" over the
   page list, the resolved bounds and the sketch claims. A Sketch page's only member is its live
   owning sketch (`Document::page_sketch`), wherever its outline lies; that sketch is a member of
   no other page. For every other object, Sketch frames do not exist: it belongs to the drawing
   and output pages it overlaps (ADR 0024 §1 test), and to none, the pasteboard, otherwise. A
   sketch that loses a concurrent claim (ADR 0026 conflict object) is an ordinary object under
   this rule. The frame move, Fit page, the export's Active page only scope (`0024` criterion
   130), placement members (ADR 0032 §6) and the press-to-activate rule call this function; none
   keeps its own overlap test.
2. **The fixed sketch** (amends ADR 0026 "Original transforms"). `Document::is_fixed_sketch(id)`
   is true for the live owner of a live Sketch page. The session leaves such a node out of
   `pickable_objects`, marquee, Select all and Layers selection in every workbench; the Sketch
   workbench never picks drawing objects (criterion 3). The sketch's placement is written only by
   decision 3 and by the ADR 0042 §9 migration. This closes the rotation of an original that ADR
   0026 allowed; ADR 0026's pending original-scaling question becomes unreachable (no command
   transforms an original) until a story reopens it.
3. **Frame move and resize** (amends ADR 0024 §5). `move_page` on a Sketch page is called with
   exactly `[owner]` (or nothing before the lazy first edit) and the existing clone-holding
   translation (`translate_page_objects`, `Hold::Whole` for every downstream clone, own placement
   or not); objects over the page are not passed. The per-user "frame alone" switch does not
   apply to Sketch pages. `resize_page` already writes the frame only; the sketch is not
   re-centred.
4. **Convert to paths** (amends ADR 0042 §1, §3; Frage 157 A). `convert_sketch_page(page, ids)`
   in `curvyo-document-core`, one commit labelled `convert_sketch_page`, a document-level
   operation like other page labels: the existing `convert_sketch_to_path` body (same node id,
   style, placement; claim cleared) plus `page.<id>.kind = "drawing"`. It is the only writer of
   `kind` after creation, only from `"sketch"` to `"drawing"`, only for the page whose live owner
   is converted. Converting an ownership-conflict object keeps `convert_sketch_to_path` and writes
   no kind. Undo and redo restore the register field by field (ADR 0014). A merge in which a peer
   kept editing the sketch can leave a drawing page with an unowned sketch: that is ADR 0026's
   visible conflict object, repaired by the existing surfaces.
5. **Delete** (amends ADR 0026 "page deletion retains global artwork" for Sketch pages; Frage 158
   A). `delete_page` on a Sketch page marks the page deleted and removes its live owner in the
   same commit, labelled `delete_sketch_page`, through the ordinary object removal, so `0049`
   criterion 31 unlinks or re-points every clone and undo restores them as clones (`0049`
   criterion 36). Conflict objects and drawing objects over the page are not touched. A clone a
   peer creates concurrently becomes an orphan and is removed by `0049` criterion 35.
6. **Placing a clone.** `place_linked_clone(original, parent, translation)` in
   `curvyo-document-core`: the existing linked-clone write plus `own_placement` on and the
   translation, one commit labelled `place_sketch_clone`. No new register.
7. **Output.** Export needs no sketch-page code beyond decision 1: a whole-plane export writes
   every page as an `inkscape:page` and every object at its coordinates (`0024` criterion 128),
   the sketch by `0024` criterion 138, clones by criterion 59; Active page only takes its scope
   from decision 1, so a sketch page exports its sketch only and a clone whose sketch is out of
   scope becomes a path. Construction geometry, dimensions and symbols are never written (ADR
   0026).
8. **Format.** No `format_version` bump; main stays at 37. Every write uses existing registers
   and existing values (`kind` already reads `"drawing"`, the claim, `clone_link`,
   `own_placement`, `deleted`, the node's type); membership and fixedness are derived. A format-37
   build reads a 0099 file without loss; it would only show objects over sketch pages as members
   and let the sketch be moved. Golden `curvyo-document-core/tests/fixtures/sketch_pages_v37.curvyo`
   (criterion 19).
9. **Crates.** No new crate, no new dependency. `document-core`: `is_fixed_sketch`,
   `convert_sketch_page`, the Sketch branch of `delete_page`, `place_linked_clone`. `ui-core`:
   `page_membership.rs`. `editor-wasm`: pickability, the frame move list, the page-row drag
   (`specs/0099-sketch-pages/adrs.md` note 4).

## Consequences

- Two membership kinds now exist: by position (drawing, output own items) and by ownership
  (sketch, output placements). Every new consumer of "what is on this page" must call decision
  1's function; a review checks for new overlap tests.
- The kind is no longer written strictly once. The rule stays narrow (one command, one
  direction); a general kind change (`0104`) needs its own ADR.
- Drawing objects over a sketch page are visible but belong to no page, so they never reach a
  page-scoped output. The maker sees them on the canvas, where the spec accepts that without a
  notice.
- With 157 B or C, decision 4 shrinks (B: `add_page` plus the held-back move, no kind write; C:
  removed) and ADR 0042 §3 stays as accepted. With 158 B, decision 5 becomes a refusal; with 158
  C, it calls the conversion. None of them changes the format.
