# Architect review: 0074 text objects, milestone 2

Reviewer: architect, 2026-10-20. Branch `story/text-objects`, commits `51d2155`
(`curvyo-text-core`, included fonts) and `2609714` (text node, `format_version` 17), against
ADR 0020, ADRs 0002, 0004, 0009, 0011 and `CLAUDE.md` §5 and §6. Checked: `cargo build
--target wasm32-unknown-unknown -p curvyo-text-core` and the document-core text tests pass.

## Verdict: CHANGES REQUESTED

The node, the transform rule, the per-character merge, the undo and duplicate paths, the boxed
variant and the crate are sound. Fix the three format items (1 to 3) before milestone 3. Item 4 is
a constraint on milestone 4: the fix is recorded here so the work does not go in the wrong direction.

## Must fix

1. **A merge of two valid edits makes a file that cannot be opened.**
   `curvyo-document-core/src/text_codec.rs:206` refuses a string over `MAX_TEXT_CHARS` on open.
   Two peers who each type 6,000 characters into one text both stay inside the limit, but the
   merged string has 12,000 characters, so `unpack` returns `Damaged`. This breaks ADR 0009 §3
   (a merge of valid edits is valid) and ADR 0004 §9 (the refusal on open is for damaged files,
   not for merged ones). Fix: open-file validation and `curvyo-text-core::layout` use a hostile-
   input bound, `MAX_STORED_TEXT_CHARS = 100_000` (one constant in document-core; text-core's
   `MAX_SHAPE_CHARS` becomes that value, `layout.rs:67`). Typing and pasting stay capped at
   `MAX_TEXT_CHARS` = 10,000 (criterion 31). An over-long merged text opens, draws and converts
   in full; the edit session refuses to make it longer but allows deleting. Add a test: two peers
   each insert 6,000 characters, then save and open. See the dated note in `adrs.md`.
2. **The face hash's case is not fixed on disk.** `texts.rs:55` and `text_codec.rs:131` accept
   upper-case hex, and `text_codec.rs:138` lower-cases it on read. A file can therefore hold a
   hash that reads differently from how it was written, and `set_text_font`'s "already has this
   font" check (`texts.rs:157`) compares the stored value with the value read back. Fix:
   `check_font` and `read_font` accept only 64 lower-case hex digits (ADR 0020 §1 says "64 hex"
   and `FaceHash::to_hex` writes lower case). Extend the damaged-register test with an
   upper-case hash.
3. **A rotation or a move rewrites `text_size`.** `text_model.rs:95-105`: under a rotation the
   uniform factor is 1 ± 1e-16, so `write_size_and_placement_if_changed` (`text_codec.rs:186`)
   writes a size that differs from the stored one in the last bits. This happens on every rotate
   (`objects.rs:409`) and every multi-object transform. Each such write is a register write the
   maker did not ask for, and the size drifts over many rotations. Fix: when `|s - 1| <=
   UNIFORM_TOLERANCE`, `uniform_scale` returns exactly 1.0. Add a test: after 1,000 rotations the
   size is bit-identical and the size register has no new op.
4. **Path commands read a text as its drawn outline (milestone 4 constraint).**
   `boolean.rs:105,208`, `combine.rs:134`, `split.rs` and `flip.rs:181` take a text through
   `drawn_path()`, and when it is `None` the result is an empty outline (`Vec::new()`). This has
   two consequences. First, a text drawn with a substitute face would be cut with the substitute,
   which ADR 0009 §5 and criterion 29 forbid. Second, a text that has not been laid out yet would
   drop out of a Boolean or a Combine without notice. Fix:
   (a) `TextOutline` (`text_model.rs:64`) gets `substitute: bool`, set by the session when it
   lays the text out with the included sans face (ADR 0020 §1, matching step c).
   (b) `drawn_path()` stays for drawing, hit test, bounds and flip previews.
   (c) Boolean, Combine, Split, Break apart, Cut, Fracture, Flatten and Offset match
   `ObjectSnapshot::Text` explicitly and call the one criterion 22/24 conversion. That conversion
   refuses with the criterion 29 notice when the text uses a substitute or has no layout, and
   never returns empty geometry. Test: Combine on a path and a text without an outline writes
   nothing.

## Rulings on the open points

**(a) Great Vibes' swash H, criterion 12.** The implementer is right. Size is the height of the
H's top above the baseline (spec "Terms", customer decision TEXT-SIZE A). The bounding-box height
is only a stand-in for that, and it is wrong for a swash that goes below the baseline. The product
owner changes the test sentence of criterion 12 to:

> Test: for each included face and for the fixture faces, a one-line text "H" at Size 20 mm
> converted to paths (criterion 22) has the top of its bounding box 20.00 mm above the baseline
> within 0.01 mm (for a face whose H stands on the baseline, this is the bounding-box height;
> Great Vibes' swash H also reaches below it).

**(b) Emptying an existing text, criterion 8.** The implementer is right. Every document change is
a commit, and every commit is a history step (ADR 0014), so a removal with "no step" cannot be
built. If it could, Undo could not bring the text back. The emptied existing text is removed in
the session's one step "Edit text", and Undo restores the node with its string as a `LoroText`
(the revival path of `history/merge.rs`). The document-core command does this:
`edit_text_string(id, "")` deletes the node in the same `edit_text` commit and never leaves an
empty text node. Add a test for it in milestone 4. The product owner changes criterion 8 to:

> 8. Given edit mode ends with an empty string, then for a new text nothing is created and no
> history step is written; for an existing text the text object is removed, and this is the
> session's one history step "Edit text" (criterion 7), which Undo takes back with the text's
> string.

An empty text can still come out of a merge (two peers delete everything at the same time). It
opens, draws nothing, is hit by its anchor point only, and its Layers row reads "Text". This is
allowed: it is not damage.

## Checked and sound

- **Node and transform rule** (`text_model.rs`, `text_codec.rs`): four registers as ADR 0020 §1
  states. The font is one map value written whole, the placement one list value. Uniform scale
  goes into the size, and when the size is clamped the remaining factor stays in the placement
  (`mapped_by`, proven by `draws_alike`). Any other map is composed into the placement.
  Translation moves the anchor.
- **Per-character merge**: `apply_string_diff` writes only the common-prefix/suffix difference in
  Unicode scalar positions. `ac09_..._per_character_merge` shows both peers' letters are kept.
- **Format 17**: the doc comment states the change, says no migration is needed and names the
  refusals. The golden `text_v17.curvyo` holds the five required texts. The generic
  `FormatTooNew` test (`acceptance_0001.rs:117`) covers refusal by older builds.
- **Read model** (`TextSnapshot::outline`): this is a sound place, with must-fix 4 as the
  condition. It is derived and `#[serde(skip)]`. No write path and no `ObjectJson::Text` reads
  it, so nothing measured is stored (ADR 0020 §1). Document-core stays pure: it holds the slot
  but does not depend on the engine. It is the same pattern as a clone's `resolved`. The difference
  is that the session fills the slot, not document-core, so `None` must mean "not known" and
  never "empty".
- **Boxed variant**: justified by clippy `large_enum_variant`. A clone boxes its chain the same
  way.
- **Undo and duplicate**: `Node::Text` in the three-way restore writes the difference only where
  nobody has typed since (otherwise the step is counted as kept). A revival and `copy_map` create
  a fresh `LoroText`. The string never flattens to a plain value.
- **`curvyo-text-core`**: one responsibility; no I/O, clock or threads (a `Mutex` around the
  glyph cache is not a thread); `#![forbid(unsafe_code)]`; `harfrust =0.14.0` and
  `skrifa =0.48.0` are pinned exactly; no `unwrap` or `expect`; every module is under 230 lines
  and every function under 60; it builds for wasm32; `deny.toml` has the AGPL exception.

## Should fix

5. **Units are types in the public API** (`CLAUDE.md` §5): `TextLayout::{ascender, descender,
   line_height}` and `LineLayout::{baseline, width}` are bare `f64` millimetres
   (`layout.rs:29-58`), although the input already uses `Mm`. Change them to `Mm` before
   milestone 4 code starts using them.
6. **Glyph point limit** (`outline.rs:279`): the limit is checked after the whole glyph has been
   drawn. Make `CubicPen` stop appending once `points > MAX_GLYPH_POINTS` and report the glyph as
   too large, so the limit also bounds memory.
7. **Cap-height floor** (`face.rs:249-253`): a hostile H whose top is 1e-6 em gives a
   `mm_per_em` of millions and outlines far beyond the document's coordinate limit. Accept a
   measured or OS/2 cap height only within 0.05 to 2.0 em, otherwise use 0.7.
8. **`proptest = "1"`** (`curvyo-text-core/Cargo.toml`): move it to `[workspace.dependencies]`
   like every other shared dev-dependency.

## Architect follow-ups done in this commit

- ADR 0011: dated amendment adding `curvyo-text-core` to the crate table and the dependency
  edges.
- `adrs.md`: dated notes for must-fix 1 (`MAX_STORED_TEXT_CHARS`), must-fix 4 and rulings (a)
  and (b).
