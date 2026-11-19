import { CirclePlay, LoaderCircle } from "lucide-react";
import { useEffect, useRef } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { ARM_TRASH, ARM_ZONE, useDeleteArm } from "@/hooks/useDeleteArm";
import { armEnds, ignoredKey, sendPress } from "@/lib/sendArm";
import { SEND_ARMED_NAME, sendName } from "@/lib/sendText";

/** The arm target of the page's button; one armed action in the app at a time. */
const ARM_ID = "send-to-machine";

/**
 * Send to machine (`specs/0125-send-to-machine` UX notes §2, §3; `specs/0134-send-and-start` UX
 * notes sections 2 to 8): a secondary symbol button next to Export with the `CirclePlay` glyph.
 * When the next press will start, it is an armed press: the first press arms (accent fill, white
 * glyph, the armed name, a polite status), a press inside the 400 ms guard is ignored, the second
 * sends and starts; it disarms silently after 4 s, on Escape, a press elsewhere, focus leaving, or
 * when the start or the job changes. Both presses read a fresh status first (P4). Disabled it
 * keeps its Tab stop (`aria-disabled`) and names its cause; while it sends and starts it turns the
 * `LoaderCircle`.
 */
export function SendToMachine({
  enabled,
  cause,
  sending,
  starting,
  starts,
  job,
  onSend,
  willStart,
  onReturnFocus,
}: {
  enabled: boolean;
  cause: string | null;
  /** The session's send runs. */
  sending: boolean;
  /** A send that asked for a start runs, until the connector's answer to the start. */
  starting: boolean;
  /** The view's will-it-start rule over the last status. */
  starts: boolean;
  /** Changes when the job a press would send changes. */
  job: number;
  onSend: (start: boolean) => Promise<void>;
  /** The rule over a fresh status, read at each press. */
  willStart: () => boolean;
  onReturnFocus: () => void;
}) {
  const arm = useDeleteArm();
  const armed = arm.armed === ARM_ID;
  const armedJob = useRef(job);
  const busy = sending || starting;
  const pressable = enabled && !busy;
  const { disarm } = arm;
  // The name is no longer "start", the button turned busy or disabled, or the job changed (P15).
  useEffect(() => {
    if (armEnds(armed, starts, pressable, job, armedJob.current)) disarm();
  }, [armed, starts, pressable, job, disarm]);

  const name = sendName(starts || starting, armed);
  const line2 = sending ? cause : starting ? "Starting" : cause;
  const describedBy = "output-send-reason";
  const tip = pressable ? (
    name
  ) : (
    <>
      <div>{name}</div>
      <div className="text-muted-foreground">{line2}</div>
    </>
  );
  const press = (pointer: boolean) => {
    const facts = { pressable, starts, fresh: willStart };
    if (!armed) {
      const action = sendPress({ ...facts, armed: false });
      if (action === "send") {
        void onSend(false);
        if (pointer) onReturnFocus();
      } else if (action === "arm") {
        // The arming press keeps focus (P9) and sends nothing.
        armedJob.current = job;
        arm.press(ARM_ID, SEND_ARMED_NAME, () => "");
      }
      return;
    }
    // Inside the 400 ms guard `arm.press` ignores the press; after it the confirm runs.
    arm.press(ARM_ID, SEND_ARMED_NAME, () => {
      const action = sendPress({ ...facts, armed: true });
      if (action !== "ignore") void onSend(action === "sendAndStart");
      if (pointer) onReturnFocus();
      return "";
    });
  };
  return (
    <>
      <Tooltip side="top" content={tip}>
        <button
          type="button"
          data-send-job
          data-armed={armed ? true : undefined}
          {...{ [ARM_TRASH]: ARM_ID, [ARM_ZONE]: ARM_ID }}
          aria-label={name}
          aria-disabled={pressable ? undefined : true}
          aria-busy={busy ? true : undefined}
          aria-describedby={pressable ? undefined : describedBy}
          onKeyDown={(event) => {
            // P14: a held key never arms or confirms.
            if (ignoredKey(event.key, event.repeat)) event.preventDefault();
          }}
          onClick={(event) => press(event.detail > 0)}
          className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 ${
            !pressable
              ? "cursor-default bg-[var(--field-disabled-bg)] text-[var(--field-disabled-fg)]"
              : armed
                ? "border border-[var(--armed-accent)] bg-[var(--armed-accent)] text-[var(--toolbar-icon-active-fg)]"
                : "border border-[var(--field-border)] bg-[var(--panel-bg)] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
          }`}
        >
          {busy ? (
            <LoaderCircle size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="animate-spin" />
          ) : (
            <CirclePlay size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          )}
        </button>
      </Tooltip>
      <span role="status" className="sr-only">
        {arm.status}
      </span>
      {pressable ? null : (
        <span id={describedBy} className="sr-only">
          {line2}
        </span>
      )}
    </>
  );
}
