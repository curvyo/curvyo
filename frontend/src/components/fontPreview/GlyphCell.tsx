// One glyph table cell's drawing (`specs/0082-glyph-table` criteria 7, 8 and 15, UX notes §3):
// the face's own glyph scaled to the Size (cap height in px), on the baseline at 0.68 of the side,
// centred on its advance and clipped at the edge; the centre lines of a single-line face; a
// dotted box and a label for a glyph that draws nothing; a quiet bar until it is drawn.
import { CELL_BASELINE } from "@/lib/glyphTableLayout";
import type { CellDrawing } from "@/lib/glyphFaces";

interface GlyphDrawingProps {
  cell: CellDrawing | null | undefined;
  /** The box's side in px. */
  side: number;
  /** Cap height in px. */
  cap: number;
  singleLine: boolean;
  /** The label of an empty glyph ("SP", "2002"). */
  label: string;
}

export function GlyphDrawing({ cell, side, cap, singleLine, label }: GlyphDrawingProps) {
  const baseline = CELL_BASELINE * side;
  if (cell === undefined) {
    return (
      <span
        aria-hidden
        className="absolute rounded-[3px] bg-[var(--tile-skeleton)]"
        style={{ left: side * 0.3, width: side * 0.4, top: baseline - cap, height: cap }}
      />
    );
  }
  if (cell === null) {
    return null;
  }
  if (cell.empty) {
    const width = Math.min(side - 8, Math.max(4, cell.adv * cap));
    return (
      <>
        <span
          aria-hidden
          className="absolute border border-dotted border-[var(--panel-muted-fg)]"
          style={{ left: (side - width) / 2, width, top: baseline - cap, height: cap }}
        />
        <span
          aria-hidden
          className="absolute inset-x-0 text-center text-[10px] leading-3 text-[var(--panel-muted-fg)] tabular-nums"
          style={{ top: baseline + 3 }}
        >
          {label}
        </span>
      </>
    );
  }
  const x = (side - cell.adv * cap) / 2;
  return (
    <svg aria-hidden width={side} height={side} className="absolute inset-0">
      {singleLine ? (
        <path
          d={cell.d}
          transform={`translate(${x} ${baseline}) scale(${cap})`}
          fill="none"
          stroke="currentColor"
          strokeWidth={1}
          strokeLinecap="round"
          strokeLinejoin="round"
          vectorEffect="non-scaling-stroke"
        />
      ) : (
        <path d={cell.d} transform={`translate(${x} ${baseline}) scale(${cap})`} fill="currentColor" />
      )}
    </svg>
  );
}
