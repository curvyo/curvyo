# Architect review: 0057 clone overrides, milestone 1

Date: 2026-10-14. Branch `story/clone-overrides`, local commit `30cf07e`, diff `origin/main...HEAD`.
Read: `specification.md`, `adrs.md`, `plan.md`, ADR 0016, ADR 0015, `CLAUDE.md` §5 and §6, every
changed file in `curvyo-document-core/src/` and `curvyo-ui-core/src/`, `tests/clone_overrides.rs`,
`tests/clone_overrides_format.rs`, the seven renumbered tester tests, and on main
`curvyo-render-core/src/shared_outline.rs`, `clone_write.rs` (unlink), `path_codec.rs` (open
validation). Ran `cargo nextest` for document-core, ui-core and render-core (3556 passed) and the
wasm32 build of `curvyo-document-core` (passes).

## Verdict

**APPROVE**, with two small items (F1, F2) to fix on the branch before the PR, and four
obligations for later milestones (L1 to L4) recorded so they are not lost. No new crate,
dependency, trait or generic parameter. `clone_overrides.rs` has one responsibility (the four
switches) and stays far below the size limits. The core/platform boundary is untouched.

## The six questions

1. **Stored form: confirmed.** `own_placement` and `own_style` are plain `Bool` values on the meta
   map, written only as `true` (`clone_codec::write_flag`); `false` deletes the key, so a file
   without overrides differs from a `0049` file only in the version. `clone_link` is unchanged.
   The switch to Own first deletes every style key and `opacity` (residue), then writes
   `write_style` and `opacity` only when not 1; the switch back deletes marker, keys and
   `opacity`. Format 14 with the doc comment on `CURRENT_FORMAT_VERSION`.
   **Damage rules: confirmed against ADR 0016 §5.** Refused: a flag that is not a `Bool` (number,
   string, list, container: `validate_clone_node` accepts only `None` or `Value(Bool)`); a style
   value that would be damaged on a path, checked only with `own_style` true; `opacity` outside
   0..=1 on any node, including a clone without own style. Read as residue: flags on a non-clone,
   style keys and `opacity` on a clone without own style, a stored `false`, missing style keys of
   an own style (defaults). Every case has a test; the golden test asserts literals read from the
   file (states, colours, opacities, a matrix to the bit, the chain's style source) and that
   opening writes nothing. Note: the as-built writer writes width and colour always and other
   keys only when not default, where ADR 0016 §1 says "all style keys". The read is identical
   (absent = default), so this is accepted; dated note in `adrs.md`.
2. **`document.json` shape (Frage 1): accepted as defaulted.** Flags only when true, `style` (the
   path's shape) only with `own_style`, `opacity` from the node registers, and the residue
   `opacity` of a clone without own style left out. Non-authoritative (ADR 0004 §1), so writing
   `false` later stays cheap if a reader wants it.
3. **API: accepted.** `set_own_placement` / `set_own_style(&[NodeId], bool) -> Result<usize,
   CloneError>` validate every id before the first write, write only clones whose state changes,
   and commit nothing on 0. Labels and history names are in place. `CloneChain` gains
   `own_placement`, `own_style`, `style_source`; `own_style` is redundant with
   `style_source == id` but cheap and set in one place (`clone_read::assemble`), so it stays.
   - *One-resolver rule: kept.* S(c) is computed only in `clone_read::walk`, memoised in
     `State::Resolved` next to `composed`; no consumer walks the chain. `node_read` and
     `layer_rows` look up the register of `chain.style_source`; `set_own_style` uses
     `read_one_clone`, which calls the same `resolve`.
   - *Shared tessellation: safe now, M4 work.* `shared_outline::draw` shares only when
     `same_but_opacity(clone.resolved.style, root.style)`; an own style that differs in a colour
     falls back to the clone's own tessellation, so M1 draws correctly and only loses the sharing.
     A clone just switched to Own has the root's style and still shares. See L3.
4. **Work beyond M1: accepted.** `set_node_opacity` writing own-style clones and the Layers row
   reading `style_source` are the writer and reader of the register this format introduces;
   without them the M1 model would be read wrong by `0050` code already on main. The skip count
   and the slider swap stay in M3 and M5.
5. **Frage 2: accepted (A, check flags only on clone nodes).** No writer of ours produces a
   mistyped flag anywhere, so refusing it on a path would protect nothing, while ignoring it keeps
   the one-dispatch validation of `path_codec::validate_object_node` (a path is validated as a
   path). A node cannot turn from a path into a clone, so a stray flag on a path never becomes
   live. **Frage 3: accepted (a locked clone may be switched).** Neither switch changes the drawn
   result, and the lock guards the picture (`0050` criterion 20); held-back writes on a locked
   clone are already allowed (`adrs.md` decision 3). Both recorded in `adrs.md`.
6. **Renumbered tester tests: accepted.** Every change replaces 13 by 14 (15 as "newer") or
   excludes the two new fixtures; no assertion is removed. The 0017 scan still opens the golden
   (only the damaged file is listed); the 0050 scan skips `clone_overrides*`, consistent with its
   existing `layers*` exclusion. Minor: the test name
   `acceptance_0049_tester.rs::ac37_a_newer_format_is_refused_and_this_one_is_13` now checks 14;
   the tester renames it to `..._and_this_one_is_current` in the final pass.

## Fix on the branch before the PR

**F1. No test reads the raw meta map after the switch back.** Rule: file formats we write are
protected; ADR 0016 §1 promises the switch back removes marker, style keys and `opacity`.
`ac14_back_to_the_originals_style_removes_the_own_one...` checks `document.json` (which never
lists style residue) and the opacity register, so a style key left behind would pass. Fix: in
`tests/clone_overrides_format.rs` (it already has `loro_of` and `tree_id`), switch a clone to Own
and both flags on, then both back, and assert the clone's meta map holds exactly `shape` and
`clone_link` (plus the node registers it had before).

**F2. Stale doc on `leaf_style`** (`curvyo-document-core/src/clone_write.rs`, about line 69): "for
a clone its root's". It now returns the effective style (S(c)'s) because `resolved.style` changed.
Rule: §5 the doc states the responsibility. Fix: "for a clone its effective style (ADR 0016 §3)";
`0049` criterion 33's boolean result then takes the base clone's effective style, which is what
`0057` criterion 40 wants.

## Obligations for later milestones

- **L1 (M3, Unlink).** `clone_write::keys_replaced_by_unlink` does not list `own_placement` and
  `own_style`, so today an unlinked clone keeps both flags as residue on the path. Add them (ADR
  0016 §4). The `opacity` of an own-style clone is already kept; for an Original's clone Unlink
  must write S(c)'s register, not the root's.
- **L2 (M3, copy, repair, restore).** Duplicate, Ctrl-copy, copy of a set, the repair copy of a
  lost style source, and object Restore must carry the two flags and the style keys; check that
  none of them copies only `clone_link` for a clone node.
- **L3 (M4, shared tessellation).** Replace `same_but_opacity` by a test of the geometry fields
  only (width, cap, join, miter, dash, markers) **and** `fill.enabled` / `stroke.enabled` equal to
  the root's, because `tessellate` builds a stroke layer only where the root's stroke draws.
  Colours and opacities come from `clone.resolved.style`, as `draw` already does.
- **L4 (M4, switch cost).** `set_own_style` resolves each clone twice (`live_clone_metas` and the
  copy), each time reading the root path from Loro. For 10,000 clones of a 100-node path this is
  20,000 path reads. Measure it in the budget test; if it passes 250 ms (the `0016` busy
  threshold), take the clones from one `read_objects()` instead. Same for `set_own_placement`.
