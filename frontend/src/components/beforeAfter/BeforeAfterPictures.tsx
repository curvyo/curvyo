import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent, PointerEvent as ReactPointerEvent, WheelEvent } from "react";

import type { BeforeAfterApi } from "@/hooks/useBeforeAfter";
import type { BeforeAfterView, PaneImage } from "@/lib/beforeAfterView";
import { fittedZoom, needsSmoothing, placedSizeMm, sizeAt100, wheelZoom, zoomedAbout } from "@/lib/beforeAfterZoom";

/** The pixels are fetched at most this often while the dots are worked (the job syncs per frame). */
const REDRAW_MS = 80;
const DIVIDER_STEP = 1;

/** Draws RGBA8 `pixels` into `canvas` at their own size; `false` when the byte count is wrong. */
function drawPixels(canvas: HTMLCanvasElement, image: PaneImage, pixels: Uint8ClampedArray): boolean {
  if (pixels.length !== image.width * image.height * 4) return false;
  canvas.width = image.width;
  canvas.height = image.height;
  canvas.getContext("2d")?.putImageData(new ImageData(pixels as Uint8ClampedArray<ArrayBuffer>, image.width, image.height), 0, 0);
  return true;
}

/**
 * Shows `source` in `target`, `shown` CSS px: below one device pixel per picture pixel it is scaled
 * down smoothly (area-averaged, as the canvas look), so no dot pattern aliases; above it the
 * picture's own pixels, `pixelated` for the dots.
 */
function present(target: HTMLCanvasElement | null, source: HTMLCanvasElement, shown: { width: number; height: number }, sharp: boolean) {
  if (!target || source.width === 0 || source.height === 0) return;
  const dpr = window.devicePixelRatio || 1;
  const smooth = needsSmoothing(source.width, shown.width, dpr);
  const width = smooth ? Math.max(1, Math.round(shown.width * dpr)) : source.width;
  const height = smooth ? Math.max(1, Math.round(shown.height * dpr)) : source.height;
  target.width = width;
  target.height = height;
  const context = target.getContext("2d");
  if (!context) return;
  context.imageSmoothingEnabled = smooth || !sharp;
  context.imageSmoothingQuality = "high";
  context.drawImage(source, 0, 0, width, height);
  target.style.imageRendering = sharp && !smooth ? "pixelated" : "auto";
}

function Tag({ side, children }: { side: "left" | "right"; children: string }) {
  return (
    <span
      className={`pointer-events-none absolute top-2 ${side === "left" ? "left-2" : "right-2"} z-10 flex h-5 items-center rounded border border-[var(--lane-line)] bg-[var(--toolbar-bg)] px-1.5 text-xs text-[var(--toolbar-icon)]`}
    >
      {children}
    </span>
  );
}

interface PicturesProps {
  api: BeforeAfterApi;
  view: BeforeAfterView;
  onZoom: (percent: number) => void;
}

/**
 * The two halves of the Before and after pane (`specs/0135-image-optimisation` criteria 11 to 13;
 * `ux-notes.md` §7): the original and the dots of the job, in Split (one picture, a divider) or
 * Side by side, one zoom and one pan for both. Both halves take the picture's placed size, so a
 * reduced live picture does not shrink them; 100 % is the document's 100 % (one dot is Interval mm
 * wide at 96 px per inch, criterion 12); the pane opens fitted. Ctrl and the wheel zoom (1.25 a
 * notch), the wheel and a drag pan.
 */
export function BeforeAfterPictures({ api, view, onZoom }: PicturesProps) {
  const before = useRef<HTMLCanvasElement | null>(null);
  const beforeSide = useRef<HTMLCanvasElement | null>(null);
  const after = useRef<HTMLCanvasElement | null>(null);
  const afterSide = useRef<HTMLCanvasElement | null>(null);
  const box = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  const [zoom, setZoom] = useState<number | null>(null);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [divider, setDivider] = useState(50);
  const drag = useRef<{ x: number; y: number; pan: { x: number; y: number } } | null>(null);
  const { pixels } = api;
  // The pictures at their own size; the visible canvases show them at the zoom.
  const beforeSource = useRef<HTMLCanvasElement | null>(null);
  const afterSource = useRef<HTMLCanvasElement | null>(null);
  const source = (ref: { current: HTMLCanvasElement | null }) => {
    ref.current ??= document.createElement("canvas");
    return ref.current;
  };

  const side = view.view === "side_by_side";
  // The size the pane fits and shows: the picture's placed size in mm, whatever the pixel count of
  // either half (a reduced live picture during a Strength drag keeps the same size).
  const frame = view.before ? sizeAt100(placedSizeMm(view.before.placement)) : null;
  const panes = side ? 2 : 1;
  const viewport = { width: size.width / panes, height: size.height };
  const fitted = frame ? fittedZoom(viewport, frame) : 1;
  const z = zoom ?? fitted;
  const shown = { width: frame ? frame.width * z : 0, height: frame ? frame.height * z : 0 };
  const showAll = useCallback(() => {
    present(before.current, source(beforeSource), shown, false);
    present(beforeSide.current, source(beforeSource), shown, false);
    present(after.current, source(afterSource), shown, true);
    present(afterSide.current, source(afterSource), shown, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [shown.width, shown.height]);
  useLayoutEffect(showAll, [showAll, side]);

  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return undefined;
    const read = () => setSize({ width: el.clientWidth, height: el.clientHeight });
    read();
    const observer = new ResizeObserver(read);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  useEffect(() => onZoom(Math.round(z * 100)), [z, onZoom]);

  // The original: read again when its identity changes.
  const beforeKey = `${view.subject}|${JSON.stringify(view.before)}|${side}`;
  useEffect(() => {
    if (!view.before) return;
    if (!drawPixels(source(beforeSource), view.before, pixels("before"))) return;
    present(before.current, source(beforeSource), shown, false);
    present(beforeSide.current, source(beforeSource), shown, false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [beforeKey, pixels]);

  // The dots: read again after a sync, at most every 80 ms (the job syncs each frame).
  const lastDraw = useRef(0);
  const shownRef = useRef(shown);
  shownRef.current = shown;
  const afterKey = `${JSON.stringify(view.after)}|${side}`;
  useEffect(() => {
    const image = view.after?.image;
    if (!image) return undefined;
    const draw = () => {
      lastDraw.current = performance.now();
      if (!drawPixels(source(afterSource), image, pixels("after"))) return;
      present(after.current, source(afterSource), shownRef.current, true);
      present(afterSide.current, source(afterSource), shownRef.current, true);
    };
    const wait = REDRAW_MS - (performance.now() - lastDraw.current);
    if (wait <= 0) {
      draw();
      return undefined;
    }
    const timer = window.setTimeout(draw, wait);
    return () => window.clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [api.syncRevision, afterKey, pixels]);

  const onWheel = useCallback(
    (event: WheelEvent<HTMLDivElement>) => {
      if (event.ctrlKey || event.metaKey) {
        const rect = event.currentTarget.getBoundingClientRect();
        const centre = { x: rect.width / 2, y: rect.height / 2 };
        const at = { x: event.clientX - rect.left - centre.x, y: event.clientY - rect.top - centre.y };
        const next = wheelZoom(z, event.deltaY, event.deltaMode);
        setPan(zoomedAbout(pan, at, z, next));
        setZoom(next);
      } else {
        setPan({ x: pan.x - event.deltaX, y: pan.y - event.deltaY });
      }
    },
    [pan, z],
  );

  if (view.empty !== null || !view.before) {
    return (
      <div ref={box} className="flex flex-1 items-center justify-center text-sm text-[var(--panel-muted-fg)]">
        {view.empty ?? "No picture selected."}
      </div>
    );
  }
  const place = { width: shown.width, height: shown.height, transform: `translate(${pan.x}px, ${pan.y}px)` };
  const canvasLook = { width: "100%", height: "100%", position: "absolute", left: 0, top: 0 } as const;

  const down = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || (event.target as HTMLElement).closest("[data-divider]")) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { x: event.clientX, y: event.clientY, pan };
  };
  const move = (event: ReactPointerEvent<HTMLDivElement>) => {
    const start = drag.current;
    if (start) setPan({ x: start.pan.x + event.clientX - start.x, y: start.pan.y + event.clientY - start.y });
  };
  const up = () => {
    drag.current = null;
  };
  const dividerKey = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.key === "ArrowLeft" ? -1 : event.key === "ArrowRight" ? 1 : 0;
    if (step === 0) return;
    event.preventDefault();
    setDivider((now) => Math.min(100, Math.max(0, now + step * DIVIDER_STEP * (event.shiftKey ? 10 : 1))));
  };
  const moveDivider = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.buttons === 0) return;
    const rect = (event.currentTarget.parentElement as HTMLElement).getBoundingClientRect();
    setDivider(Math.min(100, Math.max(0, ((event.clientX - rect.left) / rect.width) * 100)));
  };

  const halfTag = view.after?.reduced ? "After · reduced" : "After";
  // A picture in no Image operation has no dots: the After half says so instead of a white field.
  const noDots = view.after === null ? (
    <span data-pane-after-empty="" className="absolute inset-0 flex items-center justify-center p-2 text-center text-xs text-[var(--panel-muted-fg)]">
      Not engraved yet: the picture is in no Image operation.
    </span>
  ) : null;
  return (
    <div ref={box} className="relative flex min-h-0 flex-1 overflow-hidden" data-pane-pictures={view.view}>
      {side ? (
        <>
          <div className="relative flex min-w-0 flex-1 items-center justify-center overflow-hidden" onWheel={onWheel} onPointerDown={down} onPointerMove={move} onPointerUp={up} style={{ touchAction: "none" }}>
            <Tag side="left">Before</Tag>
            <div className="relative shrink-0" style={place}>
              <canvas ref={beforeSide} data-pane-half="before" style={canvasLook} />
            </div>
          </div>
          <div aria-hidden className="w-px shrink-0" style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }} />
          <div className="relative flex min-w-0 flex-1 items-center justify-center overflow-hidden" onWheel={onWheel} onPointerDown={down} onPointerMove={move} onPointerUp={up} style={{ touchAction: "none" }}>
            <Tag side="left">{halfTag}</Tag>
            <div className="relative shrink-0 bg-white" style={place}>
              <canvas ref={afterSide} data-pane-half="after" style={canvasLook} />
              {noDots}
            </div>
          </div>
        </>
      ) : (
        <div className="relative flex min-w-0 flex-1 items-center justify-center overflow-hidden" onWheel={onWheel} onPointerDown={down} onPointerMove={move} onPointerUp={up} style={{ touchAction: "none" }}>
          <Tag side="left">Before</Tag>
          <Tag side="right">{halfTag}</Tag>
          <div className="relative shrink-0" style={place}>
            <canvas ref={before} data-pane-half="before" style={canvasLook} />
            <div className="absolute inset-0 bg-white" style={{ clipPath: `inset(0 0 0 ${divider}%)` }}>
              <canvas ref={after} data-pane-half="after" style={canvasLook} />
              {noDots}
            </div>
            <div
              role="separator"
              data-divider=""
              aria-orientation="vertical"
              aria-label="Before and after divider"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(divider)}
              tabIndex={0}
              className="group absolute top-0 bottom-0 flex w-[7px] -translate-x-1/2 cursor-col-resize justify-center outline-none"
              style={{ left: `${divider}%` }}
              onKeyDown={dividerKey}
              onPointerDown={(event) => {
                event.stopPropagation();
                event.currentTarget.setPointerCapture(event.pointerId);
              }}
              onPointerMove={moveDivider}
            >
              <span aria-hidden className="h-full w-px group-hover:w-[3px] group-focus-visible:w-[3px] group-active:w-[3px] group-hover:bg-[var(--editor-accent)] group-focus-visible:bg-[var(--editor-accent)] group-active:bg-[var(--editor-accent)]" style={{ background: "color-mix(in srgb, var(--toolbar-icon) 60%, transparent)" }} />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
