import { Plus } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";

import { MaterialBody } from "@/components/panel/MaterialBody";
import { MaterialRowView, MATERIAL_ROW, type MaterialRowActions } from "@/components/panel/MaterialRow";
import { NewMaterialForm } from "@/components/panel/NewMaterialForm";
import { useDeleteArm } from "@/hooks/useDeleteArm";
import type { MaterialsApi } from "@/hooks/useMaterials";
import { useMaterialDrag } from "@/hooks/useMaterialDrag";
import { useScrollWindow } from "@/hooks/useScrollWindow";
import { MATERIAL_WINDOW_ABOVE, armText, deletedText, materialHeights, moveTarget, rowTitle } from "@/lib/materialRows";
import { insertionEdge, nextFocusIndex } from "@/lib/operationRows";
import { visibleRange } from "@/lib/operationWindow";

const OVERSCAN = 160;

interface MaterialsTabProps {
  api: MaterialsApi;
  onReturnFocus: () => void;
}

/**
 * The Materials tab of a laser page (`specs/0112-test-grid-and-materials` `ux-notes.md` 7): the
 * New material button or form, then the maker's materials as one flat, windowed list. A press on
 * a row unfolds its body (one at a time); the body holds the lasers' sets and the tests. Nothing
 * here decides a rule; the session does.
 */
export function MaterialsTab({ api, onReturnFocus }: MaterialsTabProps) {
  const { view } = api;
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [stop, setStop] = useState(0);
  /** A damaged row that was pressed: selected, opens nothing. */
  const [pressedId, setPressedId] = useState<string | null>(null);
  const [bodyHeight, setBodyHeight] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);
  const rootRef = useRef<HTMLElement>(null);
  const newRef = useRef<HTMLButtonElement>(null);
  const arm = useDeleteArm();
  const pending = useRef<string | null>(null);
  const rows = useMemo(() => view?.rows ?? [], [view]);
  const unfoldedId = view?.unfolded?.id ?? null;
  const windowed = rows.length > MATERIAL_WINDOW_ABOVE;
  const scroll = useScrollWindow(listRef, windowed, rows.length);

  const disarm = arm.disarm;
  const order = rows.map((row) => row.id).join("|");
  useEffect(() => {
    disarm();
  }, [disarm, order, unfoldedId]);

  useEffect(() => {
    const id = pending.current;
    if (id === null) return;
    const button = rootRef.current?.querySelector<HTMLElement>(`[${MATERIAL_ROW}="${id}"] [data-row-name]`);
    if (!button) return;
    pending.current = null;
    button.scrollIntoView?.({ block: "nearest" });
    button.focus();
  });
  // A material unfolded from elsewhere ("Edit material" in an operation) is brought into view.
  useEffect(() => {
    if (unfoldedId === null) return;
    rootRef.current?.querySelector<HTMLElement>(`[${MATERIAL_ROW}="${unfoldedId}"]`)?.scrollIntoView?.({ block: "nearest" });
  }, [unfoldedId]);

  const drag = useMaterialDrag(listRef, (from, to) => {
    const row = rows[from];
    if (row && api.move(row.id, to)) {
      pending.current = row.id;
      setStop(to);
    }
  });
  const onHeight = useCallback((height: number) => setBodyHeight(height), []);

  if (view === null) return null;

  const heights = materialHeights(rows, unfoldedId, bodyHeight);
  const unfoldedIndex = rows.findIndex((row) => row.id === unfoldedId);
  const range = windowed
    ? visibleRange(heights, scroll?.top ?? 0, scroll?.height ?? 600, OVERSCAN, stop, unfoldedIndex >= 0 ? unfoldedIndex : null)
    : { first: 0, last: rows.length, before: 0, after: 0 };

  const actions: MaterialRowActions = {
    select: (row, rename = false) => {
      const at = rows.findIndex((candidate) => candidate.id === row.id);
      if (at >= 0) setStop(at);
      if (row.state === "loaded") {
        setPressedId(null);
        // A press on the unfolded row folds it; a rename (the second press of a double-click) leaves the fold.
        if (rename) {
          if (unfoldedId !== row.id) api.unfold(row.id);
          setRenamingId(row.id);
        } else {
          api.unfold(unfoldedId === row.id ? "" : row.id);
        }
      } else {
        setPressedId(row.id);
        api.unfold("");
      }
    },
    rename: (row, name) => {
      api.rename(row.id, name);
    },
    endRename: (row, refocus) => {
      setRenamingId(null);
      if (refocus) pending.current = row.id;
    },
    trash: (row) => {
      arm.press(`material:${row.id}`, armText(row), () => {
        const name = rowTitle(row);
        const at = rows.findIndex((candidate) => candidate.id === row.id);
        if (api.remove(row.id)) {
          const left = rows.filter((candidate) => candidate.id !== row.id);
          pending.current = left[Math.min(at, left.length - 1)]?.id ?? null;
          if (left.length === 0) newRef.current?.focus();
        }
        return deletedText(name);
      });
    },
    moveKey: (row, key) => {
      const at = rows.findIndex((candidate) => candidate.id === row.id);
      const to = moveTarget(rows.length, at, key);
      if (to !== null && api.move(row.id, to)) {
        pending.current = row.id;
        setStop(to);
      }
    },
    dragStart: (row, event) => {
      drag.start(
        rows.findIndex((candidate) => candidate.id === row.id),
        rowTitle(row),
        event,
      );
    },
  };

  const onListKey = (event: ReactKeyboardEvent<HTMLUListElement>) => {
    if (event.altKey) return;
    const current = (event.target as HTMLElement).closest<HTMLElement>("li[data-material-index]");
    if (!current || !(event.target as HTMLElement).hasAttribute("data-row-name")) return;
    const index = Number(current.dataset.materialIndex);
    const next = nextFocusIndex(rows.length, index, event.key);
    if (next !== null) {
      event.preventDefault();
      setStop(next);
      pending.current = rows[next]?.id ?? null;
    }
  };

  return (
    <section ref={rootRef} aria-label="Materials" className="flex flex-col gap-2">
      {view.formOpen ? (
        <NewMaterialForm
          api={api}
          defaultName={view.nextName}
          onDone={(id) => {
            if (id !== null) pending.current = id;
            else window.requestAnimationFrame(() => newRef.current?.focus());
          }}
        />
      ) : (
        <button
          ref={newRef}
          type="button"
          onClick={() => api.openForm()}
          className="flex h-7 w-[244px] items-center justify-center gap-1 rounded border border-[var(--lane-line)] text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          <Plus size={16} aria-hidden />
          New material
        </button>
      )}
      {rows.length > 0 ? (
        <ul ref={listRef} role="list" aria-label="Materials" onKeyDown={onListKey} className="flex flex-col">
          <li aria-hidden style={{ height: range.before }} />
          {rows.slice(range.first, range.last).map((row, offset) => {
            const index = range.first + offset;
            return (
              <MaterialRowView
                key={row.id}
                row={row}
                index={index}
                total={rows.length}
                selected={(unfoldedId === row.id && pressedId === null) || pressedId === row.id}
                renaming={renamingId === row.id}
                armed={arm.armed === `material:${row.id}`}
                canUp={index > 0}
                canDown={index < rows.length - 1}
                tabStop={index === stop}
                dropping={drag.from !== null && drag.dropAt === index ? insertionEdge(drag.from, drag.dropAt) : null}
                dragSource={drag.from === index}
                actions={actions}
                onFocusRow={() => setStop(index)}
              >
                {row.state === "loaded" && view.unfolded?.id === row.id && pressedId === null ? (
                  <MaterialBody body={view.unfolded} api={api} arm={arm} onReturnFocus={onReturnFocus} onHeight={onHeight} />
                ) : null}
              </MaterialRowView>
            );
          })}
          <li aria-hidden style={{ height: range.after }} />
        </ul>
      ) : null}
      {api.notice ? (
        <p role={api.notice.alert ? "alert" : "status"} className="text-xs text-[var(--toolbar-icon)]">
          {api.notice.text}
        </p>
      ) : null}
      {drag.ghost ? (
        <div
          aria-hidden
          data-reorder-ghost=""
          className="pointer-events-none fixed z-50 flex h-10 items-center rounded bg-[var(--value-fill)] px-2 text-[13px] text-[var(--toolbar-icon)] shadow"
          style={{ left: drag.ghost.left, top: drag.ghost.at, width: drag.ghost.width, opacity: 0.6 }}
        >
          {drag.ghost.label}
        </div>
      ) : null}
      <div role="status" aria-atomic="true" className="sr-only">
        {arm.status}
      </div>
    </section>
  );
}
