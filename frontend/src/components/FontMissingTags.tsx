// The "Font missing" tags on the canvas (`specs/0074-text-objects` criterion 29, UX notes §4):
// on every text drawn with the substitute face, selected or not, above its line box.
import { TriangleAlert } from "lucide-react";

interface FontMissingTagsProps {
  marks: { x: number; y: number }[];
  /** Hidden during a drag, like the clone tags. */
  hidden: boolean;
}

/** The tag's height and its clearance above the line box, px. */
const TAG_PX = 20;
const CLEAR_PX = 10;

export function FontMissingTags({ marks, hidden }: FontMissingTagsProps) {
  if (hidden || marks.length === 0) {
    return null;
  }
  return (
    <>
      {marks.map((mark, index) => (
        <div
          // The marks have no identity of their own; their order is the drawing order.
          // eslint-disable-next-line react/no-array-index-key
          key={index}
          aria-hidden
          className="pointer-events-none absolute flex items-center gap-1 rounded-lg border border-[var(--field-invalid)] bg-[var(--toolbar-bg)] px-1.5 text-xs whitespace-nowrap text-[var(--toolbar-icon)]"
          style={{ left: mark.x, top: mark.y - TAG_PX - CLEAR_PX, height: TAG_PX }}
        >
          <TriangleAlert size={12} className="text-[var(--field-invalid)]" />
          Font missing
        </div>
      ))}
    </>
  );
}
