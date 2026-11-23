# ADR 0080: Generated objects, the `Extension` trait and one parametric-instance mechanism

**Status:** Accepted (customer, 2026-11-23: Frage 326 GEN-STORED A; Frage 327 GEN-RAIL: the Generators tool goes
into the More tools card). Proposed by the architect 2026-11-23, `needs-customer` (document model, file format, plugin
interface). The customer decided the route (spec `0141` Frage 277 A: box and gear through the
extension contract; Frage 278 A: a live object sharing one instance mechanism with the sketch
instances of `0099` criteria 23 to 31; EXT-NEXT A: extract the trait now). Frage 326
(GEN-STORED) answered A. Specs: [`0141`](../../specs/0141-parametric-generators/specification.md),
[`0099`](../../specs/0099-sketch-pages/specification.md) revision 2026-11-19. Amends
[ADR 0034](0034-extension-contract-compiled-in.md) §3, §4, §8 (the trait trigger is met); adds a
crate to [ADR 0011](0011-workspace-and-crate-layout.md) §2, §3; extends
[ADR 0016](0016-clone-overrides.md) (a further clone override) and follows
[ADR 0033](0033-manufacturing-data-in-the-project-file.md) §2 to §4 (atomic parameter maps,
unknown kinds kept) and [ADR 0029](0029-live-pattern-fill.md) §5, §6, §11 (determinism, memo,
drag previews).

**In plain words (for the customer).** A box or a gear is one object that remembers its numbers.
Change a number and the outline is rebuilt; the file stores the numbers and the outline that was
built, so it opens exactly as saved on every computer, also in a build that does not know the
generator (then it is kept as drawn and cannot be changed by number). A sketch placed with its own
values uses the same numbers panel, the same handles and the same Convert to paths.

## Context

The laser is the only extension crate; ADR 0034 §3 put off an `Extension` trait until a second
one. Box and Gear are that second one. Three facts shape the design:

- `curvyo-extension-core → curvyo-document-core`, so document-core can never call a generator.
  Every outline consumer (render, hit test, page membership, clones, SVG export, the laser job)
  reads geometry through document-core's read pass. Sketch clones can be solved there, because
  `curvyo-sketch-core` sits below document-core; generators cannot.
- `shape_codec::leaf_kind` sends any unknown `shape` tag to `LeafKind::Primitive(tag)`, so a new
  leaf kind needs a `format_version` bump.
- Spec `0141` criterion 7 asks that a file with an unknown generator kind opens "with the last
  paths", so the last outline must be in the file.

### Options: where a generated object's geometry comes from

- **A. Stored: one atomic value with kind, seed, parameters and the generated outline; the
  generator runs only on writes (chosen).** Every consumer reads an outline as today; an unknown
  kind is drawn and cut from what is stored; a merge never pairs one peer's numbers with another
  peer's outline (ADR 0033 §2 wrote `fixture_params` whole for the same reason).
- **B. Derived on read (ADR 0029's pattern fill).** Needs a session overlay in front of every
  consumer (ADR 0029 §7's synthetic paths, for render, hit test, membership, clones, exports and
  the job), and an unknown kind draws nothing. Rejected.
- **C. Parameters and outline in separate registers.** A concurrent edit can leave one peer's
  parameters with the other's outline. Rejected.
- **D. A path node with extra registers (no `shape` tag).** An older build would let the maker
  edit nodes of an outline the next regeneration overwrites; every committed edit would rewrite
  thousands of anchor maps. Rejected.

### Options: the trait

- **A. `manifest()` plus one method per contribution kind, each with a "contributes nothing"
  default (chosen).** Two implementations (laser, generators) and a test double; the registry
  holds `&'static dyn Extension` and the `match id` arms go.
- **B. Function pointers inside the manifest.** No trait, but the manifest stops being plain data
  (no `PartialEq`, no JSON for the front end without a filter). Rejected; the customer asked for
  the trait.
- **C. Keep `match id`.** The trigger is met (EXT-NEXT A). Rejected.

## Decision

1. **The trait** (`curvyo-extension-core/src/extension.rs`):
   ```rust
   pub trait Extension: Sync {
       fn manifest(&self) -> Manifest;
       fn notice(&self, id: &str, context: &NoticeContext) -> Option<String> { None }
       fn generate(&self, request: &GenerateRequest<'_>) -> Option<Generated> { None }
   }
   ```
   The laser implements `manifest` and `notice`, the generators `manifest` and `generate`. A
   contribution kind is added with its method (only kinds a story uses, ADR 0034 §4). A ui-core
   test asserts that every declared generator kind answers `generate` at its defaults.
   `curvyo-ui-core/src/extensions.rs` holds `[&LASER, &GENERATORS]`; `manifests`, `notice` and
   `generate` find the extension by manifest id and honour `extensions.toml`. **Out of scope:** the
   many direct `curvyo_laser_core::` calls in ui-core and editor-wasm (settings, operations, job)
   stay as they are; the trait covers the registry dispatch only.
2. **The generator contract** (pure, deterministic, no I/O, clock or randomness source):
   - `GenerateRequest { kind: &str, params: &StoredParams, seed: Seed, edit: Option<Edit> }`,
     `Edit::Param(key)` (this value was typed or dragged; e.g. Pitch diameter wins over Module) or
     `Edit::Scale(Ratio)` (criterion 5: the generator decides which lengths scale).
   - `Generated { params: StoredParams, parts: Vec<Part>, handles: Vec<Handle>, clamps:
     Vec<Clamp> }`: `params` are the normalised values the host writes (schema clamps by
     `Param::accept` first, then the generator's dependent and knock-on clamps; permit over refuse,
     never an error); `Clamp { key, value, bound: Minimum | Maximum | Dependent }` lets the shell
     write the notice sentences of the UX notes §6; a `Part` is one compound outline
     (`Vec<ClosedPath>`, holes by the compound-path rule of `0035`) in frame-local mm: a gear is
     one part, a box one part per plate; `handles` are `{ key, anchor: Point, direction: Vec2 }`
     for length parameters (empty allowed).
   - `ClosedPath { start: Point, segments: Vec<Cubic> }`, `Cubic { c1, c2, to }`, a line written
     with `c1 = start`, `c2 = to`. Contract types use document-core units only (ADR 0034 §2).
   - `Seed(u32)` is part of the contract and the file (spec wording "(parameters, seed)"); set once
     from the node id at creation, kept by copies (ADR 0029 §5); box and gear ignore it.
   - `Contribution::Generator(GeneratorKind { id, label, icon, groups: Vec<ParamGroup { label,
     params }> })`; the Generators entry lists the enabled ones. The schema elements of the UX notes
     §4: `ParamKind` gains `Count { min, max }`, `Angle { min, max, step, decimals }`, `Switch` and
     `Choice(options)`; `Length` gains `step` and `decimals`; `Param` gains `shown_when:
     Option<ShownWhen>` (`On(key)`, `Is(key, choice)`, `Positive(key)`). A derived length (Pitch
     diameter) needs no kind: it is an ordinary stored length the generator keeps consistent, and
     `Edit::Param` tells it which value wins. Stored as one-key maps `{"mm": n}`, `{"deg": n}`,
     `{"count": n}`, `{"on": b}`, `{"choice": "id"}`. Generated objects keep `params` opaque in
     document-core; `ParamValue` and this codec move to document-core (re-exported by
     extension-core at its path) with `0099`'s revision, whose read pass decodes them (§6, §9).
   - Trigonometry through `libm`, never `f64::sin` (ADR 0029 §5), so two peers write equal outlines.
3. **The crate `curvyo-generators-core`** (ADR 0034 §1): extension id `curvyo.generators`, kinds
   `gear` and `box`; modules `gear/` (involute flank, spokes and windows, bore and keyway), `boxes/`
   (finger edges, kerf and clearance, layout), `manifest.rs`. Edges: `→ extension-core,
   document-core, libm`; `ui-core → generators-core`. Outlines are built closed by construction; no
   geometry-core edge until a generator needs booleans (allowed then by ADR 0034 §2). In CI's wasm32
   list. No clock dev-dependency (ADR 0064 §9).
4. **The generated object in the document** (next free `format_version` at merge, 54 on main today):
   - a leaf with `shape = "generated"` and `LeafKind::Generated` in `shape_codec::leaf_kind` (the
     compiler lists every site that must decide);
   - `generated`: one atomic map `{ kind: "<extension id>/<kind>", seed: u32, params: { key: value },
     parts: [[[x0, y0, c1x, c1y, c2x, c2y, x, y, ...], ...], ...] }` (parts, each a list of
     subpaths), written whole by create, a parameter edit and a scale; frame-local mm;
   - `generated_frame`: `[a, b, c, d, e, f]`, rotation, mirror and translation only; move, rotate
     and mirror write only this register, so they merge with a concurrent parameter edit;
   - style keys as on a path; a clone of it reads its outline as of any outline root (ADR 0045).
   The document reads `frame × parts` as the leaf's geometry in every build (one leaf, several
   outlines; the UI never enters it); no consumer of outlines changes. A known kind is not
   regenerated on open (never rewrite on read); Frage 326.
   Damage refuses the file: `generated` not a map, a `kind` that is not `<id>/<kind>`, a seed
   outside u32, no part or an empty part, a subpath not `2 + 6n` finite numbers within ±1e7, a frame that
   is not six finite numbers or holds scale or shear beyond 1e-9. `params` are opaque to
   document-core; the extension reads them (a bad value reads as its default, shown, never
   rewritten on read, ADR 0033 §3).
5. **Unknown or disabled kind: keep everything, write nothing** (ADR 0033 §4). The object draws,
   hits and goes to the job from its stored outline and shows as plain paths (no Generator block;
   one canvas notice after the open, UX notes §12); it moves, rotates, mirrors, is copied, deleted
   and converted; a scale is refused with a status notice (it cannot be regenerated; Convert to
   paths first). Nothing is converted on open and a save keeps the record, so a newer build gets
   its parameters back. A new generator kind is therefore
   additive and needs no `format_version` bump; renaming a parameter key is an extension-side
   migration (ADR 0034 §5); a changed generator output needs none (stored outlines stay).
6. **One parametric-instance mechanism**, `curvyo-ui-core/src/instance.rs`, over
   `enum InstanceSource { Sketch { clone: NodeId }, Generated { node: NodeId } }`:
   - **Schema:** both present `Vec<Param>`: a generator from its manifest; a sketch clone from the
     sketch's exposed parameters (Length for distance, radius and diameter, Angle, Count, with the
     typed limits; key = the `ParameterId`). One Properties block (ADR 0034 `Params`) for both.
   - **Values:** one record, `{ key: value }` in the §2 encoding. A sketch clone holds its own
     values in one atomic map `instance_params` (absent key = the sketch's value; Back to the
     sketch's value removes the key; along a clone chain the nearest own value wins, `0099`
     criterion 28); the sketch holds `exposed_params`, one atomic list of `{ id, min, max }`.
     Document-core solves a clone with its values in the read pass, one solve per distinct value set
     (`0099` criterion 26), and keeps the last valid geometry with "No solution for these values".
   - **Edit:** clamp by schema, then generate or solve; one commit "Change parameter"
     (`change_parameter`). A drag or typing preview writes nothing; the session memoises one result
     per distinct `(source, values)`, compared in full (ADR 0029 §6); Escape drops it.
   - **Handles:** length parameters get a handle (a generator's `handles`, a sketch's dimension
     anchors), angle a rotation handle, count none (`0099` criterion 25); whether `0141`'s first
     slice shows them is the UX notes' call.
   - **Scale** (criterion 5, Proposal): on a generated object a scale becomes `Edit::Scale` and one
     commit of `generated` and `generated_frame` under the transform's label. The UI resizes
     generated objects proportionally only (UX notes §8); a non-uniform factor from any other
     writer uses the geometric mean of the two. A sketch clone keeps its matrix scale (`0057`).
   - **Convert to paths:** one commit. A gear (one part) becomes a path, compound when it has
     holes, with the same `NodeId`; a box (several parts) becomes a group with the same `NodeId`
     (the `group` marker set on the node, one path child per part); `frame × parts` written as anchors and `extra_subpaths`, `shape`,
     `generated` and `generated_frame` removed, as "object to path" removes `shape`; the result is
     selected in the same commit. Operation assignments (`0101`) stay on the id; for a group the
     commit writes the inner keys of the new paths as a group assignment does (ADR 0056 §2). A
     sketch instance converts by
     Unlink with its own values (`0099` criterion 29). A concurrent parameter edit on a converted
     node leaves a stray `generated` key on a path, which is never read (tolerated keys,
     `shape_codec`).
7. **History labels** (in `history_names.rs`, `step_scope.rs`, the history row guard): three static
   keys, `add_generated`, `change_parameter`, `convert_generated`; the step records its subject
   (kind, parameter key) as `;<key>=<value>` fields of the commit message (ADR 0014 §2 grammar,
   `history/step_header.rs`; no format change) and ui-core composes the row text of the UX notes §7 from the manifest
   labels ("Add gear", "Gear: Teeth", "Convert gear to paths"). Fallbacks when the kind is unknown
   or the subject is missing: "Add generated object", "Change parameter", "Convert to paths". A
   sketch instance's edit is "Change parameter" (`0099` criterion 25). A mixed selection converted
   by Object to path keeps `convert_to_paths`.
8. **Tests.** generators-core: the involute flank against the exact involute at 0.01 mm (sampled,
   explicit tolerance), every subpath closed, the box's mated plates on one line within the kerf
   value (criterion 3), clamping with notices, and golden outlines of the default box and gear
   (freezing the output). document-core: goldens `generated_v<N>.curvyo` (box and gear),
   `generated_unknown_kind_v<N>.curvyo` (opens, draws, saves back equal) and
   `generated_damaged_v<N>.curvyo`; reopen and regenerate equals the stored outline (criterion 7).
   CPU time (criterion 4) in `curvyo-editor-wasm/tests/cpu_time/generators.rs`, thread CPU with the
   existing clock: the session's parameter edit (generate, the one commit, the frame's canvas view)
   at most 100 ms for a gear of 200 teeth with spokes, windows, bore and keyway, and for a box with
   100 fingers per edge and a finger lid.
9. **The sketch side lands with `0099`'s revision**, with its own `format_version` bump and goldens
   (`instance_params`, `exposed_params`); whichever slice merges first builds `instance.rs` and the
   Params block, the other adds its arm.

> **Note 2026-11-23 (architect review of `story/extensions-sdk`).** The first slice differs from
> §2 and §8 as follows; the code is the record. `Edit::Scale` carries `ScaleFactor(f64)` (the
> `Ratio` above). `Handle` gains `rate` (value change per mm of drag, 2 for a diameter knob).
> `Bound` gains `DependentMinimum` (a dependent bound that raised the value). The CPU budget test
> is `curvyo-editor-wasm/tests/generators_cpu_time.rs` (it uses `tests/cpu_time/`). The leaf's
> outline anchors carry synthesised ids (`AnchorId(0, ordinal << 20 | index)`), unique within one
> object only and never written; Convert to paths mints real ones. `NodeId::NONE` (peer 0,
> counter 0) names the ghost's outline; it must never reach a write. The resize path reads the
> extension settings from a thread-local set by the session (`ui-core/src/extensions.rs`), a
> stopgap until `commit_group` takes `&ExtensionSettings` (`docs/technical-debt.md`).

## Consequences

- Every committed parameter edit stores the whole outline: about 1,200 cubics, some 60 KB, for a
  200-tooth gear, in the file and in the history. Drags commit once on release. Measured and stated
  in the PR; a compact encoding is a later, migratable change.
- Files are stable: a later, better generator changes an object only when a number is changed.
- Laser output, SVG export and clones need no change; they read an outline.
- The extension contract grows a method, a contribution, four parameter kinds and a path type; all
  `0.x`, recorded in `curvyo-extension-core/CHANGELOG.md`.
- A third extension is one crate, one array entry and, for a new contribution kind, one method.

## Customer question

**Frage 326 (GEN-STORED): opening a file with a box or gear.** A: it opens exactly as saved; a
later Curvyo with a better generator changes it only when you change a number (recommended: what
was cut stays what you cut, and every build can open it). B: rebuilt from the numbers on every open
(after an update the outline may differ from what you cut; every view, export and the laser job
then need a second geometry source, more work). Default: A. **Answered A, 2026-11-23.**
