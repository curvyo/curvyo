// The refusal of a drop onto a filled slot with its one control, the armed Replace
// (`specs/0154-library-into-slots` criterion 7, UX notes section 7, V10): an alert in the canvas
// notice slot for 8 s, held while hovered or focused; Escape or any new drop closes it.
import { useEffect, useRef, useState } from "react";

import { useArm } from "@/hooks/useArm";
import { setSlotReplace, useSlotReplace } from "@/lib/intoSlot";
import { Button } from "@/components/ui/button";
import { Replace } from "lucide-react";

const LIFETIME_MS = 8000;

export function SlotReplaceNotice() {
  const shown = useSlotReplace();
  const arm = useArm("slot-replace");
  const [held, setHeld] = useState(false);
  const timer = useRef<number | null>(null);
  useEffect(() => {
    if (!shown || held) {
      return;
    }
    timer.current = window.setTimeout(() => setSlotReplace(null), LIFETIME_MS);
    return () => {
      if (timer.current !== null) {
        window.clearTimeout(timer.current);
      }
    };
  }, [shown, held]);
  if (!shown) {
    return null;
  }
  const name = shown.text.replace(/ is filled\.$/, "");
  return (
    <div
      role="alert"
      className="slot-replace-notice"
      {...arm.zone}
      onPointerEnter={() => setHeld(true)}
      onPointerLeave={() => setHeld(false)}
      onFocus={() => setHeld(true)}
      onBlur={() => setHeld(false)}
      onKeyDown={(event) => {
        if (event.key === "Escape" && !arm.armed) {
          setSlotReplace(null);
        }
      }}
    >
      <span>{shown.text}</span>
      <Button icon={Replace}
        size="bar"
        className={arm.armed ? "h-6 border-[var(--destructive)] bg-[var(--destructive)] text-white" : "h-6"}
        aria-label={`Replace the content of ${name}`}
        {...arm.control}
        onClick={() =>
          arm.press(() => {
            shown.replace();
            return "";
          })
        }
      >
        Replace
      </Button>
      <span className="sr-only" aria-live="polite">
        {arm.armed ? `Press again to replace the content of ${name}.` : ""}
      </span>
    </div>
  );
}
