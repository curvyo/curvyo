/** The words of sketch pages (`specs/0099-sketch-pages` criteria 9, 10, 12, 14, 16; UX notes
 * §1 to §4, §7). Every rule is Rust's; this only words what the session reports. */

/** The sketch state of a sketch page's row, as Rust sends it. */
export type SketchState =
  | { state: "empty" | "full" | "conflict" | "failed" }
  | { state: "free"; dof: number };

/** The sketch facts of a sketch page's row. */
export interface SketchRow {
  state: SketchState;
  /** Every clone whose chain's root is the sketch, pasteboard ones included. */
  clones: number;
  /** The pages those clones lie on. */
  pages: number;
  /** Whether the sketch has something to draw (a clone, a ghost, a conversion). */
  drawable: boolean;
}

/** The short state word of line 2: "Fully constrained", "3 DOF". */
export function stateWord(s: SketchState): string {
  switch (s.state) {
    case "empty":
      return "Empty";
    case "full":
      return "Fully constrained";
    case "free":
      return `${s.dof} DOF`;
    case "conflict":
      return "Conflict";
    case "failed":
      return "Solver failed";
  }
}

/** The spoken state: "fully constrained", "3 degrees of freedom". */
export function stateSpoken(s: SketchState): string {
  if (s.state === "free") return s.dof === 1 ? "1 degree of freedom" : `${s.dof} degrees of freedom`;
  return stateWord(s).toLowerCase();
}

/** The Lucide glyph of a state. */
export function stateGlyph(s: SketchState): "CircleDashed" | "CircleCheck" | "Move" | "TriangleAlert" | "CircleX" {
  switch (s.state) {
    case "empty":
      return "CircleDashed";
    case "full":
      return "CircleCheck";
    case "free":
      return "Move";
    case "conflict":
      return "TriangleAlert";
    case "failed":
      return "CircleX";
  }
}

/** "Conflict" and "Solver failed" are drawn at full opacity, the only emphasis. */
export function stateEmphasised(s: SketchState): boolean {
  return s.state === "conflict" || s.state === "failed";
}

/** The accessible name of a sketch page's name button: "Bracket, Sketch page, fully constrained". */
export function sketchPageReading(name: string, s: SketchState): string {
  return `${name}, Sketch page, ${stateSpoken(s)}`;
}

/** The trash button's tooltip and name (criterion 16). */
export function deleteLabel(name: string, sketch: SketchRow | null): string {
  if (!sketch) return `Delete page: ${name}`;
  if (sketch.clones === 0) return `Delete ${name} and its sketch`;
  if (sketch.clones === 1) return `Delete ${name} and its sketch; 1 clone becomes a path`;
  return `Delete ${name} and its sketch; ${sketch.clones} clones become paths`;
}

/** The tail of line 2's tooltip: " · used by 3 clones", empty without clones. */
export function usedByTail(clones: number): string {
  if (clones === 0) return "";
  return clones === 1 ? " · used by 1 clone" : ` · used by ${clones} clones`;
}

/** The usage line of the Use group (criterion 12). */
export function usageLine(clones: number, pages: number): string {
  if (clones === 0) return "Not used on other pages";
  const c = clones === 1 ? "1 linked clone" : `${clones} linked clones`;
  const p = pages === 1 ? "1 page" : `${pages} pages`;
  return pages === 0 ? `Used by ${c}` : `Used by ${c} on ${p}`;
}

/** The label of Select clones: "Select clone" for one. */
export function selectClonesLabel(clones: number): string {
  return clones === 1 ? "Select clone" : "Select clones";
}

/** What the drop badge says, from the session's badge kind (UX notes §2). */
export type DropBadgeKind = "none" | "clone" | "not_on_sketch_page" | "no_sketch";

/** The badge word; the page name truncates at 24 characters. `null`: no badge. */
export function badgeWord(kind: DropBadgeKind, pageName: string): string | null {
  switch (kind) {
    case "none":
      return null;
    case "clone":
      return "Linked clone";
    case "not_on_sketch_page":
      return "Not on a sketch page";
    case "no_sketch": {
      const name = pageName.length > 24 ? `${pageName.slice(0, 23)}…` : pageName;
      return `${name} has no sketch yet`;
    }
  }
}
