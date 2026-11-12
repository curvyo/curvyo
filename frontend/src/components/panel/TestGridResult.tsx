import { Check } from "lucide-react";
import { useRef, useState } from "react";

import type { TestGridApi } from "@/hooks/useTestGrid";
import { CELL_ROW, HEADER_WIDTH, cellName, cellWidth, markStatus, moveCell, powerLabel, sparseHeader } from "@/lib/testGridResult";
import type { CellPlace, TestGridView } from "@/lib/testGridView";

const same = (a: CellPlace | null, column: number, row: number) => a !== null && a.column === column && a.row === row;

/**
 * The result table of the Grid block (`specs/0028-material-test-library` `ux-notes.md` 7.1,
 * criteria 12 to 14): power across, speed down, one Tab stop with roving focus, a press marks the
 * cell Worked or removes the mark. No colour marks a cell: the square's ink and the check do.
 */
export function TestGridResult({ view, api, onMarkChanged }: { view: TestGridView; api: TestGridApi; onMarkChanged: () => void }) {
  const columns = view.powers.length;
  const rows = view.speeds.length;
  const marked = view.worked?.cell ?? null;
  const [stop, setStop] = useState<CellPlace>(marked ?? { column: 0, row: 0 });
  const grid = useRef<HTMLDivElement>(null);
  const width = cellWidth(columns);
  const sparse = sparseHeader(view.powers, width);
  const [status, setStatus] = useState("");
  const at = { column: Math.min(stop.column, columns - 1), row: Math.min(stop.row, rows - 1) };

  const focusCell = (place: CellPlace) => {
    setStop(place);
    grid.current?.querySelector<HTMLButtonElement>(`[data-cell="${place.column},${place.row}"]`)?.focus();
  };
  const press = (column: number, row: number) => {
    setStop({ column, row });
    // The mark changes: a draft of Save as material is dropped by the block (ux-notes 9).
    onMarkChanged();
    const marking = !same(marked, column, row);
    if (api.pressCell(column, row)) setStatus(markStatus(view.powers[column] ?? 0, view.speeds[row] ?? 0, marking));
  };

  return (
    <>
      <div ref={grid} role="grid" aria-label={`Result of ${view.name}`} aria-rowcount={rows + 1} aria-colcount={columns + 1} className="flex flex-col text-[11px] tabular-nums text-[var(--panel-muted-fg)]">
        <div aria-hidden className="flex h-4 items-center">
          <span style={{ width: HEADER_WIDTH }} className="pr-1 text-right">mm/min</span>
          <span className="min-w-0 flex-1 text-center">Power, %</span>
        </div>
        <div role="row" className="flex h-4 items-center">
          <span aria-hidden style={{ width: HEADER_WIDTH }} />
          {view.powers.map((power, column) => (
            <span key={column} role="columnheader" style={{ width }} className="whitespace-nowrap text-center">
              {/* Crowded labels: every other one is read, not drawn, so none runs into the next. */}
              <span className={sparse && column % 2 === 1 ? "sr-only" : ""}>{powerLabel(power)}</span>
            </span>
          ))}
        </div>
        {view.speeds.map((speed, row) => (
          <div key={row} role="row" style={{ height: CELL_ROW }} className="flex items-center">
            <span role="rowheader" style={{ width: HEADER_WIDTH }} className="pr-1 text-right">
              {speed}
            </span>
            {view.powers.map((power, column) => {
              const isMarked = same(marked, column, row);
              const isSelected = same(view.selectedCell, column, row);
              return (
                <div key={column} role="gridcell" style={{ width, height: CELL_ROW }} className="flex items-center justify-center">
                  <button
                    type="button"
                    data-cell={`${column},${row}`}
                    aria-pressed={isMarked}
                    aria-label={cellName(power, speed, isMarked)}
                    tabIndex={same(at, column, row) ? 0 : -1}
                    onFocus={() => setStop({ column, row })}
                    onClick={() => press(column, row)}
                    onKeyDown={(event) => {
                      const next = moveCell({ column, row }, event.key, event.ctrlKey || event.metaKey, columns, rows);
                      if (next === null) return;
                      event.preventDefault();
                      focusCell(next);
                    }}
                    style={{ width, height: CELL_ROW }}
                    className={`relative flex items-center justify-center text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${isSelected ? "bg-[var(--value-fill)]" : ""}`}
                  >
                    <span aria-hidden className={`block h-3 w-3 border-[1.5px] ${isMarked ? "border-[var(--toolbar-icon)]" : "border-[var(--lane-line)]"}`} />
                    {isMarked ? <Check size={16} strokeWidth={2} aria-hidden className="absolute" /> : null}
                  </button>
                </div>
              );
            })}
          </div>
        ))}
        <div className="mt-1 flex h-6 items-center">
          {view.worked ? (
            <span className="flex items-center gap-1 text-[13px] text-[var(--toolbar-icon)]">
              <Check size={16} strokeWidth={2} aria-hidden className="shrink-0" />
              {view.worked.text}
            </span>
          ) : (
            <span className="text-xs">No cell marked</span>
          )}
        </div>
      </div>
      <div role="status" aria-atomic="true" className="sr-only">
        {status}
      </div>
    </>
  );
}
