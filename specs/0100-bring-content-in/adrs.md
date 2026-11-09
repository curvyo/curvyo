# ADRs for bring content in

Architect, 2026-11-09. Slice 3, part 1 of `0063` (concept A). Frage 150 A and 155 A assumed.

- [ADR 0045](../../docs/adr/0045-clones-of-groups-texts-images-and-movable-placed-pages.md) (new,
  Proposed, `needs-customer` for the document model and file format; accepted with the PR as
  0042 and 0043 were): legal origins (criteria 6, 8), the drawn subtree and its read (7, 9, 22 c),
  the containment cycle rule and `would_cycle` (12), Unlink and delete as copies (10, 11), own
  placement for every root and own style for outline roots only (1 b, 7 f, 9), movable placed
  pages (17 to 19), export (20), format 38 (21).
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §1, §3, §5, §6:
  amended by 0045 (note on its Status line). §2 (ancestor state), §4 (residue) and §7 unchanged.
- [ADR 0016](../../docs/adr/0016-clone-overrides.md) §1 to §4: amended by 0045 for picture roots
  and group transforms; §5 to §7 unchanged.
- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md) "Clones": an image may be an
  original (0045 §1).
- [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md) §6, §9 and
  [ADR 0042](../../docs/adr/0042-page-kinds.md) Consequences: a placement is moved, copied and
  kept on its laser page (0045 §7).
- [ADR 0043](../../docs/adr/0043-sketch-pages-content-by-ownership.md) §1: the membership read
  decides "member of a laser page" (criteria 1, 5) and Active page only (20 a); §6
  `place_linked_clone` is generalised by note 1.
- [ADR 0022](../../docs/adr/0022-texts-on-carriers-and-text-clones.md) §4: `text_source` text
  clones unchanged; a text carrier stays an outline root or a clone of one (a picture-root clone
  carries no text).
- [ADR 0030](../../docs/adr/0030-stored-tiling-nodes.md): unchanged (images in tilings stay
  copies); tiling apply calls `would_cycle` (0045 §4).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): every drop is one step; the drags
  write nothing before the release.

## Feature-local decisions

1. 2026-11-09: **Commands and history labels.** `place_linked_clones(originals, parent,
   translation)` in `curvyo-document-core` (0043 §6's body for a list: one commit, each clone
   with `own_placement` and the translation, refused whole on a depth-8 original), label
   `place_linked_clones` ("Place linked clones"); `place_sketch_clone` keeps its label.
   `place_page(page, source, matrix)`, label `place_page` ("Place page"). Moves, copies, Unlink
   and transforms of placements keep their existing labels. The `0020` criterion 41 label guard
   gets the two new entries.
2. 2026-11-09: **Budgets of criterion 22 confirmed**, thread CPU in
   `curvyo-editor-wasm/tests/cpu_time/`: (a) 16 ms per pointer move, measuring the session's move
   call and the frame's view; the ghost is the selection's snapshots taken at the drag's begin and
   only translated; (b) 16 ms, the row drag's ghost (frame and member outlines) resolved once at
   `row_drag_begin` and only translated (as `0099` note 2); (c) 300 ms read and 150 ms node edit,
   the `0049` numbers, met by sharing each member's tessellation across clones (0045 §3). If (c)
   fails, the next step is an incremental re-map of only the changed member per clone, measured
   first.
3. 2026-11-09: **Session structure** (architecture review finding 1). The cloning drop gets its
   own module `curvyo-editor-wasm/src/session/clone_drop.rs`, beside `row_drag.rs`:
   - one `Session` field `clone_drop: Option<CloneDrop>`; `CloneDrop`'s fields are private to
     the module, and the session calls only `CloneDrop::begin(selection, read)`,
     `update(point, modifiers, pages, membership) -> DropAnswer`, `ghost()` and `release()`;
   - the decision is one pure function there, `drop_mode(selection_on_laser_page, target_kind,
     alt, ctrl) -> DropMode { Clone, Move, Copy, Plain }`, with a table test of criteria 1 to 5;
   - the Select move stays in `curvyo-ui-core::select_tool`; it gains one method that ends the
     in-flight move and returns its snapped translation without committing, which the release
     calls for `DropMode::Clone`. Nothing is added to `select_tool.rs` (2,707 lines) beyond that
     method; new code goes to `select_tool/move_drag.rs`;
   - `row_drag.rs` gets `RowDragSource::DrawingPage { page }` and its drop calls `place_page`;
     the `manufacturing.selected_placement` field goes, a placement is in the one selection
     (0045 §7).
4. 2026-11-09: **Drag answer contract** (finding 3). `DropBadge` is one enum for both drags,
   adding `LinkedClone`, `Move`, `Copy`, `PlacedPage`, `OnlyOnLaserPage`. One decoder,
   `frontend/src/lib/dragAnswer.ts`, parses the row-drag and the clone-drop answers, checks every
   field and the badge against the known set, and returns `null` for anything else (no badge
   drawn); `usePageRowDrag.ts` drops its `as Answer`. Badge words are an exhaustive
   `Record<DropBadge, ...>`. Contract: `curvyo-editor-wasm/tests/drag_answer_contract.rs`
   serialises one answer per badge (an exhaustive `match`, so a new variant fails to compile
   until listed) and compares it with `frontend/src/lib/fixtures/drag-answers.json`;
   `dragAnswer.test.ts` decodes the same file and rejects an unknown badge and a missing field.
5. 2026-11-09: **Commands that need one outline** (Boolean, Combine, Offset, Stroke to path, text
   carrier, dimension slots, node snapping) treat a picture-root clone as they treat its root's
   kind: a group-root clone like a group, a text-root clone like a text, an image-root clone like
   an image (Proposal, spec issue 4). Snapping to its box and to its drawn path leaves' anchors
   works as for those leaves.
6. 2026-11-09: **Not offered on picture-root clones:** Own style (0045 §6) and Guide on/off (a
   guide flag on such a clone is residue, never drawn as a guide). Kept: Own placement, Unlink,
   Select original, Select clones.
7. 2026-11-09: **Placed-page geometry.** Drawn point = laser frame origin + matrix × (point −
   source frame origin), as ADR 0032 §6. The drop's matrix is the translation that puts the source
   frame's centre on the snapped pointer; Place page uses the laser frame's centre. Off-page test:
   the transformed source frame box against the laser frame, overlap area greater than 1e-9 mm²
   (an edge-only touch is off the page).

## Spec issues (for the product owner)

1. **Criterion 12 is too narrow.** A clone C1 of G1 inside G2 and then a clone C2 of G2 moved into
   G1 is not "inside its own original", but each draws the other forever. Proposal: "when any
   command would make a clone draw itself (inside its original, or inside a group its original
   draws)", same notice; add the two-group case to the test.
2. **Primitives as originals** (criterion 6, not converted) are missing from criteria 9 and 10
   and the Changes table. Proposal: a primitive root behaves as a path root (Style row kept, own
   style allowed); Unlink gives a primitive when the matrix keeps it one, else a path (0045 §5).
3. **Two kinds of text clone.** `0080` already has text clones (`text_source`: shared string,
   own carrier, no chains). Criterion 8's Linked clone of a text is the other kind (a picture
   under a matrix, chains, Placement row). Proposal: say so in criterion 8, keep both, and name
   them apart in the UX notes ("Linked clone" versus the `0080` command's name).
4. **Operations on picture-root clones** (Boolean, Offset, Stroke to path, carrier, dimensions)
   are not specified. Proposal: note 5.
5. **Criterion 18** names only the drag. Typed X/Y, arrow keys, align and distribute can also
   push a placed page off; Proposal: every transform is refused whole with the same notice.
6. **Criterion 20 (c)**: texts and images are written with a `transform`, so a text's stroke in
   a scaled clone scales in the file while the canvas keeps widths (criterion 9). Proposal: (c)
   speaks of outlines; stroke widths of text in scaled clones are a known difference.
7. **Criterion 9**: say that the original group's own Hide does not hide its clones (ADR 0015 §3;
   only members hidden below the group are left out), so the test does not expect otherwise.
