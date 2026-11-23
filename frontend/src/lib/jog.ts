// The jog pad of the Machine controls footer (`specs/0118-machine-control` slice 2a, criteria 15 to
// 20; UX notes W16 to W24): which arrow drives which machine axis for each origin corner, the
// steps, and the press that is a step when released within 300 ms and a hold past it. Pure, so
// the tests load it without a bundler.

/** The profile's origin corner (`specs/0029` criterion 26). */
export type Corner = "top_left" | "top_right" | "bottom_left" | "bottom_right";

/** A direction on screen. */
export type Arrow = "up" | "down" | "left" | "right";

/** A jog request as the host takes it: the machine axis, the way and the step. */
export interface JogRequest {
  axis: "X" | "Y";
  direction: "Plus" | "Minus";
  step: JogStep;
}

/** The distance of one press in millimetres: a closed set, never a typed number (Frage 315 A). */
export const STEPS = ["0.1", "1", "10", "50"] as const;
export type JogStep = (typeof STEPS)[number];
export const DEFAULT_STEP: JogStep = "1";

/** How long a press may last and still be one step (criterion 16). */
export const HOLD_AFTER_MS = 300;
/** The keep-alive of a held press: at least every 100 ms (criterion 18). */
export const KEEP_EVERY_MS = 100;

/** The machine axis and way an arrow drives, with the name the button carries ("X+"). */
export interface ArrowAxis {
  axis: "X" | "Y";
  direction: "Plus" | "Minus";
  name: string;
}

const plus = (axis: "X" | "Y"): ArrowAxis => ({ axis, direction: "Plus", name: `${axis}+` });
const minus = (axis: "X" | "Y"): ArrowAxis => ({ axis, direction: "Minus", name: `${axis}-` });

/** The arrows per origin corner (UX notes W20): a glyph is the direction on screen, so the head
 * moves the way the canvas shows the bed; the name is the machine axis it drives. */
export const ARROWS: Record<Corner, Record<Arrow, ArrowAxis>> = {
  bottom_left: { up: plus("Y"), down: minus("Y"), left: minus("X"), right: plus("X") },
  top_left: { up: minus("Y"), down: plus("Y"), left: minus("X"), right: plus("X") },
  bottom_right: { up: plus("Y"), down: minus("Y"), left: plus("X"), right: minus("X") },
  top_right: { up: minus("Y"), down: plus("Y"), left: plus("X"), right: minus("X") },
};

/** The request of a press on `arrow`. */
export function requestOf(corner: Corner, arrow: Arrow, step: JogStep): JogRequest {
  const { axis, direction } = ARROWS[corner][arrow];
  return { axis, direction, step };
}

/** The arrow whose button takes the focus after `key`, inside the pad (UX notes W24); the arrow
 * keys move the focus and never the head. */
export function focusAfter(from: Arrow, key: string): Arrow | null {
  const to: Record<string, Arrow> = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" };
  const target = to[key];
  return target === undefined || target === from ? null : target;
}

/** One arrow press from pointer or key down to its end. */
export type PressState = { phase: "idle" } | { phase: "down"; since: number } | { phase: "held" };

export type PressEvent =
  | { kind: "down"; at: number }
  /** The 300 ms timer or a 100 ms interval fired. */
  | { kind: "tick"; at: number }
  /** Up, the pointer left the button, a cancel, a blur, Escape, the pad closing: any end (W21). */
  | { kind: "end" };

/** What a press asks the host for. */
export type PressEffect = "step" | "hold" | "keep" | "release" | null;

export const IDLE: PressState = { phase: "idle" };

/** The press reducer (criteria 16, 18): released before 300 ms it is one step; still down at 300 ms
 * it starts a hold, kept alive on every tick, and its end is the release. */
export function press(state: PressState, event: PressEvent): [PressState, PressEffect] {
  switch (state.phase) {
    case "idle":
      return event.kind === "down" ? [{ phase: "down", since: event.at }, null] : [state, null];
    case "down":
      if (event.kind === "end") return [IDLE, "step"];
      if (event.kind === "tick" && event.at - state.since >= HOLD_AFTER_MS) return [{ phase: "held" }, "hold"];
      return [state, null];
    case "held":
      if (event.kind === "end") return [IDLE, "release"];
      if (event.kind === "tick") return [state, "keep"];
      return [state, null];
  }
}

/** The pad's Space and Enter keys (UX notes W24): Space can hold, Enter is one step on key down
 * and never holds, key repeat is ignored. */
export function keyPress(key: string, repeat: boolean): "space" | "enter" | null {
  if (repeat) return null;
  if (key === " " || key === "Spacebar") return "space";
  return key === "Enter" ? "enter" : null;
}

/** What the pad remembers on this computer: open or closed and the step (UX notes W16); not in the
 * project, not synced. */
export interface JogView {
  open: boolean;
  step: JogStep;
}

const VIEW_KEY = "curvyo.jogPad";

/** The remembered view; closed at 1 mm when nothing (readable) is stored. */
export function readJogView(storage: Pick<Storage, "getItem"> | null): JogView {
  try {
    const raw = storage?.getItem(VIEW_KEY);
    if (raw) {
      const value = JSON.parse(raw) as { open?: unknown; step?: unknown };
      const step = STEPS.find((candidate) => candidate === value.step) ?? DEFAULT_STEP;
      return { open: value.open === true, step };
    }
  } catch {
    // Private windows and cleared site data: the pad starts closed.
  }
  return { open: false, step: DEFAULT_STEP };
}

/** Remembers the view; a storage that refuses is no failure. */
export function writeJogView(storage: Pick<Storage, "setItem"> | null, view: JogView): void {
  try {
    storage?.setItem(VIEW_KEY, JSON.stringify(view));
  } catch {
    // Not remembered.
  }
}
