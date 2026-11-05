// The small parts the Share tab's My devices blocks share (`specs/0096-own-devices-sync`
// ux-notes 2 to 9): the button, field and text looks, and the destructive inline confirm.

import { useEffect, useState } from "react";

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
/** How long a just-shown destructive confirm ignores presses (design system "Inline confirm"). */
export const CONFIRM_GUARD_MS = 400;
/** How long a membership row shows its busy line at most when no change arrives (ms). */
export const BUSY_MS = 5000;


/** A destructive inline confirm under a row (Remove, Leave, Stop syncing): Cancel focused, the
 * action ignores presses for its first 400 ms, a refusal shows under it and the confirm stays.
 * `apply` gives the action the Apply look for a confirm that loses nothing (Make admin), which
 * needs no guard. */
export function DestructiveConfirm({
  text,
  label,
  action,
  width = 76,
  apply = false,
  onCancel,
  onConfirm,
}: {
  text: string;
  /** The group's short accessible name ("Remove Laptop"). */
  label: string;
  action: string;
  /** The action button's width (px). */
  width?: number;
  apply?: boolean;
  onCancel: () => void;
  onConfirm: () => string;
}) {
  const [armed, setArmed] = useState(apply);
  const [refusal, setRefusal] = useState("");
  useEffect(() => {
    const timer = window.setTimeout(() => setArmed(true), CONFIRM_GUARD_MS);
    return () => window.clearTimeout(timer);
  }, []);
  return (
    <div
      role="group"
      aria-label={label}
      className="flex flex-col gap-2 py-1"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          onCancel();
        }
      }}
    >
      <p className="text-xs text-[var(--toolbar-icon)]">{text}</p>
      <div className="flex flex-wrap gap-2">
        <button autoFocus type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={onCancel}>
          Cancel
        </button>
        <button
          type="button"
          className={`${BUTTON} ${apply ? APPLY : DESTRUCTIVE}`}
          style={{ width }}
          onClick={() => {
            if (armed) {
              setRefusal(onConfirm());
            }
          }}
        >
          {action}
        </button>
      </div>
      {refusal ? (
        <p role="alert" className={ALERT}>
          {refusal}
        </p>
      ) : null}
    </div>
  );
}
