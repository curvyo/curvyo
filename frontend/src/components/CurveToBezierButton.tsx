import type { EditorSession } from "@/hooks/useEditorSession";

/** Explicitly replaces editable curve controls with Bézier artwork. One line that never wraps
 * (`specs/0110` UX review U4); a refusal shows beside the button in the refusal look (U8). */
export function CurveToBezierButton({ editor }: { editor: EditorSession }) {
  const session = editor.getSession();
  const bspline = session?.can_convert_selected_bspline_to_bezier() ?? false;
  const spiro = session?.can_convert_selected_spiro_to_bezier() ?? false;
  if (!bspline && !spiro) return null;
  return <div className="pointer-events-auto relative ml-1 flex h-9 shrink-0 items-center rounded-lg bg-[var(--toolbar-bg)] px-1 shadow-md">
    <button type="button" className="h-7 rounded-md px-2 text-xs whitespace-nowrap hover:bg-[var(--editor-accent-hover)]" title="Replace curve controls with Bézier nodes" onClick={(event) => {
      const current = editor.getSession();
      const converted = spiro
        ? current?.convert_selected_spiro_to_bezier() ?? false
        : current?.convert_selected_bspline_to_bezier() ?? false;
      // The session says why for a Spiro (shown at the next frame); a B-spline gets this line.
      if (!converted && !(current?.transform_notice_is_refusal() ?? false)) {
        const box = event.currentTarget.getBoundingClientRect();
        editor.showRefusal(
          "The curve could not be converted. Check for linked dimensions, text on the curve, or sharing. Nothing changed.",
          { x: box.left, y: box.bottom },
        );
      }
      editor.sync();
      editor.containerRef.current?.focus();
    }}>Curve to Bézier</button>
  </div>;
}
