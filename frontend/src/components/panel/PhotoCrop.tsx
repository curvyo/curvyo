import { Crop } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { HANDLES, arrowStep, moveFrame, resizeFrame, shownHeight, sizeReadout, type Frame, type Handle } from "@/lib/photoSize";

const HANDLE_NAMES: Record<Handle, string> = {
  nw: "top left",
  n: "top",
  ne: "top right",
  e: "right",
  se: "bottom right",
  s: "bottom",
  sw: "bottom left",
  w: "left",
};

const BOX = 228;

/** Where a handle sits on its frame, in percent of the frame. */
function handlePlace(handle: Handle): { left: string; top: string } {
  const x = handle.includes("w") ? "0%" : handle.includes("e") ? "100%" : "50%";
  const y = handle.includes("n") ? "0%" : handle.includes("s") ? "100%" : "50%";
  return { left: x, top: y };
}

interface PhotoCropProps {
  /** The image to cut, as an object URL, with its pixel size. */
  url: string;
  width: number;
  height: number;
  onCancel: () => void;
  /** Crop: the frame in image pixels. */
  onCrop: (frame: Frame) => void;
}

/**
 * Crop (`specs/0112` `ux-notes.md` 8.5): the image with a frame and eight handles, the outside at
 * 35 % strength, the size readout, Cancel and Crop. Arrow keys move the frame by 1 px (Shift 10)
 * when it has focus; Shift+arrows on a focused handle resize it. Enter crops, Escape cancels.
 */
export function PhotoCrop({ url, width, height, onCancel, onCrop }: PhotoCropProps) {
  const [frame, setFrame] = useState<Frame>({ x: 0, y: 0, width, height });
  const bounds = { width, height };
  const scale = BOX / width;
  const boxHeight = shownHeight(width, height, BOX);
  const drag = useRef<{ kind: "move" | Handle; x: number; y: number; start: Frame } | null>(null);
  // Crop replaces the button that opened it: the frame takes the focus, so the arrows act at once.
  const frameRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    frameRef.current?.focus();
  }, []);

  const startDrag = (kind: "move" | Handle) => (event: React.PointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { kind, x: event.clientX, y: event.clientY, start: frame };
  };
  const onMove = (event: React.PointerEvent<HTMLElement>) => {
    const current = drag.current;
    if (!current) return;
    const dx = (event.clientX - current.x) / scale;
    const dy = (event.clientY - current.y) / scale;
    setFrame(
      current.kind === "move" ? moveFrame(current.start, dx, dy, bounds) : resizeFrame(current.start, current.kind, dx, dy, bounds),
    );
  };
  const endDrag = () => {
    drag.current = null;
  };
  const keys = (event: React.KeyboardEvent, handle: Handle | null) => {
    if (event.key === "Enter") {
      event.preventDefault();
      onCrop(frame);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onCancel();
      return;
    }
    const step = arrowStep(event.key, event.shiftKey && handle === null);
    if (!step) return;
    event.preventDefault();
    if (handle === null) setFrame(moveFrame(frame, step.dx, step.dy, bounds));
    else if (event.shiftKey) setFrame(resizeFrame(frame, handle, step.dx, step.dy, bounds));
  };

  return (
    <div
      role="group"
      aria-label="Crop photo"
      className="flex flex-col gap-1"
      onKeyDown={(event) => {
        // Escape anywhere in Crop cancels it (not the test detail's Back).
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          onCancel();
        }
      }}
    >
      <div
        className="relative overflow-hidden bg-[var(--canvas-bg)]"
        style={{ width: BOX, height: boxHeight }}
      >
        <img src={url} alt="" draggable={false} className="absolute inset-0 h-full w-full select-none" />
        <div
          ref={frameRef}
          role="group"
          aria-label="Crop frame"
          tabIndex={0}
          onKeyDown={(event) => keys(event, null)}
          onPointerDown={startDrag("move")}
          onPointerMove={onMove}
          onPointerUp={endDrag}
          className="absolute cursor-move outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          style={{
            left: frame.x * scale,
            top: frame.y * scale,
            width: frame.width * scale,
            height: frame.height * scale,
            outline: "1.5px solid var(--accent)",
            boxShadow: "0 0 0 9999px color-mix(in srgb, var(--canvas-bg) 65%, transparent)",
            touchAction: "none",
          }}
        >
          {HANDLES.map((handle) => (
            <button
              key={handle}
              type="button"
              aria-label={`Resize ${HANDLE_NAMES[handle]}`}
              onKeyDown={(event) => keys(event, handle)}
              onPointerDown={startDrag(handle)}
              onPointerMove={onMove}
              onPointerUp={endDrag}
              className="absolute size-3 -translate-x-1/2 -translate-y-1/2 rounded-[3px] border-[1.5px] border-[var(--accent)] bg-[var(--toolbar-bg)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              style={{ ...handlePlace(handle), touchAction: "none" }}
            />
          ))}
        </div>
      </div>
      <p className="text-[11px] tabular-nums text-[var(--panel-muted-fg)]">{sizeReadout(frame.width, frame.height)}</p>
      <div className="flex gap-1">
        <button
          type="button"
          onClick={onCancel}
          className="h-7 w-[112px] rounded border border-[var(--lane-line)] text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={() => onCrop(frame)}
          className="flex h-7 w-[112px] items-center justify-center gap-1 rounded bg-[var(--toolbar-icon-active-bg)] text-xs text-white focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          <Crop size={16} aria-hidden />
          Crop
        </button>
      </div>
    </div>
  );
}
