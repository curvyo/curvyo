# 0085 Stroke to path: UX notes

Customer answers of 2026-10-26: Frage 45 A (definition), 46 A (two objects, both selected after), 47 A (outline filled in the stroke colour). No dialog, no chip, no hint line, no options (criterion 2).

## Button

- A text button **Stroke to path**, the look of "Object to path" and "Linked clone": 28px high, 1px `--toolbar-icon` at 60% outline, `rounded-[5px]`, 14px label, 12px side padding, hover `--editor-accent-hover`, focus-visible 2px `--editor-accent` ring with 1px `--toolbar-bg` offset. **No glyph**: both neighbours are text, and a glyph would be the only icon in the group.
- Order: Object to path, Stroke to path, Linked clone. Object to path and Stroke to path form one group ("Convert", `role="group"`, 4px apart, no visible label), so they wrap together; Linked clone stays a group of its own (row "Linked clone in the UI").
- Wrap (rules of `0084`: whole groups, every wrapped line starts at the 8px padding, no divider at the start of a row, first row never moves). Estimate at 800 x 600 (bar 324px): Convert about 236px fits one row; Linked clone (about 104px plus the 25px gap) no longer fits beside it and takes a row of its own, so every selection that showed Object to path and Linked clone on one row grows by 36px (a rectangle 5 to 6 rows, a path 2 to 3). The implementer measures, the tester records the row counts, and the row "Select bar layout" gets the numbers. At 1280 x 800 all three fit one row.

## States

- Not in the tree with nothing selected (criterion 1). With a selection it is always in the tree.
- Enabled when the selection holds at least one subject (criterion 1). Otherwise `aria-disabled` at 40%, keeps its Tab stop and tooltip, a press does nothing. This is a fifth recorded exception of "Hidden, not disabled" (same reasoning as Guide and Flip: a button that appears and vanishes under the pointer is worse than a dimmed one; the neighbours do not move). Add it to that row at build.
- Tooltip (Radix, 400 ms, `side="bottom"`, one line, a state label, not a hint): enabled: "Stroke to path"; dimmed: "Stroke to path: nothing selected has a stroke." Only guides, objects with Stroke off, width 0.002 mm or less, or alpha 0 count as no stroke.

## Command and notice

- One press, one history step "Stroke to path". Focus returns to the canvas after a mouse press, stays on the button after Space or Enter.
- Selection after (criterion 20, Frage 46 A): the new outlines and the fill parts are selected, the tool stays Select, and the Select bar and Properties panel follow the new selection at once. Nothing else is selected or entered.
- Notice (row "Action notice", anchored to the button, at most 140 characters, first sentence at most 70; texts are criterion 22 and are not reworded here): "Stroke to path: 3 strokes became filled outlines." / "1 stroke became a filled outline.", then in this order "2 objects have no stroke and were left as they are.", "1 text was converted to paths.", "2 clones were unlinked." Guides are not named in the notice (they are skipped silently, as they are never subjects); they only count when the whole selection is guides, and then the button is dimmed, so no notice appears.
- Refusal (criterion 26): the sentence of `0016` criterion 17a, offenders outlined in red, same notice surface.

## Keyboard

- **Ctrl+Alt+C**, Inkscape's key. Checked against `docs/design-system.md` on 2026-10-26: no row uses Ctrl+Alt+C or any Ctrl+Alt combination; Shift+Ctrl+C (inspector, browser build) and Ctrl+C (copy) differ. Add the row "Stroke to path | Ctrl+Alt+C | the Select bar button" to the shortcut table at build.
- OS and layout risks: on Windows, Ctrl+Alt is AltGr, so the key must not fire when `getModifierState("AltGraph")` is true or while a text field or text on canvas is being edited (it would eat typed characters); some Linux desktops bind Ctrl+Alt+letter globally, in which case the window never sees it and the button remains the way. Runs only with the canvas focused and the same enabled rule as the button; with a dimmed button the key does nothing and shows no notice.
- The button is in the Tab order after Object to path.

## Review checklist

Button order and wrap numbers at 800 x 600; dimmed look, tooltip and Tab stop; no motion; contrast of the 40% state is exempt as for Guide; selection after press; Ctrl+Z restores the selection as before (criterion 21); light and dark theme; large document (200 stroked objects): button responds, notice appears once.
