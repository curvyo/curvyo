// The canvas area with one optional side pane (`specs/0077-font-preview` criteria 2 to 5 and 27,
// `adrs.md` decision 1): the drawing, and the pane at its right (split, its width dragged at the
// divider) or over the whole area (full), the drawing's view kept where it was whenever the pane
// takes or gives back width; the pane lies over the drawing's canvas, which keeps its size.
import { type CSSProperties, type ReactNode, useCallback, useLayoutEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";

import { SplitDivider } from "@/components/SplitDivider";
import {
  CANVAS_OVERHANG_VAR,
  type PaneLayout,
  drawingLoss,
  drawingWidthDelta,
  heldFull,
  paneLayout,
  switchedView,
} from "@/lib/fontPreviewLayout";

interface CanvasAreaProps {
  /** Tells the session the visible canvas width is about to change by `delta` CSS px. */
  keepViewOrigin: (delta: number) => void;
  /** Tells the session the pane now lies over the canvas's right `px` (0 for none). */
  coverDrawing: (px: number) => void;
  open: boolean;
  wantFull: boolean;
  /** The preview's Size: one tile column at it sets the split pane's narrowest width. */
  size: number;
  /** Toggles the stored split/full choice. */
  toggleFull: () => void;
  /** The kept dragged split's share of the area; undefined for the default width. */
  split: number | undefined;
  /** Keeps a dragged split, or forgets it (undefined). */
  setSplit: (split: number | undefined) => void;
  /** The rulers, the viewport and its bars. */
  drawing: ReactNode;
  /** The pane, given its layout, the Full view/Split view switch and whether the divider is being
   * dragged. */
  pane: (layout: PaneLayout, switchView: () => void, resizing: boolean) => ReactNode;
}

export function CanvasArea({
  keepViewOrigin,
  coverDrawing,
  open,
  wantFull,
  size,
  toggleFull,
  split,
  setSplit,
  drawing,
  pane,
}: CanvasAreaProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [areaWidth, setAreaWidth] = useState(0);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) {
      return undefined;
    }
    setAreaWidth(el.clientWidth);
    // Committed in the observer, before the frame is shown: the pane and the drawing take their
    // new widths in the same frame as the window.
    const observer = new ResizeObserver(() => flushSync(() => setAreaWidth(el.clientWidth)));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // A forced full view holds until the maker switches or closes (criterion 3).
  const [held, setHeld] = useState(false);
  const holds = heldFull(held, open, areaWidth, size);
  if (holds !== held) {
    setHeld(holds);
  }
  const switchView = useCallback(() => {
    if (switchedView(wantFull, holds).wantFull !== wantFull) {
      toggleFull();
    }
    setHeld(false);
  }, [wantFull, holds, toggleFull]);
  // A drag shows its share live and keeps it when it ends (criterion 27); while it moves, the
  // tile grid keeps its column count where it can (`resizing`).
  const [dragging, setDragging] = useState(false);
  const [dragShare, setDragShare] = useState<number | undefined>(undefined);
  const dragRef = useRef<number | undefined>(undefined);
  const drag = useCallback((share: number) => {
    dragRef.current = share;
    setDragShare(share);
  }, []);
  const startDrag = useCallback(() => setDragging(true), []);
  const endDrag = useCallback(() => {
    if (dragRef.current !== undefined) {
      setSplit(dragRef.current);
    }
    dragRef.current = undefined;
    setDragShare(undefined);
    setDragging(false);
  }, [setSplit]);
  const share = dragShare ?? split;
  const layout = paneLayout(areaWidth, size, wantFull, holds, share);
  const loss = drawingLoss(open, layout);
  // The drawing's canvas element always spans the whole area and is resized by the window only:
  // the split pane lies over its right part (customer report 2026-10-23: every canvas resize
  // flickered on the Linux desktop). The session hears which part is covered, for its fit,
  // centring and visibility rules. Opening, closing, switching views and moving the divider (not a
  // window resize) keep the drawing's top-left origin (criterion 2).
  const last = useRef({ key: "false|true|", loss: 0 });
  const key = `${open}|${layout.full}|${share ?? ""}`;
  useLayoutEffect(() => {
    if (key !== last.current.key) {
      const delta = drawingWidthDelta(last.current.loss, loss);
      if (delta !== 0) {
        keepViewOrigin(delta);
      }
    }
    if (loss !== last.current.loss) {
      coverDrawing(loss);
    }
    last.current = { key, loss };
  }, [key, loss, keepViewOrigin, coverDrawing]);

  const covered = open && layout.full;
  return (
    <div ref={ref} className="relative flex min-h-0 min-w-0 flex-1">
      {/* Hidden, not unmounted, under the full view: the canvas keeps its surface. */}
      <div
        inert={covered}
        className={`flex min-h-0 min-w-0 flex-1 ${covered ? "invisible" : ""}`}
        style={{ [CANVAS_OVERHANG_VAR]: `${loss}px` } as CSSProperties}
      >
        {drawing}
      </div>
      {open ? (
        <div
          className={`z-30 flex min-h-0 min-w-0 flex-col bg-[var(--canvas-bg)] ${
            layout.full
              ? "absolute inset-0"
              : `relative shrink-0 border-l border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)]`
          }`}
          style={layout.full ? undefined : { width: layout.width }}
        >
          {layout.full ? null : (
            <SplitDivider
              areaWidth={areaWidth}
              size={size}
              width={layout.width}
              onDrag={drag}
              onDragStart={startDrag}
              onDragEnd={endDrag}
              onCommit={setSplit}
              onReset={() => setSplit(undefined)}
            />
          )}
          {pane(layout, switchView, dragging)}
        </div>
      ) : null}
    </div>
  );
}
