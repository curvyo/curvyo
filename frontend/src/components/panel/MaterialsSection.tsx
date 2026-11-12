import { ChevronDown, ChevronRight } from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";

import { MaterialFields } from "@/components/panel/MaterialFields";
import { MaterialRowView, MATERIAL_ROW, type MaterialRowActions } from "@/components/panel/MaterialRow";
import { useDeleteArm } from "@/hooks/useDeleteArm";
import type { MaterialsApi } from "@/hooks/useMaterials";
import { useMaterialDrag } from "@/hooks/useMaterialDrag";
import { useScrollWindow } from "@/hooks/useScrollWindow";
import {
  MATERIAL_WINDOW_ABOVE,
  armText,
  deletedText,
  fieldsHeight,
  materialHeights,
  setArmKey,
  moveTarget,
  rowTitle,
  setArmText,
} from "@/lib/materialRows";
import { SET_WORDS } from "@/lib/operationSettingsRows";
import { insertionEdge, nextFocusIndex } from "@/lib/operationRows";
import { visibleRange } from "@/lib/operationWindow";

const OVERSCAN = 160;

interface MaterialsSectionProps {
  api: MaterialsApi;
  onReturnFocus: () => void;
}

/** Whether the section was open the last time: kept for the session, never stored. */
let rememberedExpanded = false;

/**
 * The Materials section of the Operations tab (`specs/0028-material-test-library` `ux-notes.md`
 * section 10): a collapsed disclosure "Materials (N)", then the maker's materials of this machine
 * as rows with thickness and kind symbols, rename in place, an armed trash with the use count,
 * order by arrows, drag and Alt+arrows, and the selected material's fields under its row.
 * Damaged and newer records show as such. Nothing here decides a rule; the session does.
 */
export function MaterialsSection({ api, onReturnFocus }: MaterialsSectionProps) {
  const { view, reveal } = api;
  const [expanded, setExpanded] = useState(rememberedExpanded);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [stop, setStop] = useState(0);
  /** A damaged row that was pressed: selected, opens nothing (ux-notes 10.4). */
  const [pressedId, setPressedId] = useState<string | null>(null);
  const bodyId = useId();
  const headerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const rootRef = useRef<HTMLElement>(null);
  const arm = useDeleteArm();
  const pending = useRef<string | null>(null);
  const rows = useMemo(() => view?.rows ?? [], [view]);
  const selectedId = pressedId ?? view?.selected?.id ?? null;

  const windowed = expanded && rows.length > MATERIAL_WINDOW_ABOVE;
  const scroll = useScrollWindow(listRef, windowed, rows.length);

  const disarm = arm.disarm;
  const order = rows.map((row) => row.id).join("|");
  useEffect(() => {
    disarm();
  }, [disarm, order, expanded]);

  // "Edit material": open, select, scroll into view and focus the row.
  const revealCount = reveal?.count ?? 0;
  useEffect(() => {
    if (reveal === null) return;
    rememberedExpanded = true;
    setExpanded(true);
    setPressedId(null);
    pending.current = reveal.id;
    const at = rows.findIndex((row) => row.id === reveal.id);
    if (at >= 0) setStop(at);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [revealCount]);
  useEffect(() => {
    const id = pending.current;
    if (id === null || !expanded) return;
    const button = rootRef.current?.querySelector<HTMLElement>(`[${MATERIAL_ROW}="${id}"] [data-row-name]`);
    if (!button) return;
    pending.current = null;
    button.scrollIntoView?.({ block: "nearest" });
    button.focus();
  });

  const drag = useMaterialDrag(listRef, (from, to) => {
    const row = rows[from];
    if (row && api.move(row.id, to)) {
      pending.current = row.id;
      setStop(to);
    }
  });

  if (view === null) return null;

  const heights = materialHeights(rows, selectedId, view.selected ? fieldsHeight(view.selected.sets.length, view.selected.uses) : 0);
  const selectedIndex = rows.findIndex((row) => row.id === selectedId);
  const range = windowed
    ? visibleRange(heights, scroll?.top ?? 0, scroll?.height ?? 600, OVERSCAN, stop, selectedIndex >= 0 ? selectedIndex : null)
    : { first: 0, last: rows.length, before: 0, after: 0 };

  const toggle = () => {
    rememberedExpanded = !expanded;
    setExpanded(!expanded);
  };

  const actions: MaterialRowActions = {
    select: (row, rename = false) => {
      const at = rows.findIndex((candidate) => candidate.id === row.id);
      if (at >= 0) setStop(at);
      if (row.state === "loaded") {
        setPressedId(null);
        api.select(row.id);
        if (rename) setRenamingId(row.id);
      } else {
        setPressedId(row.id);
        api.select("");
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
          if (left.length === 0) headerRef.current?.focus();
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
    if (!current || (event.target as HTMLElement).tagName === "INPUT") return;
    const index = Number(current.dataset.materialIndex);
    const next = nextFocusIndex(rows.length, index, event.key);
    if (next !== null && (event.target as HTMLElement).hasAttribute("data-row-name")) {
      event.preventDefault();
      setStop(next);
      pending.current = rows[next]?.id ?? null;
    }
  };

  return (
    <section ref={rootRef} aria-label="Materials" className="flex flex-col">
      <button
        ref={headerRef}
        type="button"
        aria-expanded={expanded}
        aria-controls={bodyId}
        onClick={toggle}
        className="flex h-7 w-[244px] items-center gap-1 rounded text-left text-sm text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        {expanded ? <ChevronDown size={12} aria-hidden /> : <ChevronRight size={12} aria-hidden />}
        Materials ({rows.length})
      </button>
      {expanded ? (
        <div id={bodyId} className="flex flex-col">
          {rows.length === 0 ? (
            <p className="py-2 text-sm text-[var(--panel-muted-fg)]">No materials for {view.profileName}.</p>
          ) : (
            <ul ref={listRef} role="list" aria-label={`Materials of ${view.profileName}`} onKeyDown={onListKey} className="flex flex-col">
              <li aria-hidden style={{ height: range.before }} />
              {rows.slice(range.first, range.last).map((row, offset) => {
                const index = range.first + offset;
                return (
                  <MaterialRowView
                    key={row.id}
                    row={row}
                    index={index}
                    total={rows.length}
                    selected={selectedId === row.id}
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
                    {row.state === "loaded" && view.selected?.id === row.id && pressedId === null ? (
                      <MaterialFields
                        material={view.selected}
                        api={api}
                        armedSet={arm.armed}
                        onReturnFocus={onReturnFocus}
                        onSetTrash={(kind) => {
                          const selected = view.selected;
                          if (!selected) return;
                          arm.press(
                            setArmKey(selected.id, kind),
                            setArmText(SET_WORDS[kind], selected.name),
                            () => {
                              api.removeSet(selected.id, kind);
                              return `${SET_WORDS[kind]} set of ${selected.name} deleted.`;
                            },
                          );
                        }}
                      />
                    ) : null}
                  </MaterialRowView>
                );
              })}
              <li aria-hidden style={{ height: range.after }} />
            </ul>
          )}
          {api.notice ? (
            <p role={api.notice.alert ? "alert" : "status"} className="mt-1 text-xs text-[var(--toolbar-icon)]">
              {api.notice.text}
            </p>
          ) : null}
        </div>
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
