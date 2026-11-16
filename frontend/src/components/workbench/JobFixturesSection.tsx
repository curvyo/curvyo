import { type KeyboardEvent, useState } from "react";

import { ApplyFixturesList } from "@/components/workbench/ApplyFixturesList";
import { FixtureAddBar } from "@/components/workbench/FixtureAddBar";
import { FixtureRows } from "@/components/workbench/FixtureRowItem";
import type { DeleteArm } from "@/hooks/useDeleteArm";
import type { FixturesApi } from "@/hooks/useFixtures";
import { JOB_ADD_KINDS } from "@/lib/fixtureRows";
import { placementNudge, placingStatus } from "@/lib/fixtureSets";
import type { FixtureKind, FixtureRow, FixturesView } from "@/lib/fixturesView";

/**
 * The Job fixtures section (`specs/0113-fixture-sets-and-pins` `ux-notes.md` 5): the symbol add bar
 * that arms placement (the pressed button is the only cue), the job fixture rows and Apply from
 * page. Keys of the armed button place by keyboard: arrows move the hanging fixture, Enter drops it,
 * Escape ends the placement.
 */
export function JobFixturesSection({
  view,
  api,
  arm,
  openId,
  onOpen,
}: {
  view: FixturesView;
  api: FixturesApi;
  arm: DeleteArm;
  openId: string | null;
  onOpen: (row: FixtureRow) => void;
}) {
  // The polite status says "Placing Pin." once on arming; an end clears it.
  const [status, setStatus] = useState("");
  const press = (kind: FixtureKind) => setStatus(api.armJobFixture(kind) ? placingStatus(kind) : "");
  const onKey = (event: KeyboardEvent<HTMLButtonElement>, kind: FixtureKind) => {
    if (view.armedKind !== kind) return;
    const nudge = placementNudge(event.key, event.shiftKey);
    if (nudge !== null) {
      event.preventDefault();
      api.nudgePlacement(nudge[0], nudge[1]);
    } else if (event.key === "Enter") {
      // While armed, Enter drops the fixture instead of pressing the button again.
      event.preventDefault();
      api.dropPlacement();
    } else if (event.key === "Escape") {
      // The focus stays on the kind button (the canvas does not take the key).
      event.preventDefault();
      event.stopPropagation();
      api.endPlacement();
      setStatus("");
    }
  };
  return (
    <>
      {view.canAddJobFixture ? (
        <FixtureAddBar label="Add job fixture" kinds={JOB_ADD_KINDS} pressed={view.armedKind} onPress={press} onKeyDown={onKey} />
      ) : null}
      <FixtureRows
        label="Job fixtures"
        addBarLabel="Add job fixture"
        job
        rows={view.jobFixtures}
        view={view}
        api={api}
        arm={arm}
        openId={openId}
        onOpen={onOpen}
      />
      <ApplyFixturesList view={view} api={api} />
      <span role="status" className="sr-only">
        {view.armedKind === null ? "" : status}
      </span>
    </>
  );
}
