# Architect review: 0049 linked clone, milestone 1

Date: 2026-10-14. Branch `story/linked-clone`, diff `origin/main...HEAD` (9066137 plan, 590fe53 M1,
36c0d33 plan update). Read: `specification.md`, `adrs.md`, `plan.md`, ADR 0015, `CLAUDE.md` §5 and
§6, the new modules in `curvyo-document-core/src/` (`affine.rs`, `clone_codec.rs`,
`clone_model.rs`, `clone_read.rs`, `object_read.rs`, `clones.rs`), the changed writers
(`transform_objects.rs`, `objects.rs`, `shapes.rs`, `document.rs`), `tests/linked_clone.rs`,
`tests/clone_format.rs`, and `curvyo-document-core/src/node_read.rs` on
`origin/story/layers-and-objects-panel` (0050).

## Verdict

**CHANGES REQUESTED.** The design is sound and follows ADR 0015 and `adrs.md`. Five must-fix items
below (F1 to F5) protect the file format and the one read rule; they are small and stay inside
milestone 1. No new crate, no new dependency, no trait or generic in product code. The wasm32
build of `curvyo-document-core` passes (checked), and the new modules use no filesystem, clock,
thread or UI.

## Must fix before milestone 2 (F1 to F5)

**F1. The golden test does not check what the golden file stores.**
`tests/clone_format.rs`, `ac37_the_golden_reads_back_and_opening_it_writes_nothing`. Rule: §5
golden-file tests for formats we write; "file formats we write" are protected. The test checks the
count, the maximum depth and that one clone has a hole. If the entry order of `[a, b, c, d, e, f]`
or the compose order changed, the test would still pass, because nothing it reads from
`clone_v12.curvyo` is compared with a fixed value. The bit-exact matrix check in
`ac37_save_and_open_keep_every_link_to_the_last_bit` runs on a document built in memory, not on
the file. Fix: in the golden test, for each of the six non-identity clones, assert `origin` and
`matrix.to_array().map(f64::to_bits)` as read from the file against the literal table in
`golden_document()`. Also assert one resolved point per kind: `second` (quarter turn plus
(30, 0.1)) maps the root's first corner to a literal point, `c3` has a literal `composed`, and the
compound clone's hole is at a literal position.

**F2. A finite composed matrix can still give an infinite outline.** `clone_read.rs` `walk` and
`object_read.rs` `read_one_clone`. Rule: ADR 0015 §3 and decision 6, "a non-finite composed matrix
makes an orphan". The rule exists so that nothing downstream gets inf or NaN. Each link is limited
to ±1e7 only when the file opens, so a hostile file (criterion 39) or a long chain built by merges
can compose to about 1e305, which is finite. Then `apply_point` of a 1e4 mm anchor gives inf, and
the renderer, the hit test and the kernel get infinite coordinates from a clone that is not an
orphan. Fix: a clone is also an orphan when its mapped outline has a non-finite coordinate. That
still depends only on the graph, so it stays independent of the walk order. Check it where the
outline is mapped (`assemble`) and mark the clones behind it as orphans too. A clone behind an
infinite one gets a composed matrix that is at least as large, so a second pass over
`Resolved` states in stack order is enough. Add one unit test: 43 links of scale 1e7 on a 1e4 mm
square gives an orphan.

**F3. There are two resolvers.** `read_one_clone` (`object_read.rs`) reimplements the chain rule
that `resolve` (`clone_read.rs`) owns: following links, the ring check, the kind check, the
composition, the finiteness check. Rule: §5 one responsibility per module. ADR 0015 §6 asks for
one read pass, and "`Document::object(id)` equals the entry of `read_objects()`" is an invariant
that nobody tests. F2 would have to be fixed twice. Fix: `read_one_clone` collects the leaves on
its chain (each clone and the final node, as `RawLeaf`, with a seen-set for the ring) and calls
`resolve` on that short list, then picks `id`. The cost stays O(depth) and the rule lives in one
place. Add a test that `object(id)` equals the `read_objects()` entry for every leaf of the golden
file and of the orphan file.

**F4. `ObjectRead::outline_transforms` pins nothing and is public.** `clone_read.rs`. Rule: §5 no
speculative API, plus test coverage that does not test anything. The counter is incremented on
the line that pushes `ObjectSnapshot::Clone`, so it always equals the number of resolved clones,
whatever the read really transforms. Criterion 63 ("one outline transformation per clone") is
therefore not tested, while the plan marks it done. Fix: remove the field. Count the calls inside
`CloneSnapshot::new` with a `#[cfg(test)]` counter (a thread-local `Cell<usize>` in
`clone_model.rs`) and assert it in the `clone_read` unit tests. The session cache test for the 5 ms
second read belongs to milestone 6.

**F5. The residue rule has no test.** Plan task 1.2 is checked off with "residue ignored (AC 38)",
but neither test file covers it. Rule: Definition of Done, every criterion covered. Fix: two tests
in `clone_format.rs`. (a) A path with a `clone_link` key opens, reads as a path and lists no
`origin` in `document.json`. (b) A clone with `anchors`, style keys, `opacity` and `rotation`
opens, is not damaged, draws the root's outline and style, and its `document.json` entry has only
the four keys.

## Answers to the seven questions

1. **Stored format: confirmed**, with F2 and F5. The `shape="clone"` leaf with one `clone_link`
   map value written with `insert` (`write_clone_link`, test
   `the_link_is_one_plain_value_not_a_container`) matches decision 1. Damaged rules (no link, not a
   map, bad origin, wrong length, non-number, non-finite, beyond ±1e7, clone with children) are
   all tested. The child case is caught by the general leaf rule of 0023, which is fine. Orphan
   rules (missing, wrong kind including group and primitive, ring, followers of either,
   non-finite composed) are correct and independent of the walk order (test
   `the_walk_order_does_not_change_the_orphans`). The fixtures (`clone_v12`,
   `clone_matrix_five_v12`, `clone_orphan_and_ring_v12`) match decision 11; see F1 for what the
   golden test must check.
2. **`document.json` entry: confirmed.** `{"shape":"clone","id","origin","matrix"}` with the matrix
   as the list of six, written for orphans too, because it reads `clone_link_of` before `object`.
   This is right: `document.json` mirrors storage, not the resolved read.
3. **Public API: confirmed**, except `outline_transforms` (F4). `Affine` with public SVG-order
   fields is consistent with `Point`/`Vec2`, which also carry millimetres as `f64`. A newtype for
   e and f would be the only exception in the crate, so this is no §5 violation. `CloneChain` is
   public too, which is fine. `CloneSnapshot::mapped_by` is correct only while nothing in the
   chain moves; milestone 2 must route `ObjectSnapshot::translated`/`rotated` for clones through
   `clone_matrices_after` and drop `mapped_by` if it then has no user.
4. **Boxed chain: accepted.** 160 bytes instead of 304 for every `ObjectSnapshot`, and no
   `large_enum_variant`, is a measured reason; the fields are the ones decision 2 names, and `id`
   lives in `resolved.id`. I will amend `adrs.md` decision 2 (layout) and decision 4 (`affine.rs`
   instead of `units.rs`, also accepted) with dated notes in my next `adrs.md` commit. The
   implementer does not edit `adrs.md`.
5. **Intermediate state: acceptable**, with two notes. No UI path makes a clone before milestone 5,
   and no other build writes format 12, so only tests reach these states. (a)
   `translate_objects` does not refuse a clone: `translate_meta` sees the `clone` tag, finds no
   primitive frame, writes nothing and still commits `translate_objects`. The plan's "writers
   refuse" is not true for it. Milestone 2 task 2.2 fixes it; no change now. (b) `object_ids()`
   listing ids for which `object()` returns `None` is permanent, not only intermediate: between a
   merge and the repair, orphans exist. Document it on `object_ids` in milestone 2.
   Recommendation for milestone 2: about 15 `read_shape_tag(..).is_some()` call sites mean "is a
   primitive" (`translate_meta`, `replace.rs`, `history/restore.rs::kind_of`, the duplicate path).
   Replace them with one `leaf_kind(meta) -> LeafKind { Path, Primitive(tag), Clone }` so that the
   compiler lists every site, the same argument as decision 2.
6. **About 90 test files touched to compile: accepted.** This is the cost decision 2 chose on
   purpose. The format pins changed to `>=` plus "the writer declares the current version" keep
   their meaning. The two entries in the 0017 skip list are correct (damaged file; orphans have no
   style). Expect trivial conflicts with 0050 on the same pins at the rebase.
7. **The 25 s debug test (criterion 39): neither `#[ignore]` nor release-only as it is.** The gate
   never runs ignored tests, so an ignored test would rot. Proving there is no recursion is already
   done fast in every run by the `clone_read` unit test (chain and ring of 10,000, no Loro). Do
   this: move `ac39_hostile_sizes_open_without_recursion` to its own test target
   `tests/clone_hostile_sizes.rs`. Use the full sizes (100,000 clones, chain and ring of 10,000)
   when `!cfg!(debug_assertions)` and reduced sizes in debug (10,000 clones, 1,000 chain and ring),
   the pattern of `compound_write_cost.rs`. Add one line
   `cargo test --release -p curvyo-document-core --test clone_hostile_sizes` to the existing
   release job in `ci.yml`. Milestone 6's budget tests join the same job.

## Section 5 and 6 checks

- Newtypes: `MAX_CLONE_DEPTH`, `depth` and the count are `usize` counts, not units. Matrix entries
  follow the crate's `Point` convention. Tolerances are explicit (`SINGULAR_DETERMINANT`, `<=`
  `MATRIX_ENTRY_LIMIT`). Pass.
- Size: `clone_read.rs` 347 lines, `affine.rs` 241, `clone_codec.rs` 266, all under 500. Every
  function is under 60 lines. Pass.
- `unwrap`: the three in `create_clone_above` and the one in `write_clone_link` carry
  `// invariant:` comments. `&roots[&root]` in `assemble` can panic like an unwrap: add
  `// invariant: every Resolved state names a Path leaf, and all of them are in roots` (or use
  `get` and skip). Should-fix.
- Trait or generic: none in product code. `IdParts` in `tests/clone_format.rs` is a trait with one
  impl (§5 applies to tests too); replace it with two free functions `peer_of(id)` and
  `counter_of(id)`. Should-fix.
- Non-recursive resolver: yes. There is an explicit stack and a state map, and the unwind computes
  each composed matrix once. A ring costs one stack entry per clone. Pass.
- Module responsibility: every new module has a one-sentence doc comment. `object_read.rs` says
  "the whole document ... or one clone", which F3 makes true by sharing one rule.
- Fixture writing goes through `CURVYO_WRITE_FIXTURES` (the repo pattern), not the `#[ignore]`
  regenerate test the plan describes. Fix the plan text.
- Core purity and wasm32: pass.

## Composing with 0050 (`read_nodes`)

The plan holds, with one adjustment to make before milestone 7. Today `read_node` in 0050 calls
`document.object(node)?` for each leaf. On this branch that runs a chain walk per clone, which is
O(depth) each, and outside `resolve`, so `clones_of` and the one-transform rule (criterion 63) do
not hold. At milestone 7, `read_nodes` must collect `RawLeaf` (through `raw_from_meta`) for every
leaf in pre-order, call `resolve` once, and put each result back into its `NodeRead`. `resolve`
leaves orphans out of `objects`, so results can no longer be matched back by position. Recommended
(should-fix in milestone 2 at the latest): have `resolve` return one
`Option<ObjectSnapshot>` per input leaf, in input order (with `orphans` and `clones_of` as today),
and let `read_objects` flatten it. Milestone 7 is then the call-site change the plan promises. Two
further points for milestone 7: a clone's effective opacity is the root's `opacity` register times
the clone's ancestors; the clone's own `opacity` key is residue, and 0050's
`from.opacity * registers.opacity` must not use it. The pre-order of `read_nodes` and
`leaves_under` must give the same leaf order, which a test should pin.

Format plan: confirmed. 12 on one line now, 13 at the rebase onto 0050, never 13 while `main` is
at 11, and the PR is opened only after 0050 has merged.
