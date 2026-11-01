// Reads the arrange groups' state from the session
// (`specs/0061-align-and-distribute/`): shown, the references, the selected
// count and the selection key the grid fields reset on.

import type { AlignBarState } from "@/hooks/useAlign";
import type { AlignReference } from "@/lib/alignText";
import type { WasmSession } from "@/lib/editorSession";

/** Before the session is ready: nothing shown. */
export const NO_ALIGN_BAR: AlignBarState = {
  shown: false,
  alignReference: "selection",
  distributeReference: "selection",
  selectedCount: 0,
  selectionKey: "",
};

/** The session's current arrange state. */
export function readAlignBar(session: WasmSession): AlignBarState {
  return {
    shown: session.align_shown(),
    alignReference: session.align_reference() as AlignReference,
    distributeReference: session.distribute_reference() as AlignReference,
    selectedCount: session.grid_count(),
    selectionKey: session.grid_selection_key(),
  };
}

/** Whether two reads are the same, so an unchanged read sets no state. */
export function sameAlignBar(a: AlignBarState, b: AlignBarState): boolean {
  return (
    a.shown === b.shown &&
    a.alignReference === b.alignReference &&
    a.distributeReference === b.distributeReference &&
    a.selectedCount === b.selectedCount &&
    a.selectionKey === b.selectionKey
  );
}
