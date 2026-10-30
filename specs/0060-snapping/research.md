# Snapping review: official comparison sources

Reviewed 2026-10-30. This is a documentation/source review, not a hands-on
benchmark of either application. Sources describe their own versions; they do
not establish that every advertised interaction is correct in the current build.

## Product identity

The [ArtCraft VectorCraft product page](https://getartcraft.com/apps/vectorcraft)
links to [storytold/vectorcraft](https://github.com/storytold/vectorcraft).
This review uses that upstream project. Independent sites hosting VectorCraft
builds are not authoritative sources for its behaviour.

## Inkscape

The [official 1.2 release notes](https://wiki.inkscape.org/wiki/Release_notes/1.2/en)
document a snap popover beside the global snap icon. Its simple mode exposes
boxes, paths and alignment; advanced mode exposes granular settings. Returning
to simple mode resets settings. Snapping preferences became global rather than
document-owned; the old document snapping tab and per-target tolerance controls
were removed. Alignment/distribution snapping adds temporary alignment guides
and optional distance labels.

The [official 1.4 release notes](https://wiki.inkscape.org/wiki/Release_notes/1.4)
describe further preferences for precedence between grid/guides and
objects/alignment/distribution. The old sketch's description of a permanent
toolbar plus document tolerance dialog should therefore not be used as a
current interface specification.

The upstream [resize modifier request](https://gitlab.com/inkscape/inbox/-/issues/8239)
reports that occupied modifiers prevented temporary suppression while resizing
in 1.2.2. It is historical issue evidence, not proof of current behaviour. It
supports checking each Curvyo tool's modifier combinations explicitly instead
of assuming that a modifier used by another editor works in every tool.

## VectorCraft

Merged [PR 453](https://github.com/storytold/vectorcraft/pull/453) documents
separate display settings for guide colour, alignment lines, anchor/path labels,
measurement labels and transformation readouts. Hiding alignment lines does
not turn snapping off. Its Smart Guides snapping tolerance is configurable in
screen pixels (0–40, default 4); Snap to Point retains its own tolerance.

Merged [PR 2537](https://github.com/storytold/vectorcraft/pull/2537) groups
Smart Guides, Point, Grid, Pixel, Tangent and Perpendicular under View →
Snapping and adds options to Properties. Tangent/perpendicular snapping there
is limited to Pen, Curvature and Line Segment creation from the previous anchor;
the PR explicitly excludes dragged anchors and moved selections from that
addition. Its labels and additional targets are not Curvyo requirements.

Upstream [issue 1280](https://github.com/storytold/vectorcraft/issues/1280)
records large-document slowdowns from whole-document scans and a follow-up
using cached bounds and spatial indexing. It also documents missing nearby
snap targets when a document-order target cap was used. This supports testing
Curvyo's candidate search with dense scenes and targets late in document order,
without adopting either its benchmark numbers or implementation as our budget.

## Curvyo requirements and review consequences

These decisions come from the customer and specification, not from inferred
competitor parity:

- Document-owned settings, restored with the file, despite Inkscape's global
  preference model (AC 1, 14).
- Dedicated right Properties entry: click opens; Shift+click toggles; subtle
  blue signals enabled independently of which panel is open (AC 2, 10).
- Capture/release defaults 8/14 screen pixels with release at least capture;
  release uses raw pointer-derived geometry (AC 3, 4). No reviewed primary
  source establishes equivalent two-zone hysteresis in either competitor.
- Boxes, guide paths, nodes and exact page fractions, with visible/locked,
  group, clone and moving-selection rules (AC 5, 6, 13, 15).
- Every specified tool must work through its actual gesture path; modifiers
  and aspect/axis constraints remain valid when snapping (AC 7, 9).
- Constant-screen-width lines/crosses without hint labels (AC 8).
- An indexed, bounded candidate query per frame, no hidden document-order cap,
  and the existing thread-CPU performance budget (AC 11).
- Exact final geometry, existing one-step gesture history and ephemeral snap
  state (AC 12); document setting changes remain undoable document changes.

Pending customer question SNAP-PAGES: use all visible pages by default,
including each origin and size; active-page-only is the alternative. Old-file
compatibility/migration is not required for this unreleased SID application.
