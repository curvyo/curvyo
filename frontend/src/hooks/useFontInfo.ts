// The Info mode's data for the subject face (`specs/0083-font-info-and-zip` criteria 2, 4 to 6):
// the fields the engine reads from the face's bytes once they are held (no second file read),
// the listing's file, size, folder path and documents, and an included font's shipped licence
// text. Documents are read or opened on request.
import { useEffect, useMemo, useState } from "react";

import { type GlyphSubject, hostFolders, hostListing } from "@/hooks/useText";
import { type FontInfoFields, isPdfName, sortedDocuments } from "@/lib/fontInfoText";
import { faceId, previewEngine, previewFaceSize, readPreviewFace } from "@/lib/previewFaces";
import { invoke, isTauri } from "@/platform/host";

export type FontInfoStatus = "none" | "missing" | "loading" | "unreadable" | "ready";

/** A document of the subject face's font folder. */
export interface InfoDocument {
  name: string;
  size: number;
  pdf: boolean;
}

export interface FontInfoState {
  status: FontInfoStatus;
  fields: FontInfoFields | null;
  file: string;
  size: number | null;
  /** The folder path (desktop, Curvyo fonts and This computer), for display only. */
  path: string | null;
  /** An included font's licence id and its shipped text once fetched. */
  licence: { id: string; text: string | null } | null;
  /** The font folder and its documents; `null` outside a font folder (no Documents row). */
  folder: { name: string; documents: InfoDocument[] } | null;
}

/** The text of a document, at most its first 1 MiB, and whether it was cut there. */
export function readDocumentText(folder: string, name: string): Promise<{ text: string; cut: boolean }> {
  return invoke<{ text: string; cut: boolean }>("read_font_document", { folder, name });
}

/** Opens a PDF document: the system viewer on the desktop, a new tab from the stored bytes in
 * the browser. */
export function openDocumentPdf(folder: string, name: string): Promise<void> {
  return invoke<void>("open_font_document", { folder, name });
}

const licenceTexts = new Map<string, Promise<string | null>>();

function licenceText(file: string): Promise<string | null> {
  let pending = licenceTexts.get(file);
  if (!pending) {
    pending = fetch(`${import.meta.env.BASE_URL}fonts/${file}`)
      .then((r) => (r.ok ? r.text() : null))
      .catch(() => null);
    licenceTexts.set(file, pending);
  }
  return pending;
}

export function useFontInfo(subject: GlyphSubject | null, active: boolean, listRevision: unknown): FontInfoState {
  const face = subject?.face ?? null;
  const key = face ? faceId(face) : "";
  const [held, setHeld] = useState<Record<string, boolean>>({});
  const [licence, setLicence] = useState<Record<string, string | null>>({});

  useEffect(() => {
    if (!active || !face || subject?.missing || held[key] !== undefined) {
      return undefined;
    }
    let live = true;
    void readPreviewFace(face).then((ok) => {
      if (live) {
        setHeld((before) => ({ ...before, [key]: ok }));
      }
    });
    return () => {
      live = false;
    };
  }, [active, face, key, subject?.missing, held]);

  const entry = face ? hostListing(face.block).find((f) => f.id === face.id && f.index === face.faceIndex) : undefined;
  const licenceFile = face?.block === "included" ? entry?.licence_file : undefined;
  useEffect(() => {
    if (!active || !licenceFile || licence[licenceFile] !== undefined) {
      return;
    }
    void licenceText(licenceFile).then((text) => setLicence((before) => ({ ...before, [licenceFile]: text })));
  }, [active, licenceFile, licence]);

  const fields = useMemo(() => {
    if (!active || !face || !held[key]) {
      return null;
    }
    const json = previewEngine().font_info(key);
    return json ? (JSON.parse(json) as FontInfoFields) : null;
  }, [active, face, key, held]);

  // `listRevision`: the listing changed (a Remove took a document or the folder).
  const folder = useMemo(() => {
    void listRevision;
    const name = face?.block === "curvyo" ? entry?.folder : null;
    if (!name) {
      return null;
    }
    const listed = hostFolders().find((f) => f.name === name);
    const documents = sortedDocuments(listed?.documents ?? []).map((d) => ({ ...d, pdf: isPdfName(d.name) }));
    return documents.length > 0 ? { name, documents } : null;
  }, [face, entry?.folder, listRevision]);

  let status: FontInfoStatus = "loading";
  if (!subject) {
    status = "none";
  } else if (subject.missing || !face) {
    status = "missing";
  } else if (held[key] === false) {
    status = "unreadable";
  } else if (fields) {
    status = "ready";
  }
  const file = entry?.file ?? face?.id.slice(face.id.lastIndexOf("/") + 1) ?? "";
  const showPath = isTauri && (face?.block === "curvyo" || face?.block === "system");
  return {
    status,
    fields,
    file,
    size: entry?.size ?? (face ? previewFaceSize(face) : null),
    path: showPath ? (entry?.path ?? null) : null,
    licence: entry?.licence && licenceFile ? { id: entry.licence, text: licence[licenceFile] ?? null } : null,
    folder,
  };
}
