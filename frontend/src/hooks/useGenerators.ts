import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import {
  type GeneratedView,
  type GeneratorKindEntry,
  decodeGeneratedView,
  decodeKinds,
} from "@/lib/generatedView";

type GeneratorsEditor = Pick<EditorSession, "getSession" | "sync" | "syncRevision" | "showHistoryNotice">;

/** How long a clamp notice stays (UX notes §6). */
const NOTICE_MS = 3000;
/** How long the notice of unknown generator objects stays (UX notes §12). */
const OPEN_NOTICE_MS = 5000;

/** The Generators bar: the kinds and the chosen one (`specs/0141` UX notes §1). */
export interface GeneratorKindsApi {
  kinds: GeneratorKindEntry[];
  /** `<extension>/<kind>`, the last choice of the session. */
  kind: string;
  setKind: (kind: string) => void;
}

/** The Generators bar's state. */
export function useGeneratorKinds(editor: GeneratorsEditor): GeneratorKindsApi {
  const { getSession, sync, syncRevision } = editor;
  const kinds = useMemo(() => {
    void syncRevision;
    const session = getSession();
    return session ? decodeKinds(session.generator_kinds_json()) : [];
  }, [getSession, syncRevision]);
  const kind = useMemo(() => {
    void syncRevision;
    return getSession()?.generator_kind() ?? "";
  }, [getSession, syncRevision]);
  const setKind = useCallback(
    (next: string) => {
      getSession()?.set_generator_kind(next);
      sync();
    },
    [getSession, sync],
  );
  return { kinds, kind, setKind };
}

/** The Generator block of the Style tab (`specs/0141` UX notes §3, §6, §11, §12). */
export interface GeneratorBlockApi {
  /** The one selected generated object's rows, `null` for no block. */
  view: GeneratedView | null;
  /** The clamp notice of the last edit, an overlay for three seconds. */
  notice: string | null;
  /** A number, switch or choice into a row; `"committed"`, `"unchanged"` or `"invalid:<code>"`. */
  setValue: (key: string, value: number | boolean | string) => string;
  /** A value stepped into a row, drawn on the canvas without writing (UX notes §7). */
  preview: (key: string, value: number) => void;
  /** Drops the preview of a stepped value (Escape, or the field left without a commit). */
  cancelPreview: () => void;
  /** Convert to paths. */
  convert: () => void;
}

/** The Generator block's state; also shows the once-only notice of an opened file with a
 * generator this version does not have, in the canvas notice slot. */
export function useGeneratorBlock(editor: GeneratorsEditor): GeneratorBlockApi {
  const { getSession, sync, syncRevision, showHistoryNotice } = editor;
  const view = useMemo(() => {
    void syncRevision;
    const session = getSession();
    return session ? decodeGeneratedView(session.generated_panel_json()) : null;
  }, [getSession, syncRevision]);
  const [notice, setNotice] = useState<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  useEffect(() => {
    void syncRevision;
    const text = getSession()?.take_generated_notice() ?? "";
    if (text !== "") {
      showHistoryNotice(text, OPEN_NOTICE_MS, "canvas", "status");
    }
  }, [getSession, syncRevision, showHistoryNotice]);

  const setValue = useCallback(
    (key: string, value: number | boolean | string): string => {
      const session = getSession();
      if (!session) {
        return "invalid:number";
      }
      const answer = JSON.parse(session.set_generated_value(key, JSON.stringify(value))) as {
        written?: boolean;
        notices?: string[];
        error?: string;
      };
      sync();
      if (answer.error !== undefined) {
        return "invalid:number";
      }
      window.clearTimeout(timer.current);
      const text = (answer.notices ?? []).join(" ");
      setNotice(text === "" ? null : text);
      if (text !== "") {
        timer.current = window.setTimeout(() => setNotice(null), NOTICE_MS);
      }
      return answer.written ? "committed" : "unchanged";
    },
    [getSession, sync],
  );
  const preview = useCallback(
    (key: string, value: number) => {
      const session = getSession();
      if (!session) {
        return;
      }
      session.preview_generated_value(key, JSON.stringify(value));
      sync();
    },
    [getSession, sync],
  );
  const cancelPreview = useCallback(() => {
    getSession()?.cancel_generated_preview();
    sync();
  }, [getSession, sync]);
  const convert = useCallback(() => {
    getSession()?.convert_selected_generated();
    setNotice(null);
    sync();
  }, [getSession, sync]);
  return { view, notice, setValue, preview, cancelPreview, convert };
}
