# ADR 0041: Laser roles keyed by layer in the manufacturing

**Status:** Proposed 2026-11-04, `needs-customer` (persistence: what a `.curvyo` file holds; and
what is output, R-MFG-005). Written for Frage 140 A, 141 A and 142 A of
`specs/0027-manufacturing-roles`; what changes on B or C is in that folder's `adrs.md`. Extends
[ADR 0033](0033-manufacturing-data-in-the-project-file.md) §3 (keyed registers next to declared
parameters) and closes for the laser the output question that
[ADR 0015](0015-node-references-and-ancestor-resolved-state.md) §2 and
[ADR 0032](0032-workbench-model-and-registry.md) §6 left open. No new crate, no new dependency.

## Context

A laser manufacturing must say per drawing layer whether its content is cut, engraved or left out
(R-MFG-002), without touching the drawing, so the same layer can have another role in another
manufacturing. ADR 0033 §3 stores process data as one register per **declared** parameter
(`x.<process id>.<param key>`); the set of layers is not declared, it is whatever the drawing has.
Two peers setting roles for two different layers must both keep theirs (ADR 0009 §3).

### Options considered

- **A. One register per layer on the manufacturing, `x.curvyo.laser.role.<layer id>` (chosen).**
  Merge, undo and history per layer for free; a format-36 build already accepts and keeps it.
- **B. One child map container `x.curvyo.laser.roles` with one key per layer.** Merges per key
  once it exists, but two peers that create it concurrently create two containers and the parent
  key keeps one of them, so the first roles of one peer are lost. `process_values`,
  `document.json` and the open check handle only plain values on a node. Rejected.
- **C. One atomic map register of all roles.** One peer's edit overwrites the other's; fails
  `0027` criterion 5. Rejected.
- **D. A register on the layer node.** Writes the drawing (forbidden by `0093` criteria 5 to 7)
  and gives one role for every manufacturing. Rejected.
- **E. Keyed by layer name.** Breaks on rename, collides on duplicate names. Rejected.

## Decision

1. **Storage.** A role is one register on the manufacturing node, key
   `x.curvyo.laser.role.<layer id>`, value the string `"cut"`, `"engrave"` or `"ignore"`. The
   layer id is the node's text form `peer:counter`, the same text as `page.<id>.owner`
   (document-core makes it public as `NodeId::to_register_text` and its parse). The objects in no
   layer use the reserved key `x.curvyo.laser.role.outside` (no node id has that form). **Absent
   means Unassigned**; clearing a role deletes the register. Add manufacturing writes none.
2. **Keyed families extend ADR 0033 §3.** An extension may own, next to its declared parameters, a
   family of registers `x.<process id>.<family>.<node id or reserved word>`. The family and its
   values are defined in the extension crate (`curvyo-laser-core`, module `roles`), not in the
   parameter schema (one user, `CLAUDE.md` §5). Its key form and values are part of the
   extension's published contract (ADR 0034 §5): renaming one is a format change.
3. **Read.** `curvyo-laser-core` reads the roles from the raw `process_values` of the
   manufacturing snapshot. A value that is not one of the three strings reads as **Unknown**:
   no role, not output, shown with a notice, never rewritten on read (ADR 0033 §3). An entry whose
   layer does not exist is kept, ignored and saved back unchanged. Document-core's open check
   refuses a container value under any `x.*` key as damaged, as ADR 0033 §3 already requires ("only
   plain Loro values"); today such a value is skipped silently. No build writes one.
4. **Write.** One generic document-core command writes or deletes one `x.<process id>.*` register
   of a manufacturing in one history step with the caller's label ("Set role"). It is a no-op
   (no step) when the value is unchanged, and refused for a key outside the manufacturing's own
   process namespace and for an unknown or disabled process (ADR 0033 §4). No drawing command
   writes it: deleting a layer leaves its role, and undo of the deletion revives the same node id
   (ADR 0014), so the role is back. Deleting the manufacturing removes the roles with the node.
5. **Merge.** Per register (ADR 0009 §3): roles of different layers both survive; two values for
   the same layer resolve as any register.
6. **What is output (for every reader that produces a job, first `0029`).** A leaf is output iff
   it is effectively visible (hidden is not output, Frage 142 A), it is not a guide, dimension or
   fixture, and the role of the layer that holds it (its top-level ancestor, or `outside`) is Cut
   or Engrave; an image only under Engrave. Locked and opacity do not matter. **Unassigned and
   Unknown are never output** (fail closed), so a newer role value read by an older build is left
   out, never run as something else.
7. **`format_version`: no bump; the project stays at 36.** The new keys are additive `x.*`
   registers (ADR 0033 Consequences). A format-36 build opens the file (its open check accepts any
   plain `x.*` register on a manufacturing), shows no roles, writes and drops none, and saves them
   back. No build outputs a job yet, so no reader can run a file differently for not knowing
   roles; the meaning of "absent" equals what a format-36 build does with a layer. The tightened
   check of §3 refuses only a shape no build writes. Golden file:
   `curvyo-document-core/tests/fixtures/laser_roles_v36.curvyo`. A later change of the meaning of
   absent, of a value's spelling or of the key form is a bump with a migration.

## Consequences

- Roles merge, undo and sync with no new mechanism; the drawing is never written.
- Role entries of deleted layers stay in the file for good (one short register each); clean-up
  is out of scope (`docs/technical-debt.md`).
- A duplicated or pasted layer is a new node and starts without a role.
- A role write is a document change, so the session re-reads the whole drawing once (as any edit
  today, `0050` adrs decision 3); measured against `0027` criterion 21, an incremental read is not
  built ahead.
- Every job reader must apply §6; `0029` gets one shared function from `curvyo-laser-core`
  instead of its own rule.
