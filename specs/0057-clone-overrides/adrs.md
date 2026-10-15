# ADRs for Clone overrides

Architect review 2026-10-14. The spec needs **one new ADR, [0016](../../docs/adr/0016-clone-overrides.md)**
(Proposed, `needs-customer`: document model and file format). No new crate, dependency, trait or
generic.

## Depends on

- [ADR 0016](../../docs/adr/0016-clone-overrides.md): the two registers `own_placement` and
  `own_style` (the latter with the ordinary style keys and the node's `opacity`); the held-back
  rule; style source and effective opacity from the nearest clone with own style (Frage 5, default
  A); repair, Unlink, merge and residue; the format bump.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §2 to §7: the
  `opacity` register, the atomic `clone_link`, the one read, the repair on removal, and the refusal
  of newer files. 0016 extends §3 (style and opacity source), §4 (residue) and §5 (style copy on
  re-point); a clone gets an `opacity` register of its own only with `own_style`.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §3: last writer wins per register;
  each override is its own register or a style key.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) §1: undo and Restore field by field.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: format bump.
- [`specs/0049-linked-clone/adrs.md`](../0049-linked-clone/adrs.md): decision 1 (a clone has no style
  keys: changed to "only with `own_style`"), 3 (shared tessellation predicate), 4 (`clone_matrices_after`
  gets the held-back rule), 5 (Unlink writes the effective style and opacity).
- [`specs/0050-layers-and-objects-panel/`](../0050-layers-and-objects-panel/): the Opacity field and the
  effective opacity of a clone (criteria 26 and 28) change as `specification.md` lists.
- [`specs/0056-mirror-and-negative-scale/adrs.md`](../0056-mirror-and-negative-scale/adrs.md): a Flip is a
  G with determinant -1; it is invertible, so held-back works.

## Feature-local decisions

1. **Disagreements with the specification, fixed in it (2026-10-14).**
   - Decision 1 asked for the flag inside the atomic link. Not needed (ADR 0016 §1).
   - Criterion 38 cited ADR 0015 §4 for residue it does not yet list; it cites 0016 §5 too.
   - Criterion 41 gets a frame-time sentence for colour-only own styles (shared tessellation).
   - Re-check after the customer's answers (second pass, same day): criterion 3 no longer says
     "no other object is written" (the held-back clones are); 7 and 31(b) say what the copy
     contains; 16 names the register that is read (the style source's, not multiplied by its
     ancestors); 18 keeps the eyedropper; 38 limits the style check to clones with own style; 42
     handles a clone without fill; the table gains `0049` criteria 12 and 22 and `0050`'s clone
     caption. The pivot criteria (19 to 25) and the second exception to the "writes only the
     clones it moves" rule fell with the pivot.
2. **Risks accepted, not changed.** A Node tool drag of every node flows through own placement;
   an Object Restore of the original flows through; two peers that hold back one clone keep one
   matrix. All are in ADR 0016 Consequences.
3. **Small notes.** Held-back writes on a locked or hidden clone are allowed: its drawn outline
   does not change. SVG export (`0024`): a clone with any override is an ordinary `<path>` with
   its effective style, as criterion 40 says; if `0049` Question 7 ever writes plain clones as
   `<use>`, a clone with own style stays a `<path>`.
4. **M1 review notes (2026-10-14, [review-architect-m1.md](review-architect-m1.md)).**
   - *"All style keys" (ADR 0016 §1)* means a complete style as the path writer stores it:
     `style_codec::write_style` (width and colour always, other keys only when not default),
     after every style key and `opacity` on the clone has been removed. Absent = default, so the
     read is the same as writing every key.
   - *`document.json` (plan Frage 1):* flags only when true, `style` only with `own_style`,
     `opacity` from the registers, residue `opacity` left out. Non-authoritative.
   - *Flag type check (plan Frage 2):* only on clone nodes. No writer produces a mistyped flag, a
     path never becomes a clone, and validation keeps its one dispatch on `shape`.
   - *Locked clones (plan Frage 3):* both switches are allowed on a locked clone; neither changes
     the drawn result, which is what the lock guards.
