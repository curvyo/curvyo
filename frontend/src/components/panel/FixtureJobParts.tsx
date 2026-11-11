import { useState, type KeyboardEvent as ReactKeyboardEvent } from "react";

import type { JobRow, PartRow, PartsView } from "@/lib/fixtureJobsView";
import { partsHeader, tickTitle } from "@/lib/fixtureJobRows";
import { nextFocusIndex } from "@/lib/operationRows";

export interface PartsActions {
  /** Ticks (`ticked`) or unticks the part `token` in the job. */
  setPart: (job: JobRow, token: string, ticked: boolean) => void;
  /** "Select": selects the part's slot, or its fixture, on the canvas. */
  selectPart: (job: JobRow, token: string) => void;
  /** Escape: the focus goes back to the job's row; the job stays selected. */
  leave: (job: JobRow) => void;
}

/**
 * The Parts list of the selected job (`specs/0103-fixture-jobs` `ux-notes.md` section 5): a
 * group under the job's row, "Parts" and the count, then one row per readable part with a check
 * box and "Select". Rendered in full (at most 51 rows). Up and Down move between the rows, Left
 * and Right between the check box and "Select" of a row, Escape back to the job.
 */
export function FixtureJobParts({ job, parts, actions }: { job: JobRow; parts: PartsView; actions: PartsActions }) {
  /** The roving stop: the row whose check box is in the tab order. */
  const [stop, setStop] = useState(0);
  const onKey = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    event.stopPropagation(); // the job list's own keys do not see the Parts list's keys
    const target = event.target as HTMLElement;
    const element = target.closest<HTMLElement>("[data-part-index]");
    const index = element ? Number(element.dataset.partIndex) : 0;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      actions.leave(job);
      return;
    }
    if (event.key === "ArrowUp" && index === 0) {
      // Up from the first part returns to the job's row, not a job move (`ux-notes.md` 5.4).
      event.preventDefault();
      actions.leave(job);
      return;
    }
    const next = nextFocusIndex(parts.rows.length, index, event.key);
    if (next !== null) {
      event.preventDefault();
      focusPart(event.currentTarget, next, target.dataset.partControl === "select" ? "select" : "check");
      return;
    }
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      focusPart(event.currentTarget, index, event.key === "ArrowLeft" ? "check" : "select");
    }
  };
  return (
    <div role="group" aria-label={`Parts of ${job.name}`} onKeyDown={onKey} className="flex flex-col pb-1 pl-4">
      <div className="flex h-6 items-center justify-between text-[11px] text-[var(--panel-muted-fg)]">
        <span className="font-semibold">Parts</span>
        <span className="tabular-nums">{partsHeader(parts)}</span>
      </div>
      <ul role="list" className="flex flex-col">
        {parts.rows.map((part, index) => (
          <PartLine
            key={part.token}
            job={job}
            part={part}
            index={index}
            stop={index === Math.min(stop, parts.rows.length - 1)}
            onFocusRow={() => setStop(index)}
            actions={actions}
          />
        ))}
      </ul>
    </div>
  );
}

function PartLine({
  job,
  part,
  index,
  stop,
  onFocusRow,
  actions,
}: {
  job: JobRow;
  part: PartRow;
  index: number;
  stop: boolean;
  onFocusRow: () => void;
  actions: PartsActions;
}) {
  return (
    <li data-part-index={index} className="flex h-7 items-center gap-2 text-[13px] text-[var(--toolbar-icon)]">
      <button
        type="button"
        role="checkbox"
        data-part-control="check"
        aria-checked={part.ticked}
        aria-label={part.name}
        aria-description={`Part of ${job.name}`}
        title={tickTitle(part.ticked, job.name)}
        tabIndex={stop ? 0 : -1}
        onFocus={onFocusRow}
        onClick={() => actions.setPart(job, part.token, !part.ticked)}
        className="flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border border-[var(--toolbar-icon)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        {part.ticked ? <span className="h-2 w-2 bg-[var(--toolbar-icon)]" /> : null}
      </button>
      <span className="min-w-0 flex-1 truncate">{part.name}</span>
      <button
        type="button"
        data-part-control="select"
        tabIndex={-1}
        aria-label={`Select ${part.name}`}
        onClick={() => actions.selectPart(job, part.token)}
        className="h-6 shrink-0 px-1 text-xs text-[var(--toolbar-icon)] underline-offset-2 hover:underline focus-visible:underline focus-visible:outline-none"
      >
        Select
      </button>
    </li>
  );
}

/** Moves the focus to the check box or the Select button of the part at `index`. */
function focusPart(list: HTMLElement, index: number, column: "check" | "select") {
  list.querySelector<HTMLButtonElement>(`[data-part-index="${index}"] [data-part-control="${column}"]`)?.focus();
}
