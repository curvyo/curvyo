import { useRef, useState } from "react";
import type { KeyboardEvent, PointerEvent as ReactPointerEvent } from "react";

import type { StepRow } from "@/lib/adjustmentsView";
import {
  LEVELS_HANDLES as HANDLES,
  LEVELS_BARS_HEIGHT as BARS_HEIGHT,
  LEVELS_MARKS_HEIGHT as MARKS_HEIGHT,
  LEVELS_WIDTH as WIDTH,
  barsPath,
  levelsMarks,
  levelsValueAt,
  levelsValueStepped,
} from "@/lib/levelsGraph";

function field(row: StepRow, key: string): number {
  return row.fields.find((f) => f.key === key)?.value ?? 0;
}

/**
 * The histogram of a Levels row's input (`specs/0135` criteria 5, 16, `ux-notes.md` §3.8): 256
 * hairline bars in muted ink on the field ground, a 25 % baseline, and under it the Black, Gamma
 * and White handles (Black filled with the accent, the others white with an accent ring). One Tab
 * stop: Left and Right pick a handle, Up and Down move it 1 % (Shift 10 %), each one step; a drag
 * of a handle moves it under the pointer and writes once at release (one step). The fields beside
 * it are the exact route.
 */
export function LevelsGraph({ row, onSet }: { row: StepRow; onSet: (key: string, value: number) => void }) {
  const [picked, setPicked] = useState<0 | 1 | 2>(0);
  const [drag, setDrag] = useState<{ which: 0 | 1 | 2; at: number } | null>(null);
  const svg = useRef<SVGSVGElement>(null);
  if (!row.histogram) return null;
  const black = field(row, "black_pct");
  const white = field(row, "white_pct");
  const gamma = field(row, "gamma_centi");
  const marks = levelsMarks(black, white, gamma);
  if (drag) marks[drag.which] = drag.at;
  const atPointer = (event: ReactPointerEvent<SVGSVGElement>) => {
    const rect = event.currentTarget.getBoundingClientRect();
    return (event.clientX - rect.left) / Math.max(1, rect.width);
  };
  const down = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (event.button !== 0) return;
    const at = atPointer(event);
    const near = marks.map((m) => Math.abs(m - at));
    const which = near.indexOf(Math.min(...near)) as 0 | 1 | 2;
    // A press on a handle: within 8 px of its mark.
    if (Math.abs(marks[which] - at) * WIDTH > 8) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    svg.current?.focus();
    setPicked(which);
    setDrag({ which, at: marks[which] });
  };
  const move = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (drag) setDrag({ which: drag.which, at: Math.min(1, Math.max(0, atPointer(event))) });
  };
  const up = () => {
    if (!drag) return;
    const value = levelsValueAt(drag.which, drag.at, black, white);
    setDrag(null);
    onSet(HANDLES[drag.which].key, value);
  };
  const key = (event: KeyboardEvent<SVGSVGElement>) => {
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      setPicked((now) => Math.min(2, Math.max(0, now + (event.key === "ArrowLeft" ? -1 : 1))) as 0 | 1 | 2);
    } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      const step = (event.key === "ArrowUp" ? 1 : -1) * (event.shiftKey ? 10 : 1);
      onSet(HANDLES[picked].key, levelsValueStepped(picked, step, black, white, gamma));
    }
  };
  const mark = (at: number, filled: boolean, which: 0 | 1 | 2) => {
    const x = Math.min(WIDTH - 5, Math.max(5, at * WIDTH));
    return (
      <path
        key={HANDLES[which].key}
        data-levels-mark={["black", "gamma", "white"][which]}
        d={`M${x.toFixed(2)} ${BARS_HEIGHT + 1}l5 9h-10z`}
        fill={filled ? "var(--editor-accent)" : "var(--panel-bg)"}
        stroke={which === picked ? "var(--toolbar-icon)" : "var(--editor-accent)"}
        strokeWidth={filled ? (which === picked ? 1 : 0) : 1.25}
      />
    );
  };
  const values = [black, gamma, white];
  return (
    <svg
      ref={svg}
      data-levels-graph=""
      role="group"
      tabIndex={0}
      aria-label={`${row.name} points, ${HANDLES[picked].name} ${values[picked]}`}
      width={WIDTH}
      height={BARS_HEIGHT + MARKS_HEIGHT}
      viewBox={`0 0 ${WIDTH} ${BARS_HEIGHT + MARKS_HEIGHT}`}
      className="block rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      style={{ touchAction: "none" }}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={() => setDrag(null)}
      onKeyDown={key}
    >
      <rect x={0.5} y={0.5} width={WIDTH - 1} height={BARS_HEIGHT - 1} fill="var(--panel-bg)" stroke="var(--field-border)" />
      <path d={barsPath(row.histogram)} stroke="var(--panel-muted-fg)" strokeWidth={0.75} fill="none" />
      <path
        d={`M1 ${BARS_HEIGHT - 1.5}H${WIDTH - 1}`}
        stroke="color-mix(in srgb, var(--toolbar-icon) 25%, transparent)"
        strokeWidth={1}
      />
      {mark(marks[0], true, 0)}
      {mark(marks[1], false, 1)}
      {mark(marks[2], false, 2)}
    </svg>
  );
}
