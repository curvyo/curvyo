// The Library tab (`specs/0137-document-library` criteria 1 to 4, 10, 11, 13; UX notes sections 2
// to 5, 7.1, 9 and 10): the sources with their documents and page rows, the search, Add folder...
// on the desktop, Import page by button or drag. The rows and every word come from Rust.
import {
  ChevronDown,
  ChevronRight,
  CircleAlert,
  FileText,
  Folder,
  FolderPlus,
  Globe,
  Import,
  LayoutGrid,
  Lock,
  Plus,
  TriangleAlert,
  X,
} from "lucide-react";
import { GLYPH, SMALL_GLYPH } from "@/components/ui/looks";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { AddSourceForm } from "@/components/panel/AddSourceForm";
import { TokenForm } from "@/components/panel/TokenForm";
import { tokenWords } from "@/lib/tokenWords";
import {
  OnlineRowContent,
  OnlineSourceCells,
  OnlineSourceGlyph,
  type OnlineHandlers,
} from "@/components/panel/OnlineRows";
import { isOnlineKind, onlineHeight } from "@/lib/onlineKinds";
import type { WasmSession } from "@/lib/editorSession";
import {
  type IconCell,
  type LibraryRow,
  addLibraryFolder,
  libraryRows,
  pagePicture,
  removeLibraryFolder,
  scanLibrary,
} from "@/lib/libraryApi";
import { watchCanvasDrag } from "@/lib/libraryDrag";
import {
  type DropAt,
  type LoadContext,
  importIcon as importLibraryIcon,
  importPage as importLibraryPage,
  isOnlineSource,
  loadPage as loadLibraryPage,
  loadTemplate as loadLibraryTemplate,
  iconSlotSource,
  intoSelectedSlot,
  pageSlotDrag,
  pageSlotSource,
  slotDragOf,
  templateSlotSource,
} from "@/lib/libraryLoad";
import { IntoSlotButton, IntoSlotList } from "@/components/IntoSlotList";
import { intoSlotListHeight } from "@/lib/intoSlot";
import { BROWSE_BUTTON_ID, notifyLibraryChanged, onBrowserFetched, toggleBrowser, useBrowserOpen } from "@/lib/libraryBrowserStore";
import {
  addReadySource,
  openLicence,
  openOnlineList,
  openOnlineSet,
  openOnlineTemplate,
  removeOnlineSource,
  beginOnlineSearch,
  searchOnlineSource,
  forgetOnlineIcons,
} from "@/lib/onlineApi";
import { openPreview, pageRowKey, usePreview } from "@/lib/libraryStore";
import { useLibraryTreeKeys } from "@/hooks/useLibraryTreeKeys";
import { Tooltip } from "@/components/ui/tooltip";
import { SearchField } from "@/components/ui/search-field";
import { IconButton } from "@/components/ui/icon-button";
import { isTauri } from "@/platform/host";
import {
  library_no_match_text,
  library_online_searching_text,
  library_remove_armed_text,
} from "@/wasm-bindings/curvyo_editor_wasm.js";
import { Button } from "@/components/ui/button";
import { AlertLine } from "@/components/ui/notice";

const HEIGHTS: Record<LibraryRow["kind"], number> = {
  heading: 32,
  source: 32,
  folder: 32,
  document: 44,
  page: 52,
  ready: 32,
  template: 52,
  set: 44,
  icons: 40,
  status: 28,
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

/** Focuses a row's element, or the first cell of an icon row (its cells are its content). */
function focusElement(element: HTMLElement | null | undefined) {
  (element?.querySelector<HTMLElement>("[data-icon-cell]") ?? element)?.focus();
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
    <FileText {...GLYPH} aria-hidden="true" className="library-document-placeholder" />
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
  const browserOpen = useBrowserOpen();
  const [query, setQuery] = useState("");
  const [expanded, setExpanded] = useState<string[]>([]);
  const [armed, setArmed] = useState<string | null>(null);
  const [refusal, setRefusal] = useState("");
  const [addingSource, setAddingSource] = useState(false);
  // The source whose access token form is open: one inline form at a time (`specs/0157` UX T8).
  const [tokenFor, setTokenFor] = useState<{ source: string; name: string; hasToken: boolean } | null>(null);
  // The key store could not delete a removed source's token: a persistent line (UX T12).
  const [keyStoreLine, setKeyStoreLine] = useState("");
  const [refreshing, setRefreshing] = useState<ReadonlySet<string>>(new Set());
  const [announce, setAnnounce] = useState("");
  const searchTicket = useRef(0);
  /** A row to focus once the rows hold it (an added source); Add source... after the form closes. */
  const focusWhenListed = useRef<string | null>(null);
  const addSourceRef = useRef<HTMLButtonElement>(null);
  const focusAddSource = useRef(false);
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(400);
  const listRef = useRef<HTMLDivElement>(null);

  const refreshRows = useCallback(() => {
    const dirty = getSession()?.is_dirty() ?? false;
    void libraryRows(query, expanded, dirty).then(setRows, () => setRows([]));
    // The library browser shows what the tab holds: it asks again whenever the tab does (ADR 0086).
    notifyLibraryChanged();
  }, [query, expanded, getSession]);
  // The online actions finish after the render that started them: they refresh by the newest state.
  const refreshNow = useRef(refreshRows);
  refreshNow.current = refreshRows;
  // What the browser fetched shows in the tree too (one data, two views, `0152` UX notes 12).
  useEffect(() => onBrowserFetched(() => refreshNow.current()), []);

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

  // The page row whose Into slot list is open, and the list's height: the rows below move down by
  // it (`specs/0154` UX notes V20).
  const [slotList, setSlotList] = useState<{ key: string; height: number } | null>(null);
  // Offsets, tree positions and places of the rows (the virtualised `0095` tree, UX notes 4, 12).
  const tops = useMemo(() => {
    const result: RowMeta[] = [];
    let at = 0;
    const stack: number[] = [];
    for (const [index, row] of rows.entries()) {
      const height = row.kind === "heading" && index === 0 ? FIRST_HEADING : (onlineHeight(row) ?? HEIGHTS[row.kind]);
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
      if (slotList && row.key === slotList.key) {
        at += slotList.height;
      }
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
  }, [rows, slotList]);

  // Roving focus: the tree is one Tab stop (UX notes 11); the focused row stays rendered.
  const focusable = rows
    .map((row, index) => ({ row, index }))
    .filter(({ row }) => row.kind !== "heading" && row.kind !== "status");
  const active = focusable.find(({ row }) => row.key === focusKey) ?? focusable[0] ?? null;

  const focusRow = (index: number) => {
    const row = rows[index];
    const meta = tops.result[index];
    const list = listRef.current;
    if (!row || !meta || row.kind === "heading" || row.kind === "status") {
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
    focusElement(listRef.current?.querySelector<HTMLElement>(`[data-row-key="${CSS.escape(row.key)}"]`));
  };

  useEffect(() => {
    if (!pendingFocus.current || !active) {
      return;
    }
    pendingFocus.current = false;
    const element = listRef.current?.querySelector<HTMLElement>(`[data-row-key="${CSS.escape(active.row.key)}"]`);
    if (element && !element.contains(document.activeElement)) {
      focusElement(element);
    }
  });

  // An added source takes the focus once the rows list it (UX notes 3); a closed form gives it back
  // to Add source....
  useEffect(() => {
    const key = focusWhenListed.current;
    const index = key ? rows.findIndex((row) => row.key === key) : -1;
    if (index >= 0) {
      focusWhenListed.current = null;
      focusRow(index);
    }
    // focusRow reads the rows of this render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows]);
  useEffect(() => {
    if (!addingSource && focusAddSource.current) {
      focusAddSource.current = false;
      addSourceRef.current?.focus();
    }
  }, [addingSource]);

  const status =
    count === null ? "Reading..." : count === 1 ? "1 document" : `${count} documents`;
  useEffect(() => onSubject(status), [onSubject, status]);

  // Opening an online source, set or template is the maker's act and fetches what it lists
  // (ADR 0085 decision 2); the rows show "Loading..." until the host answers.
  const openOnline = (row: LibraryRow) => {
    const source = row.source;
    const done = () => refreshNow.current();
    if (row.kind === "source") {
      void openOnlineList(source, false).then(done, done);
    } else if (row.kind === "set" && row.path[0]) {
      void openOnlineSet(source, row.path[0]).then(done, done);
    } else if (row.kind === "template") {
      void openOnlineTemplate(source, row.path.join("/")).then(done, done);
    }
  };

  const toggle = (row: LibraryRow) => {
    if (!row.expandable || query.trim()) {
      return;
    }
    const opening = !expanded.includes(row.key);
    setExpanded((keys) => (keys.includes(row.key) ? keys.filter((k) => k !== row.key) : [...keys, row.key]));
    if (opening && isOnlineSource(row.source)) {
      openOnline(row);
    }
  };

  const subjectOf = (row: LibraryRow) => {
    const at = rows.indexOf(row);
    const source = rows.slice(0, at).reverse().find((r) => r.kind === "source");
    const document = rows.find(
      (r) => (r.kind === "document" || r.kind === "template") && r.source === row.source && r.path.join("/") === row.path.join("/"),
    );
    const saved = document?.line.startsWith("Saved version") ? " · Saved version" : "";
    return `${source?.name ?? ""} › ${document?.kind === "template" ? document.name : documentName(row)}${saved}`;
  };

  const openRow = (row: LibraryRow) => {
    void openPreview({
      source: row.source,
      path: row.path,
      pageId: row.page_id ?? "",
      pageName: row.name,
      chip: row.chip ?? "",
      subject: subjectOf(row),
      documentName: onlineTemplateName(row) ?? documentName(row),
    });
  };

  const sourceNameOf = (row: LibraryRow) => rows.find((r) => r.kind === "source" && r.source === row.source)?.name ?? "";
  /** The template row's name for a page row of a downloaded template. */
  const onlineTemplateName = (row: LibraryRow) =>
    isOnlineSource(row.source)
      ? rows.find((r) => r.kind === "template" && r.source === row.source && r.path.join("/") === row.path.join("/"))?.name
      : undefined;

  const loadContext: LoadContext = { getSession, sync, showNotice };
  const pageOf = (row: LibraryRow) => ({
    source: row.source,
    path: row.path,
    pageId: row.page_id ?? "",
    pageName: row.name,
    documentName: documentName(row),
    sourceName: sourceNameOf(row),
  });
  const importPage = async (row: LibraryRow, drop: DropAt) => {
    if (!row.page_id) {
      return;
    }
    await importLibraryPage(loadContext, pageOf(row), drop);
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
    if (isOnlineSource(row.source)) {
      forgetOnlineIcons(row.source);
      // A newer sources file is left alone: the refusal is the tab's alert line, as Add folder's.
      const cleanup = await removeOnlineSource(row.source);
      setRefusal(cleanup.refusal ?? "");
      setKeyStoreLine(cleanup.key_store ?? "");
      if (tokenFor?.source === row.source) {
        setTokenFor(null);
      }
      refreshNow.current();
      return;
    }
    await removeLibraryFolder(row.source);
    scan();
  };

  const loadTemplate = async (row: LibraryRow) => {
    const name = await loadLibraryTemplate(loadContext, row.source, row.path, sourceNameOf(row));
    refreshNow.current();
    if (name) {
      setAnnounce(`Loaded ${name}`);
    }
  };

  const importIcon = (row: LibraryRow, cell: IconCell, at: DropAt) =>
    importLibraryIcon(loadContext, row.source, cell.prefix, cell.name, sourceNameOf(row), at);

  const refreshSource = (row: LibraryRow) => {
    setRefreshing((now) => new Set(now).add(row.source));
    forgetOnlineIcons(row.source);
    const done = () => {
      setRefreshing((now) => {
        const next = new Set(now);
        next.delete(row.source);
        return next;
      });
      refreshNow.current();
    };
    void openOnlineList(row.source, true).then(done, done);
  };

  // Enter in the search also sends the words to the Iconify sources added (Frage 340 B); typing
  // sends nothing. Every Iconify source opens with its results on top.
  const searchOnlineNow = () => {
    const words = query.trim();
    if (!isTauri || !words) {
      return;
    }
    const mine = ++searchTicket.current;
    void beginOnlineSearch(query).then((begun) => {
      if (mine !== searchTicket.current || !begun || begun.keys.length === 0) {
        return;
      }
      setAnnounce(library_online_searching_text(begun.keys.length));
      setExpanded((now) => [...new Set([...now, ...begun.keys.map((key) => `s:${key}\u001f`)])]);
      // The rows say "Searching" now; each source answers on its own, with its own announcement.
      refreshNow.current();
      for (const key of begun.keys) {
        void searchOnlineSource(key, begun.words).then(
          (said) => {
            if (mine === searchTicket.current) {
              setAnnounce(said);
              refreshNow.current();
            }
          },
          () => undefined,
        );
      }
    }, () => undefined);
  };
  const hasIconify = rows.some((r) => r.kind === "source" && r.online?.glyph === "iconify");

  const handlers: OnlineHandlers = {
    addReady: () => {
      void addReadySource().then((answer) => {
        setRefusal(answer.kind === "refused" ? answer.text : "");
        // The entry becomes the source row in the same place; the focus stays on it.
        if (answer.kind === "added" && document.activeElement?.closest(".library-tree")) {
          focusWhenListed.current = `s:${answer.key}\u001f`;
        }
        refreshNow.current();
      });
    },
    refresh: refreshSource,
    token: (row) => {
      setAddingSource(false);
      setTokenFor({ source: row.source, name: row.name, hasToken: row.online?.token.private === true });
    },
    remove: (row) => void remove(row),
    load: (row) => void loadTemplate(row),
    licence: (url) => void openLicence(url).catch(() => undefined),
    // A double-click or Enter on a cell puts the icon into a selected target slot (`specs/0154`
    // UX notes 9, customer 2026-11-26), else imports it.
    importIcon: (row, cell, at) =>
      void (async () => {
        const slotted =
          at === null
            ? await intoSelectedSlot(loadContext, () => iconSlotSource(row.source, cell.prefix, cell.name, sourceNameOf(row)))
            : null;
        if (slotted === null) {
          await importIcon(row, cell, at);
        }
      })(),
    dragIcon: (event, row, cell) => {
      if (event.button !== 0) {
        return;
      }
      watchCanvasDrag(
        event,
        () => canvasRef.current,
        cell.name,
        () => "copy",
        (at) => {
          void importIcon(row, cell, at);
        },
        null,
        slotDragOf(loadContext, () => iconSlotSource(row.source, cell.prefix, cell.name, sourceNameOf(row)), { eager: true }),
      );
    },
    armed,
    armedText: (row) => library_remove_armed_text(row.name),
    refreshing,
    onCellFocus: (key) => setFocusKey(key),
    leaveGrid: (row, delta) => {
      const at = focusable.findIndex((f) => f.row.key === row.key);
      const next = focusable[at + delta];
      if (next) {
        focusRow(next.index);
      }
    },
  };
  // A source row is busy while its list, a set or a template of it loads, or Refresh runs.
  const busySources = new Set([
    ...refreshing,
    ...rows.filter((r) => r.kind === "status" && r.online?.status === "loading").map((r) => r.source),
  ]);

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
    importPage: (row) => (row.kind === "template" ? void loadTemplate(row) : void loadLibraryPage(loadContext, pageOf(row))),
    remove: (row) => void remove(row),
    typeIntoSearch: (key) => {
      setQuery((q) => q + key);
      searchRef.current?.focus();
    },
    onReturnFocus,
  });

  const tooltipOf = (row: LibraryRow, index: number) => {
    if (row.kind === "source" && row.online?.host) {
      const lines = `${row.name}\n${row.online.host}`;
      return row.online.token.private ? `${lines}\n${tokenWords().private_tooltip}` : lines;
    }
    // A refused template's reason is cut in the narrow column; the tooltip says it whole.
    if (row.kind === "template" && row.unreadable) {
      return `${row.name}\n${row.line}`;
    }
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
        {announce || status}
      </span>
      <div className="library-search-row">
      <SearchField
        className="flex-1"
        inputRef={searchRef}
        placeholder="Search library"
        aria-label="Search library"
        value={query}
        onValue={setQuery}
        onKeyDown={(event) => {
          if (event.key === "Escape" && query) {
            event.stopPropagation();
            setQuery("");
          } else if (event.key === "Enter") {
            searchOnlineNow();
          } else if (event.key === "ArrowDown" && active) {
            event.preventDefault();
            focusRow(active.index);
          }
        }}
        action={
          isTauri && hasIconify ? (
            <Tooltip side="left" content="Search Iconify sources (Enter)">
              <IconButton
                icon={Globe}
                size={24}
                tabIndex={-1}
                aria-label="Search Iconify sources (Enter)"
                className="pointer-coarse:size-6"
                onClick={() => {
                  searchOnlineNow();
                  searchRef.current?.focus();
                }}
              />
            </Tooltip>
          ) : undefined
        }
      />
      {/* The library browser (`specs/0152` criterion 1): the big grid of what this list holds. */}
      <Tooltip side="left" content="Library browser">
        <button
          type="button"
          id={BROWSE_BUTTON_ID}
          aria-label="Library browser"
          aria-pressed={browserOpen}
          onClick={(event) => toggleBrowser(event.detail === 0)}
          className={`flex size-7 shrink-0 items-center justify-center rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
            browserOpen
              ? "border-transparent bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:bg-[var(--toolbar-icon-active-bg)]"
              : "text-[var(--toolbar-icon)]"
          }`}
        >
          <LayoutGrid {...GLYPH} aria-hidden />
        </button>
      </Tooltip>
      </div>
      {isTauri && keyStoreLine ? (
        <AlertLine className="library-keystore-line">
          <CircleAlert {...GLYPH} aria-hidden="true" style={{ color: "var(--destructive)" }} />
          {keyStoreLine}
        </AlertLine>
      ) : null}
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
            {(() => {
              const at = slotList ? rows.findIndex((r) => r.key === slotList.key) : -1;
              const row = rows[at];
              const meta = tops.result[at];
              if (!slotList || !row || !meta) {
                return null;
              }
              return (
                <IntoSlotList
                  key={row.key}
                  ctx={loadContext}
                  caption={row.name}
                  resolve={() => pageSlotSource(pageOf(row))}
                  style={{ top: meta.top + meta.height, height: slotList.height }}
                  close={() => {
                    setSlotList(null);
                    listRef.current
                      ?.querySelector<HTMLElement>(`[data-row-key="${CSS.escape(row.key)}"] [data-into-slot-button]`)
                      ?.focus();
                  }}
                />
              );
            })()}
            {shown.map(({ row, index }) => (
              <Tooltip key={row.key} side="left" content={row.kind === "heading" ? undefined : tooltipOf(row, index)}>
                <div
                  data-row-key={row.key}
                  data-page-row={row.kind === "page" ? pageRowKey(row.source, row.path, row.page_id ?? "") : undefined}
                  className={`library-row library-row-${row.kind}${isCurrent(row) ? " library-row-current" : ""}`}
                  role={row.kind === "heading" ? "presentation" : "row"}
                  aria-level={row.kind === "heading" ? undefined : row.level + 1}
                  aria-posinset={row.kind === "heading" ? undefined : tops.result[index]?.posinset}
                  aria-setsize={row.kind === "heading" ? undefined : tops.result[index]?.setsize}
                  aria-rowindex={index + 1}
                  aria-expanded={row.expandable ? row.expanded : undefined}
                  aria-busy={row.kind === "source" && busySources.has(row.source) ? true : undefined}
                  data-status={row.kind === "status" ? row.online?.status : undefined}
                  aria-label={row.kind === "heading" ? undefined : row.label}
                  aria-selected={row.kind === "page" ? isCurrent(row) : undefined}
                  style={{ top: tops.result[index]?.top, height: tops.result[index]?.height, paddingLeft: 4 + 12 * Math.min(row.level, 6) }}
                  onClick={() => {
                    if (row.kind === "heading" || row.kind === "status" || row.kind === "icons" || row.kind === "ready") {
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
                    if (
                      (row.kind !== "page" && !(row.kind === "template" && row.online?.loadable)) ||
                      event.button !== 0 ||
                      (event.target as HTMLElement).closest("button")
                    ) {
                      return;
                    }
                    if (row.kind === "template") {
                      // A template row dragged onto the drawing loads it, as Load does (UX Q12).
                      const picture = event.currentTarget.querySelector<HTMLImageElement>("img.library-document-picture")?.src ?? null;
                      watchCanvasDrag(event, () => canvasRef.current, row.name, () => "import", () => {
                        void loadTemplate(row);
                      }, picture, slotDragOf(loadContext, () => templateSlotSource(row.source, row.path, sourceNameOf(row)), { eager: false }));
                      return;
                    }
                    const picture = event.currentTarget.querySelector<HTMLImageElement>("img.library-page-picture")?.src ?? null;
                    watchCanvasDrag(event, () => canvasRef.current, row.name, () => "import", (at) => {
                      void importPage(row, at);
                    }, picture, pageSlotDrag(loadContext, pageOf(row)));
                  }}
                  tabIndex={row.kind === "heading" ? undefined : active?.index === index ? 0 : -1}
                >
                  {row.kind === "heading" ? (
                    <span className="library-heading">{row.name}</span>
                  ) : isOnlineKind(row.kind) ? (
                    <OnlineRowContent row={row} handlers={handlers} />
                  ) : (
                    <>
                      {row.kind !== "page" ? (
                        <span className="library-chevron" aria-hidden="true">
                          {row.expandable ? row.expanded ? <ChevronDown {...GLYPH} /> : <ChevronRight {...GLYPH} /> : null}
                        </span>
                      ) : null}
                      {row.unreadable ? (
                        <TriangleAlert {...GLYPH} aria-hidden="true" />
                      ) : row.kind === "source" && row.online ? (
                        <OnlineSourceGlyph row={row} />
                      ) : row.kind === "source" || row.kind === "folder" ? (
                        <Folder {...GLYPH} aria-hidden="true" />
                      ) : (
                        <PagePicture row={row} />
                      )}
                      <span className="library-names">
                        <span className={row.open ? "library-name library-open" : "library-name"}>
                          <Highlighted name={row.name} matched={row.matched} />
                          {row.chip ? <span className="library-chip">{row.chip}</span> : null}
                          {row.kind === "source" && row.online?.token.private ? (
                            <Lock {...SMALL_GLYPH} aria-hidden="true" className="library-lock" />
                          ) : null}
                        </span>
                        {row.kind === "document" || row.kind === "page" || row.unreadable ? (
                          <span className="library-line">{row.line}</span>
                        ) : null}
                      </span>
                      {row.kind === "source" && row.online ? (
                        <OnlineSourceCells row={row} handlers={handlers} />
                      ) : row.kind === "source" || row.kind === "folder" ? (
                        row.unreadable ? null : <span className="library-count">{row.line}</span>
                      ) : null}
                      {row.kind === "page" ? (
                        <IntoSlotButton
                          ctx={loadContext}
                          look="icon"
                          open={slotList?.key === row.key}
                          onToggle={() =>
                            setSlotList((open) => (open?.key === row.key ? null : { key: row.key, height: intoSlotListHeight(loadContext) }))
                          }
                        />
                      ) : null}
                      {row.kind === "page" ? (
                        <Tooltip side="left" content="Import page">
                          <button
                          type="button"
                          className="library-action"
                          aria-label={`Import page ${row.name}`}
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
                          <Import {...GLYPH} />
                        </button>
                        </Tooltip>
                      ) : null}
                      {row.removable && !row.online ? (
                        <Tooltip side="left" content={armed === row.key ? library_remove_armed_text(row.name) : `Remove ${row.name} from the library`}>
                          <button
                          type="button"
                          className={armed === row.key ? "library-action library-armed" : "library-action"}
                          tabIndex={-1}
                          aria-label={armed === row.key ? library_remove_armed_text(row.name) : `Remove ${row.name} from the library`}
                          onClick={(event) => {
                            event.stopPropagation();
                            void remove(row);
                          }}
                        >
                          <X {...GLYPH} />
                        </button>
                        </Tooltip>
                      ) : null}
                    </>
                  )}
                </div>
              </Tooltip>
            ))}
          </div>
        )}
      </div>
      {isTauri && addingSource ? (
        <AddSourceForm
          onAdded={(key) => {
            setAddingSource(false);
            setKeyStoreLine("");
            // The new source is opened and focused: its list was fetched by the check that added it.
            focusWhenListed.current = `s:${key}\u001f`;
            setExpanded((now) => [...new Set([...now, `s:${key}\u001f`])]);
            refreshNow.current();
          }}
          onCancel={() => {
            focusAddSource.current = true;
            setAddingSource(false);
          }}
        />
      ) : null}
      {isTauri && tokenFor ? (
        <TokenForm
          key={tokenFor.source}
          source={tokenFor.source}
          name={tokenFor.name}
          hasToken={tokenFor.hasToken}
          onDone={(cleanup) => {
            focusWhenListed.current = `s:${tokenFor.source}\u001f`;
            setKeyStoreLine(cleanup?.key_store ?? "");
            setTokenFor(null);
            refreshNow.current();
          }}
          onCancel={() => {
            focusWhenListed.current = `s:${tokenFor.source}\u001f`;
            setTokenFor(null);
            refreshNow.current();
          }}
        />
      ) : null}
      {isTauri ? (
        <div className="library-footer">
          <div className="library-footer-buttons">
            <Button icon={FolderPlus} className="min-w-[118px]" onClick={() => void addFolder()}>
              Add folder…
            </Button>
            {addingSource || tokenFor ? null : (
              <Button
                icon={Plus}
                ref={addSourceRef}
                className="min-w-[118px]"
                onClick={() => {
                  setTokenFor(null);
                  setAddingSource(true);
                }}
              >
                Add source…
              </Button>
            )}
          </div>
          {refusal ? (
            <AlertLine>
              {refusal}
            </AlertLine>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
