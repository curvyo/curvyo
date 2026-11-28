
import { Puzzle } from "lucide-react";
import type { WorkbenchesApi } from "@/hooks/useWorkbenches";
import { ToggleButton } from "@/components/ui/toggle-button";
import { SECTION_HEADING } from "@/components/ui/looks";

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
      <h3 className={SECTION_HEADING}>Extensions</h3>
      {list.extensions.map((extension) => (
        <div key={extension.id} className="flex h-11 items-center justify-between gap-2">
          <div className="flex min-w-0 flex-col">
            <span className="truncate text-sm">{extension.name}</span>
            <span className="text-xs text-[var(--panel-muted-fg)]">{extension.version}</span>
          </div>
          <ToggleButton icon={Puzzle} look="icon"
            label={extension.name}
            
            tooltip={extension.name}
            pressed={extension.enabled}
            onPressedChange={(enabled) => workbenches.setExtensionEnabled(extension.id, enabled)}
          />
        </div>
      ))}
    </section>
  );
}
