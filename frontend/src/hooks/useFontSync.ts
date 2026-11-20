// The font rows' sync for the page (`specs/0116-library-sync` slice 2): the engine of
// `lib/fontSync.ts` held for the life of the device group hook, the toggle the Curvyo family rows
// read, and the slow poll's pump. Everything it decides is Rust's (the group, the budgets, the
// words); this hook only moves what the host holds.
import { useCallback, useEffect, useRef } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import type { FontSyncApi } from "@/lib/devicesApi";
import { FontSync } from "@/lib/fontSync";
import { type LibrarySyncView } from "@/lib/librarySyncView";
import type { WasmDeviceGroup } from "@/wasm-bindings/curvyo_editor_wasm.js";

export interface FontSyncHook {
  api: FontSyncApi;
  /** The slow poll: one pump of the engine for `group`. */
  pump: (group: WasmDeviceGroup, newItems: boolean) => void;
  /** The maker deleted a Curvyo font: these host ids lose their entries. */
  forget: (ids: string[]) => void;
}

/** `view` is the group's library view (`null` without a group); `reread` reads it again. */
export function useFontSync(
  editor: EditorSession,
  view: LibrarySyncView | null,
  groupRef: { current: WasmDeviceGroup | null },
  showNotice: (text: string, ms: number, anchor: "canvas", role: "status") => void,
  reread: () => void,
): FontSyncHook {
  const latest = useRef({ editor, view, showNotice, reread });
  useEffect(() => {
    latest.current = { editor, view, showNotice, reread };
  });
  const engine = useRef<FontSync | null>(null);
  engine.current ??= new FontSync({
    refreshFonts: () => latest.current.editor.text.listHostFonts(),
    notice: (text) => latest.current.showNotice(text, 5000, "canvas", "status"),
    changed: () => latest.current.reread(),
  });
  const toggle = useCallback((family: Parameters<FontSyncApi["toggle"]>[0]) => {
    const group = groupRef.current;
    const current = latest.current.view;
    if (group && current) {
      void engine.current?.toggle(group, family, current.fonts).then(latest.current.reread);
    }
  }, [groupRef]);
  const pump = useCallback(
    (group: WasmDeviceGroup, newItems: boolean) => {
      const current = latest.current.view;
      if (current) {
        void engine.current?.pump(group, newItems, current.fonts).catch(() => undefined);
      }
    },
    [],
  );
  const forget = useCallback((ids: string[]) => engine.current?.forgetFiles(ids), []);
  return {
    api: {
      on: (family) => (view === null ? true : (engine.current?.isOn(family, view.fonts) ?? false)),
      toggle,
      forget,
    },
    pump,
    forget,
  };
}
