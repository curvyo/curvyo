import { useEffect, useState } from "react";

import type { ManufacturingView } from "@/hooks/useWorkbenches";

/** How long the notice stays (UX notes §5). */
const NOTICE_MS = 2000;

/**
 * The read-only notice of a manufacturing workbench (`specs/0093` criterion 7, UX notes §5):
 * text only, no controls, no pointer events; 12 px up and right of the press, or at the canvas's
 * bottom centre for a key; 2 s, a new one restarts it; announced once, politely.
 */
export function ReadOnlyNotice({ notice }: { notice: ManufacturingView["read_only_notice"] }) {
  const seq = notice?.seq ?? null;
  // A notice already there when the workbench opens is an old one: it is not shown again.
  const [mountSeq] = useState(seq);
  const [expired, setExpired] = useState<number | null>(null);
  useEffect(() => {
    if (seq === null) return;
    const timer = window.setTimeout(() => setExpired(seq), NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [seq]);
  const visible = notice !== null && notice.seq !== mountSeq && notice.seq !== expired;
  const atPointer = notice?.x !== null && notice?.x !== undefined && notice.y !== null;
  return (
    <>
      <div aria-live="polite" className="sr-only">
        {visible ? notice.text : ""}
      </div>
      {visible ? (
        <div
          aria-hidden
          className="pointer-events-none absolute z-40 rounded-md px-2 py-1 text-xs text-[var(--toolbar-icon)]"
          style={{
            background: "var(--toolbar-bg)",
            boxShadow: "var(--panel-elevation-shadow)",
            ...(atPointer
              ? { left: (notice.x ?? 0) + 12, top: (notice.y ?? 0) - 12 - 24 }
              : { left: "50%", bottom: 16, transform: "translateX(-50%)" }),
          }}
        >
          {notice.text}
        </div>
      ) : null}
    </>
  );
}
