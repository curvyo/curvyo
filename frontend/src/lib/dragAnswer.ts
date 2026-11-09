/**
 * The one decoder of the drag answers the session gives (`specs/0100-bring-content-in`
 * `adrs.md` note 4, architecture review finding 3): the page-row drag (`row_drag_move`) and the
 * cloning drop of a Select move (`clone_drop_answer`) answer `{badge, target, faint}`. Every
 * field and the badge are checked against the known set; anything else decodes to `null` (no
 * badge drawn). The Rust side checks the same fixture (`tests/drag_answer_contract.rs`).
 */

/** Every badge the session sends. */
export const DROP_BADGES = [
  "none",
  "clone",
  "not_on_sketch_page",
  "no_sketch",
  "move",
  "copy",
  "placed_page",
  "only_on_laser_page",
] as const;

export type DropBadgeKind = (typeof DROP_BADGES)[number];

/** One drag answer. */
export interface DragAnswer {
  badge: DropBadgeKind;
  target: string | null;
  faint: boolean;
}

function isBadge(value: unknown): value is DropBadgeKind {
  return typeof value === "string" && (DROP_BADGES as readonly string[]).includes(value);
}

/** The answer in `value` (already parsed JSON), or `null` when it is not one. */
export function asDragAnswer(value: unknown): DragAnswer | null {
  if (typeof value !== "object" || value === null) return null;
  const v = value as Record<string, unknown>;
  if (!isBadge(v.badge)) return null;
  if (!(v.target === null || typeof v.target === "string")) return null;
  if (typeof v.faint !== "boolean") return null;
  return { badge: v.badge, target: v.target, faint: v.faint };
}

/** The answer in the JSON `text`; `null` for an empty text, bad JSON or a wrong shape. */
export function decodeDragAnswer(text: string): DragAnswer | null {
  if (!text) return null;
  try {
    return asDragAnswer(JSON.parse(text));
  } catch {
    return null;
  }
}

/** Whether a badge is a refusal (the not-allowed cursor, no frame cue). */
export function isRefusal(kind: DropBadgeKind): boolean {
  return kind === "not_on_sketch_page" || kind === "no_sketch" || kind === "only_on_laser_page";
}

/** The glyph of each badge (lucide name); `null` for none. */
export const BADGE_GLYPHS: Record<DropBadgeKind, string | null> = {
  none: null,
  clone: "Link2",
  not_on_sketch_page: "Ban",
  no_sketch: "Ban",
  move: "Move",
  copy: "Copy",
  placed_page: "Frame",
  only_on_laser_page: "Ban",
};

/** The badge word; the page name truncates at 24 characters. `null`: no badge. */
export function badgeWord(kind: DropBadgeKind, pageName: string): string | null {
  const words: Record<DropBadgeKind, string | null> = {
    none: null,
    clone: "Linked clone",
    not_on_sketch_page: "Not on a sketch page",
    no_sketch: `${pageName.length > 24 ? `${pageName.slice(0, 23)}…` : pageName} has no sketch yet`,
    move: "Move",
    copy: "Copy",
    placed_page: "Placed page",
    only_on_laser_page: "Only on a laser page",
  };
  return words[kind];
}
