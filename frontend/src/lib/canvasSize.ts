// The canvas's backing-buffer size, shared by the editor session hook and its test.

/** A canvas's backing buffer: physical pixel size and the ratio to CSS pixels. */
export interface BackingBuffer {
  width: number;
  height: number;
  devicePixelRatio: number;
}

/** The canvas's backing-buffer (physical pixel) size for a given CSS
 * (layout) size, plus the `devicePixelRatio` that relates the two —
 * `curvyo-editor-wasm`'s `Gpu` needs both: the buffer itself sized to
 * the display's actual resolution (correctness fix: a HiDPI display
 * previously got a buffer sized 1:1 to CSS pixels, then upscaled and
 * softened by the browser compositor — the same class of bug as this
 * slice's `wgpu` surface-resize requirement, but for resolution rather
 * than staleness), and the ratio itself so `Gpu::render` can map back to
 * CSS pixels for the clip-space transform. `devicePixelRatio` can be a
 * fractional, non-integer value (Windows/Linux fractional scaling, e.g.
 * 1.25 or 1.5), so the buffer size is rounded, not assumed to divide
 * evenly. */
export function backingBufferFor(
  cssWidth: number,
  cssHeight: number,
  rawDevicePixelRatio: number,
): BackingBuffer {
  const devicePixelRatio = rawDevicePixelRatio > 0 ? rawDevicePixelRatio : 1;
  return {
    width: Math.max(1, Math.round(cssWidth * devicePixelRatio)),
    height: Math.max(1, Math.round(cssHeight * devicePixelRatio)),
    devicePixelRatio,
  };
}

/** Whether two measurements of the canvas give the same buffer, so the session
 * already has the canvas's real size. */
export function sameBackingBuffer(a: BackingBuffer, b: BackingBuffer): boolean {
  return (
    a.width === b.width && a.height === b.height && a.devicePixelRatio === b.devicePixelRatio
  );
}
