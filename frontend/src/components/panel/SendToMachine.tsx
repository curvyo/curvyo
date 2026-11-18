import { LoaderCircle, Upload } from "lucide-react";

import { Tooltip } from "@/components/ui/tooltip";
import { SEND_NAME } from "@/lib/sendText";

/**
 * Send to machine (`specs/0125-send-to-machine` UX notes §2, §3): a secondary symbol button next to
 * Export. Disabled it keeps its Tab stop (`aria-disabled`), shows Export's disabled look and names
 * its cause as the tooltip's second line and as its description; while a send runs it turns the
 * `LoaderCircle` once a second.
 */
export function SendToMachine({
  enabled,
  cause,
  sending,
  onPress,
  onReturnFocus,
}: {
  enabled: boolean;
  cause: string | null;
  sending: boolean;
  onPress: () => Promise<void>;
  onReturnFocus: () => void;
}) {
  const describedBy = "output-send-reason";
  const tip =
    cause === null ? null : (
      <>
        <div>{SEND_NAME}</div>
        <div className="text-muted-foreground">{cause}</div>
      </>
    );
  return (
    <>
      <Tooltip side="top" content={enabled ? SEND_NAME : tip}>
        <button
          type="button"
          data-send-job
          aria-label={SEND_NAME}
          aria-disabled={enabled ? undefined : true}
          aria-busy={sending ? true : undefined}
          aria-describedby={enabled ? undefined : describedBy}
          onClick={(event) => {
            if (!enabled) return;
            void onPress();
            if (event.detail > 0) onReturnFocus();
          }}
          className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 ${
            enabled
              ? "border border-[var(--field-border)] bg-[var(--panel-bg)] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
              : "cursor-default bg-[var(--field-disabled-bg)] text-[var(--field-disabled-fg)]"
          }`}
        >
          {sending ? (
            <LoaderCircle size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="animate-spin" />
          ) : (
            <Upload size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          )}
        </button>
      </Tooltip>
      {enabled ? null : (
        <span id={describedBy} className="sr-only">
          {cause}
        </span>
      )}
    </>
  );
}
