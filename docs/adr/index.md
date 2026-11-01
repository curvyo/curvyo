# Architecture Decision Records

One global, flat numbering sequence for the whole workspace, regardless of
which crate a decision is about (see
[`../guides/rust-workspace-blueprint.md`](../guides/rust-workspace-blueprint.md)
§2). Workspace-wide ADRs live here; a crate-scoped ADR lives in
`<crate>/docs/adr/` and is listed in this table with its scope and a link.

**Once an ADR is accepted, its decision text is not edited.** Changed
circumstances get a new ADR that supersedes the old one, and the old one's
Status line is updated to point at it. ADRs 0001–0011 below are accepted, so
this rule is now live: every further change to any of these decisions is a new
ADR. ADR 0012 was rejected before acceptance, so that rule never applied to it.
ADRs 0013, 0014 and 0015 are accepted as well (0014 by the customer on 2026-10-10, 0015 on
2026-10-11).
(0001–0010 were accepted by customer sign-off; 0011 is downstream assembly of
them and was accepted by the architect — see its own preamble for why
`CLAUDE.md` §3 does not require a customer round for it.)

**Name map (ADR 0013, 2026-10-07):** the product is called Curvyo. Read
`vecmanf-*` as `curvyo-*` (crates and directories) and `.vmf` as `.curvyo`
(file extension) in ADRs 0001–0012, whose text keeps the old names.

A decision too small for a full ADR gets a short dated note in that feature's
`specs/<NNNN-feature-slug>/adrs.md` instead.

| ADR | Title | Scope | Status |
|---|---|---|---|
| [0001](0001-ui-framework-and-canvas-rendering.md) | UI framework and canvas rendering | workspace | Accepted |
| [0002](0002-document-model-units-and-svg-round-trip.md) | Internal document model, units and SVG round-trip | workspace | Accepted |
| [0003](0003-geometry-kernel-booleans-offsetting-vcarving.md) | Geometry kernel, boolean operations, offsetting and V-carving | workspace | Accepted; §3 amendment 2026-10-09 in effect since PR #69 merged (`i_overlay` `=9.0.1` replaces `clipper2-rust`); §4 amendment 2026-10-10 (offsetting on `i_overlay`'s integer offsetter, customer decision) |
| [0004](0004-persistence-and-cross-machine-sync.md) | Persistence, collaboration and cross-machine sync | workspace | Accepted |
| [0005](0005-extension-and-plugin-model.md) | Extension and plugin model | workspace | Accepted |
| [0006](0006-license.md) | License | workspace | Accepted |
| [0007](0007-asset-library-connector-and-credential-storage.md) | External asset-library connector and credential storage | workspace | Accepted |
| [0008](0008-end-to-end-encryption-of-sync-and-collaboration.md) | End-to-end encryption of sync and collaboration | workspace | Accepted |
| [0009](0009-concurrent-editing-semantics.md) | Concurrent editing semantics — undo, ephemeral state and merge granularity | workspace | Accepted |
| [0010](0010-document-keyring-admins-and-revocation.md) | Document keyring — participants, admins and key revocation | workspace | Accepted |
| [0011](0011-workspace-and-crate-layout.md) | Workspace and crate layout | workspace | Accepted; §7 note 2026-10-10 (host gate on ubuntu for pull requests, Windows and macOS nightly only, customer decision 2026-10-09) |
| [0012](0012-pages-in-the-document-model.md) | Pages in the document model | workspace | Rejected (2026-10-05, dropped from MVP) |
| [0013](0013-rename-to-curvyo.md) | Rename the product to Curvyo (amends 0011 crate names, 0004 §1 file extension; [inventory](0013-rename-to-curvyo-inventory.md)) | workspace | Accepted |
| [0014](0014-history-undo-and-branches.md) | History, undo and branches over the operation log (own restore engine on `diff` instead of Loro's `UndoManager`, step header with time in commit messages, deleted objects revived by an injected tree move under a pinned Loro, previews from the live document, wipe by shallow snapshot with an epoch, budgets by step size; amended 2026-10-10 after the Loro spike; supersedes one sentence of 0004 §2) | workspace | Accepted (customer, 2026-10-10; option A on all seven questions) |
| [0015](0015-node-references-and-ancestor-resolved-state.md) | Node references and ancestor-resolved node state (a linked clone, also in a chain, reads its root original's outline, style and opacity through one atomic `clone_link`; `visible`, `locked`, `opacity` resolve through ancestors; output of hidden nodes left open; supersedes one sentence of 0002 §5; asked by specs 0049 and 0050) | workspace | Accepted (customer, 2026-10-11) |
| [0016](0016-clone-overrides.md) | Clone overrides (per-clone registers `own_placement` and `own_style` (the ordinary style keys plus `opacity`); a clone reads style and opacity from the nearest clone with own style on its chain; merge per register; extends 0015 §3 to §5; asked by spec 0057) | workspace | Accepted 2026-10-14 (customer) |
| [0017](0017-image-nodes-and-embedded-assets.md) | Image nodes, embedded assets and bitmap decoding (an image is a leaf with its own affine map; bytes in `assets/` of the container, content-addressed, only the hash in the document; one new crate `curvyo-image-core` on the `image` crate, pinned; render-core stays pixel-free; formats PNG, JPEG, WebP; asked by spec 0058) | workspace | Accepted 2026-10-16 by the customer for the `image` crate and the node/asset design, `needs-customer` (document model, file format, dependencies); spike done 2026-10-26: numbers hold, `image` =0.25.10, TIFF out, three corrections recorded |
| [0018](0018-svg-import-crate-parser-and-limits.md) | SVG import and export crate, parser and limits (new crate `curvyo-svg-core`, a pure SVG reader with no Curvyo dependency; mapping to the document in `ui-core`; the hostile-XML guard and path data parser move there from `text-core`, which depends on it; new pinned dependencies `svgtypes` and `simplecss`; no passthrough bag and no loss report (amends 0002 §10, customer 2026-10-16); supersedes "SVG import/export" in 0011's `document-core` row; asked by spec 0024) | workspace | Accepted 2026-10-26 (architect: crate and dependencies; customer veto possible in the 0024 PR) |
| [0019](0019-document-template-and-guide-paths.md) | Document template and guide paths (node register `guide`, page stays a rectangle, templates load as guides) | workspace | Accepted 2026-10-17 (`needs-customer`, design given by the customer) |
| [0020](0020-text-nodes-text-engine-and-font-sources.md) | Text nodes, the text engine and font sources (a text is a leaf with string, font reference, cap-height size and its own affine map; new crate `curvyo-text-core` on `harfrust` + `skrifa`, pinned; `fontdb` in `curvyo-storage-io`; licence record for shipped fonts; hidden-textarea input; asked by spec 0074) | workspace | Accepted 2026-10-20 by the customer (`needs-customer`: document model, file format, new crate and dependencies); wasm budget for the text engine raised to 1.3 MiB uncompressed; amended 2026-10-20 for single-line fonts (`0076`: `text_font.kind`, SVG font reader, Hershey notice, budget 2 MiB) |
| [0021](0021-per-character-values-on-a-text.md) | Per-character values on a text (kern along and across as Loro text marks on `text_string`, `ExpandType::None`, one mark per character, absent = 0, factor of Size; merge per character; copy of an object keeps them, plain-text paste does not; restore engine and `copy_map` learn marks; next `format_version`; asked by spec 0079 slice 4) | workspace | Proposed — needs-customer; default A per Frage 42 KERN-MODEL, asked 2026-10-24, unanswered |
| [0022](0022-texts-on-carriers-and-text-clones.md) | Texts on carriers and text clones (a text references its carrier through one atomic `text_carrier` plus flat on-path registers; kept copy of the outline when the carrier is removed; the carrier becomes a guide, a linked clone may carry its own `guide` (extends 0019 §2); a text clone is a text node with `text_source`, reusing `own_style`; extends 0015 §1 from two to four kinds of reference; format 22 and 23; asked by spec 0080) | workspace | Accepted (customer, Frage 43 A, 2026-10-24) |
| [0023](0023-raster-trace-crate-pipeline-and-worker.md) | Raster trace crate, pipeline and background job (fills the `curvyo-vectorize-core` slot of 0011; one pure `trace(raster, pixel_map, params, poll)` with a parameter schema as data, no trait; our luma, mask, `u32` labelling and mm² specks around `visioncortex` (MIT OR Apache-2.0) per region, `Spline` to `kurbo` in mm; run-to-run deterministic, native and wasm differ by one `acos` until fixed upstream; Web Worker in both builds, cancel by terminate; own pipeline only as fallback after step 0; no format change; asked by spec 0025) | workspace | Accepted (customer, 2026-10-28, Frage 62 option A: keep visioncortex, loosen the quality targets; rests on Frage 49 B, 50 A, 55 A, 60 B) |
| [0024](0024-spatial-pages.md) | Spatial pages in one document plane (shared object tree, per-field page registers, format 26; no old-file compatibility required; spec 0059) | workspace | Spatial model accepted by customer 2026-10-17; old-file compatibility waived by customer 2026-10-29; implementation choices Proposal |
| [0025](0025-sketch-core-and-solver.md) | Sketch core and solver boundary | workspace | Proposed; ACS 0.1.19 provisional adapter reviewed, end-to-end acceptance pending |
| [0026](0026-parametric-sketch-nodes-and-workbench.md) | Parametric sketch nodes and workbench state | workspace | Proposed implementation record; authorized graph/persistence, needs-customer only for original-sketch scaling |
| [0027](0027-linked-dimension-objects.md) | Linked dimension objects (leaf kind `dimension`, a guide by kind; point and curve slots referencing node + `AnchorId` or shape feature, or a free point; value derived on read, last number kept by the `commit_step` funnel; references never cleared, undo relinks; convert to paths + text; never output; extends 0015 §1 to five references and 0019 §2; next `format_version`; asked by spec 0070) | workspace | Accepted 2026-11-01 (customer: DIM-STORE A) |
| [0027](0027-ocr-text-in-trace.md) | Offline text recognition in Trace (new crate `curvyo-ocr-core`: weights as bytes and an RGBA raster in, text lines in px out; PaddleOCR mobile ONNX weights, Apache-2.0, on `rten` (MIT OR Apache-2.0) with our own pre/post-processing; scoped `rayon` exception to "no threads in `*-core`"; weights in `frontend/public/models/` with a licence record and SHA-256, fetched same-origin in both builds; runs in the trace worker; amends 0011 §2/§6, 0023 §5, `0088` adrs decision 1 by notes; asked by spec 0091) | workspace | Proposed; its needs-customer parts answered by the customer 2026-10-31 (Frage 109 B, 110 A, 111 default A, 112 A) |

## Customer sign-off, 2026-10-02

The customer worked through every batched question across five rounds. The
decisions each round produced are recorded in the ADRs themselves — each
affected ADR states in its own preamble what the customer directed and what the
architect chose within that direction — so they are not duplicated here. In
summary, the five rounds settled: Tauri over Dioxus (0001); CRDT plus relay with
Loro, and cloud sync as a required path (0004); plugins as day-one scope with an
unstable-then-frozen interface (0005); AGPL-3.0-or-later with no plugin linking
exception (0006); the asset connector as read **and** write, with OS-keychain
credential storage, a git forge as the first push target and the git wire
protocol rather than a forge API (0007); an end-to-end-encrypted, open-source,
self-hostable relay (0008); and participant revocation as a requirement,
implemented as a signed, git-like keyring log with multiple admins (0010).
ADR 0002 and the ADRs split out of it (0009) and out of 0008 (0010) were
reconciled internally rather than by customer direction.

The last two open sub-questions were put with a recommendation and a stated
default, and both defaults were accepted: a persistent per-room device public
key visible to the relay operator is acceptable (no accounts, email or
registration implied), and a participant is a **device** with a user-chosen
label rather than a person. Both are recorded in
[ADR 0010](0010-document-keyring-admins-and-revocation.md) under its answered
questions.

Two decisions were accepted on their stated defaults rather than by an explicit
instruction, and are called out here so nobody mistakes silence for
unawareness. Reversing either needs a new ADR, not an edit:

- **Peer-scoped Ctrl+Z** ([ADR 0009](0009-concurrent-editing-semantics.md) §1).
  Undo reverts the local peer's own commits only, as every CRDT editor does;
  reverting an arbitrary past change is an explicit history-view action in a
  later story (0009's option C). With nobody else connected this is
  indistinguishable from classic linear undo, which is the customer's normal
  case.
- **E2EE in the first collaboration story** rather than after it
  ([ADR 0008](0008-end-to-end-encryption-of-sync-and-collaboration.md), open
  question). Deferring duplicates the join-path work rather than saving it.

## Open follow-ups

- **The workspace itself is now written but not created.**
  [ADR 0011](0011-workspace-and-crate-layout.md) fixes the twelve-crate layout,
  the full dependency direction, the wasm32 checks and the frontend's place in
  the build; the root `Cargo.toml` and the crate directories it describes are
  the first commit of product work, not part of the ADR. Three crate slots are
  deliberately left empty there (a machine/toolpath core crate and a device
  `-io` crate, awaiting the first machine-family ADR; `curvyo-model-core`;
  a plugin-host core crate), each with its trigger named in 0011 §8.
  One crate name the index's earlier list had missed is in it:
  `curvyo-vectorize-core`, named by ADR 0003 §6.
- ~~`CLAUDE.md` §8's crate-suffix list needs a lead amendment~~ — done: §8 now
  names `-wasm` (0001 §3), `-server` (0004 §5) and the suffix-less plugin SDK
  (0005 §1) alongside `-core`/`-app`/`-io`, pointing at ADR 0011.
- ~~**Dangling cross-references in ADR 0004**~~ — done 2026-10-10 with the
  ADR 0014 pointers: the citations of "ADR 0002 §12" and "§12–§16" carry a
  reader's note pointing at ADR 0009.
- ~~**Boolean-crate spike (`spike/booleans`).**~~ — done, 2026-10-04: all three
  candidates (`i_overlay`, `geo`'s boolean ops, `clipper2-rust`) handled every
  degenerate fixture and build for `wasm32-unknown-unknown`; the spike picked
  `clipper2-rust` on a tie-breaker. **Amended 2026-10-09 (in effect since
  PR #69 merged the same day):** the kernel's property tests found wrong areas from
  `clipper2-rust` on lattice input and none from `i_overlay`, so the library is
  `i_overlay` `=9.0.1`. Recorded in ADR 0003 §3.
- ~~**Offsetting (ADR 0003 §4).**~~ Done 2026-10-10. `i_overlay` `=9.0.1`
  has integer outline and stroke offsetting with the joins §4 lists. The
  customer chose it for `specs/0038-path-offset`, and the amendment to §4
  records it. No new dependency.
- **ADR 0007 is over the five-minute rule and wants splitting, not trimming.**
  It carries three subjects — credential storage, the source/sink trait design,
  and the git-forge sink — and a consolidation pass took out the prose without
  getting it under the limit. The clean fix is a new ADR for the git-forge sink
  superseding 0007 §§14–16; it is a readability follow-up, not a correction, so
  it waits until a story touches that area.
- **Plugin interface freeze (`1.0`).** ADR 0005 §3 ships the interface as
  `0.x` and names the freeze trigger; the freeze itself is a follow-up ADR.
- **Plugin capability grants.** Network and filesystem access for plugins are
  denied by ADR 0005 §7. Lifting that needs its own ADR plus a consent surface,
  triggered by a story.
- **Reverting a collaborator's change.** ADR 0009 §1's option C — an explicit
  "revert this change" action in a read-only history view, able to target any
  past commit — is a story, not shipped behaviour, and is never bound to Ctrl+Z.
  Specified since 2026-10-10 as object undo and go to version in
  `specs/0041-object-history` and `0042-history-branches` (ADR 0014 Q6 A).
- **Forward secrecy.** The one gap ADR 0008 names that ADR 0010 does not close:
  participants keep every epoch key they held, deliberately, so a disclosed key
  still opens that epoch's stored log. MLS (ADR 0008 option C) remains the
  destination if group management ever becomes a real product area, and nothing
  in ADR 0010 points away from it. A follow-up ADR, triggered by a story.
- **Keyring fork resolution needs a review before it ships.** ADR 0010 §5's
  three merge rules are where a bug re-admits a removed participant. They are
  pure functions over an entry set, so the review and exhaustive small-case
  tests are the mitigation, and they share the security-review slot with
  `curvyo-crypto-core`.
- **Push flow details for the git-forge sink.** The target is settled (ADR 0007
  §14, git wire protocol). Two sub-questions remain with working defaults:
  commit-and-push versus a real pull-request flow (default: commit and push, a
  PR needs a forge-specific API and its own story), and HTTPS token versus SSH
  key (default: token, which the credential design already covers).
- **Fonts have no identified asset provider** (ADR 0007 open question 3). A
  product question, not an architecture one: a font provider, if one appears, is
  one more `AssetSource` and needs no new decision. Until then the font
  collection is the local directory of ADR 0004 §12.
- **Is the customer going to *be* an asset provider?** (ADR 0007 open question
  2.) Default assumption: no — we build the client only. Becoming a provider
  means accounts and payments, which is a materially larger scope and its own
  ADR.
- **Machine families.** Each family (laser, cutting plotter, embroidery, CNC)
  gets its own ADR when its first story is ready. No shared machine
  abstraction is designed before the second family exists (`CLAUDE.md` §6).
- **`CLAUDE.md` §7 and §8 need amending by the lead.** ADR 0001 adds a
  TypeScript frontend and an npm dependency tree; the quality gate and the
  repository-layout section currently describe a Rust-only workspace.
