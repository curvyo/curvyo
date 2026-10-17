# Clipping: punch out a bitmap or a vector with another shape, without destroying either

Status: Draft (a sketch for the customer; not Ready: it adds a node kind and touches the importer and the outputs)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "I forgot clipping as a feature, so you can 'punch out' bitmaps and other vectors."

## User value

As a maker I want to cut a photo to the shape of a heart or a circle, or hide the part of a drawing that lies outside a frame, and still be able to move the shape, move the content, or release the clip later. For an engraving of a photo the clipped bitmap is the usual job; for a cut I want the clipped vector as real outlines.

**What other tools do.**
- **Illustrator**: Clipping Mask (the top object clips what is below; contents stay editable, release is one command) and Pathfinder Crop/Intersect (destructive).
- **Inkscape**: Object > Clip > Set Clip (non-destructive `clip-path`, editable with the Node tool, release unclips); Path > Intersection is the destructive way. A bitmap can be clipped only this way.
- **Affinity**: drag an object onto another to nest it; masks per layer.
- **LightBurn and Cricut**: no live clipping; the user intersects a shape with a bitmap's trace or uses "mask" for images in one case. Cut shapes are always destructive.

## Words

- **Clip:** a closed path, compound path or primitive that limits where another object is visible.
- **Content:** the clipped object (a path, a group or a bitmap of `0058`).

## Two modes (Proposal)

1. **Non-destructive (Set clip):** the clip and the content stay as they are and a clip relationship is stored on a node (like a group with a first child as mask); the canvas shows only what is inside. Release clip gives both objects back, unchanged.
2. **Destructive for vectors (Intersect):** this is the existing Intersection of `0016`, which already gives real outlines; this entry does not repeat it.

For a bitmap there is no destructive form that is also a bitmap in the file (a bitmap in the project file is the source file, `0058`): the clipped bitmap keeps all its pixels; only the visible area changes. This is the only case where the machine output must be decided (Frage 2).

## Acceptance criteria (as far as clear)

1. Given a selection of two or more objects, then **Set clip** and **Release clip** are symbol buttons in the Select bar (always present; dimmed with a tooltip when they do not apply); the topmost closed object is the clip, as with Illustrator; the rest are the content.
2. Given a clip set, then the content is drawn only inside the clip; the clip's own fill and stroke are not drawn (Proposal; the stroke may be kept as an option in the top bar of the Node tool when editing the clip).
3. Given a clipped group, then it is selected as one object and moves, scales and rotates as one; Enter, or a double-click, enters it so that the clip and the content can be edited separately (as a group today, `0023`), with the clip shown in the marking style (`0046`).
4. Given Release clip on a clipped object, then clip and content are two ordinary objects again at their positions, one history step.
5. Given a bitmap (`0058`) as content, then the visible area is the clip; the pixels outside are kept in the file and are not drawn, not exported to PNG/SVG as visible, and not engraved.
6. Given a clip with a hole (compound path), then the hole is a hole.
7. Given a **clipped object in the output** (SVG export, G-code, job preview), then for vectors the clip is applied as real geometry (a boolean intersection at the kernel tolerance 0.01 mm) so that the output has outlines only; the project keeps the live clip.
8. Given SVG import (`0024`, which ignores `clipPath` today, criterion 48), then this entry is the reason to import a simple `clip-path` (one shape, no transform tricks) as a clip; anything else stays ignored without a message. The change of `0024` is a follow-up and is listed here, not made here.
9. Given a clip on a linked clone (`0049`), then the clip belongs to the clone, not to its original.
10. Given the Fracture and Flatten tools (`0037`), then a clipped object is treated by its visible region, like a group.
11. Given the file, then a clipped object is a new node kind or a flag on a group (the ADR decides); a `format_version` bump.

## Out of scope

- Soft masks with transparency (luminosity masks), gradient masks (no gradients in the model, ADR 0002 section 10).
- Clipping by a bitmap or by text before text exists (`0062`).
- Nested clips beyond what a group of clipped groups already gives.
- "Crop bitmap" as a destructive change of the source image.

## Open questions (customer; each has a default)

Frage 1 (CLIP-KIND): A group with a mask, or a flag on any object?
- A (default, recommended): a group whose first child is the clip (no new idea for the user; the Objects panel shows it as a group with a clip icon).
- B: a clip reference on any node (more flexible; each node kind must handle it).
- Default if unanswered: A.

Frage 2 (CLIP-OUTPUT): How is a clipped bitmap engraved?
- A (default): only the visible pixels are rasterised to the machine. B: the clipped area is turned into a vector region that bounds a fill. C: both, depending on the layer role.
- Default if unanswered: A.

Frage 3 (CLIP-SVG): Import clips from SVG?
- A (default): yes for one simple shape (criterion 8). B: no, keep ignoring. Default if unanswered: B until the importer slice is done.

## Links

Requirements: R-EDIT-043 (new)
Depends on: `0023-groups`, `0016-boolean-operations`, `0058-bitmap-import`
Related: `0024-svg-import-export` (criterion 48), `0037-fracture-and-flatten`, `0046-document-shapes` (marking style), `0049-linked-clone`
ADRs: a note on the node kind in `adrs.md` when this becomes Ready (document model, `needs-customer`)
PR: -
