/**
 * The palette of a multicolour trace in the frontend (`specs/0089` slice M, criteria 26 and 29;
 * UX notes §7, §26, §27): the swatch row read from the session after each preview and what a
 * swatch does. Exclude, delete and a pipette pick change the trace's values (traced again).
 */
import { useCallback, useEffect, useMemo, useState, type MutableRefObject } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  MULTICOLOUR,
  PICK_COLOURS,
  removedPickedText,
  type TraceChoice,
  type TraceValues,
} from "@/lib/traceText";
import {
  type PaletteInfo,
  type PaletteSwatch,
  deleteFromPalette,
  readPalette,
  stripOrder,
  excludedNote,
  sameAsNote,
  sameColour,
} from "@/lib/tracePaletteText";

/** How long "Same as ..." stays, ms (`ux-notes.md` §22.2). */
const SAME_NOTICE_MS = 3000;

/** The most colours a palette holds (the Colours field's highest count). */
const MAX_PALETTE = 16;

const NO_SWATCHES: PaletteSwatch[] = [];

export interface TracePaletteApi {
  /** The swatch row in order; empty without a multicolour preview and with Pick colours, whose
   * colours sit beside the pipette (`ux-notes.md` §25.4). */
  swatches: PaletteSwatch[];
  /** The colours Auto found (the Colours field's muted count), `null` before a result. */
  found: number | null;
  joined: number;
  excluded: (hex: string) => boolean;
  /** Ctrl+click, Enter, Space (`ux-notes.md` §27): Exclude on or off. */
  toggleExclude: (hex: string) => void;
  /** The x, Shift+Enter, Delete: the palette without `hex`, matched again, down to no colour. */
  remove: (hex: string) => void;
  /** A palette pipette pick: a new fixed colour, or the swatch it equals. */
  pick: (hex: string) => void;
  /** The palette state for the hidden status region ("Removed #4B392F", "2 colours excluded"),
   * or empty. */
  note: string;
  /** "Same as #3366CC" for 3 s after a pick that matched a colour, or empty: said by the hidden
   * region only (`ux-notes.md` §27). */
  sameNote: string;
  /** The results changed (a new preview): read the palette again. */
  refresh: () => void;
}

interface PaletteContext {
  active: boolean;
  /** The active part: the palette of another part is not this part's (customer 2026-11-10). */
  partId: number;
  /** The current values (the state the bar shows). */
  values: TraceValues;
  getSession: () => WasmSession | null;
  apply: (next: TraceChoice) => void;
  choiceRef: MutableRefObject<TraceChoice>;
}

export function useTracePalette({ active, partId, values, getSession, apply, choiceRef }: PaletteContext): TracePaletteApi {
  const [info, setInfo] = useState<PaletteInfo | null>(null);
  const [same, setSame] = useState<string | null>(null);
  /** The colour last deleted and the kept palette it left: "Removed #..." is said until that changes. */
  const [removed, setRemoved] = useState<{ hex: string; kept: string } | null>(null);

  const refresh = useCallback(() => {
    setInfo(readPalette(getSession()?.trace_palette() ?? ""));
  }, [getSession]);

  // Leaving the tool forgets the palette's session state; so does Pick colours, which has no
  // palette row (`ux-notes.md` §25.4).
  const picking = values.method === PICK_COLOURS;
  useEffect(() => {
    if (!active || picking) {
      setInfo(null);
      setSame(null);
      setRemoved(null);
    }
  }, [active, picking]);

  // Another part became active: the palette read belongs to the part before until the next preview.
  useEffect(() => {
    setInfo(null);
    setSame(null);
    setRemoved(null);
  }, [partId]);

  const edit = useCallback(
    (change: (v: TraceChoice["values"]) => Partial<TraceChoice["values"]>) => {
      const current = choiceRef.current;
      apply({ ...current, values: { ...current.values, ...change(current.values) } });
    },
    [apply, choiceRef],
  );

  const swatches = useMemo(() => (info ? stripOrder(info.colours) : []), [info]);
  const excluded = useCallback((hex: string) => values.excluded.includes(hex), [values.excluded]);

  const toggleExclude = useCallback(
    (hex: string) => {
      edit((v) => ({ excluded: v.excluded.includes(hex) ? v.excluded.filter((e) => e !== hex) : [...v.excluded, hex] }));
    },
    [edit],
  );
  const remove = useCallback(
    (hex: string) => {
      const next = deleteFromPalette(
        swatches.map((s) => s.hex),
        choiceRef.current.values.excluded,
        hex,
      );
      if (next === null) {
        return;
      }
      setRemoved({ hex, kept: next.keptPalette.join() });
      edit(() => next);
    },
    [swatches, choiceRef, edit],
  );
  const pick = useCallback(
    (hex: string) => {
      const twin = sameColour(
        swatches.map((s) => s.hex),
        hex,
      );
      if (twin) {
        setSame(twin);
        return;
      }
      setSame(null);
      // After a delete the palette is kept exactly, so a pick joins the kept colours.
      edit((v) =>
        v.keptPalette
          ? { keptPalette: v.keptPalette.length >= MAX_PALETTE ? v.keptPalette : [...v.keptPalette, hex] }
          : { fixed: v.fixed.includes(hex) ? v.fixed : [...v.fixed, hex] },
      );
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

  const multi = values.output === MULTICOLOUR;
  const shown = multi && !picking ? swatches : NO_SWATCHES;
  const excludedCount = swatches.filter((s) => values.excluded.includes(s.hex)).length;
  const note = !multi
    ? ""
    : [removed !== null && removed.kept === (values.keptPalette ?? []).join() ? removedPickedText(removed.hex) : "", excludedCount > 0 ? excludedNote(excludedCount) : ""]
        .filter((t) => t !== "")
        .join(". ");
  return {
    swatches: shown,
    found: info ? info.colours.length : null,
    joined: info?.joined ?? 0,
    excluded,
    toggleExclude,
    remove,
    pick,
    note,
    sameNote: !multi || same === null ? "" : sameAsNote(same),
    refresh,
  };
}
