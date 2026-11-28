import { useEffect, useState } from "react";

import { PickGlyph } from "@/components/panel/PickGlyph";
import type { WasmSession } from "@/lib/editorSession";
import { chipGlyphs, chipWidth, decodePickMarks, frameOffset } from "@/lib/pickOverlay";
import { PICK_COLOUR } from "@/lib/picks";

/**
 * The picks of the selected test grid drawn over the canvas (`specs/0146` criterion 8, `ux-notes.md`
 * 6): a 2 px frame in the first pick's colour around each marked cell, Second best dashed, with a
 * white casing on both sides so it reads on any page ground, and a small chip of the cell's glyphs.
 * View only: never hit, never output, never in the accessibility tree (the table is that route).
 */
export function PickOverlay({ getSession }: { getSession: () => WasmSession | null }) {
  const [json, setJson] = useState("[]");
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const next = getSession()?.pick_marks_json() ?? "[]";
      if (next !== previous) {
        previous = next;
        setJson(next);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  const marks = decodePickMarks(json);
  if (marks.length === 0) return null;
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden" data-pick-overlay="">
      {marks.map((mark, index) => {
        const first = mark.kinds[0];
        if (first === undefined) return null;
        const offset = frameOffset(mark.gap);
        const reach = offset + 2;
        const shown = chipGlyphs(mark.kinds.length, mark.pitch);
        return (
          <div key={index}>
            <span
              data-pick-frame={first}
              className="absolute box-border"
              style={{
                left: mark.x - reach,
                top: mark.y - reach,
                width: mark.width + 2 * reach,
                height: mark.height + 2 * reach,
                border: `2px ${first === "second_best" ? "dashed" : "solid"} ${PICK_COLOUR[first]}`,
                boxShadow: "0 0 0 1px #fff, inset 0 0 0 1px #fff",
              }}
            />
            {shown > 0 ? (
              <span
                data-pick-chip=""
                className="absolute flex h-4 items-center justify-center gap-0.5 rounded-[8px] border px-0.5"
                style={{
                  left: mark.x + mark.width / 2 - chipWidth(shown) / 2,
                  top: mark.y + mark.height / 2 - 8,
                  width: chipWidth(shown),
                  background: "var(--toolbar-bg)",
                  borderColor: "var(--lane-line)",
                  color: "var(--toolbar-icon)",
                }}
              >
                {mark.kinds.slice(0, shown).map((kind) => (
                  <PickGlyph key={kind} kind={kind} size={12} />
                ))}
              </span>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}
