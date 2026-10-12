/** How a Pencil pointer event becomes the flat sample batch of one wasm call
 * (`specs/0032-pen-tablet-input` criteria 1 and 2) and the text of the pen readout
 * (criterion 15). No React, so a test can drive it. No pointer id or device name is read. */

/** Values per sample: x, y (canvas CSS pixels), pressure, tiltX, tiltY, time (ms). */
export const SAMPLE_VALUES = 6;

/** The fields of a DOM pointer event the Pencil reads. */
export interface PencilPointerEvent {
  clientX: number;
  clientY: number;
  pressure: number;
  tiltX?: number;
  tiltY?: number;
  timeStamp: number;
  getCoalescedEvents?: () => PencilPointerEvent[];
}

/** The samples of `event` relative to the canvas at `origin`: for a move every coalesced event
 * when the webview provides them, else the event alone. */
export function pencilSampleValues(
  event: PencilPointerEvent,
  origin: { left: number; top: number },
  move: boolean,
): Float64Array {
  const coalesced = move && event.getCoalescedEvents ? event.getCoalescedEvents() : [];
  const readings = coalesced.length > 0 ? coalesced : [event];
  const values = new Float64Array(readings.length * SAMPLE_VALUES);
  readings.forEach((reading, index) => {
    values.set(
      [
        reading.clientX - origin.left,
        reading.clientY - origin.top,
        reading.pressure,
        reading.tiltX ?? 0,
        reading.tiltY ?? 0,
        reading.timeStamp,
      ],
      index * SAMPLE_VALUES,
    );
  });
  return values;
}

/** Whole degrees with a true minus sign. */
function degrees(value: number): string {
  const rounded = Math.round(value);
  return `${rounded < 0 ? "−" : ""}${Math.abs(rounded)}°`;
}

/** Exactly tiltX 90° and tiltY 0°: what synthetic events and some Linux WebKitGTK builds report
 * for a pen without tilt (a pen lying flat along X, which no hand holds). */
function noTilt(tiltX: number, tiltY: number): boolean {
  // The values pass through radians in the editor core, so "exactly" allows its rounding.
  return Math.abs(tiltX - 90) < 1e-6 && Math.abs(tiltY) < 1e-6;
}

/** "Pen: pressure 0.62, tilt 12° / −3°", or "tilt –" when the webview reports no tilt. */
export function penReadoutText(pressure: number, tiltX: number, tiltY: number): string {
  const tilt = noTilt(tiltX, tiltY) ? "–" : `${degrees(tiltX)} / ${degrees(tiltY)}`;
  return `Pen: pressure ${pressure.toFixed(2)}, tilt ${tilt}`;
}
