/**
 * The words of the Node tool's count, Join reason (`specs/0053-node-marquee-selection/`
 * criteria 19 and 23) and the Join and Split notices on compound paths
 * (`specs/0055-compound-path-node-editing/` criteria 5 and 6). Rust sends codes and counts; the
 * sentences are written here, once.
 */

/** What the node count and the Join reason read from `NodeToolbarState`. */
export interface NodeCountState {
  /** 0 Join enabled, 1 not exactly two nodes, 2 not end nodes, 3 another reason. */
  joinBlock: number;
  /** With `joinBlock` 2: how many selected nodes are not an end node. */
  joinNotEnd: number;
  selectedNodes: number;
  totalNodes: number;
  segmentSelected: boolean;
}

function nodes(count: number): string {
  return count === 1 ? "1 node" : `${count} nodes`;
}

/**
 * The status-bar count (criterion 23): "5 of 120 nodes", "120 nodes" with nothing selected,
 * "Segment, 120 nodes" with a segment selected; `null` when the editing set has no node.
 */
export function nodeCountText(state: NodeCountState): string | null {
  if (state.totalNodes === 0) {
    return null;
  }
  if (state.segmentSelected) {
    return `Segment, ${nodes(state.totalNodes)}`;
  }
  if (state.selectedNodes === 0) {
    return nodes(state.totalNodes);
  }
  return `${state.selectedNodes} of ${nodes(state.totalNodes)}`;
}

/**
 * The muted reason line of a disabled Join's tooltip (criterion 19), `null` when Join is
 * enabled, when no node is selected, or when Join refuses for a reason the texts do not cover.
 */
export function joinReasonText(state: NodeCountState): string | null {
  if (state.selectedNodes === 0) {
    return null;
  }
  if (state.joinBlock === 1) {
    return `Needs exactly two end nodes of open paths or outlines. ${nodes(state.selectedNodes)} selected.`;
  }
  if (state.joinBlock === 2) {
    const verb = state.joinNotEnd === 1 ? "is" : "are";
    return `Needs end nodes of open paths or outlines. ${state.joinNotEnd} of ${state.selectedNodes} selected nodes ${verb} not an end node.`;
  }
  return null;
}

/**
 * `joinReasonText` as two tooltip lines, the rule and the count, so neither wraps
 * mid-sentence in the 256 px tooltip (UX review of 2026-10-14, finding 4).
 */
export function joinReasonLines(state: NodeCountState): [string, string] | null {
  const text = joinReasonText(state);
  if (text === null) {
    return null;
  }
  const cut = text.indexOf(". ") + 1;
  return [text.slice(0, cut), text.slice(cut + 1)];
}

/**
 * The Join notice (`0055` criterion 5 c): ends in two objects moved the other object's outlines
 * besides the joined one into the survivor. `null` when none moved.
 */
export function joinNoticeText(movedOutlines: number): string | null {
  if (movedOutlines <= 0) {
    return null;
  }
  const outlines = movedOutlines === 1 ? "1 more outline" : `${movedOutlines} more outlines`;
  return `Join: ${outlines} moved into this compound path.`;
}

/**
 * The Split notice of a compound path (`0055` criterion 6), by the code `split_selected` returns:
 * the result stays one object. `null` for an ordinary path or nothing split.
 */
export function splitNoticeText(code: string): string | null {
  switch (code) {
    case "outline_in_two":
      return "Split: 1 outline became 2. They stay in this compound path.";
    case "outline_opened":
      return "Split: 1 closed outline was opened.";
    default:
      return null;
  }
}
