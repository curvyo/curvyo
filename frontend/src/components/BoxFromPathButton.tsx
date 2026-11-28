import { useRef } from "react";

import { shiftStyle, useNoticeShift } from "@/hooks/useNoticeShift";
import { Button } from "@/components/ui/button";
import { Box } from "lucide-react";


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
      <Button size="bar" icon={Box} iconOnly tooltip="Box from path" className="my-1"
        data-select-command="box-from-path"
        onClick={(event) => {
          onPress();
          // `detail` is 0 for Space and Enter (the focus stays), 1 or more for a click.
          if (event.detail > 0) {
            onReturnFocus();
          }
        }}
      >
        Box from path
      </Button>
      {/* Takes no focus; the live region is in the tree from the start. */}
      <div
        ref={noticeRef}
        role="alert"
        className="absolute top-full right-0 mt-1 w-max max-w-[320px] break-words rounded-[8px] px-2 py-1 text-xs ring-1 ring-[var(--field-invalid)]"
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
