import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import {
  type GeneratedView,
  type GeneratorKindEntry,
  decodeGeneratedView,
  canvasNoticeLook,
  decodeKinds,
} from "@/lib/generatedView";

type GeneratorsEditor = Pick<EditorSession, "getSession" | "sync" | "syncRevision" | "showHistoryNotice">;

/** How long a clamp notice stays (UX notes §6). */
const NOTICE_MS = 3000;

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
  /** The clamp notice of the last edit and the key of the edited row it sits under, an overlay
   * for three seconds. */
  notice: { key: string; text: string } | null;
  /** A number, switch or choice into a row; `"committed"`, `"unchanged"` or `"invalid:<code>"`. */
  setValue: (key: string, value: number | boolean | string) => string;
  /** A value stepped into a row, drawn on the canvas without writing (UX notes §7). */
  preview: (key: string, value: number) => void;
  /** Drops the preview of a stepped value (Escape, or the field left without a commit). */
  cancelPreview: () => void;
  /** Convert to paths. */
  convert: () => void;
  /** Replace outline: arms the pick of a new outline (`0149`). */
  replaceOutline: () => void;
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
  const [notice, setNotice] = useState<{ key: string; text: string } | null>(null);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  useEffect(() => {
    void syncRevision;
    const text = getSession()?.take_generated_notice() ?? "";
    if (text !== "") {
      const look = canvasNoticeLook(text);
      showHistoryNotice(text, look.ms, "canvas", look.kind);
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
      setNotice(text === "" ? null : { key, text });
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
  const replaceOutline = useCallback(() => {
    getSession()?.begin_replace_outline();
    sync();
  }, [getSession, sync]);
  return { view, notice, setValue, preview, cancelPreview, convert, replaceOutline };
}

/** The Select bar's Box from path (`specs/0149-path-box` UX notes §1.1). */
export interface BoxFromPathApi {
  /** One closed outline is selected: the button is in the bar. */
  shown: boolean;
  /** The press: builds the box, or shows the refusal under the button. */
  apply: () => void;
  /** The refusal under the button, `null` once it ended. */
  notice: { id: number; text: string } | null;
}

/** How long a refusal stays (row "Action notice": 8 s). */
const REFUSAL_MS = 8000;

/** The Box from path button's state; every decision is the session's. */
export function useBoxFromPath(editor: GeneratorsEditor): BoxFromPathApi {
  const { getSession, sync, syncRevision } = editor;
  const shown = useMemo(() => {
    void syncRevision;
    return getSession()?.box_from_path_shown() ?? false;
  }, [getSession, syncRevision]);
  const [notice, setNotice] = useState<{ id: number; text: string } | null>(null);
  const counter = useRef(0);
  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setNotice(null), REFUSAL_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);
  const apply = useCallback(() => {
    const session = getSession();
    const text = session?.apply_box_from_path() ?? "";
    if (text !== "") {
      // The session also keeps the refusal for the canvas notice slot; here it shows under the
      // button only (UX notes §1.1), so it is taken before the sync shows it a second time.
      session?.take_generated_notice();
    }
    sync();
    counter.current += 1;
    setNotice(text === "" ? null : { id: counter.current, text });
  }, [getSession, sync]);
  return { shown, apply, notice };
}
