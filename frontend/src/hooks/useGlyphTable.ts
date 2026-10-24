// The glyph table's data for the open pane (`specs/0082-glyph-table` criteria 4, 10, 11, 16 and
// 17): the subject face read into the engine, the names fetched on the first Glyphs open, and the
// table for the search, computed again when the face, the search or the names change.
import { useEffect, useMemo, useState } from "react";

import type { GlyphSubject } from "@/hooks/useText";
import { type GlyphTableData, glyphTableOf, loadUnicodeNames, subscribeNames } from "@/lib/glyphFaces";
import { faceId, readPreviewFace } from "@/lib/previewFaces";

export type GlyphTableStatus = "none" | "missing" | "loading" | "ready";

export interface GlyphTableState {
  status: GlyphTableStatus;
  table: GlyphTableData | null;
  /** Changes when the subject face changes: the grid goes back to the top (criterion 10). */
  faceKey: string;
}

export function useGlyphTable(subject: GlyphSubject | null, query: string, active: boolean): GlyphTableState {
  const face = subject?.face ?? null;
  const faceKey = face ? faceId(face) : "";
  const [ready, setReady] = useState<Record<string, boolean>>({});
  const [namesRevision, setNamesRevision] = useState(0);

  useEffect(() => {
    if (!active) {
      return undefined;
    }
    loadUnicodeNames();
    return subscribeNames(() => setNamesRevision((r) => r + 1));
  }, [active]);

  useEffect(() => {
    if (!active || !face || subject?.missing || ready[faceKey] !== undefined) {
      return;
    }
    let live = true;
    void readPreviewFace(face).then((ok) => {
      if (live) {
        setReady((before) => ({ ...before, [faceKey]: ok }));
      }
    });
    return () => {
      live = false;
    };
  }, [active, face, faceKey, subject?.missing, ready]);

  const held = ready[faceKey];
  const table = useMemo(
    () => (active && face && held ? glyphTableOf(face, query) : null),
    // `namesRevision`: a search by name runs again once the names are in.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [active, faceKey, held, query, namesRevision],
  );
  let status: GlyphTableStatus = "loading";
  if (!subject) {
    status = "none";
  } else if (subject.missing || !face || held === false) {
    status = "missing";
  } else if (table) {
    status = "ready";
  }
  return { status, table, faceKey };
}
