import { useCallback, useEffect, useMemo, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { decodeFitCue, type FitCue } from "@/lib/fixtureRows";
import { decodeFixturesText, type FixturesView } from "@/lib/fixturesView";

/** What the Fixtures block reads from the editor: the session and its re-read signal. */
export interface FixturesEditor {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
}

/**
 * The Fixtures block's state and commands (`specs/0102-laser-fixtures` `adrs.md` section 7; the
 * session holds the rules, this hook only calls them and re-reads the view). Every command is one
 * session call, then one re-read. The block does not grow `useEditorSession` (architecture review
 * finding 2).
 */
export interface FixturesApi {
  /** The decoded view of the active laser page; `null` on any other page. */
  view: FixturesView | null;
  /** The notice of the last command (a clamped value), `null` when it had none. */
  notice: string | null;
  /** Adds a fixture of `kind` (`grid`, `holder`, `clamp` or `stop`). */
  add: (kind: string) => boolean;
  /** Selects the fixture `id` alone (its row press). */
  select: (id: string) => boolean;
  /** Selects the slot `id` of its holder. */
  selectSlot: (id: string) => boolean;
  rename: (id: string, name: string) => boolean;
  /** Removes the fixture `id` with its slots (the second press of its armed trash). */
  remove: (id: string) => boolean;
  setActive: (id: string, on: boolean) => boolean;
  setVisible: (id: string, on: boolean) => boolean;
  /** A length field of the grid in mm: `spacing_mm`, `origin_x_mm`, `origin_y_mm`. */
  setLength: (id: string, key: string, mm: number) => boolean;
  setMajorEvery: (id: string, count: number) => boolean;
  /** A grid tape switch: `bottom`, `left`, `top` or `right`. */
  setTape: (id: string, edge: string, on: boolean) => boolean;
  /** The Fixtures snap switch: one undo step. */
  setSnapping: (on: boolean) => boolean;
  /** The fit cue of the move in flight (criteria 25, 26); `null` without one. */
  fitCue: FitCue | null;
  /** Adds a slot to the holder `id`; the new slot's id, `null` when none was added. */
  addSlot: (holder: string) => string | null;
  removeSlot: (id: string) => boolean;
  renameSlot: (id: string, name: string) => boolean;
  setSlotShape: (id: string, shape: "rect" | "circle") => boolean;
  /** A slot field in mm: `cx_mm`, `cy_mm`, `w_mm`, `h_mm` or `d_mm`. */
  setSlotLength: (id: string, key: string, mm: number) => boolean;
}

interface Snapshot {
  view: FixturesView | null;
}

function readSnapshot(session: WasmSession | null): Snapshot {
  if (!session) return { view: null };
  return { view: decodeFixturesText(session.fixtures_view_json()) };
}

/** The Fixtures block's API for `editor`. */
export function useFixtures(editor: FixturesEditor): FixturesApi {
  const { getSession, syncRevision, sync } = editor;
  const snapshot: Snapshot = useMemo(() => {
    void syncRevision; // The session's state changed: read the view again.
    return readSnapshot(getSession());
  }, [getSession, syncRevision]);
  const [notice, setNotice] = useState<string | null>(null);

  // The fit cue lives only while a drag is in flight, and a drag changes no re-read signal, so
  // it is read once per frame and a state change follows only a change of its text.
  const [cueText, setCueText] = useState("null");
  useEffect(() => {
    let frame = 0;
    let previous = "null";
    const tick = () => {
      const next = getSession()?.fixture_fit_cue_json() ?? "null";
      if (next !== previous) {
        previous = next;
        setCueText(next);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  const fitCue = useMemo(() => decodeFitCue(cueText), [cueText]);

  // A clamped value's notice shows for 3 s (`ux-notes.md` section 4.1), then clears.
  useEffect(() => {
    if (notice === null) return;
    const timer = window.setTimeout(() => setNotice(null), 3000);
    return () => window.clearTimeout(timer);
  }, [notice]);

  const run = useCallback(
    <T,>(fallback: T, call: (session: WasmSession) => T): T => {
      const session = getSession();
      if (!session) return fallback;
      const result = call(session);
      const text = session.take_fixture_notice();
      if (text) setNotice(text);
      sync();
      return result;
    },
    [getSession, sync],
  );

  const add = useCallback((kind: string) => run(false, (s) => s.add_fixture(kind)), [run]);
  const select = useCallback((id: string) => run(false, (s) => s.select_fixture(id)), [run]);
  const selectSlot = useCallback((id: string) => run(false, (s) => s.select_slot(id)), [run]);
  const rename = useCallback((id: string, name: string) => run(false, (s) => s.rename_fixture(id, name)), [run]);
  const remove = useCallback((id: string) => run(false, (s) => s.delete_fixture(id)), [run]);
  const setActive = useCallback((id: string, on: boolean) => run(false, (s) => s.set_fixture_active(id, on)), [run]);
  const setVisible = useCallback((id: string, on: boolean) => run(false, (s) => s.set_fixture_visible(id, on)), [run]);
  const setLength = useCallback(
    (id: string, key: string, mm: number) => run(false, (s) => s.set_fixture_length(id, key, mm)),
    [run],
  );
  const setMajorEvery = useCallback(
    (id: string, count: number) => run(false, (s) => s.set_grid_major_every(id, count)),
    [run],
  );
  const setTape = useCallback(
    (id: string, edge: string, on: boolean) => run(false, (s) => s.set_grid_tape(id, edge, on)),
    [run],
  );
  const setSnapping = useCallback((on: boolean) => run(false, (s) => s.set_snapping_fixtures(on)), [run]);
  const addSlot = useCallback(
    (holder: string) => run<string | null>(null, (s) => s.add_slot(holder) ?? null),
    [run],
  );
  const removeSlot = useCallback((id: string) => run(false, (s) => s.remove_slot(id)), [run]);
  const renameSlot = useCallback((id: string, name: string) => run(false, (s) => s.rename_slot(id, name)), [run]);
  const setSlotShape = useCallback(
    (id: string, shape: "rect" | "circle") => run(false, (s) => s.set_slot_shape(id, shape)),
    [run],
  );
  const setSlotLength = useCallback(
    (id: string, key: string, mm: number) => run(false, (s) => s.set_slot_length(id, key, mm)),
    [run],
  );

  return {
    view: snapshot.view,
    notice,
    add,
    select,
    selectSlot,
    rename,
    remove,
    setActive,
    setVisible,
    setLength,
    setMajorEvery,
    setTape,
    setSnapping,
    fitCue,
    addSlot,
    removeSlot,
    renameSlot,
    setSlotShape,
    setSlotLength,
  };
}
