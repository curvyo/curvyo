import { useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { EditorSession } from "@/hooks/useEditorSession";
import { WIDTH_PRESET_GLYPHS } from "@/lib/widthPresets";

/**
 * Edit width in the tool bar (`specs/0115` criterion 4): last in the Select and Node bars
 * (`inBar`) when one stroked path is selected, and as a chip of its own while width editing is on
 * in another tool. Pressed while editing,
 * where it reads "Done" and leaves the mode as Escape does (the only Done, UX review U4). A
 * refused start shows the session's reason below the button. A 14 px text button with the border of
 * its neighbours (Close path's Sharp and Smooth), filled while pressed (UX review 2026-11-12).
 */
export function WidthEditToggle({ editor, inBar = false }: { editor: EditorSession; inBar?: boolean }) {
  const session = editor.getSession();
  const active = session?.width_edit_active() ?? false;
  const [notice, setNotice] = useState("");
  const offered =
    (editor.tool === "select" || editor.tool === "node") && (session?.width_edit_offered() ?? false);
  if (!active && !offered) return null;
  return (
    <div
      data-width-edit
      className={`pointer-events-auto relative flex shrink-0 items-center ${inBar ? "" : "ml-1 h-9 rounded-lg px-1"}`}
      style={inBar ? undefined : { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <Tooltip side="bottom" content={active ? "Leave width editing (Esc)" : "Edit width: drag, double-click or scroll the diamonds"}>
        <button
          type="button"
          aria-pressed={active}
          className={`my-1 flex h-7 min-w-[96px] items-center justify-center gap-1.5 rounded-[5px] border px-3 text-sm whitespace-nowrap outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
            active ? "" : "hover:bg-[var(--editor-accent-hover)]"
          }`}
          style={
            active
              ? {
                  background: "var(--toolbar-icon-active-bg)",
                  borderColor: "var(--toolbar-icon-active-bg)",
                  color: "var(--toolbar-icon-active-fg)",
                }
              : {
                  borderColor: "color-mix(in srgb, var(--toolbar-icon) 60%, transparent)",
                  color: "var(--toolbar-icon)",
                }
          }
          onClick={() => {
            const s = editor.getSession();
            if (!s) return;
            if (active) {
              s.end_width_edit();
              setNotice("");
            } else if (!s.begin_width_edit()) {
              setNotice(s.take_unsupported_width_notice());
            }
            editor.sync();
            editor.containerRef.current?.focus();
          }}
        >
          <svg width={16} height={16} viewBox="0 0 24 24" aria-hidden="true">
            <path d={WIDTH_PRESET_GLYPHS["taper-both"]} fill="currentColor" />
          </svg>
          {active ? "Done" : "Edit width"}
        </button>
      </Tooltip>
      {notice && !active ? (
        <div
          role="status"
          className="absolute top-full right-0 mt-1 rounded-md px-2 py-1 text-xs whitespace-nowrap"
          style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
        >
          {notice}
        </div>
      ) : null}
    </div>
  );
}
