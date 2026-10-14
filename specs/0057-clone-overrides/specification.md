# Clone overrides: own placement and own style

Status: Draft (UX notes and `adrs.md` exist, customer answers of 2026-10-14 applied; becomes Ready when the customer accepts ADR 0016 and answers or leaves the default of Frage 5 CHAIN-STYLE)
Priority: Should
Origin: Customer (feedback of 2026-10-14 after trying the linked clone of PR #121, translated: "Looks extremely good already, but a few things are missing: you must be able to override things, e.g. the position, so the clone no longer moves with the original when I want that. It should also be possible to override the style. But then you always have to override the whole style. I have no idea right now what this should look like in the UI, but several colours (fill, stroke) must be possible, e.g. for embroidery. Also having its own origin would be important for rotation; for scale we don't need it."). The last sentence was a miscommunication, see "Reading of the feedback". Customer answers of 2026-10-14 are listed in "Decided by the product owner". Every criterion marked **Proposal** is my own idea and is not accepted until the customer says so.

## User value

As a maker I want to break a linked clone away from its original in two separate ways (it stays where I put it when the original is moved, it carries its own complete style), each one visible and reversible, so that I can keep one master shape and still make one fish in twelve clones, each a different thread colour for embroidery, without unlinking the clones and losing the live link to the shape.

**What the other tools do, and what we do.**

- *Inkscape* (`<use>` clone): a clone may override single properties of its original; a property that is not overridden stays inherited. There is no switch back to the original other than deleting the property in the XML editor, and Edit > Clone offers no "stay where you are" mode.
- *Ink/Stitch*: the thread colour of an embroidery object is its fill or stroke colour; a clone shows the original's colours unless the `<use>` is restyled by hand in the XML editor.
- *Illustrator and Affinity* (symbol instances): an instance follows the symbol; to change its colours the maker breaks the link. Illustrator can recolour an instance through "Recolor artwork" without breaking it.
- *LightBurn*: virtual array copies cannot be restyled one by one.

**What we do the same:** a clone that keeps its look when the original is restyled once it has its own.
**What we do differently:**

- *All or nothing for the style.* A clone either shows its original's style or owns a complete one, never a mix of single properties. The maker never has to ask which property comes from where (the customer's "you always have to override the whole style").
- *Every state is shown in the clone's own block and can be switched back with one click*, with a notice that says what happened. Inkscape hides overrides in the XML editor.
- *The shape stays linked.* Own placement and own style keep the live link to the original's outline: node edits of the original still reach the clone. Unlink is for leaving the link, not for restyling.
- *Nothing jumps.* Every switch leaves the drawn outline and the picture exactly where they were.

## Reading of the feedback

1. "Override the position so the clone no longer moves with the original": **own placement**. Today a clone's matrix is relative to its original, so moving, turning or scaling the original moves the clone (`0049` criterion 12). With own placement the clone ignores transforms of the original as an object and keeps its place on the page.
2. "Override the style, always the whole style": **own style**. All-or-nothing, with a way back. It includes the opacity (customer, 2026-10-14).
3. "Several colours (fill, stroke), e.g. embroidery": every clone with own style has its own fill colour and its own stroke colour (the two paints a style has today). Customer example: one fish, several linked clones, each a rainbow colour. Several colours inside one object is a different, larger feature (out of scope).
4. "Own origin for rotation": **dropped.** The customer said on 2026-10-14 that this was a miscommunication: he meant only that the clone can be placed freely and then does not move when the original moves, which is own placement (1). A rotation pivot of a clone is not wanted. My earlier reading (a stored pivot per clone) and everything built on it is removed.

## Words used below

- **Follow state.** Placement and style of a clone each have two states, independent of each other. *Placement:* "Follows original" (today's behaviour, the default of every clone) or "Own placement". *Style:* "Original's" (today) or "Own".
- **Style** in this spec means the style a path has in the Style tab (`0007`, `0017`, `0018`): fill (on or off, colour with alpha), stroke (on or off, colour with alpha, width, cap, join, miter limit, dash pattern) and markers, **and the object opacity register** (`0050`; customer, 2026-10-14). They are owned or inherited together.
- **Effective style of an object:** for a path, its style and opacity; for a clone with own style, that style and opacity; for a clone without, the effective style of its direct original (so a chain B → A → O reads from the nearest object down the chain that has a style of its own, else from the root O). Without overrides this is the root's, as in `0049`. The effective opacity of a clone on the canvas is that opacity times the opacities of the clone's own ancestors (groups and layers above the clone itself, `0050`); the ancestors of the object it reads from do not count.
- **Held back:** a clone with own placement is held back when its original is transformed: its drawn outline stays, its matrix absorbs the change.
- **Transform of an object (as opposed to an edit of its nodes):** any command that maps the whole object by one affine map G: the Select tool's move, resize, rotate and skew (drag, typed entries, nudge), a Flip (`0056`), a group transform (`0023`) and the document resize that moves content (`0015`). A Node tool drag of every node is a node edit, not a transform (decision 1).
- **G**, **T** and **M** are as in `0049` ("Words used below" and criteria 12, 15 and 16): G the map of the gesture, M a clone's matrix, T the change of an object's drawn outline.
- Lengths are compared with the document's geometric tolerance, angles with 1e-9 rad.

## Acceptance criteria

The numbers are stable: a criterion that the customer's answers removed stays as a "Removed" line so that references in `adrs.md`, `docs/adr/0016-clone-overrides.md` and the UX notes keep their meaning. 42 numbers, 35 of them in force.

### Own placement

1. Given one or more selected clones (nothing else), then the Style tab's clone block (`0049` criterion 22) shows a row "Placement" with a two-state control, "Follows original" and "Own placement". It shows the common state; when the selected clones differ, neither state is pressed ("mixed"). Every clone that exists before this feature and every clone that Linked clone makes starts as "Follows original". The row is in the tree only for such a selection (hidden, not disabled).
2. Given a clone, when the maker switches it to "Own placement" or back to "Follows original", then nothing moves: every point of its drawn outline is unchanged (within 1e-9 mm), the matrix is unchanged, and only the state changes. One commit each, labelled `own_placement` and `follow_placement`. A one-line status notice (3 seconds) reads "Own placement. It stays put when the original moves." or "Follows its original's placement again."
3. Given a clone C with own placement and its original O, when O is transformed by a map G (a transform in the sense of "Words used below", on O alone or together with other objects), then C's drawn outline is unchanged (within 1e-6 mm), C's matrix becomes M·T⁻¹ (T = the change of O's drawn outline, computed as in `0049` adrs decision 4), and no clone is written that is not held back (a following clone is not written, `0049` criterion 12). One commit, with the label of the transform. Test: O the square of side 10 mm at (0, 0), C with M = translate (30, 0) and own placement. Moving O 5 mm right: O covers x = 5 to 15, C still covers x = 30 to 40. The same with "Follows original": C covers x = 35 to 45. Rotating O by 90° about its centre: C is unchanged; "Follows original": C turns by 90° in its local frame (`0049` criterion 12).
4. Given the same C, when a node, handle or segment of O is edited (drag, add, delete, change of kind, Join or Split keeping the object, a Node tool drag of every node), then C shows the edited outline under its matrix, as for a following clone (`0049` criterion 10). Own placement holds against transforms of O as an object, never against edits of its shape, so the clone is always O's shape in the place the maker gave it. Test: C as in criterion 3, O's top-right node dragged by (5, 5): C's top-right node moves by (5, 5).
5. Given a chain B → A → O, then each clone decides for itself and the clones behind it see the result. (a) A has own placement, O is transformed: A is held back, so B, which follows A, does not move either. (b) B has own placement, A follows O, O is moved 5 mm right: A moves, B is held back (B's matrix becomes M_B·T_A⁻¹). (c) B has own placement and A is moved by a gesture of its own (`0049` criterion 15): B is held back. Test for (b): O the square 0 to 10 mm, M_A = translate (30, 0), M_B = translate (0, 30); O moved by (5, 0): A covers x = 35 to 45, y = 0 to 10; B still covers x = 30 to 40, y = 30 to 40, and M_B' = translate (-5, 30).
6. Given a selection that holds a clone with own placement together with its original (or with any object of its chain), then the multi-object rule of `0049` criterion 16 applies and the state makes no difference: both are transformed by G and the clone keeps its place relative to the original. Given a clone moved on its own, its matrix becomes G·M (`0049` criterion 15) in either state.
7. Given a transform of O by a G (or a chain map T_Z) with an absolute determinant below 1e-9, so that M·T⁻¹ cannot be formed, then the own-placement clones of that case that are not selected are unlinked first (`0049` criterion 28 as changed by criterion 30, so they keep the style and opacity they showed) in the same commit, and the notice adds "2 clones became paths." Given the result would put any matrix or outline beyond ±1e7, then nothing is written, all or nothing, and the refusal names the first clone (`0049` criterion 17). Test: scaling O to zero width with the corner handle through zero is `0056`'s case; here a typed S of 0 on one axis unlinks two own-placement clones and says so.
8. Given a transform of O that holds back at least one clone, then a one-line status notice (3 seconds) says "1 clone with own placement stayed in place." / "3 clones with own placement stayed in place." It does not appear when no clone is held back, and never for a node edit.
9. Given a drag that transforms O, then a held-back clone is not previewed (its outline does not change); a drag that edits O's nodes previews every downstream clone as `0049` criterion 11 says, held back or not, with the same cap of 500.

### Own style

10. Given one or more selected clones (nothing else), then the clone block shows a row "Style" with a two-state control, "Original's" and "Own", with the same mixed rule as criterion 1. Every existing clone and every new clone starts as "Original's". The Stroke, Fill and Markers sections of the Style tab are present for a clone as section headers; the sections are collapsed and greyed (no control inside) while the state is "Original's" and open as for a path while it is "Own" (customer rule: the panel does not grow or lose sections for an operation).
11. Given a clone with "Original's", when the maker switches it to "Own", then the clone gets a complete style equal to the style it shows at that moment (its effective style): fill on or off and colour with alpha, stroke on or off, colour with alpha, width, cap, join, miter limit, dash pattern, markers, **and opacity**. All values are written in the one commit (`own_style`); the drawn result does not change (every pixel of the next frame equals the one before). Test: root with a red 0.5 mm stroke, a blue fill and opacity 80 %; clone switched to Own shows the same, its Stroke section shows red and 0.5 mm, and its opacity is 80 %.
12. Given a clone with own style, then every edit in the Stroke, Fill and Markers sections (`0017`, `0018`), every edit of its opacity and the eyedropper writes to that clone only. Later changes of the original's style or opacity do not reach it. Test: the root's stroke goes green and its opacity 50 %: the clone with own style stays red and at 80 %; a clone of the same root with "Original's" turns green and shows 50 %.
13. Given twelve clones of one original, each with its own style, then each can have a different fill colour and a different stroke colour (any RGBA, independent of each other and of the original), for example the colours of a rainbow for embroidery threads. After save and reopen every fill and stroke colour is the same to the last bit (`0049` criterion 37 extended).
14. Given a clone with own style, when the maker switches it to "Original's", then the clone shows its original's effective style and opacity again, its own style and opacity are removed (not kept in the background), and the notice reads "Shows its original's style again. Its own style was removed." (status, 3 seconds). One commit, `original_style`. Undo restores the own style and opacity with every value (criterion 33).
15. Given a chain B → A → O, then a clone with "Original's" shows the effective style of its direct original (the definition in "Words used below"). Test: A has own style blue; B (Original's) is blue; the style of O is changed: A and B stay blue; A is switched back to "Original's": A and B show O's style. **Default pending the customer** (Frage 5).
16. Given a clone with own style, then its object opacity register is its own, a number in 0..1 (the value copied at the switch, criterion 11), and the Opacity field of `0050` writes to it; the canvas shows it times the opacities of the clone's own ancestors, and nothing of the original's opacity or of the original's ancestors. Given a clone with "Original's", then its opacity is the `opacity` register of the object it reads from (criterion 15's rule; for B → A → O with A Own, A's register, not A's register times A's layers), times the opacities of the clone's own ancestors (`0049` criterion 49), and the Opacity field of `0050` shows a read-only readout and writes nothing; the tooltip says "Set Style to Own to change it." Given a selection of clones in both states, the Opacity field writes to the Own ones and the notice of criterion 18 names the others. Test: root P opacity 50 %, clone C Original's in a layer of opacity 50 %: C is shown at 25 %; C switched to Own (it copies P's 50 %, shown at 25 %) and set to 100 %: C is shown at 50 %; P is set to 10 %: C is unchanged. Chain: A Own at 40 % in a layer of 50 %, B Original's of A in a layer of 100 %: B is shown at 40 %, not 20 %.
17. Given a clone with own style, then stroke width, dash lengths and marker size are document millimetres and are not scaled by any matrix of the chain (`0049` criterion 9). The switches "Scale stroke width" and "Scale corner radius" do not apply to a clone.
18. Given a style edit, an opacity edit or an eyedropper apply on a selection that holds clones and other objects, then the edit is written to every selected object that owns its style (paths, compound paths and clones with own style); clones with "Original's" are left alone (`0049` criterion 24, extended), and a status notice (3 seconds) reads "2 clones show their original's style and were not changed." when at least one was left alone. The "Mixed" values of `0017` are computed from the objects that were written. A selection of only clones with "Original's" shows no style controls (criterion 10), so the panel has nothing to edit; the eyedropper leaves them alone with the same notice.

### Removed: own rotation pivot

The customer dropped the rotation pivot on 2026-10-14 (miscommunication, see "Reading of the feedback"). The numbers stay so that references keep their meaning.

19. Removed (own rotation pivot: row, states, marker; not wanted).
20. Removed (switch to and from an own pivot; typed X and Y; marker drag).
21. Removed (rotation about the own pivot).
22. Removed (scale and skew ignoring the own pivot; with no pivot there is nothing to ignore).
23. Removed (a pivot carried by every transform).
24. Removed (hint chip for a selection with a pivoted clone).
25. Removed (a copy inheriting the pivot; the ring-by-Duplicate proposal falls with it).

### Switching, notices and the panel

26. Given the clone block, then its two rows ("Placement", "Style") sit under the text and the buttons of `0049` criterion 22 and above the Style sections; both rows are always in the tree for a clone selection, so no row appears or disappears when a state changes. The block stays inline, no popup. Each control has a tooltip of one sentence saying what the state does.
27. Given a selection that holds a clone, then keyboard access and names are as `0049` ("Keyboard, screen reader"): every control is a Tab stop in the block, announces its state (pressed or not, mixed), and no key starts any of the switches (the overall shortcut review at the end, `README.md`).
28. Given the Layers tab (`0050`), then the row of a clone with any override shows no new column or glyph; its tooltip gets a second muted line, "Own placement, own style" (only the parts that apply). Hide, lock and name of a clone are its own as before. Its opacity readout is the slider/field when the style is Own (criterion 16).
29. **Proposal:** given a selected clone with any override, then its canvas tag (`0049` criterion 57) reads "Clone, own placement", "Clone, own style" or "Clone, own placement and style". The tag keeps the placement rules of `0049` criterion 60. The customer may veto this without touching the rest.

### Interplay with the other commands

30. Given Unlink clone (`0049` criterion 28) on a clone with overrides, then the new path has the drawn outline it had (every point within 1e-9 mm), its effective style (its own style if it had one, else that of the object it read from) and the `opacity` register value it showed (own, else that of the object it read from; not multiplied by its ancestors, which still apply to the path in the same place), `rotation` 0, and no placement or style state. Clones that followed it keep following it and read the path's style as usual.
31. Given a command that removes an object (`0049` criterion 31), then (a) a re-pointed clone keeps its placement state and its own style; (b) a clone without own style whose style source (criterion 15) is a removed clone gets a copy of that source's style and opacity as its own, so that its look does not change; when its source stays, nothing is copied; (c) a clone that is unlinked gets its effective style and opacity and loses the two states. Test: B → A → O, A has own style blue, B has "Original's": Delete A: B follows O, shows blue (own style), and its drawn outline is unchanged.
32. Given Duplicate, a Ctrl-copy move or the copy of a set of a clone with overrides, then the copy has the same two states: the same placement state and a copy of the own style and opacity (independent: a later edit of the source's style does not reach the copy). The relink rules of `0049` criteria 19, 47 and 48 are unchanged. Given a copy of an original with its clones, an own-placement clone keeps its state.
33. Given `0020` has merged, then every switch is one step with the operation names "Own placement", "Follow original's placement", "Own style", "Original's style" (the guard of `0020` criterion 41), and "Spread colours" if criterion 42 is accepted. Undo of "Original's style" restores the removed own style and opacity with every value; undo of a transform that held clones back restores their matrices (the link as a whole, `0049` criterion 36). Given object undo (`0041`) and `0042`, then the two states are ordinary fields of the clone: object undo and Restore bring them back field by field, a Preview of an earlier version shows the clone as it was then (for example with "Original's" before it got its own style), and a clone of a state (`0042`) carries them.
34. Given two peers, then (a) edits of the fields of one clone's own style merge field by field like the style of a path; (b) when one peer switches a clone to "Own" while another edits the original's style, the clone ends with the copy taken at the switch and shows no later edit of the original; (c) when one peer switches a clone to "Own placement" while another transforms its original, the clone may end up moved by that transform once (the holding back is done by the writer who knew the state); nothing is lost, no clone becomes an orphan, and the maker can move it back. Replicas converge.
35. Given a Flip (`0056`) of the original, then own-placement clones are held back (criterion 3 with a G of determinant -1, which can be inverted), and following clones are mirrored in their local frames. Given a Flip of a clone, its matrix gets the negative determinant as in `0056`.
36. Given the orphan repair (`0049` criterion 35), then a clone whose original is not found is removed whatever its overrides are; the notice is unchanged. Overrides do not make a clone survive: it has no outline to keep.

### Saving, file and output

37. Given a project with clones that have overrides is saved and reopened, then every state, the own style, the own opacity and the matrix are the same to the last bit, and every clone draws as before. `format_version` goes to the next free number at merge (`specs/README.md`, "`format_version` plan"; 14 if `0049` has taken 13). Every file saved by this build, with or without overrides, is refused by an earlier build with the "saved by a newer version" message (as for any bump); for files with overrides this matters, because an earlier build would draw a clone with own style in its original's colours and would not hold back a clone with own placement; a file from an earlier build opens unchanged and is not rewritten. A golden file `clone_overrides_v<N>.curvyo` pins the format: a clone with each override alone, one with both, and a chain whose middle clone has own style.
38. Given a file with a clone whose placement or style flag is not a boolean, a clone with own style whose style value would be damaged on a path (`0017`), or any node with an opacity outside 0..1, then the file is refused as damaged and the editor does not crash. Override flags on a node that is not a clone, and style keys and an `opacity` register on a clone without own style, are ignored as merge residue (ADR 0015 §4 as extended by ADR 0016 §6); a missing style key of an own style reads as its default and is not damage.
39. Given the `document.json` view, then a clone lists its placement state and whether it has an own style (with the style and opacity), next to its original and matrix. It is non-authoritative (ADR 0004 §1).
40. Given any output (SVG export, a job, a preview, an embroidery file later), then a clone is a path with the resolved outline and its effective style; the path read of a clone with overrides equals the path read of the same clone after Unlink clone (anchors within 1e-9 mm, style equal). A later embroidery export reads the fill and stroke colour of each such path as its thread colour; that mapping belongs to its own spec. The SVG export (`0024`, not yet specified) writes a clone with any override as an ordinary `<path>`; whether a plain clone is ever written as `<use>` is `0049` Question 7.

### Performance

Measured on the CI runner, release build, native; the browser build may take up to twice as long.

41. Given 10,000 clones of one path of 100 nodes, all with own style, then reading all objects takes at most 300 ms (the budget of `0049` criterion 43; own style adds no outline transformation). Given the same 10,000 clones with own styles that differ from the root's only in fill and stroke colour, then the next frame stays within 150 ms (`0049` criterion 44): the shared root tessellation must not be given up for a colour (ADR 0016 §4). Given the same 10,000 clones with own placement and the original moved, then the commit, including the 10,000 matrix writes, finishes within 3 s and the next frame within 150 ms after it (default of the product owner, to be measured; not gated before the first measurement).

### Colours for many clones

42. **Proposal:** the clone block always contains a button "Spread colours" while a clone is selected. With fewer than two clones selected it is present and disabled (`aria-disabled`, still focusable so that the tooltip can be read; it is not hidden), and its tooltip says why: "Select two or more clones to spread colours." With two or more selected clones it is enabled; pressing it switches the clones with "Original's" to "Own" and gives the clones, in stacking order from the back, evenly spaced hues (360° divided by the number of clones) starting at the hue of the first clone's fill colour (its stroke colour if the fill is off), keeping each clone's saturation (a grey stays grey), lightness, alpha and opacity, for fill and, where the stroke is on, stroke. One commit, `spread_colours`; undo restores every colour. Test: twelve clones with the same red fill get hues 0, 30, ... 330 degrees; with one clone selected the button is in the tree, disabled, with the tooltip above, and pressing it changes nothing and posts nothing. The customer may veto this without touching the rest.

## Changes to accepted specs

Decided with the customer's answers of 2026-10-14. Each row is a change to something the customer accepted before; the architect's ADR 0016 (Proposed, `needs-customer`) carries the document-model side.

| Accepted text | Was | Becomes |
|---|---|---|
| `0049` decision 2 and criterion 8 | A clone has no style of its own and shows the root's | A clone may own a complete style (this spec, criteria 10 to 18); without it, it shows the effective style of its direct original (criterion 15, default pending) |
| `0049` criteria 12 and 16 | Clones that are not selected follow a transform of their original | Clones with own placement are held back instead (criteria 3 to 7); following clones are unchanged |
| `0049` criterion 22 | A selection of only clones shows no style section | The Stroke, Fill and Markers headers are shown, collapsed and greyed, with Original's (criterion 10) |
| `0049` criterion 24 | A style edit never touches clones | Clones with own style are edited; clones with "Original's" are skipped with a notice (criterion 18) |
| `0049` criterion 28 | Unlink copies the root's style and opacity | Unlink copies the effective style and opacity (criterion 30) |
| `0049` criterion 49 | A clone has no opacity register; its opacity is the root's | A clone with Own style has its own `opacity` register (criterion 16); with "Original's" it reads the effective opacity as before |
| `0050` criteria 24, 26 and 28 | The Opacity field never writes to a clone; a clone has no `opacity` register; the caption for a target of only clones reads "Linked clones show the opacity of their original." | The field writes to a clone with Own style and shows a read-only readout for the others (criterion 16); the caption applies to Original's clones only; the read hands over the opacity of the style source |
| ADR 0015 §3 to §5 | A clone reads exactly outline, `Style` and `opacity` from its root; a repair copies no style; residue lists only `clone_link` as clone data | A clone reads style and opacity from its style source, itself if it has Own style (ADR 0016 §3); a repair copies a lost source's style (§4); the two flags and clone-level style keys are known keys (§5) |
| `0049` criteria 19, 31, 47, 48 | A copy and a repair carry the link | They also carry the two states and the own style (criteria 31, 32) |

## Decided by the product owner

Change any of these if you disagree. Lines marked "Customer" were decided by the customer on 2026-10-14 and are no longer mine to change.

1. **Own placement is a flag with a held-back rule, not a second kind of matrix.** Because every transform is baked into the original's anchors (`0049` decision 6), the original cannot tell a clone how it was moved. The clone keeps the same relative matrix in both states; the state only decides what a transform command does to it (held back or carried along). Consequences the customer sees: switching either way never moves anything; the Node tool moving every node of the original counts as a shape edit and moves the clone (criterion 4); the architect decided (ADR 0016 §1) that the flag is its own register and not part of the atomic link value: it needs no coherence with the matrix, and a toggle must not contend with the maker's move of the same clone.
2. **Style means the Style tab's style plus the opacity.** Fill, stroke (colour, width, cap, join, miter, dash), markers and opacity. Everything in it is owned or inherited together.
3. **Customer, 2026-10-14 (Frage 4, B): the own style includes the opacity.** A clone with Own style has its own opacity; with "Original's" it shows its original's. This changes accepted text; see "Changes to accepted specs". It replaces my earlier decision that opacity stays out.
4. **Back to the original's style discards the own style and opacity.** Keeping it dormant would be a hidden state that comes back unasked on the next switch. Undo restores it (criterion 14).
5. **A clone without own style shows the effective style of its direct original** (criterion 15), so a restyled middle clone of a chain passes its look on. Without overrides this is the root's style, as in `0049`. **Default pending the customer** (Frage 5); ADR 0016 follows the default.
6. **Removed.** (The pivot as a page point.) Customer, 2026-10-14: no rotation pivot.
7. **Removed.** (The pivot for rotation of one clone.)
8. **Removed.** (Inheriting the pivot, the ring by Duplicate.)
9. **A held-back clone that cannot be held back is unlinked, not refused** (criterion 7), in the same commit and with a notice: the operator does not have to find out why a scale did not work.
10. **A style edit on a mixed selection skips following clones with a notice** (criterion 18) instead of silently turning them into own-style clones, which would hide a mass override.
11. **Two switches in the existing clone block**, not new sections (criterion 26). The block exists only for clones, already has a fixed place in the Style tab and gets two fixed rows. No row appears or disappears when a state changes.
12. **Removed.** (A movable pivot although `0008` refused one.) `0008` stands: no movable pivot.
13. **Overrides need a new ADR** (ADR 0015 says named styles, overrides or relink need one): [ADR 0016](../../docs/adr/0016-clone-overrides.md), Proposed, `needs-customer`. **Approval by the customer is pending.** The customer already accepted that a clone reads from its original (2026-10-11); this spec adds that a clone may own a complete style and opacity.
14. **Customer, 2026-10-14 (Frage 1, A): the switch is a two-state control "Style: Original's | Own" in the clone block**; the Stroke, Fill and Markers headers stay in the tree, collapsed and greyed until Own.
15. **Customer, 2026-10-14 (Frage 3, A): each clone has its own fill and stroke colour.** Customer example: one fish, several linked clones, each a rainbow colour for embroidery. Several fill or stroke layers in one object are out of scope.
16. **Customer, 2026-10-14 (Frage 2): no rotation pivot, no own origin.** The sentence in his feedback was a miscommunication; own placement is what he meant.
17. **Nothing appears out of nowhere (customer rule): the Spread colours button is always present and disabled with fewer than two clones, with a tooltip that says why** (criterion 42, Proposal).

## Out of scope

- **A rotation pivot or own origin of a clone** (not wanted, customer 2026-10-14), and a movable pivot for any object (`0008` stands).
- **Single-property overrides** (only the stroke colour, inherit the rest). The customer asked for all or nothing.
- **Several fill or stroke layers in one object**, gradients (removed in `0017`), and patterns.
- **Thread palettes, thread brand colours, colour matching** for embroidery (`0022`) and the embroidery output itself. This spec only makes sure each clone can carry its own fill and stroke colour.
- **Relink**, an "apply to all clones" command, copy-and-paste of a style between clones (the eyedropper of `0017` already does that), and a "Reset all overrides" command (two one-click switches exist).
- **Tiled or arrayed clones** from one dialog (`0049` out of scope).
- **Keyboard shortcuts** for any switch (overall review at the end).
- **What a hidden or locked clone does in a machine output** (`0050`).
- **SVG `<use>` import and export** (`0024`).

## Open questions

Frage 1 to 4 are answered (below). One question is open and has a default.

**Frage 1 (STYLE-UI): Where the maker switches a clone to its own style.** Answered 2026-10-14: A, a two-state control "Style: Original's | Own" in the clone block, with the Stroke, Fill and Markers headers collapsed and greyed until Own (criterion 10).

**Frage 2 (PIVOT): What "own origin for rotation" means.** Answered 2026-10-14: dropped; it was a miscommunication (see "Reading of the feedback").

**Frage 3 (COLOURS): What "several colours (fill, stroke)" means.** Answered 2026-10-14: A, each clone has its own fill and its own stroke colour.

**Frage 4 (OPACITY): Does the own style include the opacity?** Answered 2026-10-14: B, yes (criteria 11, 14, 16).

**Frage 5 (CHAIN-STYLE): What a clone without own style shows when its original is a clone with own style**

- *A (default, recommended):* the style (and opacity) of the nearest clone on its chain that has one (decision 5). A new clone of a blue tab is blue; deleting a blue middle clone copies its style onto the clone behind it.
- *B:* always the root's style. Simpler, but a clone of a blue tab turns up in the root's colour.
- Recommendation A. Default if unanswered: A (ADR 0016). **Default pending customer.**

## UX notes

Written by `ux-engineer`, 2026-10-14, after the customer's answers of the same day: Frage 1 = A, Frage 3 = A, Frage 4 = B (the own style includes the opacity), and the own rotation pivot is dropped (a miscommunication: the customer meant only that a clone can be placed freely and then stays where it is). So the clone block gets **two** fixed rows, Placement and Style, and there is no pivot row, marker, typed X and Y, or reset. The criteria carrying the pivot and the old opacity rule were rewritten by the product owner on the same day (criteria 19 to 25 are "Removed" placeholders). Numbers are `docs/design-system.md` rows "Properties panel: linked clone", "Action notice", "Linked clone marks" and the segmented-control pattern. Rules held from the customer: the panel never grows or loses a section for an operation; nothing appears out of nowhere; an operation has a visible trigger. The explanatory lines of `0049` ("It shows the style of its original.", "1 linked clone follows this object.") stay removed (PR #121) and nothing like them comes back: the state is read from the controls. **Product owner's correction, 2026-10-14 (customer rule: nothing appears out of nowhere):** Spread colours is not "in the tree only with two or more clones"; it is always present while a clone is selected and disabled with fewer than two (criterion 42). The `ux-engineer` adjusts the numbers (block height, Tab order, tooltip) accordingly.

### The clone block (Style tab)

Order: the text and buttons of `0049` (Select original, Select root original for chains, Unlink clone), then **Placement**, then **Style**, then **Spread colours** (criterion 42, Proposal; disabled with fewer than two clones), then the Style sections. Same inline block, `role="group"` named "Linked clone", no popup. Two rows, always in the tree for a clone selection; a state change moves nothing.

| Item | Value |
|---|---|
| Row | 8 px above. Label 12 px `--panel-muted-fg` ("Placement", "Style"), 4 px, then a **two-segment text pill**, full width 244 x 28 px (122 px per segment), 1 px `--toolbar-icon` at 60 % outline, `rounded-[5px]`, 14 px labels; the pressed segment has the `--accent` ground with white text, as the other segmented groups. Row height 48 px. Block grows by 104 px (about 216 px, 268 px for a chain, 288 px with Spread colours); the tab scrolls, as it already does at 800 x 600 |
| Placement | Segments **Follows original** / **Own**. Group `role="radiogroup"`, `aria-label` "Placement". Tooltips per segment (text, `side="left"`, 400 ms): "Moves, turns and scales with the original." / "Stays where it is when the original is moved." |
| Style | Segments **Original's** / **Own**. `aria-label` "Style". Tooltips: "Shows the original's fill, stroke, markers and opacity." / "Has its own fill, stroke, markers and opacity." |
| Labels | The criteria call the segments "Follows original", "Own placement", "Original's", "Own". In the pill both second segments read **Own**; the row label supplies the noun. `aria-label` of the items stays whole: "Follows original", "Own placement", "Original's style", "Own style" |
| Mixed | Several clones that differ: no segment pressed, no segment highlighted (the `0007` mixed rule for segmented controls); `aria-describedby` points to a visually hidden "Mixed". Pressing a segment sets all selected clones in one commit |
| Same state | Pressing the pressed segment does nothing and posts nothing |
| Keys | One Tab stop per group (roving tabindex), arrows move and set the state (the established radiogroup), Space and Enter set it. Tab order: Select original, Select root original, Unlink clone, Placement, Style, Spread colours (also when disabled, `aria-disabled`), then the first control of the Style area. Escape does nothing special. No shortcut starts a switch (criterion 27). Route from a bare canvas: select the clone, Shift+Ctrl+F, Tab |
| Focus | A switch does not move focus away from the group. The group stays where it is, so a key user can set and unset at once. A press with the mouse returns focus to the canvas (`0016` criterion 22a) |
| Notices | Action notice, status, 3 s, 4 px below the row, right-aligned, max 70 characters, never take focus. Placement: "Own placement. It stays put when the original moves." (52) / "Follows its original's placement again." (39). Style, to Own: "Own style, copied from the original." (36) / back: "Shows its original's style again. Its own style was removed." (60). Held back: "1 clone with own placement stayed in place." / "3 clones with own placement stayed in place." (44), shown in the canvas notice slot of the transform that caused it, never for a node edit. Zero-size case: "Zero size. 2 clones with own placement became paths." (52). Mixed edit: "1 clone shows its original's style and was not changed." / "2 clones show their original's style and were not changed." (58), anchored where the edit was made. Refusals keep `0049` criterion 17 |

### Style sections for a clone (Frage 1, A)

- Stroke, Fill and Markers are in the tree for every clone selection as header rows, 28 px, in the place the sections have for a path. With **Original's** a header is **collapsed and greyed**: the title in `--panel-muted-fg` (5.0:1, so the grey is still readable), a muted summary at the right (a 12 px swatch with `--checker-a/b` for alpha and "0.5 mm" for the stroke; a swatch for the fill; "None" or the marker name), no chevron, no control inside, not a Tab stop, `role="heading"`. With **Own** the sections open as for a path, with every control.
- **Exception, deliberate.** `0017` keeps what does not apply out of the tree. This header holds no control, so there is nothing inert to offer; it exists so the panel keeps its shape (the customer's rule) and the maker sees which properties the clone has. It is not a disabled control and it is not announced as one.
- The Style area is never empty and never changes height between Original's and Own more than the open sections do (they open and close with the switch, which is the maker's own action, one commit).
- A selection of clones in both states: the sections are open, the controls write to the Own ones, the notice of criterion 18 names the skipped ones. Mixed values are computed from the written ones (`0017`). Only Original's clones: all three greyed.
- **Opacity (Frage 4, B).** The Style tab has no opacity field, and none is added. The opacity of a clone with Own style is the slider and field at the bottom of the Layers tab (`0050`), which writes to the clone's own opacity. For a selection that holds only Original's clones that control is replaced by the muted read-only readout `0049` criterion 49 already has (no inert slider); its tooltip says "Set Style to Own to change it." A mixed selection edits the Own ones and skips the others with the notice above. The switch to Own copies the effective opacity along with the rest, so nothing changes in the frame.
- **Rejected options.** B (the first edit silently starts the override): the state changes as a side effect and a mixed selection needs extra rules. C (a button "Own style" in the Select bar): a second trigger far from the state; the state and the way back would still sit in the block.

### On the canvas

- **Nothing marks an unselected clone**, own placement or not: the picture is also the production picture, and a clone that stays in place looks like any shape (`0049`, "On the canvas").
- A selected clone keeps the marks of `0049` unchanged (violet dashed trace, `Link2` chip). The only addition is the tag text, with the same size, border and placement rules: "Clone, own placement", "Clone, own style", "Clone, own placement and style" (criterion 29, now a firm part of the notes; the customer may still veto it). No new glyph, colour or dash: a cue by text, as the marks of `0049` already rely on text and not on hue. Tags are dropped when none fits, as before; the segmented control carries the state.
- **No popup, no preview, no handle.** The held-back clone is not previewed (criterion 9); an original's drag shows no outline for it. Switching placement changes nothing on screen, so the notice is the only feedback.
- Layers: row tooltip second muted line "Own placement, own style" (criterion 28); accessible name appends it. The opacity readout turns into the slider when the clone's style is Own.

### Several colours, for example embroidery (Frage 3, A)

What exists with this spec, for one fish and twelve clones: switch a clone to Own, then set its Fill and Stroke colours in the Style sections (swatch, hex field, or the eyedropper of `0017`); with several clones selected a colour edit writes **the same** colour to all of them. So twelve colours take twelve rounds of "select one clone, Own, pick a colour", about four actions each. That is too slow for the customer's example. The ordinary route also has no way to give the clones a graded series.

- **Proposal, criterion 42: Spread colours.** A full-width outline button (244 x 28 px, the look of Select original) in the clone block, **always present while a clone is selected; disabled (`aria-disabled`, focusable) with fewer than two clones, tooltip "Select two or more clones to spread colours."** Press with two or more: in the selection's stacking order (back to front), the clones get evenly spaced hues around the colour wheel, starting at the hue of the first clone's fill; saturation, lightness, alpha and opacity of each clone's own fill stay, the stroke colour gets the same hue where the stroke is on; clones with Original's are switched to Own in the same commit first. One step, label "Spread colours", notice "Spread hues over 12 clones." (30). Undo restores every colour. Tooltip when enabled: "Gives the selected clones evenly spaced hues, in stacking order." No dialog, no preview, no setting; a maker who wants other colours edits single clones afterwards. The customer may veto it without touching the rest. The first-click sequence for the rainbow fish is: select the clones (marquee or `Select clones`), press Spread colours.
- Thread palettes and matching stay out of scope (`0022`).

### Large documents, undo, error and empty states

- A switch on N selected clones is one commit; 10,000 clones stay within the budget of criterion 41. No progress UI; if a commit exceeds 250 ms the busy state of `0016` applies.
- Every switch is one undo step with the names of criterion 33; undo of "Original's style" brings back the own style and opacity with every value.
- Empty state: a clone that has never been touched shows Follows original / Original's with Stroke, Fill, Markers greyed. No error state exists for the switches themselves; the refusal for beyond ±1e7 is `0049` criterion 17.

### Checks for the review

Tab order as above; both groups reachable and operable without the mouse; mixed selection shows no pressed segment; greyed headers pass 4.5:1 and are not Tab stops; the block never moves when a state changes; no notice above 70 characters; no line of explanatory text in the block; Spread colours is present and disabled with fewer than two clones and enabled with two or more; the Layers opacity slider swaps with the readout by state.

## Links

Requirements: R-EDIT-036 (`docs/requirements.md`); related R-EDIT-030, R-EDIT-012.
Builds on: `specs/0049-linked-clone/` (built in PR #121, not merged; this spec extends its clone block, its matrix rules and `clone_link`), `specs/0017-style-panel-rework/` and `specs/0018-stroke-markers/` (what a style is), `specs/0019-multi-object-transform/` (group box), `specs/0050-layers-and-objects-panel/` (opacity, flags, row tooltips)
Amends: see "Changes to accepted specs".
Related: `specs/0056-mirror-and-negative-scale/` (Flip, criterion 35), `specs/0042-history-branches/` and `specs/0041-object-history/` (criterion 33), `specs/0020-undo-redo/`, `specs/0022-color-management/` (thread palettes, out of scope), `specs/0024` (SVG, not yet specified)
ADRs: [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) (the link register and "named styles, style overrides or a relink command need a new ADR"); [ADR 0016](../../docs/adr/0016-clone-overrides.md) (Proposed, `needs-customer`, approval pending); see `adrs.md`.
Format: next free `format_version` at merge (14 if `0049` takes 13).
PR: TBD
