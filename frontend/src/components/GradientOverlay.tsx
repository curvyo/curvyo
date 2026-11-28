import { useId } from "react";

import { GradientEntryBox } from "@/components/GradientEntryBox";
import type { GradientApi } from "@/hooks/useGradient";
import { stopColour } from "@/lib/gradientText";
import type { GradientHandleView, GradientHandles } from "@/lib/gradientView";
import { cssColor } from "@/lib/styleColor";

const ACCENT = "var(--editor-accent)";
const CASING = "var(--selection-casing, #fff)";
/** The hover halo: `--hover-box`, the accent at 65 %. */
const HOVER = "color-mix(in srgb, var(--editor-accent) 65%, transparent)";

interface Props {
  gradient: GradientApi;
  containerRef: React.RefObject<HTMLDivElement | null>;
}

/** The shape of one handle: a square for Start and the centre, a circle for the rest. */
function Shape({ handle, grow, ...rest }: { handle: GradientHandleView; grow: number } & React.SVGProps<SVGElement>) {
  const { x, y } = handle;
  if (handle.kind === "start" || handle.kind === "centre") {
    const half = 5.5 + grow;
    return <rect x={x - half} y={y - half} width={half * 2} height={half * 2} rx={2 + grow} {...(rest as React.SVGProps<SVGRectElement>)} />;
  }
  const r = (handle.kind === "stop" ? 4 : 6) + grow;
  return <circle cx={x} cy={y} r={r} {...(rest as React.SVGProps<SVGCircleElement>)} />;
}

function Handle({ handle, checker }: { handle: GradientHandleView; checker: string }) {
  const colour = stopColour(handle.hex);
  return (
    <g data-gradient-handle={handle.kind} data-stop={handle.stop} data-active={handle.active ? "" : undefined}>
      {(handle.active || handle.hovered) && (
        <Shape handle={handle} grow={3} fill="none" stroke={handle.active ? ACCENT : HOVER} strokeWidth={2} />
      )}
      <Shape handle={handle} grow={0} fill="none" stroke={CASING} strokeWidth={3.5} />
      {colour && colour.opacity < 100 && <Shape handle={handle} grow={0} fill={`url(#${checker})`} />}
      <Shape handle={handle} grow={0} fill={colour ? cssColor(colour.rgb, colour.opacity) : "none"} stroke={ACCENT} strokeWidth={1.5} />
    </g>
  );
}

/** The line, the Radial circle and the handles (UX notes §6.2). */
function Marks({ handles }: { handles: GradientHandles }) {
  const checker = useId();
  const [x1, y1, x2, y2] = handles.line;
  const ordered = [...handles.handles].sort((a, b) => Number(a.active) - Number(b.active));
  const [cx, cy, ax, ay, bx, by] = handles.ellipse ?? [0, 0, 1, 0, 0, 1];
  return (
    <svg aria-hidden className="absolute inset-0 size-full overflow-visible" data-gradient-handles="">
      <defs>
        <pattern id={checker} width="8" height="8" patternUnits="userSpaceOnUse">
          <rect width="8" height="8" fill="var(--checker-a)" />
          <rect width="4" height="4" fill="var(--checker-b)" />
          <rect x="4" y="4" width="4" height="4" fill="var(--checker-b)" />
        </pattern>
      </defs>
      {handles.ellipse && (
        <g transform={`matrix(${ax} ${ay} ${bx} ${by} ${cx} ${cy})`}>
          <circle r="1" fill="none" stroke={CASING} strokeWidth={3} vectorEffect="non-scaling-stroke" />
          <circle r="1" fill="none" stroke={ACCENT} strokeWidth={1} strokeDasharray="4 3" vectorEffect="non-scaling-stroke" />
        </g>
      )}
      <line x1={x1} y1={y1} x2={x2} y2={y2} stroke={CASING} strokeWidth={3} />
      <line x1={x1} y1={y1} x2={x2} y2={y2} stroke={ACCENT} strokeWidth={1} />
      {ordered.map((handle) => (
        <Handle key={`${handle.kind}:${handle.stop}`} handle={handle} checker={checker} />
      ))}
    </svg>
  );
}

/**
 * The gradient handles over the canvas (`specs/0124-gradient-fill` criteria 6 and 7, UX notes §6,
 * §7): the line, the Radial circle, the handles in the stops' own colours, the readout of a drag
 * and the entry box. Placed by the session in the last frame; every press, drag and key is the
 * canvas's, so this layer takes no pointer except in the entry box.
 */
export function GradientOverlay({ gradient, containerRef }: Props) {
  const { handles } = gradient;
  if (!handles) {
    return null;
  }
  return (
    <div className="pointer-events-none absolute inset-0 z-10 overflow-hidden" data-gradient-overlay="">
      <Marks handles={handles} />
      {handles.readout && (
        <div
          aria-hidden
          data-gradient-readout=""
          className="absolute rounded-[5px] px-1.5 py-0.5 text-xs whitespace-nowrap tabular-nums"
          style={{
            left: handles.readout.x + 12,
            top: handles.readout.y - 28,
            background: "var(--toolbar-bg)",
            color: "var(--toolbar-icon)",
            border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
          }}
        >
          {handles.readout.text}
        </div>
      )}
      {handles.entry && (
        <GradientEntryBox
          key={`${handles.entry.kind}:${handles.entry.stop}:${handles.entry.field}`}
          entry={handles.entry}
          gradient={gradient}
          containerRef={containerRef}
        />
      )}
    </div>
  );
}
