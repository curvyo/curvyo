/**
 * The photo arithmetic of a test (`specs/0112-test-grid-and-materials` criterion 27, `ux-notes.md`
 * 8.5): the scale to 600 px on the long side, the encoding by file type, the crop frame and its
 * handles. Pure functions; the canvas work stays in `TestPhoto.tsx` and `PhotoCrop.tsx`.
 */

/** The long side a stored photo has at most. */
export const PHOTO_MAX_PX = 600;

/** The smallest side of a crop frame, px. */
export const CROP_MIN_PX = 16;

/** The size of an image of `width` x `height` scaled to at most 600 px on its long side (never up). */
export function scaledSize(width: number, height: number, max: number = PHOTO_MAX_PX): { width: number; height: number } {
  const longest = Math.max(width, height);
  if (longest <= max || longest <= 0) return { width, height };
  const scale = max / longest;
  return { width: Math.max(1, Math.round(width * scale)), height: Math.max(1, Math.round(height * scale)) };
}

/** The type a photo is encoded as: PNG stays PNG, everything else is JPEG at 0.85. */
export function encodeType(fileType: string): { mime: "image/png" | "image/jpeg"; quality: number | undefined } {
  return fileType === "image/png" ? { mime: "image/png", quality: undefined } : { mime: "image/jpeg", quality: 0.85 };
}

/** Whether a picked file is a JPEG or PNG. */
export function isPhotoFile(type: string): boolean {
  return type === "image/jpeg" || type === "image/png";
}

/** The text on the size readout of Crop: "600 × 470 px". */
export function sizeReadout(width: number, height: number): string {
  return `${Math.round(width)} × ${Math.round(height)} px`;
}

/** A frame inside an image, in image pixels. */
export interface Frame {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** The eight handles of a frame, clockwise from the top left. */
export type Handle = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";

/** The handles in the order Tab visits them. */
export const HANDLES: readonly Handle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];

/** The whole image as a frame. */
export function wholeFrame(width: number, height: number): Frame {
  return { x: 0, y: 0, width, height };
}

/** The frame moved by `dx`, `dy`, kept inside the `bounds` image. */
export function moveFrame(frame: Frame, dx: number, dy: number, bounds: { width: number; height: number }): Frame {
  const x = Math.min(Math.max(0, frame.x + dx), bounds.width - frame.width);
  const y = Math.min(Math.max(0, frame.y + dy), bounds.height - frame.height);
  return { ...frame, x, y };
}

/** The frame with `handle` moved by `dx`, `dy`: the sides it touches follow, the others stay;
 * no side goes outside the image or closer than 16 px to its opposite. */
export function resizeFrame(frame: Frame, handle: Handle, dx: number, dy: number, bounds: { width: number; height: number }): Frame {
  let left = frame.x;
  let top = frame.y;
  let right = frame.x + frame.width;
  let bottom = frame.y + frame.height;
  if (handle.includes("w")) left = Math.min(Math.max(0, left + dx), right - CROP_MIN_PX);
  if (handle.includes("e")) right = Math.max(Math.min(bounds.width, right + dx), left + CROP_MIN_PX);
  if (handle.includes("n")) top = Math.min(Math.max(0, top + dy), bottom - CROP_MIN_PX);
  if (handle.includes("s")) bottom = Math.max(Math.min(bounds.height, bottom + dy), top + CROP_MIN_PX);
  return { x: left, y: top, width: right - left, height: bottom - top };
}

/** The arrow keys' step on a frame or a handle: 1 px, 10 with Shift; `null` for another key. */
export function arrowStep(key: string, shift: boolean): { dx: number; dy: number } | null {
  const size = shift ? 10 : 1;
  switch (key) {
    case "ArrowLeft":
      return { dx: -size, dy: 0 };
    case "ArrowRight":
      return { dx: size, dy: 0 };
    case "ArrowUp":
      return { dx: 0, dy: -size };
    case "ArrowDown":
      return { dx: 0, dy: size };
    default:
      return null;
  }
}

/** The shown height of a 228 px wide photo of `width` x `height`: by its ratio, at most 228. */
export function shownHeight(width: number, height: number, box = 228): number {
  if (width <= 0 || height <= 0) return box;
  return Math.min(box, Math.round((box * height) / width));
}
