import { Check } from "lucide-react";
import { useRef, useState } from "react";

import {
  CELL_ROW,
  HEADER_WIDTH,
  bestStatus,
  cellName,
  cellWidth,
  moveCell,
  sparseHeader,
} from "@/lib/testGridResult";
import type { CellPlace, TestDetail } from "@/lib/materialsView";

const same = (a: CellPlace | null, column: number, row: number) => a !== null && a.column === column && a.row === row;

/**
 * The best-cell table of a test (`specs/0112` `ux-notes.md` 8.3): the columns' axis across, the
 * rows' down, one Tab stop with roving focus; a press, Space or Enter makes the cell the best
 * one. A press on the best cell does nothing (G14). No colour marks a cell: the frame and the
 * check do.
 */
export function JudgeTable({ test, onChoose }: { test: TestDetail; onChoose: (column: number, row: number) => void }) {
  const columns = test.columns.values.length;
  const rows = test.rows.values.length;
  const [stop, setStop] = useState<CellPlace>(test.best ?? { column: 0, row: 0 });
  const [status, setStatus] = useState("");
  const grid = useRef<HTMLDivElement>(null);
  const width = cellWidth(columns);
  const sparse = sparseHeader(test.columns.values, width);
  const at = { column: Math.min(stop.column, columns - 1), row: Math.min(stop.row, rows - 1) };

  const focusCell = (place: CellPlace) => {
    setStop(place);
    grid.current?.querySelector<HTMLButtonElement>(`[data-cell="${place.column},${place.row}"]`)?.focus();
  };
  const press = (column: number, row: number) => {
    setStop({ column, row });
    if (same(test.best, column, row)) return;
    onChoose(column, row);
    setStatus(bestStatus(test.columns, test.rows, { column, row }));
  };

  return (
    <>
      <div
        ref={grid}
        role="grid"
        aria-label={`Best cell of ${test.id}`}
        aria-rowcount={rows + 1}
        aria-colcount={columns + 1}
        className="flex flex-col text-[11px] tabular-nums text-[var(--panel-muted-fg)]"
      >
        <div aria-hidden className="flex h-4 items-center">
          <span style={{ width: HEADER_WIDTH }} className="pr-1 text-right">
            {test.rows.title}
          </span>
          <span className="min-w-0 flex-1 text-center">{test.columns.title}</span>
        </div>
        <div role="row" className="flex h-4 items-center">
          <span aria-hidden style={{ width: HEADER_WIDTH }} />
          {test.columns.values.map((value, column) => (
            <span key={column} role="columnheader" style={{ width }} className="whitespace-nowrap text-center">
              <span className={sparse && column % 2 === 1 ? "sr-only" : ""}>{value}</span>
            </span>
          ))}
        </div>
        {test.rows.values.map((value, row) => (
          <div key={row} role="row" style={{ height: CELL_ROW }} className="flex items-center">
            <span role="rowheader" style={{ width: HEADER_WIDTH }} className="pr-1 text-right">
              {value}
            </span>
            {test.columns.values.map((_, column) => {
              const best = same(test.best, column, row);
              return (
                <div key={column} role="gridcell" style={{ width, height: CELL_ROW }} className="flex items-center justify-center">
                  <button
                    type="button"
                    data-cell={`${column},${row}`}
                    aria-pressed={best}
                    aria-label={cellName(test.columns, test.rows, { column, row }, best)}
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
                    className="relative flex items-center justify-center text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
                  >
                    <span
                      aria-hidden
                      className={`block h-3 w-3 border-[1.5px] ${best ? "border-[var(--accent)] outline outline-[2px] outline-[var(--accent)]" : "border-[var(--lane-line)]"}`}
                    />
                    {best ? <Check size={16} strokeWidth={2} aria-hidden className="absolute" /> : null}
                  </button>
                </div>
              );
            })}
          </div>
        ))}
      </div>
      <div role="status" aria-atomic="true" aria-live="polite" className="sr-only">
        {status}
      </div>
    </>
  );
}
