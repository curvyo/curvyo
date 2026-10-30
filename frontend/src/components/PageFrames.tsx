import { useEffect, useState } from "react";
import type { WasmSession } from "@/lib/editorSession";
import { readPages } from "@/lib/pages";
/** Screen-sized page chrome above the un-clipped drawing. Geometry comes from Rust. */
export function PageFrames({
  getSession,
}: {
  getSession: () => WasmSession | null;
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
          <span
            className="absolute bottom-full left-0 mb-1 whitespace-nowrap text-xs"
            style={{ color: "var(--toolbar-icon)" }}
          >
            {page.name}
          </span>
        </div>
      ))}
    </div>
  );
}
