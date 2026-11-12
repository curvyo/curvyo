# ADRs for the image trace mask

Status: Proposed (architect, 2026-11-12). File shape, sync and history: [ADR 0061](../../docs/adr/0061-image-trace-mask-in-the-project-file.md) (`needs-customer` by rule; the behaviour is the customer's decision of 2026-11-12, quoted there, and the spec defaults Frage 1 to 5: CLONE-MASK A, MASK-SETTINGS A, SYNC-MASK A, ELLIPSE-REGION later, ASSET-SWAP drop). Decisions marked **(Proposal)** are the architect's and vetoable at the demo. No new crate, no new dependency, no trait, no generic.

## Depends on

- [ADR 0061](../../docs/adr/0061-image-trace-mask-in-the-project-file.md): registers `trace_part.<part>`, `trace_settings.<part>`, `trace_region.<part>.<region>` on the image node; unit-square coordinates; part `0`; per-key merge; labels; copies, clones, asset swap; format 45; goldens.
- [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md) §1: the image's unit square and `image_placement`; amended by ADR 0061 §1 and §8 (asset swap drops the mask).
- [ADR 0045](../../docs/adr/0045-clones-of-groups-texts-images-and-movable-placed-pages.md) §1, §3, §5: picture-root clones, composed matrix, Unlink and repair as a copy (carries the mask).
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §3 and [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §3, §4, §7: merge per register, opaque plain values, never repaired on read, the bump rule.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one commit per gesture is one step; revival restores a deleted image with its keys.
- [`0094` adrs.md](../0094-masked-composite-trace/adrs.md) decisions 1 to 5 and the notes of 2026-11-04 and 2026-11-05: clip in the worker, stacking, join, crop, region-only analysis, wash. Kept; only the source of a part's region changes (stored regions instead of selected paths). Its "nothing new in the document" and "parts are tool state" are replaced by ADR 0061.
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md): the kernel, unchanged (Union, then Difference; nonzero per operand).

## Feature-local decisions

- 2026-11-12: **Coordinates.** Regions are stored in the image's unit square, not in picture pixels as the spec's Terms say; the behaviour is the same (corners snap to whole pixels of the working image while drawing), but a downscaled Ctrl+C copy and a re-decode cannot shift a region. Not a customer question (representation only).
- 2026-11-12: **Copy ids.** Region and part ids are local to their image, so Duplicate and Ctrl drag copy keep them (the existing `copy_subtree` copies every key); the lead's "new ids" is not needed. Editing a copy writes only the copy's node.
- 2026-11-12: **Ctrl+C / Ctrl+V carry the mask inside one app instance** by a fingerprint of the working image kept with the mask in the page's memory (ADR 0061 §8) **(Proposal)**. From another app or another instance only the picture arrives. Criterion 14 "also into a second document" holds inside one instance.
- 2026-11-12: **Linked clones become traceable** when their root is an image (CLONE-MASK A); Trace on the clone edits the root's registers. Locked root or clone: region tools and mask commands off, tracing stays **(Proposal)**.
- 2026-11-12: **Settings writes.** A slider drag or typed field writes one `edit_trace_settings` step at its end; the live values during the drag are tool state. Setting and mode changes on an untouched image stay tool state until the first region or Add part writes part `0` with them.
- 2026-11-12: **Ctrl+Z while a lasso is being drawn** drops the unfinished lasso and undoes nothing else **(Proposal, for the UX notes)**; Esc still closes and stores it (spec Behaviour 3).
- 2026-11-12: **More than 8 parts** only arise from a merge; all are shown and traced, Add part stays dimmed **(Proposal)**.
- 2026-11-12: **For the PO, criterion 16 wording:** "no mask data remains" is true of the current state and `document.json`; the oplog keeps deleted values until a wipe (ADR 0014 §9), as for every deleted object. Suggest "the reopened file's image list and `document.json` hold no mask of it".
- 2026-11-12: **For the PO, criterion 22:** the number is 45 if this story merges before `0112`/`0113` (ADR 0057 and 0059 then move to 46 and 47); otherwise the next free number at merge.
- 2026-11-12: **UI migration.** The Mask group (Rectangle, Lasso, Subtract, Inside | Outside, Clear mask) moves into the Parts row of Advanced behind the chips, Add part and the trash, as Tracyo's ux-notes §1 to §3; the first row loses it and gets narrower (the UX notes give the new first-row width and the Parts row wrap at the narrowest supported window). Done in one commit with the removal of Pick mask paths (plan slice 4); slices 1 to 3 add Rectangle, Lasso and Subtract to the existing group so each slice stays demoable.
- 2026-11-12: **Asset swap (Frage 5):** no command changes an existing image's asset today except the tiling instance update; the drop rule lives in `image_codec::write_image` and runs inside that command's own step.

## Crate placement (no new crate)

- `curvyo-document-core`: new `trace_mask_codec.rs` ("the stored shape of an image's trace mask": keys, plain-value read with damage, write), new `trace_mask_commands.rs` ("the trace mask commands of one image, one commit each": draw, move, resize, move corner, delete, clear, mode, add part, remove part, settings, dangling sweep, labels); `image_codec.rs` (asset change drops `trace_*` keys); `image_model.rs` (`ImageSnapshot` unchanged; the mask is read on demand by `Document::trace_mask(image)`, not on every object read); `document.rs` (`CURRENT_FORMAT_VERSION` 45 and its doc block); `document.json` writer.
- `curvyo-ui-core`: new `trace_mask.rs` ("a stored trace mask as kernel regions in document mm"); new `trace_part_settings.rs` ("the stored trace settings of one part with their ranges and defaults"); new `trace_region_edit.rs` ("the geometry of drawing, hit testing and editing a rectangle or lasso region in picture pixels", ported from Tracyo's `region_edit` and extended by the lasso); `trace_region.rs` gains `TraceRegion::of_regions`; `history_names.rs`, `step_scope.rs` (labels of ADR 0061 §6).
- `curvyo-editor-wasm`: `session/trace_parts.rs` (parts read from the document; one feature-owned `TraceMaskTool` struct for active part, tool, Subtract, selection, drag, lasso in progress), new `session/trace_mask_input.rs` (pointer and keys), new `session/trace_mask_overlay.rs` (outlines, handles, rubber band, wash), `session/trace.rs` (`trace_images` with image-root clones, no `Mixed`), `session/image_clipboard.rs` and the import (fingerprint and mask); delete `session/trace_mask_pick*.rs`, the mask-path half of `trace_region.rs`; `wasm_*` exports of ADR 0061 §9.
- Frontend: Mask group in the Parts row (`TraceBar`, Advanced), settings read and written through the session per part, removal of Pick mask paths, `traceMaskPick.ts`, the `pick-mask` cursor and `TRACE_MIXED`; `imageClipboard.ts` keeps the copy memo.

## Performance and size

- Per mask change: one Union over the part's added regions and one Difference, at most 160 polygons with 8 parts of 20 regions: well under a millisecond (Tracyo measured 0.13 ms for both clips). Not run during a drag: the drag paints the ghost only and the clip is recomputed at release, so criterion 24's 50 ms frame budget is the overlay tessellation (measured in `tests/cpu_time/`, thread CPU time).
- The mask is read from the document once per document version and image (cache), not per frame.
- File: a rectangle about 100 bytes, a lasso about 16 bytes per corner, settings under 1 KB per part. Each lasso move rewrites its points into the oplog (ADR 0061 Consequences).
- Wasm: no dependency; the new Rust is a few hundred lines. CI's `metrics/wasm-size` reports it.
