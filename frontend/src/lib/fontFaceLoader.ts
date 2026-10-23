// The faces the Fonts tab sets its rows in (`specs/0074-text-objects` UX notes §4): each listed
// face's bytes loaded as a CSS font face once its row was in view, at most 64, the least
// recently shown one dropped first. Browsers draw no SVG fonts, so a single-line face's names
// are drawn from the engine's centre lines instead (`specs/0076-single-line-fonts` criterion 6).
import type { FontFace as ListedFace } from "@/lib/textText";
import { invoke } from "@/platform/host";
import { face_preview_path } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** A name drawn in a face's centre lines: path data and its box, in em units, y down. */
export interface LinePreview {
  d: string;
  box: [number, number, number, number];
}

/** The family and style names of a loaded single-line face. */
const linePreviews = new Map<string, { family: LinePreview | null; style: LinePreview | null }>();

/** The drawn names of the single-line face `id`; `undefined` until it is loaded. */
export function linePreviewOf(id: string): { family: LinePreview | null; style: LinePreview | null } | undefined {
  return linePreviews.get(id);
}

function preview(bytes: Uint8Array, index: number, text: string): LinePreview | null {
  const json = face_preview_path(bytes, index, text);
  return json ? (JSON.parse(json) as LinePreview) : null;
}

/** The faces the tab holds at most: the least recently shown one goes first (UX notes §4, the
 * visible rows plus a margin). */
const MAX_LOADED_FACES = 64;

/** The CSS font family each listed face is set in, once its bytes are loaded, least recently
 * shown first. */
const loadedFaces = new Map<string, { name: string; face: FontFace }>();
const loadingFaces = new Set<string>();
let faceCounter = 0;

/** The CSS family of the face `id`, marked as shown just now; `undefined` until it is loaded. */
export function cssFamilyOf(id: string): string | undefined {
  const entry = loadedFaces.get(id);
  if (entry) {
    loadedFaces.delete(id);
    loadedFaces.set(id, entry);
  }
  return entry?.name;
}

export function loadFace(face: ListedFace, done: () => void): void {
  if (loadedFaces.has(face.id) || loadingFaces.has(face.id) || linePreviews.has(face.id)) {
    return;
  }
  loadingFaces.add(face.id);
  faceCounter += 1;
  const name = `curvyo-face-${faceCounter}`;
  const source: Promise<ArrayBuffer> =
    face.block === "included"
      ? fetch(`${import.meta.env.BASE_URL}fonts/${face.id}`).then((r) => r.arrayBuffer())
      : invoke<ArrayBuffer>("font_bytes", { id: face.id });
  if (face.singleLine) {
    void source
      .then((buffer) => {
        const bytes = new Uint8Array(buffer);
        linePreviews.set(face.id, {
          family: preview(bytes, face.faceIndex, face.family),
          style: preview(bytes, face.faceIndex, face.style),
        });
        while (linePreviews.size > MAX_LOADED_FACES) {
          const oldest = linePreviews.keys().next().value as string;
          linePreviews.delete(oldest);
        }
        done();
      })
      .catch(() => {})
      .finally(() => loadingFaces.delete(face.id));
    return;
  }
  void source
    .then((buffer) => new FontFace(name, buffer).load())
    .then((loaded) => {
      document.fonts.add(loaded);
      loadedFaces.set(face.id, { name, face: loaded });
      while (loadedFaces.size > MAX_LOADED_FACES) {
        const [oldest, entry] = loadedFaces.entries().next().value as [string, { face: FontFace }];
        document.fonts.delete(entry.face);
        loadedFaces.delete(oldest);
      }
      done();
    })
    .catch(() => {})
    .finally(() => loadingFaces.delete(face.id));
}
