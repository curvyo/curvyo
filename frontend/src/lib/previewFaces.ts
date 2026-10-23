// The faces the font preview draws its tiles in (`specs/0077-font-preview` criteria 15, 17 and 18,
// `adrs.md` decisions 2 and 3): each face's bytes handed to the engine once, its tiles drawn from
// the engine's outlines and kept per sample (whatever the Size and width) until they are asked
// for again, and the listed families prepared ahead while the page is idle.
import { readFaceBytes } from "@/lib/fontFaceLoader";
import { TILE_DRAW_CAPS } from "@/lib/fontPreviewLayout";
import { createPrerenderer, prerenderOrder } from "@/lib/fontPreviewPrerender";
import type { FontFace as ListedFace } from "@/lib/textText";
import { wasmReady } from "@/lib/editorSession";
import { WasmPreviewFaces } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** A tile's sample as the engine draws it: path data and its box in em units (y down, origin on
 * the baseline), the face's cap height in em, the characters it has no glyph for, and
 * whether it is a single-line face. */
export interface TileDrawing {
  d: string;
  box: [number, number, number, number];
  cap: number;
  missing: string[];
  single_line: boolean;
}

let faces: WasmPreviewFaces | null = null;
/** Faces whose bytes held no usable face: their tiles keep the placeholder. */
const unreadable = new Set<string>();

/** The engine's id of a listed face: its file and its index in the file. */
function faceId(face: ListedFace): string {
  return `${face.id}#${face.faceIndex}`;
}

/** Whether the face's tiles can be drawn now. */
function isPreviewFaceLoaded(face: ListedFace): boolean {
  return faces?.has_preview_face(faceId(face)) ?? false;
}

/** Faces whose bytes are on the way, by engine id: a tile and the background preparation asking
 * at once share one read. */
const loading = new Map<string, Promise<boolean>>();

/** Hands the face's bytes to the engine once; resolves to whether its tiles can be drawn. */
function readPreviewFace(face: ListedFace): Promise<boolean> {
  const id = faceId(face);
  if (isPreviewFaceLoaded(face)) {
    return Promise.resolve(true);
  }
  if (unreadable.has(id)) {
    return Promise.resolve(false);
  }
  const pending = loading.get(id);
  if (pending) {
    return pending;
  }
  const read = Promise.all([wasmReady(), readFaceBytes(face)])
    .then(([, buffer]) => {
      faces ??= new WasmPreviewFaces();
      const added = faces.add_preview_face(id, new Uint8Array(buffer), face.faceIndex);
      if (!added) {
        unreadable.add(id);
      }
      return added;
    })
    .catch(() => false)
    .finally(() => loading.delete(id));
  loading.set(id, read);
  return read;
}

/** Waits for an idle moment, at most 200 ms: the editor's render loop leaves few. */
const prerenderer = createPrerenderer((run) =>
  window.requestIdleCallback ? window.requestIdleCallback(run, { timeout: 200 }) : window.setTimeout(run, 1),
);

/** Prepares the tiles of `listed`: the faces in view (`visible`, indices) are read at once, in
 * parallel, and `ready` runs after each that became drawable; then the next families
 * (`prerenderOrder`, from the top when nothing is in view) one at a time while the page is idle,
 * their bytes handed to the engine and the tile of `text(face)` drawn into the cache. A new call
 * replaces the families not prepared yet. */
export function prerenderTiles(
  listed: ListedFace[],
  text: (face: ListedFace) => string,
  visible: { first: number; last: number } | null,
  ready: () => void,
): void {
  const order = prerenderOrder(listed.length, visible?.first ?? 0, visible?.last ?? -1);
  const inView = visible ? Math.max(0, visible.last - visible.first + 1) : 0;
  for (const index of order.slice(0, inView)) {
    const face = listed[index];
    if (!isPreviewFaceLoaded(face)) {
      void readPreviewFace(face).then((added) => {
        if (added) {
          ready();
        }
      });
    }
  }
  prerenderer.schedule(
    order.slice(inView).map((index) => async () => {
      const face = listed[index];
      if (await readPreviewFace(face)) {
        previewTile(face, text(face));
      }
    }),
  );
}

/** Drawn tiles by face and sample, the least recently asked for dropped first. */
const drawn = new Map<string, TileDrawing | null>();
const MAX_DRAWN = 600;

/** `text` in the face as one line, drawn up to `TILE_DRAW_CAPS` cap heights (the tile scales it
 * to the Size and clips it at its edge), so one drawing per face and sample serves every Size and
 * pane width; `null` while the face is not loaded. */
export function previewTile(face: ListedFace, text: string): TileDrawing | null {
  const id = faceId(face);
  if (!faces || !faces.has_preview_face(id)) {
    return null;
  }
  const key = `${id}\u0000${text}`;
  const hit = drawn.get(key);
  if (hit !== undefined) {
    drawn.delete(key);
    drawn.set(key, hit);
    return hit;
  }
  const json = faces.preview_tile(id, text, TILE_DRAW_CAPS);
  const tile = json ? (JSON.parse(json) as TileDrawing) : null;
  drawn.set(key, tile);
  while (drawn.size > MAX_DRAWN) {
    drawn.delete(drawn.keys().next().value as string);
  }
  return tile;
}
