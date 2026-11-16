import { useCallback } from "react";

import type { WasmSession } from "@/lib/editorSession";

/** One session call and the re-read after it; the Fixtures hook owns it (and the notice). */
export type FixtureRun = <T>(fallback: T, call: (session: WasmSession) => T) => T;

/** The set, add, placement, Lock, pin field and Apply commands of the Fixtures tab. */
export interface FixtureSetCommands {
  /** "+ Set": the new set's id, `null` at 16 sets. */
  addSet: () => string | null;
  renameSet: (id: string, name: string) => boolean;
  /** The second press of a set's armed trash. */
  removeSet: (id: string) => boolean;
  /** Moves the set to the place `index` among the sets. */
  moveSet: (id: string, index: number) => boolean;
  setOnPage: (id: string, on: boolean) => boolean;
  /** Folds or unfolds a set (session state, not a step). */
  setUnfolded: (id: string, on: boolean) => boolean;
  /** A kind of a set's add bar: the new fixture's id. */
  addSetFixture: (set: string, kind: string) => string | null;
  /** A kind of the Job fixtures add bar: arms its placement, or ends it on a second press. */
  armJobFixture: (kind: string) => boolean;
  endPlacement: () => boolean;
  /** Keyboard placement: moves the hanging fixture by mm; Enter drops it. */
  nudgePlacement: (dx: number, dy: number) => boolean;
  dropPlacement: () => string | null;
  /** A pin or screw field: `diameter_mm`, `columns`, `rows`, `column_spacing_mm`, `row_spacing_mm`, `mark_mm`. */
  setGridValue: (id: string, key: string, value: number) => boolean;
  /** The Output switch of a pin's or screw's role job (the Jobs toggles). */
  setRoleOutput: (job: string, on: boolean) => boolean;
  setLocked: (id: string, locked: boolean) => boolean;
  /** The Lock of a set (criterion 32): every fixture in it keeps its place. */
  setSetLocked: (id: string, locked: boolean) => boolean;
  /** Picks the page of the Apply list (its preview shows), or `null` folds it. */
  setApplyPreview: (id: string | null) => boolean;
  /** "Apply <n> fixtures": how many were copied. */
  applyFromPage: (id: string) => number;
}

/** The commands over `run`, each one session call and one re-read. */
export function useFixtureSetCommands(run: FixtureRun): FixtureSetCommands {
  const addSet = useCallback(() => run<string | null>(null, (s) => s.add_fixture_set() ?? null), [run]);
  const renameSet = useCallback((id: string, name: string) => run(false, (s) => s.rename_fixture_set(id, name)), [run]);
  const removeSet = useCallback((id: string) => run(false, (s) => s.remove_fixture_set(id)), [run]);
  const moveSet = useCallback((id: string, index: number) => run(false, (s) => s.move_fixture_set(id, index)), [run]);
  const setOnPage = useCallback((id: string, on: boolean) => run(false, (s) => s.set_fixture_set_on_page(id, on)), [run]);
  const setUnfolded = useCallback((id: string, on: boolean) => run(false, (s) => s.set_fixture_set_unfolded(id, on)), [run]);
  const addSetFixture = useCallback(
    (set: string, kind: string) => run<string | null>(null, (s) => s.add_set_fixture(set, kind) ?? null),
    [run],
  );
  const armJobFixture = useCallback((kind: string) => run(false, (s) => s.arm_job_fixture(kind)), [run]);
  const endPlacement = useCallback(() => run(false, (s) => s.end_fixture_placement()), [run]);
  const nudgePlacement = useCallback((dx: number, dy: number) => run(false, (s) => s.nudge_fixture_placement(dx, dy)), [run]);
  const dropPlacement = useCallback(() => run<string | null>(null, (s) => s.drop_fixture_placement() ?? null), [run]);
  const setGridValue = useCallback(
    (id: string, key: string, value: number) => run(false, (s) => s.set_fixture_grid_value(id, key, value)),
    [run],
  );
  const setRoleOutput = useCallback((job: string, on: boolean) => run(false, (s) => s.set_fixture_job_output(job, on)), [run]);
  const setLocked = useCallback((id: string, locked: boolean) => run(false, (s) => s.set_fixture_locked(id, locked)), [run]);
  const setSetLocked = useCallback((id: string, locked: boolean) => run(false, (s) => s.set_fixture_set_locked(id, locked)), [run]);
  const setApplyPreview = useCallback(
    (id: string | null) => run(false, (s) => s.set_apply_preview(id)),
    [run],
  );
  const applyFromPage = useCallback((id: string) => run(0, (s) => s.apply_fixtures_from_page(id)), [run]);
  return {
    addSet,
    renameSet,
    removeSet,
    moveSet,
    setOnPage,
    setUnfolded,
    addSetFixture,
    armJobFixture,
    endPlacement,
    nudgePlacement,
    dropPlacement,
    setGridValue,
    setRoleOutput,
    setLocked,
    setSetLocked,
    setApplyPreview,
    applyFromPage,
  };
}
