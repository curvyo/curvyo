import { Eraser, File, Minus, Pencil, Plus, Redo2, Replace, Undo2 } from "lucide-react";
import {
  type ComponentType,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { EditorSession } from "@/hooks/useEditorSession";
import { useHistoryList } from "@/hooks/useHistoryList";
import {
  type HistoryRowData,
  NOW_ROW_HEIGHT_PX,
  ROW_HEIGHT_PX,
  SAVED_IN_FILE_LINE,
  WIPE_ARM_MS,
  WIPE_SHARED_LINE,
  wipeQuestion,
  nextFocus,
  rowAriaName,
  rowTooltip,
  scrollAfterInsert,
  secondLineRest,
  visibleRange,
} from "@/lib/historyRows";

type Glyph = ComponentType<{ size: number; strokeWidth: number; "aria-hidden": boolean }>;

/** The glyph of each kind of step (criterion 32): a shape, never a colour. */
const KIND_GLYPHS: Record<string, Glyph> = {
  create: Plus,
  delete: Minus,
  replace: Replace,
  change: Pencil,
  document: File,
};

interface HistoryTabProps {
  editor: EditorSession;
  /** The panel is open and this tab is the active one. */
  shown: boolean;
  onReturnFocus: () => void;
}

/** The mark of a step on the lane: a filled disc while in effect, an open ring with a dotted line
 * once taken back (criterion 39). */
function Lane({ undone, last }: { undone: boolean; last: boolean }) {
  return (
    <svg width={16} height={ROW_HEIGHT_PX} aria-hidden className="shrink-0 text-[var(--panel-muted-fg)]">
      <line
        x1={8}
        x2={8}
        y1={0}
        y2={last ? 22 : ROW_HEIGHT_PX}
        stroke="currentColor"
        strokeWidth={1.5}
        strokeDasharray={undone ? "2 3" : undefined}
      />
      {undone ? (
        <circle cx={8} cy={22} r={3.5} fill="var(--panel-bg)" stroke="currentColor" strokeWidth={1.5} />
      ) : (
        <circle cx={8} cy={22} r={4} fill="currentColor" />
      )}
    </svg>
  );
}

/**
 * The History tab (`specs/0020-undo-redo` criteria 9 and 39 to 46): the Undo and Redo buttons, the
 * pinned "Now" row, the global history as one virtualised list (newest first, 44 px rows, only the
 * rows in view are in the DOM) and the line that says the history is saved in the file. The header
 * and the footer stay in view; only the list scrolls.
 */
export function HistoryTab({ editor, shown, onReturnFocus }: HistoryTabProps) {
  const { getSession, syncRevision } = editor;
  const list = useHistoryList(getSession, syncRevision, shown);
  const scroller = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewport, setViewport] = useState(ROW_HEIGHT_PX * 8);
  const [focusedRaw, setFocused] = useState(0);
  // A wipe or a shorter list leaves the roving index inside the list, so it keeps a Tab stop.
  const focused = Math.min(focusedRaw, Math.max(0, list.total - 1));
  const refocus = useRef(false);
  const now = list.readAt;

  useLayoutEffect(() => {
    const element = scroller.current;
    if (!element) {
      return undefined;
    }
    const measure = () => setViewport(element.clientHeight);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [shown]);

  const range = visibleRange(scrollTop, viewport, list.total);
  const { ensure } = list;
  useEffect(() => {
    ensure(range.first, range.last);
  }, [ensure, range.first, range.last]);

  // A row arrives at the top: a list scrolled away keeps its place (criterion 44).
  const { takeAdded, total } = list;
  useLayoutEffect(() => {
    const added = takeAdded();
    const element = scroller.current;
    if (added > 0 && element) {
      const next = scrollAfterInsert(element.scrollTop, added);
      if (next !== element.scrollTop) {
        element.scrollTop = next;
      }
    }
  }, [total, takeAdded]);

  // Keyboard focus follows the roving index once the row exists.
  useEffect(() => {
    if (!refocus.current) {
      return;
    }
    const target = scroller.current?.querySelector<HTMLElement>(`[data-row-index="${focused}"]`);
    if (target) {
      refocus.current = false;
      target.focus();
    }
  });

  const setHover = useCallback(
    (step: string) => {
      getSession()?.set_history_hover(step);
    },
    [getSession],
  );
  // The hover link ends with the tab.
  useEffect(() => {
    if (!shown) {
      setHover("");
    }
    return () => setHover("");
  }, [shown, setHover]);

  const moveFocus = (to: number) => {
    const element = scroller.current;
    if (!element) {
      return;
    }
    const top = to * ROW_HEIGHT_PX;
    if (top < element.scrollTop) {
      element.scrollTop = top;
    } else if (top + ROW_HEIGHT_PX > element.scrollTop + element.clientHeight) {
      element.scrollTop = top + ROW_HEIGHT_PX - element.clientHeight;
    }
    refocus.current = true;
    setFocused(to);
    // setFocused changes nothing when the index did not: focus the row now when it exists.
    const row = element.querySelector<HTMLElement>(`[data-row-index="${to}"]`);
    if (row) {
      refocus.current = false;
      row.focus();
    }
  };

  const onListKeyDown = (event: React.KeyboardEvent) => {
    const pageRows = Math.max(1, Math.floor(viewport / ROW_HEIGHT_PX));
    const to = nextFocus(event.key, focused, list.total, pageRows);
    if (to !== null) {
      event.preventDefault();
      moveFocus(to);
    }
  };

  // The rows in view, and the row that holds the roving focus even when it is scrolled away.
  const indices: number[] = [];
  for (let index = Math.min(range.first, focused); index <= Math.max(range.last, focused); index += 1) {
    if (index < list.total && (index === focused || (index >= range.first && index <= range.last))) {
      indices.push(index);
    }
  }
  const rows = indices.map((index) => {
    const row = list.rowAt(index);
    return row ? (
      <StepRow
        key={row.step}
        row={row}
        index={index}
        total={list.total}
        now={now}
        tabbable={index === focused}
        onFocusRow={() => {
          setFocused(index);
          setHover(row.step);
        }}
        onHover={setHover}
      />
    ) : (
      <div
        key={`pending-${index}`}
        role="listitem"
        aria-hidden
        className="absolute right-0 left-0 flex flex-col justify-center gap-1 px-2"
        style={{ top: index * ROW_HEIGHT_PX, height: ROW_HEIGHT_PX }}
      >
        <span className="h-2 w-2/3 rounded bg-[var(--value-fill)]" />
        <span className="h-2 w-1/3 rounded bg-[var(--value-fill)]" />
      </div>
    );
  });

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-x-hidden overflow-y-auto">
      <HistoryButtons editor={editor} onReturnFocus={onReturnFocus} />
      <div
        className="flex shrink-0 items-center justify-between rounded-[5px] bg-[var(--value-fill)] px-2 text-sm text-[var(--toolbar-icon)]"
        style={{ height: NOW_ROW_HEIGHT_PX }}
        aria-current="true"
      >
        <span className="flex items-center gap-2">
          <svg width={16} height={16} aria-hidden>
            <circle cx={8} cy={8} r={4} fill="currentColor" />
          </svg>
          Now
        </span>
        <span className="select-text font-mono text-xs text-[var(--panel-muted-fg)]">{list.nowId}</span>
      </div>
      {list.total === 0 ? (
        <div className="px-2 pt-4 text-center outline-none" data-first-focus tabIndex={-1}>
          <p className="text-sm text-[var(--toolbar-icon)]">No steps yet.</p>
          <p className="text-xs text-[var(--panel-muted-fg)]">Steps appear here as you edit.</p>
        </div>
      ) : null}
      <div
        ref={scroller}
        role="list"
        aria-label="History"
        data-first-focus={list.total > 0 ? "" : undefined}
        tabIndex={-1}
        onFocus={(event) => {
          // Focus that lands on the container (Shift+Ctrl+H) goes to the roving row.
          if (event.target === event.currentTarget && list.total > 0) {
            moveFocus(focused);
          }
        }}
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
        onKeyDown={onListKeyDown}
        onMouseLeave={() => setHover("")}
        className="history-list relative min-h-[120px] flex-1 overflow-x-hidden overflow-y-auto outline-none"
        style={{ overscrollBehavior: "contain", scrollbarWidth: "thin" }}
      >
        <div className="relative" style={{ height: list.total * ROW_HEIGHT_PX }}>
          {rows}
        </div>
      </div>
      <p className="shrink-0 pt-2 text-xs text-[var(--panel-muted-fg)]">{SAVED_IN_FILE_LINE}</p>
      <WipeFooter editor={editor} total={list.total} onReturnFocus={onReturnFocus} />
    </div>
  );
}

interface StepRowProps {
  row: HistoryRowData;
  index: number;
  total: number;
  now: Date;
  tabbable: boolean;
  onFocusRow: () => void;
  onHover: (step: string) => void;
}

/** One step: kind glyph, operation name, step id, and the second line (criterion 39). */
function StepRow({ row, index, total, now, tabbable, onFocusRow, onHover }: StepRowProps) {
  const Glyph = row.label === "wipe_history" ? Eraser : (KIND_GLYPHS[row.kind] ?? Pencil);
  const undone = row.status === "undone";
  const muted = undone ? "text-[var(--panel-muted-fg)]" : "text-[var(--toolbar-icon)]";
  return (
    <div
      role="listitem"
      aria-posinset={index + 1}
      aria-setsize={total}
      className="absolute right-0 left-0"
      style={{ top: index * ROW_HEIGHT_PX, height: ROW_HEIGHT_PX }}
    >
      <Tooltip side="left" content={rowTooltip(row)}>
        <button
          type="button"
          data-row-index={index}
          tabIndex={tabbable ? 0 : -1}
          aria-label={rowAriaName(row, now)}
          onFocus={onFocusRow}
          onBlur={() => onHover("")}
          onMouseEnter={() => onHover(row.step)}
          onMouseLeave={() => onHover("")}
          className="flex h-full w-full items-stretch gap-1 rounded-[5px] text-left outline-none hover:bg-[var(--row-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
        >
          <Lane undone={undone} last={index === total - 1} />
          <span className="flex min-w-0 flex-1 flex-col justify-center pr-2">
            <span className={`flex h-5 items-center gap-1.5 text-sm ${muted}`}>
              <Glyph size={16} strokeWidth={1.5} aria-hidden />
              <span className="min-w-0 flex-1 truncate">{row.name}</span>
              <span className="select-text font-mono text-xs text-[var(--panel-muted-fg)]">
                {row.id}
              </span>
            </span>
            <span className="h-4 truncate text-xs text-[var(--panel-muted-fg)]">
              {undone ? <strong className="font-semibold">Undone</strong> : null}
              {undone ? " · " : null}
              {secondLineRest(row, now)}
            </span>
          </span>
        </button>
      </Tooltip>
    </div>
  );
}

/** The Undo and Redo buttons in two reserved 28 px slots at the right of the header row: a button
 * that cannot apply leaves the tree and its slot stays empty (criterion 9). */
function HistoryButtons({
  editor,
  onReturnFocus,
}: {
  editor: EditorSession;
  onReturnFocus: () => void;
}) {
  const { historyNames, pressHistory, canvasNotice, historyBusy } = editor;
  const slot = (
    direction: "undo" | "redo",
    name: string | null,
    Icon: Glyph,
    shortcut: string,
  ) => (
    <span className="inline-flex h-7 w-7 items-center justify-center">
      {name !== null ? (
        <Tooltip
          side="left"
          content={`${direction === "undo" ? "Undo" : "Redo"}: ${name} (${shortcut})`}
        >
          <button
            type="button"
            aria-label={`${direction === "undo" ? "Undo" : "Redo"}: ${name}`}
            disabled={historyBusy}
            onClick={(event) => {
              pressHistory(direction, "panel");
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
            className="flex h-7 w-7 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          >
            <Icon size={16} strokeWidth={1.5} aria-hidden />
          </button>
        </Tooltip>
      ) : null}
    </span>
  );
  const panelNotice = canvasNotice?.source === "panel" ? canvasNotice : null;
  return (
    <div className="relative mb-2 flex h-7 shrink-0 items-center justify-end">
      {slot("undo", historyNames.undo, Undo2, "Ctrl+Z")}
      {slot("redo", historyNames.redo, Redo2, "Ctrl+Shift+Z")}
      {/* The live region exists empty and is filled, so the text is announced. */}
      <p
        role="status"
        aria-live="polite"
        className={`pointer-events-none absolute top-full right-0 z-30 mt-1 max-w-[244px] rounded-md p-2 text-right text-xs ${
          panelNotice ? "" : "sr-only"
        }`}
        style={
          panelNotice
            ? {
                background: "var(--toolbar-bg)",
                color: "var(--toolbar-icon)",
                boxShadow: "var(--panel-elevation-shadow)",
              }
            : undefined
        }
      >
        {panelNotice?.text}
      </p>
    </div>
  );
}

/**
 * The footer's wipe control (`specs/0020-undo-redo` criteria 47, 50 and 51): "Wipe history" while
 * there is a step, replaced in place by the question with Cancel and Wipe (an inline confirm, no
 * dialog); in a shared document a line says it is not available. Escape, Cancel, a change of tab, a
 * change of selection and a new step cancel it without writing anything. Wipe ignores presses for
 * the first 400 ms, so a double click on the old button cannot confirm.
 */
function WipeFooter({
  editor,
  total,
  onReturnFocus,
}: {
  editor: EditorSession;
  total: number;
  onReturnFocus: () => void;
}) {
  const { getSession, syncRevision, selectionCount, wipeHistory, showHistoryNotice } = editor;
  const showNotice = showHistoryNotice;
  const [confirming, setConfirming] = useState(false);
  const [armed, setArmed] = useState(false);
  const cancel = useRef<HTMLButtonElement>(null);
  const opener = useRef<HTMLButtonElement>(null);
  // The confirm held the focus when it closed by itself or by Cancel or Escape: it goes back to the
  // button that opened it.
  const confirmHadFocus = useRef(false);
  const restoreToOpener = useRef(true);
  const shared = useMemo(
    () => getSession()?.history_wipe_state() === "shared",
    // The state follows the log and the session.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [getSession, syncRevision, total],
  );

  // A change of selection, a new step or a tab change (this component leaving) cancels.
  useEffect(() => {
    setConfirming(false);
  }, [selectionCount, total]);
  useLayoutEffect(() => {
    if (!confirming && confirmHadFocus.current) {
      confirmHadFocus.current = false;
      if (restoreToOpener.current) {
        opener.current?.focus();
      }
    }
  }, [confirming]);
  useEffect(() => {
    if (!confirming) {
      setArmed(false);
      return undefined;
    }
    cancel.current?.focus();
    const timer = window.setTimeout(() => setArmed(true), WIPE_ARM_MS);
    return () => window.clearTimeout(timer);
  }, [confirming]);

  if (shared) {
    return <p className="shrink-0 pt-2 text-xs text-[var(--panel-muted-fg)]">{WIPE_SHARED_LINE}</p>;
  }
  if (total === 0) {
    return null;
  }
  if (!confirming) {
    return (
      <button
        ref={opener}
        type="button"
        onClick={() => {
          // An unfinished Pen path: ignored with the hint, like the keys (criteria 6e and 51).
          if (getSession()?.history_wipe_blocked()) {
            showNotice(getSession()?.finish_pen_first_notice() ?? "", 2000);
            return;
          }
          restoreToOpener.current = true;
          setConfirming(true);
        }}
        className="mt-2 h-7 w-full shrink-0 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] text-sm text-[var(--destructive)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        Wipe history
      </button>
    );
  }
  const close = () => {
    setConfirming(false);
  };
  return (
    <div
      role="group"
      aria-label={wipeQuestion(total)}
      onFocus={() => {
        confirmHadFocus.current = true;
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          close();
        }
      }}
      className="mt-2 shrink-0"
    >
      <p className="mb-2 text-xs text-[var(--toolbar-icon)]">{wipeQuestion(total)}</p>
      <div className="flex gap-2">
        <button
          ref={cancel}
          type="button"
          onClick={close}
          className="h-7 w-[118px] rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          Cancel
        </button>
        <button
          type="button"
          aria-disabled={!armed}
          onClick={() => {
            if (!armed) {
              return;
            }
            const removed = wipeHistory();
            if (removed >= 0) {
              restoreToOpener.current = false;
              setConfirming(false);
              onReturnFocus();
            }
          }}
          className="h-7 w-[118px] rounded-[5px] border border-[var(--destructive)] bg-white text-sm text-[var(--destructive)] outline-none hover:bg-[var(--destructive)] hover:text-white aria-disabled:hover:bg-white aria-disabled:hover:text-[var(--destructive)] focus-visible:bg-[var(--destructive)] focus-visible:text-white aria-disabled:focus-visible:bg-white aria-disabled:focus-visible:text-[var(--destructive)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          Wipe
        </button>
      </div>
    </div>
  );
}
