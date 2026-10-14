import { ArrowDown, ArrowUp, Copy, Layers, LayersPlus, Link2, Trash2 } from "lucide-react";
import { Fragment, type ComponentType } from "react";
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
  reorderFor,
  rowKey,
  listWindow,
  toolbarStep,
  windowRange,
} from "@/lib/layerRows";
import { OPACITY_CLICK_MIN_P, VALUE_FIELD_SCALE_END } from "@/lib/sliderNames";
import { isTauri } from "@/platform/host";
import { Tooltip } from "@/components/ui/tooltip";

const CELLS: RowCell[] = ["name", "action", "eye", "lock"];
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
  /** Runs Linked clone on the selection (`specs/0049-linked-clone`), as the Select bar does. */
  onLinkedClone: () => void;
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

type ToolGlyph = ComponentType<{ size: number; strokeWidth: number; "aria-hidden": boolean }>;

/** One icon button of the toolbar: a tooltip and an accessible name, dimmed (`aria-disabled`,
 * still focusable, a press does nothing) when it does not apply, never hidden. */
function ToolbarButton({
  name,
  glyph: Glyph,
  applies,
  tabbable,
  buttonRef,
  onFocus,
  onPress,
}: {
  name: string;
  glyph: ToolGlyph;
  applies: boolean;
  /** The toolbar's one Tab stop (roving focus). */
  tabbable: boolean;
  buttonRef: (element: HTMLButtonElement | null) => void;
  onFocus: () => void;
  onPress: () => void;
}) {
  return (
    <Tooltip side="bottom" content={name}>
      <button
        ref={buttonRef}
        type="button"
        tabIndex={tabbable ? 0 : -1}
        onFocus={onFocus}
        aria-label={name}
        aria-disabled={!applies}
        onClick={() => {
          if (applies) {
            onPress();
          }
        }}
        className={`flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:size-9 ${
          applies ? "hover:bg-[var(--editor-accent-hover)]" : ""
        }`}
      >
        <span className={applies ? "flex" : "flex opacity-40"}>
          <Glyph size={16} strokeWidth={1.5} aria-hidden />
        </span>
      </button>
    </Tooltip>
  );
}

/**
 * The Layers tab (`specs/0050-layers-and-objects-panel` criteria 3, 9, 26, 29 to 32, 34, 36, 38 to
 * 41 and 53, UX notes, customer feedback 2026-10-14): the toolbar at the top, the virtualised
 * treegrid of layers and objects, and the Opacity field at the bottom. Every rule and word is the
 * session's; this file holds the focus, the scroll, the drag gesture and the text field of a
 * rename.
 */
export function LayersTab({
  getSession,
  syncRevision,
  sync,
  shown,
  showNotice,
  onReturnFocus,
  onLinkedClone,
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
  const edgeTimer = useRef(0);
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
  const { first, count, extra } = listWindow(range, focusedRaw, total);
  const { view, act, refresh } = useLayersList(getSession, syncRevision, sync, first, count, extra);
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
        // The canvas notice slot is a status region already: not repeated here (UX review of
        // 2026-10-14, must-fix 3).
        showNotice(answer.text, undefined, "canvas", "status");
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
      const how = reorderFor(event.key, event.shiftKey, !isTauri);
      if (how !== null) {
        // Alt+Left and Alt+Right are Back and Forward in the browsers (UX notes): taken here.
        event.preventDefault();
        reply(act((session) => session.reorder_layer_rows(idText(row.id), how)));
        // The focus stays on the moved row (criterion 32).
        const moved = getSession()?.layer_row_index(idText(row.id)) ?? -1;
        if (moved >= 0) {
          moveFocus(moved, cell);
        }
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
        reply(
          act((session) =>
            row.rowAction === "select-original"
              ? session.select_row_original(idText(row.id))
              : session.select_layer_contents(idText(row.id)),
          ),
        );
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
      // In an edge zone the list keeps scrolling while the pointer rests (UX review item 8).
      window.clearInterval(edgeTimer.current);
      const step = edgeScroll(event.clientY, box.top, box.height, rowHeight);
      if (step !== 0) {
        element.scrollTop += step;
        edgeTimer.current = window.setInterval(() => {
          element.scrollTop += step;
        }, 50);
      }
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
      window.clearInterval(edgeTimer.current);
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
        window.clearInterval(edgeTimer.current);
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

  // The Opacity field: a drag previews on the canvas, the release commits once, Escape restores
  // (criterion 26). The field sends the position; Rust maps it on the opacity scale.
  const ends = useMemo(
    () => ({
      commit: () => {
        reply(act((session) => session.commit_layer_opacity_preview()));
      },
      cancel: (session: WasmSession) => session.cancel_layer_opacity(),
    }),
    [reply, act],
  );
  const gesture = usePreviewGesture(getSession, refresh, ends);
  const opacity = view.opacity;
  const value = opacity.value ?? 100;
  const commitValue = (percent: number) =>
    reply(act((session) => session.commit_layer_opacity(percent)));
  const stepSize = (grid: GridName) => (grid === "coarse" ? 10 : 1);
  const toolbar = view.toolbar;
  // The toolbar is one Tab stop with roving focus; dimmed buttons stay in the order.
  const [toolFocus, setToolFocus] = useState(0);
  const toolRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const tools: { name: string; glyph: ToolGlyph; applies: boolean; press: () => void }[] = [
    { name: "New layer", glyph: LayersPlus, applies: toolbar.newLayer, press: newLayer },
    {
      name: "Duplicate",
      glyph: Copy,
      applies: toolbar.duplicate,
      press: () => reply(act((session) => session.duplicate_layer_target())),
    },
    // `specs/0049-linked-clone`: the Select bar's command, with its notice in the panel.
    // `Link2`, the clone glyph of the rows, tags and block, never the Select bar's Link corners.
    { name: "Linked clone", glyph: Link2, applies: toolbar.linkedClone, press: onLinkedClone },
    {
      name: "Move up",
      glyph: ArrowUp,
      applies: toolbar.moveUp,
      press: () => reply(act((session) => session.step_layer_target("up"))),
    },
    {
      name: "Move down",
      glyph: ArrowDown,
      applies: toolbar.moveDown,
      press: () => reply(act((session) => session.step_layer_target("down"))),
    },
    { name: "Delete layer", glyph: Trash2, applies: toolbar.deleteLayer, press: deleteLayer },
  ];

  const rows = view.rows.map((row) => (
    <LayerRow
      key={rowKey(row)}
      row={row}
      top={row.index * rowHeight}
      height={rowHeight}
      tabbable={row.index === focused}
      cell={row.index === focused ? cell : "name"}
      renaming={renaming === rowKey(row)}
      dragging={drag?.active ?? false}
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
      onAction={() =>
        reply(
          act((session) =>
            row.rowAction === "select-original"
              ? session.select_row_original(idText(row.id))
              : session.select_layer_contents(idText(row.id)),
          ),
        )
      }
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
    // A 12 px inset at the right keeps the rows 244 px wide and clear of an overlay scrollbar
    // (UX review item 5).
    <div className="flex min-h-0 flex-1 flex-col pr-3">
      {/* The toolbar: operations are buttons with a visible trigger, never sections that appear
          (customer feedback 2026-10-14). */}
      <div
        role="toolbar"
        aria-label="Layers"
        aria-orientation="horizontal"
        onKeyDown={(event) => {
          const to = toolbarStep(event.key, toolFocus, tools.length);
          if (to !== null) {
            event.preventDefault();
            setToolFocus(to);
            toolRefs.current[to]?.focus();
          }
        }}
        className="flex shrink-0 items-center gap-1 pb-2"
      >
        {tools.map((tool, index) => (
          <Fragment key={tool.name}>
            {index === tools.length - 1 ? <span className="flex-1" /> : null}
            <ToolbarButton
              name={tool.name}
              glyph={tool.glyph}
              applies={tool.applies}
              tabbable={index === toolFocus}
              buttonRef={(element) => {
                toolRefs.current[index] = element;
              }}
              onFocus={() => setToolFocus(index)}
              onPress={tool.press}
            />
          </Fragment>
        ))}
      </div>
      {view.count === 0 ? (
        // The empty state fills the list's place, so the Opacity slot stays at the bottom (UX item 9).
        <div className="min-h-[120px] flex-1 px-2 pt-4 text-center">
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
            <span className="rounded-full bg-[var(--editor-accent)] px-1.5 text-white">{dragBadge(drag.count)}</span>
          ) : null}
        </div>
      ) : null}
      {/* The Opacity field of the panel target, at the bottom of the tab where the other tabs keep
          their properties (customer feedback 2026-10-14). The slot is kept when nothing is
          targeted, so the list never jumps. */}
      <div className="shrink-0 pt-1">
        <div role="status" className="h-4 truncate text-xs text-[var(--panel-muted-fg)]">
          {hint}
        </div>
        <div className="flex h-7 items-center">
          {opacity.field ? (
            // One tooltip: the field's own, with the per-paint rule as its last line (UX review
            // of 2026-10-14, must-fix 1).
            <div className="min-w-0 flex-1">
                <ValueField
                  note={OPACITY_TOOLTIP}
                  label="Opacity"
                  name="Layer or object opacity"
                  text={String(value)}
                  bar={opacity.bar}
                  mixed={opacity.value === null}
                  resettable={opacity.value !== 100}
                  unit="%"
                  valueNow={value}
                  unitWords="percent"
                  valueMax={VALUE_FIELD_SCALE_END.opacity}
                  clickMinP={OPACITY_CLICK_MIN_P}
                  defaultText="100 %"
                  messages={PERCENT_MESSAGES}
                  onPreview={(p, grid) => {
                    const blocked = getSession()?.layers_pen_hint() ?? "";
                    if (blocked !== "") {
                      setHint(blocked);
                      return;
                    }
                    gesture.queue((session) => session.preview_layer_opacity(p, grid));
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
        <p className="flex h-4 items-center gap-1 truncate text-xs text-[var(--panel-muted-fg)]">
          {opacity.layer ? <Layers size={12} strokeWidth={1.5} aria-hidden /> : null}
          {opacity.caption}
        </p>
      </div>
      <div role="status" aria-live="polite" className="sr-only">
        {announce}
      </div>
    </div>
  );
}
