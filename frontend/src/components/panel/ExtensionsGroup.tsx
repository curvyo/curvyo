import { Trash2 } from "lucide-react";

import { ToolbarSwitch } from "@/components/ToolbarSwitch";
import { Tooltip } from "@/components/ui/tooltip";
import type { WorkbenchesApi } from "@/hooks/useWorkbenches";

/**
 * The Extensions group at the end of the File tab (`specs/0093` criterion 11, UX notes §7): each
 * compiled-in extension with its version and an enable switch (an app setting, not an undo step),
 * and the manufacturings whose process is off, by name, each with its trash (§8).
 */
export function ExtensionsGroup({ workbenches }: { workbenches: WorkbenchesApi }) {
  const { list } = workbenches;
  if (list.extensions.length === 0) {
    return null;
  }
  return (
    <section aria-label="Extensions" className="mt-4 flex flex-col gap-1">
      <h3 className="text-sm text-[var(--toolbar-icon)]">Extensions</h3>
      {list.extensions.map((extension) => (
        <div key={extension.id} className="flex h-11 items-center justify-between gap-2">
          <div className="flex min-w-0 flex-col">
            <span className="truncate text-sm">{extension.name}</span>
            <span className="text-xs text-[var(--panel-muted-fg)]">{extension.version}</span>
          </div>
          <ToolbarSwitch
            label={extension.name}
            shortLabel=""
            tooltip={extension.name}
            checked={extension.enabled}
            onCheckedChange={(enabled) => workbenches.setExtensionEnabled(extension.id, enabled)}
          />
        </div>
      ))}
      {list.disabled.map((manufacturing) => (
        <div key={manufacturing.id} className="flex h-9 items-center justify-between gap-2 opacity-60">
          <span className="truncate text-sm">{manufacturing.label} (disabled)</span>
          <Tooltip side="left" content={`Delete ${manufacturing.label}`}>
            <button
              type="button"
              aria-label={`Delete ${manufacturing.label}`}
              className="inline-flex h-7 w-6 items-center justify-center rounded-md text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              onClick={() => workbenches.deleteManufacturing(manufacturing.id)}
            >
              <Trash2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            </button>
          </Tooltip>
        </div>
      ))}
    </section>
  );
}
