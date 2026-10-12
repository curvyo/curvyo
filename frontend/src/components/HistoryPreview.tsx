import { useEffect, useState } from "react";

import { NO_PREVIEW, previewBanner, previewReadout, sinceText } from "@/lib/historyPreview";
import { setPreviewState, usePreviewState } from "@/lib/historyPreviewStore";
import type { WasmSession } from "@/lib/editorSession";

/**
 * The persistent line of the canvas notice slot while a version from the History list shows
 * (`specs/0042-history-branches` criteria 17 and 19): an object preview's readout, no control; a
 * document preview's banner with **Back to live** and the count of steps that arrived meanwhile.
 * `role="status"`, so each newly pressed row is announced.
 */
export function HistoryPreviewLine({ getSession }: { getSession: () => WasmSession | null }) {
  const state = usePreviewState();
  const [now] = useState(() => new Date());
  if (state.kind === "") {
    return <div role="status" aria-live="polite" className="sr-only" />;
  }
  const box = {
    background: "var(--toolbar-bg)",
    color: "var(--toolbar-icon)",
    boxShadow: "var(--panel-elevation-shadow)",
  };
  if (state.kind === "object") {
    return (
      <div role="status" aria-live="polite" className="rounded-md p-2 text-xs" style={box}>
        {previewReadout(state, now)}
      </div>
    );
  }
  const since = sinceText(state);
  return (
    <div
      role="status"
      aria-live="polite"
      className="pointer-events-auto flex items-center gap-3 rounded-md p-2 text-xs"
      style={box}
    >
      <span>
        {previewBanner(state, now)}
        {since !== "" ? <span className="text-[var(--panel-muted-fg)]"> {since}</span> : null}
      </span>
      <button
        type="button"
        onClick={() => {
          getSession()?.history_preview_end();
          setPreviewState(NO_PREVIEW);
        }}
        className="h-7 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] px-2 text-xs text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        Back to live
      </button>
    </div>
  );
}

/**
 * The signs of a document preview over the viewport that do not depend on the banner: a 2 px inset
 * `--preview-new` frame that takes no pointer events, and the plain arrow over the canvas
 * (criterion 19). The dimmed, inert tool rail is the third sign (`ToolRail`'s `paused`).
 */
export function HistoryPreviewFrame() {
  const state = usePreviewState();
  const paused = state.kind === "document";
  useEffect(() => {
    if (paused) {
      document.body.dataset.historyPreview = "document";
    } else {
      delete document.body.dataset.historyPreview;
    }
    return () => {
      delete document.body.dataset.historyPreview;
    };
  }, [paused]);
  return paused ? (
    <div
      aria-hidden
      className="pointer-events-none absolute inset-0 z-40"
      style={{ boxShadow: "inset 0 0 0 2px var(--preview-new)" }}
    />
  ) : null;
}
