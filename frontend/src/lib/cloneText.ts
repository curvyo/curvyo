// The sentences and timings of linked clones in the frontend (`specs/0049-linked-clone`). The
// notices of the commands come from the session; these are the fixed lines of the UI.

/** The Node bar and the double-click hint on a clone (criterion 26). */
export const CLONE_NODES_TEXT = "Linked clones have no nodes. Edit the original or unlink the clone.";

/** A refusal stays 8 s, a status 3 s (criterion 52, "Action notice"). */
export const CLONE_REFUSAL_MS = 8000;
export const CLONE_STATUS_MS = 3000;

/** The orphan repair after an Open stays 8 s (criterion 35). */
export const ORPHAN_NOTICE_MS = 8000;

/** The tooltip of the Select-bar button, three lines (UX notes). */
export const LINKED_CLONE_TOOLTIP = [
  "Linked clone",
  "A second copy that follows the original.",
  "Edit the original and every clone changes.",
];

/** The state of one override row over the selected clones (`specs/0057-clone-overrides`). */
export type OverrideState = "original" | "own" | "mixed";

/** What the Style tab shows about linked clones, as the session reads it. */
export interface ClonePanelState {
  clonesOnly: boolean;
  count: number;
  original: string;
  deepest: number;
  root: string;
  followers: number;
  followed: number;
  /** The Placement row: "original" is Follows original. */
  placement: OverrideState;
  /** The Style row: "original" is Original's. */
  style: OverrideState;
  /** Whether the Placement row is shown: not for text clones (`0080` criterion 40). */
  placementRow: boolean;
  /** Whether every selected clone is a text clone (Unlink makes texts, `0080` criterion 41). */
  texts: boolean;
  /** Spread colours is enabled (two or more clones). */
  spreadReady: boolean;
  /** The greyed headers' summaries (the first clone's effective style). */
  strokeSummary: string;
  strokeHex: string;
  fillHex: string;
  markersSummary: string;
}

export const NO_CLONE_PANEL: ClonePanelState = {
  clonesOnly: false,
  count: 0,
  original: "",
  deepest: 0,
  root: "",
  followers: 0,
  followed: 0,
  placement: "original",
  style: "original",
  placementRow: true,
  texts: false,
  spreadReady: false,
  strokeSummary: "",
  strokeHex: "",
  fillHex: "",
  markersSummary: "",
};

/** The two segments of an override row: label, accessible name and tooltip (`0057` UX notes). */
export interface OverrideSegment {
  own: boolean;
  label: string;
  name: string;
  title: string;
}

export const PLACEMENT_SEGMENTS: OverrideSegment[] = [
  {
    own: false,
    label: "Follows original",
    name: "Follows original",
    title: "Moves, turns and scales with the original.",
  },
  { own: true, label: "Own", name: "Own placement", title: "Stays where it is when the original is moved." },
];

export const STYLE_SEGMENTS: OverrideSegment[] = [
  {
    own: false,
    label: "Original's",
    name: "Original's style",
    title: "Shows the original's fill, stroke, markers and opacity.",
  },
  { own: true, label: "Own", name: "Own style", title: "Has its own fill, stroke, markers and opacity." },
];

/** The tooltip of Spread colours, enabled and disabled (criterion 42). */
export const SPREAD_COLOURS_TITLE = "Gives the selected clones evenly spaced hues, in stacking order.";
export const SPREAD_COLOURS_DISABLED_TITLE = "Select two or more clones to spread colours.";

/** A tag or an edge chip of the canvas marks, in canvas px. */
export interface CloneTag {
  kind: "tag" | "chip";
  text: string;
  x: number;
  y: number;
  w: number;
  toward: number;
}

/** The text lines of the clone block (criterion 22). */
export function cloneBlockLines(panel: ClonePanelState): { title: string; chain: string } {
  const many = panel.count > 1;
  const title = many ? `${panel.count} linked clones.` : `Linked clone of ${panel.original}.`;
  const chain =
    panel.deepest < 2
      ? ""
      : many
        ? `Chains of up to ${panel.deepest} clones.`
        : `Chain of ${panel.deepest} clones. The root original is ${panel.root}.`;
  return { title, chain };
}


/** The tooltip of Unlink clone: a linked clone becomes a path, a text clone a text (`0080`
 * criterion 41). */
export function unlinkTitle(panel: ClonePanelState): string {
  return panel.texts
    ? "Turns the clone into an ordinary text. Text, place and style stay."
    : "Turns the clone into an ordinary path. Shape and style stay.";
}
