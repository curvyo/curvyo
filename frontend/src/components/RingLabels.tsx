import { useEffect, useState } from "react";

import { FixtureRing } from "@/components/panel/FixtureOutputPart";
import type { WasmSession } from "@/lib/editorSession";
import { decodeRingLabelsText, type RingLabel } from "@/lib/outputView";

/**
 * The ring labels of the Toolpath view (`specs/0029-laser-job-preview-and-output` UX notes 4): the
 * canvas draws the ring, tick and arrowheads, the host writes into a 17 px circle (white ground,
 * 1.5 px ink ring) the row's digit, or the fixture glyph for a fixture job. `aria-hidden`, no
 * pointer events; read once per frame while the view is pressed.
 */
export function RingLabels({ getSession }: { getSession: () => WasmSession | null }) {
  const [rings, setRings] = useState<RingLabel[]>([]);
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const text = getSession()?.toolpath_ring_labels_json() ?? "[]";
      if (text !== previous) {
        previous = text;
        setRings(decodeRingLabelsText(text));
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  return (
    <>
      {rings.map((ring, index) =>
        ring.digit === null ? (
          <span
            key={index}
            data-ring-label="fixture"
            className="pointer-events-none absolute"
            style={{ left: ring.x - 8.5, top: ring.y - 8.5 }}
          >
            <FixtureRing size={17} />
          </span>
        ) : (
          <span
            key={index}
            aria-hidden
            data-ring-label={ring.digit}
            className="pointer-events-none absolute flex size-[17px] items-center justify-center rounded-full border-[1.5px] border-[var(--toolbar-icon)] bg-white text-[10px] font-semibold leading-none text-[var(--toolbar-icon)] tabular-nums"
            style={{ left: ring.x - 8.5, top: ring.y - 8.5 }}
          >
            {ring.digit}
          </span>
        ),
      )}
    </>
  );
}
