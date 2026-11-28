import { ArrowDown, ArrowUp, CircleAlert, GripVertical, ListRestart, LoaderCircle, SkipForward, Trash2, TriangleAlert } from "lucide-react";
import { GLYPH as ICON_GLYPH } from "@/components/ui/looks";
import { type KeyboardEvent, type PointerEvent as ReactPointer, useEffect, useId, useRef, useState } from "react";

import { ARMED_DESTRUCTIVE_BUTTON, GLYPH, ICON_BUTTON, SMALL } from "@/components/workbench/machineStyles";
import { SymbolButton } from "@/components/workbench/SymbolButton";
import { Tooltip } from "@/components/ui/tooltip";
import { RestartConfirm } from "@/components/panel/RestartConfirm";
import { ARM_TRASH, ARM_ZONE, useDeleteArm } from "@/hooks/useDeleteArm";
import { type MachineControlsModel } from "@/hooks/useMachineControls";
import {
  HOLD_MS,
  type ListEntry,
  addedAnnouncement,
  armHolds,
  confirmHolds,
  countsLine,
  dragDecision,
  dropGap,
  dropPlace,
  focusAfterRemoval,
  tabStop,
  interruptedLine,
  keepSelection,
  moveCause,
  movedAnnouncement,
  moveTo,
  ordered,
  restartAt,
  restartLabel,
  restartName,
  rights,
  rowText,
  showsSendingRow,
} from "@/lib/jobList";
import { restartCause } from "@/lib/machineControls";
import { ignoredKey } from "@/lib/sendArm";
import { AlertLine } from "@/components/ui/notice";

// The pressed toggle keeps its ground under the pointer: the hover ground would hide it.
const ON_BUTTON = ICON_BUTTON.replace(
  "hover:bg-[var(--editor-accent-hover)]",
  "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]",
);

/**
 * The job list of the Machine controls footer (`specs/0132-connector-job-queue` UX notes): the
 * jobs behind the current one, an interrupted job first. An owner drags or presses Move up and
 * Move down; a member sees the list and acts on its own jobs. The row moves when the connector's
 * list arrives, never before.
 */
export function JobList({ model }: { model: MachineControlsModel }) {
  const { entry, listPress, listChanged } = model;
  const list = entry.jobList;
  const me = entry.session?.clientName ?? null;
  const owner = entry.session?.role === "Owner";
  const rows = list?.waiting ?? [];
  const order = ordered(list);
  const [selected, setSelected] = useState<string | null>(null);
  const [focusedRow, setFocusedRow] = useState<string | null>(null);
  const before = useRef(list);
  const arm = useDeleteArm();
  const pending = useRef(false);
  const box = useRef<HTMLDivElement>(null);
  const [gap, setGap] = useState<number | null>(null);
  const dragging = useRef<{ id: string; from: number; x: number; y: number; moved: boolean; type: string; held: boolean } | null>(null);
  const [added, setAdded] = useState("");
  // A move of this app waits for the list that shows it (announced); a removal for the list that
  // drops its row (the focus moves on).
  const moved = useRef<string | null>(null);
  const removal = useRef<{ id: string; before: ListEntry[]; focused: boolean } | null>(null);
  // This app's send has been under way since the last list: that list is its result.
  const sent = useRef(false);
  // The open restart confirm and the line it names; the restart waiting for its answer (W69, W71).
  const [confirm, setConfirm] = useState<{ id: string; line: number } | null>(null);
  const [restarting, setRestarting] = useState<string | null>(null);
  const focusRestart = (id: string) =>
    requestAnimationFrame(() => box.current?.querySelector<HTMLElement>(`[data-row="${id}"] [data-restart]`)?.focus());
  useEffect(() => {
    if (confirm === null) return;
    const row = (list?.waiting ?? []).find((e) => e.job_id === confirm.id);
    if (!confirmHolds(row, confirm.line, row === undefined ? null : restartCause(entry, row))) {
      setConfirm(null);
      focusRestart(confirm.id);
    }
  }, [list, entry.status, entry.lost]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => {
    if (entry.sendingName !== null) sent.current = true;
  }, [entry.sendingName]);

  useEffect(() => {
    const line = (moved.current !== null ? movedAnnouncement(list, moved.current) : null) ?? addedAnnouncement(before.current, list, me, sent.current);
    moved.current = null;
    if (line !== null) setAdded(line);
    if (arm.armed !== null && !armHolds(before.current, list, arm.armed)) arm.disarm();
    const gone = removal.current;
    if (gone !== null && !(list?.waiting ?? []).some((e) => e.job_id === gone.id)) {
      removal.current = null;
      const next = focusAfterRemoval(gone.before, list?.waiting ?? [], gone.id);
      if (gone.focused) {
        const target =
          next !== null
            ? box.current?.querySelector<HTMLElement>(`[data-row="${next}"]`)
            : document.querySelector<HTMLElement>('[role="toolbar"][aria-label="Machine"] [tabindex="0"]');
        target?.focus();
      }
    }
    sent.current = false;
    setSelected((now) => keepSelection(before.current, list, now ?? order[0]?.job_id ?? null));
    before.current = list;
    pending.current = false;
  }, [list]); // eslint-disable-line react-hooks/exhaustive-deps

  const sending = entry.sendingName !== null && showsSendingRow(entry.status) ? entry.sendingName : null;
  if (rows.length === 0 && !list?.note && sending === null) return null;

  const press = (call: Parameters<typeof listPress>[0]) => {
    if (pending.current) return;
    pending.current = true;
    if (call.kind === "move") moved.current = call.jobId;
    void listPress(call).finally(() => {
      pending.current = false;
    });
  };
  const move = (how: "up" | "down" | "first" | "last") => {
    if (selected === null) return;
    const to = moveTo(order, selected, how);
    const from = order.findIndex((e) => e.job_id === selected) + 1;
    if (to !== null) press({ kind: "move", jobId: selected, from, to });
  };
  const remove = (row: ListEntry) =>
    arm.press(row.job_id, `Press again to remove ${row.name}`, () => {
      const element = box.current?.querySelector(`[data-row="${row.job_id}"]`);
      removal.current = { id: row.job_id, before: rows, focused: (element?.contains(document.activeElement) ?? false) || document.activeElement === document.body || document.activeElement === null };
      press({ kind: "remove", jobId: row.job_id });
      return "";
    });

  const cancelRestart = () => {
    if (confirm === null) return;
    const id = confirm.id;
    setConfirm(null);
    focusRestart(id);
  };
  const confirmRestart = (row: ListEntry) => {
    // The press reads the last status again: a cause that applies by now sends nothing (W69).
    if (restartCause(entry, row) !== null || pending.current) return cancelRestart();
    setConfirm(null);
    setRestarting(row.job_id);
    pending.current = true;
    void listPress({ kind: "restart", jobId: row.job_id, line: restartAt(row) }).then((kind) => {
      pending.current = false;
      setRestarting(null);
      // Accepted, the head moves next: the focus goes to Stop (W72); refused, it stays on Restart.
      if (kind === "done") {
        document.querySelector<HTMLElement>('[role="toolbar"][aria-label="Machine"] button[aria-label="Stop"]')?.focus();
      } else focusRestart(row.job_id);
    });
  };
  const confirmRow = confirm === null ? undefined : rows.find((e) => e.job_id === confirm.id);

  const rowKeys = (event: KeyboardEvent, row: ListEntry, at: number) => {
    const can = rights(owner, me, row);
    if (event.altKey && owner) {
      const how = { ArrowUp: "up", ArrowDown: "down", Home: "first", End: "last" }[event.key] as "up" | "down" | "first" | "last" | undefined;
      if (how !== undefined) {
        event.preventDefault();
        move(how);
      }
      return;
    }
    const go = (to: number) => {
      event.preventDefault();
      const target = rows[Math.max(0, Math.min(rows.length - 1, to))];
      if (target.interrupted_at == null) setSelected(target.job_id);
      box.current?.querySelector<HTMLElement>(`[data-row="${target.job_id}"]`)?.focus();
    };
    if (event.key === "ArrowUp") go(at - 1);
    else if (event.key === "ArrowDown") go(at + 1);
    else if (event.key === "Home") go(0);
    else if (event.key === "End") go(rows.length - 1);
    else if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
      const cells = [...(event.currentTarget.querySelectorAll<HTMLElement>("button")) ];
      const index = cells.indexOf(document.activeElement as HTMLElement);
      const next = event.key === "ArrowRight" ? index + 1 : index - 1;
      if (next >= 0 && next < cells.length) {
        event.preventDefault();
        cells[next].focus();
      } else if (next < 0) {
        event.preventDefault();
        (event.currentTarget as HTMLElement).focus();
      }
    } else if (event.key === "Enter" && can.remove && arm.armed === row.job_id && event.target === event.currentTarget) {
      event.preventDefault();
      remove(row);
    } else if (event.key === "Delete" && can.remove) {
      event.preventDefault();
      remove(row);
    } else if (event.key === "Escape" && arm.armed !== null) {
      arm.disarm();
    }
  };

  const startDrag = (event: ReactPointer, row: ListEntry) => {
    event.preventDefault();
    const hold =
      event.pointerType === "touch"
        ? setTimeout(() => {
            if (dragging.current !== null) dragging.current.held = true;
          }, HOLD_MS)
        : undefined;
    dragging.current = { id: row.job_id, from: row.position, x: event.clientX, y: event.clientY, moved: false, type: event.pointerType, held: false };
    const middles = () =>
      [...(box.current?.querySelectorAll<HTMLElement>("[data-order]") ?? [])].map((el) => {
        const r = el.getBoundingClientRect();
        return r.top + r.height / 2;
      });
    const onMove = (e: PointerEvent) => {
      const d = dragging.current;
      if (d === null) return;
      if (!d.moved) {
        const decision = dragDecision(d.type, d.held ? HOLD_MS : 0, Math.hypot(e.clientX - d.x, e.clientY - d.y));
        if (decision === "cancel") return end(null, false);
        if (decision === "wait") return;
      }
      d.moved = true;
      const area = box.current?.getBoundingClientRect();
      const inside = area !== undefined && e.clientY >= area.top - 16 && e.clientY <= area.bottom + 16;
      if (box.current && area && inside) {
        if (e.clientY < area.top + 16) box.current.scrollTop -= 8;
        else if (e.clientY > area.bottom - 16) box.current.scrollTop += 8;
      }
      setGap(inside ? dropGap(middles(), e.clientY) : null);
    };
    const end = (e: PointerEvent | null, drop: boolean) => {
      const d = dragging.current;
      dragging.current = null;
      if (hold !== undefined) clearTimeout(hold);
      document.removeEventListener("pointermove", onMove);
      document.removeEventListener("pointerup", onUp);
      document.removeEventListener("keydown", onKey, true);
      const target = d?.moved && drop && e !== null ? dropGap(middles(), e.clientY) : null;
      setGap(null);
      const place = d !== null && target !== null ? dropPlace(d.from, target) : null;
      if (d !== null && place !== null) press({ kind: "move", jobId: d.id, from: d.from, to: place });
    };
    const onUp = (e: PointerEvent) => {
      const area = box.current?.getBoundingClientRect();
      const inside = area !== undefined && e.clientY >= area.top && e.clientY <= area.bottom;
      end(e, inside);
    };
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        end(null, false);
      }
    };
    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
    document.addEventListener("keydown", onKey, true);
  };

  const first = order.length > 0 ? order[0].job_id : null;
  const tabTarget = tabStop(rows, selected, first, focusedRow);
  const upCause = moveCause(order, selected, "up");
  const downCause = moveCause(order, selected, "down");
  return (
    <div className="flex flex-col gap-1">
      <div className="flex h-7 items-center justify-between text-xs">
        <span className="text-[var(--panel-muted-fg)]">Job list</span>
        {owner && rows.length > 0 ? (
          <div role="toolbar" aria-label="Job order" className="flex">
            <SymbolButton name="Move up" enabled={upCause === null} cause={upCause} side="top" onPress={() => move("up")}>
              <ArrowUp {...GLYPH} />
            </SymbolButton>
            <SymbolButton name="Move down" enabled={downCause === null} cause={downCause} side="top" onPress={() => move("down")}>
              <ArrowDown {...GLYPH} />
            </SymbolButton>
          </div>
        ) : null}
      </div>
      {list?.note ? (
        <p role="status" className="flex items-start gap-1 text-xs text-[var(--toolbar-icon)]">
          <TriangleAlert {...SMALL} className="mt-0.5 shrink-0" />
          <span>{list.note}</span>
        </p>
      ) : null}
      {listChanged ? (
        <AlertLine>
          <span>The list changed</span>
        </AlertLine>
      ) : null}
      {confirmRow !== undefined ? (
        <RestartConfirm key={confirmRow.job_id} row={confirmRow} onCancel={cancelRestart} onConfirm={() => confirmRestart(confirmRow)} />
      ) : null}
      <div
        ref={box}
        role="grid"
        aria-label="Job list"
        className={`relative flex flex-col overflow-y-auto overflow-x-hidden ${confirmRow !== undefined ? "max-h-[80px]" : "max-h-[164px]"}`}
      >
        {rows.map((row, at) => {
          if (row.job_id === confirmRow?.job_id) return null;
          const can = rights(owner, me, row);
          const interrupted = row.interrupted_at != null;
          const isSelected = !interrupted && row.job_id === selected;
          const armed = arm.armed === row.job_id;
          const index = interrupted ? -1 : row.position - 1;
          return (
            <div
              key={row.job_id}
              role="row"
              aria-rowindex={at + 1}
              aria-selected={interrupted ? undefined : isSelected}
              aria-label={rowText(row, order.length)}
              data-row={row.job_id}
              {...(interrupted ? {} : { "data-order": index })}
              {...{ [ARM_ZONE]: row.job_id }}
              tabIndex={row.job_id === tabTarget ? 0 : -1}
              onFocus={() => setFocusedRow(row.job_id)}
              onBlur={(event) => {
                if (!event.currentTarget.contains(event.relatedTarget)) setFocusedRow(null);
              }}
              onKeyDown={(event) => rowKeys(event, row, at)}
              onPointerDown={(event) => {
                if ((event.target as Element).closest("button") === null && !interrupted) setSelected(row.job_id);
              }}
              className={`relative flex shrink-0 items-center gap-1 pl-1 outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
                interrupted ? "h-14 pointer-coarse:h-[60px]" : "h-10 pointer-coarse:h-11"
              } ${
                armed ? "bg-[var(--destructive-soft)]" : isSelected ? "bg-[var(--value-fill)]" : "hover:bg-[var(--editor-accent-hover)]"
              }`}
            >
              {isSelected ? <span aria-hidden className="absolute inset-y-0 left-0 w-[3px] bg-[var(--accent)]" /> : null}
              {gap !== null && !interrupted && index === gap ? (
                <span aria-hidden className="pointer-events-none absolute inset-x-0 top-0 h-0.5 bg-[var(--accent)]" />
              ) : null}
              {can.move ? (
                <span
                  aria-hidden
                  className="flex h-7 w-4 shrink-0 cursor-grab touch-none items-center text-[var(--panel-muted-fg)]"
                  onPointerDown={(event) => startDrag(event, row)}
                >
                  <GripVertical {...ICON_GLYPH} />
                </span>
              ) : owner ? (
                <span className="w-4 shrink-0" />
              ) : null}
              <div className={`flex min-w-0 flex-1 flex-col text-xs leading-[14px] ${owner ? "max-w-[160px]" : ""}`}>
                <RowName name={row.name} />
                {interrupted ? (
                  <span className="flex items-center gap-1 truncate">
                    <CircleAlert {...SMALL} className="shrink-0 text-[var(--destructive)]" />
                    <span className="truncate text-[var(--destructive)]">{interruptedLine(row)}</span>
                  </span>
                ) : null}
                <span className="truncate text-[var(--panel-muted-fg)]">{countsLine(row, me)}</span>
              </div>
              {can.restart ? (
                <RestartButton
                  row={row}
                  cause={restartCause(entry, row)}
                  busy={restarting === row.job_id}
                  onPress={() => setConfirm({ id: row.job_id, line: restartAt(row) })}
                />
              ) : !interrupted && can.flag ? (
                <Tooltip side="top" content="Run next automatically">
                  <button
                    type="button"
                    tabIndex={-1}
                    aria-label={`Run ${row.name} next automatically`}
                    aria-pressed={row.run_next === true}
                    className={row.run_next ? ON_BUTTON : ICON_BUTTON}
                    onClick={() => press({ kind: "runNext", jobId: row.job_id, on: row.run_next !== true })}
                  >
                    <SkipForward {...GLYPH} />
                  </button>
                </Tooltip>
              ) : !interrupted && row.run_next ? (
                <Tooltip side="top" content="Run next automatically">
                  <span className="flex h-7 w-7 shrink-0 items-center justify-center text-[var(--toolbar-icon)]">
                    <SkipForward {...GLYPH} />
                    <span className="sr-only">Runs next automatically</span>
                  </span>
                </Tooltip>
              ) : can.remove ? (
                // Keeps Remove in the column of the other rows' Remove.
                <span className="w-7 shrink-0 pointer-coarse:w-9" />
              ) : null}
              {can.remove ? (
                <Tooltip side="top" content={armed ? `Press again to remove ${row.name}` : `Remove ${row.name}`}>
                  <button
                    type="button"
                    tabIndex={-1}
                    {...{ [ARM_TRASH]: row.job_id }}
                    aria-label={armed ? `Press again to remove ${row.name}` : `Remove ${row.name}`}
                    className={armed ? ARMED_DESTRUCTIVE_BUTTON : ICON_BUTTON}
                    onClick={() => remove(row)}
                  >
                    <Trash2 {...GLYPH} />
                  </button>
                </Tooltip>
              ) : null}
            </div>
          );
        })}
        {sending !== null ? (
          <div role="row" aria-rowindex={rows.length + 1} className="flex h-10 shrink-0 items-center gap-1 pl-1 text-xs leading-[14px]">
            {owner ? <span className="w-4 shrink-0" /> : null}
            <div className="flex min-w-0 flex-1 flex-col">
              <RowName name={sending} />
              <span className="flex items-center gap-1 text-[var(--panel-muted-fg)]">
                <LoaderCircle {...SMALL} className="animate-spin" />
                Sending
              </span>
            </div>
          </div>
        ) : null}
        {gap !== null && gap === order.length ? (
          <span aria-hidden className="pointer-events-none absolute inset-x-0 bottom-0 h-0.5 bg-[var(--accent)]" />
        ) : null}
      </div>
      <p role="status" className="sr-only">
        {arm.status}
      </p>
      <p role="status" className="sr-only">
        {added}
      </p>
    </div>
  );
}

/** Restart from line in the interrupted row (`specs/0118` W67, W68, W71): greyed in place with its
 * cause, busy until the connector's answer. */
function RestartButton({ row, cause, busy, onPress }: { row: ListEntry; cause: string | null; busy: boolean; onPress: () => void }) {
  const label = restartLabel(row);
  const enabled = cause === null && !busy;
  const causeId = useId();
  const tip =
    cause === null ? (
      label
    ) : (
      <>
        <div>{label}</div>
        <div className="text-muted-foreground">{cause}</div>
      </>
    );
  return (
    <Tooltip side="top" content={tip}>
      <button
        type="button"
        tabIndex={-1}
        data-restart
        aria-label={restartName(row)}
        aria-disabled={!enabled || undefined}
        aria-busy={busy || undefined}
        aria-describedby={cause === null ? undefined : causeId}
        className={`${ICON_BUTTON} aria-disabled:opacity-40 aria-disabled:hover:bg-transparent`}
        onKeyDown={(event) => {
          if (ignoredKey(event.key, event.repeat)) event.preventDefault();
        }}
        onClick={() => {
          if (enabled) onPress();
        }}
      >
        {busy ? <LoaderCircle {...GLYPH} className="animate-spin" /> : <ListRestart {...GLYPH} />}
        {cause === null ? null : (
          <span id={causeId} className="sr-only">
            {cause}
          </span>
        )}
      </button>
    </Tooltip>
  );
}

function RowName({ name }: { name: string }) {
  const dot = name.lastIndexOf(".");
  const [stem, extension] = dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ""];
  return (
    <Tooltip side="left" content={name}>
      <span className="flex min-w-0 text-[var(--toolbar-icon)]">
        <span className="truncate">{stem}</span>
        <span className="shrink-0">{extension}</span>
      </span>
    </Tooltip>
  );
}
