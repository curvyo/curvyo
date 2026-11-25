// The Came from line (`specs/0137-document-library` criterion 7, UX notes section 8): the last
// block of the Style tab for the selection, and of the Document tab for the active page; "Open
// source" while the source document is found in a known source. The words come from Rust.
import { FileInput, LoaderCircle } from "lucide-react";

import { library_page_gone_text, library_source_gone_text } from "@/wasm-bindings/curvyo_editor_wasm.js";
import { useEffect, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { type CameFromRecord, type SourcePlace, findLibrarySource, openLibraryPath, scanLibrary } from "@/lib/libraryApi";
import { openSource } from "@/lib/libraryOpenSource";
import { openLicence } from "@/lib/onlineApi";
import { isTauri } from "@/platform/host";
import { activateWhenOpen } from "@/lib/libraryStore";

interface CameFromView {
  text: string;
  tooltip: string | null;
  open: boolean;
  record: CameFromRecord | null;
  /** The licence line of an online item ("CC BY 4.0 · Jane Doe"). */
  online?: { licence: string; licence_url: string; author: string } | null;
}

interface CameFromBlockProps {
  getSession: () => WasmSession | null;
  /** The page whose record to show; the selection's when absent. */
  pageId?: string;
  /** Re-reads after every session change. */
  revision: number;
  /** Opens a document of the open workspace behind the unsaved-changes check. */
  openInWorkspace: (path: string[]) => void;
  /** Browser: opens the document `path` of the stored workspace `source`; false when gone. */
  openStored: (source: string, path: string[]) => Promise<boolean>;
  /** The open workspace document's path, or null. */
  openPath: string[] | null;
  showNotice: (text: string, ms?: number, source?: "canvas" | "panel", kind?: "status" | "alert") => void;
}

export function CameFromBlock({ getSession, pageId, revision, openInWorkspace, openStored, openPath, showNotice }: CameFromBlockProps) {
  const [view, setView] = useState<CameFromView | null>(null);
  const [found, setFound] = useState<SourcePlace | null>(null);
  // Open source reads the sources again before it opens: past 150 ms the button shows it is busy
  // (the "busy state over 150 ms" of Add font) and a second press does nothing.
  const [busy, setBusy] = useState(false);
  const running = useRef(false);

  useEffect(() => {
    const session = getSession();
    const now = Date.now();
    const offset = -new Date().getTimezoneOffset();
    const text = session ? (pageId ? session.page_came_from_view(pageId, now, offset) : session.came_from_view(now, offset)) : "";
    setView(text ? (JSON.parse(text) as CameFromView) : null);
  }, [getSession, pageId, revision]);

  const record = view?.open ? view.record : null;
  const key = record ? `${record.page_id}|${record.document}` : "";
  useEffect(() => {
    if (!record) {
      setFound(null);
      return;
    }
    let live = true;
    void findLibrarySource(record).then(
      (place) => live && setFound(place),
      () => live && setFound(null),
    );
    return () => {
      live = false;
    };
    // The record is compared by its key.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  if (!view) {
    return null;
  }

  const run = () => {
    if (!record || running.current) {
      return;
    }
    running.current = true;
    const timer = window.setTimeout(() => setBusy(true), 150);
    const done = () => {
      window.clearTimeout(timer);
      running.current = false;
      setBusy(false);
    };
    const pagesHave = (id: string) => {
      try {
        return (JSON.parse(getSession()?.pages_json() ?? "[]") as { id: string }[]).some((p) => p.id === id);
      } catch {
        return false;
      }
    };
    void openSource(record, found, {
      scan: scanLibrary,
      find: findLibrarySource,
      desktop: isTauri,
      isOpen: (place) =>
        place.in_workspace && openPath !== null && place.path.join("/") === openPath.join("/"),
      activateNow: (id) => pagesHave(id) && (getSession()?.activate_page(id) ?? false),
      activateWhenOpen: (r) => activateWhenOpen(r.page_id, r.document, r.page, getSession()),
      openInWorkspace,
      openStored,
      openPath: openLibraryPath,
      pageGone: (r) => showNotice(library_page_gone_text(r.document, r.page), 5000, "canvas", "status"),
      sourceGone: (r) => {
        setFound(null);
        showNotice(library_source_gone_text(r.document), 8000, "panel", "alert");
      },
    }).finally(done);
  };

  return (
    <div role="group" aria-label="Came from" className="came-from-block">
      <p className="came-from-line" title={view.tooltip ?? view.text}>
        <FileInput size={16} aria-hidden="true" />
        <span>{view.text}</span>
      </p>
      {view.online ? (
        <p className="library-line" style={{ margin: 0 }}>
          {view.online.licence_url && isTauri ? (
            <button
              type="button"
              className="library-licence-link"
              title={view.online.licence_url}
              onClick={() => void openLicence(view.online?.licence_url ?? "").catch(() => undefined)}
            >
              {view.online.licence}
            </button>
          ) : (
            view.online.licence
          )}
          {view.online.author ? ` · ${view.online.author}` : ""}
        </p>
      ) : null}
      {found ? (
        <button type="button" className="came-from-open" aria-busy={busy || undefined} onClick={run}>
          {busy ? <LoaderCircle size={16} aria-hidden className="motion-safe:animate-spin" /> : null}
          Open source
        </button>
      ) : null}
    </div>
  );
}
