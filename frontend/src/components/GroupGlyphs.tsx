import type { GroupCommand } from "@/lib/groupText";

/**
 * The glyphs of Group and Ungroup (`docs/design-system.md`, row "Command glyphs"): 16 x 16 viewBox
 * drawn at 20px, 1.5px stroke kept absolute, solid shapes, `currentColor`. Group is a rounded
 * frame around two small solid squares; Ungroup is the two squares moved apart, the frame reduced
 * to corner ticks. The squares carry no stroke, so the gaps stay at least 2.4 px at 1x (checked on the glyph sheet by the UX review).
 */

const STROKE = {
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinejoin: "round" as const,
  strokeLinecap: "round" as const,
  vectorEffect: "non-scaling-stroke" as const,
};

/** The frame at (1.5, 1.5), 13 x 13, corners rounded by 2. */
const FRAME = "M3.5 1.5H12.5A2 2 0 0 1 14.5 3.5V12.5A2 2 0 0 1 12.5 14.5H3.5A2 2 0 0 1 1.5 12.5V3.5A2 2 0 0 1 3.5 1.5Z";

/** The four corner ticks of the open frame. */
const TICKS =
  "M1.5 5V3.5A2 2 0 0 1 3.5 1.5H5 M11 1.5H12.5A2 2 0 0 1 14.5 3.5V5 M14.5 11V12.5A2 2 0 0 1 12.5 14.5H11 M5 14.5H3.5A2 2 0 0 1 1.5 12.5V11";

function shapes(command: GroupCommand) {
  if (command === "group") {
    return (
      <>
        <path d={FRAME} fill="none" {...STROKE} />
        <path d="M4.5 4.5H7.25V7.25H4.5ZM8.75 8.75H11.5V11.5H8.75Z" fill="currentColor" />
      </>
    );
  }
  return (
    <>
      <path d={TICKS} fill="none" {...STROKE} />
      <path d="M4 4H6.75V6.75H4ZM9.25 9.25H12V12H9.25Z" fill="currentColor" />
    </>
  );
}

/** The glyph of `command`, 20px, in the colour of the text around it. */
export function GroupGlyph({ command }: { command: GroupCommand }) {
  return (
    <svg width={20} height={20} viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      {shapes(command)}
    </svg>
  );
}
