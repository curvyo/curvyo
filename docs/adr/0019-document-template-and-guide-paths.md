# ADR 0019: Document template and guide paths

**Status:** Accepted 2026-10-17 by the customer, as to the design he described in his answers of
that day (`needs-customer`: document model, file format). Extends
[ADR 0002](0002-document-model-units-and-svg-round-trip.md) §2,
[ADR 0004](0004-persistence-and-cross-machine-sync.md) §9 and
[ADR 0009](0009-concurrent-editing-semantics.md) §2. Asked by `specs/0046-document-shapes`.

**History.** The first version of this ADR (Proposed 2026-10-17) put a shape of the document
area (ellipse, rounded rectangle, later an outline) into root registers. The customer said yes
to an ADR, then described a different design: no shaped area, but template outlines that are
ordinary paths marked as guides. The content below replaces the first version completely; his
yes was given for the question, the design is the one from his answers.

## Context

The maker cuts key rings, coasters, labels and discs. He wants the outline of the blank on the
canvas to draw inside it. The customer's design: the document stays a rectangle (`0015`, `0045`);
a template is a set of outlines placed in the document, drawn in a marking style, selectable and
editable like any path, never part of the output. He draws an outline in Curvyo with the existing
tools (a circle, two circles for a donut), marks it, or loads it from an SVG or from another
Curvyo document. Guides and the template are the same thing. What is hard to change later: how
a guide is stored (it is in every file that uses it), and that an older build must not cut a
guide as artwork.

## Decision

1. **The page stays a rectangle.** Document size, rulers, formats and resize are unchanged. No
   shape register, no corner radius register, no outline register.
2. **A guide is a node flag.** One new node register `guide` (bool, absent means false) on a
   path, compound path, rectangle, ellipse, polygon or star node. It is a plain
   last-writer-wins node register (ADR 0009 §2) like `visible` and `locked` of
   [ADR 0015](0015-node-references-and-ancestor-resolved-state.md). A guide keeps its kind and
   parameters, so a circle marked as guide stays an ellipse node and keeps its handles; marking
   and unmarking change nothing else. A clone (ADR 0016, `0049`) reads the flag of its origin.
3. **The flag decides three things and nothing else:** the node is drawn in the marking style
   (green, dashed 2 px on / 2 px off, constant screen width, no fill); it is left out of every
   machine output and of artwork export by default; it is selectable, editable and part of
   Undo as any node. Layers, hidden, locked, opacity and the tree are untouched.
4. **The file version goes up by one (next free at merge).** An older build would draw the guide
   as an ordinary path and a laser job would cut the template; it must refuse the file
   (ADR 0004 §9). Opening writes nothing; unmarking the last guide leaves no key behind. Damage:
   a `guide` value that is not a bool.
5. **Loading a template is a command, not a model feature.** A loader adds nodes with
   `guide = true` in one commit. Sources: another Curvyo document (the nodes are copied with
   their coordinates; both documents share the origin at the top-left) and, after `0024`, an SVG
   (placed with the SVG's top-left at the document's top-left, or the top-left of the
   axis-aligned bounding box when the file has no frame). Nothing about the source is stored: no
   link, no path, no scale. Moving or deleting the source changes nothing.
6. **No scaling, no fit, no checks.** An overhang is allowed. What is in the source comes in.

## Options

| Option | Why it lost or won |
|---|---|
| **A. A `guide` flag on ordinary nodes (chosen)** | The customer's own description: it is the same thing as a template. One bool, no new node kind, every tool already works on it; a circle stays an ellipse node. |
| B. A shape of the page in root registers (the first version of this ADR) | The customer did not want a shaped page: he wants outlines he can draw and edit. Needs a point-in-shape test, a shape per format and a scaled outline; much more model for less use. Lost. |
| C. A new node kind `guide` holding a path | A second path kind: every tool (node edit, Boolean, transform, clone) would have to learn it, and a circle would lose its handles. Lost. |
| D. Guides on a reserved layer | Needs a layer type and a rule that nothing else lives there; "mark this circle" would be a move between layers. The flag is smaller and keeps the tree. Lost. |
| E. Template as an asset referenced from the file | A link that can break and a second source of truth; the customer wants the paths inside the document, editable. Lost. |

## Consequences

- One node register, one version bump, a golden fixture and one damage rule.
- Every consumer that must not see guides (machine output `0029`, export `0024`, nesting, hoop
  checks) filters on `guide`; this is the one place they differ from other nodes. A later
  option "include guides" is a switch of the exporter, not a model change.
- Snapping (`0060`, draft) can use guides as targets; multi-page (`0059`, draft) needs no
  change: a guide is an object at coordinates and belongs to the page whose rectangle holds it,
  as any object does.
- A document-area shape (a round page) is not possible and not wanted; drawing a circle as an
  object, as Inkscape and Illustrator users do, is the supported way.
- No new crate, no trait, no dependency.
