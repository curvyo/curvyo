import { useCallback, useEffect, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { operationIdText, type OperationId } from "@/lib/operationsView";
import type { SettingsField, SettingsRole } from "@/lib/operationSettingsView";

/** What the Settings block reads from the editor: the session and its re-read signal. */
export interface OperationSettingsEditor {
  getSession: () => WasmSession | null;
  sync: () => void;
}

/**
 * The Settings block's commands (`specs/0028-material-test-library` `ux-notes.md` section 8;
 * the session holds every rule, this hook only calls it and re-reads the view). It does not grow
 * `useOperations` or `useEditorSession`.
 */
export interface OperationSettingsApi {
  /** The skip notice of the last pick over several operations, shown 5 s (criterion 27), else `null`. */
  skip: string | null;
  /** Picks material `materialId` for `ids`; an empty id is "Own numbers". Whether anything was written. */
  pick: (ids: OperationId[], materialId: string) => boolean;
  /** Types `value` into `field` of the `role` set of `id`; the chip text, empty when taken. */
  edit: (id: OperationId, role: SettingsRole, field: SettingsField, value: number) => string;
  /** Reset to material of `id`. */
  reset: (id: OperationId) => boolean;
}

const SKIP_MS = 5000;

/** The Settings block's API for `editor`. */
export function useOperationSettings(editor: OperationSettingsEditor): OperationSettingsApi {
  const { getSession, sync } = editor;
  const [skip, setSkip] = useState<string | null>(null);

  useEffect(() => {
    if (skip === null) return;
    const timer = window.setTimeout(() => setSkip(null), SKIP_MS);
    return () => window.clearTimeout(timer);
  }, [skip]);

  const pick = useCallback(
    (ids: OperationId[], materialId: string) => {
      const session = getSession();
      if (!session) return false;
      const changed = session.pick_operation_material(JSON.stringify(ids.map(operationIdText)), materialId);
      // The notice is only a skip when several operations were picked for.
      setSkip(ids.length > 1 ? (session.operations_notice() ?? null) : null);
      sync();
      return changed;
    },
    [getSession, sync],
  );
  const edit = useCallback(
    (id: OperationId, role: SettingsRole, field: SettingsField, value: number) => {
      const session = getSession();
      if (!session) return "";
      const chip = session.edit_operation_setting(operationIdText(id), role, field, value);
      sync();
      return chip;
    },
    [getSession, sync],
  );
  const reset = useCallback(
    (id: OperationId) => {
      const session = getSession();
      if (!session) return false;
      const changed = session.reset_operation_to_material(operationIdText(id));
      sync();
      return changed;
    },
    [getSession, sync],
  );

  return { skip, pick, edit, reset };
}
