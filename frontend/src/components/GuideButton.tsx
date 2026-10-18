import { SquareDashed } from "lucide-react";
import { Tooltip } from "radix-ui";

import { GUIDE_NAME, type GuideState } from "@/lib/guideText";

const BUTTON_CLASS =
  "flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)]";

/**
 * The Guide button of the Select bar (`specs/0046-document-shapes` criteria 9, 10, 14 and 37):
 * one icon button after the Flip group, always in the bar. Greyed while nothing selected can be
 * a guide, pressed while everything that can is one. The tooltip is its name; the decision and
 * the write are the session's.
 */
export function GuideButton({
  state,
  onToggle,
  onReturnFocus,
}: {
  state: GuideState;
  onToggle: () => void;
  onReturnFocus: () => void;
}) {
  const enabled = state !== "disabled";
  return (
    <Tooltip.Provider delayDuration={400}>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <button
            type="button"
            aria-label={GUIDE_NAME}
            aria-pressed={state === "pressed"}
            // Not the `disabled` attribute: the button keeps its tooltip and its Tab stop.
            aria-disabled={!enabled}
            data-select-command="guide"
            onClick={(event) => {
              if (!enabled) {
                return;
              }
              onToggle();
              // `detail` is 0 for Space and Enter, 1 or more for a click.
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
            className={`${BUTTON_CLASS} ${
              enabled ? "hover:bg-[var(--editor-accent-hover)] aria-pressed:hover:bg-[var(--toolbar-icon-active-bg)]" : "opacity-40"
            }`}
          >
            <SquareDashed aria-hidden size={16} strokeWidth={1.5} absoluteStrokeWidth />
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
            {GUIDE_NAME}
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  );
}
