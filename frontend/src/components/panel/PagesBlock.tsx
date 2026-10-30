import { ArrowDown, ArrowUp, GripVertical, Plus, Trash2 } from "lucide-react";
import { useRef, useState } from "react";
import type { DocumentPanelApi } from "@/hooks/useDocumentPanel";
import type { PageRow } from "@/lib/pages";
import { Tooltip } from "@/components/ui/tooltip";

const buttonClass =
  "flex h-7 w-7 shrink-0 items-center justify-center rounded hover:bg-[var(--editor-accent-hover)] focus-visible:outline-2 focus-visible:outline-[var(--editor-accent)] focus-visible:outline-offset-1 aria-disabled:opacity-40";
export function PagesBlock({
  document: doc,
  onReturnFocus,
}: {
  document: DocumentPanelApi;
  onReturnFocus: () => void;
}) {
  const pages = doc.view.pages;
  const active = pages.findIndex((p) => p.active);
  const [editing, setEditing] = useState<string | null>(null);
  const cancelledName = useRef(false);
  const [name, setName] = useState("");
  const [announcement, announce] = useState("");
  const drag = useRef<{
    id: string;
    y: number;
    target: number;
    moved: boolean;
  } | null>(null);
  const [dragTarget, setDragTarget] = useState<number | null>(null);
  const list = useRef<HTMLDivElement>(null);
  const focusRow = (index: number) =>
    list.current
      ?.querySelector<HTMLButtonElement>(`[data-page-name="${index}"]`)
      ?.focus();
  const reorder = (page: PageRow, index: number) => {
    if (index < 0 || index >= pages.length) return;
    doc.pageAction("reorder", page.id, index);
    announce(`${page.name}, position ${index + 1} of ${pages.length}`);
    requestAnimationFrame(() => focusRow(index));
  };
  const finishName = () => {
    if (!cancelledName.current && editing && name.trim())
      doc.pageAction("rename", editing, name);
    setEditing(null);
  };
  const cancelDrag = () => {
    drag.current = null;
    setDragTarget(null);
  };
  return (
    <section aria-label="Pages" className="flex flex-col gap-1">
      <div className="flex items-center gap-1">
        <span className="mr-auto text-xs font-semibold">Pages</span>
        <Tooltip side="left" content="Add page">
          <button
            type="button"
            className={buttonClass}
            aria-label="Add page"
            onClick={() => doc.pageAction("add")}
          >
            <Plus size={16} />
          </button>
        </Tooltip>
        {([-1, 1] as const).map((delta) => (
          <Tooltip
            side="left"
            key={delta}
            content={delta < 0 ? "Move page up" : "Move page down"}
          >
            <button
              type="button"
              className={buttonClass}
              aria-label={delta < 0 ? "Move page up" : "Move page down"}
              aria-disabled={
                active + delta < 0 || active + delta >= pages.length
              }
              onClick={() => {
                if (pages[active]) reorder(pages[active], active + delta);
              }}
            >
              {delta < 0 ? <ArrowUp size={16} /> : <ArrowDown size={16} />}
            </button>
          </Tooltip>
        ))}
      </div>
      <div
        ref={list}
        role="grid"
        aria-label="Pages"
        aria-rowcount={pages.length}
        className="max-h-72 overflow-y-auto"
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            cancelDrag();
            if (editing) {
              setEditing(null);
            } else onReturnFocus();
            e.stopPropagation();
          }
        }}
      >
        {pages.map((page, index) => (
          <div
            key={page.id}
            role="row"
            aria-rowindex={index + 1}
            aria-current={page.active ? "true" : undefined}
            data-page-row={index}
            className="relative flex min-h-11 items-center border-l-[3px] gap-1"
            style={{
              borderLeftColor: page.active
                ? "var(--editor-accent)"
                : "transparent",
              borderTop:
                dragTarget === index
                  ? "2px solid var(--editor-accent)"
                  : undefined,
            }}
            onKeyDown={(e) => {
              if (e.target instanceof HTMLInputElement) return;
              if (e.key === "Escape") {
                e.preventDefault();
                cancelDrag();
                onReturnFocus();
              } else if (
                e.altKey &&
                (e.key === "ArrowUp" || e.key === "ArrowDown")
              ) {
                e.preventDefault();
                reorder(page, index + (e.key === "ArrowUp" ? -1 : 1));
              } else if (
                e.key === "ArrowUp" ||
                e.key === "ArrowDown" ||
                e.key === "Home" ||
                e.key === "End"
              ) {
                e.preventDefault();
                focusRow(
                  e.key === "Home"
                    ? 0
                    : e.key === "End"
                      ? pages.length - 1
                      : Math.max(
                          0,
                          Math.min(
                            pages.length - 1,
                            index + (e.key === "ArrowUp" ? -1 : 1),
                          ),
                        ),
                );
              } else if (e.key === "F2") {
                e.preventDefault();
                cancelledName.current = false;
                setEditing(page.id);
                setName(page.name);
              } else if (e.key === "Delete" && pages.length > 1) {
                e.preventDefault();
                doc.pageAction("delete", page.id);
                requestAnimationFrame(() =>
                  focusRow(Math.min(index, pages.length - 2)),
                );
              } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
                const cells = Array.from(
                  e.currentTarget.querySelectorAll<HTMLButtonElement>("button"),
                );
                const at = cells.indexOf(e.target as HTMLButtonElement);
                cells[
                  Math.max(
                    0,
                    Math.min(
                      cells.length - 1,
                      at + (e.key === "ArrowLeft" ? -1 : 1),
                    ),
                  )
                ]?.focus();
                e.preventDefault();
              } else {
                // Let global history shortcuts reach the Properties handler.
                return;
              }
              e.stopPropagation();
            }}
          >
            <div role="gridcell">
              <button
                type="button"
                className={`${buttonClass} touch-none`}
                aria-label={`Reorder ${page.name}`}
                tabIndex={-1}
                onPointerDown={(e) => {
                  if (e.button !== 0) return;
                  e.currentTarget.setPointerCapture(e.pointerId);
                  drag.current = {
                    id: page.id,
                    y: e.clientY,
                    target: index,
                    moved: false,
                  };
                }}
                onPointerMove={(e) => {
                  const d = drag.current;
                  if (!d) return;
                  if (Math.abs(e.clientY - d.y) >= 4) d.moved = true;
                  if (!d.moved) return;
                  const rows =
                    list.current?.querySelectorAll<HTMLElement>(
                      "[data-page-row]",
                    );
                  rows?.forEach((row, at) => {
                    const r = row.getBoundingClientRect();
                    if (e.clientY >= r.top && e.clientY <= r.bottom) {
                      d.target = at;
                      setDragTarget(at);
                    }
                  });
                }}
                onPointerUp={() => {
                  const d = drag.current;
                  if (d?.moved) reorder(page, d.target);
                  cancelDrag();
                }}
                onPointerCancel={cancelDrag}
              >
                <GripVertical size={16} />
              </button>
            </div>
            <div role="gridcell" className="min-w-0 flex-1">
              {editing === page.id ? (
                <input
                  aria-label={`Name of ${page.name}`}
                  className="w-full text-xs"
                  autoFocus
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  onBlur={finishName}
                  onKeyDown={(e) => {
                    e.stopPropagation();
                    if (e.key === "Enter") {
                      e.preventDefault();
                      finishName();
                      requestAnimationFrame(() => focusRow(index));
                    }
                    if (e.key === "Escape") {
                      e.preventDefault();
                      cancelledName.current = true;
                      setEditing(null);
                      requestAnimationFrame(() => focusRow(index));
                    }
                  }}
                />
              ) : (
                <button
                  type="button"
                  data-page-name={index}
                  title={page.name}
                  className={`block w-full truncate text-left text-xs focus-visible:outline-2 ${page.active ? "font-semibold" : ""}`}
                  tabIndex={page.active ? 0 : -1}
                  onClick={() => doc.pageAction("activate", page.id)}
                  onDoubleClick={() => {
                    cancelledName.current = false;
                    setEditing(page.id);
                    setName(page.name);
                  }}
                >
                  {page.name}
                </button>
              )}
              <div className="text-[11px] tabular-nums opacity-65">
                {page.size}
              </div>
            </div>
            <div role="gridcell">
              <Tooltip side="left" content={`Move frame only: ${page.name}`}>
                <button
                  type="button"
                  role="switch"
                  tabIndex={-1}
                  aria-checked={page.frameOnly}
                  aria-label={`Move frame only: ${page.name}`}
                  className={buttonClass}
                  onClick={() =>
                    doc.pageAction("frameOnly", page.id, !page.frameOnly)
                  }
                >
                  <span
                    aria-hidden
                    className="relative h-3 w-5 rounded-full border"
                    style={{
                      borderColor: "var(--toolbar-icon)",
                      background: page.frameOnly
                        ? "var(--editor-accent)"
                        : "transparent",
                    }}
                  >
                    <span
                      className="absolute top-px h-2 w-2 rounded-full"
                      style={{
                        background: "var(--toolbar-icon)",
                        left: page.frameOnly ? 9 : 1,
                      }}
                    />
                  </span>
                </button>
              </Tooltip>
            </div>
            {pages.length > 1 && (
              <div role="gridcell">
                <Tooltip side="left" content={`Delete page: ${page.name}`}>
                  <button
                    type="button"
                    className={buttonClass}
                    aria-label={`Delete page: ${page.name}`}
                    tabIndex={-1}
                    onClick={() => {
                      doc.pageAction("delete", page.id);
                      requestAnimationFrame(() =>
                        focusRow(Math.min(index, pages.length - 2)),
                      );
                    }}
                  >
                    <Trash2 size={16} />
                  </button>
                </Tooltip>
              </div>
            )}
          </div>
        ))}
      </div>
      {pages[active] && (
        <PagePosition key={pages[active].id} page={pages[active]} doc={doc} />
      )}
      <span role="status" className="sr-only">
        {announcement}
      </span>
    </section>
  );
}
function PagePosition({ page, doc }: { page: PageRow; doc: DocumentPanelApi }) {
  return (
    <div className="flex gap-2 text-xs">
      {(["x", "y"] as const).map((axis) => (
        <label
          key={`${axis}:${axis === "x" ? page.xText : page.yText}:${doc.view.unit}`}
          className="flex min-w-0 items-center gap-1"
        >
          {axis.toUpperCase()}
          <input
            aria-label={`Page ${axis.toUpperCase()} in ${doc.view.unit}`}
            type="number"
            step="any"
            className="min-w-0 w-full rounded border px-1 py-0.5 tabular-nums"
            defaultValue={axis === "x" ? page.xText : page.yText}
            onBlur={(e) => {
              const n = Number(e.target.value);
              if (e.target.value.trim() && Number.isFinite(n))
                doc.pageAction(axis, page.id, e.target.value);
            }}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === "Enter") e.currentTarget.blur();
              if (e.key === "Escape") {
                e.currentTarget.value = axis === "x" ? page.xText : page.yText;
                e.currentTarget.blur();
              }
            }}
          />
        </label>
      ))}
      <span className="self-center">{doc.view.unit}</span>
    </div>
  );
}
