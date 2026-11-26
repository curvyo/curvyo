# 0073 Parameter handle clone: UX notes

ux-engineer, 2026-11-26. Written from `specification.md` (Draft, criteria 1 to 8) for the copy
key **Shift** (COPY-KEY, Frage 387, answered by the customer 2026-11-26; first written for Alt,
changed to Shift by the lead with the implementation).
Checked against `docs/design-system.md` (rows "Parameter handle", "Drag ghost", "Live transform
readout", "Modifier badge", "Move entry chip", "Transform entry chip", "Limited tag", "Corner
radius readout and chip texts", "Offset handles", "Cloning drag", "Bend a segment", "Group handle
layout"; conventions "Modifier indicators in a move", "Marquee and lasso cursors", "Numeric entry
chip", "No notice after a success"), `specs/0021-ellipse-arcs-and-shaping/` (criteria 6 to 8, 23 to
25, 41 to 43, UX notes), `specs/0145-shape-spiro-handles/ux-notes.md` §3 and §5, `specs/0060-snapping/`
criterion 9, and the code: `curvyo-ui-core/src/select_tool/press.rs` (`arms_lasso`),
`frontend/src/components/MoveEntryChip.tsx` (the Copy check), `TransformEntryChip.tsx` (the
parameter chips). No new colour token, glyph, component kind, panel section, hint line or notice.
**(Proposal)** items are vetoable at the demo and listed as V1 to V7 in section 9; section 10
lists the spec deltas K1 to K5 for the product owner. Distances are screen pixels.

Terms: "knob" is a parameter handle (row "Parameter handle"). "Copy drag" is a knob drag that
would make a copy if released now. "Change drag" is today's knob drag.

## 1. Principle

A copy drag looks like a Ctrl move copy (convention "Modifier indicators in a move"): the original
stays drawn exactly as committed and keeps its selection box; the copy is drawn as itself on top;
a plus badge and the word "copy" in the readout say which of the two is happening. The state of
the current frame decides what is drawn; the state at the release decides what is written
(criterion 1). Nothing else on screen changes.

## 2. The key: when Shift counts (criterion 1)

- **Shift pressed at any time during the knob drag** (after the press, before or after the 3 px
  dead zone) makes it a copy drag in the frame of the keydown; releasing Shift makes it a change
  drag again in the frame of the keyup. Held, not toggled: a held Shift is a copy, a released one is
  not.
- The press is unchanged: Shift at a press on a knob starts the knob drag and does not toggle the
  selection; on a corner radius knob it chooses one corner (`0013`). A Shift held from the press is
  therefore no copy until it is let go and pressed again during the drag **(Proposal V1)**.
- The live state is taken from window-level key events and corrected by the `shiftKey` flag of
  every pointer event; **the release is decided by the `shiftKey` flag of the pointer-up** (the
  "Cloning drag" rule), so a lost keyup cannot turn a visible copy into a change or back.
- Shift keeps its meaning on every other handle (resize about the centre, rotate steps, skew) and
  in a move (axis lock).

Considered and not chosen: Alt (at a press it arms the lasso, `0014` criteria 16 and 17, and Linux
window managers take Alt+drag at the press); Ctrl (it snaps the value on a knob, `0021` criteria 7,
23 and 41, `0145` §3).

## 3. What the canvas shows during a copy drag (criteria 1, 2)

Scope: every primitive parameter knob of the Select tool: Curve, Start, End, inner radius, corner
radius and the Rounded knob of `0145`; not knobs that only share the look (text Position `0080`,
sketch instance `0099`, tiling `0069`, generator knobs, Offset corners) **(Proposal V7)**.

In the frame Shift is pressed and for as long as it is held:

| Element | Change drag (today) | Copy drag |
|---|---|---|
| Original | drawn as itself at the new value; its committed outline stays as the "Drag ghost" | drawn as itself **exactly as committed**; no ghost |
| Copy | (none) | drawn as itself (fill, stroke, picture) at the new value, directly above the original in its stacking place, at the same position, no offset |
| Selection box, other knobs, transform handles | follow the new geometry | **stay on the original** (the move-copy mark, "Modifier indicators in a move") **(Proposal V6)** |
| Dragged knob | dragging look, at the new value | dragging look, at the copy's value under the pointer (unchanged look) |
| Parameter handle guide, Rounded ticks | as today | as today, on the copy's value |
| Cursor | built-in `pointer` | built-in `pointer` ("Modifiers never change a handle cursor") |
| Modifier badge | none | **Copy badge** (plus on solid `--accent`) 16 px left of and below the hotspot, its placement rules; gone in the frame Shift is released |
| Readout | "curve 40.0 %" | "curve 40.0 % · copy" |

- Where the copy covers the original (same fill, a larger value), the copy is what shows; where the
  copy is smaller, the original shows around it. This is the honest result and needs no extra cue:
  the badge, the word and the box left on the original carry the difference.
- Readout words: the existing parameter readout of the dragged knob, unchanged, then " · copy"
  (lowercase, the " · " of "· all corners") **(Proposal V2)**: "curve −35.0 % · copy", "start 30.0° ·
  sweep 120.0° · copy", "r 3.5 mm · copy", "r 12.3 mm · all corners · copy", "ratio 0.45 · copy",
  "rounded 0.35 · copy". At a limit the limit word comes first: "r 12.0 mm max · copy", "rounded
  10.00 max · copy".
- Before the press no copy badge is shown over a knob (a Shift at the press is no copy; the badge
  never lies). The badge appears only once a knob drag is running and Shift counts as the copy key.
- A switch mid-drag swaps the two columns in the frame of the key event; the readout keeps its place.
- Escape (first step of the cascade), the other button, a cancelled pointer and window blur cancel
  either kind: nothing is written, the badge and the copy are gone in that frame. A press and
  release under 3 px writes nothing, with or without Shift.
- Ctrl (the snaps of `0021` criteria 7, 23, 41 and `0145` §3) and the Link corners inversion
  (Shift at the press) keep working in a copy drag and combine with it.
- **Ctrl without Shift in a knob drag shows no badge and no copy word; only Shift does.** The move's
  Ctrl-keyed plus badge and " Copy" readout rules must not be reused for knobs: there Ctrl snaps.

## 4. After the release (criteria 2, 4)

- Shift down at the pointer-up: one new object, at the same place, directly above the original, with
  the dragged value; **the copy is selected, the original is not**; the copy's box and knobs are
  drawn at once (the dragged knob in its idle look under the pointer). Shift up: the original changed,
  as today.
- No notice, no toast, no announcement beyond the selection change (rule "No notice after a
  success"). The History step label is the product owner's ("Copy with curve" and so on).
- Undo removes the copy and selects the original again (`0020` criterion 23: the selection before
  the step), so the maker is back where he was before the drag.

## 5. The check in the entry box (criterion 3)

The parameter entry chips (`TransformEntryChip.tsx` kinds `curve`, `arc-angle`, `corner-radius`,
`inner-ratio`, `rounded`) get the Copy check of the Move entry chip, same component and look:

- **Label "Copy"**, not "As copy" **(Proposal V3)**: the Move entry chip already has a check "Copy"
  for the same meaning; one word, one meaning (K1).
- **Place:** the last row of the chip, 6 px below the row above it (on the corner radius chip below
  the muted "All four corners" / "This corner only" row), left-aligned with the field. A 14 px
  native checkbox (`accent-[var(--editor-accent)]`), 6 px gap, the 12 px label; the whole 24 px
  high label row is the hit target (`label` wraps the input, `cursor-pointer`). The chip grows by
  30 px; its placement rule is unchanged (flip and clamp use the full height).
- **Starts unset every time** the chip opens (criterion 3), also when the previous Enter used it,
  and also with Shift held at the double-click **(Proposal V4)**. This differs from the Move chip,
  which presets the check from Ctrl at the second press; here Shift at the press chooses the corner
  scope (`0013`).
- **Keyboard:** the field takes the focus with its text selected, as today. Tab: field, Copy, back to
  the field; Shift+Tab reverses; the chip is a closed loop (the Move chip's `loopFocus`). Space on the
  check toggles it (native). Enter in the field or on the check applies. Escape closes from either
  and writes nothing. Key events in the chip never reach canvas shortcuts.
- **What Enter does with the check set:** the typed value goes to a new object exactly as in
  section 4 (same place, above, selected, one step), focus back to the canvas. An unedited Enter
  with the check set makes a copy with the current value **(Proposal V5)**: the maker asked for a
  copy, and Duplicate-in-place is a real result; without the check an unedited Enter still writes
  nothing.
- **Canvas while the check is toggled:** nothing changes (typed values do not preview); the knob keeps
  its dragging look while the chip is open.
- **Invalid value:** the chip stays open with the existing "Enter a number" message; the check keeps
  its state.
- **Accessibility:** the input's accessible name is "Copy" (`aria-label`), the visible text is
  `aria-hidden`, as in `MoveEntryChip.tsx`; the field keeps its name ("Curve", "Arc start", "Arc
  end", "Corner radius, all corners" and so on). Checked state is native `checked`, announced by the
  screen reader; no extra live region. Focus ring: the native focus-visible ring of the Move chip's
  check.

## 6. Limited values (criterion 5)

Nothing new: the copy shows what the change would show.

- Drag: the value is clamped to the shape's range as today; the readout carries the existing limit
  word before " · copy" (section 3).
- Typed value above the limit with the check set: the limited value is written to the copy; the
  corner radius's 1.5 s "max" readout shows as today, at the copy's knob. Curve, arc, ratio and
  Rounded show nothing extra, as today.
- After the commit the copy is selected, so the bar fields show its values and, where the stored
  value lies outside the shape's range, the existing "limited" tag.

## 7. Multi-selection (criterion 6)

There is no case: knobs are drawn only for the sole selected object ("Group handle layout": no
72 px tier, no parameter handles; `press.rs` "the sole selected"), and the entry box opens only
from a knob. A rectangle's linked corners (followers) are one object; the copy is a copy of the
whole object with all four radii as the change would set them. Nothing to design (K2).

## 8. Not present, by rule (criterion 8)

No hint chip line ("Shift: copy" is not added to the corner radius hint chip or anywhere else), no
tooltip line, no panel section, no rail or bar button, no success notice, no new cursor. The check
is the only new element.

## 9. Proposals

- V1: Shift counts only once pressed during the drag; a Shift held from the press keeps its press
  meaning until let go (section 2).
- V2: readout suffix " · copy" after the existing readout, limit word first (section 3).
- V3: the check is labelled "Copy", as in the Move entry chip (section 5).
- V4: Shift at the double-click does not preset the check (section 5).
- V5: an unedited Enter with the check set makes a copy with the current value (section 5).
- V6: the selection box and the other knobs stay on the original during a copy drag (section 3).
- V7: scope includes the Rounded knob of `0145` (a primitive parameter knob added after the spec's
  list); it excludes knobs that only share the look: text Position (`0080`), sketch instance
  (`0099`), tiling (`0069`), generator knobs, Offset corners (K4).

## 10. Spec deltas for the product owner

- K1: criterion 3 "As copy" becomes "Copy" (V3).
- K2: criterion 6 has no case on the canvas (section 7); drop it or say it applies to nothing now.
- K3: criterion 1 "blue/black preview" is superseded by the "Drag ghost" row (customer 2026-10-27);
  the copy drag uses its Ctrl-copy rule (section 3).
- K4: Terms list "number of sides": no such knob exists (Points is a bar field only); drop it, and
  add Rounded (`0145`) if V7 stands.
- K5: criterion 1 "held at the press" becomes "held at the release, pressed at any time during the
  drag" (V1).

## 11. Design-system rows for the implementer (same PR)

- New row **"Parameter handle copy drag"**: "`0073`. Shift pressed during a knob drag (a Shift held
  from the press counts once let go and pressed again) makes a copy: the original stays drawn as committed with its box and other
  knobs, the copy is drawn as itself directly above it at the same place with the dragged knob, no
  ghost; Copy badge; readout + ' · copy'. The pointer-up `shiftKey` decides. The copy is selected."
- Row "Drag ghost": add "Shift knob copy (`0073`): as the Ctrl copy, at the same place."
- Row "Modifier badge": add "and during a knob copy drag (`0073`), never before its press."
- Row "Live transform readout": add "' · copy' after a parameter readout during a knob copy drag."
- Row "Transform entry chip": add "the parameter chips end with the Copy check of the Move entry
  chip, unset on every open; an unedited Enter with Copy set makes a copy at the current value
  (`0073`, exception to 'Numeric entry chip')".
- Keyboard table: a row "Copy with a parameter handle | hold Shift during the knob drag; or the Copy
  check of its entry box | `0073`".

## 12. Review checklist

1. Knob drag, Shift pressed after the press: original unchanged and boxed, copy drawn above, badge,
   "· copy" in the readout, all in the keydown frame with the pointer at rest.
2. Shift released mid-drag: change look (ghost) in the keyup frame; release writes a change.
3. Release with Shift down: one new object at the same position (compare X, Y, W, H in the Move chip:
   identical), directly above in Layers, selected; original unchanged and not selected.
4. Shift held at the press on a knob: the knob drag (one corner on a rectangle), no toggle of the
   selection, no copy until Shift is let go and pressed again.
5. Escape, the other button and window blur during a copy drag: nothing written, badge gone.
6. One undo removes the copy; the original is selected again; History shows one step.
7. Every knob kind: Curve (ellipse, polygon, star), Start, End, inner radius, corner radius (linked,
   one corner with Shift), Rounded; Ctrl snap works together with Shift; a Ctrl snap without
   Shift shows no badge and no "copy".
8. Limits: readout "r 12.0 mm max · copy", "rounded 10.00 max · copy"; the copy has the limited value.
9. Entry chip: Copy check last, 24 px row, unset on every open, Tab loop field and check, Space
    toggles, Enter applies from both, Escape closes; screen reader reads "Copy, check box, not
    checked"; chip flips and clamps correctly with the taller body at the canvas edges.
10. No hint chip line, tooltip line, panel section, notice or new cursor anywhere.
11. 200 % zoom and 1x display: badge and readout stay clear of the knob and of each other.
