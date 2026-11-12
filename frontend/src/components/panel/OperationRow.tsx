import { ChevronDown, ChevronUp, Eye, EyeOff, Power, PowerOff, Trash2 } from "lucide-react";
import { useEffect, useId, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";

import { ARM_TRASH, ARM_ZONE } from "@/hooks/useDeleteArm";
import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { KIND_WORDS, focusAfterRename, nameKeyAction, rowLine2, rowName, tickPressAdds } from "@/lib/operationRows";
import { canUnfold } from "@/lib/operationUnfold";
import { gridRowLine } from "@/lib/testGridFields";
import type { OperationRow as Row } from "@/lib/operationsView";
import type { PressMods } from "@/lib/rowSelection";

/** What a row asks of the section: the session calls and the row's own states. */
export interface RowActions {
  /** Selects the row; `rename` also opens the name in place (F2, double-click). */
  select: (row: Row, rename?: boolean, mods?: PressMods) => void;
  /** Commits a new name (Enter, or leaving the field); an empty name reverts. */
  rename: (row: Row, name: string) => void;
  /** Leaves the in-place name field; `refocus` puts the focus back on the row (Enter, Escape). */
  endRename: (row: Row, refocus: boolean) => void;
  /** A press on the trash: arms it, or deletes it when armed (`0081`). */
  trash: (row: Row) => void;
  setShow: (row: Row, on: boolean) => void;
  setOutput: (row: Row, on: boolean) => void;
  tick: (row: Row, on: boolean) => void;
  /** Moves the row one place (the arrow buttons and Alt+arrows); the section decides the place. */
  step: (row: Row, step: -1 | 1) => void;
  /** A pointer press on the row's glyph or name starts a reorder drag (ux-notes 4.6). */
  dragStart: (row: Row, event: ReactPointerEvent<HTMLElement>) => void;
}

export interface RowProps {
  row: Row;
  /** The row's fixed height: 40 px (`ux-notes.md` 4.2). */
  height: number;
  /** A canvas drag is over this row: the drop ground and the outline. */
  dropping: boolean;
  /** This row is the one a reorder drag picks up: a dashed outline and no ground (ux-notes 4.6). */
  dragSource: boolean;
  /** The 2 px insertion line of a reorder drag, on the top or bottom edge of this row. */
  insertion: "top" | "bottom" | null;
  /** The row's place in the whole list. */
  index: number;
  /** The number on line 2 (`rowDigit`): test grid rows have none (`specs/0028` UX notes §5). */
  digit: number | null;
  /** The number of operations in the whole list (`aria-setsize`). */
  total: number;
  /** The row is the selected one (`aria-selected`). */
  open: boolean;
  /** The row is unfolded: its body shows under line 2 (`specs/0111` criterion 17). */
  unfolded: boolean;
  /** The body of the unfolded row, drawn under line 2 with the row's accent bar beside it. */
  body?: ReactNode;
  /** The name is being edited in place (criterion 3). */
  renaming: boolean;
  /** The row's trash is armed (`0081`). */
  armed: boolean;
  /** The row may move up or down inside its section (the arrow is hidden where it may not). */
  canUp: boolean;
  canDown: boolean;
  /** The roving focus stop: only this row's name button is in the tab order. */
  tabStop: boolean;
  onFocusRow: () => void;
  /** Items are selected on the canvas: the tick shows in place of the glyph. */
  selecting: boolean;
  actions: RowActions;
}

const CONTROL =
  "flex h-6 w-6 shrink-0 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** The tick for a row while items are selected (`ux-notes.md` 6.2). */
function TickBox({ row, actions }: { row: Row; actions: RowActions }) {
  const checked = row.tick === "all" ? true : row.tick === "some" ? "mixed" : false;
  const partId = useId();
  // A mixed group row: the description says which group runs here in part (`specs/0111` UX 11).
  const part = row.groupInPart === null ? null : `${row.groupInPart} in part`;
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      aria-label={row.name}
      aria-describedby={part === null ? undefined : partId}
      title={tickPressAdds(row.tick) ? `Assign to ${row.name}` : `Remove from ${row.name}`}
      onClick={() => actions.tick(row, tickPressAdds(row.tick))}
      className="flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border border-[var(--toolbar-icon)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
    >
      {row.tick === "all" ? <span className="h-2 w-2 bg-[var(--toolbar-icon)]" /> : null}
      {row.tick === "some" ? <span className="h-0.5 w-2 bg-[var(--toolbar-icon)]" /> : null}
      {part === null ? null : (
        <span id={partId} className="sr-only">
          {part}
        </span>
      )}
    </button>
  );
}

/** One operation's row (`ux-notes.md` section 4.2): line 1 glyph, name, Show, Output, trash, and the
 * arrows; line 2 the digit, the kind and the count. */
export function OperationRowView(props: RowProps) {
  const { row, index, renaming, armed, actions } = props;
  const unknown = row.kind === "unknown";
  const key = `${row.id.peer}:${row.id.counter}`;
  return (
    <li
      data-row-key={key}
      data-row-index={index}
      aria-posinset={index + 1}
      aria-setsize={props.total}
      aria-selected={props.open}
      {...{ [ARM_ZONE]: `op:${key}` }}
      style={{ height: props.height }}
      className={`relative flex flex-col rounded px-1 ${props.unfolded ? "" : "overflow-hidden"} has-[[data-row-name]:focus-visible]:ring-2 has-[[data-row-name]:focus-visible]:ring-inset has-[[data-row-name]:focus-visible]:ring-[var(--editor-accent)] ${props.open ? "bg-[var(--value-fill)]" : ""} ${props.dropping ? "bg-[var(--value-fill)] outline outline-[1.5px] -outline-offset-[1.5px] outline-[var(--editor-accent)]" : ""} ${props.dragSource ? "outline outline-[1.5px] -outline-offset-[1.5px] outline-dashed outline-[var(--drag-ghost)]" : ""}`}
    >
      {props.insertion ? (
        <div
          aria-hidden
          data-insertion={props.insertion}
          className="pointer-events-none absolute left-0 right-0 h-[2px] bg-[var(--editor-accent)]"
          style={props.insertion === "top" ? { top: 0 } : { bottom: 0 }}
        />
      ) : null}
      {props.unfolded ? (
        <div aria-hidden data-row-bar="" className="absolute top-0 bottom-0 left-0 w-[3px] rounded-l bg-[var(--accent)]" />
      ) : null}
      {/* Arming the trash tints line 1 and line 2 only; the body keeps its ground. */}
      <div className={`flex h-10 shrink-0 flex-col rounded ${armed ? "bg-[var(--destructive-soft)]" : ""}`}>
      <div className="flex h-[26px] shrink-0 items-center gap-1" onPointerDown={(event) => {
        if (!renaming && (event.target as HTMLElement).closest("button:not([data-row-name]), input") === null) actions.dragStart(row, event);
      }}>
        {props.selecting && !unknown && !row.grid ? (
          <TickBox row={row} actions={actions} />
        ) : (
          <span
            className="flex h-4 w-4 shrink-0 items-center justify-center text-[var(--toolbar-icon)]"
            onClick={(event) => pressRow(event, row, actions)}
          >
            <OperationGlyph kind={row.kind} />
          </span>
        )}
        {renaming ? (
          <RenameField row={row} actions={actions} />
        ) : (
          <button
            type="button"
            data-row-name
            tabIndex={props.tabStop ? 0 : -1}
            aria-expanded={canUnfold(row) ? props.unfolded : undefined}
            aria-label={rowName(row)}
            title={row.name}
            onFocus={props.onFocusRow}
            onClick={(event) => pressRow(event, row, actions)}
            onDoubleClick={() => actions.select(row, true)}
            onKeyDown={(event) => onNameKey(event, row, armed, actions)}
            className={`min-w-0 flex-1 truncate text-left text-[13px] focus-visible:outline-none ${row.output ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`}
          >
            {row.name}
          </button>
        )}
        {renaming || unknown ? null : (
          <>
            <button
              type="button"
              aria-pressed={row.show}
              aria-label={`Show ${row.name}`}
              title={`Show ${row.name}`}
              onClick={() => actions.setShow(row, !row.show)}
              className={CONTROL}
            >
              {row.show ? <Eye size={16} aria-hidden /> : <EyeOff size={16} aria-hidden />}
            </button>
            <button
              type="button"
              aria-pressed={row.output}
              aria-label={`Output ${row.name}`}
              title={`Output ${row.name}`}
              onClick={() => actions.setOutput(row, !row.output)}
              className={CONTROL}
            >
              {row.output ? <Power size={16} aria-hidden /> : <PowerOff size={16} aria-hidden />}
            </button>
          </>
        )}
        {renaming ? null : <TrashButton row={row} armed={armed} actions={actions} />}
        {renaming || unknown ? null : (
          <>
            <span className="w-1 shrink-0" />
            {props.canUp ? (
              <ArrowButton row={row} up actions={actions} />
            ) : (
              <span className="h-6 w-6 shrink-0" aria-hidden />
            )}
            {props.canDown ? (
              <ArrowButton row={row} up={false} actions={actions} />
            ) : (
              <span className="h-6 w-6 shrink-0" aria-hidden />
            )}
          </>
        )}
      </div>
      <div
        className="block truncate text-[11px] leading-[14px] text-[var(--panel-muted-fg)]"
        title={line2Title(row, props.digit)}
        onClick={(event) => pressRow(event, row, actions)}
      >
        <Line2 row={row} digit={props.digit} />
      </div>
      </div>
      {props.body}
    </li>
  );
}

/** A press on the glyph, name or line 2 (`specs/0111` UX 6.1). The second click of a double-click
 * is left to the rename, so the row stays unfolded after the first. */
function pressRow(event: ReactMouseEvent, row: Row, actions: RowActions) {
  if (event.detail > 1) return;
  actions.select(row, false, { ctrl: event.ctrlKey || event.metaKey, shift: event.shiftKey });
}

/** The tooltip of line 2: the whole line, which an ellipsis may cut (`specs/0111` UX 4.1). */
function line2Title(row: Row, digit: number | null): string | undefined {
  if (row.kind === "unknown" || row.grid) return undefined;
  const line = rowLine2(row, digit);
  return `${line.text}${line.part}${line.tail}`;
}

/** Line 2: the digit, the kind and the count; a mixed group row adds " · Group 3 in part" in
 * semibold ink (`specs/0111` `ux-notes.md` 4.1). */
function Line2({ row, digit }: { row: Row; digit: number | null }) {
  if (row.kind === "unknown") return <>{`${digit} · Unknown operation`}</>;
  if (row.grid) return <>{gridRowLine(KIND_WORDS[row.kind], row.grid.columns, row.grid.rows, row.output)}</>;
  const line = rowLine2(row, digit);
  return (
    <>
      {line.text}
      {line.part ? <span className="font-semibold text-[var(--toolbar-icon)]">{line.part}</span> : null}
      {line.tail}
    </>
  );
}

/** The trash (`0081`): idle, or armed as the filled square; the section's armed store owns the state. */
function TrashButton({ row, armed, actions }: { row: Row; armed: boolean; actions: RowActions }) {
  const key = `${row.id.peer}:${row.id.counter}`;
  return (
    <button
      type="button"
      {...{ [ARM_TRASH]: `op:${key}` }}
      aria-label={armed ? `Press again to delete ${row.name}` : `Delete ${row.name}`}
      title={armed ? "Press again to delete" : "Delete"}
      onClick={() => actions.trash(row)}
      className={
        armed
          ? "flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] bg-[var(--destructive)] text-white"
          : `${CONTROL} hover:bg-[var(--editor-accent-hover)]`
      }
    >
      <Trash2 size={16} aria-hidden />
    </button>
  );
}

/** One of the reorder arrows (`ux-notes.md` 4.6): Up or Down the row by one place. */
function ArrowButton({ row, up, actions }: { row: Row; up: boolean; actions: RowActions }) {
  const Icon = up ? ChevronUp : ChevronDown;
  return (
    <button
      type="button"
      aria-label={`Move ${row.name} ${up ? "up" : "down"}`}
      title={up ? "Move up" : "Move down"}
      onClick={() => actions.step(row, up ? -1 : 1)}
      className={CONTROL}
    >
      <Icon size={16} aria-hidden />
    </button>
  );
}

/** The name in place (criterion 3, ux-notes 4.4): Enter or leaving commits, Escape cancels. */
function RenameField({ row, actions }: { row: Row; actions: RowActions }) {
  const [draft, setDraft] = useState(row.name);
  const field = useRef<HTMLInputElement>(null);
  const done = useRef(false);
  useEffect(() => {
    field.current?.focus();
    field.current?.select();
  }, []);
  const finish = (reason: "enter" | "escape" | "blur") => {
    if (done.current) return;
    done.current = true;
    if (reason !== "escape" && draft.trim() !== "" && draft !== row.name) {
      actions.rename(row, draft);
    }
    actions.endRename(row, focusAfterRename(reason));
  };
  return (
    <input
      ref={field}
      aria-label={`Name of ${row.name}`}
      value={draft}
      maxLength={128}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={() => finish("blur")}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          finish("enter");
        } else if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          setDraft(row.name);
          finish("escape");
        }
      }}
      className="min-w-0 flex-1 rounded border border-[var(--editor-accent)] bg-white px-1 text-[13px] text-[var(--toolbar-icon)]"
    />
  );
}

/** The keys on a row's name button (`nameKeyAction`): the list handles the plain arrows that only
 * move the focus. */
function onNameKey(event: ReactKeyboardEvent, row: Row, armed: boolean, actions: RowActions) {
  const action = nameKeyAction(event.key, event.altKey, armed);
  if (action === "none") return;
  event.preventDefault();
  if (action === "step-up") actions.step(row, -1);
  else if (action === "step-down") actions.step(row, 1);
  else if (action === "rename") actions.select(row, true);
  else actions.trash(row);
}

