import { useCallback, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  FLIP_ANNOUNCEMENT,
  FLIP_NOTICE_MS,
  FLIP_UNCHANGED_NOTICE,
  type FlipAxis,
  flipKeyAxis,
} from "@/lib/flipText";

/** A message with a counter, so that the same sentence said twice is shown or spoken twice. */
export interface FlipMessage {
  id: number;
  text: string;
}

export interface FlipApi {
  /** A press on a flip button: the flip, or the no-op notice under that button. */
  flip: (axis: FlipAxis) => void;
  /** The no-op notice of the last button press, under its button; `null` after 3 s. */
  notice: (FlipMessage & { axis: FlipAxis }) | null;
  /** The polite announcement of the last flip, by button or key. */
  announcement: FlipMessage;
  /** Handles the outcome strings of H and V; `false` for every other outcome. */
  handleKeyOutcome: (outcome: string) => boolean;
  /** Speaks `text` in the same polite region (the "Resized. Mirrored …" of a resize). */
  announceText: (text: string) => void;
  /** Empties the region: the Select bar that holds it unmounts. */
  clearAnnouncement: () => void;
}

type ShowNotice = (text: string, ms?: number, source?: "canvas" | "panel") => void;

/**
 * The flip buttons and the keys H and V in the frontend (`specs/0056-mirror-and-negative-scale/`
 * criteria 2 to 4): forwards the press, announces a flip, and shows the no-op notice under the
 * button for a press or in the canvas notice slot for a key. Every decision is the session's.
 */
export function useFlip(
  getSession: () => WasmSession | null,
  onChange: () => void,
  showNotice: ShowNotice,
): FlipApi {
  const [notice, setNotice] = useState<FlipApi["notice"]>(null);
  const [announcement, setAnnouncement] = useState<FlipMessage>({ id: 0, text: "" });
  const counter = useRef(0);
  const timer = useRef<number | undefined>(undefined);

  const announceText = useCallback((text: string) => {
    counter.current += 1;
    setAnnouncement({ id: counter.current, text });
  }, []);
  const announce = useCallback(
    (axis: FlipAxis) => announceText(FLIP_ANNOUNCEMENT[axis]),
    [announceText],
  );
  const clearAnnouncement = useCallback(() => {
    setAnnouncement((previous) => (previous.text === "" ? previous : { id: previous.id, text: "" }));
  }, []);

  const flip = useCallback(
    (axis: FlipAxis) => {
      const session = getSession();
      if (!session) {
        return;
      }
      const outcome =
        axis === "horizontal" ? session.flip_horizontally() : session.flip_vertically();
      if (outcome === "flipped") {
        announce(axis);
      } else if (outcome === "unchanged") {
        counter.current += 1;
        setNotice({ id: counter.current, text: FLIP_UNCHANGED_NOTICE, axis });
        window.clearTimeout(timer.current);
        timer.current = window.setTimeout(() => setNotice(null), FLIP_NOTICE_MS);
      }
      onChange();
    },
    [getSession, onChange, announce],
  );

  const handleKeyOutcome = useCallback(
    (outcome: string): boolean => {
      if (outcome === "flip-unchanged") {
        showNotice(FLIP_UNCHANGED_NOTICE, FLIP_NOTICE_MS, "canvas");
        return true;
      }
      const axis = flipKeyAxis(outcome);
      if (axis) {
        announce(axis);
        return true;
      }
      return false;
    },
    [announce, showNotice],
  );

  return { flip, notice, announcement, handleKeyOutcome, announceText, clearAnnouncement };
}
