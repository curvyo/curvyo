import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type PointerEvent as ReactPointerEvent } from "react";

import { DropBadgePill } from "@/components/DropBadgePill";
import { OperationRowView, type RowActions } from "@/components/panel/OperationRow";
import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { PanelSwitch } from "@/components/ui/switch";
import type { OperationsApi } from "@/hooks/useOperations";
import { useDeleteArm } from "@/hooks/useDeleteArm";
import {
  ADD_LABELS,
  ghostTop,
  insertionEdge,
  listEntries,
  nextFocusIndex,
  neighbourIndex,
  sectionOf,
  sectionRuns,
  summaryLines,
  itemsText,
  type ListEntry,
} from "@/lib/operationRows";
import {
  operationIdKey,
  type CloneMember,
  type CountKey,
  type OperationId,
  type OperationRow,
  type OperationsView,
} from "@/lib/operationsView";
import { visibleRange } from "@/lib/operationWindow";

/** The list's scroll area: 300 px high; the rows outside it (and the overscan) are not drawn. */
const VIEWPORT = 300;
const OVERSCAN = 120;

interface OperationsSectionProps {
  api: OperationsApi;
  /** Items selected on the canvas: the ticks show in the rows while it is above 0. */
  selectionCount: number;
  onReturnFocus: () => void;
}

/** The index of a key in the view's rows, `-1` for a row that is gone. */
/** Whether the row at `index` may move by `step` inside its section: the arrow is hidden where it
 * may not (ux-notes 4.6; a Cut never moves across the section line). */
function canMove(view: OperationsView, index: number, step: -1 | 1): boolean {
  const target = index + step;
  if (target < 0 || target >= view.rows.length) return false;
  if (!view.cutsLast) return true;
  return sectionOf(view.rows[target]) === sectionOf(view.rows[index]);
}

function indexOfKey(view: OperationsView, key: string): number {
  return view.rows.findIndex((row) => operationIdKey(row.id) === key);
}

/** The key of the row an entry list points at for each row index. */
function entryOfRow(entries: ListEntry[]): number[] {
  const map: number[] = [];
  entries.forEach((entry, at) => {
    if (entry.kind === "row") map[entry.index] = at;
  });
  return map;
}

/**
 * The Operations tab of a laser page (`specs/0101-laser-operations` `ux-notes.md`): the add group,
 * the Cuts last switch, the operation list in its sections (windowed), the clone member rows, the
 * View switch, the summary with its count buttons and the notice. Nothing here decides a rule;
 * the session does, and the view is re-read after each call.
 */
/** The names of the two section groups (`ux-notes.md` 4.3). */
const SECTION_NAMES = { marking: "Marking", cutting: "Cutting" } as const;

export function OperationsSection({ api, selectionCount, onReturnFocus }: OperationsSectionProps) {
  const { view, notice, dropKey, dropBadge } = api;
  const [openKey, setOpenKey] = useState<string | null>(null);
  /** The row whose name is being edited in place (criterion 3): one at a time. */
  const [renamingKey, setRenamingKey] = useState<string | null>(null);
  /** The one armed trash of the tab (`0081`): a row's trash by its key. */
  const arm = useDeleteArm();
  /** A reorder drag (ux-notes 4.6): the source row, the start Y and the place it would land. */
  const [dropAt, setDropAt] = useState<number | null>(null);
  /** The row index of an active drag, shown without the drop ground (render-safe copy of `drag`). */
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  /** The ghost (ux-notes 4.6): the pointer's Y, the list's box and the picked row's name. */
  const [ghost, setGhost] = useState<{ y: number; offset: number; left: number; top: number; bottom: number; width: number; label: string } | null>(null);
  const drag = useRef<{ key: string; index: number; startY: number; offset: number; active: boolean } | null>(null);
  // The list changing under an armed trash disarms it (`0081`; ux-notes 4.5): a reorder, a row
  // added or removed, a Cuts last switch.
  const order = view ? view.rows.map((row) => operationIdKey(row.id)).join("|") + (view?.cutsLast ? "+" : "-") : "";
  const disarmArm = arm.disarm;
  useEffect(() => {
    disarmArm();
  }, [disarmArm, order]);

  const [stop, setStop] = useState(0);
  const [addStop, setAddStop] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const addRef = useRef<HTMLDivElement>(null);
  /** A row to focus after the next render: a moved row keeps the focus, a deleted one hands it on. */
  const pending = useRef<number | "add" | null>(null);

  useEffect(() => {
    const target = pending.current;
    pending.current = null;
    if (target === "add") {
      addRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
    } else if (typeof target === "number") {
      focusRow(listRef.current, target);
    }
  });

  const dropAtRef = useRef<number | null>(null);
  useEffect(() => {
    dropAtRef.current = dropAt;
  });

  // The reorder drag (ux-notes 4.6): starts after 4 px, lands inside the section of the row, and is
  // one step on release. Escape cancels it; a release outside the section puts the row back.
  useEffect(() => {
    if (view === null) return;
    const bounds = (index: number) => {
      const section = sectionOf(view.rows[index]);
      let first = index;
      let last = index;
      while (view.cutsLast && first > 0 && sectionOf(view.rows[first - 1]) === section) first -= 1;
      while (view.cutsLast && last < view.rows.length - 1 && sectionOf(view.rows[last + 1]) === section) last += 1;
      return { first, last };
    };
    const landingAt = (clientY: number, source: number) => {
      const list = listRef.current;
      if (!list) return null;
      const rows = Array.from(list.querySelectorAll<HTMLElement>("li[data-row-index]"));
      // A drag down lands on the last row below the source whose midpoint the pointer passed; a drag
      // up lands on the first row above the source whose midpoint the pointer is above.
      let at = source;
      for (const element of rows) {
        const index = Number(element.dataset.rowIndex);
        const box = element.getBoundingClientRect();
        const middle = box.top + box.height / 2;
        if (index > source && clientY >= middle) at = index;
        if (index < source && clientY < middle) at = Math.min(at, index);
      }
      const { first, last } = bounds(source);
      return Math.min(last, Math.max(first, at));
    };
    const onMove = (event: PointerEvent) => {
      const current = drag.current;
      if (!current) return;
      if (!current.active) {
        if (Math.abs(event.clientY - current.startY) < 4) return;
        current.active = true;
        setDragFrom(current.index);
        // The list's box and the picked row's label are read here, in the handler, not in render.
        const box = listRef.current?.getBoundingClientRect();
        setGhost({
          y: event.clientY,
          offset: current.offset,
          left: box?.left ?? 0,
          top: box?.top ?? 0,
          bottom: box?.bottom ?? 0,
          width: box?.width ?? 0,
          label: view.rows[current.index]?.name ?? "",
        });
      }
      setGhost((previous) => (previous ? { ...previous, y: event.clientY } : previous));
      setDropAt(landingAt(event.clientY, current.index));
    };
    const finish = (apply: boolean) => {
      const current = drag.current;
      drag.current = null;
      setDropAt(null);
      setDragFrom(null);
      setGhost(null);
      if (!current || !current.active || !apply || view === null) return;
      const target = dropAtRef.current;
      if (target !== null && target !== current.index) {
        const row = view.rows[current.index];
        if (api.move(row.id, target)) {
          pending.current = target;
          setStop(target);
        }
      }
    };
    const onUp = () => finish(true);
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && drag.current?.active) {
        event.preventDefault();
        event.stopPropagation();
        finish(false);
      }
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("keydown", onKey, true);
    };
  }, [view, api]);

  if (view === null) {
    return null;
  }

  const entries = listEntries(view, openKey);
  const rowEntry = entryOfRow(entries);
  const openIndex = openKey === null ? -1 : indexOfKey(view, openKey);
  const range = visibleRange(
    entries.map((entry) => entry.height),
    scrollTop,
    VIEWPORT,
    OVERSCAN,
    rowEntry[stop] ?? null,
    openIndex >= 0 ? (rowEntry[openIndex] ?? null) : null,
  );

  /** Brings row `index` into the window and focuses it once it is drawn. */
  const goToRow = (index: number) => {
    const list = listRef.current;
    const at = rowEntry[index];
    if (list && at !== undefined) {
      const top = entries.slice(0, at).reduce((sum, entry) => sum + entry.height, 0);
      if (top < list.scrollTop || top + entries[at].height > list.scrollTop + VIEWPORT) {
        list.scrollTop = Math.max(0, top - VIEWPORT / 2);
        setScrollTop(list.scrollTop);
      }
    }
    setStop(index);
    pending.current = index;
  };

  const actions: RowActions = {
    select: (row, rename = false) => {
      setOpenKey(operationIdKey(row.id));
      setStop(indexOfKey(view, operationIdKey(row.id)));
      if (rename) setRenamingKey(operationIdKey(row.id));
    },
    rename: (row, name) => {
      api.rename(row.id, name);
    },
    endRename: (row, refocus) => {
      setRenamingKey(null);
      // Enter and Escape keep the focus on the row (ux-notes 4.4, 15): the row's name is focused once drawn.
      if (refocus) pending.current = indexOfKey(view, operationIdKey(row.id));
    },
    trash: (row) => {
      const key = operationIdKey(row.id);
      const label = `Press again to delete ${row.name}. Its ${itemsText(row.items)} become Not assigned.`;
      arm.press(`op:${key}`, label, () => {
        const index = indexOfKey(view, key);
        const left = view.rows.length - 1;
        if (api.remove(row.id)) {
          setOpenKey(null);
          pending.current = left === 0 ? "add" : Math.min(index, left - 1);
        }
        return `${row.name} deleted.`;
      });
    },
    setShow: (row, on) => {
      api.setShow(row.id, on);
    },
    setOutput: (row, on) => {
      api.setOutput(row.id, on);
    },
    tick: (row, on) => {
      api.tick(row.id, on);
    },
    step: (row, step) => {
      const index = indexOfKey(view, operationIdKey(row.id));
      const to = neighbourIndex(view, index, step);
      if (to !== null && api.move(row.id, to)) {
        pending.current = to;
        setStop(to);
      }
    },
    dragStart: (row, event: ReactPointerEvent<HTMLElement>) => {
      if (event.button !== 0 || view === null) return;
      const row_top = (event.currentTarget as HTMLElement).closest("li")?.getBoundingClientRect().top ?? event.clientY;
      drag.current = {
        key: operationIdKey(row.id),
        index: indexOfKey(view, operationIdKey(row.id)),
        startY: event.clientY,
        offset: event.clientY - row_top,
        active: false,
      };
    },
  };


  const onListKey = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.altKey) return;
    const current = (event.target as HTMLElement).closest<HTMLElement>("li[data-row-index]");
    const index = current ? Number(current.dataset.rowIndex) : stop;
    const next = nextFocusIndex(view.rows.length, index, event.key);
    if (next !== null) {
      event.preventDefault();
      goToRow(next);
    }
  };

  const onAddKey = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>("button"));
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const at = (addStop + step + buttons.length) % buttons.length;
    setAddStop(at);
    buttons[at]?.focus();
  };

  const press = (kind: string) => {
    const id = api.press(kind);
    if (id) setOpenKey(operationIdKey(id));
  };

  const lines = summaryLines(view);
  const openRow = openIndex >= 0 ? view.rows[openIndex] : null;
  return (
    <section aria-label="Operations" className="flex flex-col gap-2">
      <div
        ref={addRef}
        role="toolbar"
        aria-label="Add operation"
        onKeyDown={onAddKey}
        className="flex h-7 items-center gap-1"
      >
        {ADD_LABELS.map(([kind, label], at) => (
          <button
            key={kind}
            type="button"
            aria-label={label}
            title={selectionCount > 0 ? `${label}. Assigns the ${selectionCount} selected items.` : label}
            tabIndex={at === addStop ? 0 : -1}
            onFocus={() => setAddStop(at)}
            onClick={() => press(kind)}
            className="flex h-7 w-7 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          >
            <OperationGlyph kind={kind} />
          </button>
        ))}
      </div>
      <PanelSwitch
        label="Cuts last"
        checked={view.cutsLast}
        onCheckedChange={(on) => api.setCutsLast(on)}
        onReturnFocus={onReturnFocus}
      />
      {view.rows.length === 0 ? (
        <p className="py-2 text-sm text-[var(--panel-muted-fg)]">No operations on this page.</p>
      ) : (
        <div
          ref={listRef}
          aria-label="Operations"
          role={view.cutsLast ? "group" : "list"}
          onKeyDown={onListKey}
          onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
          style={{ maxHeight: VIEWPORT }}
          className="flex flex-col overflow-y-auto"
        >
          <div aria-hidden style={{ height: range.before }} />
          {sectionRuns(entries.slice(range.first, range.last), view.cutsLast).map((run, runIndex) => {
            const rows = run.entries.flatMap((entry) => (entry.kind === "row" ? [entry] : []));
            const header = run.entries.find((entry) => entry.kind === "header");
            const list = (
              <ul role="list" className="flex flex-col">
                {rows.map((entry) => (
                  <OperationRowView
                    key={operationIdKey(entry.row.id)}
                    row={entry.row}
                    index={entry.index}
                    total={view.rows.length}
                    height={entry.height}
                    dropping={dropKey === operationIdKey(entry.row.id)}
                    dragSource={dragFrom === entry.index}
                    insertion={dragFrom !== null && dropAt === entry.index ? insertionEdge(dragFrom, dropAt) : null}
                    open={openKey === operationIdKey(entry.row.id)}
                    renaming={renamingKey === operationIdKey(entry.row.id)}
                    armed={arm.armed === `op:${operationIdKey(entry.row.id)}`}
                    canUp={canMove(view, entry.index, -1)}
                    canDown={canMove(view, entry.index, 1)}
                    tabStop={entry.index === stop}
                    onFocusRow={() => setStop(entry.index)}
                    selecting={selectionCount > 0}
                    actions={actions}
                  />
                ))}
              </ul>
            );
            if (run.section === null) {
              return <div key={`flat-${runIndex}`}>{list}</div>;
            }
            // A section is a named group (`ux-notes.md` 4.3): its header is the group's name, drawn.
            return (
              <div key={`run-${run.section}-${runIndex}`} role="group" aria-label={SECTION_NAMES[run.section]} className="flex flex-col">
                {header?.kind === "header" && (
                  <div
                    aria-hidden
                    style={{ height: header.height }}
                    className="border-t border-[var(--divider,transparent)] pt-2 text-[11px] font-semibold text-[var(--panel-muted-fg)] first:border-t-0"
                  >
                    {SECTION_NAMES[run.section]}
                  </div>
                )}
                {list}
              </div>
            );
          })}
          <div aria-hidden style={{ height: range.after }} />
        </div>
      )}
      <CloneMembers members={view.cloneMembers} view={view} openRow={openRow} api={api} />
      <section aria-label="Operations summary" className="flex flex-col text-xs tabular-nums text-[var(--toolbar-icon)]">
        <div role="status" aria-atomic="true" className="flex flex-col">
          {lines.map((line) =>
            line.count === null ? (
              <p key={line.text} className="leading-4">
                {line.text}
              </p>
            ) : (
              <SummaryButton key={line.text} text={line.text} count={line.count} onPress={api.selectItems} />
            ),
          )}
        </div>
      </section>
      <div role="radiogroup" aria-label="View" className="flex h-7 gap-1 self-start rounded border border-[var(--lane-line)] p-0.5 text-[13px]">
        {(["operations", "drawn"] as const).map((mode) => {
          const checked = view.viewMode === mode;
          return (
            <button
              key={mode}
              type="button"
              role="radio"
              aria-checked={checked}
              tabIndex={checked ? 0 : -1}
              title={mode === "operations" ? "View: Operations" : "View: Drawn"}
              onClick={() => api.setDrawn(mode === "drawn")}
              className={`rounded px-2 ${checked ? "bg-[var(--value-fill)] text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"} focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]`}
            >
              {mode === "operations" ? "Operations" : "Drawn"}
            </button>
          );
        })}
      </div>
      {notice ? (
        <p role="status" className="text-xs text-[var(--toolbar-icon)]">
          {notice}
        </p>
      ) : null}
      {dropBadge && <DropBadgePill badge={dropBadge} position="fixed" />}
      {ghost && (
        <div
          aria-hidden
          data-reorder-ghost=""
          className="pointer-events-none fixed z-50 flex h-10 items-center rounded bg-[var(--value-fill)] px-2 text-[13px] text-[var(--toolbar-icon)] shadow"
          style={{
            left: ghost.left,
            top: ghostTop(ghost.y, ghost.offset, ghost.top, ghost.bottom, 40),
            width: ghost.width,
            opacity: 0.6,
          }}
        >
          {ghost.label}
        </div>
      )}
      <div role="status" aria-atomic="true" className="sr-only">
        {arm.status}
      </div>
    </section>
  );
}

/** A summary line that selects its items on the canvas when pressed (criterion 17). */
function SummaryButton({
  text,
  count,
  onPress,
}: {
  text: string;
  count: CountKey;
  onPress: (count: CountKey) => boolean;
}) {
  return (
    <button
      type="button"
      onClick={() => onPress(count)}
      className="flex h-6 items-center text-left underline-offset-2 hover:underline focus-visible:underline focus-visible:outline-none"
    >
      {text}
    </button>
  );
}

/**
 * The clone member rows (criterion 11, Frage 167 B): read-only, one per member of each clone. A
 * member row takes the open operation: its check holds or drops the member in that operation.
 */
function CloneMembers({
  members,
  view,
  openRow,
  api,
}: {
  members: CloneMember[];
  view: OperationsView;
  openRow: OperationRow | null;
  api: OperationsApi;
}) {
  if (members.length === 0) return null;
  const nameOf = (id: OperationId) =>
    view.rows.find((row) => operationIdKey(row.id) === operationIdKey(id))?.name ?? "";
  return (
    <div role="group" aria-label="Clone members" className="flex flex-col gap-1">
      <p className="text-[11px] font-semibold text-[var(--panel-muted-fg)]">Clone members</p>
      <ul role="list" className="flex flex-col">
        {members.map((member) => {
          const held = openRow !== null && member.operations.some((id) => operationIdKey(id) === operationIdKey(openRow.id));
          const chips = member.operations.map(nameOf).filter((name) => name !== "");
          const label = `${member.name}, in ${member.cloneName}, read-only${chips.length ? `, ${chips.join(", ")}` : ""}`;
          return (
            <li key={`${operationIdKey(member.clone)}/${operationIdKey(member.member)}`} className="flex h-7 items-center gap-2 text-[13px] text-[var(--toolbar-icon)]">
              <button
                type="button"
                role="checkbox"
                aria-checked={held}
                aria-readonly="true"
                aria-disabled={openRow === null}
                aria-label={label}
                title={openRow === null ? "Open an operation to hold this member" : `${held ? "Remove from" : "Assign to"} ${openRow.name}`}
                onClick={() => {
                  if (openRow) setCloneMember(api, openRow.id, member, !held);
                }}
                className="flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border border-[var(--toolbar-icon)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              >
                {held ? <span className="h-2 w-2 bg-[var(--toolbar-icon)]" /> : null}
              </button>
              <span className="min-w-0 flex-1 truncate">
                {member.name}, in {member.cloneName}
              </span>
              <span className="flex max-w-[110px] shrink-0 truncate text-[11px] text-[var(--panel-muted-fg)]">
                {chips.join(", ")}
              </span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function setCloneMember(api: OperationsApi, id: OperationId, member: CloneMember, on: boolean) {
  api.setCloneMember(id, member.clone, member.member, on);
}

/** Moves the focus to the name button of the row at `index`. */
function focusRow(list: HTMLElement | null, index: number) {
  list?.querySelector<HTMLButtonElement>(`li[data-row-index="${index}"] [data-row-name]`)?.focus();
}
