import { useEffect, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";
import { readMatLabels } from "@/lib/matLabels";

/**
 * The printing of a plotter page's mat (`specs/0159-cutting-plotter` criterion 5, UX notes 5.2):
 * the cm and inch numbers and the grip word in `--mat-print`, at the mat's scale, some turned 180
 * degrees as printed. The session decides which show; this only draws them, `aria-hidden`,
 * never hit.
 */
export function MatLabels({ getSession }: { getSession: () => WasmSession | null }) {
  const [json, setJson] = useState("[]");
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const next = getSession()?.plotter_mat_labels_json() ?? "[]";
      if (next !== previous) {
        previous = next;
        setJson(next);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  const labels = readMatLabels(json);
  if (labels.length === 0) return null;
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden" data-mat-labels="">
      {labels.map((label, index) => (
        <span
          key={`${label.text}-${index}`}
          className="absolute whitespace-nowrap leading-none tabular-nums"
          style={{
            left: label.x,
            top: label.y,
            fontSize: label.size,
            color: "var(--mat-print)",
            transform: `translate(-50%, -50%)${label.turned ? " rotate(180deg)" : ""}`,
          }}
        >
          {label.text}
        </span>
      ))}
    </div>
  );
}
