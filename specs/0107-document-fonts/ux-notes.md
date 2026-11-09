# UX notes: Document Fonts (0107)

- **Placement:** group "Document Fonts" first in the tree, header like "Curvyo fonts" with the family count, pinned like the others; rows are the same family rows (name set in its own face, star, tag line, faces on open).
- **Row states:** installed: star and + active, nothing at the trash position. Not installed: star and + greyed (`disabled`, `aria-disabled`, no focus stop), the **install icon of the top-row Add font** at the trash position. Never a trash can.
- **Install:** one press, no armed state (it is not destructive); the row turns into the installed state. Tooltip "Install font"; failures use Add font's notice.
- **No hint texts**, no banner, no "embedded" badge. The tag line of a greyed row stays visible but inert.
- Keyboard: the install icon is reachable as the trash can is (the row's action zone); greyed controls are skipped.
- Update `docs/design-system.md` (Fonts tab groups) with the row states.
