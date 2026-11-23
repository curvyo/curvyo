# Plan for Parametric generators

Implementer, 2026-11-23, branch `story/extensions-sdk` (from `main` + the 0141 docs commits). Built under
`adrs.md`, ADR 0080 (Accepted), ADR 0034 and `ux-notes.md`. Own choices are marked **(Proposal)**.

## Affected crates/modules

- **`curvyo-extension-core`:** `extension.rs` (new: the `Extension` trait, `GenerateRequest`, `Edit`,
  `Generated`, `Part`, `ClosedPath`, `Cubic`, `Handle`, `Clamp`, `Seed`), `param.rs` (kinds `Count`, `Angle`,
  `Switch`, `Choice`, `step`/`decimals` on lengths, `shown_when`), `manifest.rs` (`Contribution::Generator`),
  `CHANGELOG.md`.
- **`curvyo-generators-core`** (new crate, ADR 0080 §3; `libm` only besides the workspace edges): `manifest.rs`,
  `gear/` (involute flank, spokes and windows, bore and keyway), `boxes/` (finger edges, kerf, clearance,
  layout), `lib.rs` (`GENERATORS`, `impl Extension`). In CI's wasm32 list.
- **`curvyo-laser-core`:** `impl Extension for Laser` (`manifest`, `notice`); the free functions stay.
- **`curvyo-document-core`:** `generated_model.rs` / `generated_codec.rs` (the leaf: `generated`,
  `generated_frame`, read as a compound path, validation, writers), `shape_codec.rs` (`LeafKind::Generated`),
  `object_read.rs` and `clone_*` (the leaf reads as a path: no outline consumer changes),
  `transform_objects.rs` (a move or rotate writes only `generated_frame`), `objects.rs` (translate on paste),
  `generated_commands.rs` (`add_generated`, `change_parameter`, `convert_generated`), `document.rs`
  (format 54), history labels.
- **`curvyo-ui-core`:** `extensions.rs` (`[&LASER, &GENERATORS]`, trait dispatch), `instance.rs` (the shared
  instance mechanism for the generated source: schema view, edit, memo, handles, scale, convert plan),
  `history_names.rs`.
- **`curvyo-editor-wasm`:** `session/generators.rs` (tool, placing, panel read, edits, preview memo, convert,
  unknown-kind notice), `wasm_generators.rs` (bindings), `tests/cpu_time/generators.rs`.
- **`frontend`:** the Generators tool in the More tools card, the Generators bar, the schema-drawn Generator
  block, Convert to paths, the unknown-kind notice.
- **CI / deny:** `curvyo-generators-core` in the wasm32 list; `libm` is already allowed.

## Tasks

- [x] 1. Trait and contract in `curvyo-extension-core`, laser migrated, registry `[&LASER, &GENERATORS]`,
  a test double and a test that every declared generator kind answers at its defaults (AC 8).
- [x] 2. `curvyo-generators-core`, tests first: gear involute within 0.01 mm of the exact involute, closed
  outlines, spokes and windows, bore and keyway, clamps and knock-on clamps (AC 2, 4); golden outline of the
  default gear (AC 2).
- [x] 3. Box: finger edges, kerf and clearance, plates side by side, lid none / flat / fingers; mated plates lie
  on one line within the kerf (AC 3, 4); golden outline of the default box.
- [x] 4. Leaf `generated` + `generated_frame`, format 54, goldens `generated_v54`, `generated_unknown_kind_v54`,
  `generated_damaged_v54` in `tests/fixtures`; reopen equals stored, regenerate equals stored; unknown kind
  opens, draws, saves back equal; move/rotate write only the frame (AC 5, 7).
- [x] 5. Commands in document-core: add, change parameter, convert to paths (gear: one path; box: group of
  plates, same `NodeId`, selected in the same commit); history labels (AC 1, 6).
- [x] 6. `instance.rs` in ui-core: schema view, apply edit with schema clamp then generator clamps, notice
  texts of UX notes §6, scale (`Edit::Scale`) for proportional resize, handles (AC 4, 5).
- [x] 7. Editor session and wasm bindings: Generators tool, place at the pointer, Style-tab block data, edit
  field with live preview and one commit, resize writes parameters, unknown-kind notice, refused resize of an
  unknown kind (AC 1, 4, 5, 6, 7).
- [x] 8. CPU-time test per ADR 0080 §8: parameter edit of a 200-tooth gear with spokes, windows, bore and keyway
  and of a box with 100 fingers per edge and finger lid at most 100 ms thread CPU (AC 4).
- [x] 9. Frontend: tool button in More tools, bar with Box / Gear, schema-drawn block, Convert to paths,
  unknown-kind notice (AC 1, 2, 3, 6, 7).
- [x] 10. Step names "Add gear", "Gear: Teeth", "Convert gear to paths" (AC 1); a resize of an object whose
  generator is off or unknown is refused at the press and at the size chip with the UX §12 status, and the
  commit path reads the session's extension settings (AC 7).
- [x] 11. Generated objects resize from the corners only and proportionally (edge and skew handles are neither
  drawn nor hit, the drag and the size chip are linked), K does nothing for them alone (AC 5).
- [x] 12. The Generators ghost draws the real outline (box plates with fingers, gear teeth) in the "Drag ghost"
  look, the defaults generated once per kind (AC 1).
- [x] 13. A selection of several generated objects shows no block; the Node bar says "Generated objects have no
  nodes. Change their parameters or convert them to paths." and the Node tool has no overlay for them.
- [x] 14. Live preview while a field is stepped: the canvas and the block follow the arrow key, the key-up
  is the one commit, Escape drops it (AC 4).
- [x] 15. Instance knobs of UX §9: gear Pitch diameter, Bore, Hub diameter, Rim width and box Inner width,
  depth and height, drawn from 72 px, culled on collision, dragged along the generator's direction with Shift
  snapping to whole millimetres, a blue regenerated preview, the "max" / "min" readout, Escape, one step
  "Gear: Pitch diameter" (AC 5).
- [x] 16. document-core tests for AC 6 and 7: Convert to paths is one undo step, save and reopen keep kind and
  parameters, an unknown kind opens as stored and still converts; ui-core: regeneration equals stored.
  Not built (listed for the lead): the hover dimension look and the double-click entry chip of a knob, the
  Escape / Enter keys of the Generators tool, one block for several generated objects of the same kind, the
  generation-time status of a clamped knob drag in the canvas notice slot.
- [x] 17. Full gate from `CLAUDE.md` §7 once into a file; fix until green (fmt, clippy, doc, frontend green; nextest 16836 tests green, two 70 s sweeps re-run alone after a stopped run).
- [x] 18. Architect review (CHANGES REQUESTED) fixed: `curvyo-generators-core` licence exception in `deny.toml`
  (`cargo deny check licenses` ok); `libm::hypot` instead of `f64::hypot` in generator code; a group resize
  plans every generated resize before writing any (`instance::plan_resize` / `apply_resize`, test
  `a_refused_generator_resize_writes_nothing_of_the_batch`); `gear::generate`, `boxes::generate` below 60 lines;
  `instance_notice.rs` and `generated_outline.rs` split out; `subpaths.remove(0)` replaced by an
  `// invariant:`; Object to path converts generated objects (`generators_session.rs`); a converted box keeps
  its operation (`generated_instance.rs`); stale "proposed" wording in ADR index, ADR 0011/0016/0034.
  Left: `ghost_path` keeps `NodeId::NONE` (its caller has no node either); `commit_group` keeps the
  thread-local settings (debt entry in `docs/technical-debt.md`).
- [x] 19. UX review (CHANGES REQUESTED) fixed: History steps of the Object scope carry the manifest names
  ("Add gear", "Gear: Teeth", "Convert gear to paths"); Layers rows of generated objects read "Gear <id>" /
  "Box <id>" with the Cog / Box glyph (`RowKind::Generated`, `LayerRow::generator`); the Select bar offers
  Object to path for them; the refused resize tells an unknown kind ("a generator this version does not
  have") from an extension switched off ("switched off under Extensions"); the knob memo follows the
  generator's availability, so a knob of a switched-off generator is neither drawn nor hit; the Generators
  rail button is not drawn while no kind is on; Enter in the Generators tool places at the pointer or the
  visible middle; the Generator block's number field is a spin button (`aria-valuenow/min/max`) and Ctrl
  steps by a tenth. Deferred (technical-debt.md): one block for several generated objects of one kind
  (Mixed), the knob hover dimension look and the double-click entry chip of a knob, History names after a
  reopen.

## Validation

Goldens for the file format and for both default outlines, a thread-CPU test (no wall time), the involute and
mating-line property tests with explicit tolerances, and the full gate. Browser check by the tester/ux
review, not by the implementer.

## Own choices (Proposal)

- The CPU test is `curvyo-editor-wasm/tests/generators_cpu_time.rs` (a test binary that includes the
  `cpu_time` helper, as the other budget tests do) instead of `tests/cpu_time/generators.rs`.
- A moved or turned generated object writes `generated_frame` only; a resize goes through
  `instance::resize` inside the gesture's batch and writes record and frame in the same step.

- A generated leaf reads as a compound **path** in every build (`frame x parts`, synthesised anchor ids), so
  render, hit test, clones, SVG export and the laser job need no new arm; the generator-only facts (kind, seed,
  params) are read through `Document::generated(id)`. Path-only writers refuse the leaf because its
  `LeafKind` is not `Path`.
- Gear and box defaults: module 2 mm, 24 teeth, bore 6 mm, solid; box 100 x 80 x 50 mm inner, 3 mm thick,
  fingers 6 mm, no lid, gap 5 mm.

## For the PR review guide (tester-owned files this story touched)

- `curvyo-editor-wasm/tests/acceptance_0093_tester.rs`, `curvyo-ui-core/tests/acceptance_0093_tester.rs` and
  `frontend/tests/acceptance_0093_tester.test.ts` changed: each edit carries a "Superseded by" comment
  (fixture sets of `0113`, G-code writers of `0029`, `curvyo-image-core` / `libm` in the extension crates'
  dependency allow-list, the generators extension staying enabled when Laser is switched off).
- `curvyo-document-core/tests/image_adjust_ai_format.rs` and `gradient_format.rs`: the `replacen` helper
  now builds the version string from `CURRENT_FORMAT_VERSION`, so the next format bump does not break them.
