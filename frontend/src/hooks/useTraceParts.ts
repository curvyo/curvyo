/** The parts of a composite trace in the Trace tool (`specs/0094` slice 2, UX notes §3 to §5): each
 * part's settings in the frontend, keyed by the session's stable part id; the session keeps each
 * part's mask paths and mode. Nothing of it is remembered (criterion 29). */
import { type MutableRefObject, useCallback, useMemo, useRef } from "react";

import type { WasmSession } from "@/lib/editorSession";
import type { TraceChoice, TraceValues } from "@/lib/traceText";
import { PartCache, type TracePartInfo, readParts } from "@/lib/traceParts";

export interface TracePartsApi {
  /** The chips in order. */
  parts: TracePartInfo[];
  /** The kept results of the parts' jobs (criterion 25). */
  cache: MutableRefObject<PartCache>;
  /** The settings of `part`: the bar's for the active part. */
  choiceOf: (part: TracePartInfo) => TraceChoice;
  /** A chip pick: the part becomes active, its settings load into the bar. */
  selectPart: (index: number) => void;
  /** Add part: a copy of the active part's settings, no mask paths. */
  addPart: () => void;
  /** Remove part `index` (after the inline confirm). */
  removePart: (index: number) => void;
  /** The analysed values the maker set for part `id` (they stay when its region is read). */
  makerOf: (id: number) => ReadonlySet<keyof TraceValues>;
  /** The maker set `fields` of the active part. */
  markMaker: (fields: readonly (keyof TraceValues)[]) => void;
  /** New settings of part `id`: into the bar when it is the active part. */
  setChoiceOf: (id: number, choice: TraceChoice) => void;
}

/** `choice` with no colour of its own: nothing picked, no fixed, excluded or kept colour. */
function withoutColours(choice: TraceChoice): TraceChoice {
  return { ...choice, values: { ...choice.values, swatches: [], fixed: [], excluded: [], keptPalette: null } };
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
  const choices = useRef(new Map<number, TraceChoice>());
  const makers = useRef(new Map<number, Set<keyof TraceValues>>());
  const cache = useRef(new PartCache());

  /** The active part's settings go back into its entry before another part becomes active. */
  const storeActive = useCallback((session: WasmSession) => {
    const active = readParts(session.trace_parts()).find((p) => p.active);
    if (active) {
      choices.current.set(active.id, choiceRef.current);
    }
  }, [choiceRef]);

  /** The now active part's settings into the bar. */
  const loadActive = useCallback((session: WasmSession) => {
    const active = readParts(session.trace_parts()).find((p) => p.active);
    const choice = active ? choices.current.get(active.id) : undefined;
    if (choice) {
      load(choice);
    }
  }, [load]);

  const choiceOf = useCallback(
    (part: TracePartInfo) => (part.active ? choiceRef.current : (choices.current.get(part.id) ?? choiceRef.current)),
    [choiceRef],
  );

  const selectPart = useCallback(
    (index: number) => {
      const session = getSession();
      if (!session) {
        return;
      }
      storeActive(session);
      if (session.set_trace_active_part(index)) {
        loadActive(session);
      }
      onChange();
    },
    [getSession, onChange, storeActive, loadActive],
  );

  const addPart = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    storeActive(session);
    const before = readParts(session.trace_parts()).find((p) => p.active)?.id;
    const id = session.add_trace_part();
    if (id >= 0) {
      // A copy of the settings of the part before (criterion 19), what the maker set in them
      // included, but not its colours: a new part starts without picked colours (customer
      // 2026-11-10).
      const fresh = withoutColours(choiceRef.current);
      choices.current.set(id, fresh);
      load(fresh);
      makers.current.set(id, new Set(before === undefined ? [] : (makers.current.get(before) ?? [])));
    }
    onChange();
  }, [getSession, onChange, storeActive, choiceRef, load]);

  const removePart = useCallback(
    (index: number) => {
      const session = getSession();
      if (!session) {
        return;
      }
      const removed = readParts(session.trace_parts())[index];
      storeActive(session);
      if (session.remove_trace_part(index)) {
        if (removed) {
          choices.current.delete(removed.id);
          makers.current.delete(removed.id);
        }
        loadActive(session);
      }
      onChange();
    },
    [getSession, onChange, storeActive, loadActive],
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
      if (active?.id === id) {
        load(choice);
      } else {
        choices.current.set(id, choice);
      }
    },
    [getSession, load],
  );

  return { parts, cache, choiceOf, selectPart, addPart, removePart, makerOf, markMaker, setChoiceOf };
}
