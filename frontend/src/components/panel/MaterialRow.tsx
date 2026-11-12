import { ChevronDown, ChevronUp, Trash2, TriangleAlert } from "lucide-react";
import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from "react";

import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { ARM_TRASH, ARM_ZONE } from "@/hooks/useDeleteArm";
import { hasTrash, line2Text, materialKeyAction, materialRowName, rowTitle } from "@/lib/materialRows";
import type { MaterialRow as Row } from "@/lib/materialsView";
import { thicknessText } from "@/lib/operationSettingsRows";
import { focusAfterRename } from "@/lib/operationRows";

/** What a row asks of the section. */
export interface MaterialRowActions {
  /** A press on the row selects it (a readable one opens its fields). */
  select: (row: Row, rename?: boolean) => void;
  rename: (row: Row, name: string) => void;
  endRename: (row: Row, refocus: boolean) => void;
  trash: (row: Row) => void;
  /** The arrow buttons and Alt+arrows, Home and End: the key (or "ArrowUp" and "ArrowDown" for the buttons). */
  moveKey: (row: Row, key: string) => void;
  dragStart: (row: Row, event: ReactPointerEvent<HTMLElement>) => void;
}

interface MaterialRowProps {
  row: Row;
  index: number;
  total: number;
  selected: boolean;
  renaming: boolean;
  armed: boolean;
  canUp: boolean;
  canDown: boolean;
  tabStop: boolean;
  dropping: "top" | "bottom" | null;
  dragSource: boolean;
  actions: MaterialRowActions;
  onFocusRow: () => void;
  /** The selected material's fields, under the row, inside the same item. */
  children?: ReactNode;
}

const CONTROL =
  "flex h-6 w-6 shrink-0 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** The id attribute the section finds a row by. */
export const MATERIAL_ROW = "data-material-id";

/** The kind symbols of a loaded row (Cut red, the others ink). */
function Symbols({ row }: { row: Row }) {
  if (row.state !== "loaded") return null;
  if (row.kinds.length === 0) return <span>no sets</span>;
  return (
    <span className="flex gap-1" aria-hidden>
      {row.kinds.map((kind) => (
        <OperationGlyph key={kind} kind={kind} size={12} />
      ))}
    </span>
  );
}

/**
 * One row of the Materials section (`specs/0028` `ux-notes.md` 10.2, 10.4): line 1 the name (in
 * place rename), the armed trash and the arrows; line 2 the thickness and the kind symbols. A
 * damaged row shows the warning symbol and its reason, a newer one "Needs a newer Curvyo" and no
 * trash. The selected material's fields sit under the row.
 */
export function MaterialRowView(props: MaterialRowProps) {
  const { row, index, total, selected, renaming, armed, actions } = props;
  const readable = row.state === "loaded";
  return (
    <li
      {...{ [MATERIAL_ROW]: row.id }}
      data-material-index={index}
      aria-posinset={index + 1}
      aria-setsize={total}
      aria-selected={selected}
      {...{ [ARM_ZONE]: `material:${row.id}` }}
      className={`relative flex flex-col rounded px-1 ${selected ? "bg-[var(--value-fill)]" : ""} ${armed ? "bg-[var(--destructive-soft)]" : ""} ${props.dragSource ? "outline outline-[1.5px] -outline-offset-[1.5px] outline-dashed outline-[var(--drag-ghost)]" : ""}`}
    >
      {selected ? <span aria-hidden className="absolute left-0 top-0 h-10 w-[3px] rounded bg-[var(--accent)]" /> : null}
      {props.dropping ? (
        <div
          aria-hidden
          data-insertion={props.dropping}
          className="pointer-events-none absolute left-0 right-0 h-[2px] bg-[var(--editor-accent)]"
          style={props.dropping === "top" ? { top: 0 } : { bottom: 0 }}
        />
      ) : null}
      <div className="flex h-10 flex-col">
        <div
          className="flex h-[26px] shrink-0 items-center gap-1"
          onPointerDown={(event) => {
            if (!renaming && (event.target as HTMLElement).closest("button:not([data-row-name]), input") === null) {
              actions.dragStart(row, event);
            }
          }}
        >
          {row.state === "damaged" ? <TriangleAlert size={16} aria-hidden className="shrink-0 text-[var(--toolbar-icon)]" /> : null}
          {renaming && readable ? (
            <RenameField row={row} actions={actions} />
          ) : (
            <button
              type="button"
              data-row-name
              tabIndex={props.tabStop ? 0 : -1}
              aria-expanded={readable ? selected : undefined}
              aria-label={materialRowName(row)}
              title={rowTitle(row)}
              onFocus={props.onFocusRow}
              onClick={() => actions.select(row)}
              onDoubleClick={() => readable && actions.select(row, true)}
              onKeyDown={(event) => onNameKey(event, row, armed, actions)}
              className={`min-w-0 flex-1 truncate text-left text-[13px] focus-visible:outline-none ${readable ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`}
            >
              {rowTitle(row)}
            </button>
          )}
          {renaming ? null : (
            <>
              {hasTrash(row) ? <TrashButton row={row} armed={armed} actions={actions} /> : <span className="h-6 w-6 shrink-0" aria-hidden />}
              <span className="w-1 shrink-0" />
              {props.canUp ? <ArrowButton row={row} up actions={actions} /> : <span className="h-6 w-6 shrink-0" aria-hidden />}
              {props.canDown ? <ArrowButton row={row} up={false} actions={actions} /> : <span className="h-6 w-6 shrink-0" aria-hidden />}
            </>
          )}
        </div>
        <div className="flex h-[14px] items-center gap-1 truncate text-[11px] leading-[14px] text-[var(--panel-muted-fg)]" title={line2Text(row)}>
          {row.state === "loaded" ? (
            <>
              <span className="tabular-nums">{thicknessText(row.thicknessMm)}</span>
              <span aria-hidden>·</span>
              <Symbols row={row} />
            </>
          ) : (
            <span className="truncate">{line2Text(row)}</span>
          )}
        </div>
      </div>
      {props.children}
    </li>
  );
}

function TrashButton({ row, armed, actions }: { row: Row; armed: boolean; actions: MaterialRowActions }) {
  const name = rowTitle(row);
  return (
    <button
      type="button"
      {...{ [ARM_TRASH]: `material:${row.id}` }}
      aria-label={armed ? `Press again to delete ${name}` : `Delete ${name}`}
      title={armed ? "Press again to delete" : "Delete"}
      onClick={() => actions.trash(row)}
      className={
        armed
          ? "flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] bg-[var(--destructive)] text-white"
          : CONTROL
      }
    >
      <Trash2 size={16} aria-hidden />
    </button>
  );
}

function ArrowButton({ row, up, actions }: { row: Row; up: boolean; actions: MaterialRowActions }) {
  const Icon = up ? ChevronUp : ChevronDown;
  return (
    <button
      type="button"
      aria-label={`Move ${rowTitle(row)} ${up ? "up" : "down"}`}
      title={up ? "Move up" : "Move down"}
      onClick={() => actions.moveKey(row, up ? "ArrowUp" : "ArrowDown")}
      className={CONTROL}
    >
      <Icon size={16} aria-hidden />
    </button>
  );
}

/** The name in place (ux-notes 10.2): Enter or leaving commits, Escape cancels, empty reverts. */
function RenameField({ row, actions }: { row: Row; actions: MaterialRowActions }) {
  const current = rowTitle(row);
  const [draft, setDraft] = useState(current);
  const field = useRef<HTMLInputElement>(null);
  const done = useRef(false);
  useEffect(() => {
    field.current?.focus();
    field.current?.select();
  }, []);
  const finish = (reason: "enter" | "escape" | "blur") => {
    if (done.current) return;
    done.current = true;
    if (reason !== "escape" && draft.trim() !== "" && draft !== current) actions.rename(row, draft);
    actions.endRename(row, focusAfterRename(reason));
  };
  return (
    <input
      ref={field}
      aria-label={`Name of ${current}`}
      value={draft}
      maxLength={60}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={() => finish("blur")}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          finish("enter");
        } else if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          setDraft(current);
          finish("escape");
        }
      }}
      className="min-w-0 flex-1 rounded border border-[var(--editor-accent)] bg-white px-1 text-[13px] text-[var(--toolbar-icon)]"
    />
  );
}

function onNameKey(event: ReactKeyboardEvent, row: Row, armed: boolean, actions: MaterialRowActions) {
  const action = materialKeyAction(event.key, event.altKey, armed);
  if (action === "none") return;
  if (action === "open") {
    event.preventDefault();
    actions.select(row);
    return;
  }
  event.preventDefault();
  if (action === "move") actions.moveKey(row, event.key);
  else if (action === "rename") {
    if (row.state === "loaded") actions.select(row, true);
  } else if (hasTrash(row)) actions.trash(row);
}
