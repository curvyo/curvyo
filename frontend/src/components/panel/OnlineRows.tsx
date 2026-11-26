// The rows of the Library tab's Online group (`specs/0138-template-library` UX notes sections 2 to
// 8): the ready Curvyo catalogue, the added sources, templates with their licence line, icon sets,
// icon cells and the state lines. Every word is Rust's (`curvyo-ui-core::library_online`); this
// file draws them and hands the maker's actions up. Nothing here fetches by itself except the
// picture of a visible row, which is part of the list fetch the maker asked for.
import {
  ChevronDown,
  ChevronRight,
  CircleAlert,
  FileText,
  Globe,
  GitBranch,
  Import,
  KeyRound,
  LoaderCircle,
  Plus,
  RefreshCw,
  Search,
  TriangleAlert,
  WifiOff,
  X,
} from "lucide-react";
import { type KeyboardEvent, useEffect, useState } from "react";

import type { IconCell, LibraryRow } from "@/lib/libraryApi";
import { onlineIconSvg, onlineThumbnail } from "@/lib/onlineApi";

/** An icon, drawn from its SVG as an image: nothing in it can run. */
function IconPicture({ source, cell }: { source: string; cell: IconCell }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    let made: string | null = null;
    // The cells of one screen are asked for together (`onlineIconSvg` collects them for 50 ms).
    void onlineIconSvg(source, cell.prefix, cell.name).then((svg) => {
      if (live && svg) {
        made = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
        setUrl(made);
      }
    });
    return () => {
      live = false;
      if (made) {
        URL.revokeObjectURL(made);
      }
    };
  }, [source, cell.prefix, cell.name]);
  return url ? <img src={url} alt="" width={24} height={24} draggable={false} /> : null;
}

/** A template's catalogue thumbnail, 40 x 28, loaded once its row is drawn. */
function Thumbnail({ row }: { row: LibraryRow }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!row.picture) {
      return;
    }
    let live = true;
    let made: string | null = null;
    const timer = window.setTimeout(() => {
      void onlineThumbnail(row.source, row.path.join("/")).then(
        (png) => {
          if (live && png.length > 0) {
            made = URL.createObjectURL(new Blob([png as BlobPart], { type: "image/png" }));
            setUrl(made);
          }
        },
        () => undefined,
      );
    }, 100);
    return () => {
      live = false;
      window.clearTimeout(timer);
      if (made) {
        URL.revokeObjectURL(made);
      }
    };
  }, [row.picture, row.source, row.path]);
  if (row.unreadable) {
    return <TriangleAlert size={16} aria-hidden="true" className="library-document-placeholder" />;
  }
  return url ? (
    <img className="library-document-picture" src={url} alt="" draggable={false} />
  ) : (
    <FileText size={16} aria-hidden="true" className="library-document-placeholder" />
  );
}

export interface OnlineHandlers {
  /** Add on the ready Curvyo catalogue's entry. */
  addReady: () => void;
  /** Refresh of a source row. */
  refresh: (row: LibraryRow) => void;
  /** Change token or Add token of a source row: opens the token form. */
  token: (row: LibraryRow) => void;
  /** Remove of a source row (armed first). */
  remove: (row: LibraryRow) => void;
  /** Load of a template row. */
  load: (row: LibraryRow) => void;
  /** Opens a licence address in the system browser. */
  licence: (url: string) => void;
  /** Imports an icon (a double-click, Enter or Space, or a drag handed `at`). */
  importIcon: (row: LibraryRow, cell: IconCell, at: { x: number; y: number } | null) => void;
  /** A press on a cell starts a possible drag onto the drawing. */
  dragIcon: (event: React.PointerEvent, row: LibraryRow, cell: IconCell) => void;
  /** Whether the row's Remove is armed, and the words of its arming. */
  armed: string | null;
  armedText: (row: LibraryRow) => string;
  /** The source keys whose Refresh runs now. */
  refreshing: ReadonlySet<string>;
  /** The key of the cell that has the focus, if any. */
  onCellFocus: (key: string) => void;
  /** Up or Down past the edge of an icon grid: the tree's row before or after `row`. */
  leaveGrid: (row: LibraryRow, delta: 1 | -1) => void;
}

/** The chevron cell of an expandable row. */
function Chevron({ row }: { row: LibraryRow }) {
  return (
    <span className="library-chevron" aria-hidden="true">
      {row.expandable ? row.expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} /> : null}
    </span>
  );
}

function cellKeys(event: KeyboardEvent<HTMLButtonElement>, row: LibraryRow, cell: IconCell, h: OnlineHandlers) {
  const button = event.currentTarget;
  const rowEl = button.closest<HTMLElement>("[data-row-key]");
  const cells = [...(rowEl?.querySelectorAll<HTMLElement>("[data-icon-cell]") ?? [])];
  const at = cells.indexOf(button);
  const column = at;
  const goto = (el: Element | null) => {
    const target = el?.querySelectorAll<HTMLElement>("[data-icon-cell]");
    if (target && target.length > 0) {
      target[Math.min(column, target.length - 1)]?.focus();
      return true;
    }
    return false;
  };
  if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
    event.preventDefault();
    event.stopPropagation();
    const next = cells[at + (event.key === "ArrowRight" ? 1 : -1)];
    if (next) {
      next.focus();
    } else {
      // Past the end of a row: the next or the previous icon row, at its first or last cell; Left
      // before the first cell of the grid goes back to the row.
      const sibling = event.key === "ArrowRight" ? rowEl?.nextElementSibling : rowEl?.previousElementSibling;
      const more = sibling?.querySelectorAll<HTMLElement>("[data-icon-cell]");
      if (more && more.length > 0) {
        (event.key === "ArrowRight" ? more[0] : more[more.length - 1])?.focus();
      } else if (event.key === "ArrowLeft") {
        rowEl?.focus();
      }
    }
  } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    event.stopPropagation();
    const sibling = event.key === "ArrowDown" ? rowEl?.nextElementSibling : rowEl?.previousElementSibling;
    if (!(sibling?.querySelector("[data-icon-cell]") && goto(sibling))) {
      h.leaveGrid(row, event.key === "ArrowDown" ? 1 : -1);
    }
  } else if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    event.stopPropagation();
    h.importIcon(row, cell, null);
  } else if (event.key === "Escape") {
    // Back to the row, whose own handler returns to the canvas.
    rowEl?.focus();
  }
}

/** The content of one Online row (the positioned wrapper is the tab's). */
export function OnlineRowContent({ row, handlers: h }: { row: LibraryRow; handlers: OnlineHandlers }) {
  const extra = row.online;
  if (!extra) {
    return null;
  }
  if (row.kind === "ready") {
    return (
      <>
        <span className="library-chevron" aria-hidden="true" />
        <Globe size={16} aria-hidden="true" style={{ opacity: 0.55 }} />
        <span className="library-names">
          <span className="library-name" style={{ color: "var(--panel-muted-fg)" }}>
            <span className="library-name-text">{row.name}</span>
          </span>
        </span>
        <button
          type="button"
          className="library-online-add"
          aria-label={`Add ${row.name}`}
          title={`Add ${row.name}\n${extra.host}`}
          tabIndex={-1}
          onClick={(event) => {
            event.stopPropagation();
            h.addReady();
          }}
        >
          Add
        </button>
      </>
    );
  }
  if (row.kind === "status") {
    const Glyph =
      extra.status === "loading" ? LoaderCircle : extra.status === "offline" ? WifiOff : extra.status === "search" ? Search : CircleAlert;
    return (
      <>
        <span className="library-chevron" aria-hidden="true" />
        <Glyph
          size={14}
          aria-hidden="true"
          className={extra.status === "loading" ? "motion-safe:animate-spin" : undefined}
          style={{ color: extra.status === "failed" ? "var(--destructive)" : "var(--panel-muted-fg)" }}
        />
        <span className="library-line">{row.name}</span>
      </>
    );
  }
  if (row.kind === "icons") {
    return (
      <span className="library-icon-cells" role="group" aria-label={row.label}>
        {extra.cells.map((cell) => (
          <button
            key={`${cell.prefix}:${cell.name}`}
            type="button"
            data-icon-cell
            className="library-icon-cell"
            aria-label={cell.label}
            title={cell.tooltip}
            tabIndex={-1}
            onFocus={() => h.onCellFocus(row.key)}
            onKeyDown={(event) => cellKeys(event, row, cell, h)}
            onDoubleClick={(event) => {
              event.stopPropagation();
              h.importIcon(row, cell, null);
            }}
            onPointerDown={(event) => h.dragIcon(event, row, cell)}
            onClick={(event) => event.stopPropagation()}
          >
            <IconPicture source={row.source} cell={cell} />
          </button>
        ))}
      </span>
    );
  }
  if (row.kind === "set") {
    return (
      <>
        <Chevron row={row} />
        <span className="library-set-samples" aria-hidden="true">
          {extra.samples.slice(0, 3).map((cell) => (
            <IconPicture key={cell.name} source={row.source} cell={cell} />
          ))}
        </span>
        <span className="library-names">
          <span className="library-name">
            <span className="library-name-text">{row.name}</span>
          </span>
          <span className="library-line library-licence-line">
            <LicenceLink extra={extra} h={h} />
            {extra.author ? ` · ${extra.author}` : null}
            {` · ${extra.size}`}
          </span>
        </span>
      </>
    );
  }
  if (row.kind === "template") {
    return (
      <>
        <Chevron row={row} />
        <Thumbnail row={row} />
        <span className="library-names">
          <span className="library-name">
            <span className="library-name-text">{row.name}</span>
          </span>
          {row.unreadable ? (
            <span className="library-line">{row.line}</span>
          ) : (
            <span className="library-line library-licence-line">
              <LicenceLink extra={extra} h={h} />
              {extra.author ? ` · ${extra.author}` : null}
              {` · ${extra.size}`}
            </span>
          )}
          {extra.font_note ? <span className="library-line">{extra.font_note}</span> : null}
        </span>
        {extra.loadable ? (
          <button
            type="button"
            className="library-action"
            aria-label={`Load ${row.name}`}
            title="Load template"
            tabIndex={-1}
            onClick={(event) => {
              event.stopPropagation();
              h.load(row);
            }}
          >
            <Import size={16} />
          </button>
        ) : null}
      </>
    );
  }
  return null;
}

function LicenceLink({ extra, h }: { extra: NonNullable<LibraryRow["online"]>; h: OnlineHandlers }) {
  if (!extra.licence_url) {
    return <span>{extra.licence}</span>;
  }
  // A link in the line's text (not a button box), so a cut line ends in an ellipsis; the host opens
  // the address in the system browser, the page never navigates.
  const open = (event: React.SyntheticEvent) => {
    event.preventDefault();
    event.stopPropagation();
    h.licence(extra.licence_url);
  };
  return (
    <a
      className="library-licence-link"
      href={extra.licence_url}
      title={extra.licence_url}
      tabIndex={-1}
      draggable={false}
      onClick={open}
      onAuxClick={(event) => event.preventDefault()}
      onPointerDown={(event) => event.stopPropagation()}
    >
      {extra.licence}
    </a>
  );
}

/** The state and the Refresh and Remove cells of an online source row. */
export function OnlineSourceCells({ row, handlers: h }: { row: LibraryRow; handlers: OnlineHandlers }) {
  const extra = row.online;
  if (!extra) {
    return null;
  }
  const running = h.refreshing.has(row.source);
  return (
    <>
      <span className="library-count">{extra.state}</span>
      <button
        type="button"
        className={`library-action${running || row.expanded ? " library-action-shown" : ""}`}
        aria-label={`Refresh ${row.name}`}
        title="Refresh"
        tabIndex={-1}
        onClick={(event) => {
          event.stopPropagation();
          h.refresh(row);
        }}
      >
        <RefreshCw size={16} className={running ? "motion-safe:animate-spin" : undefined} />
      </button>
      {extra.token.action ? (
        <button
          type="button"
          className={`library-action${extra.token.attention ? " library-action-shown" : ""}`}
          aria-label={`${extra.token.action} ${row.name}`}
          title={extra.token.action}
          tabIndex={-1}
          onClick={(event) => {
            event.stopPropagation();
            h.token(row);
          }}
        >
          <KeyRound size={16} />
        </button>
      ) : null}
      <button
        type="button"
        className={h.armed === row.key ? "library-action library-armed" : "library-action"}
        tabIndex={-1}
        aria-label={h.armed === row.key ? h.armedText(row) : `Remove ${row.name} from the library`}
        title={h.armed === row.key ? h.armedText(row) : `Remove ${row.name} from the library`}
        onClick={(event) => {
          event.stopPropagation();
          h.remove(row);
        }}
      >
        <X size={16} />
      </button>
    </>
  );
}

/** The glyph of a source row. */
export function OnlineSourceGlyph({ row }: { row: LibraryRow }) {
  return row.online?.glyph === "git" ? (
    <GitBranch size={16} aria-hidden="true" />
  ) : (
    <Globe size={16} aria-hidden="true" />
  );
}

export { Plus };
