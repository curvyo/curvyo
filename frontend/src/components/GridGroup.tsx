import { Tooltip } from "radix-ui";
import { useRef } from "react";

import { ArrangeTooltip, BarField, BUTTON_CLASS } from "@/components/AlignGroup";
import type { AlignApi } from "@/hooks/useAlign";
import { GRID_TEXT } from "@/lib/alignText";

/** The Grid glyph: four 4.5 px squares in a 2 x 2 block, outlined, in the
 * family of the align glyphs (UX notes, "Grid group"). */
function GridGlyph() {
  return (
    <svg aria-hidden viewBox="0 0 16 16" className="size-4" fill="none">
      {[
        [2, 2],
        [9.5, 2],
        [2, 9.5],
        [9.5, 9.5],
      ].map(([x, y]) => (
        <rect key={`${x}-${y}`} x={x} y={y} width="4.5" height="4.5" stroke="currentColor" strokeWidth="1.5" />
      ))}
    </svg>
  );
}

/**
 * The Grid group of the Select bar (`specs/0061-align-and-distribute/`
 * criteria 18 to 22): the Arrange in a grid button, then Rows, Columns and
 * Gap. In the bar from two selected objects; Enter in a field arranges.
 */
export function GridBarGroup({ align, onReturnFocus }: { align: AlignApi; onReturnFocus: () => void }) {
  const { grid } = align;
  // Escape restores all three fields as they were when the field got focus:
  // typing a count clears the other, so one field's own value is not enough.
  const snapshot = useRef(grid.fields);
  const remember = () => {
    snapshot.current = grid.fields;
  };
  const restore = () => grid.restore(snapshot.current);
  return (
    <Tooltip.Provider delayDuration={400}>
      <div role="group" aria-label="Arrange in a grid" className="flex flex-wrap items-center gap-x-1 gap-y-2 py-1">
        <ArrangeTooltip name={GRID_TEXT.name} rule={GRID_TEXT.rule}>
          <button
            type="button"
            aria-label={GRID_TEXT.name}
            onClick={(event) => {
              grid.arrange();
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
            className={BUTTON_CLASS}
          >
            <GridGlyph />
          </button>
        </ArrangeTooltip>
        <BarField
          label="Rows"
          accessibleName="Rows"
          widthClass="w-[44px]"
          value={grid.fields.rows}
          placeholder={String(grid.derived.rows)}
          derivedPlaceholder
          error={grid.errors.rows}
          onChange={(text) => grid.setField("rows", text)}
          onEnter={grid.arrange}
          onFocusField={remember}
          onEscape={restore}
        />
        <BarField
          label="Columns"
          accessibleName="Columns"
          widthClass="w-[44px]"
          value={grid.fields.columns}
          placeholder={String(grid.derived.columns)}
          derivedPlaceholder
          error={grid.errors.columns}
          onChange={(text) => grid.setField("columns", text)}
          onEnter={grid.arrange}
          onFocusField={remember}
          onEscape={restore}
        />
        <BarField
          label="Gap"
          accessibleName="Grid gap, millimetres"
          widthClass="w-[72px]"
          suffix="mm"
          value={grid.fields.gap}
          placeholder="0"
          error={grid.errors.gap}
          onChange={(text) => grid.setField("gap", text)}
          onEnter={grid.arrange}
          onFocusField={remember}
          onEscape={restore}
        />
      </div>
    </Tooltip.Provider>
  );
}
