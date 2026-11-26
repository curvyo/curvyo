# 0155 Generator joint options: UX notes

Short notes for slices 2 to 4, checked against `specs/0147-rounded-box/ux-notes.md` and
`docs/design-system.md`. Everything here is **(Proposal)**, vetoable at the demo. No new colour token,
no new component, no popup, no hint label, no notice after a success; hidden, not disabled.

## Slice 2: Bend through tabs

- Row **Bend through tabs** in the Slits group of the Rounded box and the Path box, directly under
  **Wall slits**. Two-cell segmented choice (the existing Choice look): **Flat window** | **Curve**
  (ids `window` | `curve`). A new box shows Curve; a box saved before shows what it stores.
- Shown with the Flat top only (the curved top has no tab on a bent span). Not in the Box.
- The row is shown whether or not the footprint has a bent span; with none, both choices give one output.
- Notice (Dependent, status line only) when a radius is too small for a slot between two columns:
  "Bend through tabs: radius N mm is too small for a tab; fewer tabs on this bend." No dialog.

## Slices 3 and 4: the Advanced group

- Group **Advanced** at the end of the Generator block, collapsed by default (state not stored), same
  chevron header as the other groups.
- Rows in order: **Joint pattern** (Even | Centred), **Joint count** (Auto), **Joint size** (Auto; in the
  Box it is the existing Finger width, shown once under this name), **Joint gap** and **Joint spread**
  (only for Centred with Count 2 or more).
- **Auto** values are muted and show the used value (as `0147` section 7); typing overrides, the reset
  returns to Auto.
- **Joint spread** is a slider 0 to 100 % with the value readout, step 1 %, default 0 %; the canvas
  preview follows while dragging.
- Clamp notices name the field and the limit ("Joint size: at most 24 mm on this edge").
