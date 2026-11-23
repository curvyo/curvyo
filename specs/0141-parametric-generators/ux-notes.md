# 0141 Parametric generators: UX notes

ux-engineer, 2026-11-23. Checked against `specification.md` (criteria 1 to 8, Fragen 277 A and 278 A),
`specs/0099-sketch-pages/specification.md` (revision criteria 23 to 31) and its `ux-notes.md`,
`specs/0071-pattern-fill/ux-notes.md` (the nearest precedent: live parameters in the panel, Convert to
paths), `specs/0057-clone-overrides/specification.md` (override rows), `specs/0065-extensions-sdk/`
(EXT-UI A: declarative schema, no TSX) and `docs/design-system.md` (rows "Tool rail architecture", "Tool
rail budget with Measure", "Rail tooltip and notice placement", "Schema-rendered workbench tab", "Size
field", "Parameter handle", "Parameter handle layout", "Live preview outline", "Drag ghost", "Vector
dimension look", "Action notice", "Canvas notice slot", "Choice strip", "No popups in the properties
panel", "Plain fields, not sliders", "Hidden, not disabled", "Place a shape by click", "Linked clone in
the UI"). Customer rules applied: no popups in the panel, no flyout from the rail, no operating tips or
hint labels (2026-10-16), no notice after a success (2026-10-31), hidden not disabled, plain fields. A
label here names a control or a state. No new colour token. Choices marked **(Proposal)** are vetoable
at the demo.

**Shared with `0099`.** The `0099` revision (criteria 23 to 31) has no UX notes yet. Sections 4 (schema
fields) and 9 (instance handles) are written as the one look of a parametric instance: a sketch clone's
exposed parameter list and its handles use them unchanged; section 9 names the only differences.

## 1. The Generators entry (criterion 1)

- **Where.** A third button appended to the **More tools** card of column A, after Offset and Measure
  (the spec's Proposal). It is a **tool**, like Offset and Measure: it takes the active-tool look
  (`--toolbar-icon-active-bg`) and the one-active-tool rule. Lucide `Blocks` 20px (Proposal), tooltip and
  accessible name "Generators". No letter, no chord (Proposal; the Recognise text precedent).
- **Rail budget conflict.** Column A is full (row "Tool rail budget with Measure": "a further tool is a
  customer decision"). The card grows from 92px to 136px, column A ends at 624px; in the smallest window
  (560px) it scrolls under Tools by the existing rule, 64px instead of 20px. No card moves. This needs
  the customer: decided 2026-11-23 (Frage 327 A, section 14).
- **No flyout.** The rail opens nothing (row "Rail tooltip and notice placement"). Box and Gear are
  chosen in the tool's bar, so a later generator is one more segment, not one more rail button.
- **Generators bar.** In the overlay row, centred like the polygon/star bar: one pill, 36px row, one
  `role="radiogroup"` named "Generator" of text segments with a 16px glyph each (Box: Lucide `Box`; Gear:
  Lucide `Cog`), the bar's own segment look (`--value-fill` pressed), one Tab stop, arrows move and select.
  Order: the registry rule of `0065` criterion 3 (built-ins by name: Box, Gear). The last choice is kept
  for the session (Proposal; first use: Box). Nothing else in the bar: no parameters before placing (they
  are in the panel once the object exists). More than the bar's width wraps to row 2 by the bar overflow
  rule; with two generators it does not arise.

## 2. Placing at the pointer (criterion 1)

- **Ghost.** While the tool is active and the pointer is over the canvas, the chosen generator's outline
  at its defaults, true size at the current zoom, follows the pointer, centred on it, in the "Drag ghost"
  look (1.5px hollow `--drag-ghost` on the white casing, no fill), snapping as a Select move (`0060`).
  Off the canvas it is gone. Under 6px on screen it draws as an 8px ring. Cursor: crosshair.
- **Place.** A click places the object with its box centre at the click point; a press and drag moves the
  ghost and places it at the release. One undo step "Add gear" / "Add box". The object is on top of the
  active layer, selected, and the tool returns to **Select** (Proposal; the Import precedent: what was
  brought in is selected with Select), so its handles show at once.
- **Tab.** Placing shows the Style tab (and the panel body if hidden) even when History, Layers or File is
  open: a stated exception to "History, Layers and File never switch by themselves", because criterion 1
  puts the parameters in front of the maker (Proposal). Focus is not moved by it.
- **Which extension.** `0065` criterion 3 ("the card says which extension added it") applies to
  third-party contributions; built-in generators show nothing extra (no tooltip line), since the rule
  against hint labels holds and every built-in looks native (Proposal; product owner to confirm).
- **Keyboard.** With the tool active and focus on the canvas, **Enter** places at the pointer if it is over
  the canvas, else at the centre of the visible canvas (the "Place a shape by click" rule). Focus stays on
  the canvas; Shift+Ctrl+F then reaches the first parameter field.
- **Escape** leaves the tool for Select and places nothing (the Escape cascade). A release over the panel
  or the rail places nothing.
- **Pages.** On a drawing page or the pasteboard as any object; on a laser page an own item by position,
  Not assigned (`0098` criterion 11). The entry exists where the More tools card is drawn; the sketch rail
  has no More tools card, so a generator is not placed on a sketch page.

## 3. The Generator block in the Properties panel (criteria 1, 2, 3)

- **Tab and place.** The **Style** tab, first block, above the Style sections; the place where `0099`
  criterion 24 puts a clone's Parameters list. Not a dialog, not a new tab. `role="group"` named by the
  kind ("Gear", "Box"). Header subject line "Gear", "Box", "2 gears", "3 boxes"; mixed kinds "N objects"
  (section 10).
- **Layout.** Built only from the generator's declarative schema (no generator TSX): per schema group a
  heading (14px `--toolbar-icon`, `h3`, the Params block rule of "Schema-rendered workbench tab"), then its
  controls, 244px wide, 28px high, 8px apart, 16px between groups. Then 16px and **Convert to paths**
  (section 11), last. A parameter added to a generator later is appended to its group, never inserted
  (the Params block rule).
- **Conditional rows.** A row that does not apply is out of the tree (hidden, not disabled): Keyway
  width and depth only while Keyway is on; spoke rows only while Web is Spokes; Finger width and
  Clearance only while Joint is Fingers. A hidden value is kept and comes back as it was. A row coming or
  going moves only the rows below it.
- **Style sections below.** A gear is one compound path (`0035`): the Style sections as for a compound
  path. A box is one object of its plates (five without a lid): the Style sections write all plates.
- **Height.** Gear with spokes about 560px, box about 520px: the tab scrolls in its one scrollbar, as
  Pattern fill does at 800 x 640.

## 4. Schema element kinds and their components

Every generator field is drawn by the shell from a schema element. One field component for all numbers:
the **Params field** of "Schema-rendered workbench tab", which is the "Size field" look widened to 244px
(white ground, label 12px inside the left edge, value 14px tabular right-aligned, unit inside the right
edge). Not the panel `ValueField`: it needs a per-parameter drag scale the schema would have to carry,
and the direct drag of a distance is the handle on the canvas (section 9). The sketch clone's list of
`0099` criterion 24 uses the same field (its "a value field each" is read as this field).

| Element | Component | Exists? | Used by |
|---|---|---|---|
| Length (mm) | Params field, unit "mm" whatever the display unit (as Stroke width and Pattern fill), up to two decimals trimmed | yes (Params block) | most fields |
| Integer | Params field, no unit, whole numbers; a typed fraction rounds half up without a notice | **new** | Teeth, Spokes |
| Angle (degrees) | Params field, unit "°", one decimal trimmed | **new** | Pressure angle |
| Precision | a length may name its shown decimals (default two); Module shows up to four, trimmed | **new** | Module |
| Derived length | Params field; its value is computed from other parameters; a commit writes the parameter the generator names (section 5) | **new** | Pitch diameter |
| Choice (2 to 8 named) | "Choice strip" (`PresetStrip`), full width, one Tab stop, arrows set | yes (component), **new** as schema element | Joint, Lid, Web |
| Switch | `PanelSwitch` 28px row, label left | yes (component), **new** as schema element | Keyway |
| Shown when | visibility of a row by another parameter's value | **new** | section 3 |
| Handle anchor | the generator's output names, per handled parameter, an anchor point and a drag direction in the object's frame | **new** | section 9 |
| Bound report | the generator reports the effective bound when it clamps, so the notice can name it | **new** | section 6 |

**Field keys (every number).** Up and Down step by the schema's step (Shift x10, Ctrl x0.1; integers step
1, Shift 10, Ctrl does nothing), live, one commit on key-up (the "Size field" rule); a digit, Enter or F2 types, Enter, Tab or
blur commits, Escape restores; decimal point or comma, `inputmode="decimal"`; `role="spinbutton"` with
`aria-valuemin/max/now` and `aria-valuetext` saying the unit in words ("48 millimetres"). Mixed: muted
"Mixed"; a typed value writes all selected objects of that kind.

## 5. Gear and box fields (criteria 2, 3)

Order is the schema order (Proposal); labels are the visible names and the accessible names.

**Gear**

| Group | Row | Element |
|---|---|---|
| Teeth | Module | number, mm |
| | Teeth | integer, 6 to 200 |
| | Pitch diameter | derived length (below) |
| | Pressure angle | angle, default 20° |
| Centre | Bore | length, 0 = none |
| | Keyway | switch; the row is shown only while Bore is above 0 (Proposal) |
| | Keyway width, Keyway depth | length, only while Keyway is on |
| Web | Web | choice strip **Solid / Spokes** (one choice for criterion 2's Solid and Spokes) |
| | Spokes, Rim width, Hub diameter | integer and lengths, only with Spokes; the windows between the spokes are rounded by the generator |

**Box**

| Group | Row | Element |
|---|---|---|
| Size | Inner width, Inner depth, Inner height | length |
| Material | Thickness, Kerf compensation | length; Kerf default 0 |
| Joints | Joint | choice strip **Fingers / None** |
| | Finger width, Clearance | length, only with Fingers |
| Lid | Lid | choice strip **None / Flat / Fingers** |
| Layout | Gap | length (the only layout is the plates side by side, so the spec's Layout is its gap; Proposal) |

**Pitch diameter.** Directly under Teeth, so cause and effect sit together. It looks and behaves like every
other field: no read-only look, no caption, no lock glyph (the no-hint rule; the label says what it is).
Typing or stepping it writes **Module = Pitch diameter / Teeth** with Teeth kept; Module updates in the
same frame. Changing Teeth or Module updates it in that frame. Module is stored as computed, not rounded to
its display, so a typed 50 mm reads back as 50 mm. Step name "Gear: Pitch diameter". With two gears of
different Teeth a typed value gives each gear its own module.

## 6. Clamping without an error (criterion 4)

- **Typing.** A value outside the schema's bounds, or beyond a bound that depends on other parameters
  (a bore larger than the root circle allows, a finger width longer than its edge), is **applied
  clamped**: the field shows the clamped value and the step commits it. Never `aria-invalid`, never the
  `--field-invalid` validation chip (that refuses; this permits), never a dialog.
- **Notice.** The "Action notice" **status** look (`--toolbar-bg`, `--toolbar-icon` text, 12px, 8px padding
  and radius, `role="status"`, 3s or until the next action), 4px under the field, right-aligned, max 244px,
  an overlay that moves no row, never takes focus. Texts: "Teeth set to 6, the minimum." / "Teeth set to
  200, the maximum." / "Thickness set to 0.1 mm, the minimum." / for a dependent bound "Bore set to 38.4
  mm, the most this gear allows." ("this box allows"). This is a limit, so the no-success-notice rule does
  not remove it.
- **Knock-on.** If a change makes another stored value exceed its dependent bound (fewer teeth shrink the
  root circle under the bore), that value is clamped and stored in the same step, and the notice under the
  edited field names it: "Bore set to 20 mm, the most this gear allows." (Proposal: the numbers stay true.)
- **Handles and chips.** A handle drag stops at the bound and its readout says "max" / "min" (the corner
  radius readout rule); no notice. A handle's entry chip commits clamped and posts the same sentence to the
  canvas notice slot.
- **Rounding** an integer is not a clamp: no notice.

## 7. Live regeneration and undo (criterion 4)

- Field stepping and typing redraw the object in place (as Pattern fill does). A handle drag or a resize
  draws the regenerated result as the "Live preview outline" over the committed object (blue new, black
  old). In both cases the latest finished regeneration is drawn; the previous one stays until the next is
  complete: never blank, never half a gear.
- One step per commit. Names (Proposal): "Add gear", "Add box"; "Gear: Teeth", "Box: Thickness" (the
  field label; a handle drag carries its parameter's name); "Scale" as built; "Convert gear to paths",
  "Convert box to paths", "Convert to paths" for a mixed selection. Escape during a drag or while typing
  restores and writes nothing.

## 8. As an object on the canvas (criterion 5)

- **Selection and handles.** Box, move, rotate and flip as for any object. **Resize is proportional only**
  (Proposal): corner resize handles only, as if Ctrl were held; no edge and no skew handles; `K` and the
  skew chip skew the other selected objects and leave generated ones as they are (with only generated
  objects selected, `K` does nothing; Proposal). Reason: a gear cannot be stretched, and a
  box stretched by a factor would change Thickness, which is a material. A resize writes parameters: gear
  Module, Bore, Keyway, Rim width and Hub diameter by the factor, Teeth, Pressure angle and Spokes kept;
  box Inner width, depth and height by the factor, Thickness, Kerf, Finger width, Clearance and Gap kept.
  The size readout and the `S` chip show the outer size as for any object.
- **A box is not entered.** Its plates are one group in the document (criterion 3), but in the UI the box
  is one object (Proposal): a double-click does not enter it and nothing happens, and the Layers list shows
  one row without children, because editing inside a live object would be lost on the next regeneration.
  Convert to paths gives the enterable group.
- **Size chip.** `S` (or a double-click on a corner handle) opens the size chip with W and H; typing one
  sets the other in proportion in the same frame (the Ctrl rule of a resize), then the factor is applied as
  above. Proposal.
- **Layers.** Kind glyph `Cog` (gear) or `Box` (box), default names "Gear" and "Box". Status bar left
  "Gear · <page>".
- **Node tool.** The Node bar slot holds the text-only pill of the clone precedent: "Generated objects have
  no nodes. Change their parameters or convert them to paths." (`role="status"`). No node overlay.
- **Commands.** Boolean, Path and Offset commands take a generated object as they take a primitive (its
  paths); the result is ordinary paths. Laser operations (`0101`) assign to it as to any item. Linked
  clone of a generated object: as for any object (follows its parameters).

## 9. Instance handles for distance parameters (shared with `0099` criteria 25 to 31)

- **Glyph and hit.** The existing "Parameter handle" (10px knob, 12px hit radius, `pointer` cursor),
  drawn by the Select tool with the transform handles from a shorter box side of 72px on screen. No new
  glyph: "a new parameter kind adds a knob position and a readout".
- **Which.** A schema length marked as handled, at the anchor and direction the generator outputs (a
  sketch clone: the dimension's anchor and direction, `0099` criterion 25). Integers (Teeth, Spokes) and
  Pressure angle have no handle (a rotation arc would mean nothing there); Thickness, Kerf, Finger width,
  Clearance, Gap and the keyway have none either (too small to aim at; fields only). Proposal:

| Object | Handle | Position (object frame) | Drag |
|---|---|---|---|
| Gear | Pitch diameter | on the pitch circle at 45° | radial; Teeth kept, Module follows |
| Gear | Bore | on the bore circle at 0°; absent while Bore is 0 | radial |
| Gear | Hub diameter | on the hub circle at 180°, Spokes only | radial |
| Gear | Rim width | on the rim's inner circle at 135°, Spokes only | radial, inward grows the rim |
| Box | Inner width, Inner depth, Inner height | the midpoints of the base plate's right and top edge and of a side plate's outer edge, as the layout puts them | along the edge's normal |

- **Collisions.** In the order of the table; a knob within 16px of an earlier knob or of a transform
  handle is not drawn (the value stays in its field). Zooming in brings it back.
- **Hover and drag.** While hovered or dragged, the measured distance is drawn in the "Vector dimension
  look" without its text (extension lines and arrowheads in `--dim-guide`; for a diameter, the line
  through the centre): "the dimension's own arrow" of `0099` criterion 25. During the drag the readout chip
  at the pointer names it: "Pitch diameter 48.0 mm", "Bore 6.0 mm", at a bound "... max". Shift snaps the
  value to whole millimetres (`0099`'s Proposal). The regenerated result is the blue preview (section 7).
  One step per drag; Escape restores; a double-click on the knob opens the numeric entry chip with one field
  named by the label.
- **Angle parameters.** No generator here has a handled angle. A sketch clone's exposed angle (`0099`
  criterion 25) gets the same knob on its arc, placed and dragged by the "Arc handle layout" rule, with the
  readout "<name> 30.0°" and Shift snapping to whole degrees (Ctrl does nothing here); everything else as above.
- **Differences for a sketch clone.** The drag writes the clone's **own** value and makes its Parameters
  row Own (`0099` criterion 24); a clone without a solution keeps its last geometry in the warning style
  (`0099` criterion 26). A generated object has no original, so it has no Original's / Own row: its
  values are simply its own.

## 10. Several selected

- Two or more objects of one generator kind: one block, Mixed where values differ, a typed value writes
  all; choice strips with no cell pressed when mixed. Handles only for a single selected object.
- Different kinds, or generated objects with other objects: no Generator block; the Style sections as for
  a mixed selection; Convert is the Select bar's Object to path (section 11).

## 11. Convert to paths (criterion 6)

- **Panel.** A full-width (244 x 28px) outline text button **Convert to paths**, last in the block, the
  look of 0071's and the sketch page's. No glyph, no tooltip beyond its name, no confirm, no notice.
- **Select bar.** The existing **Object to path** converts generated objects too, so a mixed selection
  converts in one press (Proposal; it is the bar's command for "live object to plain paths").
- **Result.** One step. A gear becomes one compound path, a box a group of its plates as paths, **selected in the same
  commit** (0071 review finding 2), so box, header and Style tab agree. Focus goes to the canvas (the
  button has left the tree). Undo gives the generated object back, selected.

## 12. A file with an unknown generator kind (criterion 7)

- The file opens; each such object shows its stored outline (ADR 0080 §5; drawn, cut, moved, rotated,
  copied, deleted, converted; no resize or number change). Once after the open, the canvas notice slot
  shows a status (5s): "2 objects come from a generator this version does not have. They are shown as
  plain outlines." (one: "1 object comes from ..."). No dialog, no banner, no per-object mark.
- Opening does not mark the file changed (the precedent of the converted sketch page). Whether a save keeps
  the unknown record is decided (ADR 0080 §5): a save keeps it unchanged. A resize of such an object is
  refused with a status notice (wording: "This object comes from a generator this version does not have;
  it cannot be resized.").

## 13. Accessibility and keyboard

- Tab order is screen order: block fields group by group, choice strips one stop each, the Keyway switch,
  Convert to paths, then the Style sections. Every control has a visible label; the bar segments carry
  glyph and word.
- Canvas handles are not focusable; their keyboard route is the field of the same name.
- Keys added: Enter on the canvas with the Generators tool (place). No new letter or chord.
- Contrast: tokens only (`--toolbar-icon` 8.3:1, `--panel-muted-fg` 5.0:1, `--dim-guide` 5.8:1 on the
  canvas). Nothing animates.

## 14. Questions and flags (resolved 2026-11-23)

1. **GEN-RAIL (Frage 327): decided.** The Generators tool sits in the More tools card with Offset, Measure
   and Grid, simply there for now (option A of the question; the column ends 624px and scrolls 64px in the
   smallest window as drawn in section 1). The rails must later be reworked to scroll so they fit the screen
   size; on a small screen all rails could be one column, below each other (`docs/technical-debt.md`).
2. **Spec readings, taken into `specification.md`:** Web is one choice (Solid / Spokes; Spokes 2 to 12);
   window rounding is the generator's; Layout is a Gap field; resize is proportional only, no skew; the box
   is one object in the UI and not enterable, Convert to paths gives the group of plates; criterion 1 is met
   by the bar's segments and placing switches to the Style tab (section 2); `0065` criterion 3 is read as
   third-party only (section 2).
3. **Architect (`adrs.md`): answered** in ADR 0080 §2 and §5 (schema elements; an unknown record survives a
   save).
4. **For `0099`:** its revision UX notes can point to sections 4 and 9 instead of a second design. This
   reads criterion 24's "a value field each" as the Params field, not the panel `ValueField` (no drag in
   the field; the drag is the canvas handle). The product owner of `0099` confirms or vetoes (still open).
5. **Spec lines outside UX notes:** Status and "Still open before Ready" are updated (Ready).
6. **To go into `docs/design-system.md` when Done:** the Generators tool and bar, the Generator block, the
   schema element table, the clamp notice texts, the instance handle row.
