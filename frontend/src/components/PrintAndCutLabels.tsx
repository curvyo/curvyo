import { useEffect, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { type PrintAndCutLabel, readPrintAndCutLabels } from "@/lib/printAndCutView";

/** The 2px `--dim-halo` around the tags, as the dimension text has it. */
const HALO = "0 0 2px var(--dim-halo), 0 0 2px var(--dim-halo), 0 0 2px var(--dim-halo)";

/** Where a tag sits against its anchor and in which colour (`specs/0121` UX notes §5). */
function look(label: PrintAndCutLabel): { transform: string; color: string; size: string } {
  switch (label.kind) {
    // The mark's number up and right of its centre, guide green.
    case "mark":
      return { transform: "translate(6px, calc(-100% - 6px))", color: "var(--guide)", size: "11px" };
    // The ring's number down and left, the other side from the mark's.
    case "ring":
      return { transform: "translate(calc(-100% - 7px), 7px)", color: "var(--editor-accent)", size: "11px" };
    // "Fit" at the fitted frame's top-left, above it.
    default:
      return { transform: "translate(0, calc(-100% - 2px))", color: "var(--editor-accent)", size: "12px" };
  }
}

/**
 * The number tags of the faint crosses and the measured rings and the "Fit" tag of the dashed
 * fit frame over a laser page (`specs/0121-print-and-cut` UX notes §5). The session places them;
 * this only draws them, `aria-hidden`, never hit.
 */
export function PrintAndCutLabels({ getSession }: { getSession: () => WasmSession | null }) {
  const [json, setJson] = useState("[]");
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const next = getSession()?.print_and_cut_labels_json() ?? "[]";
      if (next !== previous) {
        previous = next;
        setJson(next);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  const labels = readPrintAndCutLabels(json);
  if (labels.length === 0) return null;
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden" data-print-and-cut-labels="">
      {labels.map((label, index) => {
        const { transform, color, size } = look(label);
        return (
          <span
            key={`${label.kind}-${index}`}
            className="absolute whitespace-nowrap leading-none tabular-nums"
            style={{ left: label.x, top: label.y, transform, color, fontSize: size, textShadow: HALO }}
          >
            {label.text}
          </span>
        );
      })}
    </div>
  );
}
