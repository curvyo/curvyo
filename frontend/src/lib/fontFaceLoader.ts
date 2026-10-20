// The faces the Fonts tab sets its rows in (`specs/0074-text-objects` UX notes §4): each listed
// face's bytes loaded as a CSS font face once its row was in view, at most 64, the least
// recently shown one dropped first.
import type { FontFace as ListedFace } from "@/lib/textText";
import { invoke } from "@/platform/host";

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
  if (loadedFaces.has(face.id) || loadingFaces.has(face.id)) {
    return;
  }
  loadingFaces.add(face.id);
  faceCounter += 1;
  const name = `curvyo-face-${faceCounter}`;
  const source: Promise<ArrayBuffer> =
    face.block === "included"
      ? fetch(`${import.meta.env.BASE_URL}fonts/${face.id}`).then((r) => r.arrayBuffer())
      : invoke<ArrayBuffer>("font_bytes", { id: face.id });
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
