import { Lock } from "lucide-react";

import type { FixturesApi } from "@/hooks/useFixtures";
import { BAR_ROW_CLASS } from "@/lib/barLayout";
import { lockTooltip } from "@/lib/fixtureSets";
import type { FixtureRow } from "@/lib/fixturesView";

/**
 * The Select bar of one selected job fixture (`specs/0113-fixture-sets-and-pins` criterion 31, UX
 * notes 7): one group, the Lock toggle, and nothing else (the settings switches, Flip and every
 * other group leave the bar, UX note M7). A set fixture gets the same bar (criterion 32); while
 * its set is locked the toggle reads pressed and does not press, tooltip "Locked with <set>".
 */
export function FixtureLockBar({
  row,
  setName,
  api,
  onReturnFocus,
}: {
  row: FixtureRow;
  /** The set of a set fixture: its name is in the tooltip while the set is locked. */
  setName: string | null;
  api: FixturesApi;
  onReturnFocus: () => void;
}) {
  const fixed = row.lockedBySet;
  const locked = row.locked || fixed;
  return (
    <div
      data-context-bar=""
      className="pointer-events-auto flex min-w-0 flex-col rounded-lg px-2 text-sm"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <div className={BAR_ROW_CLASS}>
        <div role="group" aria-label="Fixture" className="flex min-h-9 items-center py-1">
          <button
            type="button"
            aria-label="Lock"
            aria-pressed={locked}
            aria-description={fixed ? lockTooltip(true, setName ?? "its set") : undefined}
            title={fixed ? lockTooltip(true, setName ?? "its set") : undefined}
            onClick={(event) => {
              if (fixed) return;
              api.setLocked(row.id, !locked);
              if (event.detail > 0) onReturnFocus();
            }}
            className={`inline-flex h-7 items-center gap-1 rounded-md px-2 text-[13px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${locked ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"}`}
          >
            <Lock size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            Lock
          </button>
        </div>
      </div>
    </div>
  );
}
