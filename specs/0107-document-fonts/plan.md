# Plan: document fonts (0107)

Built by the lead in the implementer role (customer: no subagents, Sonnet only, for this feature).

- [x] 1. `curvyo-document-core`: `Document::used_fonts`, `used_font_hashes`, wipe keeps used font hashes (criteria 1-3, 5). Tests `tests/font_assets.rs`.
- [x] 2. `curvyo-ui-core`: `FontBlock::Document`, embedded face beats a same-named installed face with other bytes (criterion 8).
- [x] 3. `curvyo-text-core`: `FaceStore::add_shared`; `curvyo-editor-wasm`: font files kept shared, `embed_used_fonts` before pack and room snapshot and sync push, `refresh_document_fonts` on open and on arrival, `installed` in the tab JSON, `document_font_file` (criteria 1, 6, 7, 9, 14, 16-18, 21). Tests `session/document_fonts_tests.rs`, golden `tests/fixtures/document_font_foreign.curvyo`.
- [x] 4. Frontend: block "document" first in the Fonts tab, greyed row with Install (criteria 10-15); `fontFileName`.
- [x] 5. Journey `frontend/tests/browser/document-fonts.cjs`; docs: design system, technical debt, ADR 0047.
