import { useCallback, useEffect, useMemo, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { decodeFixtureJobsText, type FixtureJobsView, type JobKindText } from "@/lib/fixtureJobsView";
import { WORK_REFUSED } from "@/lib/fixtureJobRows";

/** What the Fixture jobs section reads from the editor: the session and its re-read signal. */
export interface FixtureJobsEditor {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
}

/**
 * The Fixture jobs section's state and commands (`specs/0103-fixture-jobs` `adrs.md` section 6;
 * the session holds the rules, this hook only calls them and re-reads the view). Every command is
 * one session call, then one re-read. It does not grow `useEditorSession` (architecture review
 * finding 2).
 */
export interface FixtureJobsApi {
  /** The decoded section of the active laser page; `null` on any other page. */
  view: FixtureJobsView | null;
  /** The refusal of a work-item drop onto a job row (criterion 10), `null` otherwise; 8 s. */
  refusal: string | null;
  /** Adds a job of `kind` to the one selected fixture; the new job's id, `null` when none. */
  add: (kind: JobKindText) => string | null;
  /** Selects the job `id` and opens its Parts list. */
  select: (id: string) => boolean;
  deselect: () => boolean;
  rename: (id: string, name: string) => boolean;
  /** Removes the job `id` with its ticks (the second press of its armed trash). */
  remove: (id: string) => boolean;
  setShow: (id: string, on: boolean) => boolean;
  setOutput: (id: string, on: boolean) => boolean;
  /** Moves the job `id` to the place `to` inside its fixture's group. */
  move: (id: string, to: number) => boolean;
  /** Ticks (`ticked` true) or unticks the part `token` in the job `id`. */
  setPart: (id: string, token: string, ticked: boolean) => boolean;
  /** "Select" on a part: selects its slot, or its fixture, on the canvas. */
  selectPart: (id: string, token: string) => boolean;
}

const REFUSAL_MS = 8000;

function readView(session: WasmSession | null): FixtureJobsView | null {
  return session ? decodeFixtureJobsText(session.fixture_jobs_view_json()) : null;
}

/** The Fixture jobs section's API for `editor`. */
export function useFixtureJobs(editor: FixtureJobsEditor): FixtureJobsApi {
  const { getSession, syncRevision, sync } = editor;
  const view = useMemo(() => {
    void syncRevision; // The session's state changed: read the view again.
    return readView(getSession());
  }, [getSession, syncRevision]);
  const [refusal, setRefusal] = useState<string | null>(null);

  // Work items dragged from the canvas and released over a job row are refused (criterion 10):
  // the drag is cancelled, the items stay, and the notice shows for 8 s. The window capture phase
  // runs before the canvas, so the canvas never sees this release.
  useEffect(() => {
    const onRelease = (event: PointerEvent) => {
      const session = getSession();
      if (!session || session.selection_count() === 0 || !session.pointer_is_down()) return;
      const row = document.elementFromPoint(event.clientX, event.clientY)?.closest("[data-job-row]");
      if (!row) return;
      event.stopPropagation();
      session.key_down("Escape", false, false, false, false, false, false, performance.now());
      setRefusal(WORK_REFUSED);
    };
    window.addEventListener("pointerup", onRelease, true);
    return () => window.removeEventListener("pointerup", onRelease, true);
  }, [getSession]);
  useEffect(() => {
    if (refusal === null) return;
    const timer = window.setTimeout(() => setRefusal(null), REFUSAL_MS);
    return () => window.clearTimeout(timer);
  }, [refusal]);

  // Every command: one session call, then one re-read (the sync signal).
  const run = useCallback(
    <T,>(fallback: T, call: (session: WasmSession) => T): T => {
      const session = getSession();
      if (!session) return fallback;
      const result = call(session);
      sync();
      return result;
    },
    [getSession, sync],
  );

  const add = useCallback(
    (kind: JobKindText) => run<string | null>(null, (s) => s.add_fixture_job(kind) ?? null),
    [run],
  );
  const select = useCallback((id: string) => run(false, (s) => s.select_fixture_job(id)), [run]);
  const deselect = useCallback(() => run(false, (s) => s.deselect_fixture_job()), [run]);
  const rename = useCallback((id: string, name: string) => run(false, (s) => s.rename_fixture_job(id, name)), [run]);
  const remove = useCallback((id: string) => run(false, (s) => s.delete_fixture_job(id)), [run]);
  const setShow = useCallback((id: string, on: boolean) => run(false, (s) => s.set_fixture_job_show(id, on)), [run]);
  const setOutput = useCallback((id: string, on: boolean) => run(false, (s) => s.set_fixture_job_output(id, on)), [run]);
  const move = useCallback((id: string, to: number) => run(false, (s) => s.move_fixture_job(id, to)), [run]);
  const setPart = useCallback(
    (id: string, token: string, ticked: boolean) => run(false, (s) => s.set_fixture_job_part(id, token, ticked)),
    [run],
  );
  const selectPart = useCallback(
    (id: string, token: string) => run(false, (s) => s.select_fixture_job_part(id, token)),
    [run],
  );

  return { view, refusal, add, select, deselect, rename, remove, setShow, setOutput, move, setPart, selectPart };
}
