import { useCallback, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import type { AlignCommand, DistributeCode } from "@/lib/alignText";
import { parseGap } from "@/lib/alignText";

export interface AlignApi {
  /** The Align group is in the bar: the Select tool with something selected. */
  shown: boolean;
  /** The Distribute group is in the bar: two or more selected objects. */
  distributeShown: boolean;
  /** An align press: every selected unit onto the reference. */
  align: (command: AlignCommand) => void;
  /** A distribute press, with the Gap field's value (empty keeps the ends). */
  distribute: (code: DistributeCode) => void;
  /** The Gap field's text, as typed. */
  gap: string;
  /** Sets the Gap field's text and clears its refusal. */
  setGap: (text: string) => void;
  /** The Gap field's refusal of the last Enter, until the text changes. */
  gapError: string | null;
}

/**
 * The align and distribute groups of the Select bar
 * (`specs/0061-align-and-distribute/`): forwards a press to the session,
 * which plans the translations and commits once. The shown flags follow the
 * selection and are re-read by `onChange` after every session change.
 */
export function useAlign(
  getSession: () => WasmSession | null,
  onChange: () => void,
  shown: boolean,
  distributeShown: boolean,
): AlignApi {
  const [gap, setGapText] = useState("");
  const [gapError, setGapError] = useState<string | null>(null);

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

  return { shown, distributeShown, align, distribute, gap, setGap, gapError };
}
