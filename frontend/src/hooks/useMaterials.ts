import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { flushMaterialWrites, loadMaterialPhoto, photoFailureText } from "@/lib/materialsFile";
import { saveFailureText } from "@/lib/materialRows";
import { decodeMaterialsText, type MaterialsView } from "@/lib/materialsView";
import type { SetKind } from "@/lib/operationSettingsView";

/** What the Materials tab reads from the editor: the session and its re-read signal. */
export interface MaterialsEditor {
  getSession: () => WasmSession | null;
  syncRevision: number;
  sync: () => void;
}

/** A notice of the tab: a refusal, or a record or photo that could not be saved (`role="alert"`). */
export interface MaterialsNotice {
  text: string;
  alert: boolean;
}

/** The New material form's typed text; `name` is `null` until the maker types (the default name follows the view). */
export interface MaterialDraft {
  name: string | null;
  thickness: string;
}

/** The result of Add on the New material form. */
export type AddResult = { ok: true; id: string } | { ok: false; text: string };

/**
 * The Materials tab's state and commands (`specs/0112-test-grid-and-materials` `ux-notes.md` 7 and
 * 8; the session holds the rules, this hook calls them, hands the records and photos the session
 * changed to the host and re-reads the view). It does not grow `useEditorSession`.
 */
export interface MaterialsApi {
  /** The decoded tab of the active laser page; `null` on any other page. */
  view: MaterialsView | null;
  notice: MaterialsNotice | null;
  /** The form's draft: a tab change keeps it, Cancel and Add drop it. */
  draft: MaterialDraft;
  setDraft: (draft: MaterialDraft) => void;
  /** Opens the New material form; `grid` is the node id JSON of the grid whose bar asked, or empty. */
  openForm: (grid?: string) => void;
  cancelForm: () => void;
  add: (name: string, thicknessMm: number) => AddResult;
  /** A press on a row: unfolds the material, `""` folds. */
  unfold: (id: string) => void;
  rename: (id: string, name: string) => boolean;
  remove: (id: string) => boolean;
  move: (id: string, index: number) => boolean;
  setThickness: (id: string, mm: number) => string;
  toggleLaser: (profileId: string) => void;
  openLaserPick: (open: boolean) => void;
  pickLaser: (id: string, profileId: string) => boolean;
  addSet: (id: string, profileId: string, kind: SetKind) => string;
  editSet: (id: string, profileId: string, kind: SetKind, field: string, value: number) => string;
  deleteSet: (id: string, profileId: string, kind: SetKind) => string;
  openTest: (test: string) => void;
  judge: (material: string, test: string, column: number, row: number) => string;
  setNote: (material: string, test: string, note: string) => string;
  setPhoto: (material: string, test: string, bytes: Uint8Array, cropped: boolean) => string;
  removePhoto: (material: string, test: string) => string;
  deleteTest: (material: string, test: string) => string;
  place: (material: string, test: string) => string;
  /** The bytes of the stored photo `name`, or `"missing"` (shows "Photo not found"). */
  photo: (name: string) => Promise<Uint8Array | "missing">;
  /** Shows `text` as a notice (an action of the photo controls). */
  showNotice: (text: string, alert: boolean) => void;
}

const NOTICE_MS = 8000;

function randomBytes(): Uint8Array {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return bytes;
}

/** The Materials tab's API for `editor`. */
export function useMaterials(editor: MaterialsEditor): MaterialsApi {
  const { getSession, syncRevision, sync } = editor;
  const view = useMemo(() => {
    void syncRevision;
    const session = getSession();
    return session ? decodeMaterialsText(session.materials_view_json()) : null;
  }, [getSession, syncRevision]);
  const [notice, setNotice] = useState<MaterialsNotice | null>(null);
  const [draft, setDraft] = useState<MaterialDraft>({ name: null, thickness: "" });
  // The host writes of the last command; a photo read waits for them, so a photo just stored or
  // cropped is read back, not found missing while its write is still running.
  const lastFlush = useRef<Promise<unknown>>(Promise.resolve());

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
      const flushed = flushMaterialWrites(session, sync);
      lastFlush.current = flushed.catch(() => null);
      void flushed.then((failure) => {
        if (failure !== null) {
          setNotice({
            text: failure.photo ? photoFailureText(failure.reason) : saveFailureText(failure.name, failure.reason),
            alert: true,
          });
        }
      });
      return result;
    },
    [getSession, sync],
  );
  /** A command whose text result is its refusal. */
  const refused = useCallback(
    (call: (session: WasmSession) => string): string => {
      const cause = run(call) ?? "";
      if (cause) setNotice({ text: cause, alert: false });
      return cause;
    },
    [run],
  );

  const openForm = useCallback(
    (grid = "") => {
      setDraft({ name: null, thickness: "" });
      run((s) => s.open_material_form(grid));
    },
    [run],
  );
  const cancelForm = useCallback(() => {
    setDraft({ name: null, thickness: "" });
    run((s) => s.cancel_material_form());
  }, [run]);
  const add = useCallback(
    (name: string, thicknessMm: number): AddResult => {
      const answer = run((s) => s.add_material(name, thicknessMm, randomBytes()));
      if (answer === undefined) return { ok: false, text: "Could not add the material." };
      try {
        const value: unknown = JSON.parse(answer);
        if (typeof value === "object" && value !== null) {
          const { ok, id, text } = value as { ok?: unknown; id?: unknown; text?: unknown };
          if (ok === true && typeof id === "string") {
            setDraft({ name: null, thickness: "" });
            return { ok: true, id };
          }
          if (ok === false && typeof text === "string") return { ok: false, text };
        }
      } catch {
        // falls through to the generic refusal
      }
      return { ok: false, text: "Could not add the material." };
    },
    [run],
  );
  const unfold = useCallback((id: string) => void run((s) => s.unfold_material(id)), [run]);
  const rename = useCallback((id: string, name: string) => refused((s) => s.rename_material(id, name)) === "", [refused]);
  const remove = useCallback((id: string) => refused((s) => s.delete_material(id)) === "", [refused]);
  const move = useCallback((id: string, index: number) => run((s) => s.move_material(id, index)) ?? false, [run]);
  const setThickness = useCallback((id: string, mm: number) => refused((s) => s.set_material_thickness(id, mm)), [refused]);
  const toggleLaser = useCallback((profileId: string) => void run((s) => s.toggle_material_laser(profileId)), [run]);
  const openLaserPick = useCallback((open: boolean) => void run((s) => s.open_material_laser_pick(open)), [run]);
  const pickLaser = useCallback((id: string, profileId: string) => run((s) => s.pick_material_laser(id, profileId)) ?? false, [run]);
  const addSet = useCallback(
    (id: string, profileId: string, kind: SetKind) => refused((s) => s.add_material_set(id, profileId, kind, randomBytes())),
    [refused],
  );
  const editSet = useCallback(
    (id: string, profileId: string, kind: SetKind, field: string, value: number) =>
      refused((s) => s.edit_material_set(id, profileId, kind, field, value, randomBytes())),
    [refused],
  );
  const deleteSet = useCallback(
    (id: string, profileId: string, kind: SetKind) => refused((s) => s.delete_material_set(id, profileId, kind, randomBytes())),
    [refused],
  );
  const openTest = useCallback((test: string) => void run((s) => s.open_material_test(test)), [run]);
  const judge = useCallback(
    (material: string, test: string, column: number, row: number) =>
      refused((s) => s.judge_test(material, test, column, row, randomBytes())),
    [refused],
  );
  const setNote = useCallback((material: string, test: string, note: string) => refused((s) => s.set_test_note(material, test, note)), [refused]);
  const setPhoto = useCallback(
    (material: string, test: string, bytes: Uint8Array, cropped: boolean) =>
      refused((s) => s.set_test_photo(material, test, bytes, cropped)),
    [refused],
  );
  const removePhoto = useCallback((material: string, test: string) => refused((s) => s.remove_test_photo(material, test)), [refused]);
  const deleteTest = useCallback((material: string, test: string) => refused((s) => s.delete_test(material, test)), [refused]);
  const place = useCallback((material: string, test: string) => refused((s) => s.place_material_test(material, test)), [refused]);
  const photo = useCallback(
    async (name: string): Promise<Uint8Array | "missing"> => {
      await lastFlush.current;
      const session = getSession();
      return session ? loadMaterialPhoto(session, name) : "missing";
    },
    [getSession],
  );
  const showNotice = useCallback((text: string, alert: boolean) => setNotice({ text, alert }), []);

  return {
    view,
    notice,
    draft,
    setDraft,
    openForm,
    cancelForm,
    add,
    unfold,
    rename,
    remove,
    move,
    setThickness,
    toggleLaser,
    openLaserPick,
    pickLaser,
    addSet,
    editSet,
    deleteSet,
    openTest,
    judge,
    setNote,
    setPhoto,
    removePhoto,
    deleteTest,
    place,
    photo,
    showNotice,
  };
}
