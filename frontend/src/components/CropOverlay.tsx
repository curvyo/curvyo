import { useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";

import type { CropBarApi } from "@/hooks/useCropBar";
import { CROP_GRIPS, type CropGrip, type CropPct, dragCrop, gripPoint, toScreen, toUnit } from "@/lib/cropBar";

const ACCENT = "var(--editor-accent)";
const CASING = "var(--selection-casing, #fff)";
/** The Trace preview wash: white at 65 % outside the kept part (`ux-notes.md` §3.9). */
const WASH = "rgba(255, 255, 255, 0.65)";
/** The Parameter handle's hover ground (`docs/design-system.md`, "Parameter handle"). */
const HOVER = "color-mix(in srgb, var(--editor-accent) 20%, white)";
/** The Parameter handle's hit-test radius in px: a 24 px target. */
const HIT_RADIUS = 12;

/** The whole working image drawn into the picture's frame on screen. */
function WholePicture({ api }: { api: CropBarApi }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const view = api.view;
  const width = view?.width ?? 0;
  const height = view?.height ?? 0;
  useEffect(() => {
    const target = canvas.current;
    const bytes = api.pixels();
    if (!target || !bytes || bytes.length !== width * height * 4) return;
    target.width = width;
    target.height = height;
    target.getContext("2d")?.putImageData(new ImageData(new Uint8ClampedArray(bytes), width, height), 0, 0);
  }, [api, width, height]);
  if (!view) return null;
  const [a, b, c, d, e, f] = view.quad;
  return (
    <canvas
      ref={canvas}
      aria-hidden
      data-crop-picture=""
      className="absolute left-0 top-0 origin-top-left"
      style={{
        width,
        height,
        transform: `matrix(${a / width}, ${b / width}, ${c / height}, ${d / height}, ${e}, ${f})`,
      }}
    />
  );
}

/**
 * The crop frame over the canvas while the Crop bar is open (`specs/0135` criterion 5,
 * `ux-notes.md` §3.9): the whole working image, the part outside the crop washed with white at
 * 65 %, the frame 1 px in the accent with the white casing, eight handles in the Parameter
 * handle look (a 24 px target). A corner drag moves two
 * sides (Shift keeps the ratio), a side drag one, a drag inside moves the frame; the frame
 * follows the object's rotation and skew. Each release is one step "Edit adjustments"; the fields
 * of the bar are the exact route.
 */
export function CropOverlay({ api }: { api: CropBarApi }) {
  const [drag, setDrag] = useState<{ grip: CropGrip; from: [number, number]; start: CropPct; now: CropPct } | null>(null);
  const [hover, setHover] = useState<CropGrip | null>(null);
  const view = api.view;
  if (!view) return null;
  const pct = drag?.now ?? view.pct;
  const q = view.quad;
  const unitAt = (event: ReactPointerEvent<Element>): [number, number] | null => {
    const host = (event.currentTarget as Element).closest("[data-crop-overlay]")?.getBoundingClientRect();
    if (!host) return null;
    return toUnit(q, event.clientX - host.left, event.clientY - host.top);
  };
  const down = (grip: CropGrip) => (event: ReactPointerEvent<Element>) => {
    if (event.button !== 0) return;
    const at = unitAt(event);
    if (!at) return;
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
    setDrag({ grip, from: at, start: view.pct, now: view.pct });
  };
  const move = (event: ReactPointerEvent<Element>) => {
    if (!drag) return;
    const at = unitAt(event);
    if (!at) return;
    setDrag({ ...drag, now: dragCrop(drag.start, drag.grip, at[0] - drag.from[0], at[1] - drag.from[1], event.shiftKey) });
  };
  const up = () => {
    if (!drag) return;
    const now = drag.now;
    setDrag(null);
    if (now.some((v, i) => v !== drag.start[i])) api.setRect(now);
  };
  const [l, t, w, h] = pct.map((p) => p / 100);
  const corners = (rect: [number, number, number, number]) =>
    [
      [rect[0], rect[1]],
      [rect[0] + rect[2], rect[1]],
      [rect[0] + rect[2], rect[1] + rect[3]],
      [rect[0], rect[1] + rect[3]],
    ]
      .map(([u, v]) => toScreen(q, u, v).join(" "))
      .join(" L");
  const whole = `M${corners([0, 0, 1, 1])}Z`;
  const kept = `M${corners([l, t, w, h])}Z`;
  return (
    <div className="absolute inset-0 z-10 overflow-hidden pointer-events-none" data-crop-overlay="">
      <WholePicture api={api} />
      <svg className="absolute inset-0 size-full overflow-visible" onPointerMove={move} onPointerUp={up} onPointerCancel={() => setDrag(null)}>
        <path d={`${whole} ${kept}`} fill={WASH} fillRule="evenodd" />
        <path
          d={kept}
          fill="transparent"
          className="pointer-events-auto cursor-move"
          data-crop-grip="inside"
          onPointerDown={down("inside")}
        />
        <path d={kept} fill="none" stroke={CASING} strokeWidth={3} />
        <path d={kept} fill="none" stroke={ACCENT} strokeWidth={1} />
        {CROP_GRIPS.map((grip) => {
          const [x, y] = toScreen(q, ...gripPoint(pct, grip));
          const active = drag?.grip === grip;
          const ground = active ? ACCENT : hover === grip ? HOVER : "white";
          return (
            <g
              key={grip}
              data-crop-grip={grip}
              className="pointer-events-auto"
              style={{ cursor: grip === "tl" || grip === "br" ? "nwse-resize" : grip === "tr" || grip === "bl" ? "nesw-resize" : grip === "t" || grip === "b" ? "ns-resize" : "ew-resize" }}
              onPointerDown={down(grip)}
              onPointerEnter={() => setHover(grip)}
              onPointerLeave={() => setHover((h) => (h === grip ? null : h))}
            >
              <circle cx={x} cy={y} r={HIT_RADIUS} fill="transparent" />
              <circle cx={x} cy={y} r={5} fill={ground} stroke={ACCENT} strokeWidth={1.5} />
              <circle cx={x} cy={y} r={2} fill={active ? "white" : ACCENT} />
            </g>
          );
        })}
      </svg>
    </div>
  );
}
