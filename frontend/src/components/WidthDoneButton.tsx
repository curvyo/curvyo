import type { EditorSession } from "@/hooks/useEditorSession";

/**
 * The visible way out of width editing in the tool bar (`specs/0110` UX notes, "Width"): Done
 * leaves the mode as Escape does. Shown only while width editing is on; one line that never wraps,
 * and the only Done (UX review U4).
 */
export function WidthDoneButton({ editor }: { editor: EditorSession }) {
  if (!(editor.getSession()?.width_edit_active() ?? false)) return null;
  return (
    <div
      data-width-done
      className="pointer-events-auto ml-1 flex h-9 shrink-0 items-center gap-2 rounded-lg px-2 text-xs whitespace-nowrap"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)", color: "var(--toolbar-icon)" }}
    >
      <span>Width</span>
      <button
        type="button"
        className="h-7 rounded-md px-2 ring-1 ring-border"
        onClick={() => {
          editor.getSession()?.end_width_edit();
          editor.sync();
          editor.containerRef.current?.focus();
        }}
      >
        Done
      </button>
    </div>
  );
}
