import { LineSquiggle } from "lucide-react";
import { useReducer, useRef, type ReactNode } from "react";

import { ToggleGroup } from "@/components/ui/toggle-group";
import type { EditorSession } from "@/hooks/useEditorSession";
import { PEN_MODES, type PenModeCode, showsSmoothing } from "@/lib/penModes";

/** A mode glyph in the lucide grid (24 units, 2 px stroke), drawn at 18 px. */
function Glyph({ children }: { children: ReactNode }) {
  return (
    <svg width={18} height={18} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
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
  freehand: <LineSquiggle size={18} />,
};

const OPTIONS = PEN_MODES.map((mode) => ({ ...mode, icon: ICONS[mode.value] }));

/** The Pen bar (`specs/0110` criterion 1): one icon button per mode, Smoothing for Freehand. A
 * refused mode change shows beside the pointer in the refusal look (UX review U8). */
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
    // A mode change ends the Spiro notice (UX review U1); the session says so on the next frame.
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
      className="pointer-events-auto flex h-10 min-w-0 shrink-0 items-center gap-3 rounded-lg px-1 text-sm"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <ToggleGroup
        label="Pen mode"
        options={OPTIONS}
        value={mode}
        onChange={select}
        itemWidth={36}
        itemHeight={32}
        tooltipSide="bottom"
        onReturnFocus={() => editor.containerRef.current?.focus()}
      />
      {showsSmoothing(mode) ? (
        <label className="flex items-center gap-2 pr-2 text-[var(--toolbar-icon)]">
          Smoothing
          <input
            aria-label="Freehand smoothing"
            type="range"
            min="0"
            max="100"
            value={smoothing}
            onChange={(event) => {
              if (editor.getSession()?.set_pen_smoothing(Number(event.target.value) / 100)) refresh();
            }}
          />
          <span className="w-9 tabular-nums">{smoothing}%</span>
        </label>
      ) : null}
    </div>
  );
}
