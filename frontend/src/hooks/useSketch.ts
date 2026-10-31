import { useState } from "react";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { WasmSession } from "@/lib/editorSession";

export type { SketchQuantity, SketchConstraint, SketchParameter, SketchView } from "@/lib/sketchView";

/** The host holds drafts only; all committed sketch state is read from Rust. */
export function useSketch(editor: EditorSession) {
  const [error, setError] = useState<string | null>(null);
  const view = editor.sketchView;
  const run = (command: (session: WasmSession) => unknown) => {
    const session = editor.getSession();
    if (!session) return false;
    try { command(session); setError(null); editor.sync(); return true; }
    catch (error) { setError(String(error)); editor.sync(); return false; }
  };
  const numericValues = (action = ""): [string,string] => {
    const session = editor.getSession();
    return session ? JSON.parse(session.sketch_numeric_values_json(action)) : ["",""];
  };
  const focusCanvas = () => editor.containerRef.current?.focus({ preventScroll: true });
  const switchWorkbench = (name: "vector" | "sketch") => {
    if (view.workbench === name) return;
    // Stop the old Vector text/Trace context before the shared canvas changes owner.
    editor.setTool("select");
    run(session => session.set_workbench(name));
  };
  const inspectConstraint = (id: string, edit: boolean) => {
    if (!run(session => session.sketch_inspect_constraint(id, edit))) return;
    if (edit) requestAnimationFrame(() => {
      const field = document.querySelector<HTMLInputElement>(`[data-sketch-dimension="${CSS.escape(id)}"] input`);
      field?.focus(); field?.select();
    });
  };
  return { numericValues, presentation: editor.sketchPresentation, inspectConstraint, view, error: error ?? view.error, run, switchWorkbench, focusCanvas, clearError: () => setError(null) };
}
export type SketchApi = ReturnType<typeof useSketch>;
