import { useRef, useState } from "react";

import { PickGlyph } from "@/components/panel/PickGlyph";
import {
  CELL_ROW,
  HEADER_WIDTH,
  cellName,
  cellWidth,
  kindsAt,
  markingText,
  moveCell,
  pickStatus,
  sparseHeader,
} from "@/lib/testGridResult";
import type { CellPlace, TestDetail } from "@/lib/materialsView";
import { PICK_COLOUR, PICK_KINDS, type PickKind } from "@/lib/picks";
import { Tooltip } from "@/components/ui/tooltip";

/** The corner of the inner box each pick's glyph sits in when a cell carries several (criterion 6). */
const QUADRANT: Record<PickKind, string> = {
  best: "left-0 top-0",
  second_best: "right-0 top-0",
  fastest: "bottom-0 left-0",
  deepest: "bottom-0 right-0",
};

/** The frame and glyphs of a cell that carries picks (`specs/0146` `ux-notes.md` 4.3). */
function CellMark({ kinds, width }: { kinds: readonly PickKind[]; width: number }) {
  const first = kinds[0];
  if (first === undefined) {
    return <span aria-hidden className="block h-3 w-3 border-[1.5px] border-[var(--lane-line)]" />;
  }
  const outer = Math.min(width - 4, 20);
  const several = kinds.length > 1 && width >= 24;
  return (
    <>
      <span
        aria-hidden
        data-frame={first}
        style={{
          width: outer,
          height: 20,
          borderColor: PICK_COLOUR[first],
          borderStyle: first === "second_best" ? "dashed" : "solid",
        }}
        className="absolute box-border border-2"
      />
      {several ? (
        <span aria-hidden className="absolute h-4 w-4 text-[var(--toolbar-icon)]">
          {PICK_KINDS.filter((kind) => kinds.includes(kind)).map((kind) => (
            <span key={kind} className={`absolute flex ${QUADRANT[kind]}`}>
              <PickGlyph kind={kind} size={8} />
            </span>
          ))}
        </span>
      ) : (
        <span aria-hidden className="absolute flex text-[var(--toolbar-icon)]">
          <PickGlyph kind={first} size={width >= 24 ? 16 : 12} />
        </span>
      )}
    </>
  );
}

/**
 * The pick table of a test (`specs/0146` `ux-notes.md` 4; `specs/0112` 8.3): the columns' axis
 * across, the rows' down, one Tab stop with roving focus. A press, Space or Enter marks the cell
 * with the pressed pick, or takes the pick off a cell that already carries it. A cell shows a
 * frame in its first pick's colour and a glyph per pick; colour only helps.
 */
export function JudgeTable({
  test,
  pick,
  onMark,
}: {
  test: TestDetail;
  /** The pressed pick kind: what a press marks. */
  pick: PickKind;
  onMark: (column: number, row: number) => void;
}) {
  const columns = test.columns.values.length;
  const rows = test.rows.values.length;
  const best = test.picks.find((p) => p.pick === "best")?.cell ?? null;
  const [stop, setStop] = useState<CellPlace>(best ?? { column: 0, row: 0 });
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
    const carries = kindsAt(test.picks, { column, row }).includes(pick);
    onMark(column, row);
    setStatus(pickStatus(test.columns, test.rows, { column, row }, pick, carries));
  };

  return (
    <>
      <div
        ref={grid}
        role="grid"
        aria-label={`Picks of ${test.id}`}
        aria-description={markingText(pick)}
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
              const kinds = kindsAt(test.picks, { column, row });
              const name = cellName(test.columns, test.rows, { column, row }, kinds);
              return (
                <div key={column} role="gridcell" style={{ width, height: CELL_ROW }} className="flex items-center justify-center">
                  <Tooltip side="left" content={kinds.length > 0 ? name : undefined}>
                    <button
                      type="button"
                      data-cell={`${column},${row}`}
                      aria-pressed={kinds.includes(pick)}
                      aria-label={name}
                      tabIndex={at.column === column && at.row === row ? 0 : -1}
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
                      <CellMark kinds={kinds} width={width} />
                    </button>
                  </Tooltip>
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
