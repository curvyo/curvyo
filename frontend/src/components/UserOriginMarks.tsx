import { useEffect, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { decodeUserOriginMarks, type UserOriginMarks as Marks } from "@/lib/outputView";

/**
 * The user origin on the laser canvas (`specs/0118` criterion 32, UX notes W46, W47): a 4 px square
 * with four 6 px arms on the point, ink over a casing, and the dashed box where the job burns
 * while it is placed away from the drawing. View only: `aria-hidden`, no pointer events; read once
 * per frame.
 */
export function UserOriginMarks({ getSession }: { getSession: () => WasmSession | null }) {
  const [marks, setMarks] = useState<Marks>({ cross: null, box: null });
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const text = getSession()?.user_origin_marks_json() ?? "";
      if (text !== previous) {
        previous = text;
        setMarks(decodeUserOriginMarks(text));
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  if (marks.cross === null && marks.box === null) return null;
  return (
    <svg aria-hidden data-user-origin-marks className="pointer-events-none absolute inset-0 size-full overflow-visible">
      {marks.box === null ? null : (
        <rect
          x={marks.box[0] + 0.5}
          y={marks.box[1] + 0.5}
          width={Math.max(marks.box[2] - 1, 0)}
          height={Math.max(marks.box[3] - 1, 0)}
          fill="none"
          stroke="var(--toolbar-icon)"
          strokeWidth={1}
          strokeDasharray="4 4"
        />
      )}
      {marks.cross === null ? null : <Cross x={marks.cross[0]} y={marks.cross[1]} />}
    </svg>
  );
}

function Cross({ x, y }: { x: number; y: number }) {
  const arms = `M${x - 8} ${y}h6M${x + 2} ${y}h6M${x} ${y - 8}v6M${x} ${y + 2}v6`;
  return (
    <g data-user-origin-cross>
      <path d={arms} stroke="var(--selection-casing)" strokeWidth={4} strokeLinecap="square" fill="none" />
      <rect x={x - 3} y={y - 3} width={6} height={6} fill="var(--selection-casing)" />
      <path d={arms} stroke="var(--toolbar-icon)" strokeWidth={2} fill="none" />
      <rect x={x - 2} y={y - 2} width={4} height={4} fill="var(--toolbar-icon)" />
    </g>
  );
}
