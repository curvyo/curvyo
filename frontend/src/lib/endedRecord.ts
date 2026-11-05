// The ended record of a removed or left workspace as the page reads it (`specs/0097`
// criteria 20 and 21, ADR 0041 decision 6): the marker's `share.ended.json` and its list. The
// session decides from the bytes; the page only needs where it points and why it ended.

/** The marker's ended record and list, as the host answers them. */
export interface EndedMarker {
  ended_json: number[];
  list: number[];
}

/** What the page reads from an ended record. */
export interface EndedFacts {
  server: string;
  /** The workspace's room id (hex): the credential store's slot of its keys. */
  room: string;
  left: boolean;
}

/** The facts of `share.ended.json`, `null` when it is not one this build reads. */
export function parseEnded(bytes: ArrayLike<number>): EndedFacts | null {
  try {
    const parsed = JSON.parse(
      new TextDecoder().decode(Uint8Array.from(bytes)),
    ) as {
      format_version?: number;
      server?: string;
      room?: string;
      ended?: string;
    };
    if (
      parsed.format_version !== 1 ||
      typeof parsed.server !== "string" ||
      typeof parsed.room !== "string"
    ) {
      return null;
    }
    if (parsed.ended !== "removed" && parsed.ended !== "left") {
      return null;
    }
    return {
      server: parsed.server,
      room: parsed.room,
      left: parsed.ended === "left",
    };
  } catch {
    return null;
  }
}

/** What the device group's list says, for telling a switch-off from a removal. */
export interface GroupFacts {
  /** The workspaces the list names, on or off, not left. */
  listed: ReadonlySet<string>;
  /** Those switched on. */
  switchedOn: ReadonlySet<string>;
  /** The member ids of my devices. */
  members: ReadonlySet<string>;
}

/** Whether the end of workspace `id` is a switch-off by my own devices, not a removal by
 * another maker's admin (`specs/0096-own-devices-sync` ux-notes 6.4, `specs/0097` criterion
 * 20): decided by who signed the removal (`removedBy`, hex), else, when the keyring does not
 * hold it (refused on reconnect), by the list's switch. A leave is never a switch-off. */
export function isSwitchOff(
  group: GroupFacts,
  id: string,
  removedBy: string,
  left: boolean,
): boolean {
  if (left || !group.listed.has(id)) {
    return false;
  }
  return removedBy !== ""
    ? group.members.has(removedBy)
    : !group.switchedOn.has(id);
}
