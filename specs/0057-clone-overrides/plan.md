# Plan for Clone overrides (0057)

One branch `story/clone-overrides`, one PR, opened when the maker can switch placement and style
of clones in the app and the full local gate is green (`CLAUDE.md` §7, the frontend checks, the
budget tests). Milestones are commits on the branch, backed up to `wip/clone-overrides`; none is
a PR. `format_version` goes to **14** (the next free number after `0049`'s 13; renumbered on
rebase if another bump merges first).

Started 2026-10-14 from `main` at `34a7625` (format 13, `0049` and `0053` merged).

## Affected crates and modules

| Crate | Modules |
|---|---|
| `curvyo-document-core` | `clone_codec` (two registers, validation), `clone_read` and `clone_model` (style source, effective style), `node_read` (effective opacity), new `clone_overrides` (the four switches), `clone_transform` (held-back rule), `clone_write`, `clone_repair`, `clone_copy` (unlink, repair, copy), `styles`, `node_edit` (writes to own-style clones), `document` (format 14, `document.json`) |
| `curvyo-render-core` | `shared_outline` (share the root tessellation across colour-only own styles) |
| `curvyo-ui-core` | `clone_panel` (the two rows, mixed state, Spread colours), `clone_gesture` and `group_transform` (held-back preview), `layer_rows` (opacity readout, tooltip line), `clone_marks` (tag text) |
| `curvyo-editor-wasm` | `session/clones.rs`, `wasm_clone.rs` (switch commands, notices, view) |
| `frontend` | `CloneBlock.tsx`, the Style sections, the Layers opacity control, `cloneText.ts` |

No new crate, dependency, trait or generic parameter.

## Design notes

- **Stored form (ADR 0016 §1).** On a clone node: `own_placement: true` and `own_style: true`,
  plain booleans, absent when false (a switch back deletes the key, so a file without overrides is
  byte-for-byte what `0049` wrote apart from the version). With `own_style` the ordinary style keys
  (`style_codec::write_style`, all keys written) and `opacity` (only when not 1). Not part of
  `clone_link`.
- **Style source in the one resolver.** `RawLeaf::Clone` carries the clone's override flags and,
  when `own_style`, its stored `Style`. `walk` carries the index of the nearest own-style clone
  down the stack next to `root`, depth and `composed`, so S(c) is memoised per clone and nothing
  recurses. `CloneChain` gains `own_placement`, `own_style` and `style_source: NodeId`;
  `resolved.style` is S(c)'s style. `read_nodes` reads the opacity register of
  `chain.style_source` instead of `chain.root`.
- **Switches** are four commands on `Document` with the spec's labels; each takes a set of
  clones, writes only those whose state changes, commits once and returns how many changed (0:
  no commit, the session posts nothing).
- **M1 as built (2026-10-14).** Public API: `Document::set_own_placement(&[NodeId], bool)` and
  `Document::set_own_style(&[NodeId], bool)`, both `Result<usize, CloneError>` (how many changed);
  the four label constants; `CloneChain::{own_placement, own_style, style_source}`. Also in M1,
  because it is the read side of ADR 0016 §3: `set_node_opacity` writes clones with own style
  (criterion 16's writer; the skip count for the notice is M3), and the Layers row reads the
  opacity of `style_source` (`curvyo-ui-core/src/layer_rows.rs`). The switch to Own first removes
  every style key and `opacity` on the clone (residue), then writes `write_style` (width and
  colour always, the rest where not default) and `opacity` where not 1. The four labels have
  operation names in `curvyo-ui-core/src/history_names.rs` (the names guard).
- **Tester tests renumbered for the bump** (for the lead and the tester): seven tests pinned
  `format_version` 13 or listed every fixture. Each now names 14 (or 15 as the "newer" file) or
  skips the two new fixtures, with no check removed: `acceptance_0042_tester.rs`,
  `acceptance_0042_feedback_tester.rs`, `history_versions.rs`, `acceptance_0049_tester.rs`
  (`curvyo-document-core`), `acceptance_0042_tester.rs`, `acceptance_0042_feedback_tester.rs`,
  `acceptance_0047_part_a_tester.rs` (`curvyo-editor-wasm`); the fixture scans of
  `acceptance_0017_block1_tester.rs` (damaged file listed) and `acceptance_0050_tester.rs`
  (`clone_overrides*` skipped, a later build that stores opacities).
- **M2 as built.** `clone_matrices_after(links, held, selected, g)` takes the set of clones
  with own placement (`Document::own_placement_clones`) and returns `held_back` and
  `held_unlinked` next to the selected matrices; one internal variant takes one map per object.
  Writers that get results instead of a gesture map (`rotate_object`, `resize_path`,
  `transform_objects`) read each outline's change back with `outline_map` (least squares over
  anchors and handle ends, refused when the residual passes 1e-6 mm per mm of extent, so an edit
  that is no affine map holds nothing back). Counts for the notices: `Document::take_clone_hold`.
  The drag preview takes a `transform` flag (`clone_previews`). Unlink, the repair, Offset and
  the transform unlink write the style source's `opacity` and drop both flags (L1, done here
  because the held-back unlink needs it).
- **M3 as built.** `edit_style` skips clones without own style and lets own-style clones take
  marker edits; `curvyo_ui_core::style_scope` targets own-style clones and
  `curvyo_ui_core::skipped_clones` counts the others for the notice (a separate function, so the
  tester's literal `StyleScope` values stay valid). The Opacity target of the session keeps
  own-style clones. The repair copies the style and register opacity of a removed style source
  onto every clone that read from it and stays a clone (`clone_repair::style_copies`); copies
  carry every meta key already, so both states come along (L2 checked by test); object undo
  restores the registers field by field.
- **M4 as built (measured 2026-10-15, release, this VM).** `shared_outline::same_geometry`
  compares everything but the fill and stroke colours and opacities (L3). The switches read
  the document once (`read_objects`) instead of once per clone (L4): own style for 10,000
  clones 87 ms, own placement 71 ms (budget 250 ms; it was 1.2 s with one chain read per
  clone); read of 10,000 own-style clones 51 ms (300 ms); a move of the original holding back
  10,000 clones 118 ms (target 3 s, not gated); the frame after the original of 10,000 clones
  with own colours moved 110 ms (150 ms), with all of them held back 111 ms (not gated). Both
  budget files are already in the `performance-budgets` CI job.
- **M5 as built.** ui-core: `CloneBlock` gains `placement`, `style` (`OverrideState`, mixed
  when the clones differ) and the first clone's effective style for the greyed headers;
  `overrides_words` (tag, criterion 29) and `overrides_line` (Layers, criterion 28);
  `spread_colours` (hues back to front, saturation and lightness kept) on
  `Document::recolour_clones` (one commit `spread_colours`). editor-wasm:
  `session/clone_overrides.rs` with the three commands and the hold and skip notices. Frontend:
  two radiogroup rows and Spread colours (`aria-disabled` below two clones) in `CloneBlock.tsx`;
  for Original's clones `GreyedStyleHeaders` (Stroke, Fill, Markers, muted summary, no control),
  for Own or mixed the ordinary Stroke and Fill sections; the hold notice in the canvas slot, the
  skip notice in the panel slot (not anchored at the edited control: Frage 4).
- **Held-back rule** (milestone 2) extends `clone_matrices_after` with the flag in `CloneLinks`
  and a second output, the held-back matrices of unselected clones; every transform writer that
  calls it writes them (the guard test lists the writers).

## Milestones and tasks

### M1: document model and format (stop here for the architect's review)

- [x] Task 1.1: `clone_codec`: read and write `own_placement` and `own_style`; open-file
  validation of a clone (flags are booleans, a style that would be damaged on a path is refused
  when `own_style` is true). Criteria 37, 38.
- [x] Task 1.2: resolver: style source S(c) and effective style; `CloneChain` gains the three
  fields. Criteria 11, 12 (read side), 13, 15.
- [x] Task 1.3: `read_nodes`: effective opacity from S(c)'s register times the clone's own
  ancestors. Criterion 16 (read side).
- [x] Task 1.4: the four switches `set_own_placement` / `set_own_style` with commit labels
  `own_placement`, `follow_placement`, `own_style`, `original_style`; nothing moves, the drawn
  result does not change, back to Original's removes keys and `opacity`. Criteria 2, 11, 14.
- [x] Task 1.5: `document.json`: `own_placement`, `own_style` with `style` (and `opacity` from
  the registers) on a clone; an `opacity` residue on a clone without own style is left out.
  Criterion 39.
- [x] Task 1.6: `format_version` 14, the doc comment on `CURRENT_FORMAT_VERSION`, golden
  `clone_overrides_v14.curvyo` (each override alone, both, a chain whose middle clone has own
  style), every damage case refused, every residue case read, older fixtures open unchanged.
  Criteria 37, 38.

### M2: held-back rule

- [x] Task 2.1: `clone_matrices_after` with the held-back output (`M·T⁻¹`, singular T unlinks,
  ±1e7 refuses all). Criteria 3, 5, 6, 7.
- [x] Task 2.2: every transform writer (move, nudge, typed entries, resize, rotate, skew, Flip,
  group transform, document resize, fit-to-content) writes the held-back matrices; guard test
  over the writers; held-back count for the notice. Criteria 3, 7, 8, 35.
- [x] Task 2.3: node edits never go through it (criterion 4); the drag preview leaves held-back
  clones alone (criterion 9, ui-core).

### M3: commands that meet overrides

- [x] Task 3.1: style edits, eyedropper and `set_node_opacity` write own-style clones and skip
  Original's ones with a count. Criteria 12, 16, 17, 18.
- [x] Task 3.2: Unlink writes S(c)'s style and register opacity and drops both flags. Criterion 30.
- [x] Task 3.3: repair on removal copies a lost style source; re-pointed clones keep their state;
  orphan repair unchanged. Criteria 31, 36.
- [x] Task 3.4: Duplicate, Ctrl-copy and copy of a set carry both states. Criterion 32.
- [x] Task 3.5: history (undo, object undo, Restore, Preview) and two-peer merges. Criteria 33, 34.
- [x] Task 3.6: output equals the unlinked path. Criterion 40.

### M4: rendering and budgets

- [x] Task 4.1: `shared_outline`: share the root tessellation when only colours and opacities
  differ (ADR 0016 §3). Criterion 41.
- [x] Task 4.2: budget tests: 10,000 own-style clones read in 300 ms, frame in 150 ms; the
  10,000 held-back writes measured (not gated). Criterion 41.

### M5: panel, canvas and Layers

- [x] Task 5.1: ui-core clone block model: two rows with mixed state, Spread colours enabled
  state and hue computation. Criteria 1, 10, 26, 42.
- [x] Task 5.2: editor-wasm commands and notices. Criteria 2, 8, 14, 18, 42.
- [x] Task 5.3: frontend: two segmented rows, greyed Stroke/Fill/Markers headers, Spread colours,
  keyboard and names. Criteria 1, 10, 26, 27, 42.
- [x] Task 5.4: Layers opacity readout or slider by state, tooltip line; canvas tag text.
  Criteria 16, 28, 29.

### M6: docs and hand-over

- [x] Task 6.1: `specs/README.md` format table and status; `docs/` where behaviour changed.
- [ ] Task 6.2: full gate, PR with "What changes in the demo" and the Review guide.

## Validation

- Unit tests next to the code in `curvyo-document-core` for the codec, the resolver and the
  held-back rule; integration tests `tests/clone_overrides.rs` (commands) and
  `tests/clone_overrides_format.rs` (golden, damage, residue, JSON).
- ui-core tests for the panel model and the gesture preview; editor-wasm session tests for the
  notices; frontend unit tests for the text helpers.
- Budgets in `tests/clone_budget.rs` (document-core and editor-wasm), release build.
- Full gate of `CLAUDE.md` §7 before the PR.

## Open questions

Frage 1 (JSON-SHAPE): How `document.json` shows an own style.
Default taken: the clone entry gets `"own_placement": true` and `"own_style": true` only when set,
and with own style a `"style"` object of the same shape as a path's; `opacity` comes from the
node registers as for every node and is left out for a clone without own style (it is residue
there). Change if a reader wants `false` written out.

Frage 2 (FLAG-RESIDUE-TYPE): Is a mistyped override flag on a node that is not a clone damage?
Default taken: no. The flags are checked only on clone nodes (criterion 38's wording); ADR 0016 §5
lists "a flag that is not a bool" without naming the node kind. On any other node the keys are
ignored whatever their type, so a merge that unlinks a clone never makes a file unreadable.

Frage 4 (SKIP-NOTICE-PLACE): Where the notice of criterion 18 shows.
Default taken: the panel's notice slot, not 4 px under the control that made the edit (the
style sections have no anchor for it today). The UX review may ask for the anchor.

Frage 3 (LOCKED-SWITCH): May a locked clone be switched?
Default taken: yes. Neither switch changes the drawn result, so the lock (which guards the
picture, `0050` criterion 20) does not refuse them; the panel may still offer no block for a
locked selection. Change to a refusal if the customer wants a lock to freeze the states too.
