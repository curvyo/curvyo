import { LineSquiggle } from "lucide-react";
import { useReducer, useRef, type ReactNode } from "react";

import { TraceField } from "@/components/TraceField";
import { ToggleGroup } from "@/components/ui/toggle-group";
import type { EditorSession } from "@/hooks/useEditorSession";
import { PEN_MODES, SMOOTHING_PARAM, type PenModeCode, showsSmoothing } from "@/lib/penModes";

/** A mode glyph in the lucide grid (24 units, 2 px stroke), drawn at 16 px like the other bars'
 * icons in their 28 px buttons. */
function Glyph({ children }: { children: ReactNode }) {
  return (
    <svg width={16} height={16} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {children}
    </svg>
  );
}

/** Bézier: a curve with a handle at its start (not the rail's pen nib, UX review U6). */
function BezierIcon() {
  return (
    <Glyph>
      <path d="M4 19C4 11 12 6 20 6" />
      <path d="M4 19V9M20 6h-8" strokeWidth={1.5} />
      <circle cx="4" cy="8" r="2" fill="currentColor" stroke="none" />
      <circle cx="11" cy="6" r="2" fill="currentColor" stroke="none" />
    </Glyph>
  );
}

/** Spiro: an S through three points. */
function SpiroIcon() {
  return (
    <Glyph>
      <path d="M18 5C10 3 6 8 12 12S14 21 6 19" />
      <circle cx="18" cy="5" r="2" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="2" fill="currentColor" stroke="none" />
      <circle cx="6" cy="19" r="2" fill="currentColor" stroke="none" />
    </Glyph>
  );
}

/** B-spline: a curve under its control polygon, whose square points sit off it. */
function BSplineIcon() {
  return (
    <Glyph>
      <path d="M4 19 7 5h10l3 14" strokeWidth={1} />
      <path d="M4 19C6 10 9 8 12 8s6 2 8 11" />
      <rect x="5.5" y="3.5" width="3" height="3" fill="currentColor" strokeWidth={1} />
      <rect x="15.5" y="3.5" width="3" height="3" fill="currentColor" strokeWidth={1} />
    </Glyph>
  );
}

/** Lines: a zigzag of straight segments. */
function LinesIcon() {
  return (
    <Glyph>
      <path d="M3 17 8 7l5 10 5-10 3 6" />
    </Glyph>
  );
}

const ICONS: Record<PenModeCode, ReactNode> = {
  bezier: <BezierIcon />,
  spiro: <SpiroIcon />,
  bspline: <BSplineIcon />,
  lines: <LinesIcon />,
  freehand: <LineSquiggle size={16} />,
};

const OPTIONS = PEN_MODES.map((mode) => ({ ...mode, icon: ICONS[mode.value] }));

/** The Pen bar (`specs/0110` criterion 1): one icon button per mode, Smoothing for Freehand. A
 * refused mode change shows beside the pointer in the refusal look (UX review U8). The bar, its
 * 28 px buttons and the Smoothing value field have the look of the other tool bars and of the
 * Style panel's Opacity (customer 2026-11-11). */
export function PenModeBar({ editor }: { editor: EditorSession }) {
  const [, refresh] = useReducer((n: number) => n + 1, 0);
  const pressed = useRef<{ x: number; y: number } | null>(null);
  const bar = useRef<HTMLDivElement>(null);
  const session = editor.getSession();
  const mode = (session?.pen_mode() ?? "bezier") as PenModeCode;
  const smoothing = Math.round((session?.pen_smoothing() ?? 0.4) * 100);
  const select = (next: PenModeCode) => {
    const current = editor.getSession();
    if (!current) return;
    // The session says why: an unfinished path, or a curve mode in a shared document.
    const refusal = current.pen_mode_refusal(next);
    if (refusal || !current.set_pen_mode(next)) {
      const box = bar.current?.getBoundingClientRect();
      const at = pressed.current ?? (box ? { x: box.left, y: box.bottom } : undefined);
      editor.showRefusal(refusal || "Finish the path first.", at);
      return;
    }
    if (next === "freehand") current.set_pen_smoothing(smoothing / 100);
    editor.sync();
    refresh();
  };
  return (
    <div
      ref={bar}
      role="toolbar"
      aria-label="Pen options"
      onPointerDown={(event) => {
        pressed.current = { x: event.clientX, y: event.clientY };
      }}
      onKeyDown={() => {
        pressed.current = null;
      }}
      className="pointer-events-auto flex h-9 min-w-0 shrink-0 items-center gap-3 rounded-lg px-2 text-sm"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <ToggleGroup
        label="Pen mode"
        options={OPTIONS}
        value={mode}
        onChange={select}
        itemWidth={28}
        itemHeight={28}
        bar
        tooltipSide="bottom"
        onReturnFocus={() => editor.containerRef.current?.focus()}
      />
      {showsSmoothing(mode) ? (
        <TraceField
          param={SMOOTHING_PARAM}
          name="Freehand smoothing"
          value={smoothing}
          onValue={(value) => {
            if (editor.getSession()?.set_pen_smoothing(value / 100)) refresh();
          }}
          onEndDrag={() => undefined}
          onCreate={() => undefined}
          onReturnFocus={() => editor.containerRef.current?.focus()}
        />
      ) : null}
    </div>
  );
}
