# Sketch document integration map

2026-10-30. Bounded source navigation for the implementer, not a new ADR or
production review. All paths below are relative to `curvyo-document-core/src/`.
Read with ADR0026; external scaling remains pending the customer's specific answer.
No builds or tests were run for these notes.

## Object identity and resolved geometry

- `primitive_model.rs`: `ObjectSnapshot`, `style`, `style_mut`, `id`, `rotation`,
  `drawn_path`, `translated`, `rotated`. Add an explicit Sketch arm, keeping its
  graph distinct from the derived outline. Shared read-only geometry goes through
  `drawn_path`; this must not grant ordinary path writers permission to edit it.
- `shape_codec.rs`: `LeafKind`, `leaf_kind`, `read_shape_tag_checked` distinguish
  stored kinds. A new tag must not fall through to primitive/path interpretation.
- `object_read.rs`: `Document::read_objects`, `raw_leaf`, `raw_from_meta` are the
  shared read entry. Insert sketch decoding before path fallback; retain one
  original snapshot per read and no solver invocation for each clone.
- `clone_read.rs`: `RawLeaf`, `resolve`, `walk`, `assemble` currently distinguish
  path roots from kinds that cannot be originals. Add sketch root support through
  its resolved outline while returning the original's Sketch snapshot unchanged.
  Preserve finite-outline guards, iterative cycle handling and input stacking order.

## Stored data, JSON and format

- `document.rs`: `CURRENT_FORMAT_VERSION` is 27 at this inspection; allocate the
  next version at integration. Document import calls `path_codec::validate_path_tree`.
- `path_codec.rs`: extend `validate_path_tree` kind dispatch for the sketch codec.
  Distinguish malformed records from valid-but-conflicting graph references.
- `document.rs`: `Document::export_json`, `json_children`, `ObjectJson` conversion
  provide the **non-authoritative** readable snapshot. Include graph/diagnostic
  checkpoint representation without treating JSON as the authoritative importer.
- `container.rs`: `pack`, `unpack`, manifest checking and golden-fixture generator.
  `document.loro` remains authoritative; pack derives both members from one state.
  Keep newer-version refusal and update golden fixtures; no SID migration work.
- Keep per-ID records atomic values, not nested independently editable fields
  within one entity definition. The last-valid checkpoint is derived and
  fingerprinted; do not disguise it as a second writable Path object.

## History and merge granularity

- `history/node_value.rs`: `Node`, `read_map` preserve the distinction between
  a plain register value and a nested Loro map container.
- `history/merge.rs`: `restore_map`, `write_entry` implement conditional restore
  of changed keys while preserving peers' independent changes. Plain map values
  remain atomic; map containers are restored recursively.
- `history/restore_plan.rs`: `Snapshot`, `plan_return` plan object-level restoration.
- `history/restore.rs`: `restore_step`, `apply_plan`, `kind_of` restore records
  and report object kinds. Add sketch reporting; do not introduce whole-graph
  replacement as the undo implementation for a one-dimension edit.
- `history/object_restore.rs` also participates in object restoration. Include
  object undo alongside normal undo/redo, graph/checkpoint mismatches, topology
  revival, independent parameter edits and dangling concurrent references.

## Clone operations, deletion and conversion

- `clone_write.rs`: `origin_is_live`, `clone_limits_hold`, `leaf_style` and the
  limited root read currently have kind-specific assumptions. Accept sketches
  as sources without converting them and without losing clone limit validation.
- `clones.rs`: audit clone-creation kind matching alongside the new snapshot arm;
  a sketch source must remain parametric after Linked clone.
- `clone_repair.rs`: `repair_clones_of_removed` materializes/repoints surviving
  clone chains. Its source read must use the sketch's last-valid resolved path.
- `objects.rs`: `delete_objects` and `history/restore.rs::apply_plan` invoke
  clone repair. Test direct delete and history-driven removal of sketch roots.
- `clone_write.rs::unlink_meta` writes a clone's resolved path; it must never
  copy the source sketch graph into an unlinked clone.
- `shapes.rs`: `convert_to_paths`, `convert_to_paths_uncommitted` are the existing
  transactional conversion pattern; they currently target primitive conversion.
  Sketch conversion must preserve object ID/style, include closed subpaths, remove
  sketch-kind/graph fields coherently and retain source links from existing clones.
- `shape_codec.rs::write_converted_path_fields` and `strip_primitive_keys` are
  primitive-specific helpers, not sufficient sketch cleanup by themselves.

## Ordinary editing boundaries

- `objects.rs`: `translate_meta`, `rotate_object`, duplication/copy helpers;
  `transform_objects.rs`: transform dispatch. Preserve sketch identity and graph.
  Do not implement unresolved external-scale semantics by analogy with paths.
- A new enum variant makes exhaustive matches visible, but generic `drawn_path`
  consumers still need read-versus-write review. Node edits, joins, booleans and
  path conversion must obey explicit conversion boundaries rather than silently
  writing anchors over parametric geometry.
- This map is not proof that renderer/export/session integration is complete;
  compile-guided review and independent acceptance remain required after wiring.
