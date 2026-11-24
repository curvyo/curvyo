// The page pictures a save writes into the container (`specs/0137-document-library` criterion 2,
// ADR 0084 §1): each of the first 32 pages drawn by the canvas renderer at 224 x 160 px (the
// 56 x 40 library row picture at 4x) and joined as `pages/<id>.png` by the wasm, in both builds.
// In the browser the active page's `thumbnail.png` is joined in the same pass (decision 10).
import {
  WasmThumbnail,
  library_with_pictures,
  type WasmSession,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

const WIDTH = 224;
const HEIGHT = 160;
const RATIO = 2;

interface Renderer {
  thumbnail: WasmThumbnail;
  canvas: HTMLCanvasElement;
}

let renderer: Promise<Renderer | null> | null = null;

function pictureRenderer(): Promise<Renderer | null> {
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

async function png(canvas: HTMLCanvasElement): Promise<Uint8Array | null> {
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  return blob ? new Uint8Array(await blob.arrayBuffer()) : null;
}

/** `bytes` with the page pictures of the session `current()` returns (and `thumbnail` when given)
 * joined in; the bytes as they are when nothing can be drawn. A page that fails has no picture. */
export async function withPagePictures(
  bytes: Uint8Array,
  current: () => WasmSession | null,
  thumbnail: Uint8Array | null,
): Promise<Uint8Array> {
  const made = await pictureRenderer();
  const session = current();
  if (!made || !session) {
    return bytes;
  }
  let ids: string[];
  try {
    ids = JSON.parse(session.picture_page_ids()) as string[];
  } catch {
    return bytes;
  }
  const done: string[] = [];
  const pictures: Uint8Array[] = [];
  for (const id of ids) {
    const live = current();
    if (!live) {
      break;
    }
    try {
      if (!live.render_page_picture(made.thumbnail, id)) {
        continue;
      }
    } catch {
      continue;
    }
    // Read in the same task as the draw: a WebGL canvas keeps its picture only until it is shown.
    const picture = await png(made.canvas);
    if (picture) {
      done.push(id);
      pictures.push(picture);
    }
  }
  if (done.length === 0 && !thumbnail) {
    return bytes;
  }
  const total = pictures.reduce((sum, p) => sum + p.length, 0);
  const joined = new Uint8Array(total);
  let at = 0;
  for (const picture of pictures) {
    joined.set(picture, at);
    at += picture.length;
  }
  const lengths = Uint32Array.from(pictures.map((p) => p.length));
  return library_with_pictures(bytes, thumbnail ?? new Uint8Array(), JSON.stringify(done), joined, lengths);
}
