// The one adapter from pen input to the session's flat `PenSample` records (`specs/0158-pen-display`
// slice 1, ADR 0094 decision 5, ADR 0095 decision 2): Pointer Events with their coalesced events
// (Windows, macOS, the browser build, and Linux before the first native sample of a stroke) and the
// native `PenBatch`es of `pen_watch` on Linux. One flush per animation frame; touch never draws.
//
// Pure and DOM-free: the caller passes events and the canvas rectangle, so the tests feed
// synthetic events.

import type { NativeBatch, NativeSample } from "../lib/penProbe.ts";

/** Numbers per record; the session's `pen_sample_stride()` must say the same. */
export const PEN_SAMPLE_STRIDE = 9;

const END_TIP = 0;
const END_ERASER = 1;
const PHASE_HOVER = 0;
const PHASE_CONTACT = 1;
const PHASE_OUT = 2;
/** `PointerEvent.buttons` bit of the primary button and of a pen's eraser end. */
const BUTTON_PRIMARY = 1;
const BUTTON_BARREL = 2;
const BUTTON_ERASER = 32;
/** Frames the feed waits after a release for the native batch that ends the stroke. */
const NATIVE_END_WAIT_FRAMES = 1;

/** The fields of a Pointer Event the feed reads. */
export interface PointerLike {
  type: string;
  pointerType: string;
  clientX: number;
  clientY: number;
  pressure: number;
  tiltX: number;
  tiltY: number;
  buttons: number;
  timeStamp: number;
  getCoalescedEvents?: () => PointerLike[];
}

/** Where the canvas is in the viewport (its `getBoundingClientRect()`). */
export interface CanvasRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** What one frame's flush carries. */
export interface PenFlush {
  /** The records, `PEN_SAMPLE_STRIDE` numbers each. */
  flat: Float64Array;
  /** Whether a contact ended in this flush (a stroke or an erase was released). */
  ended: boolean;
  /** Whether the contact that ended came from a pen (not a mouse). */
  penEnded: boolean;
}

interface Last {
  x: number;
  y: number;
  time: number;
  end: number;
}

/** Collects one frame's samples from both sources and hands them over as one flat batch. */
export class PenSampleFeed {
  private queue: number[] = [];
  private contact = false;
  private contactIsPen = false;
  private ended = false;
  private penEnded = false;
  /** A press on the canvas is down (native contact samples count only then). */
  private pressed = false;
  /** Native contact samples are carrying the current contact: Pointer Events add no geometry. */
  private nativeContact = false;
  /** Frames left to wait for the native end after a release, or -1. */
  private endWait = -1;
  private last: Last | null = null;
  /** Whether a pen (pointer type pen, or any native sample) was seen. */
  penSeen = false;

  /** A Pointer Event on the canvas (`pointerdown`, `pointermove`, `pointerup`, `pointercancel`). */
  pointer(event: PointerLike, rect: CanvasRect): void {
    if (event.pointerType === "touch") {
      return;
    }
    const pen = event.pointerType === "pen";
    this.penSeen ||= pen;
    if (event.type === "pointerdown") {
      this.pressed = true;
      this.endWait = -1;
    }
    const release = event.type === "pointerup" || event.type === "pointercancel";
    if (release) {
      this.pressed = false;
      if (this.nativeContact) {
        // The last native batch may still be on its way: it ends the stroke, or the wait does.
        this.endWait = NATIVE_END_WAIT_FRAMES;
        return;
      }
    }
    if (this.nativeContact) {
      return;
    }
    const events =
      event.type === "pointermove" ? (event.getCoalescedEvents?.() ?? []) : [];
    for (const one of events.length > 0 ? events : [event]) {
      this.pushPointer(one, event.type, pen, rect);
    }
  }

  private pushPointer(event: PointerLike, type: string, pen: boolean, rect: CanvasRect): void {
    let phase: number;
    if (type === "pointercancel") {
      phase = PHASE_OUT;
    } else if (type === "pointerup") {
      phase = PHASE_HOVER;
    } else if (type === "pointerdown") {
      phase = PHASE_CONTACT;
    } else {
      phase = (event.buttons & (BUTTON_PRIMARY | BUTTON_ERASER)) !== 0 ? PHASE_CONTACT : PHASE_HOVER;
    }
    const eraser = pen && (event.buttons & BUTTON_ERASER) !== 0;
    const end = phase === PHASE_CONTACT ? (eraser ? END_ERASER : END_TIP) : (this.last?.end ?? END_TIP);
    this.push(
      [
        event.clientX - rect.left,
        event.clientY - rect.top,
        pen ? event.pressure : Number.NaN,
        pen ? event.tiltX / 90 : Number.NaN,
        pen ? event.tiltY / 90 : Number.NaN,
        end,
        phase,
        pen && (event.buttons & BUTTON_BARREL) !== 0 ? 1 : 0,
        event.timeStamp,
      ],
      pen,
    );
  }

  /** A native batch (Linux, `pen_watch`): its samples carry the contact while the canvas is
   * pressed; hover samples count only over the canvas. */
  native(batch: NativeBatch, rect: CanvasRect): void {
    for (const sample of batch.samples) {
      this.penSeen = true;
      if (sample.phase === "contact") {
        if (!this.pressed && !this.nativeContact) {
          continue;
        }
        this.nativeContact = true;
      } else {
        const inside =
          sample.x >= rect.left &&
          sample.x <= rect.left + rect.width &&
          sample.y >= rect.top &&
          sample.y <= rect.top + rect.height;
        const ending = this.nativeContact;
        this.nativeContact = false;
        if (!ending && !inside) {
          continue;
        }
        this.endWait = -1;
      }
      this.pushNative(sample, rect);
    }
  }

  private pushNative(sample: NativeSample, rect: CanvasRect): void {
    const phase =
      sample.phase === "contact" ? PHASE_CONTACT : sample.phase === "hover" ? PHASE_HOVER : PHASE_OUT;
    this.push(
      [
        sample.x - rect.left,
        sample.y - rect.top,
        sample.pressure ?? Number.NaN,
        sample.tilt ? sample.tilt.x : Number.NaN,
        sample.tilt ? sample.tilt.y : Number.NaN,
        sample.end === "eraser" ? END_ERASER : END_TIP,
        phase,
        sample.barrel,
        sample.time_ms,
      ],
      true,
    );
  }

  private push(record: number[], pen: boolean): void {
    const contact = record[6] === PHASE_CONTACT;
    if (contact && !this.contact) {
      this.contactIsPen = pen;
    }
    if (!contact && this.contact) {
      this.ended = true;
      this.penEnded ||= this.contactIsPen;
    }
    this.contact = contact;
    this.last = { x: record[0], y: record[1], time: record[8], end: record[5] };
    this.queue.push(...record);
  }

  /** The frame's batch, or `null` when nothing arrived. A release whose native end did not come
   * within the wait is ended here at the last position. */
  take(): PenFlush | null {
    if (this.endWait === 0 && this.nativeContact && this.last) {
      this.nativeContact = false;
      const { x, y, time, end } = this.last;
      this.push([x, y, Number.NaN, Number.NaN, Number.NaN, end, PHASE_OUT, 0, time], true);
    }
    if (this.endWait >= 0) {
      this.endWait -= 1;
    }
    if (this.queue.length === 0) {
      return null;
    }
    const flush: PenFlush = {
      flat: Float64Array.from(this.queue),
      ended: this.ended,
      penEnded: this.penEnded,
    };
    this.queue = [];
    this.ended = false;
    this.penEnded = false;
    return flush;
  }

  /** Forgets a contact in progress (a tool switch). */
  reset(): void {
    this.queue = [];
    this.contact = false;
    this.pressed = false;
    this.nativeContact = false;
    this.endWait = -1;
    this.ended = false;
    this.penEnded = false;
  }
}
