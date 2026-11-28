// The small parts the Share tab's My devices blocks share (`specs/0096-own-devices-sync`
// ux-notes 2 to 9): the field and text looks, the armed destructive action, and the button looks
// of `components/ui/button.tsx` under their old names.

import type { Arm } from "@/hooks/useArm";
import { BUTTON_BASE, BUTTON_LOOK } from "@/components/ui/looks";
import { AlertLine } from "@/components/ui/notice";

export const BUTTON = BUTTON_BASE;
export const OUTLINE = BUTTON_LOOK.secondary;
export const APPLY = BUTTON_LOOK.primary;
export const DESTRUCTIVE = BUTTON_LOOK.destructive;
export { FIELD_WIDE as FIELD } from "@/components/ui/looks";
export const MUTED = "text-xs text-[var(--panel-muted-fg)]";
export const SURFACE = "rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)]";
/** How long a membership row shows its busy line at most when no change arrives (ms). */
export const BUSY_MS = 5000;
/** The look of an armed action: filled `--destructive`, white (design system "Armed action"). */
export const ARMED = "border-[var(--destructive)] bg-[var(--destructive)] text-white hover:brightness-110";

/** The refusal of an armed action's second press (an alert); an armed action has no sentence
 * (customer 2026-11-09). */
export function ArmLines({ arm, className = "" }: { arm: Arm; className?: string }) {
  return arm.refusal ? (
    <AlertLine className={className}>
      {arm.refusal}
    </AlertLine>
  ) : null;
}
