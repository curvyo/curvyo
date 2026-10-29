/**
 * Local files a paste or a drop names (`specs/0087-image-clipboard-and-drop` criteria 6 and 21,
 * lead decision 2026-10-29): how the desktop host's answer becomes the import's files. Finding
 * the list and reading the files is the host's (`curvyo-app` `read_local_files`); nothing here
 * touches the network.
 */

import type { ImportFile } from "@/hooks/useProject";

/** One file the host was asked for (`curvyo-app` `LocalFile`). */
export type LocalFile =
  | { kind: "bytes"; name: string; bytes: number[] }
  | { kind: "too_large"; name: string; bytes: number }
  | { kind: "refused"; name: string }
  | { kind: "skipped"; name: string };

/** The host's answer as the import's files: a refused or skipped file is read as no bytes (the
 * import refuses or skips it with its notice), one over the limit carries its length. */
export function importFilesOf(files: LocalFile[]): ImportFile[] {
  return files.map((file) => {
    switch (file.kind) {
      case "bytes":
        return { name: file.name, bytes: new Uint8Array(file.bytes) };
      case "too_large":
        return { name: file.name, bytes: new Uint8Array(), tooLarge: file.bytes };
      default:
        return { name: file.name, bytes: new Uint8Array() };
    }
  });
}
