# Parameter handle clone: a copy key on a parameter handle makes a copy instead of changing the shape

Status: Done (PR #499) (2026-11-26; the copy key is Shift, Frage 387 COPY-KEY answered by the customer 2026-11-26: Shift pressed during the drag, decided at the release)
Priority: Should
Origin: Customer (2026-10-19, translated): "If one presses Ctrl on the inner radius or the curve handle, a clone is made instead of a modification. Likewise a checkbox when one double-clicks on it." Decided by the customer on the same day: the copy is an ordinary copy like Duplicate (CURVE-CLONE-KIND A), it applies to all parameter handles (CURVE-CLONE-SCOPE A), and it gets no offset. The key was open because Ctrl already snaps on parameter handles and Alt at a press is the lasso; the customer chose Shift (Frage 387, 2026-11-26).

## User value

As a maker I want to pull a handle and get a new shape with the new value while the original stays as it is, so that a flower, a sun or a row of stars with different curves is made from one shape in a few drags, without Duplicate and Undo in between.

## Terms

- **Parameter handle**: a handle of the Select tool that changes one parameter of the one selected primitive. Exactly these exist today (`ParamHandle` in `curvyo-ui-core/src/param_handles.rs`):

  | Handle | On | Spec |
  |---|---|---|
  | Corner radius (four handles, one per corner) | rectangle | `0013`, `0009` |
  | Inner radius | star | `0009`, `0003` |
  | Arc Start and arc End | ellipse | `0021` |
  | Curve | ellipse, polygon, star | `0021` |
  | Rounded | polygon, star, ellipse without an arc | `0145` |

  There is no "sides" handle (Points, Randomized and the other values are Select bar fields, not handles); Rounded (`0145`) counts as a parameter handle (UX note V7). Handles that only share the look are not parameter handles here: text Position (`0080`), sketch instance (`0099`), tiling (`0069`), generator knobs (`0141`), Offset corners.
- **Copy**: an ordinary new object, as Duplicate makes it (`0041` criterion 23): new id, same style, same parent (layer, entered group, page) and the z-order position directly above the original, independent of it afterwards. Not a linked clone (`0049`).
- **Copy key** (Frage 387, answered 2026-11-26): **Shift pressed during the handle drag**; the state of Shift at the release decides. Snap keeps Ctrl (`0021` criteria 7, 23 and 41).
- **Key check done for this story:** a parameter handle drag reads no Shift today (`shape_knobs.rs`, `param_edit.rs`); Shift at the press keeps its meanings (the one-corner drag of `0013`, add to selection elsewhere), so a Shift held from the press is not the copy key until it is let go and pressed again during the drag (Proposal). Shift during a resize, rotate or skew drag keeps its meaning (about the centre, `0005`, `0008`).

## Acceptance criteria

1. **Shift during the drag.** Given the Select tool with one primitive selected, handles drawn, and a parameter handle drag running, when the maker presses Shift (before or after the 3 px dead zone), then the drag is a copy drag from that frame on; when Shift is released again, it is a plain drag again from that frame (held, not toggled). **The state of Shift at the pointer release decides** what is written (the `shiftKey` flag of the pointer-up, so a lost key-up cannot turn a visible copy into a change or back). A Shift held from the press counts only once it was let go and pressed again (Proposal: Shift at the press chose one corner, `0013`, and keeps that meaning alone).
2. **Press unchanged.** Given a press on a parameter handle, with or without Shift, then it starts the handle drag exactly as in `0009`, `0013`, `0021` and `0145` (Shift at the press does not toggle the selection) and the drag changes the shape until Shift is pressed during it. There is no copy by Shift at the press alone.
3. **Original stays.** Given a copy drag, then the original keeps all its parameters during the drag and after release (compared with its state at the press, every parameter and its outline).
4. **Result.** Given a drag that moves more than 3 screen pixels and is released with Shift down (pressed during the drag), then exactly one new object exists that equals the original in everything except the dragged parameter, which has the value a plain drag would have written; it lies at exactly the same position as the original (no offset), in the same parent directly above the original; the copy is selected and the original is not.
5. **Live preview (Proposal, UX notes V2, V6).** Given a copy drag in flight (Shift down), then: the original is drawn as committed, with no drag ghost; the copy is drawn as itself at the value the release would commit, directly above the original at the same place; the selection box, the transform handles and the other parameter handles stay on the original; the dragged handle keeps its dragging look at the copy's value; the "Copy" Modifier badge (design system, "Modifier badge") shows at the pointer; and the readout of the plain drag gets the suffix " · copy" ("curve 40.0 % · copy", "r 12.0 mm max · copy"; a limit word comes first). In the frame Shift is released the badge, the copy and the suffix are gone and the plain drag look returns. No badge shows before a handle drag runs, and Ctrl without Shift shows no badge and no "copy". The preview costs no more than the plain drag of the same handle (budgets of `0021` criterion 62). The preview reads the raw live Shift and Ctrl state.
6. **Escape restores.** Given a copy drag in flight, when the maker presses Escape (or the other button, a cancelled pointer, window blur), then no copy is made, the original and the document are as at the press, the selection is unchanged and no undo step exists. A press and release under 3 screen pixels of movement makes no copy and writes nothing, with or without Shift (`0021` criterion 23).
7. **Snap on a copy.** Given a copy drag of an arc handle, the curve handle of an ellipse, polygon or star, or the Rounded handle, then Ctrl and the automatic stops snap the copy's value exactly as `0021` criteria 7, 23 and 41 and `0145` say for the plain drag; the same drag with and without Shift gives the same value for the same pointer path. The live preview reads the raw Ctrl state, so a drag with Shift and Ctrl together previews the value it commits.
8. **Limited values.** Given a value the shape limits (`k_min`, half the shorter side, the inner-ratio limits), then the copy gets the limited value and the same limited readout as the plain drag ("max" and the like), no extra notice.
9. **Corner radius.** Given a rectangle's corner radius copy drag with the corners linked (or an unlinked drag by the Shift at the press, `0013`), then the copy gets what the plain drag would have written (all four radii, or only that corner) and the original keeps its radii; a one-corner drag by Shift at the press becomes a copy when Shift is let go and pressed again during the drag (Proposal).
10. **Arc on a whole ellipse.** Given a copy drag of an arc handle on a whole ellipse, then the copy becomes an arc by the rules of `0021` criterion 3 and the original stays a whole ellipse.
11. **Entry box.** Given the entry box of a parameter handle (double-click, `0021` criterion 8, 24 and 42, `0145` criterion 3), then it holds a check labelled "Copy" (the same check as in the Move entry chip: a check box inside the box, not a button, not a panel section; accessible name "Copy"; last row of the box). With it set, Enter applies the typed value to a copy exactly as criteria 3, 4 and 8 say, with the same limiting and the same invalid-value rules as the plain entry. The check starts unset every time the box opens, and Shift at the double-click does not preset it (it chose the corner scope, `0013`; Proposal, V4). **An untouched Enter with Copy set makes a copy at the current value** (Proposal, V5); without Copy an untouched Enter still writes nothing. Escape and a click elsewhere close the box and write nothing. The limit notice of the entry names the copy and shows the copy's value (the copy is selected before the notice is made). Every check (the object is still current, an invalid value) runs before the copy is made, so no failure leaves a plain, unlabelled copy.
12. **One commit.** Given criterion 4 or 11, then it is one commit with one undo step in the History tab (`0020`), named by the handle: "Copy with corner radius", "Copy with inner ratio", "Copy with arc", "Copy with curve", "Copy with rounding" (Proposal). One undo removes the copy and the selection returns to the original; one redo brings the copy back. The object history of the copy begins with the row "Duplicate of ..." (`0041` criterion 23 gets this third source next to Duplicate and the Ctrl-copy move).
13. **Other things are untouched.** Given a copy, then linked clones of the original (`0049`) still follow the original (unchanged), the copy is not one of them and not linked to the original; the copy is not made a member of any other page than the original's (`0099`); no `format_version` change (the copy is an ordinary primitive).
14. **Which handles do nothing new.** Given the node, resize, rotate, skew, centre, gradient and offset handles, the Select bar fields (Points, Radius, Ratio, Curve, Rounded, Randomized and the others) and a typed Move or Transform entry, then Shift has the meaning it has today and no copy is made.
15. **Objects without parameter handles.** Given a path (including a converted shape), a text, an image, a group, a linked clone (`0049`, `0021` criterion 55), a rectangle, ellipse, polygon or star that was converted to a path, or a **generated object** (`0141`), then there is no parameter handle and so no copy by this story. **Generated objects are excluded (Proposal):** their knobs (`curvyo-ui-core/src/instance_handles.rs`, snap = Shift) change the generator's values through their own mechanism, and the sketch-clone knobs of `0099` slice 2 set a clone's own value; both stay as they are, and a copy of a generated object remains Duplicate.
16. **Locked, hidden.** Given a hidden or locked object (own flag or an ancestor's), then it is never selected (`0050`), has no handle and gets no copy; given an entry box whose object was changed, hidden, locked or deleted since it opened, then Enter writes nothing and makes no copy. There is no multi-selection case: several objects get the group box, which has no parameter handles (`0009`: exactly one primitive), so there is nothing to copy.
17. **No clutter.** Given this story, then there is no hint text, no tip and no new panel section (`docs/design-system.md`, Interaction conventions); the Copy badge, the " · copy" readout suffix and the "Copy" check are the only new elements.
18. **Keyboard and screen reader.** Given the handles are pointer-only (`0021` criterion 65), then the entry check is reachable by Tab inside the box (field, Copy, back to the field), toggled by Space, and announced with its name and state ("Copy, check box, not checked"); Enter on the field or on the check applies; no new keyboard route to a copy exists.

## Out of scope

- A linked clone (`0049`): the customer chose an ordinary copy.
- An offset or a placement dialog for the copy; a copy that goes to another place.
- Copies of generated objects and of the sketch-clone values (own mechanism, criterion 15); copying from the Select bar fields.
- A keyboard route to a copy (`0021` criterion 65).

## Open questions

Frage 387 (COPY-KEY): answered by the customer 2026-11-26: **Shift** (pressed during the drag, its state at the release decides).

Considered: Alt (at the press it is the lasso, `0014` criteria 16 and 17, and Linux window managers take Alt+drag); Ctrl (it snaps the value on a parameter handle, `0021` criteria 7, 23 and 41, `0145`).

## UX notes

See `ux-notes.md` (ux-engineer): the Copy badge on a handle drag, the look of the "Copy" check inside the entry box, and the copy drag preview.

## Links

Requirements: R-EDIT-004. Builds on `0009`, `0013`, `0021`, `0145` (the handles), `0014` (modifiers), `0041` (object history), `0020` (undo). Not touched: `0049`, `0099` slice 2, `0141`. PR: #498
