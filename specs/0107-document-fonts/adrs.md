# ADRs for document fonts

Lead acting as architect, 2026-11-09. One ADR: [0047](../../docs/adr/0047-document-fonts.md) (Accepted). No new crate, no new dependency, no `format_version` change, no relay protocol change. Questions FONT-LICENCE, FONT-SAME-NAME and FONT-INSTALL-MEANING have defaults (A) in the spec.

Per criterion: 1-5 ADR decisions 1-3; 6-9 decision 4; 10-15 decision 5; 16-18 decision 6; 21 decision 7.

Touched crates: `curvyo-document-core` (font hashes of the tree, prune), `curvyo-ui-core` (`FontBlock::Document`, match order), `curvyo-editor-wasm` (embed step, Document block, tab JSON, install bytes), `curvyo-text-core` (fs_type check only), frontend (Fonts tab rows).
