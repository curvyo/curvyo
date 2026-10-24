# ADR 0022: Texts on carriers and text clones

**Status:** Accepted (customer, Frage 43 A, 2026-10-24).
Proposed by the architect 2026-10-24. `needs-customer` (document model: two new kinds of
reference between nodes; file format: `format_version` bumps). Extends
[ADR 0015](0015-node-references-and-ancestor-resolved-state.md) §1 (the ways a node's meaning may
depend on another node) and [ADR 0019](0019-document-template-and-guide-paths.md) §2 (a clone
reads the guide flag of its origin). Reuses [ADR 0016](0016-clone-overrides.md) §1 (`own_style`)
and builds on [ADR 0020](0020-text-nodes-text-engine-and-font-sources.md) §1 (the text node).
Asked by `specs/0080-text-on-path` (slice 1: criteria 4 to 7, 17 to 19, 24, 25, 34, 35; slice 2:
criteria 37 to 42). Detail per decision: [`0080` adrs.md](../../specs/0080-text-on-path/adrs.md)
decisions 1, 2, 3 and 12.

## Context

A text on path sits on the outline of another node, its carrier: a path, compound path,
primitive, or linked clone of one of these. It follows every edit of the carrier, keeps its shape
when the carrier is deleted (Frage 36 A), and stays a text: editable, convertible, one row in
Layers (customer: "it stays text"). Slice 2 adds text clones: texts that show another text's
string, font and size, each on its own carrier.

ADR 0015 §1 allows a node's meaning to depend on another node in exactly two ways (ancestor state,
`clone_link`) and says a third way needs a new ADR. A text on a carrier is a third way; a text
clone reading its source is a fourth. The primitives must stay primitives (ADR 0002 §6): a circle
carrying a text keeps its handles.

## Options

| Option | Why it won or lost |
|---|---|
| **A. A reference from the text to its carrier, a kept copy of the outline on deletion; text clones as text nodes with a source reference (chosen)** | The carrier stays what it was; the text stays one text node and every text reader keeps working; follows the `clone_link` pattern (one atomic reference, resolved in the one read pass of ADR 0015 §6). |
| B. Convert the carrier into a path and store the text's own copy of the curve | Loses primitives: a circle carrying a text loses its handles and parameters (ADR 0002 §6), and the copy must be rewritten on every carrier edit, which disagrees between peers after a merge (ADR 0015 option B). Rejected. |
| C. A group node "text + carrier" | Changes selection and ownership semantics: clicking the carrier selects a group, the carrier can no longer live in its own layer, a second text on the same carrier needs a second group or a shared one, and every command must learn the pair. Rejected. |

## Decision

### 1. A text references its carrier

Seven optional keys on the text node, all absent on a straight text, a default never written:

```text
text_carrier        { node: NodeId string, outline: u32 }   one atomic value, as clone_link
text_path_position  f64 mm, arc length of the text's middle      always written with text_carrier
text_path_side      "below"                                  absent: above
text_path_line      "top"                                    absent: baseline
text_path_distance  f64 mm                                   absent: 0
text_path_mirror    true                                     absent: false (Mirror letters)
text_carrier_kept   { closed: bool, anchors: [[x, y, in_x, in_y, out_x, out_y], ...] }
                    document mm, one outline, one atomic value; absent: none
```

`text_carrier` is written whole, so a carrier id never pairs with another peer's outline index;
the other values are flat registers so concurrent Side, Position and Distance edits all survive
(ADR 0009 §3). Values tied to the carrier are mm and do not scale with the text. `text_placement`
keeps the straight placement from before and is written only by Release from path. Read rules: an
unknown code reads as the default, a non-finite Position or Distance reads the text as a straight
text at its stored anchor (criterion 35), an `outline` index past the carrier's outlines as its
last outline, a malformed kept outline is ignored. Release from path
deletes all seven keys.

### 2. The carrier becomes a guide; a clone may carry its own guide flag

Put text on path writes the link, the position and the carrier's `guide = true` in one commit (a
carrier on a locked layer is not marked). **Extension of ADR 0019 §2:** a linked clone may carry
its own `guide`; it is a guide when its own flag **or** its root's is set. Marking the root would
turn the original and all its clones into guides. Releasing, removing or converting the text
writes nothing on the carrier.

### 3. Which outline the text sits on, and the kept copy

- **Resolution in the one read pass** (ADR 0015 §6): a live path or compound path gives outline
  `outline`; a primitive gives the outline the renderer and exporters draw (no parametric curve:
  one curve for drawing, cutting and text); a resolved linked clone gives that outline of its
  resolved path. Any other node kind is not a carrier. The read also yields carrier → texts.
- **Fallback order:** live carrier, then `text_carrier_kept`, then none (the text draws straight
  at `text_placement`).
- **Kept copy on removal:** the removal funnel every removal passes (ADR 0015 §5) writes
  `text_carrier_kept` (the outline as resolved just before) on each live text whose carrier is
  removed, in the same commit, and leaves `text_carrier` as it is. Undo of the removal brings the
  carrier back and the live link wins at once.
- **Copy and paste:** a copied pair remaps `text_carrier` to the carrier's copy; a text pasted
  without its carrier gets the outline as kept copy.

### 4. A text clone is a text node with a source reference (slice 2)

A new register on a text node, `text_source: NodeId`. The read pass fills its text from the
source: string, characters, font, size, layout, and style unless the clone has `own_style`
(ADR 0016 §1, reused as is). Its id, name, visible, locked, placement and the seven on-path keys
are its own. String edits in a text clone commit to the source. No chains: a source that is
missing, not a text, or itself a text clone makes an orphan (not drawn; removed by the repair
after Open and merge, ADR 0015 §5). Removing a source unlinks its clones in the same commit: each
gets a copy of the source's values and loses `text_source`; Unlink clone does the same for one.

### 5. ADR 0015 §1, extended

The sentence of ADR 0015 §1 now reads: "A node's meaning may depend on another node in exactly
four ways: ancestor state (ADR 0015 §2), `clone_link` (ADR 0015 §3), a text's `text_carrier` and
a text's `text_source` (ADR 0022 §1 and §4)." A fifth way needs a new ADR. All four resolve in
the one read pass, and every removal repairs all four in the same commit.

## Consequences

- **`format_version` 22 for slice 1** (golden file `text_on_path_v22.curvyo`): a version-21 build
  would draw every text on path straight and a guide clone as artwork (a cut), so it must refuse
  the file (ADR 0004 §9). **`format_version` 23 for slice 2** (`text_source`). No migration in
  either. The numbers are the next free ones at merge.
- If the customer picks B or C, only the `document-core` storage changes; the arc-length measure,
  the letter placement and the UI do not.
- Primitives stay primitives under a text; a carrier edit never writes the text.
- Guide on/off now acts on a selected linked clone (it skipped clones before).
- Merge residue, left for the sharing story: a peer puts a text on a carrier another peer
  deletes; the text has no kept copy and reads straight. Fixable later by a repair after merge,
  no file change.
- No new crate, no new external dependency.
