import { useCallback, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import type { AlignCommand, AlignReference, DistributeCode } from "@/lib/alignText";
import { parseGap } from "@/lib/alignText";
import {
  GRID_NOT_ENOUGH_CELLS,
  type GridErrors,
  type GridFields,
  gridPlan,
  nearestSquareGrid,
} from "@/lib/gridText";

/** The Grid group's state (`specs/0061-align-and-distribute/` criteria 18 to 22). */
export interface GridApi {
  /** The group is in the bar: two or more selected objects (criterion 18). */
  shown: boolean;
  /** The fields as typed; an empty count shows the derived value. */
  fields: GridFields;
  /** The counts the empty fields stand for (criterion 19). */
  derived: { rows: number; columns: number };
  /** The refusal per field of the last press, until that field changes. */
  errors: GridErrors;
  /** Sets one field; typing a count makes it the driver, the other derived. */
  setField: (name: keyof GridFields, text: string) => void;
  /** A press of the Grid button or Enter in a field. */
  arrange: () => void;
  /** Escape in a field: the three fields as they were on focus, no refusal. */
  restore: (fields: GridFields) => void;
}

export interface AlignApi {
  /** The Align and Distribute groups are in the bar: the Select tool. */
  shown: boolean;
  /** What an align press acts on now. */
  alignReference: AlignReference;
  /** What a distribute press acts on now. */
  distributeReference: AlignReference;
  /** The number of selected objects (a group counts once). */
  selectedCount: number;
  /** An align press. */
  align: (command: AlignCommand) => void;
  /** A distribute press, with the Gap field's value (empty keeps the ends). */
  distribute: (code: DistributeCode) => void;
  /** The Gap field's text, as typed. */
  gap: string;
  /** Sets the Gap field's text and clears its refusal. */
  setGap: (text: string) => void;
  /** The Gap field's refusal of the last Enter, until the text changes. */
  gapError: string | null;
  /** Shows or hides the page cue on the canvas (UX notes, "Canvas cue"). */
  setPageCue: (on: boolean) => void;
  /** The Grid group. */
  grid: GridApi;
}

const EMPTY_FIELDS: GridFields = { rows: "", columns: "", gap: "" };

/** What the session says about the groups, re-read after every change. */
export interface AlignBarState {
  shown: boolean;
  alignReference: AlignReference;
  distributeReference: AlignReference;
  selectedCount: number;
  /** Changes with the selected objects: the grid fields reset then (criterion 21). */
  selectionKey: string;
}

/**
 * The Align, Distribute and Grid groups of the Select bar
 * (`specs/0061-align-and-distribute/`): forwards a press to the session,
 * which plans the translations and commits once, and keeps the fields.
 */
export function useAlign(
  getSession: () => WasmSession | null,
  onChange: () => void,
  state: AlignBarState,
): AlignApi {
  const [gap, setGapText] = useState("");
  const [gapError, setGapError] = useState<string | null>(null);
  const [fields, setFields] = useState<GridFields>(EMPTY_FIELDS);
  const [errors, setErrors] = useState<GridErrors>({});
  const [fieldsKey, setFieldsKey] = useState(state.selectionKey);
  // The grid fields keep their values until the selection changes (criterion 21).
  if (fieldsKey !== state.selectionKey) {
    setFieldsKey(state.selectionKey);
    setFields(EMPTY_FIELDS);
    setErrors({});
  }

  const setGap = useCallback((text: string) => {
    setGapText(text);
    setGapError(null);
  }, []);

  const distribute = useCallback(
    (code: DistributeCode) => {
      const session = getSession();
      if (!session) {
        return;
      }
      const parsed = parseGap(gap);
      if (!parsed.ok) {
        setGapError(parsed.message);
        return;
      }
      session.distribute(code, parsed.mm);
      onChange();
    },
    [getSession, gap, onChange],
  );

  const align = useCallback(
    (command: AlignCommand) => {
      const session = getSession();
      if (!session) {
        return;
      }
      session.align(command);
      onChange();
    },
    [getSession, onChange],
  );

  const setPageCue = useCallback(
    (on: boolean) => {
      getSession()?.set_page_cue(on);
    },
    [getSession],
  );

  const setField = useCallback((name: keyof GridFields, text: string) => {
    setFields((previous) => {
      if (name === "rows") {
        return { ...previous, rows: text, columns: "" };
      }
      if (name === "columns") {
        return { ...previous, columns: text, rows: "" };
      }
      return { ...previous, gap: text };
    });
    setErrors((previous) =>
      name === "gap" ? { ...previous, gap: undefined } : { gap: previous.gap },
    );
  }, []);

  const restore = useCallback((previous: GridFields) => {
    setFields(previous);
    setErrors({});
  }, []);

  const count = state.selectedCount;
  const arrange = useCallback(() => {
    const session = getSession();
    if (!session || count < 2) {
      return;
    }
    const plan = gridPlan(count, fields);
    if (!plan.ok) {
      setErrors(plan.errors);
      return;
    }
    const outcome = session.arrange_grid(plan.rows, plan.columns, plan.gapMm);
    if (outcome === "too-few-cells") {
      setErrors({ columns: GRID_NOT_ENOUGH_CELLS(count), rows: "" });
      return;
    }
    onChange();
  }, [getSession, count, fields, onChange]);

  const plan = gridPlan(count, fields);
  const derived = plan.ok ? plan.derived : nearestSquareGrid(count);

  return {
    shown: state.shown,
    alignReference: state.alignReference,
    distributeReference: state.distributeReference,
    selectedCount: count,
    align,
    distribute,
    gap,
    setGap,
    gapError,
    setPageCue,
    grid: { shown: state.shown && count >= 2, fields, derived, errors, setField, arrange, restore },
  };
}
