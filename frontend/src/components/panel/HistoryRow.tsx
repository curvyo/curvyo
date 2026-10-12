// The rows of the History list (`specs/0020-undo-redo`, `0041-object-history`,
// `0042-history-branches`): a step row with its lane gutter and row action, and the fold rows of
// branches.

import {
  ChevronDown,
  ChevronRight,
  Copy,
  CopyPlus,
  Eraser,
  File,
  Minus,
  Pencil,
  Plus,
  Redo2,
  Replace,
  Undo2,
} from "lucide-react";
import { type ComponentType, useRef } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import {
  type FoldItem,
  type GutterSegment,
  type RowItem,
  edgeY,
  forkPath,
  gutterWidth,
  laneX,
  MAX_SUB_ROWS,
  objectsHeight,
  SUB_ROW_HEIGHT_PX,
} from "@/lib/historyItems";
import {
  type HistoryRowData,
  isUndone,
  leadWords,
  ROW_HEIGHT_PX,
  rowAriaName,
  rowDisplay,
  rowTooltip,
  secondLineRest,
} from "@/lib/historyRows";

type Glyph = ComponentType<{ size: number; strokeWidth: number; "aria-hidden": boolean }>;

type GlyphKey = "create" | "delete" | "replace" | "change" | "document" | "undoes" | "redoes" | "clone-start" | "floor";

/** The glyph of each kind of step and of each row that exists only in the Object scope (criterion
 * 32, `0041` UX notes): a shape, never a colour. */
const GLYPHS: Record<GlyphKey, Glyph> = {
  create: Plus,
  delete: Minus,
  replace: Replace,
  change: Pencil,
  document: File,
  undoes: Undo2,
  redoes: Redo2,
  "clone-start": Copy,
  floor: Eraser,
};

/** Which glyph a row has: by class, then by what a replacing step did to the object, then by
 * kind. */
function glyphKey(row: HistoryRowData): GlyphKey {
  if (row.label === "wipe_history") {
    return "floor";
  }
  if (row.class && row.class !== "step") {
    return row.class;
  }
  if (row.kind === "replace" && row.role === "created") {
    return "create";
  }
  if (row.kind === "replace" && row.role === "deleted") {
    return "delete";
  }
  return row.kind;
}

/** An item's slice of the lane gutter (`specs/0042-history-branches` UX notes): the main lane (a
 * filled disc while in effect, an open ring on a dotted line once taken back, dashed for an
 * inherited row; criterion 39 of `0020`), branch lanes with open rings, and the connector at a
 * fork point. Every item draws its own slice, so the lines are continuous in a virtualised list. */
export function Gutter({ segments, lanes, height }: { segments: GutterSegment[]; lanes: number; height: number }) {
  return (
    <svg
      width={gutterWidth(lanes)}
      height={height}
      aria-hidden
      className="shrink-0 text-[var(--toolbar-icon)]"
    >
      {segments.map((segment, n) => {
        const key = `${n}`;
        switch (segment[0]) {
          case "line": {
            const [, lane, from, to, style] = segment;
            const branch = style === "branch";
            const quiet = branch || style === "dotted";
            return (
              <line
                key={key}
                x1={laneX(lane)}
                x2={laneX(lane)}
                y1={edgeY(from, height)}
                y2={edgeY(to, height)}
                stroke={quiet ? "var(--lane-line)" : "currentColor"}
                strokeWidth={branch ? 1.5 : 2}
                strokeDasharray={style === "dotted" ? "2 3" : style === "dashed" ? "6 3" : undefined}
              />
            );
          }
          case "disc":
            return <circle key={key} cx={laneX(segment[1])} cy={height / 2} r={4} fill="currentColor" />;
          case "ring":
            return (
              <circle
                key={key}
                cx={laneX(segment[1])}
                cy={height / 2}
                r={3.5}
                fill="var(--panel-bg)"
                stroke={segment[1] === 0 ? "currentColor" : "var(--lane-line)"}
                strokeWidth={1.5}
              />
            );
          case "fork":
            return (
              <path
                key={key}
                d={forkPath(segment[1], segment[2], height)}
                fill="none"
                stroke="var(--lane-line)"
                strokeWidth={1.5}
              />
            );
          default:
            return null;
        }
      })}
    </svg>
  );
}

/**
 * The History tab (`specs/0020-undo-redo` criteria 9 and 39 to 46): the Undo and Redo buttons, the
 * pinned "Now" row, the global history as one virtualised list (newest first, 44 px rows, only the
 * rows in view are in the DOM) and the line that says the history is saved in the file. The header
 * and the footer stay in view; only the list scrolls.
 */
interface StepRowProps {
  row: RowItem;
  index: number;
  total: number;
  lanes: number;
  place: { top: number; height: number };
  now: Date;
  tabbable: boolean;
  onFocusRow: () => void;
  onHover: (step: string) => void;
  onPress: () => void;
  /** The row an exploration went to last: the selected look. */
  selected?: boolean;
  /** The row's action (`specs/0042-history-branches` criteria 22 and 24): Clone in the Object
   * scope, the objects disclosure in the Document scope; none for a row without one. */
  action: "clone" | "objects" | null;
  /** Whether the objects disclosure is open. */
  objectsOpen?: boolean;
  onToggleObjects?: () => void;
  /** Clone of the row's version: of the object (`undefined`) or of the `touched`-th object. */
  onClone: (touched?: number) => void;
}

const ACTION_CLASS =
  "flex h-6 w-6 shrink-0 items-center justify-center self-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

const CLONE_TOOLTIP = "Clone this version as a new object";

function kindTitle(shape: string): string {
  return shape.charAt(0).toUpperCase() + shape.slice(1);
}

/** One step: kind glyph, operation name, step id, the second line (criterion 39 of `0020`), and
 * the row action in the slot at the right. Right moves focus into the action, Left back
 * (`0042` criterion 9). */
export function StepRow({
  row,
  index,
  total,
  lanes,
  place,
  now,
  tabbable,
  onFocusRow,
  onHover,
  onPress,
  selected = false,
  action,
  objectsOpen = false,
  onToggleObjects,
  onClone,
}: StepRowProps) {
  const Glyph = GLYPHS[glyphKey(row)] ?? Pencil;
  const undone = isUndone(row);
  const lead = leadWords(row);
  const muted = undone ? "text-[var(--panel-muted-fg)]" : "text-[var(--toolbar-icon)]";
  const main = useRef<HTMLButtonElement>(null);
  const side = useRef<HTMLButtonElement>(null);
  const continuation = row.gutter
    .filter((segment) => segment[0] === "line" && segment[3] === "bottom")
    .map((segment) => ["line", segment[1], "top", "bottom", segment[4]] as GutterSegment);
  return (
    <div
      role="listitem"
      aria-posinset={index + 1}
      aria-setsize={total}
      className="absolute right-0 left-0 flex flex-col"
      style={place}
    >
      <div className="flex shrink-0 items-stretch" style={{ height: ROW_HEIGHT_PX }}>
        <Tooltip side="left" content={rowTooltip(row)}>
          <button
            ref={main}
            type="button"
            data-row-index={index}
            tabIndex={tabbable ? 0 : -1}
            aria-label={rowAriaName(row, now)}
            onFocus={onFocusRow}
            onClick={onPress}
            onKeyDown={(event) => {
              if (event.key === "ArrowRight" && side.current) {
                event.preventDefault();
                side.current.focus();
              }
            }}
            onBlur={() => onHover("")}
            onMouseEnter={() => onHover(row.step)}
            onMouseLeave={() => onHover("")}
            aria-current={selected ? "true" : undefined}
            className={`flex h-full min-w-0 flex-1 items-stretch gap-1 rounded-[5px] text-left outline-none hover:bg-[var(--row-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
              selected ? "bg-[var(--value-fill)] shadow-[inset_3px_0_0_var(--editor-accent)]" : ""
            }`}
          >
            <Gutter segments={row.gutter} lanes={lanes} height={ROW_HEIGHT_PX} />
            <span className="flex min-w-0 flex-1 flex-col justify-center pr-2">
              <span className={`flex h-5 items-center gap-1.5 text-sm ${muted}`}>
                <Glyph size={16} strokeWidth={1.5} aria-hidden />
                <span className="min-w-0 flex-1 truncate">{rowDisplay(row).name}</span>
                <span className="select-text font-mono text-xs text-[var(--panel-muted-fg)]">
                  {row.id}
                </span>
              </span>
              <span className="h-4 truncate text-xs text-[var(--panel-muted-fg)]">
                {lead !== "" ? <strong className="font-semibold">{lead}</strong> : null}
                {lead !== "" ? " · " : null}
                {secondLineRest(row, now)}
              </span>
            </span>
          </button>
        </Tooltip>
        {action !== null ? (
          <span className="flex w-7 shrink-0 items-center justify-center">
            <Tooltip side="left" content={action === "clone" ? CLONE_TOOLTIP : "Objects of this step"}>
              <button
                ref={side}
                type="button"
                tabIndex={-1}
                aria-label={action === "clone" ? CLONE_TOOLTIP : "Objects of this step"}
                aria-expanded={action === "objects" ? objectsOpen : undefined}
                onKeyDown={(event) => {
                  if (event.key === "ArrowLeft") {
                    event.preventDefault();
                    main.current?.focus();
                  }
                }}
                onClick={() => (action === "clone" ? onClone() : onToggleObjects?.())}
                className={ACTION_CLASS}
              >
                {action === "clone" ? (
                  <CopyPlus size={16} strokeWidth={1.5} aria-hidden />
                ) : objectsOpen ? (
                  <ChevronDown size={16} strokeWidth={1.5} aria-hidden />
                ) : (
                  <ChevronRight size={16} strokeWidth={1.5} aria-hidden />
                )}
              </button>
            </Tooltip>
          </span>
        ) : null}
      </div>
      {objectsOpen ? (
        <div className="flex min-h-0 flex-1">
          <Gutter segments={continuation} lanes={lanes} height={objectsHeight(row)} />
          <ul className="min-w-0 flex-1 pl-5">
            {row.objects.slice(0, MAX_SUB_ROWS).map((object, n) => {
              const name = `${kindTitle(object.shape)} ${object.short ?? ""}`;
              return (
                <li
                  key={`${object.short ?? ""}-${n}`}
                  className="flex items-center gap-1.5 text-xs text-[var(--toolbar-icon)]"
                  style={{ height: SUB_ROW_HEIGHT_PX }}
                >
                  <span className="min-w-0 flex-1 truncate">
                    {name}
                    {object.role === "deleted" ? (
                      <span className="text-[var(--panel-muted-fg)]">, deleted</span>
                    ) : null}
                  </span>
                  {object.shape !== "group" ? (
                    <Tooltip side="left" content={CLONE_TOOLTIP}>
                      <button
                        type="button"
                        aria-label={`Clone ${name} at version ${row.id}`}
                        onClick={() => onClone(n)}
                        className={ACTION_CLASS}
                      >
                        <CopyPlus size={16} strokeWidth={1.5} aria-hidden />
                      </button>
                    </Tooltip>
                  ) : null}
                </li>
              );
            })}
            {row.objects.length > MAX_SUB_ROWS ? (
              <li
                className="flex items-center text-xs text-[var(--panel-muted-fg)]"
                style={{ height: SUB_ROW_HEIGHT_PX }}
              >
                and {row.objects.length - MAX_SUB_ROWS} more
              </li>
            ) : null}
          </ul>
        </div>
      ) : null}
    </div>
  );
}

interface FoldRowProps {
  item: FoldItem;
  index: number;
  total: number;
  lanes: number;
  place: { top: number; height: number };
  tabbable: boolean;
  onFocusRow: () => void;
  onToggle: () => void;
}

/** A branch header row ("Branch from a1b2c3d, 2 steps") or the "N more branches" row: a press or
 * Enter folds or unfolds it (`specs/0042-history-branches` criterion 8). */
export function FoldRow({ item, index, total, lanes, place, tabbable, onFocusRow, onToggle }: FoldRowProps) {
  const Chevron = item.expanded ? ChevronDown : ChevronRight;
  return (
    <div role="listitem" aria-posinset={index + 1} aria-setsize={total} className="absolute right-0 left-0" style={place}>
      <button
        type="button"
        data-row-index={index}
        tabIndex={tabbable ? 0 : -1}
        aria-expanded={item.expanded}
        onFocus={onFocusRow}
        onClick={onToggle}
        className="flex h-full w-full items-stretch gap-1 rounded-[5px] text-left outline-none hover:bg-[var(--row-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
      >
        <Gutter segments={item.gutter} lanes={lanes} height={place.height} />
        <span
          className={`flex min-w-0 flex-1 items-center justify-between pr-2 text-xs text-[var(--panel-muted-fg)] ${
            item.type === "header" ? "font-semibold" : ""
          }`}
        >
          <span className="truncate">{item.text}</span>
          <Chevron size={12} strokeWidth={1.5} aria-hidden />
        </span>
      </button>
    </div>
  );
}

