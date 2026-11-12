import { ListTree } from "lucide-react";
import { useEffect, useState } from "react";
import type { WasmSession } from "@/lib/editorSession";
import { OPERATIONS_VIEW_NAME, operationsToggle, readPages } from "@/lib/pages";
import { Tooltip } from "@/components/ui/tooltip";
import { KindChip } from "@/components/KindChip";
/** Screen-sized page chrome above the un-clipped drawing. Geometry comes from Rust. */
export function PageFrames({
  getSession,
  dropTarget = null,
  onToggleOperationsView,
}: {
  getSession: () => WasmSession | null;
  /** The laser page a canvas drag of objects is over (its badge is no refusal): frame cue. */
  dropTarget?: string | null;
  /** The Operations view toggle of a laser page's label (`specs/0111` criterion 15). */
  onToggleOperationsView?: (page: string) => void;
}) {
  const [json, setJson] = useState("[]");
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const next = getSession()?.pages_json() ?? "[]";
      if (next !== previous) {
        previous = next;
        setJson(next);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  return (
    <div
      className="pointer-events-none absolute inset-0 overflow-hidden"
      data-page-frames=""
    >
      {readPages(json).map((page) => (
        <div
          key={page.id}
          className="absolute"
          style={{
            left: page.screenX,
            top: page.screenY,
            width: page.screenWidth,
            height: page.screenHeight,
            outline: `${page.active ? 2 : 1}px solid var(--toolbar-icon)`,
            outlineOffset: -1,
            outlineColor: page.active
              ? "var(--toolbar-icon)"
              : "color-mix(in srgb, var(--toolbar-icon) 65%, transparent)",
          }}
        >
          {/* The frame cue of a page-row drop (`specs/0099` UX notes §2): 2px outside. */}
          {(page.dropTarget || page.id === dropTarget) && (
            <div
              data-drop-target=""
              aria-hidden
              className="absolute"
              style={{ inset: -4, outline: "2px solid var(--editor-accent)" }}
            />
          )}
          {/* The label is never wider than its frame: the name truncates first, the word goes
              below 96 px, the whole label below 24 px (`specs/0098` UX notes §5). */}
          {page.screenWidth >= 24 && (
            <span
              data-page-label=""
              className="absolute bottom-full left-0 mb-1 flex max-w-full items-center gap-1.5 whitespace-nowrap text-xs"
              style={{ color: "var(--toolbar-icon)" }}
            >
              <span aria-hidden className={`min-w-0 truncate ${page.active || page.home ? "font-semibold" : ""}`}>
                {page.name}
              </span>
              <span aria-hidden className="flex shrink-0 items-center">
                <KindChip
                  size={12}
                  kind={{
                    icon: page.kindIcon,
                    word: page.screenWidth >= 96 ? page.kindWord : "",
                    off: page.kindOff,
                  }}
                />
              </span>
              {operationsToggle(page) && onToggleOperationsView ? (
                <OperationsViewToggle
                  pressed={operationsToggle(page)?.pressed ?? true}
                  onPress={() => onToggleOperationsView(page.id)}
                />
              ) : null}
            </span>
          )}
        </div>
      ))}
    </div>
  );
}

/**
 * The Operations view toggle after the Laser chip (`specs/0111-laser-operation-tools`
 * `ux-notes.md` 5): a 20 px `ListTree` button in a 24 px hit area, pressed by default (the
 * pressed look of the "Link corners toggle"), a thin border when unpressed so it still reads as a
 * button over the pasteboard. Per page and session only; no step, no notice.
 */
function OperationsViewToggle({ pressed, onPress }: { pressed: boolean; onPress: () => void }) {
  return (
    <Tooltip side="bottom" content={OPERATIONS_VIEW_NAME}>
      <button
        type="button"
        aria-pressed={pressed}
        aria-label={OPERATIONS_VIEW_NAME}
        onClick={onPress}
        className="pointer-events-auto -my-0.5 flex size-6 shrink-0 items-center justify-center rounded-[7px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        <span
          className={`flex size-5 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] ${
            pressed
              ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
              : "border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] hover:bg-[var(--editor-accent-hover)]"
          }`}
        >
          <ListTree aria-hidden size={14} strokeWidth={1.5} absoluteStrokeWidth />
        </span>
      </button>
    </Tooltip>
  );
}
