# Architect review, story/template-and-guide-paths (8545f8a2)

Verdict: APPROVE (no must-fix).

Should-fix
1. curvyo-document-core/src/history/checkout.rs:96 repeats the "a clone reads its root's flag" rule
   that node_read.rs:130-150 already implements. Two places can drift; have the preview reuse one helper.
2. curvyo-document-core/src/node_read.rs:81 `is_output_node` has no production caller yet (only
   tests/guides_model.rs). It is the intended single filter (ADR 0019 Consequences), so keep it, but the
   first output slice (0029/0024) must call it and not re-test `.guide`; note it in plan.md.
3. curvyo-editor-wasm/src/session/guides.rs:~100 `loaded.unwrap_or_default()` hides an error, and a
   0-node load still calls show_page_and_guides (viewport moves with nothing loaded). Return early on
   empty/Err.
4. guides.rs:89 the 64 MiB limit is on the packed bytes only; unpack inflation is bounded only by the
   existing Open path (same as before, so debt, not new). Add one line to docs/technical-debt.md.
5. Tests literal-pin the format version in 6 places (0042, 0049, 0057, 0047, history_versions). The edits
   are necessary and minimal, but each future bump repeats them; prefer asserting against
   CURRENT_FORMAT_VERSION. Frontend test titles still say "five buttons" (fileTab.test.ts:73, 0051 c15).
6. path_topology.rs is 1143 lines (pre-existing over §5); this diff adds 17. Not blocking.

Checked, fine
- Core pure, no unsafe, wasm32-safe; no new trait, crate or dependency. guide.rs/template.rs small, one
  responsibility each. `guide` is a plain LWW bool; absent = false; unmark deletes the key; damage rule
  in registers_are_valid; flag on clones is residue and read from the root. Format 15, older builds refuse,
  goldens guide_v15 / guide_damaged_v15 present.
- Criterion 34: Boolean/Combine (all_guides), Join (only a guide `a` with an ordinary `b` is unmarked),
  Split and pieces (inherit), Duplicate (meta copied; tested) are right and tested.
- load_template: anchor-id validation before any write, one commit, deterministic (ids minted by caller),
  top of active layer, guides-else-visible source rule, clone resolved.
- Tester-test edits (0017, 0050, 0042, 0049, 0057, 0051 file tab): legitimate; none weakened (new fixtures
  and the new button added to expected lists, version numbers moved).
