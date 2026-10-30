# ADR 0024: Spatial pages in one document plane

**Status:** Accepted for the spatial model (customer, 2026-10-17, spec 0059
PAGE-MODEL A); implementation choices below are **Proposal**, vetoable at the
story demo. `needs-customer` document-model authority is the recorded decision,
not a new unanswered question. Written 2026-10-29. Replaces the deferred scope
of rejected ADR 0012; extends ADRs 0002, 0009 and 0014. Customer clarification
2026-10-29: old files do not matter in this unreleased development build;
backward compatibility is not required.

## Context

Several product outlines must share originals, clones and layers while having
independent page frames. The customer chose positional membership and one
object tree. Existing files store a single size in the document root; existing
history restores that root map field by field. The customer explicitly removed
old-file compatibility from this story, so no migration layer is needed.

Options considered:

- **Spatial frames in the existing plane (chosen):** preserves cross-page
  clones, layers and global millimetre coordinates.
- Page nodes owning object subtrees: rejects the customer's overlapping-page
  membership and requires reparenting whenever objects move between pages.
- Separate documents per page: duplicates shared resources and breaks the
  single history and clone graph.
- One serialized page-array register: easy to read but concurrent changes to
  unrelated pages overwrite one another. Rejected in favour of per-field LWW.

## Decision

1. Pages are named, ordered rectangular frames with stable IDs, position and
   size. They never own objects. Membership is calculated from resolved global
   object bounds. Layers remain shared under the spec's stated default.
   ADR 0019 already settled that templates are ordinary guide objects: the old
   spec wording about copying a page's shape introduces no shape register.
2. Global coordinates, object IDs and clone references stay unchanged. Active
   page, last pointer context and frame-only movement are view state. No
   active-page register enters the operation log.
3. **Proposal:** store page fields in the existing root map, using independent
   registers keyed by page ID; keep ID generation peer-safe. A flattened key
   namespace avoids concurrent first-write replacement of a new shared nested
   container. Store ordering separately from geometry, with deterministic ID
   tie-breaking for concurrent equal positions. Reordering never moves frames.
   Do not store the entire page list as one LWW value. The concrete key spelling
   and ordering representation belong in the implementation plan.
4. **Proposal:** create a fully represented first page at (0, 0), named
   "Page 1", as part of new-document initialization. Use the same page model
   from the first edit onward; no lazy migration, implicit legacy page or
   dual old/new storage contract is required. Every current valid document
   exposes at least one page. Concurrent deletion of different remaining
   pages must resolve deterministically without exposing zero pages. The
   implementation may use a read-time fallback to maintain that invariant;
   this is a current-model merge rule, not old-format compatibility.
5. Every page edit is one labelled document operation, including the complete
   object movement for a frame drag. Reuse the existing register-aware restore
   engine and clone-aware transform rules. Partially overlapping objects stay
   in place when a frame moves. A leaf must never receive the translation twice
   through a selected ancestor and itself. History previews retain the page
   list of their version, not the current page list.
6. The container version advances from 25 to **26** (next free at merge).
   The Loro snapshot version is independent and stays unchanged. Add a
   current-format multi-page golden fixture and test save/open round trips.
   Older formats have no compatibility guarantee; no migration or historical
   fixture support is required for this change.
7. No new crate or dependency. `document-core` owns persisted frames and edit
   transactions; `ui-core` owns active-page state, context and input handling;
   resolved geometry supplies bounds; rendering and the existing wasm/frontend
   boundary expose the result. Core code remains free of I/O and UI types.

## Consequences

The shared tree makes linked clones and layers work across frames without
conversion. Rendering must draw all backgrounds before artwork, then page
edges and labels; content is never clipped. Rulers subtract the active frame's
origin. Existing document-size, format, fitting, template placement and preview
consumers must be audited for assumptions that the only origin is (0, 0).

Root-map history can restore page fields using its current merge semantics,
but page labels must be classified as document-level history operations. Tests
must cover concurrent independent page edits, deterministic ordering, deletion,
undo/redo, previews, clone movement and current-format save/open.
The 200-page scenario is a measured performance check, not a page limit.
