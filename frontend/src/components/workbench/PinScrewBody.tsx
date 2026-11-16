import { Crosshair, Diameter } from "lucide-react";

import { NumberField } from "@/components/workbench/FixtureFields";
import { Tooltip } from "@/components/ui/tooltip";
import type { FixturesApi } from "@/hooks/useFixtures";
import { type JobToggle, type RoundField, roundBody } from "@/lib/fixtureRound";
import type { FixtureRow } from "@/lib/fixturesView";

const SIZE_WIDTH = 88;
/** A count's least width; its Spacing fills the rest of the line. */
const COUNT_WIDTH = 84;

/**
 * The unfolded body of a pin or a screw (`specs/0113-fixture-sets-and-pins` `ux-notes.md` 6.1):
 * Diameter (a pin only), Columns and Rows each on one line with its Spacing (always shown, greyed
 * out at one), then the Jobs toggles. The fields fill the row, so nothing leaves the 280 px panel.
 * A typed value goes to the session, which clamps it and says so in a notice. A locked pin or
 * screw shows its values as text, no field (criterion 32); its Jobs toggles still work.
 */
export function PinScrewBody({ row, api, readOnly = false }: { row: FixtureRow; api: FixturesApi; readOnly?: boolean }) {
  if (row.round === null) return null;
  const body = roundBody(row.round);
  const locked = row.locked || row.lockedBySet;
  const field = (spec: RoundField, width?: number) => (
    <NumberField
      key={spec.key}
      label={spec.label}
      unit={spec.unit}
      value={spec.value}
      whole={spec.whole}
      width={width}
      fill={width === undefined}
      disabled={spec.disabled === true}
      readOnly={locked}
      describedBy={row.name}
      onCommit={(value) => api.setGridValue(row.id, spec.key, value)}
    />
  );
  return (
    <div className="flex flex-col gap-2 pb-1" data-locked={locked ? "" : undefined}>
      {readOnly ? null : (
        <>
          {body.diameter ? <div className="flex">{field(body.diameter)}</div> : null}
          {/* Each count and its Spacing share one line; the Spacing fills what the count leaves. */}
          <div className="flex gap-1">{body.columns.map((spec) => field(spec, spec.key === "columns" ? COUNT_WIDTH : undefined))}</div>
          <div className="flex gap-1">{body.rows.map((spec) => field(spec, spec.key === "rows" ? COUNT_WIDTH : undefined))}</div>
        </>
      )}
      {body.jobs.length > 0 ? (
        <>
          <p className="h-4 text-xs font-semibold leading-4 text-[var(--panel-muted-fg)]">Jobs</p>
          {body.jobs.map((job) => (
            <div key={job.label} className="flex flex-wrap items-center gap-x-2 gap-y-1">
              <JobToggleButton job={job} fixtureName={row.name} onPress={() => api.setRoleOutput(job.toggle.job, !job.toggle.on)} />
              {job.size && !readOnly ? field(job.size, SIZE_WIDTH) : null}
            </div>
          ))}
        </>
      ) : null}
    </div>
  );
}

/** A Jobs toggle: the role job's Output; pressed look while on. */
function JobToggleButton({ job, fixtureName, onPress }: { job: JobToggle; fixtureName: string; onPress: () => void }) {
  const Icon = job.label === "Centre mark" ? Crosshair : Diameter;
  return (
    <Tooltip side="left" content={job.tooltip}>
      <button
        type="button"
        aria-pressed={job.toggle.on}
        aria-description={fixtureName}
        onClick={onPress}
        className={`inline-flex h-7 items-center gap-1 rounded-md border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] px-2 text-[13px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${job.toggle.on ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"}`}
      >
        <Icon size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        {job.label}
      </button>
    </Tooltip>
  );
}
