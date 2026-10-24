# ADRs for the inline delete confirm

ux-engineer for the architect, 2026-10-23. **No new ADR, no new crate, no new dependency, no
project `format_version` change, no font library `format` change.** The slice is frontend only:
a new button look, one colour token and one shared armed state in the Fonts tab.

## Depends on

- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §3: the panel is React DOM
  over the wasm engine; the armed state is UI state and never crosses into wasm or the document.
- [`0075` adrs.md](../0075-font-library/adrs.md): the remove and tag-delete logic, the Curvyo
  folder deletion through the host (ADR 0020 §3), the TTC rule and the font library file
  (ADR 0004 §7) are called exactly as today; only the control that triggers them changes.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): unchanged; removing a font or
  deleting a tag still writes no history step.

## Notes

- 2026-10-23: the 400 ms guard and 4 s timeout are UI constants in the frontend, not settings
  (spec "Out of scope"), so they need no persistence decision.
