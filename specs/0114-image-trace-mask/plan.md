# Plan for the image trace mask (0114), slices 1 to 5

Branch `story/image-trace-mask`, on `main` at format 44 (target 45, ADR 0061). Built on
`specification.md` (criteria 1 to 25), `adrs.md`, ADR 0061 and the UX notes (to come; slices 2
to 5 need them, slice 1 can start on Tracyo's ux-notes §1 to §5 for the Rectangle). Spec defaults
apply: CLONE-MASK A, MASK-SETTINGS A, SYNC-MASK A, no ellipse, ASSET-SWAP drop. Author: architect
(the implementer refines task order in this file before building). One PR for the story, opened
when slice 5 is done; slices 1 to 4 are commits on this branch that the customer can try from the
branch build, each ending with the gate of its crates; slice 5 runs the full gate of `CLAUDE.md`
§7. Tests first for every core module.

## Slice 1: stored rectangles end to end (model, format, sync, history, Rectangle tool)

The customer draws rectangles on a picture, closes the Trace tool, reopens the file, and the
rectangles are back; undo works; two peers keep both rectangles.

1. `curvyo-document-core/trace_mask_codec.rs`: keys and fields of ADR 0061 §1, ids, order, read
   with damage (§7), write; `Document::trace_mask(image)`. Tests: round trip, sorting by
   (order, id), every damage case. Criteria 12, 22.
2. `trace_mask_commands.rs`: draw rect, move, resize, delete, clear, mode, add part, remove part
   (rect only), part `0` on the first gesture, dangling sweep, labels. One commit each. Tests:
   one step per command, undo/redo, image delete and undo restores the mask. Criteria 16, 17, 20.
3. Sync tests (two `LoroDoc` peers): concurrent new regions both kept, concurrent first gesture
   shares part `0`, same-region edits converge, remove part against draw, byte-identical
   serialisation. Criterion 18.
4. `image_codec::write_image` asset-change drop; Duplicate and Ctrl drag copy carry the mask (test
   on `copy_subtree`). Criteria 14 (Duplicate part), Frage 5.
5. `CURRENT_FORMAT_VERSION` 45 with its doc block; goldens `trace_mask_v45`,
   `trace_mask_damaged_v45`; older goldens open with no mask; `document.json`. Criterion 22.
6. `curvyo-ui-core/trace_mask.rs` and `TraceRegion::of_regions` (Union, then Difference, one
   operand per region); `trace_region_edit.rs` rectangle half (drawn, hit, handles, resized,
   moved, pixel snap). Tests: R1 on M1 equals 0094 criterion 2 and 3 results; permutation of
   regions gives equal bytes; rotated and skewed placement. Criteria 2, 3, 8, 9, 11, 13.
7. `curvyo-ui-core/trace_part_settings.rs` with part `0`'s settings written by the first gesture
   (read back in slice 3 for the UI). Criterion 12 (partly).
8. Session: `TraceMaskTool` struct, Rectangle tool input and overlay, parts read from the document
   (mask paths still selectable in parallel until slice 4), clips from stored regions into the
   existing worker path. Exports of ADR 0061 §9 for Rectangle, Subtract, mode, clear.
9. Frontend: Rectangle and Subtract toggles added to the existing Mask group (layout move in
   slice 4), status texts. Criteria 2, 3, 8, 19.

## Slice 2: Lasso

1. `trace_region_edit.rs` lasso half: corners, rubber band, close on first point, double click
   (no extra corner), Enter, auto-close on Esc, tool change, leaving Trace, opening another image;
   under 3 points dropped; Backspace removes the last point; corner drag; hit by nonzero
   containment. Criteria 4, 5, 6, 7.
2. Commands `draw_trace_lasso`, `move_trace_lasso_corner`; the lasso in `trace_mask.rs`
   (self-crossing by nonzero). Criteria 10, 17, 23.
3. Session input and overlay for the lasso (handles per corner, close target); Ctrl+Z drops an
   unfinished lasso (Proposal). Frontend Lasso toggle and icon from the UX notes.
4. Tests with L1 and L1c on M1: equal stored geometry and trace; Inside within 0.005 mm of the
   slanted side; Inside plus Outside gap and overlap under 0.01 mm². Criteria 5, 10.

## Slice 3: parts persist with their settings

1. Parts from the document in the session: Add part and Remove part write the document; chips
   from stored parts; the first part active on entry; leaving the tool and Create paths no longer
   end parts. Criteria 12, 19, 20.
2. Frontend reads `trace_part_settings(index)` and writes `set_trace_part_settings` at the end of
   a field edit or slider drag (`edit_trace_settings`); re-reads on document version change so
   undo/redo while the tool is open reshows values. Ranges read from Rust. Criteria 12, 17.
3. Reopen test: two parts, different settings, preview and Create paths equal before and after;
   sync of the same. Criteria 11, 12, 18.

## Slice 4: old pick workflow removed, Mask group in the Parts row, clones and clipboard

1. Delete `trace_mask_pick*.rs`, Shift+click, mask paths as trace input, `TraceSelection::Mixed`,
   `TRACE_MIXED`; `trace_images` takes images and image-root clones, ignores others. Rewrite or
   delete `traceMaskPick.test.ts`, `acceptance_0094_pick_*`, the Shift+click and `TRACE_MIXED`
   cases of `acceptance_0094_tester`, `acceptance_0089_*`, `acceptance_0025_slice3_tester`,
   `traceBar.test.ts`; the `pick-mask` cursor. Criteria 1, 21.
2. Layout: Mask group (Rectangle, Lasso, Subtract, Inside | Outside, Clear mask) into the Parts row
   after the chips, Add part and trash; first row without it (narrower). Test that fails when the
   Mask group is in row 1; re-captured reference screenshots. Criteria 1, 25.
3. Clones: Trace on an image-root clone shows and edits the root's mask through the composed
   matrix; locked root or clone disables the region tools; Unlink keeps a copy (golden
   `trace_mask_clone_v45`). Criterion 15.
4. Ctrl+C / Ctrl+V memo with the fingerprint; paste into a second document of the same instance.
   Criterion 14.

## Slice 5: limits, docs and the full gate

1. Budget test: 8 parts of 20 mixed regions, edit and retrace without a panic, no frame over
   50 ms (thread CPU time, `tests/cpu_time/`); invalid geometry gives the notices. Criterion 24.
2. Determinism: Create paths twice and after reopen, byte-identical serialisation. Criterion 11.
3. Docs: `docs/tracing-images.md` Mask, Parts, Create paths and the bar text; `docs/design-system.md`
   rows; "Replaced by 0114" notes in the 0094 spec and ux-notes; `specs/README.md` row; ADR 0017
   and 0045 notes pointing to ADR 0061; the dated note on `0094` adrs.md.
4. Full gate (`CLAUDE.md` §7), tester pass (format and document model: full pass, Opus).

## Risks

- **Removal touches many tests** (0025, 0089, 0094 suites). Rewrite per criterion, never skip;
  keep the clip, stack and join tests unchanged as the safety net of the pipeline.
- **Settings ownership moves from the frontend to the document.** The frontend's per-part React
  state must become a view of the session's settings; a missed path (presets, Analyse, region-only
  analysis applying automatic values) would write steps without a gesture or fail to reshow on
  undo. Rule to test: no document write without a gesture (criterion 19 compares before and after).
- **Format number collision** with `0112` (ADR 0057) and `0113` (ADR 0059), which claim 45 and 46
  provisionally: the number is taken at merge; the later stories renumber their goldens.
- **Clipboard fingerprint** fails for pictures with partial transparency on browsers that
  premultiply on the way: the paste then has no mask (safe failure, stated).
- **Clones of images were not traceable before**: the trace pipeline's placement must come from
  the clone's composed matrix everywhere (preview, wash, crop, Create paths); one test per path.
- **Lasso hit testing and handles** have no Tracyo precedent; the UX notes decide radii and the
  close target before slice 2.
