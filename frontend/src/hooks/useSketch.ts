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
  /** Runs a command typed into the on-canvas entry box: Rust keeps its refusal for the box, so
   * nothing goes to Sketches (UX notes revision 2, section 3). */
  const runInBox = (command: (session: WasmSession) => unknown) => {
    const session = editor.getSession();
    if (!session) return false;
    try { command(session); return true; }
    catch { return false; }
    finally { editor.sync(); }
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
  return { presentation: editor.sketchPresentation, inspectConstraint, view, error: error ?? view.error, run, runInBox, switchWorkbench, focusCanvas, clearError: () => setError(null) };
}
export type SketchApi = ReturnType<typeof useSketch>;
