import { Magnet, X } from "lucide-react";
import { Tooltip } from "@/components/ui/tooltip";
import type { SnappingApi } from "@/hooks/useSnapping";

interface SnappingToolbarProps {
  snapping: SnappingApi;
}

/**
 * A compact snapping toolbar shown above the Properties panel header.
 * Displays a magnet icon toggle for snapping enabled, and shows when
 * snapping is suspended (Ctrl+Alt) or a drag is in flight.
 */
export function SnappingToolbar({ snapping }: SnappingToolbarProps) {
  const { view } = snapping;

  return (
    <div className="flex h-6 shrink-0 items-center justify-between gap-2 border-b border-[color-mix(in_srgb,var(--toolbar-icon)_15%,transparent)] bg-[var(--panel-bg)] px-3 py-1">
      <div className="flex items-center gap-2">
        <Tooltip
          side="bottom"
          content={
            view.enabled
              ? view.suspended
                ? "Snapping suspended (Ctrl+Alt)"
                : "Snapping on"
              : "Snapping off"
          }
        >
          <button
            type="button"
            aria-label={view.enabled ? "Disable snapping" : "Enable snapping"}
            aria-pressed={view.enabled}
            className={`flex size-6 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--panel-bg)] ${
              view.enabled && !view.suspended
                ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
                : "text-[var(--toolbar-icon)] hover:bg-[color-mix(in_srgb,var(--toolbar-icon)_10%,transparent)]"
            }`}
            onClick={() => snapping.toggle()}
          >
            {view.suspended ? (
              <X size={14} strokeWidth={2.5} />
            ) : (
              <Magnet size={14} strokeWidth={view.enabled ? 2.5 : 1.5} />
            )}
          </button>
        </Tooltip>
        {view.hasDrag && (
          <div className="h-1.5 w-1.5 rounded-full bg-[var(--editor-accent)]" aria-hidden="true" />
        )}
      </div>
      <div className="flex items-center gap-1 text-xs text-[var(--panel-muted-fg)]">
        {view.objects && <span className="px-1">Objects</span>}
        {view.guides && <span className="px-1">Guides</span>}
        {view.nodes && <span className="px-1">Nodes</span>}
        {view.page && <span className="px-1">Page</span>}
      </div>
    </div>
  );
}