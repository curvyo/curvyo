import {
  ChevronDown,
  ChevronRight,
  Copy,
  Eraser,
  File,
  Minus,
  Pencil,
  Plus,
  Redo2,
  Replace,
  Undo2,
} from "lucide-react";
import {
  type ComponentType,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { ToggleGroup, type ToggleOption } from "@/components/ui/toggle-group";
import { Tooltip } from "@/components/ui/tooltip";
import type { EditorSession } from "@/hooks/useEditorSession";
import { useHistoryList } from "@/hooks/useHistoryList";
import {
  type FoldItem,
  type GutterSegment,
  type RowItem,
  edgeY,
  forkPath,
  gutterWidth,
  itemHeight,
  itemRange,
  itemTop,
  laneX,
  listHeight,
} from "@/lib/historyItems";
import type { HistoryScopeState } from "@/hooks/useHistoryScope";
import {
  type HistoryRowData,
  type HistoryScope,
  isUndone,
  leadWords,
  NO_OBJECT_STEPS_LINE,
  NOW_ROW_HEIGHT_PX,
  objectButtonTooltip,
  objectWipeQuestion,
  ROW_HEIGHT_PX,
  SAVED_IN_FILE_LINE,
  WIPE_ARM_MS,
  WIPE_SHARED_LINE,
  wipeQuestion,
  nextFocus,
  rowAriaName,
  rowDisplay,
  rowTooltip,
  scrollAfterInsert,
  secondLineRest,
  canRestoreScroll,
  wipeConfirmKey,
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

interface HistoryTabProps {
  editor: EditorSession;
  /** The panel is open and this tab is the active one. */
  shown: boolean;
  onReturnFocus: () => void;
  /** The scope of the list and the selected object's subject. */
  historyScope: HistoryScopeState;
}

/** An item's slice of the lane gutter (`specs/0042-history-branches` UX notes): the main lane (a
 * filled disc while in effect, an open ring on a dotted line once taken back, dashed for an
 * inherited row; criterion 39 of `0020`), branch lanes with open rings, and the connector at a
 * fork point. Every item draws its own slice, so the lines are continuous in a virtualised list. */
function Gutter({ segments, lanes, height }: { segments: GutterSegment[]; lanes: number; height: number }) {
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
export function HistoryTab({ editor, shown, onReturnFocus, historyScope }: HistoryTabProps) {
  const { getSession, syncRevision } = editor;
  const { scope, info, scrollMemory } = historyScope;
  const list = useHistoryList(getSession, syncRevision, shown, scope, info?.key ?? "");
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

  // The scroll position of each scope is kept: it is saved as the list scrolls and put back once
  // the other scope's rows have arrived (`0041` criterion 1).
  const wanted = `${scope}:${info?.key ?? ""}`;
  const restore = useRef<number | null>(null);
  useLayoutEffect(() => {
    restore.current = scrollMemory.get(scope);
  }, [scope, scrollMemory]);
  // The position is put back only once the rows of the wanted scope are in the list: before that
  // the list is short and the browser would clamp it to the top.
  useLayoutEffect(() => {
    const element = scroller.current;
    if (
      element &&
      canRestoreScroll({
        pending: restore.current,
        total: list.total,
        loadedFor: list.loadedFor,
        wanted,
      })
    ) {
      element.scrollTop = restore.current ?? 0;
      restore.current = null;
    }
  }, [wanted, list.total, list.loadedFor]);

  const { shape } = list;
  const range = itemRange(scrollTop, viewport, shape);
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
    const top = itemTop(to, shape);
    const height = itemHeight(to, shape);
    if (top < element.scrollTop) {
      element.scrollTop = top;
    } else if (top + height > element.scrollTop + element.clientHeight) {
      element.scrollTop = top + height - element.clientHeight;
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
    const item = list.itemAt(index);
    const place = { top: itemTop(index, shape), height: itemHeight(index, shape) };
    if (item && item.type !== "row") {
      return (
        <FoldRow
          key={item.key}
          item={item}
          index={index}
          total={list.total}
          lanes={shape.lanes}
          place={place}
          tabbable={index === focused}
          onFocusRow={() => {
            setFocused(index);
            setHover("");
          }}
          onToggle={() => {
            refocus.current = true;
            setFocused(index);
            list.toggle(index);
          }}
        />
      );
    }
    return item ? (
      <StepRow
        key={item.step}
        row={item}
        index={index}
        total={list.total}
        lanes={shape.lanes}
        place={place}
        now={now}
        tabbable={index === focused}
        onFocusRow={() => {
          setFocused(index);
          setHover(item.step);
        }}
        onHover={setHover}
      />
    ) : (
      <div
        key={`pending-${index}`}
        role="listitem"
        aria-hidden
        className="absolute right-0 left-0 flex flex-col justify-center gap-1 px-2"
        style={place}
      >
        <span className="h-2 w-2/3 rounded bg-[var(--value-fill)]" />
        <span className="h-2 w-1/3 rounded bg-[var(--value-fill)]" />
      </div>
    );
  });

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-x-hidden overflow-y-auto">
      <HistoryHeader editor={editor} historyScope={historyScope} onReturnFocus={onReturnFocus} />
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
        <span className="select-text font-mono text-xs text-[var(--toolbar-icon)]">{list.nowId}</span>
      </div>
      {list.total === 0 && scope === "object" ? (
        <div className="px-2 pt-4 text-center outline-none" data-first-focus tabIndex={-1}>
          <p className="text-xs text-[var(--panel-muted-fg)]">{NO_OBJECT_STEPS_LINE}</p>
        </div>
      ) : list.total === 0 ? (
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
        onScroll={(event) => {
          setScrollTop(event.currentTarget.scrollTop);
          if (restore.current === null && list.loadedFor === wanted) {
            scrollMemory.set(scope, event.currentTarget.scrollTop);
          }
        }}
        onKeyDown={onListKeyDown}
        onMouseLeave={() => setHover("")}
        className="history-list relative min-h-[120px] flex-1 overflow-x-hidden overflow-y-auto outline-none"
        style={{ overscrollBehavior: "contain", scrollbarWidth: "thin" }}
      >
        <div className="relative" style={{ height: listHeight(shape) }}>
          {rows}
        </div>
      </div>
      <p className="shrink-0 pt-2 text-xs text-[var(--panel-muted-fg)]">{SAVED_IN_FILE_LINE}</p>
      <WipeFooter
        editor={editor}
        scope={scope}
        subjectKey={info?.key ?? ""}
        total={scope === "object" ? (info?.wipeCount ?? 0) : list.total}
        onReturnFocus={onReturnFocus}
      />
    </div>
  );
}

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
}

/** One step: kind glyph, operation name, step id, and the second line (criterion 39). */
function StepRow({ row, index, total, lanes, place, now, tabbable, onFocusRow, onHover }: StepRowProps) {
  const Glyph = GLYPHS[glyphKey(row)] ?? Pencil;
  const undone = isUndone(row);
  const lead = leadWords(row);
  const muted = undone ? "text-[var(--panel-muted-fg)]" : "text-[var(--toolbar-icon)]";
  return (
    <div
      role="listitem"
      aria-posinset={index + 1}
      aria-setsize={total}
      className="absolute right-0 left-0"
      style={place}
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
function FoldRow({ item, index, total, lanes, place, tabbable, onFocusRow, onToggle }: FoldRowProps) {
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

const SCOPE_OPTIONS: readonly ToggleOption<HistoryScope>[] = [
  {
    value: "document",
    label: "Document",
    tooltip: "Every step of the document.",
    icon: <span className="text-sm">Document</span>,
    width: 76,
  },
  {
    value: "object",
    label: "Object",
    tooltip: "The steps of the selected object.",
    icon: <span className="text-sm">Object</span>,
    width: 56,
  },
];

/** Header row 1: the scope group at the left (in the tree only with exactly one object selected;
 * its slot stays empty otherwise) and the Undo and Redo buttons in two reserved 28 px slots at the
 * right. A button that cannot apply leaves the tree and its slot stays (`0020` criterion 9, `0041`
 * criteria 1 and 16). */
function HistoryHeader({
  editor,
  historyScope,
  onReturnFocus,
}: {
  editor: EditorSession;
  historyScope: HistoryScopeState;
  onReturnFocus: () => void;
}) {
  const { historyNames, pressHistory, canvasNotice, historyBusy } = editor;
  const { scope, info, hasGroup, choose } = historyScope;
  const [now] = useState(() => new Date());
  const objectScope = scope === "object" && info !== null;
  const slot = (
    direction: "undo" | "redo",
    available: boolean,
    tooltip: string,
    keys: string,
    Icon: Glyph,
  ) => (
    <span className="inline-flex h-7 w-7 items-center justify-center">
      {available ? (
        <Tooltip side="left" content={tooltip}>
          <button
            type="button"
            aria-label={tooltip.replace(/ \(.*\)$/, "")}
            aria-keyshortcuts={keys}
            disabled={historyBusy}
            onClick={(event) => {
              pressHistory(objectScope ? `object-${direction}` : direction, "panel");
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
  const undoTip = objectScope
    ? info.undo
      ? objectButtonTooltip("Undo", info.undo, "Ctrl+U", now)
      : ""
    : `Undo: ${historyNames.undo ?? ""} (Ctrl+Z)`;
  const redoTip = objectScope
    ? info.redo
      ? objectButtonTooltip("Redo", info.redo, "Ctrl+Shift+U", now)
      : ""
    : `Redo: ${historyNames.redo ?? ""} (Ctrl+Shift+Z)`;
  const canUndo = objectScope ? info.undo !== null : historyNames.undo !== null;
  const canRedo = objectScope ? info.redo !== null : historyNames.redo !== null;
  const panelNotice = canvasNotice?.source === "panel" ? canvasNotice : null;
  return (
    <div className="relative mb-2 flex h-7 shrink-0 items-center justify-between">
      <span className="inline-flex min-w-[132px]">
        {hasGroup ? (
          <ToggleGroup
            label="History scope"
            options={SCOPE_OPTIONS}
            value={scope}
            onChange={choose}
            itemWidth={66}
            // 26 px items and the 1 px borders make the 28 px of the row: with 28 px items the group
            // is 30 px high and the row (h-7) clips its top border and focus ring.
            itemHeight={26}
            onReturnFocus={onReturnFocus}
          />
        ) : null}
      </span>
      <span className="inline-flex">
        {slot("undo", canUndo, undoTip, objectScope ? "Control+U" : "Control+Z", Undo2)}
        {slot(
          "redo",
          canRedo,
          redoTip,
          objectScope ? "Control+Shift+U" : "Control+Shift+Z",
          Redo2,
        )}
      </span>
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
  scope,
  subjectKey,
  total,
  onReturnFocus,
}: {
  editor: EditorSession;
  scope: HistoryScope;
  /** Which object the Object scope is about: another one closes an open confirm. */
  subjectKey: string;
  /** The rows a wipe would hide. */
  total: number;
  onReturnFocus: () => void;
}) {
  const {
    getSession,
    syncRevision,
    selectionCount,
    wipeHistory,
    wipeObjectHistory,
    showHistoryNotice,
  } = editor;
  const objectScope = scope === "object";
  const question = objectScope ? objectWipeQuestion(total) : wipeQuestion(total);
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
  const wipeKey = wipeConfirmKey({ selectionCount, subjectKey, total, scope });
  useEffect(() => {
    setConfirming(false);
  }, [wipeKey]);
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

  if (shared && !objectScope) {
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
        {objectScope ? "Wipe object history" : "Wipe history"}
      </button>
    );
  }
  const close = () => {
    setConfirming(false);
  };
  return (
    <div
      role="group"
      aria-label={question}
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
      <p className="mb-2 text-xs text-[var(--toolbar-icon)]">{question}</p>
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
            const removed = objectScope ? wipeObjectHistory() : wipeHistory();
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
