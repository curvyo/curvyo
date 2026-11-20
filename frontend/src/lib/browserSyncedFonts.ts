// The browser's reception of fonts that arrived from the maker's other devices
// (`specs/0116-library-sync` criteria 16 and 18, ADR 0062 decision 8): the file and its licence
// files are written into `curvyo-fonts` like Add font writes them, with the same SHA-256 per
// file; the host's `add_synced_fonts` command in the browser. Never uploaded anywhere.
import { STORES, type StoredFaces, allFaces, database, freeFontName, isQuota, sha256, splitKey } from "@/lib/browserFonts";
import { font_file_faces, synced_name_ok } from "@/wasm-bindings/curvyo_editor_wasm.js";

interface ArrivedFont {
  file_name: string;
  folder: string | null;
  bytes: Uint8Array | number[];
  documents: { name: string; bytes: Uint8Array | number[] }[];
}

const EXTENSIONS = [".ttf", ".otf", ".ttc", ".svg"];

function buffer(bytes: Uint8Array | number[]): ArrayBuffer {
  const array = bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
  return array.buffer.slice(array.byteOffset, array.byteOffset + array.byteLength) as ArrayBuffer;
}

function plain(name: string): boolean {
  return name !== "" && name !== "." && name !== ".." && !/[/\\\0]/.test(name) && synced_name_ok(name, false);
}

/** Writes each arrived font; one outcome per font: `"written"`, `"present"` (the same bytes are
 * stored already) or `"refused"` (not a plain name, not a font, or the storage is full). */
export async function addBrowserSyncedFonts(fonts: ArrivedFont[]): Promise<string[]> {
  const known = await allFaces();
  const outcomes: string[] = [];
  const db = await database();
  for (const font of fonts) {
    const lower = font.file_name.toLowerCase();
    const folderOk = font.folder === null || (plain(font.folder) && !font.folder.startsWith("."));
    if (!plain(font.file_name) || !folderOk || !EXTENSIONS.some((ext) => lower.endsWith(ext))) {
      outcomes.push("refused");
      continue;
    }
    const bytes = buffer(font.bytes);
    const faces = JSON.parse(font_file_faces(new Uint8Array(bytes))) as StoredFaces["faces"];
    const hash = await sha256(bytes);
    if (faces.length === 0) {
      outcomes.push("refused");
    } else if (known.some((other) => other.length === bytes.byteLength && other.sha256 === hash)) {
      outcomes.push("present");
    } else {
      const taken = known.map((f) => splitKey(f.name)).filter(([folder]) => folder === font.folder).map(([, file]) => file);
      const free = freeFontName(font.file_name, taken);
      const name = font.folder === null ? free : `${font.folder}/${free}`;
      const stored = { name, faces, length: bytes.byteLength, sha256: hash };
      try {
        await new Promise<void>((resolve, reject) => {
          const tx = db.transaction([STORES.files, STORES.faces, STORES.documents], "readwrite");
          tx.objectStore(STORES.files).put({ name, bytes });
          tx.objectStore(STORES.faces).put(stored);
          if (font.folder !== null) {
            // A licence name only, and a file that is there is never overwritten.
            const documents = tx.objectStore(STORES.documents);
            for (const doc of font.documents.filter((d) => plain(d.name) && synced_name_ok(d.name, true))) {
              const key = `${font.folder}/${doc.name}`;
              const held = documents.getKey(key);
              held.onsuccess = () => {
                if (held.result === undefined) {
                  documents.put({ key, bytes: buffer(doc.bytes) });
                }
              };
            }
          }
          tx.oncomplete = () => resolve();
          tx.onerror = () => reject(tx.error);
          tx.onabort = () => reject(tx.error);
        });
        known.push(stored);
        outcomes.push("written");
      } catch (error) {
        if (!isQuota(error)) {
          throw error;
        }
        outcomes.push("refused");
      }
    }
  }
  return outcomes;
}
