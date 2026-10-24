import { Tooltip } from "radix-ui";
import { useEffect, useRef } from "react";

import type { FlipApi } from "@/hooks/useFlip";
import { useNoticeShift } from "@/hooks/useNoticeShift";
import { FLIP_DISABLED_LINE, FLIP_TEXT, type FlipAxis } from "@/lib/flipText";

const BUTTON_CLASS =
  "flex size-7 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

/**
 * The flip glyph (`specs/0056-mirror-and-negative-scale/` UX notes, "Glyphs"; `review-ux.md`
 * finding 4): a dashed axis with a triangle on each side pointing at it, the flat side outside,
 * the original outlined and the image filled. The
 * vertical flip is the same drawing turned a quarter turn, so the axis is horizontal and the
 * outlined triangle is on top.
 */
function FlipGlyph({ axis }: { axis: FlipAxis }) {
  return (
    <svg aria-hidden viewBox="0 0 16 16" className="size-4" fill="none">
      <g transform={axis === "vertical" ? "rotate(90 8 8)" : undefined}>
        <line
          x1="8"
          y1="1.5"
          x2="8"
          y2="14.5"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeDasharray="2.5 2"
        />
        <path
          d="M1.75 3.5 L5.75 8 L1.75 12.5 Z"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinejoin="round"
        />
        <path
          d="M14.25 3.5 L10.25 8 L14.25 12.5 Z"
          fill="currentColor"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinejoin="round"
        />
      </g>
    </svg>
  );
}

interface FlipButtonProps {
  axis: FlipAxis;
  enabled: boolean;
  flip: FlipApi;
  onReturnFocus: () => void;
}

function FlipButton({ axis, enabled, flip, onReturnFocus }: FlipButtonProps) {
  const text = FLIP_TEXT[axis];
  const notice = flip.notice?.axis === axis ? flip.notice : null;
  // Left-aligned to the button, shifted to stay right of the tool rail and inside the window
  // (`review-ux.md` finding 2).
  const noticeRef = useRef<HTMLDivElement>(null);
  const shift = useNoticeShift(noticeRef, notice);
  return (
    <div className="relative">
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <button
            type="button"
            aria-label={text.name}
            // Not the `disabled` attribute: the button keeps its tooltip and its Tab stop.
            aria-disabled={!enabled}
            onClick={(event) => {
              if (!enabled) {
                return;
              }
              flip.flip(axis);
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
            className={`${BUTTON_CLASS} ${
              enabled ? "hover:bg-[var(--editor-accent-hover)]" : "opacity-40"
            }`}
          >
            <FlipGlyph axis={axis} />
          </button>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            side="bottom"
            sideOffset={6}
            className="z-50 max-w-[260px] rounded-md px-2 py-1 text-xs"
            style={{
              background: "var(--toolbar-bg)",
              color: "var(--toolbar-icon)",
              boxShadow: "var(--panel-elevation-shadow)",
            }}
          >
            <div className="font-semibold">
              {text.name} ({text.key})
            </div>
            <div style={{ color: "var(--panel-muted-fg)" }}>{text.rule}</div>
            {!enabled && <div style={{ color: "var(--panel-muted-fg)" }}>{FLIP_DISABLED_LINE}</div>}
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
      {/* The no-op notice of criterion 3, 4 px under its button, left-aligned. */}
      <div
        ref={noticeRef}
        role="status"
        className="absolute top-full left-0 z-50 mt-1 w-max max-w-[320px] rounded-md px-2 py-1 text-xs"
        style={
          notice
            ? {
                background: "var(--toolbar-bg)",
                color: "var(--toolbar-icon)",
                boxShadow: "var(--panel-elevation-shadow)",
                transform: shift !== 0 ? `translateX(${shift}px)` : undefined,
              }
            : { display: "none" }
        }
      >
        {notice?.text}
      </div>
    </div>
  );
}

/**
 * The Flip group of the Select bar (`specs/0056-mirror-and-negative-scale/` criteria 1 to 3):
 * two icon buttons, shown in every selection state and disabled only while nothing is
 * selected, and the polite announcement of a flip made by a button or by H or V.
 */
export function FlipGroup({
  enabled,
  flip,
  onReturnFocus,
}: {
  enabled: boolean;
  flip: FlipApi;
  onReturnFocus: () => void;
}) {
  // The region lives in the Select bar: empty it when the bar goes, so that coming back to
  // the Select tool does not re-create it holding the last sentence.
  const { clearAnnouncement } = flip;
  useEffect(() => clearAnnouncement, [clearAnnouncement]);
  return (
    <Tooltip.Provider delayDuration={400}>
      <div role="group" aria-label="Flip" className="flex items-center gap-1 py-1">
        <FlipButton axis="horizontal" enabled={enabled} flip={flip} onReturnFocus={onReturnFocus} />
        <FlipButton axis="vertical" enabled={enabled} flip={flip} onReturnFocus={onReturnFocus} />
        <div role="status" aria-live="polite" className="sr-only">
          {/* Keyed on the counter, so the same sentence twice is spoken twice. */}
          <span key={flip.announcement.id}>{flip.announcement.text}</span>
        </div>
      </div>
    </Tooltip.Provider>
  );
}
