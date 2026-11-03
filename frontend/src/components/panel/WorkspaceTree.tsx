import { ChevronDown, ChevronRight, Copy, FilePlus, FileText, Folder, Trash2, TriangleAlert } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import { WorkspaceNameField } from "@/components/panel/WorkspaceNameField";
import { Tooltip } from "@/components/ui/tooltip";
import type { ThumbnailState, WorkspaceApi } from "@/hooks/useWorkspace";
import { nextRow } from "@/lib/layerRows";
import {
  DOCUMENT_ROW_PX,
  DRAG_THRESHOLD_PX,
  HOVER_EXPAND_MS,
  THUMBNAIL_REST_MS,
  type WorkspaceRowData,
  indentPx,
  isInside,
  parentOf,
  pathKey,
  rowHeight,
  rowOffsets,
  rowWindow,
  samePath,
  targetFolder,
  thumbnailKey,
} from "@/lib/workspaceRows";
import { isMac } from "@/lib/historyText";

const MAC = isMac(navigator.platform);

/** The tree's height: at least four document rows, else what the panel leaves below the other
 * File tab blocks (ux-notes section 3). */
const TREE_HEIGHT = "max(176px, calc(100vh - 600px))";

/** The cells of a document row reached with Right (ux-notes section 3). */
type Cell = "row" | "duplicate" | "delete";

/** A row drag in flight (criterion 16). */
interface RowDrag {
  path: string[];
  startY: number;
  active: boolean;
  /** The folder it would drop into; `[]` for the root, null for none. */
  target: string[] | null;
  x: number;
  y: number;
  name: string;
}

interface WorkspaceTreeProps {
  workspace: WorkspaceApi;
  /** The focused row changed: the header's New document acts on its folder. */
  onFocusRow: (row: WorkspaceRowData | null) => void;
  /** After a mouse open the keyboard goes back to the canvas. */
  onReturnFocus: () => void;
  /** The drop target "root" is the header's name. */
  rootDropRef: React.RefObject<HTMLElement | null>;
}

const ICON_BUTTON =
  "flex size-6 shrink-0 items-center justify-center rounded-[4px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/**
 * The workspace tree (`specs/0095-workspace` criteria 5 to 9, 11, 13 to 17; `ux-notes.md`
 * sections 3 and 11): a virtualised treegrid with one Tab stop and roving focus, folders that
 * expand, documents with thumbnails, the inline name field, drag and Ctrl+X/V moves. The rows,
 * plans and words are the session's; this holds the focus, the scroll and the drag.
 */
export function WorkspaceTree({ workspace, onFocusRow, onReturnFocus, rootDropRef }: WorkspaceTreeProps) {
  const rows = workspace.view.rows;
  const scroller = useRef<HTMLDivElement>(null);
  const coarse = useMemo(() => window.matchMedia?.("(pointer: coarse)").matches ?? false, []);
  const offsets = useMemo(() => rowOffsets(rows, coarse), [rows, coarse]);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewport, setViewport] = useState(176);
  const [focusedRaw, setFocused] = useState(0);
  const [cell, setCell] = useState<Cell>("row");
  const [drag, setDrag] = useState<RowDrag | null>(null);
  const dragRef = useRef<RowDrag | null>(null);
  const expandTimer = useRef(0);
  const focused = Math.min(focusedRaw, Math.max(0, rows.length - 1));
  const focusedRow = rows[focused] ?? null;

  // New document in the header acts on the row the maker chose, not on the row the focus
  // landed on when the workspace opened (ux-notes section 3.2).
  const [chosen, setChosen] = useState(false);
  useEffect(() => onFocusRow(chosen ? focusedRow : null), [chosen, focusedRow, onFocusRow]);

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
  }, []);

  const { first, count } = rowWindow(offsets, scrollTop, viewport);
  const shownIndices = useMemo(() => {
    const indices = Array.from({ length: count }, (_, index) => first + index);
    if (rows.length > 0 && (focused < first || focused >= first + count)) {
      indices.push(focused);
    }
    return indices;
  }, [count, first, focused, rows.length]);

  // Thumbnails of the rows on screen load once the list rests (criterion 6).
  const { loadThumbnails } = workspace;
  useEffect(() => {
    const timer = window.setTimeout(
      () => loadThumbnails(rows.slice(first, first + count)),
      THUMBNAIL_REST_MS,
    );
    return () => window.clearTimeout(timer);
  }, [count, first, loadThumbnails, rows]);

  const rowElement = useCallback(
    (index: number) => scroller.current?.querySelector<HTMLElement>(`[data-ws-row="${index}"]`) ?? null,
    [],
  );

  const scrollToRow = useCallback(
    (index: number) => {
      const element = scroller.current;
      if (!element || index >= offsets.length - 1) {
        return;
      }
      const top = offsets[index];
      const bottom = offsets[index + 1];
      if (top < element.scrollTop) {
        element.scrollTop = top;
      } else if (bottom > element.scrollTop + element.clientHeight) {
        element.scrollTop = bottom - element.clientHeight;
      }
    },
    [offsets],
  );

  const focusRow = useCallback(
    (index: number, nextCell: Cell = "row") => {
      setFocused(index);
      setCell(nextCell);
      scrollToRow(index);
      requestAnimationFrame(() => {
        const element = rowElement(index);
        const target =
          nextCell === "row" ? element : element?.querySelector<HTMLElement>(`[data-ws-cell="${nextCell}"]`);
        target?.focus({ focusVisible: true } as FocusOptions);
      });
    },
    [rowElement, scrollToRow],
  );

  // A new workspace puts the focus on its first row (ux-notes section 11).
  const workspaceName = workspace.name;
  const [seenName, setSeenName] = useState(workspaceName);
  if (seenName !== workspaceName) {
    setSeenName(workspaceName);
    setFocused(0);
    setCell("row");
    setChosen(false);
  }
  useEffect(() => {
    if (scroller.current) {
      scroller.current.scrollTop = 0;
    }
  }, [workspaceName]);

  const refocus = useCallback(() => focusRow(focused), [focusRow, focused]);

  const indexOf = useCallback(
    (path: readonly string[]) => rows.findIndex((row) => samePath(row.path, path)),
    [rows],
  );

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const row = focusedRow;
    if (!row || workspace.naming) {
      return;
    }
    const primary = MAC ? event.metaKey : event.ctrlKey;
    const pageRows = Math.max(1, Math.floor(viewport / DOCUMENT_ROW_PX));
    const to = nextRow(event.key, focused, rows.length, pageRows);
    const handled = () => {
      event.preventDefault();
      event.stopPropagation();
    };
    if (to !== null && !primary && !event.altKey) {
      handled();
      focusRow(to);
      return;
    }
    if (primary && event.key.toLowerCase() === "x") {
      handled();
      workspace.setCut(row.path);
      return;
    }
    if (primary && event.key.toLowerCase() === "v") {
      handled();
      if (workspace.cut && row.kind === "folder") {
        workspace.move(workspace.cut, row.path);
        workspace.setCut(null);
      }
      return;
    }
    if (primary || event.altKey) {
      return;
    }
    switch (event.key) {
      case "ArrowRight":
        handled();
        if (row.kind === "folder" && !row.expanded) {
          workspace.setExpanded(row.path, true);
        } else if (row.kind === "folder") {
          if (rows[focused + 1]?.level === row.level + 1) {
            focusRow(focused + 1);
          }
        } else {
          focusRow(focused, cell === "row" ? "duplicate" : "delete");
        }
        break;
      case "ArrowLeft":
        handled();
        if (cell !== "row") {
          focusRow(focused, cell === "delete" ? "duplicate" : "row");
        } else if (row.kind === "folder" && row.expanded) {
          workspace.setExpanded(row.path, false);
        } else if (row.level > 1) {
          const parent = indexOf(parentOf(row.path));
          if (parent >= 0) {
            focusRow(parent);
          }
        }
        break;
      case "Enter":
        if (cell !== "row") {
          return;
        }
        handled();
        if (row.kind === "folder") {
          workspace.setExpanded(row.path, !row.expanded);
        } else {
          workspace.openRow(row, refocus);
        }
        break;
      case " ":
        if (row.kind === "folder" && cell === "row") {
          handled();
          workspace.setExpanded(row.path, !row.expanded);
        }
        break;
      case "F2":
        handled();
        workspace.startNaming({ mode: "rename", path: [...row.path], initial: row.name, error: "" });
        break;
      case "Delete":
        handled();
        workspace.askDelete(row.path);
        break;
      case "Escape":
        if (workspace.cut) {
          handled();
          workspace.setCut(null);
        }
        break;
      default:
        break;
    }
  };

  // ---- the drag ------------------------------------------------------------------------------

  const dropTargetAt = useCallback(
    (x: number, y: number): string[] | null => {
      const element = document.elementFromPoint(x, y);
      if (rootDropRef.current && element && rootDropRef.current.contains(element)) {
        return [];
      }
      const target = element?.closest<HTMLElement>("[data-ws-folder]");
      return target ? (JSON.parse(target.dataset.wsFolder ?? "[]") as string[]) : null;
    },
    [rootDropRef],
  );

  // `drag` only starts and ends the listeners; its fields are read from the ref.
  const dragging = drag !== null;
  useEffect(() => {
    if (!dragging) {
      return undefined;
    }
    const onMove = (event: PointerEvent) => {
      const current = dragRef.current;
      if (!current) {
        return;
      }
      const active = current.active || Math.abs(event.clientY - current.startY) > DRAG_THRESHOLD_PX;
      const target = active ? dropTargetAt(event.clientX, event.clientY) : null;
      if (active && target && !samePath(target, current.target ?? null)) {
        window.clearTimeout(expandTimer.current);
        const folder = rows.find((row) => samePath(row.path, target));
        if (folder?.kind === "folder" && !folder.expanded) {
          expandTimer.current = window.setTimeout(
            () => workspace.setExpanded(target, true),
            HOVER_EXPAND_MS,
          );
        }
      }
      const next = { ...current, active, target, x: event.clientX, y: event.clientY };
      dragRef.current = next;
      setDrag(next);
    };
    const onUp = () => {
      const current = dragRef.current;
      window.clearTimeout(expandTimer.current);
      dragRef.current = null;
      setDrag(null);
      if (current?.active && current.target) {
        workspace.move(current.path, current.target);
      }
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        window.clearTimeout(expandTimer.current);
        dragRef.current = null;
        setDrag(null);
      }
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("keydown", onKey);
    };
  }, [dragging, dropTargetAt, rows, workspace]);

  const height = offsets[offsets.length - 1] ?? 0;
  const empty = rows.length === 0;

  return (
    <div className="relative">
      <div
        ref={scroller}
        role="treegrid"
        aria-label="Workspace documents"
        aria-rowcount={rows.length}
        aria-multiselectable={false}
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
        onKeyDown={(event) => {
          setChosen(true);
          onKeyDown(event);
        }}
        onPointerDown={() => setChosen(true)}
        className="relative overflow-y-auto overscroll-contain"
        style={{ height: TREE_HEIGHT }}
      >
        {empty ? (
          <EmptyTree onNew={() => workspace.startNaming({ mode: "new-document", folder: [], error: "" })} />
        ) : (
          <div style={{ height }} className="relative">
            {shownIndices.map((index) => {
              const row = rows[index];
              return (
                <TreeRow
                  key={pathKey(row.path)}
                  row={row}
                  index={index}
                  top={offsets[index]}
                  height={rowHeight(row, coarse)}
                  tabbable={index === focused}
                  cell={index === focused ? cell : "row"}
                  open={samePath(row.path, workspace.openPath)}
                  cut={workspace.cut !== null && isInside(row.path, workspace.cut)}
                  dropTarget={drag?.active === true && samePath(drag.target, row.path)}
                  thumbnail={
                    workspace.thumbnails.get(thumbnailKey(row.path, row.time)) ??
                    workspace.thumbnails.get(pathKey(row.path)) ??
                    null
                  }
                  workspace={workspace}
                  refocus={refocus}
                  onFocus={(nextCell) => {
                    setFocused(index);
                    setCell(nextCell);
                  }}
                  onPress={(event) => {
                    setFocused(index);
                    setCell("row");
                    if (row.kind === "folder") {
                      workspace.setExpanded(row.path, !row.expanded);
                    } else {
                      workspace.openRow(row, refocus);
                      if (event.detail > 0) {
                        onReturnFocus();
                      }
                    }
                  }}
                  onDragStart={(event) => {
                    const next: RowDrag = {
                      path: [...row.path],
                      startY: event.clientY,
                      active: false,
                      target: null,
                      x: event.clientX,
                      y: event.clientY,
                      name: row.name,
                    };
                    dragRef.current = next;
                    setDrag(next);
                  }}
                />
              );
            })}
          </div>
        )}
      </div>
      {drag?.active ? (
        <div
          aria-hidden
          className="pointer-events-none fixed z-50 rounded-[4px] bg-[var(--toolbar-bg)] px-2 py-1 text-sm text-[var(--toolbar-icon)] opacity-70 border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)]"
          style={{ left: drag.x + 12, top: drag.y + 8 }}
        >
          {drag.name}
        </div>
      ) : null}
    </div>
  );
}

/** The empty workspace: one line and New document (ux-notes section 10). */
function EmptyTree({ onNew }: { onNew: () => void }) {
  return (
    <div className="flex flex-col items-center pt-4">
      <p className="text-xs text-[var(--panel-muted-fg)]">No documents.</p>
      <button
        type="button"
        onClick={onNew}
        className="mt-4 h-7 w-[118px] rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9"
      >
        New document
      </button>
    </div>
  );
}

interface TreeRowProps {
  row: WorkspaceRowData;
  index: number;
  top: number;
  height: number;
  tabbable: boolean;
  cell: Cell;
  open: boolean;
  cut: boolean;
  dropTarget: boolean;
  thumbnail: ThumbnailState | null;
  workspace: WorkspaceApi;
  onFocus: (cell: Cell) => void;
  onPress: (event: React.MouseEvent) => void;
  onDragStart: (event: React.PointerEvent) => void;
  /** Puts the focus back on the tree's focused row (after a rename). */
  refocus: () => void;
}

/** One row: a folder (32 px) or a document (44 px, 64 px outside a shared workspace) with its
 * thumbnail and actions. */
function TreeRow({
  row,
  index,
  top,
  height,
  tabbable,
  cell,
  open,
  cut,
  dropTarget,
  thumbnail,
  workspace,
  onFocus,
  onPress,
  onDragStart,
  refocus,
}: TreeRowProps) {
  const renaming = workspace.naming?.mode === "rename" && samePath(workspace.naming.path, row.path);
  const unreadable = thumbnail?.kind === "unreadable" ? thumbnail.text : null;
  const folder = row.kind === "folder";
  return (
    <div
      role="row"
      data-ws-row={index}
      data-ws-folder={folder ? JSON.stringify(row.path) : undefined}
      tabIndex={tabbable && cell === "row" ? 0 : -1}
      aria-level={row.level}
      aria-posinset={row.posinset}
      aria-setsize={row.setsize}
      aria-rowindex={index + 1}
      aria-expanded={folder ? row.expanded : undefined}
      aria-selected={open}
      aria-label={unreadable ? `${row.name}, document, ${unreadable}` : row.label}
      onFocus={(event) => {
        if (event.target === event.currentTarget) {
          onFocus("row");
        }
      }}
      onClick={onPress}
      onDoubleClick={(event) => {
        if ((event.target as HTMLElement).closest("[data-ws-name]")) {
          workspace.startNaming({ mode: "rename", path: [...row.path], initial: row.name, error: "" });
        }
      }}
      onPointerDown={(event) => {
        if (event.button === 0 && !renaming) {
          onDragStart(event);
        }
      }}
      className={`group absolute right-0 left-0 flex items-center gap-2 rounded-[4px] pr-1 outline-none select-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-inset ${
        open ? "bg-[color-mix(in_srgb,var(--editor-accent)_14%,transparent)] font-semibold" : "hover:bg-[var(--editor-accent-hover)]"
      } ${dropTarget ? "outline outline-[1.5px] outline-[var(--editor-accent)]" : ""} ${renaming ? "z-10 bg-[var(--toolbar-bg)]" : ""}`}
      style={{ top, height, paddingLeft: indentPx(row.level) + 4, opacity: cut ? 0.55 : 1 }}
    >
      {open ? <span aria-hidden className="absolute top-1 bottom-1 left-0 w-[3px] rounded bg-[var(--editor-accent)]" /> : null}
      {folder ? (
        <>
          <span aria-hidden className="flex text-[var(--toolbar-icon)]">
            {row.expanded ? <ChevronDown size={16} strokeWidth={1.5} /> : <ChevronRight size={16} strokeWidth={1.5} />}
          </span>
          <span aria-hidden className="flex text-[var(--toolbar-icon)]">
            <Folder size={16} strokeWidth={1.5} />
          </span>
        </>
      ) : (
        <Thumbnail state={thumbnail} />
      )}
      <div className="flex min-w-0 flex-1 flex-col justify-center">
        {renaming && workspace.naming?.mode === "rename" ? (
          <WorkspaceNameField
            label={`Rename ${row.name}`}
            initial={workspace.naming.initial}
            error={workspace.naming.error}
            onCommit={async (text) => {
              const refusal = await workspace.commitName(text);
              if (refusal === "") {
                requestAnimationFrame(refocus);
              }
              return refusal;
            }}
            onCancel={() => {
              workspace.cancelNaming();
              refocus();
            }}
          />
        ) : (
          <span data-ws-name="" title={row.name} className="truncate text-sm text-[var(--toolbar-icon)]">
            {row.name}
          </span>
        )}
        {!folder && !renaming ? (
          <span className="truncate text-xs font-normal text-[var(--panel-muted-fg)]">{unreadable ?? row.time}</span>
        ) : null}
        {row.state === "outside" && !renaming ? (
          <span className="mt-0.5 flex">
            <StateButton
              label={`Add ${row.name} to workspace`}
              text="Add to workspace"
              onPress={() => workspace.addToWorkspace(row.path)}
            />
          </span>
        ) : null}
      </div>
      {folder ? (
        <>
          <span className="text-xs font-normal text-[var(--panel-muted-fg)]">{row.count}</span>
          <span className="hidden group-hover:flex group-focus-within:flex">
            <RowButton
              label={`New document in ${row.name}`}
              cellName={null}
              tabbable={false}
              onPress={() => workspace.startNaming({ mode: "new-document", folder: targetFolder(row), error: "" })}
            >
              <FilePlus size={16} strokeWidth={1.5} aria-hidden />
            </RowButton>
          </span>
        </>
      ) : row.state === "outside" ? null : row.state === "missing" ? (
        <StateButton
          label={`Download ${row.name} again`}
          text="Download again"
          onPress={workspace.downloadAgain}
        />
      ) : (
        <span className={`${open || tabbable ? "flex" : "hidden group-hover:flex"} w-[52px] shrink-0 justify-end gap-1`}>
          <RowButton
            label={`Duplicate ${row.name}`}
            cellName="duplicate"
            tabbable={tabbable && cell === "duplicate"}
            onFocus={() => onFocus("duplicate")}
            onPress={() => workspace.duplicate(row.path)}
          >
            <Copy size={16} strokeWidth={1.5} aria-hidden />
          </RowButton>
          <RowButton
            label={`Delete ${row.name}`}
            cellName="delete"
            tabbable={tabbable && cell === "delete"}
            onFocus={() => onFocus("delete")}
            onPress={() => workspace.askDelete(row.path)}
          >
            <Trash2 size={16} strokeWidth={1.5} aria-hidden />
          </RowButton>
        </span>
      )}
    </div>
  );
}

/** The text button of a shared row's state (`ux-notes.md` sections 3 and 6): Add to workspace
 * on a third line of a file outside it (the row grows to 64 px), Download again in the action
 * slot of a missing document **(Proposal)**. */
function StateButton({ label, text, onPress }: { label: string; text: string; onPress: () => void }) {
  return (
    <button
      type="button"
      role="gridcell"
      tabIndex={-1}
      aria-label={label}
      onPointerDown={(event) => event.stopPropagation()}
      onClick={(event) => {
        event.stopPropagation();
        onPress();
      }}
      className="h-6 shrink-0 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent px-1.5 text-xs text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9"
    >
      {text}
    </button>
  );
}

function RowButton({
  label,
  cellName,
  tabbable,
  onFocus,
  onPress,
  children,
}: {
  label: string;
  cellName: "duplicate" | "delete" | null;
  tabbable: boolean;
  onFocus?: () => void;
  onPress: () => void;
  children: React.ReactNode;
}) {
  return (
    <Tooltip side="bottom" content={label}>
      <button
        type="button"
        role="gridcell"
        data-ws-cell={cellName ?? undefined}
        tabIndex={tabbable ? 0 : -1}
        aria-label={label}
        onFocus={onFocus}
        onPointerDown={(event) => event.stopPropagation()}
        onClick={(event) => {
          event.stopPropagation();
          onPress();
        }}
        className={ICON_BUTTON}
      >
        {children}
      </button>
    </Tooltip>
  );
}

/** The 40 x 28 thumbnail, its placeholder, or the mark of a file that cannot be read. */
function Thumbnail({ state }: { state: ThumbnailState | null }) {
  return (
    <span
      aria-hidden
      className="flex h-7 w-10 shrink-0 items-center justify-center overflow-hidden rounded-[3px] border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] bg-[var(--canvas-bg)] text-[var(--toolbar-icon)]"
    >
      {state?.kind === "unreadable" ? (
        <TriangleAlert size={16} strokeWidth={1.5} />
      ) : state?.kind === "thumbnail" && state.url ? (
        <img src={state.url} alt="" draggable={false} className="h-full w-full object-contain" />
      ) : (
        <FileText size={16} strokeWidth={1.5} />
      )}
    </span>
  );
}
