import { useRef } from "react";

import { shiftStyle, useNoticeShift } from "@/hooks/useNoticeShift";

/** The look of "Object to path" and "Linked clone": a 28 px text button. */
const BUTTON_CLASS =
  "my-1 h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

/**
 * The Box from path button of the Select bar (`specs/0149-path-box` UX notes §1.1): a text button,
 * last in the bar, shown (hidden, not disabled) for one closed outline. A refusal ends with
 * "Nothing was changed." and sits 4 px under the button in the refusal look of the design system
 * ("Action notice").
 */
export function BoxFromPathButton({
  onPress,
  onReturnFocus,
  notice,
}: {
  onPress: () => void;
  onReturnFocus: () => void;
  notice: { id: number; text: string } | null;
}) {
  const noticeRef = useRef<HTMLDivElement>(null);
  const shift = useNoticeShift(noticeRef, notice);
  return (
    <div className="relative">
      <button
        type="button"
        data-select-command="box-from-path"
        onClick={(event) => {
          onPress();
          // `detail` is 0 for Space and Enter (the focus stays), 1 or more for a click.
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
        className={BUTTON_CLASS}
      >
        Box from path
      </button>
      {/* Takes no focus; the live region is in the tree from the start. */}
      <div
        ref={noticeRef}
        role="alert"
        className="absolute top-full right-0 mt-1 w-max max-w-[320px] break-words rounded-md px-2 py-1 text-xs ring-1 ring-[var(--field-invalid)]"
        style={
          notice
            ? { background: "var(--popover)", color: "var(--field-invalid)", ...shiftStyle(shift) }
            : { display: "none" }
        }
      >
        {notice?.text}
      </div>
    </div>
  );
}
