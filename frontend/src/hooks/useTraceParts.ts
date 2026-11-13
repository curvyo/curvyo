/** The parts of a composite trace in the Trace tool (`specs/0094` slice 2, UX notes §3 to §5;
 * `specs/0114` slice 3): the parts and their settings are the image's, stored by the session; the
 * page keeps per part only what the analysis filled in for it (automatic values are not stored
 * as values, ADR 0061 §5) and which analysed values the maker set, keyed by the session's stable
 * part id. */
import { type MutableRefObject, useCallback, useMemo, useRef } from "react";

import type { WasmSession } from "@/lib/editorSession";
import type { TraceChoice, TraceValues } from "@/lib/traceText";
import { readPartChoice } from "@/lib/traceMaskSettings";
import { PartCache, type TracePartInfo, readParts } from "@/lib/traceParts";

export interface TracePartsApi {
  /** The chips in order. */
  parts: TracePartInfo[];
  /** The kept results of the parts' jobs (criterion 25). */
  cache: MutableRefObject<PartCache>;
  /** The settings of `part`: the bar's for the active part, else its stored ones with what the
   * analysis filled in. */
  choiceOf: (part: TracePartInfo) => TraceChoice;
  /** A chip pick: the part becomes active, its settings load into the bar. */
  selectPart: (index: number) => void;
  /** Add part: the session stores a copy of the active part's settings without its colours. */
  addPart: () => void;
  /** Remove part `index` (after the inline confirm). */
  removePart: (index: number) => void;
  /** The analysed values the maker set for part `id` (they stay when its region is read). */
  makerOf: (id: number) => ReadonlySet<keyof TraceValues>;
  /** The maker set `fields` of the active part. */
  markMaker: (fields: readonly (keyof TraceValues)[]) => void;
  /** New settings of part `id` from its analysis: into the bar when it is the active part, else
   * kept for its jobs. Never stored: no document write without a gesture. */
  setChoiceOf: (id: number, choice: TraceChoice) => void;
  /** The active part's stored settings into the bar when an undo, redo or another peer changed
   * them (`specs/0114` criterion 17). */
  reshow: () => void;
}

/**
 * `partsJson` is the session's chips as last read; `choiceRef` holds the bar's (the active
 * part's) settings and `load` puts another part's settings into the bar.
 */
export function useTraceParts(
  getSession: () => WasmSession | null,
  onChange: () => void,
  partsJson: string,
  choiceRef: MutableRefObject<TraceChoice>,
  load: (choice: TraceChoice) => void,
): TracePartsApi {
  const parts = useMemo(() => readParts(partsJson), [partsJson]);
  /** What the analysis filled in for an inactive part, by part id. */
  const analysed = useRef(new Map<number, TraceChoice>());
  const makers = useRef(new Map<number, Set<keyof TraceValues>>());
  const cache = useRef(new PartCache());

  /** Part `index`'s stored settings over the bar's, `null` for an untouched image. */
  const storedChoice = useCallback(
    (session: WasmSession, index: number): TraceChoice | null => readPartChoice(session.trace_part_settings(index), choiceRef.current),
    [choiceRef],
  );

  /** The now active part's settings into the bar: its stored ones, the threshold the analysis
   * read for it kept while the stored one is Auto. */
  const loadActive = useCallback(
    (session: WasmSession) => {
      const list = readParts(session.trace_parts());
      const index = list.findIndex((p) => p.active);
      const stored = index >= 0 ? storedChoice(session, index) : null;
      if (!stored) {
        return;
      }
      const auto = analysed.current.get(list[index].id);
      load(stored.values.threshold === null && auto ? { ...stored, values: { ...stored.values, threshold: auto.values.threshold } } : stored);
    },
    [load, storedChoice],
  );

  const choiceOf = useCallback(
    (part: TracePartInfo) => {
      if (part.active) {
        return choiceRef.current;
      }
      const session = getSession();
      const index = parts.findIndex((p) => p.id === part.id);
      const stored = session && index >= 0 ? storedChoice(session, index) : null;
      const auto = analysed.current.get(part.id);
      if (stored && auto) {
        const v = stored.values;
        return {
          ...stored,
          values: {
            ...v,
            threshold: v.threshold ?? auto.values.threshold,
            background: v.backgroundAuto ? auto.values.background : v.background,
          },
        };
      }
      return stored ?? auto ?? choiceRef.current;
    },
    [choiceRef, getSession, parts, storedChoice],
  );

  const selectPart = useCallback(
    (index: number) => {
      const session = getSession();
      if (!session) {
        return;
      }
      if (session.set_trace_active_part(index)) {
        loadActive(session);
      }
      onChange();
    },
    [getSession, onChange, loadActive],
  );

  const addPart = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const before = readParts(session.trace_parts()).find((p) => p.active)?.id;
    const id = session.add_trace_part();
    if (id >= 0) {
      // A copy of the settings of the part before (criterion 19), what the maker set in them
      // included, but not its colours (customer 2026-11-10): the session stored them so.
      makers.current.set(id, new Set(before === undefined ? [] : (makers.current.get(before) ?? [])));
      loadActive(session);
    }
    onChange();
  }, [getSession, onChange, loadActive]);

  const removePart = useCallback(
    (index: number) => {
      const session = getSession();
      if (!session) {
        return;
      }
      const removed = readParts(session.trace_parts())[index];
      if (session.remove_trace_part(index)) {
        if (removed) {
          analysed.current.delete(removed.id);
          makers.current.delete(removed.id);
        }
        loadActive(session);
      }
      onChange();
    },
    [getSession, onChange, loadActive],
  );

  const makerOf = useCallback((id: number): ReadonlySet<keyof TraceValues> => makers.current.get(id) ?? new Set(), []);

  const markMaker = useCallback(
    (fields: readonly (keyof TraceValues)[]) => {
      const session = getSession();
      const active = session ? readParts(session.trace_parts()).find((p) => p.active) : undefined;
      if (!active || fields.length === 0) {
        return;
      }
      const set = makers.current.get(active.id) ?? new Set<keyof TraceValues>();
      fields.forEach((f) => set.add(f));
      makers.current.set(active.id, set);
    },
    [getSession],
  );

  const setChoiceOf = useCallback(
    (id: number, choice: TraceChoice) => {
      const session = getSession();
      const active = session ? readParts(session.trace_parts()).find((p) => p.active) : undefined;
      analysed.current.set(id, choice);
      if (active?.id === id) {
        load(choice);
      }
    },
    [getSession, load],
  );

  const reshow = useCallback(() => {
    const session = getSession();
    if (session) {
      loadActive(session);
    }
  }, [getSession, loadActive]);

  return { parts, cache, choiceOf, selectPart, addPart, removePart, makerOf, markMaker, setChoiceOf, reshow };
}
