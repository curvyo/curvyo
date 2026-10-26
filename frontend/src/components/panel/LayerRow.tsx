import {
  ChevronDown,
  ChevronRight,
  Circle,
  Eye,
  EyeOff,
  Group,
  Hexagon,
  Image,
  Layers,
  Link2,
  ListChecks,
  Locate,
  Lock,
  LockOpen,
  Spline,
  Square,
  Star,
  Type,
} from "lucide-react";
import type { ComponentType } from "react";
import { useEffect, useRef } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { type LayerRowData, indentPx } from "@/lib/layerRows";

type Glyph = ComponentType<{ size: number; strokeWidth: number; "aria-hidden": boolean }>;

/** The compound path glyph: a plate with a hole (the rail's Split art, row "Command glyphs"). */
function CompoundGlyph() {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" aria-hidden fill="none">
      <path
        d="M2.5 2.5h11v11h-11z M6 6h4v4H6z"
        stroke="currentColor"
        strokeWidth={1.5}
        fillRule="evenodd"
      />
    </svg>
  );
}

/** The kind glyph of each row kind (UX notes, "Kind glyphs"): shapes, never colour. */
const KIND_GLYPHS: Record<string, Glyph> = {
  layer: Layers,
  group: Group,
  path: Spline,
  rectangle: Square,
  ellipse: Circle,
  polygon: Hexagon,
  star: Star,
  // A linked clone (`specs/0049-linked-clone`, UX notes "The Layers list").
  clone: Link2,
  // A text object (`specs/0074-text-objects` criterion 27).
  text: Type,
  // An image object (`specs/0058-bitmap-import` criterion 14).
  image: Image,
};

/** The four cells of a row, in order (criterion 38, customer feedback 2026-10-14). */
export type RowCell = "name" | "action" | "eye" | "lock";

export interface LayerRowProps {
  row: LayerRowData;
  /** The row's top inside the list, px. */
  top: number;
  height: number;
  /** The row holds the list's one Tab stop. */
  tabbable: boolean;
  /** The cell the roving focus is in, when the row has it. */
  cell: RowCell;
  /** The name is a text field. */
  renaming: boolean;
  /** The drop mark a drag shows on this row. */
  drop: "above" | "below" | "into" | null;
  /** A row drag runs: no tooltip opens (UX review item 12). */
  dragging: boolean;
  onPress: (event: React.MouseEvent | React.KeyboardEvent) => void;
  onFlag: (flag: "visible" | "locked") => void;
  onExpand: (expanded: boolean) => void;
  onAction: () => void;
  onRename: (name: string | null) => void;
  onStartRename: () => void;
  onDragStart: (event: React.PointerEvent) => void;
  onHover: (hovered: boolean) => void;
  onFocusCell: (cell: RowCell) => void;
}

const ICON_BUTTON =
  "flex size-6 shrink-0 items-center justify-center rounded-[4px] outline-none hover:text-[var(--toolbar-icon)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-10 pointer-coarse:w-9";

/** The eye or the lock: its own state as a shape, an inherited one muted in a dotted ring. */
function FlagButton({
  kind,
  on,
  inherited,
  label,
  focused,
  onPress,
  onFocus,
}: {
  kind: "visible" | "locked";
  /** Its own flag shows the strong state: hidden or locked. */
  on: boolean;
  /** An ancestor gives the state: "Hidden by layer Cut". */
  inherited: string;
  label: string;
  focused: boolean;
  onPress: () => void;
  onFocus: () => void;
}) {
  const ref = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (focused) {
      ref.current?.focus();
    }
  }, [focused]);
  const strong = kind === "visible" ? EyeOff : Lock;
  const calm = kind === "visible" ? Eye : LockOpen;
  const Shown = on || inherited !== "" ? strong : calm;
  const color = on
    ? "text-[var(--toolbar-icon)]"
    : inherited !== ""
      ? "text-[var(--panel-muted-fg)]"
      : "text-[var(--lane-line)]";
  return (
    <Tooltip side="left" content={inherited !== "" && !on ? `${label} (${inherited})` : label}>
      <button
        ref={ref}
        type="button"
        tabIndex={-1}
        aria-label={label}
        aria-description={inherited !== "" ? inherited : undefined}
        // A press on the eye or the lock keeps the focus where it was (criterion 40).
        onMouseDown={(event) => event.preventDefault()}
        onPointerDown={(event) => event.stopPropagation()}
        onClick={(event) => {
          event.stopPropagation();
          onPress();
        }}
        onFocus={onFocus}
        className={`${ICON_BUTTON} ${color}`}
      >
        <span
          className={
            inherited !== "" && !on
              ? "flex size-5 items-center justify-center rounded-full border border-dotted border-current"
              : "flex"
          }
        >
          <Shown size={16} strokeWidth={1.5} aria-hidden />
        </span>
      </button>
    </Tooltip>
  );
}

/**
 * One row of the Layers list (`specs/0050-layers-and-objects-panel` criteria 6 to 8, 22, 38 and
 * 41, UX notes "Rows"): the indent, the chevron, the kind glyph, the name, then the opacity
 * readout, the row action, the eye and the lock (customer feedback 2026-10-14). Every word is the session's;
 * a name is text, never markup (criterion 17).
 */
export function LayerRow({
  row,
  top,
  height,
  tabbable,
  cell,
  renaming,
  drop,
  dragging,
  onPress,
  onFlag,
  onExpand,
  onAction,
  onRename,
  onStartRename,
  onDragStart,
  onHover,
  onFocusCell,
}: LayerRowProps) {
  const rowRef = useRef<HTMLDivElement>(null);
  const actionRef = useRef<HTMLButtonElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (tabbable && cell === "name" && !renaming) {
      rowRef.current?.focus();
    }
    if (tabbable && cell === "action") {
      actionRef.current?.focus();
    }
  }, [tabbable, cell, renaming]);
  useEffect(() => {
    if (renaming) {
      inputRef.current?.focus();
      inputRef.current?.select();
    }
  }, [renaming]);
  const Kind = KIND_GLYPHS[row.kind];
  const selected = row.mark === "selected";
  const active = row.mark === "active";
  const entered = row.mark === "entered";
  const hidden = !row.effectiveVisible;
  const tooltip = (
    <span className="flex flex-col">
      <span>{row.name}</span>
      {row.chainLine ? (
        <span className="text-[var(--panel-muted-fg)]">{row.chainLine}</span>
      ) : null}
      {row.overridesLine ? (
        <span className="text-[var(--panel-muted-fg)]">{row.overridesLine}</span>
      ) : null}
      {row.stateLine !== "" ? (
        <span className="text-[var(--panel-muted-fg)]">{row.stateLine}</span>
      ) : null}
    </span>
  );
  const showAction = row.rowAction !== "";
  // A clone row selects its original, a group or layer row its contents (`0049`, `0050`).
  const actionLabel = row.rowAction === "select-original" ? "Select original" : "Select contents";
  const nameClass = `min-w-0 flex-1 truncate ${active ? "font-semibold" : ""} ${
    hidden ? "text-[var(--panel-muted-fg)]" : "text-[var(--toolbar-icon)]"
  }`;
  return (
    <div
      ref={rowRef}
      role="row"
      aria-level={row.level}
      aria-posinset={row.posinset}
      aria-setsize={row.setsize}
      aria-rowindex={row.index + 1}
      aria-selected={selected}
      aria-expanded={row.container ? row.expanded : undefined}
      aria-current={active || entered ? "true" : undefined}
      aria-label={row.label}
      tabIndex={tabbable && cell === "name" ? 0 : -1}
      data-row-index={row.index}
      onFocus={(event) => {
        if (event.target === event.currentTarget) {
          onFocusCell("name");
        }
      }}
      onMouseEnter={() => onHover(true)}
      onMouseLeave={() => onHover(false)}
      onClick={(event) => onPress(event)}
      className={`group/row absolute right-0 left-0 flex items-center rounded-[4px] pr-1 text-sm outline-none select-none hover:bg-[var(--row-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
        selected ? "!bg-[var(--value-fill)]" : ""
      } ${entered || drop === "into" ? "ring-[1.5px] ring-inset ring-[var(--editor-accent)]" : ""}`}
      style={{ top, height }}
    >
      {selected || active ? (
        <span aria-hidden className="absolute top-0 bottom-0 left-0 w-[3px] rounded-l bg-[var(--editor-accent)]" />
      ) : null}
      {drop === "above" || drop === "below" ? (
        <span
          aria-hidden
          className="pointer-events-none absolute right-0 left-0 h-[2px] bg-[var(--editor-accent)]"
          style={drop === "above" ? { top: -1 } : { bottom: -1 }}
        />
      ) : null}
      <span aria-hidden className="shrink-0" style={{ width: indentPx(row.level) }} />
      <span role="gridcell" className="relative flex min-w-0 flex-1 items-center">
        {row.container ? (
          <button
            type="button"
            tabIndex={-1}
            aria-label={row.expanded ? "Collapse" : "Expand"}
            onMouseDown={(event) => event.preventDefault()}
            onPointerDown={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation();
              onExpand(!row.expanded);
            }}
            className="flex h-7 w-6 shrink-0 items-center justify-center text-[var(--toolbar-icon)] outline-none pointer-coarse:h-10 pointer-coarse:w-9"
          >
            {row.expanded ? (
              <ChevronDown size={16} strokeWidth={1.5} aria-hidden />
            ) : (
              <ChevronRight size={16} strokeWidth={1.5} aria-hidden />
            )}
          </button>
        ) : (
          <span aria-hidden className="w-6 shrink-0 pointer-coarse:w-9" />
        )}
        <span
          className="flex min-w-0 flex-1 items-center gap-1"
          onPointerDown={renaming ? undefined : onDragStart}
          onDoubleClick={(event) => {
            event.stopPropagation();
            onStartRename();
          }}
          style={{ touchAction: "pan-y" }}
        >
          <span className="flex shrink-0 text-[var(--toolbar-icon)]">
            {Kind ? <Kind size={16} strokeWidth={1.5} aria-hidden /> : <CompoundGlyph />}
          </span>
          {renaming ? (
            <input
              ref={inputRef}
              defaultValue={row.name}
              aria-label="Name"
              maxLength={128}
              onClick={(event) => event.stopPropagation()}
              onPointerDown={(event) => event.stopPropagation()}
              onKeyDown={(event) => {
                event.stopPropagation();
                if (event.key === "Enter") {
                  event.preventDefault();
                  onRename(event.currentTarget.value);
                } else if (event.key === "Escape") {
                  event.preventDefault();
                  onRename(null);
                }
              }}
              onBlur={(event) => onRename(event.currentTarget.value)}
              className="h-7 min-w-0 flex-1 rounded-[5px] border border-[var(--field-border)] bg-white px-1.5 text-sm text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            />
          ) : dragging ? (
            <span className={nameClass}>{row.name}</span>
          ) : (
            <Tooltip side="left" content={tooltip}>
              <span className={nameClass}>{row.name}</span>
            </Tooltip>
          )}
          {row.containsSelection ? (
            <Tooltip side="left" content="Contains the selection">
              <svg width={6} height={6} viewBox="0 0 6 6" aria-hidden className="shrink-0">
                <path d="M3 0 6 3 3 6 0 3z" fill="var(--editor-accent)" />
              </svg>
            </Tooltip>
          ) : null}
        </span>
      </span>
      {row.readout !== null ? (
        <span className="w-7 shrink-0 text-right text-xs tabular-nums text-[var(--panel-muted-fg)]">
          {row.readout}
        </span>
      ) : null}
      {showAction ? (
        // Its own 24 px slot after the opacity readout, before the eye and the lock (customer
        // feedback 2026-10-14); the slot stays when the button is not shown, so nothing reflows.
        <span role="gridcell" className="flex w-6 shrink-0 items-center">
          <Tooltip side="left" content={actionLabel}>
            <button
              ref={actionRef}
              type="button"
              tabIndex={-1}
              aria-label={actionLabel}
              onPointerDown={(event) => event.stopPropagation()}
              onClick={(event) => {
                event.stopPropagation();
                onAction();
              }}
              onFocus={() => onFocusCell("action")}
              className={`${ICON_BUTTON} text-[var(--toolbar-icon)] ${
                selected || active || (tabbable && cell === "action")
                  ? ""
                  : "invisible group-hover/row:visible group-focus-within/row:visible"
              }`}
            >
              {row.rowAction === "select-original" ? (
                <Locate size={16} strokeWidth={1.5} aria-hidden />
              ) : (
                <ListChecks size={16} strokeWidth={1.5} aria-hidden />
              )}
            </button>
          </Tooltip>
        </span>
      ) : null}
      <span role="gridcell" className="flex shrink-0 items-center">
        <FlagButton
          kind="visible"
          on={!row.visible}
          inherited={row.hiddenBy}
          label={row.visible ? "Hide" : "Show"}
          focused={tabbable && cell === "eye"}
          onPress={() => onFlag("visible")}
          onFocus={() => onFocusCell("eye")}
        />
      </span>
      <span role="gridcell" className="flex shrink-0 items-center">
        <FlagButton
          kind="locked"
          on={row.locked}
          inherited={row.lockedBy}
          label={row.locked ? "Unlock" : "Lock"}
          focused={tabbable && cell === "lock"}
          onPress={() => onFlag("locked")}
          onFocus={() => onFocusCell("lock")}
        />
      </span>
    </div>
  );
}
