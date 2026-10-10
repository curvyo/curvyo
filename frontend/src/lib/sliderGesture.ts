/**
 * The pointer gestures of a slider (`specs/0047-transform-polish/` Part B, criteria 17
 * to 24 and 27): a click sets the value at the pointer, a click on the thumb does
 * nothing, a drag moves the thumb, two clicks in a row are a double-click. Pure: no DOM,
 * the caller passes positions in the slider's own pixels and the clock.
 */

export interface Point {
  x: number;
  y: number;
}

/** A press and release that move less than this are a click (criterion 17). */
export const CLICK_SLOP_PX = 3;
/** The same for a touch or a pen contact. */
export const TOUCH_SLOP_PX = 8;
/** Two clicks within this many milliseconds are a double-click. The browser reports no
 * platform interval for a pointer, so this is the usual 500 ms of Windows and macOS. */
export const DOUBLE_CLICK_MS = 500;
/** The second click of a double-click lies within this of the first. */
export const DOUBLE_CLICK_SLOP_PX = 4;
/** The same for a double-tap. */
export const DOUBLE_TAP_SLOP_PX = 24;

/** Where the thumb is drawn, in the slider's pixels: its centre and radius. */
export interface Thumb {
  at: Point;
  radius: number;
}

/** What a pointer event did to the gesture. */
export type GestureEvent =
  /** Nothing yet: still inside the click slop, or no press. */
  | { kind: "none" }
  /** The press became a drag. `grab` is the thumb's centre minus the pointer at the press
   * when the press was on the thumb (the thumb keeps its offset to the pointer), else
   * `null` (the thumb goes under the pointer). */
  | { kind: "drag-start"; grab: Point | null }
  /** Another step of a drag. */
  | { kind: "drag" }
  /** The release of a drag. */
  | { kind: "drag-end" }
  /** A click. `onThumb`: it was on the thumb, so the value stays. */
  | { kind: "click"; onThumb: boolean }
  /** The second click of a double-click. The value is the one the first click set. */
  | { kind: "double-click" };

const distance = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

interface Press {
  at: Point;
  touch: boolean;
  grab: Point | null;
  onThumb: boolean;
  dragging: boolean;
}

interface Click {
  at: Point;
  time: number;
}

/**
 * One slider's gesture state. `down`, `move` and `up` take positions in the slider's
 * pixels. Nothing changes the slider's value at the press: a click acts at the release
 * and a drag after the slop, so the second press of a double-click cannot move the
 * thumb (criterion 24).
 */
export class SliderGesture {
  private press: Press | null = null;

  private lastClick: Click | null = null;

  /** A press of the primary button. `thumb` is the thumb now, or `null` when there is none
   * (mixed). */
  down(at: Point, touch: boolean, thumb: Thumb | null): void {
    const onThumb = thumb !== null && distance(at, thumb.at) <= thumb.radius;
    this.press = {
      at,
      touch,
      grab:
        onThumb && thumb
          ? { x: thumb.at.x - at.x, y: thumb.at.y - at.y }
          : null,
      onThumb,
      dragging: false,
    };
  }

  /** The pointer moved with the button down. */
  move(at: Point): GestureEvent {
    const press = this.press;
    if (!press) {
      return { kind: "none" };
    }
    if (press.dragging) {
      return { kind: "drag" };
    }
    const slop = press.touch ? TOUCH_SLOP_PX : CLICK_SLOP_PX;
    if (distance(at, press.at) < slop) {
      return { kind: "none" };
    }
    press.dragging = true;
    this.lastClick = null;
    return { kind: "drag-start", grab: press.grab };
  }

  /** The release. `time` is the event's time stamp in milliseconds. */
  up(at: Point, time: number): GestureEvent {
    const press = this.press;
    this.press = null;
    if (!press) {
      return { kind: "none" };
    }
    if (press.dragging) {
      return { kind: "drag-end" };
    }
    const slop = press.touch ? DOUBLE_TAP_SLOP_PX : DOUBLE_CLICK_SLOP_PX;
    const last = this.lastClick;
    if (
      last &&
      time - last.time <= DOUBLE_CLICK_MS &&
      distance(at, last.at) <= slop
    ) {
      // A third click of a triple click starts over: it is a click like any other.
      this.lastClick = null;
      return { kind: "double-click" };
    }
    this.lastClick = { at, time };
    return { kind: "click", onThumb: press.onThumb };
  }

  /** The system took the pointer away: nothing is a click or a drag. */
  cancel(): void {
    this.press = null;
  }

  /** Whether a press is down and has become a drag. */
  get dragging(): boolean {
    return this.press?.dragging ?? false;
  }
}
