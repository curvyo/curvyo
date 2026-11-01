// The words of the align and distribute buttons
// (`specs/0061-align-and-distribute/` criteria 1, 2, 6, 8 and the UX notes).
// The planning, the reference and the commit are the session's; these are
// the fixed lines of the UI.

/** The six align commands, in bar order. */
export type AlignCommand = "left" | "centre-h" | "right" | "top" | "centre-v" | "bottom";

/** The four distribute buttons, in bar order. */
export type DistributeCode = "h-gaps" | "h-centres" | "v-gaps" | "v-centres";

/** The accessible name and the tooltip's rule line of every align button. */
export const ALIGN_TEXT: Record<AlignCommand, { name: string; rule: string }> = {
  left: { name: "Align left edges", rule: "Left edges land on the reference's left edge." },
  "centre-h": {
    name: "Align horizontal centres",
    rule: "Centres land on the reference's horizontal centre.",
  },
  right: { name: "Align right edges", rule: "Right edges land on the reference's right edge." },
  top: { name: "Align top edges", rule: "Top edges land on the reference's top edge." },
  "centre-v": {
    name: "Align vertical centres",
    rule: "Centres land on the reference's vertical centre.",
  },
  bottom: { name: "Align bottom edges", rule: "Bottom edges land on the reference's bottom edge." },
};

/** The align rules with the **active page** as the reference: what the
 * buttons do when nothing is selected (the customer's extension of
 * 2026-10-30). */
export const ALIGN_PAGE_RULES: Record<AlignCommand, string> = {
  left: "Nothing selected: every object of the page onto its left edge.",
  "centre-h": "Nothing selected: every object of the page on its horizontal centre.",
  right: "Nothing selected: every object of the page onto its right edge.",
  top: "Nothing selected: every object of the page onto its top edge.",
  "centre-v": "Nothing selected: every object of the page on its vertical centre.",
  bottom: "Nothing selected: every object of the page onto its bottom edge.",
};

/** The accessible name and the tooltip's rule line of every distribute button. */
export const DISTRIBUTE_TEXT: Record<DistributeCode, { name: string; rule: string }> = {
  "h-gaps": {
    name: "Distribute horizontal gaps",
    rule: "Equal gaps between the objects, horizontally. The outermost stay.",
  },
  "h-centres": {
    name: "Distribute horizontal centres",
    rule: "Equal distances between the centres, horizontally. The outermost stay.",
  },
  "v-gaps": {
    name: "Distribute vertical gaps",
    rule: "Equal gaps between the objects, top to bottom. The outermost stay.",
  },
  "v-centres": {
    name: "Distribute vertical centres",
    rule: "Equal distances between the centres, top to bottom. The outermost stay.",
  },
};

/** The distribute rules spread over the **active page**: what the buttons do
 * when nothing is selected (the customer's extension of 2026-10-30). */
export const DISTRIBUTE_PAGE_RULES: Record<DistributeCode, string> = {
  "h-gaps": "Nothing selected: the page's objects evenly across the page, equal gaps.",
  "h-centres": "Nothing selected: the page's objects evenly across the page, equal centres.",
  "v-gaps": "Nothing selected: the page's objects evenly down the page, equal gaps.",
  "v-centres": "Nothing selected: the page's objects evenly down the page, equal centres.",
};

/** What a press acts on (the session's `align_reference` and
 * `distribute_reference`): the page's objects with nothing selected, the
 * selected objects relative to the page, or the selection. */
export type AlignReference = "page-objects" | "page" | "selection";

/** The align rules with **one object** selected: the page is the reference
 * (criterion 14, UX notes "Grid and page reference"). */
export const ALIGN_ONE_RULES: Record<AlignCommand, string> = {
  left: "Its left edge onto the page's left edge.",
  "centre-h": "Its centre on the page's horizontal centre.",
  right: "Its right edge onto the page's right edge.",
  top: "Its top edge onto the page's top edge.",
  "centre-v": "Its centre on the page's vertical centre.",
  bottom: "Its bottom edge onto the page's bottom edge.",
};

/** The distribute rules with **one object** selected: centred across the
 * page (criterion 16). */
export const DISTRIBUTE_ONE_RULES: Record<DistributeCode, string> = {
  "h-gaps": "Centred across the page, equal gaps to both edges.",
  "h-centres": "Centred across the page.",
  "v-gaps": "Centred down the page, equal gaps to both edges.",
  "v-centres": "Centred down the page.",
};

/** The distribute rules with **two objects** selected: spread over the page
 * (criterion 16, Proposal wording). */
export const DISTRIBUTE_TWO_RULES: Record<DistributeCode, string> = {
  "h-gaps": "Across the page, equal gaps between them and to both edges.",
  "h-centres": "Across the page, the centres on its thirds.",
  "v-gaps": "Down the page, equal gaps between them and to both edges.",
  "v-centres": "Down the page, the centres on its thirds.",
};

/** The tooltip's rule line of an align button, for the current reference. */
export function alignRule(command: AlignCommand, reference: AlignReference): string {
  if (reference === "page-objects") {
    return ALIGN_PAGE_RULES[command];
  }
  return reference === "page" ? ALIGN_ONE_RULES[command] : ALIGN_TEXT[command].rule;
}

/** The tooltip's rule line of a distribute button, for the current reference
 * and the number of selected objects. */
export function distributeRule(
  code: DistributeCode,
  reference: AlignReference,
  count: number,
): string {
  if (reference === "page-objects") {
    return DISTRIBUTE_PAGE_RULES[code];
  }
  if (reference === "page") {
    return count <= 1 ? DISTRIBUTE_ONE_RULES[code] : DISTRIBUTE_TWO_RULES[code];
  }
  return DISTRIBUTE_TEXT[code].rule;
}

/** Whether a button with this reference marks the active page on the canvas
 * while hovered or focused (UX notes, "Canvas cue"). */
export function pageCueFor(reference: AlignReference): boolean {
  return reference !== "selection";
}

/** The Grid button's name and rule line (criterion 18). */
export const GRID_TEXT = {
  name: "Arrange in a grid",
  rule: "Fills rows and columns in reading order, centred in equal cells.",
} as const;

/** The bar order of the six align buttons. */
export const ALIGN_ORDER: AlignCommand[] = [
  "left",
  "centre-h",
  "right",
  "top",
  "centre-v",
  "bottom",
];

/** The bar order of the four distribute buttons. */
export const DISTRIBUTE_ORDER: DistributeCode[] = ["h-gaps", "h-centres", "v-gaps", "v-centres"];

/** The Gap field's refusal of text that is not a number (the bar fields' rule). */
export const GAP_INVALID_NUMBER = "Enter a number";

/** The Gap field's refusal of a negative gap. */
export const GAP_NEGATIVE = "Must be 0 or more.";

/** Reads the Gap field: an empty line is "keep the ends" (`null`), a valid
 * line its millimetres, an invalid one the message to show. */
export function parseGap(text: string): { ok: true; mm: number | null } | { ok: false; message: string } {
  const cleaned = text.trim().replaceAll(",", ".").replaceAll("−", "-");
  if (cleaned === "") {
    return { ok: true, mm: null };
  }
  const value = Number(cleaned);
  if (!Number.isFinite(value)) {
    return { ok: false, message: GAP_INVALID_NUMBER };
  }
  if (value < 0) {
    return { ok: false, message: GAP_NEGATIVE };
  }
  return { ok: true, mm: value };
}
