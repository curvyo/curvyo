# ADRs for page auto-layout

Architect, 2026-11-10; revised the same day after the customer's answers (Fragen 146 B, 147 A,
148 A, 149 B, and later 150 A, 151 B).

- [ADR 0051](../../docs/adr/0051-page-auto-layout-and-ownership-by-centre.md) (new, Proposed):
  the edge-shift reflow on a size change as a pure function over frames in
  `curvyo-document-core` (criteria 1 to 13, 32, 33), the drop push of a dragged page (22 to 24),
  the one-sided close of the hole on a delete (30, 35 to 41, §3a), ownership by the centre of the
  top-level object in `curvyo-ui-core::page_membership` for all three and for what a drag carries
  (14 to 21, 37), one batch under the triggering label (26, 27, 41), no format change (28), no
  layout on read or merge (12).
- [ADR 0024](../../docs/adr/0024-spatial-pages.md) §1, §5: frames in one plane, frame move with
  its content in one commit; amended by 0051 (a size change or a drop moves other frames and
  their owned objects).
- [ADR 0031](../../docs/adr/0031-sync-engine-relay-protocol-and-transport.md) 9a: one frame
  register; resize keeps the centre (criterion 1, Frage 147 A). Unchanged.
- [ADR 0042](../../docs/adr/0042-page-kinds.md) §4, §6: placements and fixtures page-local, so a
  moved laser page carries them by its frame write; the profile pick resizes about the origin
  corner and now runs the reflow (criterion 32). Its "laser page over a drawing page" overlap can
  no longer be made by a drop (criterion 24, Frage 150 A). §5: a deleted laser page takes its
  manufacturing node in the same commit, which the close joins (criteria 36, 41).
- [ADR 0043](../../docs/adr/0043-sketch-pages-content-by-ownership.md) §1, §3: the one membership
  read gains `owner_by_centre`; a moved sketch page carries only its sketch, clones held back
  (criterion 19); amended by 0051. §5: a deleted sketch page takes its sketch, its clones become
  paths, under its own label, which the close joins (criteria 36, 41).
- [ADR 0045](../../docs/adr/0045-clones-of-groups-texts-images-and-movable-placed-pages.md) §7:
  placed pages stay on their laser page by ownership and move with it; a moved source page does
  not write its placements (criterion 19).
- [ADR 0016](../../docs/adr/0016-clone-overrides.md) §2: `translate_page_objects` holds back every
  downstream clone (criterion 19).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one labelled commit, fieldwise
  restore (criteria 26, 27).

## Notes

- 2026-11-10: `set_page_position` is removed by PR #311; this story starts after that merge and
  needs no position command.
- 2026-11-10: the `0106` precedence (laser over drawing) is reused for a centre in two frames; the
  edit context itself keeps its reach rule (spec Out of scope).
- 2026-11-10: Frage 146 B replaces the push-only rule by the edge shift; the extent of an edge is
  taken from the frame that reaches farther out on that side, so a turn and its inverse pick the
  same pages (ADR 0051 options). The round trip is exact when the first step capped no pull and
  sent no page to the fallback row.
- 2026-11-10: Frage 148 A gives `move_page` the centre owner's contents; `Reach::Inside` loses its
  only product caller (`contained_page_objects`) and goes with this story.
- 2026-11-10: Frage 150 A: no modifier on the drop; `push_apart` has no bypass.
- 2026-11-10: Frage 151 B adds `close_gap` (ADR 0051 §3a): the chain behind the deleted page
  (its right edge in a row, else its bottom edge) moves in by the width or height plus the
  nearest distance, with the reflow's chains, cap and fallback; the pages before it never move,
  since a reflow about the centre would move them and leave a double distance. `delete_page` takes
  the caller's `PageContents`; the deleted page's own content is handled as today (`0059` 6,
  `0098` 16, `0099` 17, `0100` 24), none of it moves.
