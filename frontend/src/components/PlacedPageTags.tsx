import { Frame } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { WasmSession } from "@/lib/editorSession";

/** One placed page's name tag as the session places it (canvas CSS px). */
interface PlacedTag {
  id: string;
  name: string;
  x: number;
  y: number;
  width: number;
  selected: boolean;
}

function isTag(value: unknown): value is PlacedTag {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.id === "string" &&
    typeof v.name === "string" &&
    typeof v.x === "number" &&
    typeof v.y === "number" &&
    typeof v.width === "number" &&
    typeof v.selected === "boolean"
  );
}

/** The tags of the session's `placed_pages_json`; none for anything else. */
function readPlacedTags(text: string): PlacedTag[] {
  try {
    const parsed: unknown = JSON.parse(text);
    return Array.isArray(parsed) ? parsed.filter(isTag) : [];
  } catch {
    return [];
  }
}

/** Where a tag stands apart from its base place so it never covers a page's own label. */
interface TagShift {
  dx: number;
  dy: number;
}

/** Clear space kept between two labels, px. */
const LABEL_GAP = 4;

/**
 * The name tag of every placed page (`specs/0100-bring-content-in` UX notes §5): the `Frame`
 * glyph and the source page's name, upright above the top-left of the frame's screen bounds,
 * 6 px clear, semibold while selected, never wider than the frame, gone below 24 px. A tag that
 * would cover a page's own label (a placed page flush with its laser page's top-left corner)
 * moves right of that label when the whole name still fits the frame's width, else above it.
 */
export function PlacedPageTags({ getSession }: { getSession: () => WasmSession | null }) {
  const [json, setJson] = useState("[]");
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const next = getSession()?.placed_pages_json() ?? "[]";
      if (next !== previous) {
        previous = next;
        setJson(next);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  const root = useRef<HTMLDivElement>(null);
  const [shifts, setShifts] = useState<Record<string, TagShift>>({});
  const tags = readPlacedTags(json).filter((tag) => tag.width >= 24);
  useLayoutEffect(() => {
    const container = root.current;
    if (!container) return;
    const origin = container.getBoundingClientRect();
    const labels = [...document.querySelectorAll<HTMLElement>("[data-page-label]")].map((el) => {
      const r = el.getBoundingClientRect();
      return { l: r.left - origin.left, r: r.right - origin.left, t: r.top - origin.top, b: r.bottom - origin.top };
    });
    const next: Record<string, TagShift> = {};
    for (const tag of tags) {
      const el = container.querySelector<HTMLElement>(`[data-tag="${CSS.escape(tag.id)}"]`);
      const name = el?.lastElementChild;
      if (!el || !name) continue;
      const natural = 16 + name.scrollWidth;
      const width = Math.min(tag.width, natural);
      const box = { l: tag.x, r: tag.x + width, t: tag.y - 6 - el.offsetHeight, b: tag.y - 6 };
      const hit = labels.find(
        (m) => box.l < m.r + LABEL_GAP && m.l < box.r + LABEL_GAP && box.t < m.b + LABEL_GAP && m.t < box.b + LABEL_GAP,
      );
      if (!hit) continue;
      const dx = hit.r + 8 - tag.x;
      next[tag.id] = tag.width - dx >= natural ? { dx, dy: 0 } : { dx: 0, dy: -(hit.b - hit.t + LABEL_GAP) };
    }
    setShifts((previous) => {
      const same =
        Object.keys(next).length === Object.keys(previous).length &&
        Object.entries(next).every(([id, s]) => previous[id]?.dx === s.dx && previous[id]?.dy === s.dy);
      return same ? previous : next;
    });
  }, [tags]);
  return (
    <div
      ref={root}
      aria-hidden
      className="pointer-events-none absolute inset-0 overflow-hidden"
      data-placed-tags=""
    >
      {tags.map((tag) => {
        const shift = shifts[tag.id] ?? { dx: 0, dy: 0 };
        return (
          <span
            key={tag.id}
            data-tag={tag.id}
            className="absolute flex items-center gap-1 whitespace-nowrap text-xs"
            style={{
              left: tag.x + shift.dx,
              top: tag.y - 6 + shift.dy,
              maxWidth: tag.width - shift.dx,
              transform: "translateY(-100%)",
              color: "var(--toolbar-icon)",
            }}
          >
            <Frame size={12} className="shrink-0" />
            <span className={`min-w-0 truncate ${tag.selected ? "font-semibold" : ""}`}>{tag.name}</span>
          </span>
        );
      })}
    </div>
  );
}
