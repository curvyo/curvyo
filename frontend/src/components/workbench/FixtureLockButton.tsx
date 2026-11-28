import { Lock, LockOpen } from "lucide-react";

import { Tooltip } from "@/components/ui/tooltip";
import { lockTooltip } from "@/lib/fixtureSets";

/** `ICON_BUTTON` without its text colour, so the ink and the muted state each set their own. */
const LOCK_BUTTON =
  "inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";
/** Only a Lock that presses lights on hover. */
const PRESSABLE = "hover:bg-[var(--editor-accent-hover)]";
const INK = "text-[var(--toolbar-icon)]";
const MUTED = "text-[var(--panel-muted-fg)]";

/**
 * The Lock toggle of a set row, a job fixture row or a set fixture row: `Lock` in ink when
 * locked, `LockOpen` muted when not. A fixture of a locked set (`lockedWith` names the set) shows
 * `Lock` muted and does not press; no colour is invented (UX notes 2).
 */
export function FixtureLockButton({
  name,
  locked,
  lockedWith = null,
  onPress,
}: {
  name: string;
  locked: boolean;
  lockedWith?: string | null;
  onPress: () => void;
}) {
  const fixed = lockedWith !== null;
  const Icon = locked || fixed ? Lock : LockOpen;
  return (
    <Tooltip side="left" content={lockTooltip(locked, lockedWith)}>
      <button
        type="button"
        aria-label={`Lock ${name}`}
        aria-pressed={locked || fixed}
        aria-description={fixed ? lockTooltip(true, lockedWith) : undefined}
        aria-disabled={fixed || undefined}
        tabIndex={-1}
        className={`${LOCK_BUTTON} ${locked && !fixed ? INK : MUTED} ${fixed ? "cursor-default" : PRESSABLE}`}
        onClick={fixed ? undefined : onPress}
      >
        <Icon size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
      </button>
    </Tooltip>
  );
}
