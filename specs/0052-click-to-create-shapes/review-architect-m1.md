# Architect review: 0052 milestone M1 (`create_primitive`)

Date: 2026-10-14. Commit reviewed: 58b522c on `story/click-to-create-shapes` (branch diff against
825c2b0). Scope: `curvyo-document-core/src/shapes.rs`, `primitive_model.rs` (doc comment), `plan.md`.

## Verdict

APPROVE. No must-fix findings.

## Checked

1. **The one API addition (`adrs.md` decision 2).** The signature is exactly
   `Document::create_primitive(&self, shape: Shape, rotation: Angle) -> NodeId`. One commit per call,
   labels `create_rect`, `create_ellipse`, `create_polygon`, `create_star` unchanged, so
   `history_names.rs` needs no entry (criterion 6). The four `create_*` methods delegate with a zero
   angle and keep their signatures, so the ~650 call sites and `0050`'s `set_draw_parent` path are
   untouched (decision 9). `create_rect` still writes zero corner radii through the same
   `write_corner_radii_if_changed`, so its stored keys are identical to before; the test
   `create_primitive_without_rotation_writes_what_create_star_and_its_siblings_write` pins that per kind.
2. **§5 units.** `rotation` is an `Angle`; no bare `f64` crosses the API.
3. **§5 tolerance on the zero check.** The exact comparison `rotation.normalized() != 0` is correct
   here and needs no `Tolerance`. It is a storage-encoding decision ("absent reads as 0"), not a
   geometric comparison: writing a register of 1e-16 rad reads back as the same angle as omitting it,
   so no tolerance changes what the document means. A tolerance would instead silently drop a tiny
   but intended turn. It matches the existing writers (`paths.rs` `create_path`: exact `!= 0.0`;
   `convert_to_paths`: exact compare after `normalized()`). Normalizing before the check means a full
   turn (2π) stores nothing, which is the right result.
4. **Normalized storage.** `path_codec::write_rotation` normalizes to (-π, π] itself, as for every
   other writer; the test with 450° confirms it.
5. **No format change (ADR 0004 §9, criterion 36).** No new key, kind or container field. The
   `rotation` register on primitives exists since `0005` and is already written by the Select-tool
   rotate and accepted by open-file validation (`rotation_is_valid`). No `format_version` bump, no
   `Cargo.toml`/`Cargo.lock` change.
6. **§5 structure.** No trait, no generic, no new crate. `create_primitive` is about 45 lines;
   the production part of `shapes.rs` stays under 500 lines (tests follow from line 485). The
   `unwrap` keeps its `// invariant:` comment.
7. **Doc comment** on `PrimitiveSnapshot::rotation` updated as decision 2 asked.

## Notes (no change required for M1)

1. `shapes.rs`, `create_primitive` accepts a nonzero `rotation` for a polygon or star although
   decision 2 says their register stays 0 for a placement (the typed angle goes into
   `StarFrame.angle`). The API is not wrong: a turned star is a valid document state the Select tool
   already produces. The rule belongs in the M2 caller (`ShapePlacement::result` returns
   `rotation = 0` for `PolyStar`); cover it with one M2 test rather than restricting the core API.
2. `Angle::from_radians(0.0)` now appears five times in `shapes.rs`. If M2/M3 add more, an
   `Angle::ZERO` constant in `units.rs` is the simple fix; not worth a change on its own.

## plan.md milestone list

- The order M1 (core API) → M2 (ui-core) → M3 (session/wasm) → M4 (frontend, docs) follows the
  dependency direction and puts the only document-model change first, reviewed in isolation. Good.
- One PR at the end, milestones as commits (`CLAUDE.md` §4). Good.
- **Risk: M3 is the largest milestone** (tasks 6 to 11: click decision, gate, Escape, Enter,
  draw, wasm surface, test rewrites, benchmark). Keep `session/placement.rs` to the one
  responsibility decision 1 gives it; if it passes ~500 lines, split the chip view out
  (e.g. `session/placement_view.rs`) instead of growing `session/mod.rs` or `shapes.rs`.
- **Risk: tests at risk are listed only after M3's first run.** Run the full suite right after the
  click-threshold change (task 6), before the rest of M3, so rewrites per decision 8 are scoped
  early. Rewrite, never delete.
- `shape_placement.rs` and `resolve.rs` must not depend on `curvyo-editor-wasm` or DOM types; the
  `Placed { shape, rotation }` result reuses `curvyo-document-core::Shape` and `Angle`, which keeps
  the existing ui-core → document-core direction. No new edge in the crate graph.
- The `#[ignore]` benchmark (decision 7) belongs in `curvyo-editor-wasm/tests/`, not a new
  `benches/` crate or a `criterion` dependency: no new dependency is planned and none is needed.
