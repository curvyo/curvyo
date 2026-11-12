import { useCallback, useEffect, useMemo, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { flushMaterialWrites } from "@/lib/materialsFile";
import { saveFailureText } from "@/lib/materialRows";
import { decodeMaterialsText, type MaterialsView } from "@/lib/materialsView";
import type { SetKind } from "@/lib/operationSettingsView";

/** What the Materials section reads from the editor: the session and its re-read signal. */
export interface MaterialsEditor {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
}

/** A notice of the section: a refusal, or a record that could not be saved (`role="alert"`). */
export interface MaterialsNotice {
  text: string;
  alert: boolean;
}

/**
 * The Materials section's state and commands (`specs/0028-material-test-library` `ux-notes.md`
 * section 10; the session holds the rules, this hook calls them, hands the records the session
 * changed to the host and re-reads the view). It does not grow `useEditorSession`.
 */
export interface MaterialsApi {
  /** The decoded section of the active laser page; `null` on any other page. */
  view: MaterialsView | null;
  notice: MaterialsNotice | null;
  /** Counts up when "Edit material" asks the section to open, select and focus a row. */
  reveal: { id: string; count: number } | null;
  select: (id: string) => void;
  /** "Edit material" in the Settings block: selects the material and asks the section to open. */
  edit: (id: string) => void;
  rename: (id: string, name: string) => boolean;
  /** The armed second press. */
  remove: (id: string) => boolean;
  move: (id: string, index: number) => boolean;
  /** The Thickness field; the chip text, empty when taken. */
  setThickness: (id: string, mm: number) => string;
  /** A number of a set; the chip text, empty when taken. */
  setNumber: (id: string, kind: SetKind, field: "power" | "speed" | "passes", value: number) => string;
  /** The armed set trash. */
  removeSet: (id: string, kind: SetKind) => boolean;
  /** Save as material from the marked cell: `existing` empty for a new one. The chip text, empty when taken. */
  save: (existing: string, name: string, thicknessMm: number, passes: number) => string;
}

const NOTICE_MS = 8000;

function randomBytes(): Uint8Array {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return bytes;
}

/** The Materials section's API for `editor`. */
export function useMaterials(editor: MaterialsEditor): MaterialsApi {
  const { getSession, syncRevision, sync } = editor;
  const view = useMemo(() => {
    void syncRevision;
    const session = getSession();
    return session ? decodeMaterialsText(session.materials_view_json()) : null;
  }, [getSession, syncRevision]);
  const [notice, setNotice] = useState<MaterialsNotice | null>(null);
  const [reveal, setReveal] = useState<{ id: string; count: number } | null>(null);

  useEffect(() => {
    if (notice === null) return;
    const timer = window.setTimeout(() => setNotice(null), NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);

  /** One command: the session call, the host writes it caused, then one re-read. */
  const run = useCallback(
    <T,>(call: (session: WasmSession) => T): T | undefined => {
      const session = getSession();
      if (!session) return undefined;
      const result = call(session);
      const refusal = session.take_materials_notice();
      if (refusal) setNotice({ text: refusal, alert: false });
      sync();
      void flushMaterialWrites(session, sync).then((failure) => {
        if (failure !== null) {
          setNotice({ text: saveFailureText(failure.name, failure.reason), alert: true });
        }
      });
      return result;
    },
    [getSession, sync],
  );

  const select = useCallback((id: string) => void run((s) => s.select_material(id)), [run]);
  const edit = useCallback(
    (id: string) => {
      run((s) => s.select_material(id));
      setReveal((previous) => ({ id, count: (previous?.count ?? 0) + 1 }));
    },
    [run],
  );
  const rename = useCallback(
    (id: string, name: string) => {
      const refusal = run((s) => s.rename_material(id, name));
      if (refusal) setNotice({ text: refusal, alert: false });
      return !refusal;
    },
    [run],
  );
  const remove = useCallback(
    (id: string) => {
      const refusal = run((s) => s.delete_material(id));
      if (refusal) setNotice({ text: refusal, alert: false });
      return !refusal;
    },
    [run],
  );
  const move = useCallback((id: string, index: number) => run((s) => s.move_material(id, index)) ?? false, [run]);
  const setThickness = useCallback((id: string, mm: number) => run((s) => s.set_material_thickness(id, mm)) ?? "", [run]);
  const setNumber = useCallback(
    (id: string, kind: SetKind, field: "power" | "speed" | "passes", value: number) =>
      run((s) => s.edit_material_set(id, kind, field, value, randomBytes())) ?? "",
    [run],
  );
  const removeSet = useCallback(
    (id: string, kind: SetKind) => {
      const refusal = run((s) => s.remove_material_set(id, kind, randomBytes()));
      if (refusal) setNotice({ text: refusal, alert: false });
      return !refusal;
    },
    [run],
  );

  const save = useCallback(
    (existing: string, name: string, thicknessMm: number, passes: number) => {
      const answer = run((s) => s.save_as_material(existing, name, thicknessMm, passes, randomBytes()));
      if (answer === undefined) return "Could not save the material.";
      const value: unknown = JSON.parse(answer);
      const failed = typeof value === "object" && value !== null && (value as { ok?: unknown }).ok === false;
      return failed ? String((value as { text?: unknown }).text ?? "") : "";
    },
    [run],
  );

  return { view, notice, reveal, select, edit, rename, remove, move, setThickness, setNumber, removeSet, save };
}
