import { Tooltip } from "radix-ui";
import { useRef } from "react";

import type { StrokeToPathBarNotice } from "@/hooks/useStrokeToPath";
import { shiftStyle, useNoticeShift } from "@/hooks/useNoticeShift";
import { STROKE_TO_PATH_DIMMED, STROKE_TO_PATH_NAME } from "@/lib/strokeToPathText";

/** The look of "Object to path" and "Linked clone": a 28 px text button. */
const BUTTON_CLASS =
  "my-1 h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

/**
 * The Stroke to path button of the Select bar (`specs/0085-stroke-to-path` criteria 1, 2 and 22,
 * UX notes): a text button after Object to path, in the tree whenever something is selected,
 * dimmed (`aria-disabled`, 40 %, Tab stop and tooltip kept) while nothing selected draws a
 * stroke, the fifth exception of "Hidden, not disabled". One press, one result, the notice 4 px
 * under it. The decision and the write are the session's.
 */
export function StrokeToPathButton({
  enabled,
  onPress,
  onReturnFocus,
  notice,
}: {
  enabled: boolean;
  onPress: () => void;
  onReturnFocus: () => void;
  notice: StrokeToPathBarNotice | null;
}) {
  const noticeRef = useRef<HTMLDivElement>(null);
  const shift = useNoticeShift(noticeRef, notice);
  return (
    <div className="relative">
      <Tooltip.Provider delayDuration={400}>
        <Tooltip.Root>
          <Tooltip.Trigger asChild>
            <button
              type="button"
              // Not the `disabled` attribute: the button keeps its tooltip and its Tab stop.
              aria-disabled={!enabled}
              data-select-command="stroke-to-path"
              onClick={(event) => {
                if (!enabled) {
                  return;
                }
                onPress();
                // `detail` is 0 for Space and Enter (the focus stays), 1 or more for a click.
                if (event.detail > 0) {
                  onReturnFocus();
                }
              }}
              className={`${BUTTON_CLASS} ${enabled ? "hover:bg-[var(--editor-accent-hover)]" : "opacity-40"}`}
            >
              {STROKE_TO_PATH_NAME}
            </button>
          </Tooltip.Trigger>
          <Tooltip.Portal>
            <Tooltip.Content
              side="bottom"
              sideOffset={6}
              className="z-50 rounded-md px-2 py-1 text-xs"
              style={{
                background: "var(--toolbar-bg)",
                color: "var(--toolbar-icon)",
                boxShadow: "var(--panel-elevation-shadow)",
              }}
            >
              {enabled ? STROKE_TO_PATH_NAME : STROKE_TO_PATH_DIMMED}
            </Tooltip.Content>
          </Tooltip.Portal>
        </Tooltip.Root>
      </Tooltip.Provider>
      {/* Takes no focus; the live region is in the tree from the start (row "Action notice"). */}
      <div
        ref={noticeRef}
        role={notice?.alert ? "alert" : "status"}
        className={`absolute top-full right-0 mt-1 w-max max-w-[320px] break-words rounded-md px-2 py-1 text-xs ${
          notice?.alert ? "ring-1 ring-[var(--field-invalid)]" : ""
        }`}
        style={
          notice
            ? notice.alert
              ? { background: "var(--popover)", color: "var(--field-invalid)", ...shiftStyle(shift) }
              : {
                  background: "var(--toolbar-bg)",
                  color: "var(--toolbar-icon)",
                  boxShadow: "var(--panel-elevation-shadow)",
                  ...shiftStyle(shift),
                }
            : { display: "none" }
        }
      >
        {notice?.text}
      </div>
    </div>
  );
}
