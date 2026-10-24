# ADRs for text on path

Architect draft 2026-10-24, written against `story/text-kerning` at `f3f6aeb` (PR #189, ADR 0021,
`CURRENT_FORMAT_VERSION = 21`) without running code. Slice 1 (criteria 1 to 36, 43 to 48) is
decided in detail; slice 2 (criteria 37 to 42) as far as the ADR needs it.

**Both slices need one new ADR, 0022 "Texts on carriers and text clones", `needs-customer`
(document model, file format).** Reason: ADR 0015 §1 says a node's meaning may depend on another
node in exactly two ways (ancestor state, `clone_link`), and "a third way needs a new ADR". A
text on a carrier is a third way (slice 1), a text clone reading its source is a fourth (slice 2).
The ADR is [`docs/adr/0022-texts-on-carriers-and-text-clones.md`](../../docs/adr/0022-texts-on-carriers-and-text-clones.md), with the content of decisions 1, 2, 3 and 12 below; it is put to
the customer as one question, default A (the options chosen here). The slice-1 build may start
on the default: if the customer picks otherwise, only the `document-core` storage of decisions 1
to 3 changes, nothing in the engine or the UI. No new crate, no new external dependency,
`format_version` 22 for slice 1, 23 for slice 2.

## Depends on

- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §5 (ids, own
  registers), §6 (primitives stay primitives), §7 (no measured layout stored).
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §1 (two ways of
  reference; ADR 0022 adds two), §4 (residue), §5 (every removal repairs references in the same
  commit), §6 (one read pass resolves all references).
- [ADR 0016](../../docs/adr/0016-clone-overrides.md) §1, §3: `own_style` and the style source,
  reused for text clones (slice 2).
- [ADR 0019](../../docs/adr/0019-document-template-and-guide-paths.md) §2, §3: the `guide` flag;
  ADR 0022 lets a linked clone carry its own (decision 2).
- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md) §1 (text node,
  placement, size), §2 (text-core lays out lines; drawing, editing, hit box and Object to path
  share one layout). [ADR 0021](../../docs/adr/0021-per-character-values-on-a-text.md) §6: kerns
  are applied in line space by the engine.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §3 (merge per register), §5
  (drawing never writes). [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) §1 (one
  step, field-wise restore). [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md)
  §9 (format bump).
- [`0079` adrs.md](../0079-text-layout-controls/adrs.md) decisions 1 (flat registers, absent =
  default), 4 (`LayoutOptions`, one mapping in `ui-core::text_layout`), 5 (one layout for caret,
  hit box, Object to path), slice-3 notes (line space and frame), decision 14 (kerns).
- [`0003` adrs.md](../0003-primitive-shapes/adrs.md) "outline construction lives in
  `curvyo-document-core`"; [`0021` adrs.md](../0021-ellipse-arcs-and-shaping/adrs.md) decision 4
  (an open arc is the one open primitive outline); `drawing_outline_of` (criterion 67 of `0021`).
- [`0049` adrs.md](../0049-linked-clone/adrs.md) decisions 2 to 4 (clone read, shared root,
  `clone_previews`); [`0057` adrs.md](../0057-clone-overrides/adrs.md) (own style row);
  [`0046` adrs.md](../0046-document-shapes/adrs.md) (marking, `effective_guide`).

## Slice 1: decisions (2026-10-24)

1. **Storage on the text node (ADR 0022 §1; criteria 5, 17 to 19, 34, 35).** Seven optional keys
   on the text node's meta map, all absent on a straight text:

   ```text
   text_carrier        { node: NodeId string, outline: u32 }   one atomic value, as clone_link
   text_path_position  f64 mm, arc length of the text's middle      always written with text_carrier
   text_path_side      "below"                                  absent: above
   text_path_line      "top"                                    absent: baseline
   text_path_distance  f64 mm                                   absent: 0
   text_path_mirror    true                                     absent: false (Mirror letters)
   text_carrier_kept   { closed: bool, anchors: [[x, y, in_x, in_y, out_x, out_y], ...] }
                       document mm, one outline, one atomic value; absent: none
   ```

   Rules: `text_carrier` is written whole, so a carrier id never pairs with another peer's
   outline index. The other values are flat registers so that Side, Position and Distance edits
   by two peers both survive (`0079` decision 1, option A). Values tied to the carrier are mm
   (Position, Distance, the kept outline); a uniform resize of the text does not rescale them.
   A default is never written; Release from path deletes all seven. `text_placement` is **not
   written** by putting on, by an on-path edit or by a carrier edit: it holds the straight
   placement from before (criterion 6 Undo, criterion 35 fallback); only Release writes it.
   Read rules (criterion 35): an unknown side or line code reads as the default; a non-finite
   position or distance reads as 0; an `outline` index past the carrier's outlines reads as its
   last outline (an outline removed by a node edit keeps the text visible); a kept outline that
   is not well formed is ignored. Model: `TextSnapshot` gains `on_path: Option<TextOnPath>` with
   `TextOnPath { carrier: CarrierRef, side: Side, line: LineOnPath, position: Length, distance:
   Length, mirror_letters: bool, curve: Option<CarrierCurve> }`, `curve` being the resolved
   outline (decision 3), `None` when neither the carrier nor a kept copy is readable: the text
   then draws straight at its stored placement.

   | Option | Why it lost or won |
   |---|---|
   | **A. Atomic carrier link plus flat registers on the text (chosen)** | The clone_link pattern where coherence matters, `0079`'s flat registers where it does not; a text on path stays one text node and every text reader keeps working. |
   | B. One atomic map with all on-path values | Concurrent Side and Position edits lose one side (ADR 0009 §3). |
   | C. The carrier stores the texts on it (a list on the path node) | A list on a path every path command must carry through Join, Split, Combine; two peers adding texts merge into an ordered list nobody needs; the text could not be read alone. |
   | D. A new node kind "text on path" | Every exhaustive match, tool and command gets an arm for what is still a text (customer: "it stays text"). |

2. **The carrier becomes a guide; a linked clone may carry its own `guide` (ADR 0022 §2,
   criteria 4, 4a, 7).** "Put text on path" writes `text_carrier`, `text_path_position` and the
   carrier's `guide = true` in one commit (`Document::put_text_on_path`); a carrier on a locked
   layer is not marked. A linked clone of a path is a carrier kind (spec Terms), and ADR 0019 §2
   lets only its root's flag decide: marking the root would turn the original and all its clones
   into guides. So ADR 0022 extends ADR 0019 §2: a clone node may carry its own `guide`; it is a
   guide when its own flag **or** its root's is set (`effective_guide`, `is_markable` in
   `guides.rs`). This also lets Guide on/off act on a selected clone (today it skips clones); the
   UX review checks the 0046 buttons on a clone. Releasing, removing or converting the text
   writes nothing on the carrier (4a).

3. **Which outline the text sits on, and the kept copy (ADR 0022 §3, criteria 24, 25, 35).**
   - **Resolution in the one read pass** (ADR 0015 §6): new `document-core/src/text_carrier.rs`
     ("the outline a text on path sits on, read from its carrier or from its kept copy, and the
     copy written when a carrier is removed"). For each text with `text_carrier`: a live path or
     compound path gives its outline `outline` (storage order of `PathSnapshot::subpaths`); a
     primitive gives `drawing_outline_of_rotated` (the outline the renderer and exporters draw,
     arcs in 22.5° pieces; open for an arc); a resolved linked clone gives that outline of its
     `resolved` path. A text, image, group, layer or orphan clone is not a carrier: then the kept
     copy; then none. The read also yields `texts_on: HashMap<NodeId, Vec<NodeId>>` (carrier →
     texts) next to `clones_of`, for previews, the Layers tooltip and the removal hook.
   - **Start and direction** are those of the outline functions, which the spec's Terms already
     follow (`rect_outline` starts at the end of the top-left arc, `ellipse_outline` at
     `(cx + rx, cy)`, polygon and star at their first vertex, all clockwise on screen; a path at
     its first node). No outline function changes; one test per kind pins the start.
   - **Rejected: a parametric (exact) outline per primitive in `geometry-core`.** Exact circle
     arc length is trivial, but an ellipse needs elliptic integrals, the curved polygon and star
     have no closed form, and the text would sit on a different curve than the one drawn and cut
     (the four-cubic circle is off by 2.7e-4 R, 0.27 mm at R = 1,000 mm). One curve for drawing,
     hit and text. The criteria's test points (criterion 10: 12 and 6 o'clock) are anchors,
     where the cubic circle is exact.
   - **Kept copy (Frage 36 A):** `Document::repair_clones_of_removed` is the funnel every removal
     passes (Delete, Booleans, Combine, Split, Break apart, Cut, Fracture, Flatten, Object to
     path with a new id, restore). It gains a second step, `keep_carriers_of_removed`: for each
     live text whose carrier is removed, write `text_carrier_kept` (the outline as resolved just
     before, document mm) in the same commit, and keep `text_carrier` as it is. The reader prefers
     a live carrier, so Undo of the removal brings the carrier back and the link works at once,
     whether or not the restore also removes the kept copy. Rejected: reading the deleted node's
     tombstone (lost by the history wipe of ADR 0014), keeping the carrier as a hidden node (a
     delete would not delete). Release removes the copy. A transform of a text with a kept copy
     (document resize, fit to content) maps the copy; a carrier edit never writes the text.
   - **Copy and paste:** `copy_map` remaps `text_carrier` to the carrier's copy when both are
     copied (the clone-copy rule of `clone_copy.rs`); a text pasted into a document without its
     carrier gets the carrier's outline as kept copy.
   - **Merge residue, left for the sharing story** (ADR 0015 consequences): a peer puts a text
     on a carrier another peer deletes; the text has no kept copy and reads straight. Fixable
     later by a repair after merge, no file change.

4. **Arc length along the carrier: new `geometry-core/src/path_measure.rs` (criteria 8, 15, 26,
   43, 45, 46).** One responsibility: "arc length, point, tangent and nearest arc length along one
   outline." `PathMeasure::new(&[OutlineAnchor or anchors], closed)` builds the cumulative length
   table once (kurbo `arclen` per segment, accuracy 1e-5 mm); `at(s) -> (Point, unit tangent)`
   evaluates the **exact cubic** (inverse arclen per segment, binary search over the table), so
   tangents are exact; `nearest(Point) -> s` reuses `nearest_point_on_segment`. Rules: at a
   segment boundary the incoming segment's tangent (criterion 11); a zero derivative (cusp,
   collapsed handle) takes the chord to the next distinct point; a carrier of length 0 places the
   text straight along +x at that point. Beyond the ends: open, the straight extension of the end
   tangents; closed, `s` modulo the length (criterion 26). No kurbo type in the public API (the
   rule of `segment.rs`). Rejected: walking a flattened polyline (at R = 50 mm and 0.001 mm
   chord tolerance the tangent is off by up to 6 mrad, 0.06 mm at the top of a 10 mm letter).
   Not bit-identical across targets and need not be: nothing measured is stored.

5. **Placing the letters: new `ui-core/src/text_on_path.rs` (criteria 8 to 12, 18, 19, 47, 48).**
   One responsibility: "the rigid map of each cluster of a laid-out text onto its carrier." text-core
   is unchanged: it still lays out lines in line space (kerns, spacing, alignment included,
   ADR 0021 §6); ui-core maps. For each cluster with centre `x_c` (`PlacedGlyph::centre_x`, after
   alignment) and the text's middle `x_m` (the middle of all lines' extent; Position places it,
   customer 2026-10-24):
   - `σ = +1` when the text reads in carrier direction (Above + Baseline, Below + Top edge),
     else `−1`; `s = position + σ·(x_c − x_m)`; `(P, τ) = measure.at(s)`; `n` = left normal of `τ`
     (y down: `(τy, −τx)`).
   - Frame: text-space x goes to `σ·τ`, text-space up (−y) to `σ·n`; the touching line
     `ℓ = 0` (baseline) or `−size` (top of H); the letters' side normal is `n` for Above, `−n`
     for Below; the glyph point `(x, y)` maps to `P + σ·(x − x_c)·τ − (y − ℓ)·σ·n +
     distance·side_normal`. Line k needs nothing extra: its y is already k line heights down
     (criterion 12). Vertical kern moves y, horizontal kern moves `x_c` (criterion 47).
   - **Mirror letters** (criterion 19): in line space, before the map, `x ↦ a + b − x` with
     `[a, b]` the line's extent: each glyph mirrored about its own axis, order reversed, the
     line on the same span. **Mirror** (criterion 18) is only a write of Side and Line.
   - **Vertical mode** (criterion 48): `layout_options` gets `writing_mode = Horizontal` and the
     default orientation when `on_path` is set; the stored values are not touched.
   - `lay_out` (`ui-core::text_layout`) branches once: straight as today, or on path through this
     module. Draw, hit, caret and Object to path read its result, so criteria 9, 29 and 30 hold by
     construction. A letter whose frame point is non-finite or past ±1e7 mm is left out (45).
   - Defaults (criterion 5), the 12-o'clock point (topmost point via the segments' y extrema,
     first in carrier direction), Release placement (criterion 23: frame of the first cluster at
     its baseline start, with a reflection when Mirror letters is on; the vertical layout's first
     character for a vertical text), the handle drag (nearest `s`, Side switch past 8 px), degrees
     on an ellipse (readout only: the angle of the point in the ellipse frame; stored is mm) and
     the arrow nudge live in a second module `ui-core/src/text_path_edit.rs` ("the commands and
     handle math of a text on path"), so `text_on_path.rs` stays the pure map.

6. **Hit test, caret, selection box (criteria 27, 28).** `TextOutline` gains `letter_boxes:
   Vec<[Point; 4]>` (each cluster's advance by ascender to descender through its frame), filled
   only on path. Hit: any letter box contains the point; marquee/lasso: all boxes inside;
   `line_box` becomes the axis-parallel box of the letter boxes, which the selection box already
   reads. Caret: `caret_point` in line space mapped by the frame at the boundary's x; a click maps
   back through the hit letter's frame into line space and calls `nearest_boundary`. The edit
   session (`editor-wasm/src/session/text.rs`) uses the same `lay_out` path; no second geometry.

7. **Live update and invalidation (criteria 24, 36, 43).** Committed edits need nothing new: the
   object cache reads and lays out every text once per document version (`object_cache.rs`), so a
   carrier commit or a merge relays the texts on it. **Drag previews** (the read is pinned while a
   drag is in flight): new `document-core::text_path_previews(objects, changed)`, the analogue of
   `clone_previews`: through `texts_on` it returns each text whose carrier is among `changed`
   (the drag's preview objects, including clones previewed behind a dragged original) with its
   `curve` replaced; the session lays out only those, reusing the straight layout cached in the
   read (string and face did not change), so a frame costs one `PathMeasure` and the placement
   pass. A text being dragged along its carrier previews the same way with a new position. The
   preview cap of `clone_previews` applies to the count of texts.

8. **Transforms and commands (criteria 20 to 22, 29 to 31).** A text on path never takes a
   placement write from `transform_objects` and the Select tool: with its carrier selected the
   carrier is transformed and the text follows on the next read; a Ctrl uniform corner scale
   writes `text_size` (alone: about the reference point, which stays on the carrier; with the
   pair: by the pair's factor); Flip toggles `text_path_mirror`; rotate on an ellipse carrier and
   a body move write `text_path_position`. Commands that convert texts first (Object to path,
   Weld, Booleans, Split) read the on-path outline. Duplicate copies all seven keys, position +5 mm.

9. **Document API.** `put_text_on_path(text, CarrierRef, position, mark_guide: bool)` ("Put text
   on path"); `set_text_path(&[(NodeId, OnPathValues)], OnPathLabel)` writing only differing
   keys, one commit per interaction with the spec's labels ("Change text side", "Change line on
   path", "Move text on path", "Change distance on path", "Mirror text on path", "Flip");
   `release_from_path(&[(NodeId, Affine)])` ("Release from path"). The placement, defaults and
   positions are computed in `ui-core` (document-core knows no faces), as `0079` decision 3.
   The keys are read and written in a new `document-core/src/text_path_codec.rs` (`text_codec.rs`
   is 561 lines and does not grow); types in `text_path_model.rs`.

10. **`format_version` 22, golden file `text_on_path_v22.curvyo` (criterion 34).** A version-21
    build would ignore the keys and draw every text on path as a straight text at its old
    placement, and a clone carrying its own guide flag as artwork (a cut): it must refuse the
    file (ADR 0004 §9). Migration from 21: none. Golden content as criterion 34 lists, plus a
    clone carrier marked as guide. The number is provisional by the rule above
    `CURRENT_FORMAT_VERSION`; this branch is stacked on #189 (21), so 22 if nothing merges between.

11. **Budgets (criteria 43 to 46).** No new dependency; the new code is three small modules, the
    wasm growth is measured at the end of the slice against the 2 MiB text-engine budget (about
    156 KiB left before `text-kerning`; the plan records the figure after #189). Criterion 43:
    the measure table for 1,000 cubics is one `arclen` per segment, the placement 200 binary
    searches; the plan adds one release-build timing test on that fixture for a position drag
    and a carrier node drag. Criterion 45: the table is O(n) and is built once per read and per
    preview frame; the plan times 100,000 segments for "does not hang" (no frame budget). All of
    it in `geometry-core`, `document-core` and `ui-core`, which build for wasm32 (criterion 46).

## Slice 2: what it needs (ADR 0022 §4)

12. **A text clone is a text node with a source reference, not a clone node.** New register on a
    text node, `text_source: NodeId` (the source text). The read pass fills its `TextSnapshot`
    from the source (string, chars, font, size, layout, style unless `own_style` of ADR 0016 §1,
    reused as is); `id`, `name`, `visible`, `locked`, the seven on-path keys and the placement
    are its own (criterion 39). `TextSnapshot` gains `source: Option<NodeId>`; every text reader
    works unchanged, the edit session commits string edits to the source id (criterion 38).
    No chains: a source that is missing, not a text, or itself a text clone makes an orphan (not
    drawn; the repair after Open and merge deletes it, ADR 0015 §5). Removing a source unlinks
    its clones in the same commit through the removal funnel: each gets a copy of the source's
    values and loses `text_source` (criterion 41); Unlink clone does the same for one.
    Budget 44: the session shapes a source once per read and places it on each carrier.
    `format_version` 23 at its merge.

    | Option | Why it lost or won |
    |---|---|
    | **A. Text node with `text_source` (chosen)** | Still a text everywhere (hit, edit, convert, Layers row); placement per carrier is its own by construction; `own_style` reused. |
    | B. The `0049` clone node with a text origin | `clone_link`'s matrix means nothing on a carrier; `CloneSnapshot.resolved` is a path, so every clone consumer (draw, hit, output, Booleans, held-back rule, Placement row) needs a text branch; clones of texts are not origins in `clone_read.rs` today. |
    | C. Copies written on every edit | ADR 0015 option B: an edit reaches only the copies a peer knows; copies disagree after a merge. |

## Dated notes

- 2026-10-24 (customer change of slice 1): `text_path_position` is the arc length of the text's
  middle, not of its start. Format 22 is not on main yet, so the key keeps its name and only its
  meaning changes; no migration, the golden file is regenerated. The default on an open carrier
  is half the text's width (its start at the carrier's start), on a closed one the topmost point.

## For the customer (one question, through the lead)

ADR 0022 "Texts on carriers and text clones" (`needs-customer`: document model and file format):
a text stores a reference to its carrier and keeps a copy of the curve when the carrier is
deleted (slice 1); a text clone is a text that reads string, font and size from its source
(slice 2); a linked clone may be a guide on its own. Default if unanswered: A as above; the
slice-1 build starts on it.

## For the product owner

- Distance and Position are mm and do not follow a Size change; letter values stay factors of
  Size (`0079`). A ring text whose Size is changed keeps its gap to the ring.
- Criterion 2 lists "a linked clone of a path" as carrier; making it a guide marks the clone
  alone, not its original (decision 2).
