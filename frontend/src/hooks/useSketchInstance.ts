import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import { decodeSketchInstance, type SketchInstanceView } from "@/lib/sketchInstanceView";

type InstanceEditor = Pick<EditorSession, "getSession" | "sync" | "syncRevision">;

/** How long a clamp notice stays (`0141` UX notes §6). */
const NOTICE_MS = 3000;

/** The Parameters override of the selected clones of a sketch (`specs/0099-sketch-pages`
 * criterion 24, UX notes 12.2). Every rule and every sentence is the session's. */
export interface SketchInstanceApi {
  /** The pill and the fields; `null` while the selection holds no clone of a sketch that exposes a value. */
  view: SketchInstanceView | null;
  /** The clamp notice of the last edit and the key of the field it sits under. */
  notice: { key: string; text: string } | null;
  /** A number into the field `key` (one step); `"committed"`, `"unchanged"` or `"invalid:number"`.
   * `onlyThis` (Shift at the gesture's start on a linked value) changes it alone (criterion 34). */
  setValue: (key: string, value: number, onlyThis?: boolean) => string;
  /** A value stepped into a field, shown on the canvas without writing. */
  preview: (key: string, value: number, onlyThis?: boolean) => void;
  cancelPreview: () => void;
  /** Back to the sketch's (or the original's) value of one field. */
  reset: (key: string) => void;
  /** The Original's segment: every own value dropped. */
  originals: () => void;
  /** The Own segment: every shown value becomes the clones' own. */
  own: () => void;
}

/** The Parameters row's state. */
export function useSketchInstance(editor: InstanceEditor): SketchInstanceApi {
  const { getSession, sync, syncRevision } = editor;
  const view = useMemo(() => {
    void syncRevision;
    const session = getSession();
    return session ? decodeSketchInstance(session.sketch_instance_json()) : null;
  }, [getSession, syncRevision]);
  const [notice, setNotice] = useState<{ key: string; text: string } | null>(null);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  const setValue = useCallback(
    (key: string, value: number, onlyThis = false): string => {
      const session = getSession();
      if (!session) {
        return "invalid:number";
      }
      const answer = JSON.parse(session.set_sketch_instance_value(key, value, onlyThis)) as {
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
    (key: string, value: number, onlyThis = false) => {
      getSession()?.preview_sketch_instance_value(key, value, onlyThis);
      sync();
    },
    [getSession, sync],
  );
  const cancelPreview = useCallback(() => {
    getSession()?.cancel_sketch_instance_preview();
    sync();
  }, [getSession, sync]);
  const reset = useCallback(
    (key: string) => {
      getSession()?.reset_sketch_instance(key);
      setNotice(null);
      sync();
    },
    [getSession, sync],
  );
  const originals = useCallback(() => reset(""), [reset]);
  const own = useCallback(() => {
    getSession()?.own_sketch_instance();
    sync();
  }, [getSession, sync]);
  return { view, notice, setValue, preview, cancelPreview, reset, originals, own };
}
