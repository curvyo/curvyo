import type { ComponentProps } from "react";
import { PANEL_CONTROL_PX, PANEL_LABEL, PANEL_ROW } from "@/components/ui/looks";

/**
 * One 28 px row of the Style section (`docs/design-system.md`, "Rows"): the
 * label column (60 px), an 8 px gap, the control column (176 px). Extra props
 * pass through so a popover can anchor to the whole row.
 */
export function StyleRow({
  label,
  children,
  ...props
}: { label: string } & ComponentProps<"div">) {
  return (
    <div className={PANEL_ROW} {...props}>
      <span className={PANEL_LABEL}>{label}</span>
      <div className="flex items-center gap-1" style={{ width: PANEL_CONTROL_PX }}>
        {children}
      </div>
    </div>
  );
}
