import { useCallback, useEffect, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { playKeyAction } from "@/lib/playKeys";
import { decodePlayViewText, type PlayView } from "@/lib/outputView";

/**
 * The play bar's state (`specs/0029-laser-job-preview-and-output` criterion 37; UX notes 5): it
 * runs while the bar is mounted, one read of the session's play view per animation frame and, while
 * Play runs, one tick of the clock. The slider position, the readout and Play are the session's;
 * nothing here is a history step.
 */
export interface PlayBarApi {
  view: PlayView | null;
  /** Play or Pause. */
  toggle: () => void;
  /** The slider moved to `fraction` (0 to 1) of the job's time. */
  seek: (fraction: number) => void;
  /** A key on the slider; whether it was one the slider handles. */
  key: (key: string) => boolean;
}

/** The play bar's API for the session `getSession` gives. */
export function usePlayBar(getSession: () => WasmSession | null): PlayBarApi {
  const [view, setView] = useState<PlayView | null>(null);
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const session = getSession();
      if (session) {
        let text = session.play_view_json();
        if (decodePlayViewText(text)?.playing) {
          session.play_tick(performance.now());
          text = session.play_view_json();
        }
        if (text !== previous) {
          previous = text;
          setView(decodePlayViewText(text));
        }
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  const toggle = useCallback(() => {
    getSession()?.toggle_play(performance.now());
  }, [getSession]);
  const seek = useCallback(
    (fraction: number) => {
      getSession()?.set_play_position(Math.min(1, Math.max(0, fraction)));
    },
    [getSession],
  );
  const key = useCallback(
    (name: string) => {
      const action = playKeyAction(name);
      const session = getSession();
      if (!action || !session) return false;
      if (action.kind === "step") session.step_play_position(action.percent);
      else session.set_play_position(action.fraction);
      return true;
    },
    [getSession],
  );
  return { view, toggle, seek, key };
}
