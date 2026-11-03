// The thumbnail a save writes into the container (`specs/0094-workspace` criterion 6): the
// active page drawn by the canvas renderer into one offscreen canvas, encoded as PNG. The host
// puts the PNG into the container as `thumbnail.png`.
import { WasmThumbnail, type WasmSession } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** Physical size of the thumbnail: the 40 x 28 px row thumbnail (ux-notes section 2) at 4x, so it
 * stays sharp at 2x and in a larger view later. */
const WIDTH = 160;
const HEIGHT = 112;
const RATIO = 2;

interface Renderer {
  thumbnail: WasmThumbnail;
  canvas: HTMLCanvasElement;
}

let renderer: Promise<Renderer | null> | null = null;

/** The one renderer of the page, made on the first save; `null` where no GPU context exists. */
function pageRenderer(): Promise<Renderer | null> {
  renderer ??= (async () => {
    const canvas = document.createElement("canvas");
    canvas.width = WIDTH;
    canvas.height = HEIGHT;
    try {
      return { thumbnail: await WasmThumbnail.attach(canvas, WIDTH, HEIGHT, RATIO), canvas };
    } catch {
      return null;
    }
  })();
  return renderer;
}

/** The PNG of the active page of the session `current()` returns at drawing time, or `null` when
 * it cannot be drawn; a save then goes on without a thumbnail. */
export async function renderPageThumbnail(current: () => WasmSession | null): Promise<Uint8Array | null> {
  const made = await pageRenderer();
  const session = current();
  if (!made || !session) {
    return null;
  }
  try {
    session.render_thumbnail(made.thumbnail);
  } catch {
    return null;
  }
  // Read in the same task as the draw: a WebGL canvas keeps its picture only until it is shown.
  const blob = await new Promise<Blob | null>((resolve) => made.canvas.toBlob(resolve, "image/png"));
  return blob ? new Uint8Array(await blob.arrayBuffer()) : null;
}
