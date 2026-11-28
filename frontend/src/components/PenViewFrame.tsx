// The pen window's frame in the main window (`specs/0158-pen-display` slice 2b criterion 6, UX
// 10.5, design system "Pen window frame"): four ink corner brackets joined by a faint line and a
// "Pen window" tab around the area the pen window shows. An overlay only: not in the document, no
// pointer input, clipped at the canvas edge.
import { useEffect, useRef } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { bracketPath } from "@/lib/penViewFrame";
import { penViewFrameShown } from "@/lib/penWindowStore";

/** The frame, redrawn every animation frame from the session's last pen view. */
export function PenViewFrame({ getSession }: { getSession: () => WasmSession | null }) {
  const group = useRef<SVGGElement>(null);
  const tab = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let last = "";
    let frame = requestAnimationFrame(function draw() {
      const rect = penViewFrameShown() ? (getSession()?.link_pen_frame() ?? []) : [];
      const key = Array.from(rect).join(" ");
      if (key !== last) {
        last = key;
        const shown = rect.length === 4;
        const g = group.current;
        if (g) {
          g.style.display = shown ? "" : "none";
          if (shown) {
            const [x0, y0, x1, y1] = Array.from(rect);
            const box = `M${x0} ${y0}H${x1}V${y1}H${x0}Z`;
            const brackets = bracketPath([x0, y0, x1, y1]);
            for (const path of Array.from(g.querySelectorAll("path"))) {
              path.setAttribute("d", path.dataset.part === "line" ? box : brackets);
            }
          }
        }
        if (tab.current) {
          tab.current.style.display = shown ? "" : "none";
          if (shown) tab.current.style.transform = `translate(${rect[0]}px, ${rect[1] - 4}px) translateY(-100%)`;
        }
      }
      frame = requestAnimationFrame(draw);
    });
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 z-10 overflow-hidden" data-pen-view-frame="">
      <svg className="absolute inset-0 h-full w-full overflow-hidden">
        <g ref={group} fill="none" style={{ display: "none" }}>
          <path data-part="line" stroke="var(--selection-casing, #FFFFFF)" strokeWidth={3} />
          <path data-part="line" stroke="var(--pen-view-frame)" strokeOpacity={0.4} strokeWidth={1} />
          <path data-part="brackets" stroke="var(--selection-casing, #FFFFFF)" strokeWidth={4} />
          <path data-part="brackets" stroke="var(--pen-view-frame)" strokeWidth={2} />
        </g>
      </svg>
      <div
        ref={tab}
        className="absolute top-0 left-0 rounded-full bg-[var(--toolbar-bg)] px-2 text-[12px] leading-5 whitespace-nowrap text-[var(--toolbar-icon)]"
        style={{ display: "none" }}
      >
        Pen window
      </div>
    </div>
  );
}
