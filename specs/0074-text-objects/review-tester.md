# Tester and architect review: 0074 text objects, finished slice

Reviewer: tester (with the architect checklist), 2026-10-20. Branch `story/text-objects`, four
commits on `origin/main`. Acceptance tests written from `specification.md` before the
implementation was read: `curvyo-editor-wasm/tests/acceptance_0074_tester.rs` (20 tests, for the
criteria the implementer's tests leave open or cover thinly: 3, 7, 9, 10, 11, 13, 14, 15, 18, 22,
24, 25, 27, 28, 31, 33).

## Verdict: FAIL

Two acceptance tests fail on real defects, and a frontend loop re-reads the font folder without
end while the Text tool is active.

## Must fix

1. **Object to path leaves a selection the rest of the editor cannot use** (criterion 22, "the
   result is selected"). `curvyo-editor-wasm/src/session/text.rs:740-743` toggles the new
   *group* ids into the object selection. Everywhere else (Ctrl+G,
   `session/grouping.rs:90-92`) a selected group is held as its leaves. Effect: after Object to
   path the Style tab scope is empty (subject ""), and the arrow keys do not nudge the result.
   Test: `ac22_two_texts_one_step`. Repro: type "Ab", Escape, Object to path, press Right:
   nothing moves. The implementer's `text_tests.rs::ac12_ac22_object_to_path` asserts the
   group id in the selection, so it encodes the defect; change it with the fix.
2. **Undo from the History tab in edit mode takes back the step before the session, and the
   typing is lost** (criteria 7 and 9, open point (b)). `session/history.rs:253`
   (`press_history`) does not end edit mode first. Repro: add "Base", click into it with the
   Text tool, type "X", press Undo in the History tab: "Add text" is undone, the node goes,
   edit mode stays on, and Escape writes nothing (`TextEdit::finish` fails on the deleted
   node and `finish_text_edit_inner` clears the selection silently, `text.rs:279`). For a new
   text the previous step (for example "Draw rectangle") is undone while typing continues.
   Test: `ac09_history_undo_in_edit_mode_takes_the_session_back`. Fix: in `press_history`, call
   `finish_text_edit_inner` first, as Save does (UX notes §2).
3. **The font folder is listed in a loop while the Text tool is active** (criteria 19 and 20).
   `frontend/src/hooks/useText.ts:257-259` calls `listHostFonts()` on every sync in the Text
   tool. Every `font_folder_list` answer bumps `listsRevision` (line 179), which is a
   dependency of the same effect, so it runs again, lists the folder again (disk read and a full
   `fontdb` parse of every Curvyo font), resends all lists through `set_font_list` (with 2,000
   system faces, JSON in and out every round) and re-renders the Fonts tab. It never settles.
   Criterion 19 asks for a fresh read each time the Fonts tab is *shown*. Side effect:
   `TextFonts::set_block` re-appends the Curvyo block at the end
   (`session/text_fonts.rs:65-79`), so face indices shift on every round, and a pick from a
   Fonts tab rendered one round earlier can name the wrong face. Fix: list on Text tool entry
   and in `FontsTab`'s `onShown` only, not in the sync effect.

## Should fix

4. Criterion 21: colour and bitmap fonts are listed. `curvyo-storage-io/src/fonts.rs` filters
   by extension only, so a CBDT/sbix font (Noto Color Emoji) shows in "This computer" and then
   fails in the engine (`Face::new` refuses it, so it is drawn as missing). Filter faces without
   `glyf` or `CFF`/`CFF2` at listing time.
5. `curvyo-app/src/fonts.rs:96-107` and `list_font_folder` read a whole file of any size. Cap it
   at text-core's `MAX_FACE_BYTES` before reading, so a stray multi-GB `.ttf` in the folder
   does not go through IPC and memory first.
6. `session/text.rs` has 796 lines (rule: about 500, or a reason in the PR). Split it (edit mode,
   Text bar/Fonts, Object to path) or justify it in the PR. `text_fonts.rs`'s doc sentence lists
   four duties.
7. Cut on a text with a multi-outline glyph is refused as "Compound" (`0048` rule) after the
   silent conversion. That is consistent with criterion 24, but the refusal text points to
   Split/Break apart for something the maker sees as a text. Product owner to word it.
8. `text_edit.rs:265`: the preedit caret is snapped against the raw IME text but stored with the
   normalised text (tab to four spaces, CR dropped), so the two can disagree. Normalise first,
   then snap.

## Open points (a) to (d)

- (a) Live "Size 24.0 mm" readout while resizing: not a criterion. Criterion 26 says "Size shows
  the new height", and the Size field shows it after the release. This is a UX notes §5 gap for
  the ux-engineer, not a FAIL.
- (b) History Undo in edit mode: **breaks criteria 7 and 9** (must-fix 2).
- (c) Missing-font refusal in the canvas notice slot: matches the spec's Terms ("Notices ... in
  the canvas notice slot, as in `0056`"). Only UX notes §4 says "under the button". No breach.
- (d) The Fonts tab never drops loaded faces: criterion 16 asks only for loading on scroll and a
  virtualised list. The "visible rows plus 10" cap is UX notes §4. No breach. It is a memory
  risk with 2,000 system faces scrolled through, so it is a should-fix.

## Tab lists of 0043, 0050, 0051

Every changed line in those tester files only appends `"fonts"`/`"Fonts"` (length 5 to 6, the
list gains a last entry). `history_tab_session.rs` checks that History is second-to-last instead
of last, which is the same claim with Fonts appended. Nothing was removed or weakened. All other
test-file changes on the branch add `ObjectSnapshot::Text(_) => unreachable!` arms only.

## Architect checks

- M2 must-fixes landed: `MAX_STORED_TEXT_CHARS` = 100,000 (`text_model.rs:17`, used on open,
  equal to text-core's `MAX_SHAPE_CHARS` by a test, merge test `ac31_a_merge_past_the_typing_limit_opens`);
  hashes are lower-case hex only (`is_face_hash`, `text_model.rs:102`, on write and read);
  a factor within 1e-9 of 1 is exactly 1 (`text_model.rs:131`, 1,000-rotation test); every path
  command (Boolean, Combine, Split, Break apart, Cut, Offset, Fracture, Flatten) plans through
  `text_convert::texts_as_paths` after `texts_refusal` (all or nothing). Should-fixes 5 to 8 landed too.
- `curvyo-storage-io`/`curvyo-app`: `fontdb =0.24.0` with `std, fs, fontconfig`, no `memmap`
  (no `unsafe` mmap); the system list runs in `spawn_blocking`; ids are opaque path hashes; the
  page never sees a path. Sound apart from items 3 to 5.
- Core purity (CLAUDE.md §6, criterion 33): no `std::fs`, threads or clock in `curvyo-text-core`,
  the ui-core text modules or document-core's text files; text-core builds for wasm32.
- `document-core/text_paths.rs`: checks everything before the first write and is one commit with
  the primitives. The `unwrap`s carry one `invariant` comment at the top of `write_group`. A
  repeated id in `texts` would hit `tree.delete` twice. Callers pass selection ids, which are
  unique, so this cannot happen today.

## Wasm size (open for the customer)

Measured by the implementer: +1,414 KiB (496 KiB gzip), engine about 1,290 KiB, slice code about
120 KiB, 83 KiB over 1.3 MiB.

- The engine's feature flags have no slack. `harfrust` and `skrifa` are on
  `default-features = false, features = ["std"]`, so `skrifa`'s `autohint_shaping` is already off,
  and `icu`/`libm` are unused. One `read-fonts` serves both engine crates.
- In the slice's own 120 KiB, nothing is cheap to remove: `sha2` is 7 KiB, and the rest is the
  feature.
- Build settings: release is `opt-level = 3`, LTO, one codegen unit, and `wasm-opt` is not run.
  The per-crate result is in "Measurements" below.

## Measurements

Release editor wasm after `wasm-bindgen`, same sources (HEAD `a04bc21`):

| Build | Bytes | gzip -9 |
|---|---|---|
| As configured (`opt-level = 3` everywhere) | 11,818,369 | 3,914,429 |
| `harfrust`, `skrifa`, `read-fonts` at `opt-level = "s"` | 11,927,810 (+107 KiB) | 3,937,040 (+22 KiB) |

So per-crate `opt-level = "s"` makes the module bigger, not smaller (under LTO). The only
remaining lever found is a `wasm-opt` pass over the whole module. It is not installed and not
part of `frontend/scripts/build-wasm.sh`, and it is a build change for the whole app, not for
this slice. The other lever is the deferred module for the engine (`docs/technical-debt.md`). The 83 KiB over
budget is 0.7 % of the module. The customer decides.

## Gate

Run locally on the branch with the tests in this review:

- fmt, clippy (`-D warnings`), wasm32 build of every `-core` crate, `cargo deny`, `cargo doc` with
  `-D warnings`: green.
- `cargo nextest run --workspace`: 7,759 run, 7,757 passed, **2 failed**. Both are this review's
  tests on must-fixes 1 and 2: `ac22_two_texts_one_step` and
  `ac09_history_undo_in_edit_mode_takes_the_session_back`.
- Frontend: `npm run build` green, `npm test` 406/406, `oxlint` warnings only (no errors).
