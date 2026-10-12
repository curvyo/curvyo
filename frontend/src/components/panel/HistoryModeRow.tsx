import type { ReactNode } from "react";

import { ToggleGroup, type ToggleOption } from "@/components/ui/toggle-group";
import { MODE_TOOLTIP, modeCaption } from "@/lib/historyPreview";

type Mode = "preview" | "live";

const MODE_OPTIONS: readonly ToggleOption<Mode>[] = [
  {
    value: "preview",
    label: "Preview",
    tooltip: MODE_TOOLTIP,
    icon: <span className="text-sm">Preview</span>,
    width: 88,
  },
  {
    value: "live",
    label: "Live",
    tooltip: MODE_TOOLTIP,
    icon: <span className="text-sm">Live</span>,
    width: 88,
  },
];

/**
 * The Object scope's Mode row and the 32 px caption slot under it (`specs/0042-history-branches`
 * criterion 14): "Mode" in the label column, the Preview / Live switch in the control column, and a
 * slot that is always there, so nothing moves. It holds the caption of the position, or `action`
 * while an exploration lasts (criterion 12).
 */
export function HistoryModeRow({
  live,
  onChange,
  onReturnFocus,
  action,
}: {
  live: boolean;
  onChange: (live: boolean) => void;
  onReturnFocus: () => void;
  action?: ReactNode;
}) {
  return (
    <div className="mb-2 shrink-0">
      <div className="flex h-7 items-center">
        <span className="w-[60px] shrink-0 text-sm text-[var(--toolbar-icon)]">Mode</span>
        <ToggleGroup
          label="Mode"
          description={MODE_TOOLTIP}
          options={MODE_OPTIONS}
          value={live ? "live" : "preview"}
          onChange={(value) => onChange(value === "live")}
          itemWidth={88}
          itemHeight={26}
          onReturnFocus={onReturnFocus}
        />
      </div>
      <div className="flex h-8 items-center">
        {action ?? <p className="text-xs leading-4 text-[var(--panel-muted-fg)]">{modeCaption(live)}</p>}
      </div>
    </div>
  );
}
