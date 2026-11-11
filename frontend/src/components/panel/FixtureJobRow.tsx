import { ChevronDown, ChevronUp, Eye, EyeOff, Power, PowerOff, Trash2 } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent } from "react";

import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { ARM_TRASH, ARM_ZONE } from "@/hooks/useDeleteArm";
import type { JobRow } from "@/lib/fixtureJobsView";
import { armKey, jobAccessibleName, jobLine2 } from "@/lib/fixtureJobRows";
import { focusAfterRename, nameKeyAction } from "@/lib/operationRows";

/** What a job row asks of the section: the session calls and the row's own states. */
export interface JobRowActions {
  /** Selects the job; `rename` also opens the name in place (F2, double-click). */
  select: (row: JobRow, rename?: boolean) => void;
  rename: (row: JobRow, name: string) => void;
  /** Leaves the name field; `refocus` puts the focus back on the row (Enter, Escape). */
  endRename: (row: JobRow, refocus: boolean) => void;
  /** A press on the trash: arms it, or removes the job when armed (`0081`). */
  trash: (row: JobRow) => void;
  setShow: (row: JobRow, on: boolean) => void;
  setOutput: (row: JobRow, on: boolean) => void;
  /** Moves the job one place inside its group (the arrows and Alt+arrows). */
  step: (row: JobRow, step: -1 | 1) => void;
  /** Alt+Home and Alt+End: the job goes to the first or the last place of its group. */
  edge: (row: JobRow, edge: "first" | "last") => void;
  /** A press on the glyph or the name starts a reorder drag inside the group. */
  dragStart: (row: JobRow, event: ReactPointerEvent<HTMLElement>) => void;
}

export interface JobRowProps {
  row: JobRow;
  /** The row's place in the whole section (flat), for `aria-posinset` and the focus. */
  index: number;
  /** The number of jobs in the whole section (`aria-setsize`). */
  total: number;
  /** The job is the selected one: its Parts list is open (`aria-selected`). */
  selected: boolean;
  renaming: boolean;
  armed: boolean;
  canUp: boolean;
  canDown: boolean;
  /** The roving focus stop: only this row's name is in the tab order. */
  tabStop: boolean;
  /** This row is the one a reorder drag picks up. */
  dragSource: boolean;
  insertion: "top" | "bottom" | null;
  onFocusRow: () => void;
  actions: JobRowActions;
  /** The open Parts list, drawn under the row's second line. */
  children?: ReactNode;
}

const CONTROL =
  "flex h-6 w-6 shrink-0 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/**
 * One job's row (`specs/0103-fixture-jobs` `ux-notes.md` section 4): line 1 the glyph, the name,
 * Show, Output, the trash and the arrows; line 2 the kind and the state or the count. An Unknown
 * job keeps only its name and its trash (criterion 21).
 */
export function FixtureJobRowView(props: JobRowProps) {
  const { row, index, renaming, armed, actions } = props;
  const unknown = row.kind === null;
  const key = armKey(row.id);
  return (
    <li
      data-row-key={row.id}
      data-row-index={index}
      data-job-row={row.id}
      aria-posinset={index + 1}
      aria-setsize={props.total}
      {...{ [ARM_ZONE]: key }}
      className={`relative flex flex-col rounded px-1 has-[[data-row-name]:focus-visible]:ring-2 has-[[data-row-name]:focus-visible]:ring-inset has-[[data-row-name]:focus-visible]:ring-[var(--editor-accent)] ${armed ? "bg-[var(--destructive-soft)]" : ""} ${props.dragSource ? "outline outline-[1.5px] -outline-offset-[1.5px] outline-dashed outline-[var(--drag-ghost)]" : ""} ${props.selected ? "bg-[var(--value-fill)] shadow-[inset_3px_0_0_var(--editor-accent)]" : ""}`}
      style={{ height: props.children ? undefined : 40 }}
    >
      {props.insertion ? (
        <div
          aria-hidden
          data-insertion={props.insertion}
          className="pointer-events-none absolute left-0 right-0 h-[2px] bg-[var(--editor-accent)]"
          style={props.insertion === "top" ? { top: 0 } : { bottom: 0 }}
        />
      ) : null}
      <div
        className="flex h-[26px] shrink-0 items-center gap-1"
        onPointerDown={(event) => {
          if (!renaming && (event.target as HTMLElement).closest("button:not([data-row-name]), input") === null) actions.dragStart(row, event);
        }}
      >
        <span className="flex h-4 w-4 shrink-0 items-center justify-center text-[var(--toolbar-icon)]">
          <OperationGlyph kind={row.kind?.text ?? "unknown"} />
        </span>
        {renaming ? (
          <RenameField row={row} actions={actions} />
        ) : (
          <button
            type="button"
            data-row-name
            tabIndex={props.tabStop ? 0 : -1}
            aria-expanded={unknown ? undefined : props.selected}
            aria-label={jobAccessibleName(row)}
            title={row.name}
            onFocus={props.onFocusRow}
            onClick={() => actions.select(row)}
            onDoubleClick={() => actions.select(row, true)}
            onKeyDown={(event) => onNameKey(event, row, armed, actions)}
            className={`min-w-0 flex-1 truncate text-left text-[13px] focus-visible:outline-none ${row.show && row.output && !unknown ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`}
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
            {props.canUp ? <ArrowButton row={row} up actions={actions} /> : <span className="h-6 w-6 shrink-0" aria-hidden />}
            {props.canDown ? <ArrowButton row={row} up={false} actions={actions} /> : <span className="h-6 w-6 shrink-0" aria-hidden />}
          </>
        )}
      </div>
      <div className="block truncate text-[11px] leading-[14px] text-[var(--panel-muted-fg)]">{jobLine2(row)}</div>
      {props.children}
    </li>
  );
}

/** The trash (`0081`): idle, or armed as the filled square; the section's arm store owns the state. */
function TrashButton({ row, armed, actions }: { row: JobRow; armed: boolean; actions: JobRowActions }) {
  return (
    <button
      type="button"
      {...{ [ARM_TRASH]: armKey(row.id) }}
      aria-label={armed ? `Press again to delete ${row.name}` : `Delete ${row.name}`}
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

/** One arrow: Up or Down the job by one place inside its group. */
function ArrowButton({ row, up, actions }: { row: JobRow; up: boolean; actions: JobRowActions }) {
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

/** The name in place (criterion 4): Enter or leaving commits, Escape cancels; 128 characters. */
function RenameField({ row, actions }: { row: JobRow; actions: JobRowActions }) {
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

/** The keys on a job's name (`nameKeyAction`): Alt+arrows move it, F2 renames, Delete arms. */
function onNameKey(event: ReactKeyboardEvent, row: JobRow, armed: boolean, actions: JobRowActions) {
  if (row.kind === null && event.key !== "Delete" && event.key !== "Enter" && event.key !== "F2") return;
  if (event.altKey && row.kind !== null && (event.key === "Home" || event.key === "End")) {
    event.preventDefault();
    actions.edge(row, event.key === "Home" ? "first" : "last");
    return;
  }
  const action = nameKeyAction(event.key, event.altKey, armed);
  if (action === "none") return;
  event.preventDefault();
  if (action === "step-up") actions.step(row, -1);
  else if (action === "step-down") actions.step(row, 1);
  else if (action === "rename") actions.select(row, true);
  else actions.trash(row);
}
