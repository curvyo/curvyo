import type { PathCommand } from "@/lib/pathText";

/**
 * The glyphs of the Path card (`docs/design-system.md`, row "Command glyphs"): 16 x 16 viewBox
 * drawn at 20px, 1.5px stroke kept absolute, solid shapes, `currentColor`. Gaps are measured
 * between stroke edges: the 1.5px strokes of two neighbours each reach 0.75px into the gap, so
 * every gap that must read at 1x is 2.8 units (3.5px) of geometry.
 *
 * Combine is one solid rounded square (11 x 11) with a concentric square hole (5 x 5, evenodd):
 * one object with a hole. Split is a smaller solid square with a hole (7 x 7, hole 3 x 3) at the
 * upper left and, 2.8 away from it, a separate solid 4 x 4 square at the lower right: the plate
 * keeps its hole, only the separate piece leaves. Break apart is the same two pieces with the
 * hole released: the upper left square has no hole.
 */

const STROKE = {
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinejoin: "round" as const,
  vectorEffect: "non-scaling-stroke" as const,
};

/** The 11 x 11 square at (2.5, 2.5), corners rounded by 1, and its 5 x 5 hole at (5.5, 5.5). */
const COMBINED =
  "M3.5 2.5H12.5A1 1 0 0 1 13.5 3.5V12.5A1 1 0 0 1 12.5 13.5H3.5A1 1 0 0 1 2.5 12.5V3.5A1 1 0 0 1 3.5 2.5Z M5.5 5.5V10.5H10.5V5.5Z";

/** The 7 x 7 plate at (1.5, 1.5), corners rounded by 1, and its 3 x 3 hole at (3.5, 3.5). */
const PLATE =
  "M2.5 1.5H7.5A1 1 0 0 1 8.5 2.5V7.5A1 1 0 0 1 7.5 8.5H2.5A1 1 0 0 1 1.5 7.5V2.5A1 1 0 0 1 2.5 1.5Z M3.5 3.5V6.5H6.5V3.5Z";

/** The same 7 x 7 square without the hole. */
const SOLID_PLATE =
  "M2.5 1.5H7.5A1 1 0 0 1 8.5 2.5V7.5A1 1 0 0 1 7.5 8.5H2.5A1 1 0 0 1 1.5 7.5V2.5A1 1 0 0 1 2.5 1.5Z";

/** The separate 4 x 4 piece, 2.8 from the plate's corner on both axes: at (11.3, 11.3). */
const PIECE = "M11.3 11.3H15.3V15.3H11.3Z";

function shapes(command: PathCommand) {
  if (command === "combine") {
    return <path d={COMBINED} fill="currentColor" fillRule="evenodd" {...STROKE} />;
  }
  const plate = command === "split" ? PLATE : SOLID_PLATE;
  return (
    <>
      <path d={plate} fill="currentColor" fillRule="evenodd" {...STROKE} />
      <path d={PIECE} fill="currentColor" {...STROKE} />
    </>
  );
}

/** The glyph of `command`, 20px, in the colour of the text around it. */
export function PathGlyph({ command }: { command: PathCommand }) {
  return (
    <svg width={20} height={20} viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      {shapes(command)}
    </svg>
  );
}
