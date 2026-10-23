// The rows of the Fonts tab's tree (`specs/0074-text-objects` UX notes §4,
// `specs/0075-font-library` UX tags amendment §2): block headers, the Missing row, a family row of
// two lines (its name in its own face with chevron, current mark, star and face count; under it
// the tag line), a face row.
import { ChevronDown, ChevronRight, Star, TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { cssFamilyOf, type LinePreview, linePreviewOf } from "@/lib/fontFaceLoader";

/** A name drawn in a single-line face's own centre lines (`specs/0076-single-line-fonts`
 * criterion 6, design system "Single-line name"): 1 em = `em` px, 1 CSS px round lines in the
 * row's text colour, clipped to the slot. */
function LineName({ preview, em }: { preview: LinePreview; em: number }) {
  const [x, y, width, height] = preview.box;
  return (
    <svg
      aria-hidden
      viewBox={`${x} ${y} ${width} ${height}`}
      width={width * em}
      height={height * em}
      className="block shrink-0"
    >
      <path
        d={preview.d}
        fill="none"
        stroke="currentColor"
        strokeWidth={1}
        strokeLinecap="round"
        strokeLinejoin="round"
        vectorEffect="non-scaling-stroke"
      />
    </svg>
  );
}
import { tagsLine } from "@/lib/fontLibraryText";
import { type FontFace as ListedFace, type FontFamily, familyName } from "@/lib/textText";

interface FamilyRowProps {
  rowKey: string;
  family: FontFamily;
  open: boolean;
  focused: boolean;
  holdsCurrent: boolean;
  favourite: boolean;
  /** The names of its tags, in chip order. */
  tagNames: string[];
  /** The tag line under the name. */
  tagLine: ReactNode;
  style: React.CSSProperties;
  onFocus: () => void;
  onPick: (byKeyboard: boolean) => void;
  onToggle: () => void;
  onStar: () => void;
}

export function FamilyRow({
  rowKey,
  family,
  open,
  focused,
  holdsCurrent,
  favourite,
  tagNames,
  tagLine,
  style,
  onFocus,
  onPick,
  onToggle,
  onStar,
}: FamilyRowProps) {
  const cssFamily = cssFamilyOf(family.faces[0].id);
  const lines = family.faces[0].singleLine ? linePreviewOf(family.faces[0].id)?.family : undefined;
  const faces = family.faces.length;
  const tagsPart = tagNames.length > 0 ? `, tags ${tagNames.join(", ")}` : "";
  const name = `${familyName(family)}${favourite ? ", favourite" : ""}${tagsPart}`;
  const tip = [
    `${family.family}, ${faces === 1 ? "1 face" : `${faces} faces`}`,
    holdsCurrent && !open ? "Contains the current font" : null,
    tagsLine(tagNames) || null,
  ]
    .filter(Boolean)
    .join("\n");
  return (
    <div
      role="treeitem"
      aria-level={1}
      aria-expanded={open}
      aria-label={name}
      data-row-key={rowKey}
      tabIndex={focused ? 0 : -1}
      onFocus={onFocus}
      onClick={(event) => onPick(event.detail === 0)}
      title={tip}
      style={style}
      className="flex cursor-default flex-col rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
    >
      <div className="flex h-7 shrink-0 items-center gap-1">
        <span
          aria-hidden
          onClick={(event) => {
            event.stopPropagation();
            onToggle();
          }}
          className="flex h-7 w-6 shrink-0 items-center justify-center"
        >
          {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        </span>
        {lines ? (
          <span className="min-w-0 flex-1 overflow-hidden">
            <span className="sr-only">{family.family}</span>
            <LineName preview={lines} em={16} />
          </span>
        ) : (
          <span
            className="min-w-0 flex-1 truncate"
            style={cssFamily ? { fontFamily: cssFamily, fontSize: 16 } : { fontSize: 14 }}
          >
            {family.family}
          </span>
        )}
        {holdsCurrent && !open ? <span aria-hidden className="size-1.5 rotate-45 bg-[var(--editor-accent)]" /> : null}
        <Tooltip side="left" content="Favourite">
          <button
            type="button"
            tabIndex={-1}
            aria-pressed={favourite}
            aria-label={`Favourite: ${family.family}`}
            onMouseDown={(event) => event.preventDefault()}
            onClick={(event) => {
              event.stopPropagation();
              onStar();
            }}
            className="flex h-7 w-6 shrink-0 items-center justify-center rounded-[5px] hover:bg-[var(--editor-accent-hover)]"
          >
            <Star
              size={16}
              strokeWidth={1.5}
              aria-hidden
              className={favourite ? "fill-[var(--toolbar-icon)] text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}
            />
          </button>
        </Tooltip>
        <span className="pr-2 text-xs tabular-nums text-[var(--panel-muted-fg)]">{faces}</span>
      </div>
      {tagLine}
    </div>
  );
}

interface FaceRowProps {
  rowKey: string;
  face: ListedFace;
  current: boolean;
  focused: boolean;
  style: React.CSSProperties;
  onFocus: () => void;
  onPick: (byKeyboard: boolean) => void;
}

export function FaceRow({ rowKey, face, current, focused, style, onFocus, onPick }: FaceRowProps) {
  const cssFamily = cssFamilyOf(face.id);
  const lines = face.singleLine ? linePreviewOf(face.id)?.style : undefined;
  return (
    <div
      role="treeitem"
      aria-level={2}
      aria-selected={current}
      data-row-key={rowKey}
      tabIndex={focused ? 0 : -1}
      onFocus={onFocus}
      onClick={(event) => onPick(event.detail === 0)}
      style={style}
      className={`flex cursor-default items-center rounded-[5px] pl-5 text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
        current ? "border-l-[3px] border-[var(--editor-accent)] bg-[var(--value-fill)] pl-[17px]" : ""
      }`}
    >
      {lines ? (
        <span className="min-w-0 overflow-hidden">
          <span className="sr-only">{face.style}</span>
          <LineName preview={lines} em={14} />
        </span>
      ) : (
        <span className="min-w-0 truncate" style={cssFamily ? { fontFamily: cssFamily, fontSize: 14 } : { fontSize: 14 }}>
          {face.style}
        </span>
      )}
    </div>
  );
}

interface BlockHeaderProps {
  title: string;
  /** "12", "3 of 120", "Reading…" or "checking 340/2000". */
  count: string;
  tip?: string;
  style: React.CSSProperties;
}

export function BlockHeader({ title, count, tip, style }: BlockHeaderProps) {
  return (
    <div
      style={style}
      title={tip}
      className="flex items-center justify-between bg-[var(--panel-bg)] text-xs font-semibold text-[var(--panel-muted-fg)]"
    >
      <span>{title}</span>
      <span className="font-normal tabular-nums">{count}</span>
    </div>
  );
}

interface MissingRowProps {
  rowKey: string;
  name: string;
  focused: boolean;
  style: React.CSSProperties;
  onFocus: () => void;
}

export function MissingRow({ rowKey, name, focused, style, onFocus }: MissingRowProps) {
  return (
    <div
      role="treeitem"
      aria-level={2}
      aria-selected
      data-row-key={rowKey}
      tabIndex={focused ? 0 : -1}
      onFocus={onFocus}
      style={style}
      className="flex items-center gap-1 border-l-[3px] border-[var(--editor-accent)] bg-[var(--value-fill)] pl-[17px] text-sm outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
    >
      <TriangleAlert size={16} aria-hidden className="shrink-0 text-[var(--field-invalid)]" />
      <span className="min-w-0 flex-1 truncate">{name}</span>
      <span className="pr-2 text-xs text-[var(--field-invalid)]">missing</span>
    </div>
  );
}
