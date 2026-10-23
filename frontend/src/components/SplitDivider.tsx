// The drag handle on the split pane's left edge (`specs/0077-font-preview` criterion 27, UX notes
// §1): a pointer drag or the arrow keys resize the pane, a double-click resets it.
import { type KeyboardEvent, type PointerEvent, useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";

import { DRAWING_MIN_PX, SPLIT_KEY_STEP_PX, oneColumnPane, splitAfterDrag } from "@/lib/fontPreviewLayout";
import { latestPerFrame } from "@/lib/latestPerFrame";

interface SplitDividerProps {
  areaWidth: number;
  size: number;
  /** The pane's width in px. */
  width: number;
  /** A drag moved the divider: the pane's new share of the canvas area, not kept yet. Called at
   * most once per animation frame, inside it, and committed there, so the pane takes its width
   * in that frame. Must keep its identity: the first one is used. */
  onDrag: (share: number) => void;
  /** A pressed divider moved for the first time: the drag starts (not on a mere click). */
  onDragStart: () => void;
  /** The drag ended: keep its last share. */
  onDragEnd: () => void;
  /** An arrow key moved the divider: keep this share. */
  onCommit: (share: number) => void;
  /** Double-click: back to the default width, the kept share forgotten. */
  onReset: () => void;
}

export function SplitDivider({
  areaWidth,
  size,
  width,
  onDrag,
  onDragStart,
  onDragEnd,
  onCommit,
  onReset,
}: SplitDividerProps) {
  const start = useRef<{ x: number; width: number; moved: boolean } | null>(null);
  const [dragging, setDragging] = useState(false);
  // `onDrag` is stable (the first one is kept).
  const [moves] = useState(() => latestPerFrame((share) => flushSync(() => onDrag(share))));
  useEffect(() => () => moves.cancel(), [moves]);
  const percent = (px: number) => Math.round((px / areaWidth) * 100);

  const down = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    start.current = { x: event.clientX, width, moved: false };
    setDragging(true);
  };
  const move = (event: PointerEvent<HTMLDivElement>) => {
    if (start.current) {
      if (!start.current.moved) {
        if (event.clientX === start.current.x) {
          return;
        }
        start.current.moved = true;
        flushSync(onDragStart);
      }
      // The divider sits on the pane's left edge: moving it left widens the pane.
      moves.push(splitAfterDrag(areaWidth, size, start.current.width - (event.clientX - start.current.x)));
    }
  };
  const end = () => {
    if (start.current) {
      start.current = null;
      moves.flush();
      setDragging(false);
      onDragEnd();
    }
  };
  const key = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = { ArrowLeft: SPLIT_KEY_STEP_PX, ArrowRight: -SPLIT_KEY_STEP_PX }[event.key];
    if (step !== undefined) {
      event.preventDefault();
      onCommit(splitAfterDrag(areaWidth, size, width + step));
    }
  };

  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label="Font preview width"
      aria-valuemin={percent(oneColumnPane(size))}
      aria-valuemax={percent(areaWidth - DRAWING_MIN_PX)}
      aria-valuenow={percent(width)}
      aria-valuetext={`${width} px`}
      tabIndex={0}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={end}
      onPointerCancel={end}
      onLostPointerCapture={end}
      onDoubleClick={onReset}
      onKeyDown={key}
      className="group absolute top-0 bottom-0 -left-[4px] z-10 w-[7px] cursor-col-resize touch-none outline-none"
    >
      <span
        aria-hidden
        className={`absolute top-0 bottom-0 left-[2px] w-[3px] ${
          dragging
            ? "bg-[var(--editor-accent)]"
            : "group-hover:bg-[var(--editor-accent)] group-focus-visible:bg-[var(--editor-accent)]"
        }`}
      />
    </div>
  );
}
