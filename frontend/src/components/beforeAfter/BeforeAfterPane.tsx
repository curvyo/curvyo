import { Maximize2, Minimize2, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import { BeforeAfterPictures } from "@/components/beforeAfter/BeforeAfterPictures";
import { PresetStrip } from "@/components/panel/PresetStrip";
import { Tooltip } from "@/components/ui/tooltip";
import type { BeforeAfterApi } from "@/hooks/useBeforeAfter";
import { operationIdText } from "@/lib/operationsView";
import { type PaneLayout, headerRows } from "@/lib/fontPreviewLayout";

const BUTTON =
  "flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";
const VIEW_CELLS = [
  { id: "split", content: "Split", label: "Split", tooltip: "" },
  { id: "side_by_side", content: "Side by side", label: "Side by side", tooltip: "" },
];
/** The id of the group's Before and after button, to give the focus back on Close. */
const NOOP = () => undefined;

interface PaneProps {
  api: BeforeAfterApi;
  layout: PaneLayout;
  /** The Full view or Split view switch of the canvas area. */
  onSwitchView: () => void;
  /** The canvas takes the focus back. */
  onReturnFocus: () => void;
}

/**
 * The Before and after side pane of the canvas area (`specs/0135-image-optimisation` criteria 11
 * to 13; `ux-notes.md` §7): header (View strip, the operation pick with two or more Image
 * operations, the subject line, the zoom readout, Full view, Close) and the two halves.
 * Escape with the focus inside closes it.
 */
export function BeforeAfterPane({ api, layout, onSwitchView, onReturnFocus }: PaneProps) {
  const { view } = api;
  const root = useRef<HTMLElement>(null);
  const [percent, setPercent] = useState(100);
  const onZoom = useCallback((next: number) => setPercent(next), []);
  const close = useCallback(() => {
    const inside = root.current?.contains(document.activeElement) ?? false;
    api.close();
    if (inside) {
      window.requestAnimationFrame(() => {
        const button = document.querySelector<HTMLElement>('[data-adjustments] [aria-label="Before and after"]');
        if (button) button.focus();
        else onReturnFocus();
      });
    }
  }, [api, onReturnFocus]);
  const closeRef = useRef(close);
  useEffect(() => {
    closeRef.current = close;
  });
  // Escape anywhere outside a control that handled it closes the pane, as the font preview does.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented && root.current?.contains(document.activeElement)) {
        event.preventDefault();
        closeRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  if (!view) return null;
  const two = headerRows(layout.width) === 2;
  return (
    <section ref={root} role="region" aria-label="Before and after" tabIndex={-1} className="flex min-h-0 min-w-0 flex-1 flex-col outline-none">
      <div className={`flex shrink-0 items-center gap-2 px-2 py-1 ${two ? "flex-wrap" : ""}`}>
        <div className="w-[180px] shrink-0">
          <PresetStrip
            label="View"
            width="w-full"
            tooltipSide="top"
            cells={VIEW_CELLS}
            value={view.view}
            onPress={(id) => api.setView(id === "side_by_side" ? "side_by_side" : "split")}
            onReturnFocus={NOOP}
          />
        </div>
        {view.operations.length >= 2 ? (
          <div data-pane-operations="">
            <PresetStrip
              label="Operation"
              width="w-auto"
              tooltipSide="top"
              cells={view.operations.map((op) => ({ id: operationIdText(op.id), content: op.name, label: op.name, tooltip: "" }))}
              value={(() => {
                const on = view.operations.find((op) => op.pressed);
                return on ? operationIdText(on.id) : null;
              })()}
              onPress={(id) => {
                const op = view.operations.find((o) => operationIdText(o.id) === id);
                if (op) api.pickOperation(op.id);
              }}
              onReturnFocus={NOOP}
            />
          </div>
        ) : null}
        <p title={view.subject} className="min-w-0 flex-1 truncate text-xs text-[var(--panel-muted-fg)]">
          {view.subject}
        </p>
        <span aria-label="Zoom" className="shrink-0 text-xs tabular-nums text-[var(--toolbar-icon)]">
          {percent} %
        </span>
        {layout.canSplit || layout.full ? (
          <Tooltip side="bottom" content={layout.full ? "Split view" : "Full view"}>
            <button type="button" aria-label={layout.full ? "Split view" : "Full view"} onClick={onSwitchView} className={BUTTON}>
              {layout.full ? <Minimize2 aria-hidden size={16} strokeWidth={1.5} /> : <Maximize2 aria-hidden size={16} strokeWidth={1.5} />}
            </button>
          </Tooltip>
        ) : null}
        <Tooltip side="bottom" content="Close">
          <button type="button" aria-label="Close" onClick={close} className={BUTTON}>
            <X aria-hidden size={16} strokeWidth={1.5} />
          </button>
        </Tooltip>
      </div>
      <BeforeAfterPictures api={api} view={view} onZoom={onZoom} />
    </section>
  );
}
