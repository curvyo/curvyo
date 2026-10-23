// One tile of the font preview (`specs/0077-font-preview` criteria 11, 14, 15, 17 to 20 and 22,
// UX notes §3 and §4): a white specimen card with the sample drawn from the engine's outlines (a
// filled path, or the centre lines of a single-line face; a quiet bar until then), the family name
// and the star, the family's tag line in a row of its own (all its tags with x, "+N" and Add tag,
// the list's own component), the missing-glyph marker and the current mark.
import { Star, TriangleAlert } from "lucide-react";

import { Tooltip } from "@/components/ui/tooltip";
import { AREA_SLACK_EM, BASELINE_EM, emPx, sampleAreaHeight } from "@/lib/fontPreviewLayout";
import { missingText, tileDescription, tileName } from "@/lib/fontPreviewText";
import type { TileDrawing } from "@/lib/previewFaces";

interface PreviewTileProps {
  index: number;
  /** The family's key (`familyKey`), where the tag line is looked up. */
  rowKey: string;
  family: string;
  /** What the tile sets: the sample, or the family name for an empty sample. */
  text: string;
  drawing: TileDrawing | null;
  size: number;
  cuttable: boolean;
  singleLine: boolean;
  favourite: boolean;
  current: boolean;
  focused: boolean;
  /** The family's tag line and its width in the footer (criterion 14). */
  tagLine: React.ReactNode;
  tagLineStyle: React.CSSProperties;
  /** After "+N": every tag, wrapped, over the sample until the pointer leaves or Escape. */
  allTags: React.ReactNode;
  onLeaveTags: () => void;
  style: React.CSSProperties;
  onFocus: () => void;
  onPick: (byKeyboard: boolean) => void;
  onStar: () => void;
}

/** The sample as the engine drew it, one line on the tile's baseline, however wide (the sample
 * area clips it): 1 em of the face = size / cap height px. */
function Sample({ drawing, size }: { drawing: TileDrawing; size: number }) {
  const [x, y, width, height] = drawing.box;
  const scale = size / (drawing.cap > 0 ? drawing.cap : 0.7);
  if (width <= 0 || height <= 0) {
    return null;
  }
  return (
    <svg
      aria-hidden
      viewBox={`${x} ${y} ${width} ${height}`}
      width={width * scale}
      height={height * scale}
      className="absolute left-0 max-w-none overflow-visible"
      style={{ top: emPx(size) * (AREA_SLACK_EM + BASELINE_EM) + y * scale }}
    >
      {drawing.single_line ? (
        <path
          d={drawing.d}
          fill="none"
          stroke="currentColor"
          strokeWidth={1}
          strokeLinecap="round"
          strokeLinejoin="round"
          vectorEffect="non-scaling-stroke"
        />
      ) : (
        <path d={drawing.d} fill="currentColor" />
      )}
    </svg>
  );
}

export function PreviewTile(props: PreviewTileProps) {
  const { index, family, text, drawing, size, cuttable, singleLine, favourite, current, focused, style } = props;
  const missing = drawing?.missing ?? [];
  const descriptionId = `font-preview-tile-${index}`;
  return (
    <div
      role="option"
      aria-selected={current}
      aria-label={tileName(family, { favourite, cuttable, singleLine, current })}
      aria-describedby={descriptionId}
      data-tile-index={index}
      data-row-key={props.rowKey}
      tabIndex={focused ? 0 : -1}
      onFocus={props.onFocus}
      onClick={(event) => props.onPick(event.detail === 0)}
      onPointerLeave={props.allTags ? props.onLeaveTags : undefined}
      style={style}
      className={`group absolute flex cursor-default flex-col overflow-hidden rounded-lg border border-[var(--tile-edge)] px-4 pt-3 pb-2 text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
        current ? "bg-[var(--value-fill)]" : "bg-[var(--tile-bg)] hover:bg-[var(--editor-accent-hover)]"
      }`}
    >
      <span id={descriptionId} className="sr-only">
        {tileDescription(text, missing)}
      </span>
      {current ? <span aria-hidden className="absolute top-0 bottom-0 left-0 w-[3px] bg-[var(--editor-accent)]" /> : null}
      {/* One line, cut off hard at the tile's right edge (customer decision 2026-10-23). */}
      <div
        aria-hidden
        className="relative -mr-4 shrink-0 overflow-hidden"
        style={{ height: sampleAreaHeight(size) }}
      >
        {drawing ? (
          <Sample drawing={drawing} size={size} />
        ) : (
          // Until the face's own glyphs are drawn: a quiet bar of the line's cap height, never the
          // sample in another font (customer decision 2026-10-23).
          <span
            data-skeleton
            className="absolute left-0 w-3/5 rounded-[3px] bg-[var(--tile-skeleton)]"
            style={{ top: emPx(size) * (AREA_SLACK_EM + BASELINE_EM) - size, height: size }}
          />
        )}
      </div>
      {props.allTags ? (
        <div
          className="absolute inset-x-0 top-0 bottom-[60px] z-10 overflow-y-auto bg-[var(--tile-bg)] px-4 pt-3"
          onClick={(event) => event.stopPropagation()}
        >
          {props.allTags}
        </div>
      ) : null}
      {missing.length > 0 ? (
        <Tooltip side="bottom" content={missingText(missing)}>
          <span
            aria-hidden
            className="absolute top-2 right-2 flex size-6 items-center justify-center rounded-[5px] bg-inherit"
          >
            <TriangleAlert size={14} strokeWidth={1.5} className="text-[var(--panel-muted-fg)]" />
          </span>
        </Tooltip>
      ) : null}
      {/* Two rows: the name and the star, then the tag row, reserved on every tile (customer
       * request 2026-10-23). */}
      <div className="mt-2 flex h-7 shrink-0 items-center gap-1">
        <span className="min-w-0 flex-1 truncate text-xs text-[var(--panel-muted-fg)]">{family}</span>
        <Tooltip side="bottom" content="Favourite">
          <button
            type="button"
            tabIndex={-1}
            aria-pressed={favourite}
            aria-label={`Favourite: ${family}`}
            onMouseDown={(event) => event.preventDefault()}
            onClick={(event) => {
              event.stopPropagation();
              props.onStar();
            }}
            className="-mr-1 flex h-7 w-6 shrink-0 items-center justify-center rounded-[5px] hover:bg-[var(--editor-accent-hover)]"
          >
            <Star
              size={16}
              strokeWidth={1.5}
              aria-hidden
              className={favourite ? "fill-[var(--toolbar-icon)] text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}
            />
          </button>
        </Tooltip>
      </div>
      {/* A press on a chip, its x, "+N" or Add tag edits the tags and never picks the font. */}
      <div className="flex h-6 min-w-0 shrink-0" style={props.tagLineStyle} onClick={(event) => event.stopPropagation()}>
        {props.tagLine}
      </div>
    </div>
  );
}
