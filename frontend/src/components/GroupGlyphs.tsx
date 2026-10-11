import type { GroupCommand } from "@/lib/groupText";

/**
 * The glyphs of Group and Ungroup (`docs/design-system.md`, row "Command glyphs"): 16 x 16 viewBox
 * drawn at 20px, 1.5px stroke kept absolute, solid shapes, `currentColor`. Group is a rounded
 * frame around two small solid squares; Ungroup is the two squares moved apart, the frame reduced
 * to corner ticks. Not final art: the pair has to read apart at 1x on the glyph sheet.
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
        <path d="M4.5 4.5H7.5V7.5H4.5Z" fill="currentColor" {...STROKE} />
        <path d="M8.5 8.5H11.5V11.5H8.5Z" fill="currentColor" {...STROKE} />
      </>
    );
  }
  return (
    <>
      <path d={TICKS} fill="none" {...STROKE} />
      <path d="M4 4H6.5V6.5H4Z" fill="currentColor" {...STROKE} />
      <path d="M9.5 9.5H12V12H9.5Z" fill="currentColor" {...STROKE} />
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
