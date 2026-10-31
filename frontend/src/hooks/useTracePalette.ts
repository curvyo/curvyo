/**
 * The palette of a multicolour trace in the frontend (`specs/0089` slice M, criteria 26 and 29;
 * UX notes §7): the strip read from the session after each preview, the selected swatch, and the
 * five actions. Exclude, Merge into, Remove and a pipette pick change the trace's values (traced
 * again); Isolate and Recolour change only how the session draws and writes the result.
 */
import { useCallback, useEffect, useMemo, useState, type MutableRefObject } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  BRIGHTNESS,
  MULTICOLOUR,
  PICK_COLOURS,
  type TraceChoice,
  type TraceValues,
} from "@/lib/traceText";
import {
  type PaletteInfo,
  type PaletteSwatch,
  readPalette,
  stripOrder,
  excludedNote,
  isolatingNote,
  mergingNote,
  sameAsNote,
  sameColour,
} from "@/lib/tracePaletteText";

/** How long "Same as ..." stays, ms (`ux-notes.md` §22.2). */
const SAME_NOTICE_MS = 3000;

/** What the palette shows of `hex`: excluded, merged into another, its fill. */
export interface SwatchState {
  excluded: boolean;
  mergedInto: string | null;
  fill: string;
}

export interface TracePaletteApi {
  /** The strip in order; empty without a multicolour preview. */
  swatches: PaletteSwatch[];
  /** The colours Auto found (the Colours field's muted count), `null` before a result. */
  found: number | null;
  joined: number;
  selected: string | null;
  select: (hex: string | null) => void;
  state: (hex: string) => SwatchState;
  isolated: string | null;
  merging: string | null;
  isolate: (hex: string) => void;
  recolour: (hex: string, fill: string | null) => void;
  startMerge: (hex: string) => void;
  mergeInto: (target: string) => void;
  cancelMerge: () => void;
  toggleExclude: (hex: string) => void;
  remove: (hex: string) => void;
  /** A pipette pick in Brightness + Multicolour: a new fixed colour, or the swatch it equals. */
  pick: (hex: string) => void;
  /** Pick colours with Single colour: the strip holds the picked colours only, and Remove is the
   * one action (`ux-notes.md` §22.3). */
  pickedOnly: boolean;
  /** The Advanced status line's palette text ("Isolating #FFF2A8", "2 colours excluded"), or empty. */
  note: string;
  /** "Same as #3366CC" for 3 s after a pick that matched a colour, or empty: a bar notice. */
  sameNote: string;
  /** The results changed (a new preview): read the palette again. */
  refresh: () => void;
}

interface PaletteContext {
  active: boolean;
  /** The current values (the state the bar shows). */
  values: TraceValues;
  getSession: () => WasmSession | null;
  onChange: () => void;
  apply: (next: TraceChoice) => void;
  choiceRef: MutableRefObject<TraceChoice>;
}

export function useTracePalette({ active, values, getSession, onChange, apply, choiceRef }: PaletteContext): TracePaletteApi {
  const [info, setInfo] = useState<PaletteInfo | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [merging, setMerging] = useState<string | null>(null);
  const [recolours, setRecolours] = useState<Readonly<Record<string, string>>>({});
  const [isolated, setIsolated] = useState<string | null>(null);
  const [same, setSame] = useState<string | null>(null);

  const refresh = useCallback(() => {
    setInfo(readPalette(getSession()?.trace_palette() ?? ""));
  }, [getSession]);

  // The session draws and writes with the recolours and the isolated colour.
  useEffect(() => {
    if (!active) {
      return;
    }
    const pairs = Object.entries(recolours);
    getSession()?.set_trace_palette_view(JSON.stringify(pairs), isolated ?? "");
    onChange();
  }, [active, recolours, isolated, getSession, onChange]);

  // Leaving the tool forgets the palette's session state.
  useEffect(() => {
    if (!active) {
      setInfo(null);
      setSelected(null);
      setMerging(null);
      setIsolated(null);
      setRecolours({});
      setSame(null);
    }
  }, [active]);

  const edit = useCallback(
    (change: (v: TraceChoice["values"]) => Partial<TraceChoice["values"]>) => {
      const current = choiceRef.current;
      apply({ ...current, values: { ...current.values, ...change(current.values) } });
    },
    [apply, choiceRef],
  );

  const swatches = useMemo(() => (info ? stripOrder(info.colours) : []), [info]);
  const state = useCallback(
    (hex: string): SwatchState => ({
      excluded: values.excluded.includes(hex),
      mergedInto: values.merged.find(([from]) => from === hex)?.[1] ?? null,
      fill: recolours[hex] ?? hex,
    }),
    [values.excluded, values.merged, recolours],
  );

  const isolate = useCallback((hex: string) => setIsolated((now) => (now === hex ? null : hex)), []);
  const recolour = useCallback((hex: string, fill: string | null) => {
    setRecolours((now) => {
      const next = { ...now };
      if (fill === null || fill === hex) {
        delete next[hex];
      } else {
        next[hex] = fill;
      }
      return next;
    });
  }, []);
  const startMerge = useCallback(
    (hex: string) => {
      if (values.merged.some(([from]) => from === hex)) {
        // Pressing Merge into again on a merged swatch undoes it.
        edit((v) => ({ merged: v.merged.filter(([from]) => from !== hex) }));
        return;
      }
      setMerging((now) => (now === hex ? null : hex));
    },
    [values.merged, edit],
  );
  const mergeInto = useCallback(
    (target: string) => {
      const from = merging;
      setMerging(null);
      if (!from || from === target) {
        return;
      }
      edit((v) => ({ merged: [...v.merged.filter(([f]) => f !== from), [from, target] as const] }));
    },
    [merging, edit],
  );
  const cancelMerge = useCallback(() => setMerging(null), []);
  const toggleExclude = useCallback(
    (hex: string) => {
      edit((v) => ({ excluded: v.excluded.includes(hex) ? v.excluded.filter((e) => e !== hex) : [...v.excluded, hex] }));
    },
    [edit],
  );
  const remove = useCallback(
    (hex: string) => {
      setSelected(null);
      edit((v) =>
        v.method === BRIGHTNESS ? { fixed: v.fixed.filter((f) => f !== hex) } : { swatches: v.swatches.filter((s) => s !== hex) },
      );
    },
    [edit],
  );
  const pick = useCallback(
    (hex: string) => {
      const twin = sameColour(
        swatches.map((s) => s.hex),
        hex,
      );
      if (twin) {
        setSelected(twin);
        setSame(twin);
        return;
      }
      setSame(null);
      edit((v) => ({ fixed: v.fixed.includes(hex) ? v.fixed : [...v.fixed, hex] }));
    },
    [swatches, edit],
  );
  // "Same as ..." is a short notice (`ux-notes.md` §22.2).
  useEffect(() => {
    if (same === null) {
      return undefined;
    }
    const timer = setTimeout(() => setSame(null), SAME_NOTICE_MS);
    return () => clearTimeout(timer);
  }, [same]);
  const select = useCallback((hex: string | null) => {
    setSame(null);
    setSelected(hex);
  }, []);

  const multi = values.output === MULTICOLOUR;
  const pickedOnly = !multi && values.method === PICK_COLOURS;
  const shown = useMemo<PaletteSwatch[]>(
    () => (multi ? swatches : pickedOnly ? values.swatches.map((hex) => ({ hex, fixed: true, pixels: 0 })) : []),
    [multi, pickedOnly, swatches, values.swatches],
  );
  const excludedCount = swatches.filter((s) => values.excluded.includes(s.hex)).length;
  const note = !multi
    ? ""
    : merging
      ? mergingNote(merging)
      : isolated
        ? isolatingNote(isolated)
        : excludedCount > 0
          ? excludedNote(excludedCount)
          : "";
  return {
    swatches: shown,
    found: info ? info.colours.length : null,
    joined: info?.joined ?? 0,
    selected: selected !== null && shown.some((s) => s.hex === selected) ? selected : null,
    select,
    state,
    isolated,
    merging,
    isolate,
    recolour,
    startMerge,
    mergeInto,
    cancelMerge,
    toggleExclude,
    remove,
    pick,
    pickedOnly,
    note,
    sameNote: !multi || same === null ? "" : sameAsNote(same),
    refresh,
  };
}
