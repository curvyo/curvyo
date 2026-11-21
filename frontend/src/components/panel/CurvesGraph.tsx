import { useRef, useState } from "react";
import type { KeyboardEvent, PointerEvent as ReactPointerEvent } from "react";

import { CURVE_SIDE as SIDE, curvePath } from "@/lib/curvePath";

/** A press within this many px of a point picks it. */
const PICK_PX = 6;

/** What the graph asks of its row: the session calls with Input and Output in whole %. */
export interface CurveActions {
  /** `true` when the point was added. */
  add: (input: number, output: number) => boolean;
  move: (point: number, input: number, output: number) => void;
  remove: (point: number) => void;
}

function clampPct(value: number): number {
  return Math.min(100, Math.max(0, Math.round(value)));
}

/**
 * The Curves graph of a Curves row (`specs/0135` criterion 5, `ux-notes.md` §3.8): 120 x 120, a
 * quarter grid in ink 18 %, the identity dashed in muted ink, the curve 1.5 px ink, points as 8 px
 * squares (white with an accent ring, the selected one filled). A click on the curve adds a point
 * (at most 8) and picks it, a drag moves one and writes once at release, the end points move
 * only vertically. One Tab stop: Left and Right pick a point, Up and Down move it 1 % (Shift 10 %), Delete removes
 * an inner one; each one step. The In and Out fields beside it are the exact route.
 */
export function CurvesGraph({
  name,
  points,
  picked,
  onPick,
  actions,
}: {
  name: string;
  points: [number, number][];
  picked: number;
  onPick: (point: number) => void;
  actions: CurveActions;
}) {
  const [drag, setDrag] = useState<{ point: number; at: [number, number] } | null>(null);
  const svg = useRef<SVGSVGElement>(null);
  const shown = points.map((p, i) => (drag && drag.point === i ? drag.at : p));
  const last = points.length - 1;
  const at = (event: ReactPointerEvent<SVGSVGElement>): [number, number] => {
    const rect = event.currentTarget.getBoundingClientRect();
    const x = ((event.clientX - rect.left) / Math.max(1, rect.width)) * 100;
    const y = 100 - ((event.clientY - rect.top) / Math.max(1, rect.height)) * 100;
    return [clampPct(x), clampPct(y)];
  };
  const down = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    svg.current?.focus();
    const [x, y] = at(event);
    const near = points.findIndex((p) => Math.hypot(p[0] - x, p[1] - y) * (SIDE / 100) <= PICK_PX);
    if (near < 0) {
      // The new point is picked, so In and Out show it and the arrows move it.
      if (actions.add(x, y)) onPick(points.filter((p) => p[0] < x).length);
      return;
    }
    event.currentTarget.setPointerCapture(event.pointerId);
    onPick(near);
    setDrag({ point: near, at: points[near] });
  };
  const move = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (!drag) return;
    const [x, y] = at(event);
    const lo = drag.point === 0 ? 0 : points[drag.point - 1][0] + 1;
    const hi = drag.point === last ? 100 : points[drag.point + 1][0] - 1;
    const input = drag.point === 0 || drag.point === last ? points[drag.point][0] : Math.min(hi, Math.max(lo, x));
    setDrag({ point: drag.point, at: [input, y] });
  };
  const up = () => {
    if (!drag) return;
    const [x, y] = drag.at;
    setDrag(null);
    if (x !== points[drag.point][0] || y !== points[drag.point][1]) actions.move(drag.point, x, y);
  };
  const key = (event: KeyboardEvent<SVGSVGElement>) => {
    const p = points[picked];
    if (!p) return;
    const step = event.shiftKey ? 10 : 1;
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      onPick(Math.min(last, Math.max(0, picked + (event.key === "ArrowLeft" ? -1 : 1))));
    } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      actions.move(picked, p[0], clampPct(p[1] + (event.key === "ArrowUp" ? step : -step)));
    } else if (event.key === "Delete" && picked > 0 && picked < last) {
      event.preventDefault();
      actions.remove(picked);
      onPick(picked - 1);
    }
  };
  const px = (v: number) => (v / 100) * SIDE;
  return (
    <svg
      ref={svg}
      role="group"
      aria-label={`${name} graph, point ${picked + 1} of ${points.length}`}
      tabIndex={0}
      width={SIDE}
      height={SIDE}
      viewBox={`-4 -4 ${SIDE + 8} ${SIDE + 8}`}
      data-curves-graph=""
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={() => setDrag(null)}
      onKeyDown={key}
      className="shrink-0 touch-none rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
    >
      <rect x={0} y={0} width={SIDE} height={SIDE} fill="var(--field-bg, white)" stroke="var(--field-border)" strokeWidth={1} />
      {[0.25, 0.5, 0.75].map((f) => (
        <path
          key={f}
          d={`M${SIDE * f} 0V${SIDE}M0 ${SIDE * f}H${SIDE}`}
          stroke="color-mix(in srgb, var(--toolbar-icon) 18%, transparent)"
          strokeWidth={1}
        />
      ))}
      <path d={`M0 ${SIDE}L${SIDE} 0`} stroke="var(--panel-muted-fg)" strokeWidth={1} strokeDasharray="3 3" />
      <path d={curvePath(shown)} fill="none" stroke="var(--toolbar-icon)" strokeWidth={1.5} />
      {shown.map(([x, y], i) => (
        <rect
          key={i}
          data-curve-point={i}
          x={px(x) - 4}
          y={SIDE - px(y) - 4}
          width={8}
          height={8}
          fill={i === picked ? "var(--editor-accent)" : "white"}
          stroke="var(--editor-accent)"
          strokeWidth={1.25}
        />
      ))}
    </svg>
  );
}
