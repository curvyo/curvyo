/**
 * The Fixtures tab's set, placement and Apply logic (`specs/0113-fixture-sets-and-pins`
 * `ux-notes.md` sections 3, 4, 5 and 9): the section counts, the status words, the keys of a set
 * row, where a drag lands, the keys of keyboard placement and the Apply list's texts. Pure: the
 * components only draw what these return, and the session holds every rule.
 */
import type { FixtureKind, FixturesView, SetRow } from "./fixturesView.ts";
import { KIND_WORDS } from "./fixtureRows.ts";

/** The most characters a set's or a fixture's name holds (UX notes 4.1). */
export const MAX_NAME = 40;

/** Machine sets' heading count "2 of 3 on this page"; `null` while the page has no set (section 3). */
export function setsCountText(view: Pick<FixturesView, "sets" | "setsOnPage">): string | null {
  return view.sets.length === 0 ? null : `${view.setsOnPage} of ${view.sets.length} on this page`;
}

/** "1 fixture", "2 fixtures": what a set's armed trash says goes with it. */
function fixtureCount(n: number): string {
  return n === 1 ? "1 fixture" : `${n} fixtures`;
}

/** The status on arming a set's trash (section 4.1). */
export function setArmStatus(set: Pick<SetRow, "name" | "fixtures">): string {
  const n = set.fixtures.length;
  return n === 0
    ? `Press again to delete ${set.name}.`
    : `Press again to delete ${set.name}. Its ${fixtureCount(n)} ${n === 1 ? "goes" : "go"} with it.`;
}

/** The status after a removal (section 10). */
export function removedStatus(name: string): string {
  return `${name} removed.`;
}

/** The status on arming the trash of a fixture (section 10). */
export function fixtureArmStatus(name: string): string {
  return `Press again to delete ${name}.`;
}

/** The status on arming a placement, once (section 5.1). */
export function placingStatus(kind: FixtureKind): string {
  return `Placing ${KIND_WORDS[kind]}.`;
}

/** The accessible name of a set row's On this page check. */
export function onPageName(name: string): string {
  return `${name} on this page`;
}

/** A name as the session takes it: trimmed, 1 to 40 characters; `null` for an empty text. */
export function cleanName(text: string): string | null {
  const trimmed = text.trim();
  return trimmed === "" ? null : trimmed.slice(0, MAX_NAME);
}

/** The renamed text when it is a real change of `name`, else `null`. */
export function renamedText(text: string, name: string): string | null {
  const clean = cleanName(text);
  return clean !== null && clean !== name ? clean : null;
}

/** What a key on a focused set row asks for (section 4.1 and 9). */
export type SetKeyAction =
  | { kind: "focus"; index: number }
  | { kind: "toggle" }
  | { kind: "rename" }
  | { kind: "arm" }
  | { kind: "remove" }
  | { kind: "move"; to: number };

/**
 * The action of `key` on the set row `index` of `count`, or `null` for a key the row keeps. Up,
 * Down, Home and End move the focus (no wrap); Enter and Space fold, F2 renames, Delete arms and a
 * second Delete or Enter removes; Alt+Up, Alt+Down, Alt+Home and Alt+End move the set (a move that
 * would not change its place is `null`).
 */
export function setKeyAction(key: string, alt: boolean, index: number, count: number, armed: boolean): SetKeyAction | null {
  if (count <= 0) return null;
  const last = count - 1;
  if (alt) {
    const to = key === "ArrowUp" ? index - 1 : key === "ArrowDown" ? index + 1 : key === "Home" ? 0 : key === "End" ? last : null;
    if (to === null || to < 0 || to > last || to === index) return null;
    return { kind: "move", to };
  }
  switch (key) {
    case "ArrowDown":
      return { kind: "focus", index: Math.min(last, index + 1) };
    case "ArrowUp":
      return { kind: "focus", index: Math.max(0, index - 1) };
    case "Home":
      return { kind: "focus", index: 0 };
    case "End":
      return { kind: "focus", index: last };
    case "Enter":
      return armed ? { kind: "remove" } : { kind: "toggle" };
    case " ":
      return armed ? null : { kind: "toggle" };
    case "F2":
      return { kind: "rename" };
    case "Delete":
      return armed ? { kind: "remove" } : { kind: "arm" };
    default:
      return null;
  }
}

/** The pointer travel after which a press on a set row becomes a drag, px (section 4.1). */
export const DRAG_START_PX = 4;

/** Whether the pointer has travelled far enough from the press to start the drag. */
export function dragStarts(startY: number, pointerY: number): boolean {
  return Math.abs(pointerY - startY) >= DRAG_START_PX;
}

/** Where a dragged set lands. */
export interface SetLanding {
  /** The gap the 2px insertion line sits in: 0 above the first row, `count` under the last. */
  gap: number;
  /** The place the set takes among the sets once it left its own; `null` where it stays. */
  to: number | null;
}

/**
 * The landing of the set at `source` for a pointer at `pointerY`, given the rows' vertical
 * midpoints in order: the gap is the number of midpoints above the pointer.
 */
export function setLanding(source: number, middles: readonly number[], pointerY: number): SetLanding {
  const gap = middles.filter((middle) => pointerY >= middle).length;
  const to = gap > source ? gap - 1 : gap;
  return { gap, to: to === source ? null : to };
}

/** The distance in mm a keyboard placement moves per arrow (UX notes 5.1); Shift moves ten. */
export const PLACE_STEP_MM = 1;

/** The move of an arrow key during keyboard placement, in mm (y grows downward); `null` for any other key. */
export function placementNudge(key: string, shift: boolean): [number, number] | null {
  const step = shift ? PLACE_STEP_MM * 10 : PLACE_STEP_MM;
  switch (key) {
    case "ArrowLeft":
      return [-step, 0];
    case "ArrowRight":
      return [step, 0];
    case "ArrowUp":
      return [0, -step];
    case "ArrowDown":
      return [0, step];
    default:
      return null;
  }
}

/** The label of the Apply button: "Apply 4 fixtures" (section 5.3). */
export function applyLabel(count: number): string {
  return `Apply ${fixtureCount(count)}`;
}

/**
 * The line under a picked Apply row: Apply replaces this page's `held` job fixtures
 * (`specs/0113-fixture-sets-and-pins` criterion 35); `null` when this page has none.
 */
export function applyReplacesText(held: number): string | null {
  if (held <= 0) return null;
  return held === 1 ? "Replaces 1 job fixture on this page." : `Replaces ${held} job fixtures on this page.`;
}

/** The count at the right of an Apply row: "4 job fixtures". */
export function candidateCountText(count: number): string {
  return count === 1 ? "1 job fixture" : `${count} job fixtures`;
}

/** The Apply row of `picked` among `ids` after `key` (arrows choose a page, no wrap); `null` for another key. */
export function applyKeyTarget(key: string, ids: readonly string[], picked: string | null): string | null {
  if (ids.length === 0) return null;
  const at = picked === null ? -1 : ids.indexOf(picked);
  if (key === "ArrowDown" || key === "ArrowRight") return ids[Math.min(ids.length - 1, at + 1)] ?? null;
  if (key === "ArrowUp" || key === "ArrowLeft") return ids[Math.max(0, at - 1)] ?? null;
  return null;
}

/** How long a notice stays, ms: the Apply notice "n fixtures lie outside the work area." 5 s, a clamp 3 s (section 10). */
export function noticeMs(text: string): number {
  return text.includes("outside the work area") ? 5000 : 3000;
}

/** A command's notice and the id of the field whose commit gave it, `null` when no field did. */
export interface FieldNotice {
  text: string;
  field: string | null;
}

/** The text of `notice` under the field `field`: a clamp's Action notice (section 10). */
export function noticeUnder(notice: FieldNotice | null, field: string): string | null {
  return notice !== null && notice.field === field ? notice.text : null;
}

/** The text of `notice` at the tab's foot: a notice no field gave. */
export function noticeInTab(notice: FieldNotice | null): string | null {
  return notice !== null && notice.field === null ? notice.text : null;
}

/** The tooltip of a Lock slot (`specs/0113-fixture-sets-and-pins` `ux-notes.md` 4.5, 7): "Locked
 * with <set>" while the fixture's set is locked, else "Unlock" or "Lock". */
export function lockTooltip(locked: boolean, lockedWith: string | null): string {
  if (lockedWith !== null) return `Locked with ${lockedWith}`;
  return locked ? "Unlock" : "Lock";
}
