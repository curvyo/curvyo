import { useEffect, useSyncExternalStore } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { RAIL_CARD_CLASS, RAIL_CARD_STYLE } from "@/components/railCard";
import type { ManifestWorkbenchContribution } from "@/workbenches/registry";
import { WorkbenchIcon } from "@/workbenches/icons";
import { setViewTool, subscribeViewTool, toggleViewTool, viewTool } from "@/lib/viewTool";

/** The shell commands a toolbox button may name, with their key (`specs/0093` UX notes §3). */
const KEYS: Record<string, string> = { pan: "h", zoom: "z" };

interface ManifestToolboxProps {
  contribution: ManifestWorkbenchContribution;
  /** Fit page. */
  onFitPage: () => void;
  paused: boolean;
}

/**
 * The tool rail of a manufacturing workbench, drawn from its manifest's toolbox cards (ADR 0034
 * §4): each command a 40 px button of a 48 px card. Pan and Zoom are view tools (pressed while
 * active, H and Z on the canvas); Fit page shows the output page whole. Nothing selects.
 */
export function ManifestToolbox({ contribution, onFitPage, paused }: ManifestToolboxProps) {
  const pressed = useSyncExternalStore(subscribeViewTool, viewTool);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (event.ctrlKey || event.metaKey || event.altKey || target?.closest("input, textarea, [contenteditable]")) {
        return;
      }
      const key = event.key.toLowerCase();
      if (key === KEYS.pan) toggleViewTool("pan");
      if (key === KEYS.zoom) toggleViewTool("zoom");
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      setViewTool("none");
    };
  }, []);
  const run = (id: string) => {
    if (id === "pan" || id === "zoom") toggleViewTool(id);
    if (id === "fit_page") onFitPage();
  };
  return (
    <div
      inert={paused}
      className={`pointer-events-none absolute top-3 left-3 z-30 flex flex-col gap-2 ${paused ? "opacity-40" : ""}`}
    >
      {contribution.toolbox_cards.map((card) => (
        <div key={card.label} role="toolbar" aria-label={card.label} aria-orientation="vertical" className={RAIL_CARD_CLASS} style={RAIL_CARD_STYLE}>
          {card.commands.map((command) => {
            const toggles = command.id === "pan" || command.id === "zoom";
            return (
              <Tooltip key={command.id} side="right" content={command.label}>
                <button
                  type="button"
                  aria-label={command.label}
                  aria-pressed={toggles ? pressed === command.id : undefined}
                  className="inline-flex size-10 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)]"
                  onClick={() => run(command.id)}
                >
                  <WorkbenchIcon name={command.icon} />
                </button>
              </Tooltip>
            );
          })}
        </div>
      ))}
    </div>
  );
}
