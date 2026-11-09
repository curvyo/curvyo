// The small parts the Share tab's My devices blocks share (`specs/0096-own-devices-sync`
// ux-notes 2 to 9): the button, field and text looks, and the armed destructive action.

import type { Arm } from "@/hooks/useArm";

export const BUTTON =
  "inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-[5px] border text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9 aria-disabled:opacity-40";
export const OUTLINE =
  "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";
export const APPLY =
  "border-[var(--toolbar-icon-active-bg)] bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:brightness-110";
export const DESTRUCTIVE =
  "border-[var(--destructive)] bg-[var(--background)] text-[var(--destructive)] hover:bg-[var(--destructive)] hover:text-white focus-visible:bg-[var(--destructive)] focus-visible:text-white";
export const FIELD =
  "h-7 w-full rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-[var(--background)] px-2 text-sm text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-invalid:border-[var(--field-invalid)]";
export const MUTED = "text-xs text-[var(--panel-muted-fg)]";
export const ALERT = "text-xs text-[var(--field-invalid)]";
export const SURFACE = "rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)]";
/** How long a membership row shows its busy line at most when no change arrives (ms). */
export const BUSY_MS = 5000;
/** The look of an armed action: filled `--destructive`, white (design system "Armed action"). */
export const ARMED = "border-[var(--destructive)] bg-[var(--destructive)] text-white hover:brightness-110";

/** The refusal of an armed action's second press (an alert); an armed action has no sentence
 * (customer 2026-11-09). */
export function ArmLines({ arm, className = "" }: { arm: Arm; className?: string }) {
  return arm.refusal ? (
    <p role="alert" className={`${ALERT} ${className}`}>
      {arm.refusal}
    </p>
  ) : null;
}
