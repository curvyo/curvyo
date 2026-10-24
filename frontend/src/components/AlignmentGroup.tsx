// The Alignment group of the Text bar and the Select bar (`specs/0079-text-layout-controls`
// criterion 1; UX notes §1): four buttons as one radio group with roving focus (arrows move
// focus, Enter or Space presses), the look of the Style group's weight buttons. The session
// decides the pressed state; a press acts on pointer up.
import { TextAlignCenter, TextAlignEnd, TextAlignJustify, TextAlignStart } from "lucide-react";
import { Tooltip } from "radix-ui";
import { useRef, useState } from "react";

import {
  ALIGN_CODES,
  ALIGN_NAMES,
  type AlignCode,
  alignFocusMove,
  alignTabStop,
  alignTooltipLine,
} from "@/lib/textLayoutText";

const TOOLTIP_STYLE = {
  background: "var(--toolbar-bg)",
  color: "var(--toolbar-icon)",
  boxShadow: "var(--panel-elevation-shadow)",
} as const;

const BUTTON_CLASS =
  "flex size-7 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

const GLYPHS: Record<AlignCode, typeof TextAlignStart> = {
  left: TextAlignStart,
  centre: TextAlignCenter,
  right: TextAlignEnd,
  justify: TextAlignJustify,
};

interface AlignmentGroupProps {
  /** The pressed alignment; `null` when the subject texts differ (none pressed). */
  align: AlignCode | null;
  onPress: (align: AlignCode) => void;
  /** Focus back to the canvas, or to the edit sink in edit mode. */
  onReturnFocus: () => void;
}

export function AlignmentGroup({ align, onPress, onReturnFocus }: AlignmentGroupProps) {
  const [focused, setFocused] = useState<number | null>(null);
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const tabStop = focused ?? alignTabStop(align);
  const line = alignTooltipLine(align);
  return (
    <div
      role="radiogroup"
      aria-label="Alignment"
      className="flex items-center gap-1"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          onReturnFocus();
          return;
        }
        const next = alignFocusMove(event.key, focused ?? tabStop);
        if (next !== null) {
          event.preventDefault();
          setFocused(next);
          buttons.current[next]?.focus();
        }
      }}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
          setFocused(null);
        }
      }}
    >
      {ALIGN_CODES.map((code, index) => {
        const Glyph = GLYPHS[code];
        const pressed = align === code;
        return (
          <Tooltip.Root key={code}>
            <Tooltip.Trigger asChild>
              <button
                ref={(element) => {
                  buttons.current[index] = element;
                }}
                type="button"
                role="radio"
                aria-label={ALIGN_NAMES[code]}
                aria-checked={pressed}
                tabIndex={index === tabStop ? 0 : -1}
                onFocus={() => setFocused(index)}
                onClick={(event) => {
                  if (!pressed) {
                    onPress(code);
                  }
                  // A mouse press hands focus back; Enter or Space keeps it here.
                  if (event.detail > 0) {
                    onReturnFocus();
                  }
                }}
                className={`${BUTTON_CLASS} ${
                  pressed
                    ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
                    : "hover:bg-[var(--editor-accent-hover)]"
                }`}
              >
                <Glyph size={16} strokeWidth={1.5} aria-hidden />
              </button>
            </Tooltip.Trigger>
            <Tooltip.Portal>
              <Tooltip.Content
                side="bottom"
                sideOffset={6}
                className="z-50 max-w-[300px] rounded-md px-2 py-1 text-xs"
                style={TOOLTIP_STYLE}
              >
                <div className="font-semibold">{ALIGN_NAMES[code]}</div>
                {line ? <div style={{ color: "var(--panel-muted-fg)" }}>{line}</div> : null}
              </Tooltip.Content>
            </Tooltip.Portal>
          </Tooltip.Root>
        );
      })}
    </div>
  );
}
