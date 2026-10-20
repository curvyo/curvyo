// The Fonts tab (`specs/0074-text-objects` criteria 15 to 19 and 29; UX notes §4): a search
// field and one virtualised tree of the blocks Missing, Curvyo fonts, Included and This computer,
// each family set in its own face once it has been in view for 100 ms.
import { ChevronDown, ChevronRight, Search, TriangleAlert, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { invoke, isTauri } from "@/platform/host";
import {
  type FontBlockRows,
  type FontFace as ListedFace,
  type FontFamily,
  type FontsTabData,
  FOCUS_CURRENT_FONT_EVENT,
  blockCount,
  familyName,
  fontBlocks,
} from "@/lib/textText";

const HEADER_PX = 24;
const FAMILY_PX = 32;
const FACE_PX = 28;
/** A row loads its face once it has been in view this long, so fast scrolling loads nothing. */
const LOAD_AFTER_MS = 100;

type Row =
  | { key: string; kind: "header"; block: FontBlockRows; height: number }
  | { key: string; kind: "missing"; name: string; height: number }
  | { key: string; kind: "family"; family: FontFamily; height: number }
  | { key: string; kind: "face"; face: ListedFace; family: FontFamily; height: number };

/** The faces the tab holds at most: the least recently shown one goes first (UX notes §4, the
 * visible rows plus a margin). */
const MAX_LOADED_FACES = 64;

/** The CSS font family each listed face is set in, once its bytes are loaded, least recently
 * shown first. */
const loadedFaces = new Map<string, { name: string; face: FontFace }>();
const loadingFaces = new Set<string>();
let faceCounter = 0;

/** The CSS family of the face `id`, marked as shown just now; `undefined` until it is loaded. */
function cssFamilyOf(id: string): string | undefined {
  const entry = loadedFaces.get(id);
  if (entry) {
    loadedFaces.delete(id);
    loadedFaces.set(id, entry);
  }
  return entry?.name;
}

function loadFace(face: ListedFace, done: () => void): void {
  if (loadedFaces.has(face.id) || loadingFaces.has(face.id)) {
    return;
  }
  loadingFaces.add(face.id);
  faceCounter += 1;
  const name = `curvyo-face-${faceCounter}`;
  const source: Promise<ArrayBuffer> =
    face.block === "included"
      ? fetch(`${import.meta.env.BASE_URL}fonts/${face.id}`).then((r) => r.arrayBuffer())
      : invoke<ArrayBuffer>("font_bytes", { id: face.id });
  void source
    .then((buffer) => new FontFace(name, buffer).load())
    .then((loaded) => {
      document.fonts.add(loaded);
      loadedFaces.set(face.id, { name, face: loaded });
      while (loadedFaces.size > MAX_LOADED_FACES) {
        const [oldest, entry] = loadedFaces.entries().next().value as [string, { face: FontFace }];
        document.fonts.delete(entry.face);
        loadedFaces.delete(oldest);
      }
      done();
    })
    .catch(() => {})
    .finally(() => loadingFaces.delete(face.id));
}

interface FontsTabProps {
  data: FontsTabData;
  onPick: (face: ListedFace, byKeyboard: boolean) => void;
  onReturnFocus: () => void;
  /** The tab is shown: the Curvyo folder is read again (criterion 19). */
  onShown: () => void;
}

export function FontsTab({ data, onPick, onReturnFocus, onShown }: FontsTabProps) {
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const [collapsedCurrent, setCollapsedCurrent] = useState(false);
  const [focusKey, setFocusKey] = useState<string | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(400);
  const [, setLoadedRevision] = useState(0);
  const listRef = useRef<HTMLDivElement | null>(null);
  const searchRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    onShown();
  }, [onShown]);

  const blocks = useMemo(() => fontBlocks(data, search, isTauri), [data, search]);
  const currentFace = data.current?.index ?? null;
  const familyKey = (family: FontFamily) => `${family.block}:${family.family}`;
  const currentFamily = useMemo(() => {
    for (const block of blocks) {
      for (const family of block.families) {
        if (family.faces.some((face) => face.index === currentFace)) {
          return familyKey(family);
        }
      }
    }
    return null;
  }, [blocks, currentFace]);

  const rows = useMemo(() => {
    const out: Row[] = [];
    if (data.current?.missing) {
      out.push({
        key: "missing-header",
        kind: "header",
        block: { block: "curvyo", title: "Missing", families: [], total: 1 },
        height: HEADER_PX,
      });
      out.push({
        key: "missing",
        kind: "missing",
        name: `${data.current.family} ${data.current.style}`,
        height: FACE_PX,
      });
    }
    for (const block of blocks) {
      out.push({ key: `header:${block.block}`, kind: "header", block, height: HEADER_PX });
      for (const family of block.families) {
        const key = familyKey(family);
        out.push({ key, kind: "family", family, height: FAMILY_PX });
        const open =
          expanded.has(key) || (key === currentFamily && !collapsedCurrent && !search);
        if (open) {
          for (const face of family.faces) {
            out.push({ key: `face:${face.index}`, kind: "face", face, family, height: FACE_PX });
          }
        }
      }
    }
    return out;
  }, [blocks, data, expanded, currentFamily, collapsedCurrent, search]);

  const tops = useMemo(() => {
    const at: number[] = [];
    let y = 0;
    for (const row of rows) {
      at.push(y);
      y += row.height;
    }
    return { at, total: y };
  }, [rows]);

  // The list element comes and goes (the empty states), so it is observed by a callback ref; an
  // observer of a replaced element reports 0 px and only the first rows would be drawn.
  const [listEl, setListEl] = useState<HTMLDivElement | null>(null);
  const setList = useCallback((el: HTMLDivElement | null) => {
    listRef.current = el;
    setListEl(el);
  }, []);
  useEffect(() => {
    if (!listEl) {
      return undefined;
    }
    const observer = new ResizeObserver(() => {
      if (listEl.clientHeight > 0) {
        setHeight(listEl.clientHeight);
      }
    });
    observer.observe(listEl);
    return () => observer.disconnect();
  }, [listEl]);

  const first = Math.max(0, tops.at.findIndex((top, i) => top + rows[i].height > scrollTop) - 2);
  const visible: number[] = [];
  for (let i = first; i < rows.length && tops.at[i] < scrollTop + height + 64; i += 1) {
    visible.push(i);
  }

  // Faces of rows that stay in view load after 100 ms.
  const visibleKey = visible.map((i) => rows[i].key).join("|");
  useEffect(() => {
    const timer = window.setTimeout(() => {
      for (const i of visible) {
        const row = rows[i];
        const face =
          row.kind === "family" ? row.family.faces[0] : row.kind === "face" ? row.face : null;
        if (face) {
          loadFace(face, () => setLoadedRevision((r) => r + 1));
        }
      }
    }, LOAD_AFTER_MS);
    return () => window.clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visibleKey]);

  const focusable = rows.filter((row) => row.kind !== "header");
  const activeKey =
    focusKey && focusable.some((row) => row.key === focusKey)
      ? focusKey
      : (rows.find((row) => row.kind === "face" && row.face.index === currentFace)?.key ??
        focusable[0]?.key ??
        null);

  const focusRow = useCallback(
    (key: string) => {
      setFocusKey(key);
      const index = rows.findIndex((row) => row.key === key);
      const list = listRef.current;
      if (list && index >= 0) {
        const top = tops.at[index];
        if (top < list.scrollTop) {
          list.scrollTop = top;
        } else if (top + rows[index].height > list.scrollTop + list.clientHeight) {
          list.scrollTop = top + rows[index].height - list.clientHeight;
        }
      }
      window.requestAnimationFrame(() =>
        listRef.current?.querySelector<HTMLElement>(`[data-row-key="${CSS.escape(key)}"]`)?.focus(),
      );
    },
    [rows, tops],
  );

  // The Font button and Shift+Ctrl+T focus the current face row.
  useEffect(() => {
    const onFocusCurrent = () => {
      const current = rows.find((row) => row.kind === "face" && row.face.index === currentFace);
      const missing = rows.find((row) => row.kind === "missing");
      const target = missing ?? current ?? focusable[0];
      if (target) {
        focusRow(target.key);
      }
    };
    window.addEventListener(FOCUS_CURRENT_FONT_EVENT, onFocusCurrent);
    return () => window.removeEventListener(FOCUS_CURRENT_FONT_EVENT, onFocusCurrent);
  }, [rows, currentFace, focusable, focusRow]);

  const toggle = (family: FontFamily, open?: boolean) => {
    const key = familyKey(family);
    if (key === currentFamily && !expanded.has(key)) {
      setCollapsedCurrent((collapsed) => (open === undefined ? !collapsed : !open));
      return;
    }
    setExpanded((before) => {
      const next = new Set(before);
      const willOpen = open ?? !next.has(key);
      if (willOpen) {
        next.add(key);
      } else {
        next.delete(key);
      }
      return next;
    });
  };

  const pickFamily = (family: FontFamily, byKeyboard: boolean) => {
    toggle(family, true);
    onPick(family.faces[0], byKeyboard);
  };

  const onListKey = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const at = focusable.findIndex((row) => row.key === activeKey);
    const row = focusable[at];
    const move = (to: number) => {
      const target = focusable[Math.max(0, Math.min(focusable.length - 1, to))];
      if (target) {
        event.preventDefault();
        focusRow(target.key);
      }
    };
    const page = Math.max(1, Math.floor(height / FAMILY_PX) - 1);
    switch (event.key) {
      case "ArrowDown":
        return move(at + 1);
      case "ArrowUp":
        return move(at - 1);
      case "Home":
        return move(0);
      case "End":
        return move(focusable.length - 1);
      case "PageDown":
        return move(at + page);
      case "PageUp":
        return move(at - page);
      case "ArrowRight":
        event.preventDefault();
        if (row?.kind === "family") {
          const open = rows.some((r) => r.kind === "face" && r.family === row.family);
          if (open) {
            move(at + 1);
          } else {
            toggle(row.family, true);
          }
        }
        return;
      case "ArrowLeft":
        event.preventDefault();
        if (row?.kind === "face") {
          focusRow(familyKey(row.family));
        } else if (row?.kind === "family") {
          toggle(row.family, false);
        }
        return;
      case "Enter":
      case " ":
        event.preventDefault();
        if (row?.kind === "family") {
          pickFamily(row.family, true);
        } else if (row?.kind === "face") {
          onPick(row.face, true);
        }
        return;
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        onReturnFocus();
        return;
      default:
        if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
          event.preventDefault();
          setSearch((text) => text + event.key);
          searchRef.current?.focus();
        }
    }
  };

  const searching = search.trim().length > 0;
  const skipped = data.skipped.length;
  const headerRow = (row: Extract<Row, { kind: "header" }>, style: React.CSSProperties) => {
    const count =
      row.key === "missing-header"
        ? ""
        : row.block.block === "system" && isTauri && !data.systemRead
          ? "Reading…"
          : blockCount(row.block, searching, skipped);
    const tip =
      row.block.block === "curvyo" && skipped > 0 ? data.skipped.slice(0, 10).join("\n") : undefined;
    return (
      <div
        key={row.key}
        style={style}
        title={tip}
        className="flex items-center justify-between bg-[var(--panel-bg)] text-xs font-semibold text-[var(--panel-muted-fg)]"
      >
        <span>{row.block.title}</span>
        <span className="font-normal tabular-nums">{count}</span>
      </div>
    );
  };
  // The header of the block at the scroll top, once its own row has scrolled away, and how far
  // the next header pushes it up.
  const pinned = (() => {
    let at = -1;
    for (let i = 0; i < rows.length && tops.at[i] <= scrollTop; i += 1) {
      if (rows[i].kind === "header") {
        at = i;
      }
    }
    const row = rows[at];
    if (at < 0 || row.kind !== "header" || tops.at[at] >= scrollTop) {
      return null;
    }
    const next = rows.findIndex((r, i) => i > at && r.kind === "header");
    const push = next < 0 ? 0 : Math.min(0, tops.at[next] - scrollTop - HEADER_PX);
    return { row, push };
  })();
  const firstFamily = blocks.flatMap((block) => block.families)[0];

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="relative mb-2 h-7 shrink-0">
        <Search
          size={16}
          aria-hidden
          className="pointer-events-none absolute top-1.5 left-2 text-[var(--panel-muted-fg)]"
        />
        <input
          ref={searchRef}
          type="text"
          value={search}
          aria-label="Search fonts"
          placeholder="Search fonts"
          onChange={(event) => setSearch(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "ArrowDown" && focusable[0]) {
              event.preventDefault();
              focusRow(focusable[0].key);
            } else if (event.key === "Enter" && firstFamily) {
              event.preventDefault();
              pickFamily(firstFamily, false);
            } else if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              if (search) {
                setSearch("");
              } else {
                onReturnFocus();
              }
            }
          }}
          className="h-7 w-full rounded-[5px] bg-white pr-7 pl-8 text-sm outline-none ring-1 ring-border placeholder:text-[var(--field-placeholder)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        />
        {search ? (
          <button
            type="button"
            aria-label="Clear search"
            onClick={() => {
              setSearch("");
              searchRef.current?.focus();
            }}
            className="absolute top-0.5 right-0.5 flex size-6 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          >
            <X size={14} aria-hidden />
          </button>
        ) : null}
      </div>
      {data.faces.length === 0 && !(isTauri && !data.systemRead) ? (
        <p className="text-sm text-[var(--toolbar-icon)]">Loading fonts…</p>
      ) : searching && blocks.length === 0 ? (
        <p className="text-sm text-[var(--toolbar-icon)]">No font matches "{search.trim()}".</p>
      ) : (
        <div
          ref={setList}
          role="tree"
          aria-label="Fonts"
          onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
          onKeyDown={onListKey}
          className="relative min-h-[120px] flex-1 overflow-y-auto [scrollbar-gutter:stable]"
        >
          {pinned ? (
            // The header of the block at the scroll top stays pinned while its rows scroll
            // (UX notes §4); the next header pushes it up.
            <div
              aria-hidden
              className="sticky top-0 z-10 -mb-6 h-6"
              style={{ transform: pinned.push < 0 ? `translateY(${pinned.push}px)` : undefined }}
            >
              {headerRow(pinned.row, { position: "absolute", top: 0, left: 0, right: 0, height: HEADER_PX })}
            </div>
          ) : null}
          <div style={{ height: tops.total, position: "relative" }}>
            {visible.map((i) => {
              const row = rows[i];
              const style = { position: "absolute" as const, top: tops.at[i], left: 0, right: 0, height: row.height };
              if (row.kind === "header") {
                return headerRow(row, style);
              }
              const focused = row.key === activeKey;
              if (row.kind === "missing") {
                return (
                  <div
                    key={row.key}
                    role="treeitem"
                    aria-level={2}
                    aria-selected
                    data-row-key={row.key}
                    tabIndex={focused ? 0 : -1}
                    onFocus={() => setFocusKey(row.key)}
                    style={style}
                    className="flex items-center gap-1 border-l-[3px] border-[var(--editor-accent)] bg-[var(--value-fill)] pl-[17px] text-sm outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
                  >
                    <TriangleAlert size={16} aria-hidden className="shrink-0 text-[var(--field-invalid)]" />
                    <span className="min-w-0 flex-1 truncate">{row.name}</span>
                    <span className="pr-2 text-xs text-[var(--field-invalid)]">missing</span>
                  </div>
                );
              }
              if (row.kind === "family") {
                const open = rows[i + 1]?.kind === "face" && rows[i + 1].key !== row.key;
                const face = row.family.faces[0];
                const cssFamily = cssFamilyOf(face.id);
                const holdsCurrent = row.family.faces.some((f) => f.index === currentFace);
                const faces = row.family.faces.length;
                return (
                  <div
                    key={row.key}
                    role="treeitem"
                    aria-level={1}
                    aria-expanded={open}
                    aria-label={familyName(row.family)}
                    data-row-key={row.key}
                    tabIndex={focused ? 0 : -1}
                    onFocus={() => setFocusKey(row.key)}
                    onClick={(event) => pickFamily(row.family, event.detail === 0)}
                    title={`${row.family.family}, ${faces === 1 ? "1 face" : `${faces} faces`}${holdsCurrent && !open ? "\nContains the current font" : ""}`}
                    style={style}
                    className="flex cursor-default items-center gap-1 rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
                  >
                    <span
                      aria-hidden
                      onClick={(event) => {
                        event.stopPropagation();
                        toggle(row.family);
                      }}
                      className="flex h-8 w-6 shrink-0 items-center justify-center"
                    >
                      {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                    </span>
                    <span
                      className="min-w-0 flex-1 truncate"
                      style={cssFamily ? { fontFamily: cssFamily, fontSize: 16 } : { fontSize: 14 }}
                    >
                      {row.family.family}
                    </span>
                    {holdsCurrent && !open ? (
                      <span aria-hidden className="size-1.5 rotate-45 bg-[var(--editor-accent)]" />
                    ) : null}
                    <span className="pr-2 text-xs tabular-nums text-[var(--panel-muted-fg)]">{faces}</span>
                  </div>
                );
              }
              const current = row.face.index === currentFace;
              const cssFamily = cssFamilyOf(row.face.id);
              return (
                <div
                  key={row.key}
                  role="treeitem"
                  aria-level={2}
                  aria-selected={current}
                  data-row-key={row.key}
                  tabIndex={focused ? 0 : -1}
                  onFocus={() => setFocusKey(row.key)}
                  onClick={(event) => onPick(row.face, event.detail === 0)}
                  style={style}
                  className={`flex cursor-default items-center rounded-[5px] pl-5 text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${
                    current ? "border-l-[3px] border-[var(--editor-accent)] bg-[var(--value-fill)] pl-[17px]" : ""
                  }`}
                >
                  <span className="min-w-0 truncate" style={cssFamily ? { fontFamily: cssFamily, fontSize: 14 } : { fontSize: 14 }}>
                    {row.face.style}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
