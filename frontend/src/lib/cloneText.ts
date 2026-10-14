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

/** What the Style tab shows about linked clones, as the session reads it. */
export interface ClonePanelState {
  clonesOnly: boolean;
  count: number;
  original: string;
  deepest: number;
  root: string;
  followers: number;
  followed: number;
}

export const NO_CLONE_PANEL: ClonePanelState = {
  clonesOnly: false,
  count: 0,
  original: "",
  deepest: 0,
  root: "",
  followers: 0,
  followed: 0,
};

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

