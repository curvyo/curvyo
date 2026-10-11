import { useCallback, useEffect, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import {
  GROUP_REFUSAL_MS,
  type GroupCommand,
  type GroupingAvailabilityState,
  type GroupingResult,
} from "@/lib/groupText";

/** What the Grouping card shows and does (`specs/0023-groups`). */
export interface GroupingCommands {
  availability: GroupingAvailabilityState;
  /** Runs `command` on the selection. Does nothing while dimmed (the session ignores it). */
  apply: (command: GroupCommand) => void;
}

/** What a screen reader hears about the entered group, with a counter so that the same sentence
 * said twice is announced twice. */
export interface ContextAnnouncement {
  id: number;
  text: string;
}

export interface GroupingApi {
  commands: GroupingCommands;
  /** How many groups deep the Select tool is; 0 at the document root. */
  depth: number;
  announcement: ContextAnnouncement;
  /** A crumb of the indicator: makes `level` the context. */
  leaveToLevel: (level: number) => void;
  /** Re-reads the session. Call after any change of it. */
  refresh: () => void;
  /** Handles the outcome strings of the Group and Ungroup chords (the notice); `false` for every
   * other outcome. Entering and leaving a group need nothing but the re-read. */
  handleKeyOutcome: (outcome: string) => boolean;
}

const NONE: GroupingAvailabilityState = { group: false, ungroup: false };

/** Shows `result` in the canvas notice slot: a status for 3 s, a refusal as an alert for 8 s. */
type ShowNotice = (
  text: string,
  ms?: number,
  source?: "canvas" | "panel",
  kind?: "status" | "alert",
) => void;

/**
 * Groups in the frontend (`specs/0023-groups`): the card's availability, the two commands, the
 * depth the indicator "Open groups" shows and the announcements of entering and leaving. Every
 * decision is the session's; this hook forwards presses and shows what comes back. A key and a
 * button produce one message in one place, the canvas notice slot.
 */
export function useGrouping(
  getSession: () => WasmSession | null,
  onChange: () => void,
  showNotice: ShowNotice,
): GroupingApi {
  const [availability, setAvailability] = useState<GroupingAvailabilityState>(NONE);
  const [depth, setDepth] = useState(0);
  const [announcement, setAnnouncement] = useState<ContextAnnouncement>({ id: 0, text: "" });
  const counter = useRef(0);
  const onChangeRef = useRef(onChange);
  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  const refresh = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    const raw = session.grouping_availability();
    const next = { group: raw.group, ungroup: raw.ungroup };
    raw.free();
    setAvailability((previous) =>
      previous.group === next.group && previous.ungroup === next.ungroup ? previous : next,
    );
    setDepth(session.context_depth());
    const said = session.take_context_announcement();
    if (said !== "") {
      counter.current += 1;
      setAnnouncement({ id: counter.current, text: said });
    }
  }, [getSession]);

  const show = useCallback(
    (result: GroupingResult) => {
      if (result.kind === "ignored" || result.notice === "") {
        return;
      }
      const refused = result.kind === "refused";
      showNotice(result.notice, refused ? GROUP_REFUSAL_MS : undefined, "canvas", refused ? "alert" : "status");
    },
    [showNotice],
  );

  const apply = useCallback(
    (command: GroupCommand) => {
      const session = getSession();
      if (!session) {
        return;
      }
      const raw = command === "group" ? session.group_selection() : session.ungroup_selection();
      const result = { kind: raw.kind, notice: raw.notice };
      raw.free();
      show(result);
      onChangeRef.current();
    },
    [getSession, show],
  );

  const leaveToLevel = useCallback(
    (level: number) => {
      if (getSession()?.leave_to_level(level)) {
        onChangeRef.current();
      }
    },
    [getSession],
  );

  const handleKeyOutcome = useCallback(
    (outcome: string): boolean => {
      switch (outcome) {
        case "group":
        case "ungroup": {
          const raw = getSession()?.take_grouping_result();
          if (raw) {
            show({ kind: raw.kind, notice: raw.notice });
            raw.free();
          }
          return true;
        }
        default:
          return false;
      }
    },
    [getSession, show],
  );

  return {
    commands: { availability, apply },
    depth,
    announcement,
    leaveToLevel,
    refresh,
    handleKeyOutcome,
  };
}
