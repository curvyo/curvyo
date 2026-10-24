// The glyph table's detail bar (`specs/0082-glyph-table` criterion 19, UX notes P4 and §5): always
// 97px at the pane's bottom in Glyphs mode; the hovered cell, else the current one, in an 80px box
// at cap height 40px, its Unicode name, "U+00E4 · adieresis · Latin-1 Supplement" as selectable
// text, and Insert.
import { GlyphDrawing } from "@/components/fontPreview/GlyphCell";
import { cachedCell, cachedDetail, drawCells, namesLoading } from "@/lib/glyphFaces";
import { DETAIL_BAR_PX } from "@/lib/glyphTableLayout";
import { charOf, detailMeta, emptyCellLabel } from "@/lib/glyphTableText";
import type { FontFace as ListedFace } from "@/lib/textText";

const BOX_PX = 80;
const BOX_CAP_PX = 40;

interface GlyphDetailBarProps {
  face: ListedFace | null;
  cp: number | null;
  singleLine: boolean;
  onInsert: (cp: number, byKeyboard: boolean) => void;
}

export function GlyphDetailBar({ face, cp, singleLine, onInsert }: GlyphDetailBarProps) {
  const shown = face && cp !== null ? cp : null;
  if (face && shown !== null && cachedCell(face, shown) === undefined) {
    drawCells(face, [shown]);
  }
  const cell = face && shown !== null ? cachedCell(face, shown) : undefined;
  const detail = face && shown !== null ? cachedDetail(face, shown) : null;
  return (
    <div
      className="flex shrink-0 items-center gap-3 border-t border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] bg-[var(--panel-bg)] p-2"
      style={{ height: DETAIL_BAR_PX }}
    >
      {shown !== null ? (
        <>
          <div
            className="relative shrink-0 overflow-hidden rounded-[5px] border border-[var(--tile-edge)] bg-[var(--tile-bg)] text-[var(--toolbar-icon)]"
            style={{ width: BOX_PX, height: BOX_PX }}
          >
            <GlyphDrawing
              cell={cell}
              side={BOX_PX - 2}
              cap={BOX_CAP_PX}
              singleLine={singleLine}
              label={emptyCellLabel(detail?.abbreviation ?? null, shown)}
            />
            {/* The character itself, so selecting across the bar copies it. */}
            <span className="absolute inset-0 overflow-hidden text-transparent select-text">{charOf(shown)} </span>
          </div>
          <div className="flex min-w-0 flex-1 flex-col self-start pt-1 select-text">
            {detail?.name ? (
              <p className="line-clamp-2 text-sm break-words text-[var(--toolbar-icon)]">{detail.name}</p>
            ) : namesLoading() ? (
              <span aria-hidden className="block h-3.5 w-3/5 rounded-[3px] bg-[var(--tile-skeleton)]" />
            ) : null}
            <p className="mt-1 truncate text-xs text-[var(--panel-muted-fg)] tabular-nums">
              {detailMeta(shown, detail?.glyph_name ?? null, detail?.block ?? "")}
            </p>
          </div>
          <button
            type="button"
            onClick={(event) => onInsert(shown, event.detail === 0)}
            className="h-7 shrink-0 rounded-[5px] px-3 text-sm text-[var(--toolbar-icon)] outline-none ring-1 ring-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          >
            Insert
          </button>
        </>
      ) : null}
    </div>
  );
}
