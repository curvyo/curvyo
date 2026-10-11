/**
 * The words of the Grouping card and the "Open groups" indicator (`specs/0023-groups` UX notes,
 * "Group and Ungroup" and "Entered group"). The notices themselves come from the session as
 * sentences (one command, one message, whether a key or a button caused it); the tooltips and
 * the crumbs are written here.
 */

/** The two commands of the card, in rail order. */
export type GroupCommand = "group" | "ungroup";

export const GROUP_COMMANDS: readonly GroupCommand[] = ["group", "ungroup"];

/** What the card shows (`WasmSession.grouping_availability`). */
export interface GroupingAvailabilityState {
  /** Two or more objects of the context are selected and the Select tool is active. */
  group: boolean;
  /** The selection holds a group and the Select tool is active. */
  ungroup: boolean;
}

/** What `group_selection`, `ungroup_selection` and `take_grouping_result` return. */
export interface GroupingResult {
  /** `ignored`, `grouped`, `ungrouped` or `refused`. */
  kind: string;
  /** The sentence for the canvas notice slot; empty when ignored. */
  notice: string;
}

/** How long a refusal stays in the canvas notice slot (8 s, a status 3 s). */
export const GROUP_REFUSAL_MS = 8000;

/** The modifier word of the platform. */
function modifier(mac: boolean): string {
  return mac ? "Cmd" : "Ctrl";
}

/** The chord of a command: "Ctrl+G", "Ctrl+Shift+G", with Cmd on macOS. */
export function groupChord(command: GroupCommand, mac: boolean): string {
  return command === "group" ? `${modifier(mac)}+G` : `${modifier(mac)}+Shift+G`;
}

/** The first tooltip line and the accessible name: "Group (Ctrl+G)". */
export function groupCommandName(command: GroupCommand, mac: boolean): string {
  return `${command === "group" ? "Group" : "Ungroup"} (${groupChord(command, mac)})`;
}

/** The second tooltip line. */
export function groupCommandRule(command: GroupCommand): string {
  return command === "group"
    ? "Binds the selection into one group."
    : "Releases the objects of a group.";
}

/** Whether the button is dimmed. */
export function groupCommandDimmed(
  command: GroupCommand,
  availability: GroupingAvailabilityState,
): boolean {
  return command === "group" ? !availability.group : !availability.ungroup;
}

/** The third tooltip line: what the press does, or why the button is dimmed (another tool adds
 * "Use the Select tool." on a line of its own). */
export function groupCommandNote(
  command: GroupCommand,
  availability: GroupingAvailabilityState,
  selectTool: boolean,
): string {
  if (!groupCommandDimmed(command, availability)) {
    return command === "group"
      ? "Objects in between move below the group."
      : "One level only. Inner groups stay groups.";
  }
  const why = command === "group" ? "Select two or more objects." : "Select a group.";
  // The tool line stands on its own line, so the tooltip never wraps it with an orphan word.
  return selectTool ? why : `${why}\nUse the Select tool.`;
}

/** The crumbs of the indicator for `depth` entered groups: "Document", then one "Group" per
 * level. More than three levels collapse the middle ones. `level` is what a press makes the
 * context (0 is the document); `current` marks the last. */
export interface Crumb {
  label: string;
  level: number;
  current: boolean;
}

export interface CrumbRow {
  crumbs: Crumb[];
  /** How many levels the "..." stands for, 0 when nothing collapsed. */
  collapsed: number;
  /** Where the collapsed levels sit in `crumbs` (the "..." goes before this index). */
  collapsedAt: number;
}

export function crumbRow(depth: number): CrumbRow {
  const all: Crumb[] = [{ label: "Document", level: 0, current: depth === 0 }];
  for (let level = 1; level <= depth; level += 1) {
    all.push({ label: "Group", level, current: level === depth });
  }
  if (depth <= 3) {
    return { crumbs: all, collapsed: 0, collapsedAt: 0 };
  }
  // Document, "...", the last two groups.
  return {
    crumbs: [all[0], ...all.slice(-2)],
    collapsed: depth - 2,
    collapsedAt: 1,
  };
}

/** The accessible name of a crumb: "Document", or "Group, level 2" so that the levels differ. */
export function crumbName(crumb: Crumb): string {
  return crumb.level === 0 ? crumb.label : `${crumb.label}, level ${crumb.level}`;
}

/** The accessible name of the collapsed levels: "4 more levels". */
export function collapsedName(collapsed: number): string {
  return `${collapsed} more ${collapsed === 1 ? "level" : "levels"}`;
}
