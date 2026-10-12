import { Layers, Plus, Trash2 } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import { LayerRow, type RowCell } from "@/components/panel/LayerRow";
import { ValueField } from "@/components/panel/ValueField";
import { useLayersList } from "@/hooks/useLayersList";
import type { GridName } from "@/hooks/useStylePanel";
import { usePreviewGesture } from "@/hooks/usePreviewGesture";
import type { WasmSession } from "@/lib/editorSession";
import {
  COARSE_ROW_HEIGHT_PX,
  DRAG_THRESHOLD_PX,
  HOVER_EXPAND_MS,
  LONG_PRESS_MS,
  type LayerRowData,
  type PanelReplyData,
  ROW_HEIGHT_PX,
  dragBadge,
  dropPlace,
  edgeScroll,
  idText,
  nextRow,
  parseReply,
  reorderFor,
  rowKey,
  windowRange,
} from "@/lib/layerRows";
import { OPACITY_CLICK_MIN_P, VALUE_FIELD_SCALE_END } from "@/lib/sliderNames";

const CELLS: RowCell[] = ["name", "eye", "lock", "action"];
const PERCENT_MESSAGES = { percent: "Enter a number from 0 to 100" } as const;
const OPACITY_TOOLTIP = "Fades fill and stroke separately. Overlapping parts show through.";

interface LayersTabProps {
  getSession: () => WasmSession | null;
  /** Counts the re-reads of the session (`EditorSession.syncRevision`). */
  syncRevision: number;
  /** Re-reads the whole editor after a panel call. */
  sync: () => void;
  /** The tab is on screen. */
  shown: boolean;
  /** Shows a line in the canvas notice slot. */
  showNotice: (text: string, ms?: number, source?: "canvas" | "panel", kind?: "status" | "alert") => void;
  onReturnFocus: () => void;
}

/** A drag of rows in flight (criterion 29). */
interface RowDrag {
  pressed: LayerRowData;
  startY: number;
  pointerId: number;
  active: boolean;
  count: number;
  target: { row: LayerRowData; place: "above" | "below" | "into" } | null;
  refusal: string;
  x: number;
  y: number;
}

/**
 * The Layers tab (`specs/0050-layers-and-objects-panel` criteria 3, 9, 26, 29 to 32, 34, 36, 38 to
 * 41 and 53, UX notes): the Opacity header, the virtualised treegrid of layers and objects, and
 * the footer with New layer and Delete layer. Every rule and word is the session's; this file
 * holds the focus, the scroll, the drag gesture and the text field of a rename.
 */
export function LayersTab({
  getSession,
  syncRevision,
  sync,
  shown,
  showNotice,
  onReturnFocus,
}: LayersTabProps) {
  const scroller = useRef<HTMLDivElement>(null);
  const coarse = useMemo(() => window.matchMedia?.("(pointer: coarse)").matches ?? false, []);
  const rowHeight = coarse ? COARSE_ROW_HEIGHT_PX : ROW_HEIGHT_PX;
  const [scrollTop, setScrollTop] = useState(0);
  const [viewport, setViewport] = useState(rowHeight * 14);
  const [focusedRaw, setFocused] = useState(0);
  const [cell, setCell] = useState<RowCell>("name");
  const [renaming, setRenaming] = useState<string | null>(null);
  const [hint, setHint] = useState("");
  const [announce, setAnnounce] = useState("");
  const [drag, setDrag] = useState<RowDrag | null>(null);
  const dragRef = useRef<RowDrag | null>(null);
  const expandTimer = useRef(0);
  const longPress = useRef(0);

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

  // The row count of the last read decides the window of the next; the window always holds the
  // focused row, so focus is never lost when it scrolls away (criterion 48).
  const [total, setTotal] = useState(0);
  const range = windowRange(scrollTop, viewport, total, rowHeight);
  const first = Math.min(range.first, focusedRaw);
  const count = Math.min(Math.max(range.first + range.count, focusedRaw + 1) - first, 60);
  const { view, act, refresh } = useLayersList(getSession, syncRevision, sync, first, count);
  if (view.count !== total) {
    // A change of the row count during render, the pattern React documents for derived state.
    setTotal(view.count);
  }
  const focused = Math.min(focusedRaw, Math.max(0, view.count - 1));

  const scrollToRow = useCallback(
    (index: number) => {
      const element = scroller.current;
      if (!element) {
        return;
      }
      const top = index * rowHeight;
      if (top < element.scrollTop) {
        element.scrollTop = top;
      } else if (top + rowHeight > element.scrollTop + element.clientHeight) {
        element.scrollTop = top + rowHeight - element.clientHeight;
      }
    },
    [rowHeight],
  );

  // A canvas selection scrolls its first row into view (criterion 12); the scroll is the DOM's.
  useEffect(() => {
    const to = getSession()?.take_layer_scroll() ?? -1;
    if (to >= 0) {
      scrollToRow(to);
    }
  });

  // The panel hint of an unfinished Pen path lasts 2 s (criterion 53).
  useEffect(() => {
    if (hint === "") {
      return undefined;
    }
    const timer = window.setTimeout(() => setHint(""), 2000);
    return () => window.clearTimeout(timer);
  }, [hint]);

  const reply = useCallback(
    (answer: PanelReplyData) => {
      if (answer.kind === "hint") {
        setHint(answer.text);
      } else if (answer.kind === "notice") {
        showNotice(answer.text, undefined, "canvas", "status");
        setAnnounce(answer.text);
      } else if (answer.kind === "refusal") {
        showNotice(answer.text, undefined, "canvas", "alert");
      } else if (answer.kind === "announce") {
        setAnnounce(answer.text);
      }
      return answer;
    },
    [showNotice],
  );

  const rowAt = (index: number) => view.rows.find((row) => row.index === index);
  const focusedRow = rowAt(focused);

  const moveFocus = (to: number, toCell: RowCell = "name") => {
    scrollToRow(to);
    setFocused(to);
    setCell(toCell);
  };

  const press = (row: LayerRowData, how: "plain" | "toggle" | "range") => {
    setFocused(row.index);
    setCell("name");
    reply(act((session) => session.press_layer_row(idText(row.id), how)));
  };

  const flag = (row: LayerRowData, which: "visible" | "locked") =>
    reply(act((session) => session.toggle_layer_row_flag(idText(row.id), which)));

  const expand = (row: LayerRowData, expanded: boolean) => {
    getSession()?.set_layer_expanded(idText(row.id), expanded);
    refresh();
  };

  const startRename = (row: LayerRowData) => {
    const blocked = getSession()?.layers_pen_hint() ?? "";
    if (blocked !== "") {
      setHint(blocked);
      return;
    }
    setFocused(row.index);
    setRenaming(rowKey(row));
  };

  const finishRename = (row: LayerRowData, name: string | null) => {
    if (renaming !== rowKey(row)) {
      return;
    }
    setRenaming(null);
    if (name !== null) {
      reply(act((session) => session.rename_layer_row(idText(row.id), name)));
    }
    setCell("name");
  };

  // After New layer, the new layer's name is in edit (criterion 34).
  const renameNewLayer = useRef(false);
  useEffect(() => {
    if (!renameNewLayer.current || view.activeLayer === null) {
      return;
    }
    const row = view.rows.find((candidate) => idText(candidate.id) === idText(view.activeLayer));
    if (row) {
      renameNewLayer.current = false;
      scrollToRow(row.index);
      setFocused(row.index);
      setRenaming(rowKey(row));
    }
  }, [view, scrollToRow]);

  const newLayer = () => {
    const answer = reply(act((session) => session.new_layer_row()));
    if (answer.kind !== "hint") {
      renameNewLayer.current = true;
    }
  };

  const deleteLayer = () => {
    reply(act((session) => session.delete_active_layer()));
  };

  const onListKeyDown = (event: React.KeyboardEvent) => {
    if (renaming !== null || !focusedRow) {
      return;
    }
    const row = focusedRow;
    const pageRows = Math.max(1, Math.floor(viewport / rowHeight));
    if (event.altKey) {
      const how = reorderFor(event.key);
      if (how !== null) {
        // Alt+Left and Alt+Right are Back and Forward in the browsers (UX notes): taken here.
        event.preventDefault();
        reply(act((session) => session.reorder_layer_rows(idText(row.id), how)));
      }
      return;
    }
    if (event.shiftKey && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
      event.preventDefault();
      const to = nextRow(event.key, focused, view.count, pageRows);
      const target = to === null ? undefined : rowAt(to);
      if (target && to !== null) {
        moveFocus(to);
        reply(act((session) => session.press_layer_row(idText(target.id), "range")));
      }
      return;
    }
    if (event.key === "Escape") {
      // Escape returns to the canvas and never clears the selection (criterion 40).
      event.preventDefault();
      onReturnFocus();
      return;
    }
    if (event.key === "F2") {
      event.preventDefault();
      startRename(row);
      return;
    }
    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      reply(act((session) => session.delete_layer_row(idText(row.id))));
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (cell === "eye") {
        flag(row, "visible");
      } else if (cell === "lock") {
        flag(row, "locked");
      } else if (cell === "action") {
        reply(act((session) => session.select_layer_contents(idText(row.id))));
      } else {
        press(row, event.ctrlKey || event.metaKey ? "toggle" : "plain");
      }
      return;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      if (cell === "name" && row.container && !row.expanded) {
        expand(row, true);
      } else {
        const cells = CELLS.filter((c) => c !== "action" || row.rowAction !== "");
        const next = cells[Math.min(cells.indexOf(cell) + 1, cells.length - 1)];
        setCell(next ?? "name");
      }
      return;
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      if (cell !== "name") {
        const cells = CELLS.filter((c) => c !== "action" || row.rowAction !== "");
        setCell(cells[Math.max(cells.indexOf(cell) - 1, 0)] ?? "name");
      } else if (row.container && row.expanded) {
        expand(row, false);
      } else if (row.level > 1) {
        const parent = [...view.rows]
          .filter((candidate) => candidate.index < row.index && candidate.level === row.level - 1)
          .pop();
        if (parent) {
          moveFocus(parent.index);
        }
      }
      return;
    }
    const to = nextRow(event.key, focused, view.count, pageRows);
    if (to !== null) {
      event.preventDefault();
      moveFocus(to, cell);
    }
  };

  // The drag gesture: pointer events, not HTML5 drag and drop (UX notes, "Drag").
  const updateDrag = (next: RowDrag | null) => {
    dragRef.current = next;
    setDrag(next);
  };

  const onDragStart = (row: LayerRowData) => (event: React.PointerEvent) => {
    if (event.button !== 0) {
      return;
    }
    const start: RowDrag = {
      pressed: row,
      startY: event.clientY,
      pointerId: event.pointerId,
      active: false,
      count: 1,
      target: null,
      refusal: "",
      x: event.clientX,
      y: event.clientY,
    };
    dragRef.current = start;
    if (event.pointerType === "touch") {
      // A touch drag starts after a hold; a vertical pan before it scrolls (UX notes, "Touch").
      longPress.current = window.setTimeout(() => {
        const current = dragRef.current;
        if (current && current.pointerId === start.pointerId) {
          updateDrag({ ...current, active: true });
        }
      }, LONG_PRESS_MS);
    }
  };

  useEffect(() => {
    const onMove = (event: PointerEvent) => {
      const current = dragRef.current;
      const element = scroller.current;
      if (!current || !element || event.pointerId !== current.pointerId) {
        return;
      }
      if (!current.active) {
        if (event.pointerType === "touch") {
          if (Math.abs(event.clientY - current.startY) > DRAG_THRESHOLD_PX) {
            window.clearTimeout(longPress.current);
            dragRef.current = null;
          }
          return;
        }
        if (Math.abs(event.clientY - current.startY) <= DRAG_THRESHOLD_PX) {
          return;
        }
      }
      const box = element.getBoundingClientRect();
      element.scrollTop += edgeScroll(event.clientY, box.top, box.height, rowHeight);
      const offset = event.clientY - box.top + element.scrollTop;
      const index = Math.floor(offset / rowHeight);
      const over = view.rows.find((row) => row.index === index);
      const session = getSession();
      let target: RowDrag["target"] = null;
      let refusal = "";
      if (over && session) {
        const place = dropPlace(offset - index * rowHeight, rowHeight, over.container);
        target = { row: over, place };
        refusal = session.check_layer_drop(idText(current.pressed.id), idText(over.id), place);
        window.clearTimeout(expandTimer.current);
        if (place === "into" && over.container && !over.expanded) {
          expandTimer.current = window.setTimeout(() => expand(over, true), HOVER_EXPAND_MS);
        }
      }
      const selectedCount = current.pressed.mark === "selected" ? (session?.selection_count() ?? 1) : 1;
      updateDrag({
        ...current,
        active: true,
        count: selectedCount,
        target,
        refusal,
        x: event.clientX,
        y: event.clientY,
      });
    };
    const end = (event: PointerEvent, commit: boolean) => {
      const current = dragRef.current;
      if (!current || event.pointerId !== current.pointerId) {
        return;
      }
      window.clearTimeout(longPress.current);
      window.clearTimeout(expandTimer.current);
      updateDrag(null);
      if (commit && current.active && current.target && current.refusal === "") {
        const { row, place } = current.target;
        reply(act((session) => session.drop_layer_rows(idText(current.pressed.id), idText(row.id), place)));
      }
    };
    const onUp = (event: PointerEvent) => end(event, true);
    const onCancel = (event: PointerEvent) => end(event, false);
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && dragRef.current?.active) {
        // Escape cancels a drag and writes nothing (criterion 29).
        event.preventDefault();
        event.stopPropagation();
        window.clearTimeout(expandTimer.current);
        updateDrag(null);
      }
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onCancel);
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onCancel);
      window.removeEventListener("keydown", onKey, true);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view.rows, rowHeight, getSession, act, reply]);

  // The hover box on the canvas ends with the tab.
  useEffect(() => {
    const session = getSession();
    return () => session?.set_layer_hover("");
  }, [getSession]);

  // The Opacity field: a drag previews, the release commits once, Escape restores (criterion 26).
  const ends = useMemo(
    () => ({
      commit: (session: WasmSession) => {
        reply(parseReply(session.commit_layer_opacity_preview()));
      },
      cancel: (session: WasmSession) => session.cancel_layer_opacity(),
    }),
    [reply],
  );
  const gesture = usePreviewGesture(getSession, refresh, ends);
  const opacity = view.opacity;
  const value = opacity.value ?? 100;
  const commitValue = (percent: number) =>
    reply(act((session) => session.commit_layer_opacity(percent)));
  const stepSize = (grid: GridName) => (grid === "coarse" ? 10 : 1);

  const rows = view.rows.map((row) => (
    <LayerRow
      key={rowKey(row)}
      row={row}
      top={row.index * rowHeight}
      height={rowHeight}
      tabbable={row.index === focused}
      cell={row.index === focused ? cell : "name"}
      renaming={renaming === rowKey(row)}
      drop={
        drag?.active && drag.target && rowKey(drag.target.row) === rowKey(row) && drag.refusal === ""
          ? drag.target.place
          : null
      }
      onPress={(event) => {
        if (drag?.active) {
          return;
        }
        press(row, event.shiftKey ? "range" : event.ctrlKey || event.metaKey ? "toggle" : "plain");
      }}
      onFlag={(which) => flag(row, which)}
      onExpand={(expanded) => expand(row, expanded)}
      onAction={() => reply(act((session) => session.select_layer_contents(idText(row.id))))}
      onRename={(name) => finishRename(row, name)}
      onStartRename={() => startRename(row)}
      onDragStart={onDragStart(row)}
      onHover={(hovered) => getSession()?.set_layer_hover(hovered ? idText(row.id) : "")}
      onFocusCell={(focusCell) => {
        setFocused(row.index);
        setCell(focusCell);
        getSession()?.set_layer_hover(idText(row.id));
      }}
    />
  ));

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="shrink-0" style={{ height: 52 }}>
        <div className="flex h-7 items-center gap-2">
          <span className="w-[60px] shrink-0 text-xs text-[var(--toolbar-icon)]" title={OPACITY_TOOLTIP}>
            Opacity
          </span>
          {opacity.field ? (
            <div className="min-w-0 flex-1">
              <ValueField
                label="Opacity"
                name="Layer or object opacity"
                text={String(value)}
                bar={value / 100}
                mixed={opacity.value === null}
                resettable={opacity.value !== 100}
                unit="%"
                valueNow={value}
                unitWords="percent"
                valueMax={VALUE_FIELD_SCALE_END.opacity}
                clickMinP={OPACITY_CLICK_MIN_P}
                defaultText="100 %"
                messages={PERCENT_MESSAGES}
                onPreview={(p) => {
                  const blocked = getSession()?.layers_pen_hint() ?? "";
                  if (blocked !== "") {
                    setHint(blocked);
                    return;
                  }
                  gesture.queue((session) => session.preview_layer_opacity(Math.round(p * 100)));
                }}
                onSettle={gesture.settle}
                onStep={(steps, grid) => commitValue(Math.max(0, Math.min(100, value + steps * stepSize(grid))))}
                onReset={() => commitValue(100)}
                onSubmit={(text) => {
                  const number = Number(text.replace(",", ".").replace("%", "").trim());
                  if (!Number.isFinite(number) || number < 0 || number > 100) {
                    return "invalid:percent";
                  }
                  if (opacity.value === Math.round(number)) {
                    return "unchanged";
                  }
                  commitValue(Math.round(number));
                  return "committed";
                }}
                onReturnFocus={onReturnFocus}
              />
            </div>
          ) : null}
        </div>
        <p className="mt-1 flex h-4 items-center gap-1 truncate pl-[68px] text-xs text-[var(--panel-muted-fg)]">
          {opacity.layer ? <Layers size={12} strokeWidth={1.5} aria-hidden /> : null}
          {opacity.caption}
        </p>
      </div>
      {view.count === 0 ? (
        <div className="px-2 pt-4 text-center">
          <p className="text-sm text-[var(--toolbar-icon)]">No objects yet.</p>
          <p className="text-xs text-[var(--panel-muted-fg)]">Draw something or add a layer.</p>
        </div>
      ) : null}
      <div
        ref={scroller}
        role="treegrid"
        aria-label="Layers and objects"
        aria-rowcount={view.count}
        aria-multiselectable
        data-first-focus={view.count > 0 ? "" : undefined}
        tabIndex={view.count > 0 ? -1 : undefined}
        hidden={view.count === 0}
        onFocus={(event) => {
          if (event.target === event.currentTarget && view.count > 0) {
            moveFocus(view.firstSelected ?? focused);
          }
        }}
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
        onKeyDown={onListKeyDown}
        onMouseLeave={() => getSession()?.set_layer_hover("")}
        className="relative min-h-[120px] flex-1 overflow-x-hidden overflow-y-auto outline-none"
        style={{
          overscrollBehavior: "contain",
          scrollbarWidth: "thin",
          scrollbarGutter: "stable",
          cursor: drag?.active ? (drag.refusal !== "" ? "not-allowed" : "grabbing") : undefined,
        }}
      >
        <div className="relative" style={{ height: view.count * rowHeight }}>
          {rows}
        </div>
      </div>
      {drag?.active ? (
        <div
          aria-hidden
          className="pointer-events-none fixed z-50 flex items-center gap-1 rounded-[4px] bg-[var(--panel-bg)] px-2 py-1 text-xs text-[var(--toolbar-icon)] opacity-70 shadow"
          style={{ left: drag.x + 12, top: drag.y + 8 }}
        >
          <span className="max-w-[160px] truncate">{drag.refusal !== "" ? drag.refusal : drag.pressed.name}</span>
          {dragBadge(drag.count) !== "" ? (
            <span className="rounded-full bg-[var(--accent)] px-1.5 text-white">{dragBadge(drag.count)}</span>
          ) : null}
        </div>
      ) : null}
      <div className="flex shrink-0 items-center justify-between pt-3">
        <button
          type="button"
          onClick={newLayer}
          className="flex h-7 w-[118px] items-center justify-center gap-1 rounded-[5px] border border-[var(--field-border)] text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9"
        >
          <Plus size={16} strokeWidth={1.5} aria-hidden />
          New layer
        </button>
        {view.canDeleteLayer ? (
          <button
            type="button"
            onClick={deleteLayer}
            className="flex h-7 w-[118px] items-center justify-center gap-1 rounded-[5px] border border-[var(--field-border)] text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9"
          >
            <Trash2 size={16} strokeWidth={1.5} aria-hidden />
            Delete layer
          </button>
        ) : (
          <span aria-hidden className="w-[118px]" />
        )}
      </div>
      <div role="status" className="mt-1 h-4 text-xs text-[var(--panel-muted-fg)]">
        {hint}
      </div>
      <div role="status" aria-live="polite" className="sr-only">
        {announce}
      </div>
    </div>
  );
}
