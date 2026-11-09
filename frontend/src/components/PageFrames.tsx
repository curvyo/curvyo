import { useEffect, useState } from "react";
import type { WasmSession } from "@/lib/editorSession";
import { readPages } from "@/lib/pages";
import { KindChip } from "@/components/KindChip";
/** Screen-sized page chrome above the un-clipped drawing. Geometry comes from Rust. */
export function PageFrames({
  getSession,
  dropTarget = null,
}: {
  getSession: () => WasmSession | null;
  /** The laser page a canvas drag of objects is over (its badge is no refusal): frame cue. */
  dropTarget?: string | null;
}) {
  const [json, setJson] = useState("[]");
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const next = getSession()?.pages_json() ?? "[]";
      if (next !== previous) {
        previous = next;
        setJson(next);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  return (
    <div
      aria-hidden
      className="pointer-events-none absolute inset-0 overflow-hidden"
      data-page-frames=""
    >
      {readPages(json).map((page) => (
        <div
          key={page.id}
          className="absolute"
          style={{
            left: page.screenX,
            top: page.screenY,
            width: page.screenWidth,
            height: page.screenHeight,
            outline: `${page.active ? 2 : 1}px solid var(--toolbar-icon)`,
            outlineOffset: -1,
            outlineColor: page.active
              ? "var(--toolbar-icon)"
              : "color-mix(in srgb, var(--toolbar-icon) 65%, transparent)",
          }}
        >
          {/* The frame cue of a page-row drop (`specs/0099` UX notes §2): 2px outside. */}
          {(page.dropTarget || page.id === dropTarget) && (
            <div
              data-drop-target=""
              className="absolute"
              style={{ inset: -4, outline: "2px solid var(--editor-accent)" }}
            />
          )}
          {/* The label is never wider than its frame: the name truncates first, the word goes
              below 96 px, the whole label below 24 px (`specs/0098` UX notes §5). */}
          {page.screenWidth >= 24 && (
            <span
              data-page-label=""
              className="absolute bottom-full left-0 mb-1 flex max-w-full items-center gap-1.5 whitespace-nowrap text-xs"
              style={{ color: "var(--toolbar-icon)" }}
            >
              <span className={`min-w-0 truncate ${page.active || page.home ? "font-semibold" : ""}`}>{page.name}</span>
              <KindChip
                size={12}
                kind={{
                  icon: page.kindIcon,
                  word: page.screenWidth >= 96 ? page.kindWord : "",
                  off: page.kindOff,
                }}
              />
            </span>
          )}
        </div>
      ))}
    </div>
  );
}
