/**
 * Where to put the live numeric readout chip
 * (`specs/0005-object-transform/specification.md`, UX notes; the
 * design-system "Live transform readout" row): normally `offset` px up and
 * to the right of the pointer, but never off the canvas — flipped to the
 * left of the pointer when it would cross the right edge, flipped below
 * it when it would cross the top edge, then clamped inside the canvas as
 * a last resort (a canvas smaller than the chip).
 */
export interface Size {
  width: number;
  height: number;
}

export interface Placement {
  left: number;
  top: number;
}

/** A rectangle in the canvas's own coordinates, CSS pixels. */
export interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** Clear space kept between the chip and an avoided rectangle, CSS pixels. */
const AVOID_GAP_PX = 4;

function meets(placement: Placement, chip: Size, rect: Rect): boolean {
  return (
    placement.left < rect.right + AVOID_GAP_PX &&
    placement.left + chip.width > rect.left - AVOID_GAP_PX &&
    placement.top < rect.bottom + AVOID_GAP_PX &&
    placement.top + chip.height > rect.top - AVOID_GAP_PX
  );
}

/**
 * `avoid` is a floating bar the chip must not land on (the Select bar,
 * `specs/0014-advanced-selection/`): when the chip would meet it, it flips below
 * the pointer like it does at the top edge, and when the pointer itself is
 * over the bar, below the bar.
 */
export function placeReadout(
  anchor: { x: number; y: number },
  chip: Size,
  canvas: Size,
  offset: number,
  avoid?: Rect,
): Placement {
  let left = anchor.x + offset;
  let top = anchor.y - offset - chip.height;
  if (left + chip.width > canvas.width) {
    left = anchor.x - offset - chip.width;
  }
  if (top < 0) {
    top = anchor.y + offset;
  }
  if (avoid && meets({ left, top }, chip, avoid)) {
    top = anchor.y + offset;
    if (meets({ left, top }, chip, avoid)) {
      top = avoid.bottom + AVOID_GAP_PX;
    }
  }
  return {
    left: Math.max(0, Math.min(left, canvas.width - chip.width)),
    top: Math.max(0, Math.min(top, canvas.height - chip.height)),
  };
}

/**
 * Like [`placeReadout`], but on the side of the pointer given by `away` (a unit vector, y down):
 * right when it points right, left when it points left, below when it points down, above when up.
 * Used where the usual up-and-right place may cover geometry the chip is about (the Pen's hint
 * chip over a closing node). Clamped inside the canvas.
 */
export function placeAway(
  anchor: { x: number; y: number },
  chip: Size,
  canvas: Size,
  offset: number,
  away: { x: number; y: number },
): Placement {
  const left = away.x >= 0 ? anchor.x + offset : anchor.x - offset - chip.width;
  const top = away.y >= 0 ? anchor.y + offset : anchor.y - offset - chip.height;
  return clampInside(left, top, chip, canvas);
}

/** Distance from a handle's glyph edge to the entry chip, in CSS pixels
 * (`docs/design-system.md`, "Transform entry chip": 10px clear space). */
const ENTRY_CHIP_GAP_PX = 10;

/** Half the largest handle glyph (the 12px rotate arrow), in CSS pixels. */
const HANDLE_GLYPH_RADIUS_PX = 6;

function clampInside(left: number, top: number, chip: Size, canvas: Size): Placement {
  return {
    left: Math.max(0, Math.min(left, canvas.width - chip.width)),
    top: Math.max(0, Math.min(top, canvas.height - chip.height)),
  };
}

function coversPoint(placement: Placement, chip: Size, point: { x: number; y: number }): boolean {
  const margin = HANDLE_GLYPH_RADIUS_PX;
  return (
    point.x >= placement.left - margin &&
    point.x <= placement.left + chip.width + margin &&
    point.y >= placement.top - margin &&
    point.y <= placement.top + chip.height + margin
  );
}

/**
 * Where to put the typed-entry chip (`object-transform-refinements` UX
 * notes, "Numeric entry control"): upright, centred outward from the handle
 * along the line from the box centre through it, with 10px clear of the
 * handle's glyph edge (`glyphReach`: further out where a skew arrow sits
 * on the same side), then clamped inside the canvas. If the clamp would
 * cover the handle it flips to the inner side. While the handle is off
 * screen the chip stays clamped at the canvas edge.
 */
export function placeEntryChip(
  handle: { x: number; y: number },
  center: { x: number; y: number },
  chip: Size,
  canvas: Size,
  glyphReach = HANDLE_GLYPH_RADIUS_PX,
): Placement {
  let dx = handle.x - center.x;
  let dy = handle.y - center.y;
  const length = Math.hypot(dx, dy);
  if (length < 1e-6) {
    dx = 0;
    dy = -1;
  } else {
    dx /= length;
    dy /= length;
  }
  const at = (sign: number): Placement => {
    // The chip's half-extent along the direction, so the nearest point of
    // the chip (not its centre) is the gap away from the glyph.
    const reach =
      glyphReach +
      ENTRY_CHIP_GAP_PX +
      (Math.abs(dx) * chip.width) / 2 +
      (Math.abs(dy) * chip.height) / 2;
    const cx = handle.x + sign * dx * reach;
    const cy = handle.y + sign * dy * reach;
    return clampInside(cx - chip.width / 2, cy - chip.height / 2, chip, canvas);
  };
  const outward = at(1);
  return coversPoint(outward, chip, handle) ? at(-1) : outward;
}

/** Distance of the typed-move chip's near corner from the centre glyph, in
 * CSS pixels, on both axes (`docs/design-system.md`, "Move entry chip"). */
const MOVE_CHIP_OFFSET_PX = 16;

/**
 * Where to put the typed-move chip (`edit-interaction-polish` criterion 18):
 * its top left 16px right of and 16px below the centre glyph, so it never
 * covers the glyph, flipped to the left of the glyph when it would cross the
 * right edge and above it when it would cross the bottom edge (the readout's
 * rule, mirrored), then clamped inside the canvas.
 */
export function placeMoveChip(
  center: { x: number; y: number },
  chip: Size,
  canvas: Size,
): Placement {
  let left = center.x + MOVE_CHIP_OFFSET_PX;
  let top = center.y + MOVE_CHIP_OFFSET_PX;
  if (left + chip.width > canvas.width) {
    left = center.x - MOVE_CHIP_OFFSET_PX - chip.width;
  }
  if (top + chip.height > canvas.height) {
    top = center.y - MOVE_CHIP_OFFSET_PX - chip.height;
  }
  return clampInside(left, top, chip, canvas);
}

/**
 * Where to put the typed-move chip with the reference control (`specs/0047-transform-polish/`
 * UX notes, "Chip"): left as `placeMoveChip` has it, the top 16px below the lower edge of the
 * box (never higher than the centre glyph's 16px), so the lower right mark is not under the
 * chip for a box of ordinary size. It flips above the centre glyph when the chip, taken at its
 * Absolute height `flipHeight` (the same in both modes, so switching the mode never changes the
 * side), would cross the bottom edge, then it is clamped inside the canvas.
 */
export function placeMoveEntryChip(
  anchor: { center: { x: number; y: number }; boxBottomY: number },
  chip: Size,
  canvas: Size,
  flipHeight: number,
): Placement {
  const { center, boxBottomY } = anchor;
  let left = center.x + MOVE_CHIP_OFFSET_PX;
  let top = Math.max(center.y, boxBottomY) + MOVE_CHIP_OFFSET_PX;
  if (left + chip.width > canvas.width) {
    left = center.x - MOVE_CHIP_OFFSET_PX - chip.width;
  }
  if (top + flipHeight > canvas.height) {
    top = center.y - MOVE_CHIP_OFFSET_PX - chip.height;
  }
  return clampInside(left, top, chip, canvas);
}

/** The Offset chip's gap below or above the result box, in CSS pixels: the bottom corner handles
 * reach 13.5px below the box (`specs/0038-path-offset/ux-rework.md` 2.4). */
const OFFSET_CHIP_GAP_PX = 20;

/**
 * Where to put the Offset entry chip (`specs/0038-path-offset/ux-rework.md` 2.4): its left edge
 * 16px right of the centre glyph and its top 20px below the box around the selection and its
 * standing preview; left of the centre at the right edge and above the box (not over it) at the
 * bottom edge, so the outline being judged stays visible; then clamped inside the canvas and right
 * of the tool rail (`railClear`).
 */
export function placeOffsetEntryChip(
  anchor: {
    center: { x: number; y: number };
    box: { left: number; top: number; right: number; bottom: number };
  },
  chip: Size,
  canvas: Size,
  railClear: number,
): Placement {
  const { center, box } = anchor;
  let left = center.x + MOVE_CHIP_OFFSET_PX;
  let top = box.bottom + OFFSET_CHIP_GAP_PX;
  if (left + chip.width > canvas.width) {
    left = center.x - MOVE_CHIP_OFFSET_PX - chip.width;
  }
  if (top + chip.height > canvas.height) {
    top = box.top - OFFSET_CHIP_GAP_PX - chip.height;
  }
  const clamped = clampInside(left, top, chip, canvas);
  return { ...clamped, left: Math.max(clamped.left, railClear) };
}

/** The placement chip's gap from the pointer, and from the pivot marker when it flips
 * (`docs/design-system.md`, "Placement chip"). */
const PLACEMENT_CHIP_GAP_PX = 12;
/** Half the 6 px pivot marker. */
const PIVOT_MARKER_HALF_PX = 3;

/**
 * The click-to-create placement chip (`specs/0052-click-to-create-shapes/` criteria 11, 33): its
 * corner nearest the pointer 12 px from the pointer on the side away from A on each axis (an axis
 * with no offset counts as right or down). On an axis without room the chip goes to the other
 * side of A only when the other axis does not already keep it clear of A; otherwise it is clamped
 * on that axis, so it stays next to the pointer. It keeps right of `railClear` when it fits there
 * (at 800 px the rectangle chip cannot, which is accepted), and off `avoid`, the active tool's
 * bar, by moving to the pointer's other vertical side or below the bar.
 */
export function placePlacementChip(
  pointer: { x: number; y: number },
  a: { x: number; y: number },
  chip: Size,
  canvas: Size,
  avoid?: Rect,
  railClear = 0,
): Placement {
  const clear = PLACEMENT_CHIP_GAP_PX + PIVOT_MARKER_HALF_PX;
  const minLeft = canvas.width - railClear >= chip.width ? railClear : 0;
  const near = (p: number, at: number, size: number) =>
    p >= at ? p + PLACEMENT_CHIP_GAP_PX : p - PLACEMENT_CHIP_GAP_PX - size;
  const otherSide = (p: number, at: number, size: number) =>
    p >= at ? at - clear - size : at + clear;
  const fits = (start: number, size: number, low: number, room: number) =>
    start >= low && start + size <= room;
  const keepsClear = (start: number, size: number, at: number) =>
    start > at + PIVOT_MARKER_HALF_PX || start + size < at - PIVOT_MARKER_HALF_PX;
  const clampAxis = (start: number, size: number, low: number, room: number) =>
    Math.max(low, Math.min(start, room - size));

  let left = near(pointer.x, a.x, chip.width);
  let top = near(pointer.y, a.y, chip.height);
  if (!fits(left, chip.width, minLeft, canvas.width)) {
    left = keepsClear(top, chip.height, a.y)
      ? clampAxis(left, chip.width, minLeft, canvas.width)
      : otherSide(pointer.x, a.x, chip.width);
  }
  if (!fits(top, chip.height, 0, canvas.height)) {
    top = keepsClear(left, chip.width, a.x)
      ? clampAxis(top, chip.height, 0, canvas.height)
      : otherSide(pointer.y, a.y, chip.height);
  }
  if (avoid && meets({ left, top }, chip, avoid)) {
    const flipped =
      pointer.y >= a.y
        ? pointer.y - PLACEMENT_CHIP_GAP_PX - chip.height
        : pointer.y + PLACEMENT_CHIP_GAP_PX;
    top =
      fits(flipped, chip.height, 0, canvas.height) && !meets({ left, top: flipped }, chip, avoid)
        ? flipped
        : avoid.bottom + AVOID_GAP_PX;
  }
  const clamped = clampInside(left, top, chip, canvas);
  return { ...clamped, left: Math.max(clamped.left, minLeft) };
}

/** Whether the segment from `a` to `b` crosses the chip's rectangle at `at` (Liang-Barsky). */
function segmentMeets(
  a: { x: number; y: number },
  b: { x: number; y: number },
  at: Placement,
  chip: Size,
): boolean {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  let low = 0;
  let high = 1;
  const edges: [number, number][] = [
    [-dx, a.x - at.left],
    [dx, at.left + chip.width - a.x],
    [-dy, a.y - at.top],
    [dy, at.top + chip.height - a.y],
  ];
  for (const [p, q] of edges) {
    if (p === 0) {
      if (q < 0) {
        return false;
      }
    } else if (p < 0) {
      low = Math.max(low, q / p);
    } else {
      high = Math.min(high, q / p);
    }
  }
  return low <= high;
}

/**
 * The Pen entry chip (`specs/0072-pen-angle-snap-and-typed-values/`): the placement chip's rule
 * with the last node L as A. With `end`, the typed point of a frozen chip, a chip that would
 * cover the segment from L to it goes to the other side of L vertically, the rule's own
 * fallback, clamped in the canvas.
 */
export function placePenEntryChip(
  pointer: { x: number; y: number },
  last: { x: number; y: number },
  end: { x: number; y: number } | null,
  chip: Size,
  canvas: Size,
  railClear = 0,
): Placement {
  const at = placePlacementChip(pointer, last, chip, canvas, undefined, railClear);
  if (!end || !segmentMeets(last, end, at, chip)) {
    return at;
  }
  const clear = PLACEMENT_CHIP_GAP_PX + PIVOT_MARKER_HALF_PX;
  const top = at.top >= last.y ? last.y - clear - chip.height : last.y + clear;
  return { left: at.left, top: Math.max(0, Math.min(top, canvas.height - chip.height)) };
}
