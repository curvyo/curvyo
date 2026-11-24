// The Library tab (`specs/0137-document-library` criteria 1 to 4, 10, 11, 13; UX notes sections 2
// to 5, 7.1, 9 and 10): the sources with their documents and page rows, the search, Add folder...
// on the desktop, Import page by button or drag. The rows and every word come from Rust.
import { ChevronDown, ChevronRight, FileText, Folder, FolderPlus, Import, Search, TriangleAlert, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  type LibraryRow,
  addLibraryFolder,
  libraryRows,
  pagePicture,
  removeLibraryFolder,
  scanLibrary,
} from "@/lib/libraryApi";
import { watchCanvasDrag } from "@/lib/libraryDrag";
import { importFromLibrary } from "@/lib/libraryImport";
import { openPreview, pageRowKey, previewNow, readerFor, usePreview } from "@/lib/libraryStore";
import { useLibraryTreeKeys } from "@/hooks/useLibraryTreeKeys";
import { isTauri } from "@/platform/host";
import {
  library_import_failed_text,
  library_no_match_text,
  library_remove_armed_text,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

const HEIGHTS: Record<LibraryRow["kind"], number> = {
  heading: 32,
  source: 32,
  folder: 32,
  document: 44,
  page: 52,
};
const OVERSCAN_PX = 200;
const ARM_MS = 4000;
/** The first group heading has no 8 px above it (UX notes 4.1). */
const FIRST_HEADING = 24;

interface LibraryTabProps {
  getSession: () => WasmSession | null;
  sync: () => void;
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  showNotice: (text: string, ms?: number, source?: "canvas" | "panel", kind?: "status" | "alert") => void;
  /** The panel header's subject line: "214 documents" (UX notes section 3, row 1). */
  onSubject: (text: string) => void;
  /** Escape on the tree, and a mouse import, give focus back to the canvas. */
  onReturnFocus: () => void;
}

interface RowMeta {
  top: number;
  height: number;
  parent: number;
  posinset: number;
  setsize: number;
  /** "Box workspace › Sides › Lid": where the row sits, for its tooltip. */
  place: string;
}

function documentName(row: LibraryRow): string {
  const file = row.path[row.path.length - 1] ?? "";
  return file.replace(/\.[^.]*$/, "");
}

/** The name, cut with an ellipsis; its matched part semibold (P14). One text box, so the chip
 * beside it never shrinks and a match inside a word has no gap. */
function Highlighted({ name, matched }: { name: string; matched?: [number, number] }) {
  if (!matched) {
    return <span className="library-name-text">{name}</span>;
  }
  const chars = Array.from(name);
  return (
    <span className="library-name-text">
      {chars.slice(0, matched[0]).join("")}
      <strong>{chars.slice(matched[0], matched[1]).join("")}</strong>
      {chars.slice(matched[1]).join("")}
    </span>
  );
}

/** A page row's picture, loaded once the row is on screen. */
/** A page row's 56 x 40 picture, or a document row's 40 x 28 one of its first page (UX notes
 * 4.2), loaded once the row is on screen; a document without a picture shows `FileText`. */
function PagePicture({ row }: { row: LibraryRow }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!row.picture || !row.page_id) {
      return;
    }
    let live = true;
    let made: string | null = null;
    const timer = window.setTimeout(() => {
      void pagePicture(row.source, row.path, row.page_id ?? "").then((png) => {
        if (live && png.length > 0) {
          made = URL.createObjectURL(new Blob([png as BlobPart], { type: "image/png" }));
          setUrl(made);
        }
      }, () => undefined);
    }, 100);
    return () => {
      live = false;
      window.clearTimeout(timer);
      if (made) {
        URL.revokeObjectURL(made);
      }
    };
  }, [row.picture, row.page_id, row.source, row.path]);
  const className = row.kind === "document" ? "library-document-picture" : "library-page-picture";
  if (url) {
    return <img className={className} src={url} alt="" />;
  }
  return row.kind === "document" ? (
    <FileText size={16} aria-hidden="true" className="library-document-placeholder" />
  ) : (
    <span className={className} />
  );
}

export function LibraryTab({ getSession, sync, canvasRef, showNotice, onSubject, onReturnFocus }: LibraryTabProps) {
  const [rows, setRows] = useState<LibraryRow[]>([]);
  const [count, setCount] = useState<number | null>(null);
  const [unreadable, setUnreadable] = useState(false);
  const [focusKey, setFocusKey] = useState<string | null>(null);
  const pendingFocus = useRef(false);
  const searchRef = useRef<HTMLInputElement>(null);
  const { page: current } = usePreview();
  const [query, setQuery] = useState("");
  const [expanded, setExpanded] = useState<string[]>([]);
  const [armed, setArmed] = useState<string | null>(null);
  const [refusal, setRefusal] = useState("");
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(400);
  const listRef = useRef<HTMLDivElement>(null);

  const refreshRows = useCallback(() => {
    const dirty = getSession()?.is_dirty() ?? false;
    void libraryRows(query, expanded, dirty).then(setRows, () => setRows([]));
  }, [query, expanded, getSession]);

  const scan = useCallback(() => {
    void scanLibrary().then((found) => {
      setCount(found);
      setUnreadable(false);
      refreshRows();
    }, () => {
      setCount(0);
      setUnreadable(true);
    });
  }, [refreshRows]);

  // The scan runs when the tab is shown and on window focus while it is shown (ADR 0084 §3).
  useEffect(() => {
    scan();
    window.addEventListener("focus", scan);
    return () => window.removeEventListener("focus", scan);
    // Only on show; the rows follow query and expansion below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  useEffect(refreshRows, [refreshRows]);

  useEffect(() => {
    const list = listRef.current;
    if (!list) {
      return;
    }
    const observer = new ResizeObserver(() => setHeight(list.clientHeight));
    observer.observe(list);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!armed) {
      return;
    }
    const timer = window.setTimeout(() => setArmed(null), ARM_MS);
    return () => window.clearTimeout(timer);
  }, [armed]);

  // Offsets, tree positions and places of the rows (the virtualised `0095` tree, UX notes 4, 12).
  const tops = useMemo(() => {
    const result: RowMeta[] = [];
    let at = 0;
    const stack: number[] = [];
    for (const [index, row] of rows.entries()) {
      const height = row.kind === "heading" && index === 0 ? FIRST_HEADING : HEIGHTS[row.kind];
      if (row.kind === "heading") {
        stack.length = 0;
        result.push({ top: at, height, parent: -1, posinset: 0, setsize: 0, place: "" });
      } else {
        while (stack.length > 0 && (rows[stack[stack.length - 1] ?? 0]?.level ?? 0) >= row.level) {
          stack.pop();
        }
        const parent = stack[stack.length - 1] ?? -1;
        const names = stack.map((i) => rows[i]?.name ?? "");
        result.push({ top: at, height, parent, posinset: 0, setsize: 0, place: names.join(" › ") });
        stack.push(index);
      }
      at += height;
    }
    const sets = new Map<string, number[]>();
    for (const [index, row] of rows.entries()) {
      if (row.kind === "heading") {
        continue;
      }
      const group = `${result[index]?.parent ?? -1}|${row.level}`;
      const members = sets.get(group) ?? [];
      members.push(index);
      sets.set(group, members);
    }
    for (const members of sets.values()) {
      members.forEach((index, at) => {
        const meta = result[index];
        if (meta) {
          meta.posinset = at + 1;
          meta.setsize = members.length;
        }
      });
    }
    return { result, total: at };
  }, [rows]);

  // Roving focus: the tree is one Tab stop (UX notes 11); the focused row stays rendered.
  const focusable = rows.map((row, index) => ({ row, index })).filter(({ row }) => row.kind !== "heading");
  const active = focusable.find(({ row }) => row.key === focusKey) ?? focusable[0] ?? null;

  const focusRow = (index: number) => {
    const row = rows[index];
    const meta = tops.result[index];
    const list = listRef.current;
    if (!row || !meta || row.kind === "heading") {
      return;
    }
    if (list) {
      if (meta.top < list.scrollTop) {
        list.scrollTop = meta.top;
      } else if (meta.top + meta.height > list.scrollTop + list.clientHeight) {
        list.scrollTop = meta.top + meta.height - list.clientHeight;
      }
    }
    pendingFocus.current = true;
    setFocusKey(row.key);
    listRef.current?.querySelector<HTMLElement>(`[data-row-key="${CSS.escape(row.key)}"]`)?.focus();
  };

  useEffect(() => {
    if (!pendingFocus.current || !active) {
      return;
    }
    pendingFocus.current = false;
    const element = listRef.current?.querySelector<HTMLElement>(`[data-row-key="${CSS.escape(active.row.key)}"]`);
    if (element && document.activeElement !== element) {
      element.focus();
    }
  });

  const status =
    count === null ? "Reading..." : count === 1 ? "1 document" : `${count} documents`;
  useEffect(() => onSubject(status), [onSubject, status]);

  const toggle = (row: LibraryRow) => {
    if (!row.expandable || query.trim()) {
      return;
    }
    setExpanded((keys) => (keys.includes(row.key) ? keys.filter((k) => k !== row.key) : [...keys, row.key]));
  };

  const subjectOf = (row: LibraryRow) => {
    const at = rows.indexOf(row);
    const source = rows.slice(0, at).reverse().find((r) => r.kind === "source");
    const document = rows.find((r) => r.kind === "document" && r.source === row.source && r.path.join("/") === row.path.join("/"));
    const saved = document?.line.startsWith("Saved version") ? " · Saved version" : "";
    return `${source?.name ?? ""} › ${documentName(row)}${saved}`;
  };

  const openRow = (row: LibraryRow) => {
    void openPreview({
      source: row.source,
      path: row.path,
      pageId: row.page_id ?? "",
      pageName: row.name,
      chip: row.chip ?? "",
      subject: subjectOf(row),
      documentName: documentName(row),
    });
  };

  const importPage = async (row: LibraryRow, drop: { x: number; y: number } | null) => {
    const session = getSession();
    if (!session || !row.page_id) {
      return;
    }
    const open = previewNow();
    const same = open.page && open.page.source === row.source && open.page.path.join("/") === row.path.join("/");
    try {
      const reader = same && open.reader ? open.reader : await readerFor(row.source, row.path);
      const answer = importFromLibrary(
        session,
        reader,
        { documentName: documentName(row), pageName: row.name, pageId: row.page_id },
        true,
        [],
        drop,
      );
      if (reader !== open.reader) {
        reader.free();
      }
      sync();
      if (answer.error) {
        showNotice(library_import_failed_text(row.name, answer.error), 8000, "canvas", "alert");
      } else if (answer.notice) {
        showNotice(answer.notice, 5000, "canvas", "status");
      }
    } catch (error) {
      showNotice(library_import_failed_text(row.name, String(error)), 8000, "canvas", "alert");
    }
  };

  const addFolder = async () => {
    setRefusal("");
    const answer = await addLibraryFolder();
    if (answer.kind === "refused") {
      setRefusal(answer.text);
    } else if (answer.kind === "added") {
      setExpanded((keys) => [...keys, `s:${answer.key}\u001f`]);
      scan();
    }
  };

  const remove = async (row: LibraryRow) => {
    if (armed !== row.key) {
      setArmed(row.key);
      return;
    }
    setArmed(null);
    await removeLibraryFolder(row.source);
    scan();
  };

  const first = tops.result.findIndex((meta) => meta.top + meta.height >= scrollTop - OVERSCAN_PX);
  const shown = rows
    .map((row, index) => ({ row, index }))
    .filter(({ index }) => index >= Math.max(0, first))
    .filter(({ index }) => (tops.result[index]?.top ?? 0) <= scrollTop + height + OVERSCAN_PX)
    .slice(0, 60);
  if (active && !shown.some(({ index }) => index === active.index)) {
    shown.push(active);
  }

  const rowKey = useLibraryTreeKeys({
    rows,
    focusable,
    parentOf: (index) => tops.result[index]?.parent ?? -1,
    pageRows: () => Math.max(1, Math.floor((listRef.current?.clientHeight ?? 300) / 44)),
    searching: query.trim() !== "",
    focusRow,
    toggle,
    openRow,
    importPage: (row) => void importPage(row, null),
    remove: (row) => void remove(row),
    typeIntoSearch: (key) => {
      setQuery((q) => q + key);
      searchRef.current?.focus();
    },
    onReturnFocus,
  });

  const tooltipOf = (row: LibraryRow, index: number) => {
    const place = tops.result[index]?.place ?? "";
    return place ? `${row.name}\n${place}` : row.name;
  };

  const isCurrent = (row: LibraryRow) =>
    row.kind === "page" &&
    current !== null &&
    row.source === current.source &&
    row.page_id === current.pageId &&
    row.path.join("/") === current.path.join("/");

  const emptyText = query.trim()
    ? library_no_match_text(query)
    : count === null
      ? ""
      : unreadable && !isTauri
        ? "Cannot read this browser's storage."
        : isTauri
          ? "No documents yet."
          : "No documents in this browser.";

  return (
    <div className="library-tab">
      {/* The visible subject is the panel header's (row 1); this region announces it. */}
      <span className="sr-only" role="status">
        {status}
      </span>
      <label className="library-search">
        <Search size={16} aria-hidden="true" />
        <input
          ref={searchRef}
          type="search"
          placeholder="Search library"
          aria-label="Search library"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape" && query) {
              event.stopPropagation();
              setQuery("");
            } else if (event.key === "ArrowDown" && active) {
              event.preventDefault();
              focusRow(active.index);
            }
          }}
        />
        {query ? (
          <button type="button" aria-label="Clear search" onClick={() => setQuery("")}>
            <X size={16} />
          </button>
        ) : null}
      </label>
      <div
        className="library-tree"
        role="treegrid"
        aria-label="Library"
        aria-rowcount={rows.length}
        ref={listRef}
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      >
        {rows.length === 0 ? (
          <p className="library-empty">{emptyText}</p>
        ) : (
          <div style={{ height: tops.total, position: "relative" }}>
            {shown.map(({ row, index }) => (
              <div
                key={row.key}
                data-row-key={row.key}
                data-page-row={row.kind === "page" ? pageRowKey(row.source, row.path, row.page_id ?? "") : undefined}
                className={`library-row library-row-${row.kind}${isCurrent(row) ? " library-row-current" : ""}`}
                role={row.kind === "heading" ? "presentation" : "row"}
                aria-level={row.kind === "heading" ? undefined : row.level + 1}
                aria-posinset={row.kind === "heading" ? undefined : tops.result[index]?.posinset}
                aria-setsize={row.kind === "heading" ? undefined : tops.result[index]?.setsize}
                aria-rowindex={index + 1}
                aria-expanded={row.expandable ? row.expanded : undefined}
                aria-label={row.kind === "heading" ? undefined : row.label}
                aria-selected={row.kind === "page" ? isCurrent(row) : undefined}
                style={{ top: tops.result[index]?.top, height: tops.result[index]?.height, paddingLeft: 4 + 12 * Math.min(row.level, 6) }}
                onClick={() => {
                  if (row.kind === "heading") {
                    return;
                  }
                  setFocusKey(row.key);
                  if (row.kind === "page") {
                    openRow(row);
                  } else {
                    toggle(row);
                  }
                }}
                onKeyDown={(event) => rowKey(event, row, index)}
                onPointerDown={(event) => {
                  if (row.kind !== "page" || event.button !== 0 || (event.target as HTMLElement).closest("button")) {
                    return;
                  }
                  const picture = event.currentTarget.querySelector<HTMLImageElement>("img.library-page-picture")?.src ?? null;
                  watchCanvasDrag(event, () => canvasRef.current, row.name, () => "import", (at) => {
                    void importPage(row, at);
                  }, picture);
                }}
                tabIndex={row.kind === "heading" ? undefined : active?.index === index ? 0 : -1}
                title={row.kind === "heading" ? undefined : tooltipOf(row, index)}
              >
                {row.kind === "heading" ? (
                  <span className="library-heading">{row.name}</span>
                ) : (
                  <>
                    {row.kind !== "page" ? (
                      <span className="library-chevron" aria-hidden="true">
                        {row.expandable ? row.expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} /> : null}
                      </span>
                    ) : null}
                    {row.unreadable ? (
                      <TriangleAlert size={16} aria-hidden="true" />
                    ) : row.kind === "source" || row.kind === "folder" ? (
                      <Folder size={16} aria-hidden="true" />
                    ) : (
                      <PagePicture row={row} />
                    )}
                    <span className="library-names">
                      <span className={row.open ? "library-name library-open" : "library-name"}>
                        <Highlighted name={row.name} matched={row.matched} />
                        {row.chip ? <span className="library-chip">{row.chip}</span> : null}
                      </span>
                      {row.kind === "document" || row.kind === "page" || row.unreadable ? (
                        <span className="library-line">{row.line}</span>
                      ) : null}
                    </span>
                    {row.kind === "source" || row.kind === "folder" ? (
                      row.unreadable ? null : <span className="library-count">{row.line}</span>
                    ) : null}
                    {row.kind === "page" ? (
                      <button
                        type="button"
                        className="library-action"
                        aria-label={`Import page ${row.name}`}
                        title="Import page"
                        tabIndex={-1}
                        onClick={(event) => {
                          event.stopPropagation();
                          void importPage(row, null);
                          // A press on an import action returns focus to the canvas (UX notes 11).
                          if (event.detail > 0) {
                            onReturnFocus();
                          }
                        }}
                      >
                        <Import size={16} />
                      </button>
                    ) : null}
                    {row.removable ? (
                      <button
                        type="button"
                        className={armed === row.key ? "library-action library-armed" : "library-action"}
                        tabIndex={-1}
                        aria-label={armed === row.key ? library_remove_armed_text(row.name) : `Remove ${row.name} from the library`}
                        title={armed === row.key ? library_remove_armed_text(row.name) : `Remove ${row.name} from the library`}
                        onClick={(event) => {
                          event.stopPropagation();
                          void remove(row);
                        }}
                      >
                        <X size={16} />
                      </button>
                    ) : null}
                  </>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
      {isTauri ? (
        <div className="library-footer">
          <button type="button" className="library-add" onClick={() => void addFolder()}>
            <FolderPlus size={16} aria-hidden="true" />
            Add folder...
          </button>
          {refusal ? (
            <p className="library-refusal" role="alert">
              {refusal}
            </p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
