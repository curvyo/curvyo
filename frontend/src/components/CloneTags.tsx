import { ArrowRight, Link2 } from "lucide-react";
import { SMALL_GLYPH } from "@/components/ui/looks";
import { useLayoutEffect, useRef, useState } from "react";

import { useBarBox } from "@/hooks/useBarBox";
import { PAGE_LABEL_GAP, PAGE_LABEL_HEIGHT } from "@/lib/pages";
import type { CloneTag } from "@/lib/cloneText";

/** A tag is 20 px high (`docs/design-system.md`, "Linked clone marks"). */
const TAG_HEIGHT_PX = 20;
/** The chip's distance from the canvas edges and from the tool rail (`--rail-right` + 12). */
const EDGE_GAP_PX = 12;

const CHIP_STYLE = {
  height: TAG_HEIGHT_PX,
  background: "var(--toolbar-bg)",
  border: "1px solid var(--clone-link)",
  color: "var(--toolbar-icon)",
} as const;

/** An edge chip, centred on the session's point and clamped with its real width right of the
 * tool rail, 12 px below the bars' measured bottom (any number of rows) and inside the canvas
 * (UX review must-fix 6, 0164 F42). */
function EdgeChip({ tag, top }: { tag: CloneTag; top: number }) {
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    setWidth(ref.current?.offsetWidth ?? 0);
  }, [tag.text]);
  const half = width / 2;
  return (
    <div
      ref={ref}
      className="absolute flex items-center gap-1 whitespace-nowrap rounded-[8px] px-1.5 text-xs"
      style={{
        ...CHIP_STYLE,
        left: `clamp(calc(var(--rail-right, 0px) + ${EDGE_GAP_PX}px), ${tag.x - half}px, calc(100% - ${width + EDGE_GAP_PX}px))`,
        top: `clamp(${top}px, ${tag.y - TAG_HEIGHT_PX / 2}px, calc(100% - ${TAG_HEIGHT_PX + EDGE_GAP_PX}px))`,
      }}
    >
      <Link2 {...SMALL_GLYPH} />
      {tag.text}
      <ArrowRight {...SMALL_GLYPH} style={{ transform: `rotate(${tag.toward}rad)` }} />
    </div>
  );
}

interface CloneTagsProps {
  tags: CloneTag[];
}

/**
 * The tags and edge chips of the marks of a selected linked clone (`specs/0049-linked-clone`
 * criteria 57, 58 and 60): DOM chips over the canvas, placed by the session, `aria-hidden`
 * (the selection announcer says the link) and with no pointer events.
 */
export function CloneTags({ tags }: CloneTagsProps) {
  const [root, bars] = useBarBox();
  // Below the bar and the page label that may sit under it (`pageLabelTop`), so a chip never covers it.
  const top = (bars?.bottom ?? 0) + PAGE_LABEL_GAP + PAGE_LABEL_HEIGHT + PAGE_LABEL_GAP;
  if (tags.length === 0) {
    return null;
  }
  return (
    <div ref={root} aria-hidden data-clone-tags="" className="pointer-events-none absolute inset-0 z-[15] overflow-hidden">
      {tags.map((tag, index) =>
        tag.kind === "chip" ? (
          <EdgeChip key={`chip:${tag.text}:${index}`} tag={tag} top={top} />
        ) : (
          <div
            key={`tag:${tag.text}:${index}`}
            className="absolute flex items-center gap-1 whitespace-nowrap rounded-[8px] px-1.5 text-xs"
            style={{ ...CHIP_STYLE, left: tag.x, top: tag.y }}
          >
            <Link2 {...SMALL_GLYPH} />
            {tag.text}
          </div>
        ),
      )}
    </div>
  );
}
