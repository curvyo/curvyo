import { useEffect, useState } from "react";
import type { WasmSession } from "@/lib/editorSession";

/** One text over the canvas as the session places it (canvas CSS px). */
interface FixtureLabel {
  kind: "tape" | "fixture" | "slot" | "tag";
  text: string;
  x: number;
  y: number;
  /** The edge of a tape number, when the session says it. */
  edge?: "bottom" | "left" | "top" | "right";
}

const KINDS = new Set(["tape", "fixture", "slot", "tag"]);
const EDGES = new Set(["bottom", "left", "top", "right"]);

function isLabel(value: unknown): value is FixtureLabel {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.kind === "string" &&
    KINDS.has(v.kind) &&
    typeof v.text === "string" &&
    typeof v.x === "number" &&
    typeof v.y === "number" &&
    (v.edge === undefined || (typeof v.edge === "string" && EDGES.has(v.edge)))
  );
}

/** The labels of the session's `fixture_labels_json`; none for anything else. */
function readFixtureLabels(text: string): FixtureLabel[] {
  try {
    const parsed: unknown = JSON.parse(text);
    return Array.isArray(parsed) ? parsed.filter(isLabel) : [];
  } catch {
    return [];
  }
}

/** Where the text sits against its anchor: tape numbers lie on the inner side of their band. */
function placement(label: FixtureLabel): { transform: string; color: string } {
  if (label.kind === "tape") {
    const color = "var(--ruler-label)";
    switch (label.edge) {
      case "bottom":
        return { transform: "translate(2px, calc(-100% - 1px))", color };
      case "top":
        return { transform: "translate(2px, 1px)", color };
      case "left":
        return { transform: "translate(2px, -50%)", color };
      case "right":
        return { transform: "translate(calc(-100% - 2px), -50%)", color };
      default:
        return { transform: "translate(-50%, -50%)", color };
    }
  }
  const color = "var(--toolbar-icon)";
  // The selected fixture's tag stands above its box's corner (`ux-notes.md` section 6.3).
  if (label.kind === "tag") return { transform: "translate(0, calc(-100% - 4px))", color };
  // A board or clamp name stands inside the top-left corner; a slot name is centred.
  return label.kind === "fixture"
    ? { transform: "translate(4px, 4px)", color }
    : { transform: "translate(-50%, -50%)", color };
}

/**
 * The texts of the bed's fixtures (`specs/0102-laser-fixtures` UX notes sections 5.1 and 5.2):
 * the tape numbers of the grid, the name of a board or clamp, the name of a slot. The session
 * decides which ones fit and where; this only draws them, 11 px, `aria-hidden`, never hit.
 */
export function FixtureLabels({ getSession }: { getSession: () => WasmSession | null }) {
  const [json, setJson] = useState("[]");
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const next = getSession()?.fixture_labels_json() ?? "[]";
      if (next !== previous) {
        previous = next;
        setJson(next);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  const labels = readFixtureLabels(json);
  if (labels.length === 0) return null;
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden" data-fixture-labels="">
      {labels.map((label, index) => {
        const { transform, color } = placement(label);
        // The tag is a chip: 20 px high, toolbar ground, a 25 % border (`ux-notes.md` section 6.3).
        const chip = label.kind === "tag";
        return (
          <span
            key={`${label.kind}-${index}`}
            className={
              chip
                ? "absolute flex h-5 items-center whitespace-nowrap rounded-lg border px-1.5 text-xs"
                : "absolute whitespace-nowrap text-[11px] leading-none tabular-nums"
            }
            style={{
              left: label.x,
              top: label.y,
              transform,
              color,
              ...(chip
                ? {
                    background: "var(--toolbar-bg)",
                    borderColor: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
                  }
                : {}),
              // A name over a stop or a clamp's hatch keeps its own light edge.
              textShadow:
                label.kind === "tape" || chip
                  ? undefined
                  : "0 0 2px var(--toolbar-bg), 0 0 2px var(--toolbar-bg)",
            }}
          >
            {label.text}
          </span>
        );
      })}
    </div>
  );
}
