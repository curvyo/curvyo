import { ListTree, Route } from "lucide-react";
import { useEffect, useLayoutEffect, useState } from "react";
import { useBarBox } from "@/hooks/useBarBox";
import type { WasmSession } from "@/lib/editorSession";
import { OPERATIONS_VIEW_NAME, TOOLPATH_VIEW_NAME, operationsToggle, pageLabelTop, readPages, toolpathToggle } from "@/lib/pages";
import { PlayBar } from "@/components/PlayBar";
import { RingLabels } from "@/components/RingLabels";
import { UserOriginMarks } from "@/components/UserOriginMarks";
import { Tooltip } from "@/components/ui/tooltip";
import { KindChip } from "@/components/KindChip";
import { pagesTick, perfPagesCommitted, perfPagesTick, perfRender, perfStart } from "@/lib/perfProbe";
/** Screen-sized page chrome above the un-clipped drawing. Geometry comes from Rust. */
export function PageFrames({
  getSession,
  dropTarget = null,
  onToggleOperationsView,
  onToggleToolpathView,
}: {
  getSession: () => WasmSession | null;
  /** The laser page a canvas drag of objects is over (its badge is no refusal): frame cue. */
  dropTarget?: string | null;
  /** The Operations view toggle of a laser page's label (`specs/0111` criterion 15). */
  onToggleOperationsView?: (page: string) => void;
  /** The Toolpath view toggle of a laser page's label (`specs/0029` criterion 8). */
  onToggleToolpathView?: (page: string) => void;
}) {
  perfRender("PageFrames");
  const [json, setJson] = useState("[]");
  useLayoutEffect(() => perfPagesCommitted(), [json]);
  useEffect(() => {
    let frame = 0;
    let previous = "";
    const tick = () => {
      const probeStart = perfStart();
      perfPagesTick(true);
      const next = getSession()?.pages_json() ?? "[]";
      const changed = next !== previous;
      if (changed) {
        previous = next;
        setJson(next);
      }
      perfPagesTick(false);
      pagesTick(probeStart, changed);
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  const pages = readPages(json);
  const [root, bars] = useBarBox();
  // The play bar and the rings belong to the active page while its Toolpath view is pressed.
  const route = pages.find((page) => page.active && page.toolpathView === true);
  return (
    <div
      ref={root}
      className="pointer-events-none absolute inset-0 overflow-hidden"
      data-page-frames=""
    >
      {pages.map((page) => (
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
              below 96 px, the whole label below 24 px (`specs/0098` UX notes §5). It is never under a
              bar: below the bars where they cover its place (0164 F50). */}
          {page.screenWidth >= 24 && (
            <span
              data-page-label=""
              className="absolute left-0 flex h-4 max-w-full items-center gap-1.5 whitespace-nowrap text-xs"
              style={{ color: "var(--toolbar-icon)", top: pageLabelTop(page, bars) }}
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
              {toolpathToggle(page) && onToggleToolpathView ? (
                <ToolpathViewToggle
                  pressed={toolpathToggle(page)?.pressed ?? false}
                  onPress={() => onToggleToolpathView(page.id)}
                />
              ) : null}
            </span>
          )}
        </div>
      ))}
      <UserOriginMarks getSession={getSession} />
      {route ? (
        <>
          <RingLabels getSession={getSession} />
          <PlayBar
            getSession={getSession}
            frame={{
              left: route.screenX,
              bottom: route.screenY + route.screenHeight,
              width: route.screenWidth,
            }}
          />
        </>
      ) : null}
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
  return <ViewToggle name={OPERATIONS_VIEW_NAME} Glyph={ListTree} pressed={pressed} onPress={onPress} />;
}

/**
 * The Toolpath view toggle after the Operations view toggle (`specs/0029-laser-job-preview-and-output`
 * `ux-notes.md` 3): the same look, `Route`, unpressed by default; pressing it releases the
 * Operations view in the session (the pair is exclusive).
 */
function ToolpathViewToggle({ pressed, onPress }: { pressed: boolean; onPress: () => void }) {
  return <ViewToggle name={TOOLPATH_VIEW_NAME} Glyph={Route} pressed={pressed} onPress={onPress} />;
}

function ViewToggle({
  name,
  Glyph,
  pressed,
  onPress,
}: {
  name: string;
  Glyph: typeof ListTree;
  pressed: boolean;
  onPress: () => void;
}) {
  return (
    <Tooltip side="bottom" content={name}>
      <button
        type="button"
        aria-pressed={pressed}
        aria-label={name}
        onClick={onPress}
        className="pointer-events-auto -my-0.5 flex size-6 shrink-0 items-center justify-center rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        <span
          className={`flex size-5 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] ${
            pressed
              ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
              : "border border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] hover:bg-[var(--editor-accent-hover)]"
          }`}
        >
          <Glyph aria-hidden size={14} strokeWidth={1.5} absoluteStrokeWidth />
        </span>
      </button>
    </Tooltip>
  );
}
