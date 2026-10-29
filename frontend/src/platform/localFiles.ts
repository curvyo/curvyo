import type { ImportFile } from "@/hooks/useProject";
import { importFilesOf, type LocalFile } from "@/lib/localFiles";
import { invoke, isTauri } from "@/platform/host";

/**
 * The local files the clipboard (`"paste"`) or the drag just dropped (`"drop"`) names, read by
 * the desktop host (`specs/0087-image-clipboard-and-drop` criteria 6 and 21, lead decision
 * 2026-10-29). The page never names a file: the host reads the list itself (on Linux, where
 * WebKitGTK hides it from the page). None in the browser build, which cannot read a path, and none
 * when nothing names a local file.
 */
export async function resolveLocalFiles(from: "paste" | "drop"): Promise<ImportFile[]> {
  if (!isTauri) {
    return [];
  }
  const files = await invoke<LocalFile[]>("read_local_files", { from }).catch(() => []);
  return importFilesOf(files);
}
