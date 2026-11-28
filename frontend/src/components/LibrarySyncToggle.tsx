import { CloudOff, CloudSync, RefreshCw } from "lucide-react";

import { Tooltip } from "@/components/ui/tooltip";
import { librarySyncGlyph, type LibrarySyncView } from "@/lib/librarySyncView";

const RING = "focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

interface LibrarySyncToggleProps {
  /** The row's name: the accessible name is "Sync <name> to my devices". */
  name: string;
  /** The stored on/off state, still shown while greyed (UX notes §3). */
  on: boolean;
  /** The group's library view, `null` while this device is in no group: the toggle greys. */
  view: LibrarySyncView | null;
  onToggle: () => void;
  /** The row's control slot: `h-7 w-6` in the machine row, `h-6 w-6` in the material row. */
  slot: string;
  /** Whether the tooltip names the row key `S` (the fonts tree has no such key: type-to-search
   * keeps every letter). */
  keyHint?: boolean;
}

/**
 * The per-item library sync toggle (`specs/0116-library-sync` UX notes §1): a symbol toggle in
 * the row's own control slot, immediately before the trash, not a Tab stop. Off is the outlined
 * cloud in muted ink, on the pressed look with `--toolbar-icon-active-bg` / `-fg`; the glyph
 * swaps `RefreshCw` while this item transfers and `CloudOff` while offline or refused. One press
 * switches, no armed second press (UX notes §4). Greyed without a paired device, its reason as
 * tooltip and `aria-description`.
 */
export function LibrarySyncToggle({ name, on, view, onToggle, slot, keyHint = true }: LibrarySyncToggleProps) {
  const greyed = view === null;
  const reason = "Pair a device to sync.";
  const glyph = librarySyncGlyph(on, greyed, view?.online ?? false, view?.acked ?? true);
  const Glyph = glyph === "transferring" ? RefreshCw : glyph === "offline" ? CloudOff : CloudSync;
  const second = greyed || !on ? "" : (view?.state ?? "");
  const tooltip = (
    <>
      <div>{greyed ? reason : keyHint ? "Sync to my devices (S)" : "Sync to my devices"}</div>
      {second ? <div className="text-muted-foreground">{second}</div> : null}
    </>
  );
  return (
    <Tooltip side="left" content={tooltip}>
      <button
        type="button"
        tabIndex={-1}
        aria-pressed={on}
        aria-disabled={greyed || undefined}
        aria-description={greyed ? reason : undefined}
        aria-busy={glyph === "transferring" || undefined}
        aria-label={`Sync ${name} to my devices`}
        onMouseDown={(event) => event.preventDefault()}
        onClick={(event) => {
          event.stopPropagation();
          if (!greyed) {
            onToggle();
          }
        }}
        className={`${slot} shrink-0 items-center justify-center rounded-[5px] outline-none ${RING} ${
          on
            ? "aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] aria-pressed:hover:bg-[var(--toolbar-icon-active-bg)]"
            : "text-[var(--panel-muted-fg)] hover:bg-[var(--editor-accent-hover)]"
        } ${greyed ? "opacity-40" : ""}`}
      >
        <Glyph size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
      </button>
    </Tooltip>
  );
}
