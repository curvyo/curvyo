// What the jog pad remembers on this computer: open or closed and the step (`specs/0118` UX notes
// W16). Not in the project, not synced; a storage that is blocked or empty means "closed, 1 mm".
import { useState } from "react";

import { type JogView, readJogView, writeJogView } from "@/lib/jog";

function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** The remembered view and a setter that remembers the new one. */
export function useJogView(): [JogView, (view: JogView) => void] {
  const [view, set] = useState<JogView>(() => readJogView(storage()));
  return [
    view,
    (next) => {
      set(next);
      writeJogView(storage(), next);
    },
  ];
}
