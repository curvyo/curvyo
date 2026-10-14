import { RatioSlider } from "@/components/RatioSlider";
import { RATIO_SLIDER_NAMES } from "@/lib/sliderNames";
import type { PolyStarMode, Tool } from "@/hooks/useEditorSession";

export interface ShapeToolbarProps {
  tool: Tool;
  polyStarMode: PolyStarMode;
  polyStarPointCount: number;
  polyStarRatio: number;
  onSetPolyStarMode: (mode: PolyStarMode) => void;
  onSetPolyStarPointCount: (count: number) => void;
  onSetPolyStarRatio: (ratio: number) => void;
  /** Returns the keyboard focus to the canvas (after Enter or Escape in the ratio entry). */
  onReturnFocus: () => void;
}

const MIN_POINT_COUNT = 3;
const MAX_POINT_COUNT = 1024;

/**
 * The contextual bar of the creation tools (`specs/0009-unified-object-editing/`
 * criteria 29 and 30). The Rectangle and Ellipse tools have none: they only
 * create, and every edit of a shape (radius, "Remove rounding", "Object to
 * path") is the Select bar's. The Polygon/Star tool keeps the mode toggle and
 * the settings for the *next* shape, marked "New:" so they are not read as
 * acting on the selection (they never change a selected shape).
 */
export function ShapeToolbar({
  tool,
  polyStarMode,
  polyStarPointCount,
  polyStarRatio,
  onSetPolyStarMode,
  onSetPolyStarPointCount,
  onSetPolyStarRatio,
  onReturnFocus,
}: ShapeToolbarProps) {
  if (tool !== "polygon-star") {
    return null;
  }
  return (
    <div
      // A press here does not cancel a click-to-create placement (`0052` criterion 25).
      data-placement-keeps
      className="flex pointer-events-auto h-9 min-w-0 items-center gap-3 rounded-lg px-2 text-sm"
      style={{
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
      }}
    >
      <div
        role="group"
        aria-label="Polygon or star mode"
        className="flex overflow-hidden rounded-md ring-1 ring-border"
      >
        <button
          type="button"
          aria-pressed={polyStarMode === "polygon"}
          onClick={() => onSetPolyStarMode("polygon")}
          className="px-2 py-1"
          style={{
            background:
              polyStarMode === "polygon" ? "var(--toolbar-icon-active-bg)" : "transparent",
            color:
              polyStarMode === "polygon" ? "var(--toolbar-icon-active-fg)" : "var(--toolbar-icon)",
          }}
        >
          Polygon
        </button>
        <button
          type="button"
          aria-pressed={polyStarMode === "star"}
          onClick={() => onSetPolyStarMode("star")}
          className="px-2 py-1"
          style={{
            background: polyStarMode === "star" ? "var(--toolbar-icon-active-bg)" : "transparent",
            color: polyStarMode === "star" ? "var(--toolbar-icon-active-fg)" : "var(--toolbar-icon)",
          }}
        >
          Star
        </button>
      </div>

      <label className="flex items-center gap-1 text-[var(--toolbar-icon)]">
        <span className="opacity-60">New:</span> Points
        <input
          type="number"
          min={MIN_POINT_COUNT}
          max={MAX_POINT_COUNT}
          step={1}
          value={polyStarPointCount}
          onChange={(event) => {
            const next = Math.round(Number(event.target.value));
            if (Number.isFinite(next) && next >= MIN_POINT_COUNT && next <= MAX_POINT_COUNT) {
              onSetPolyStarPointCount(next);
            }
          }}
          className="w-16 rounded-md border border-border bg-transparent px-1 py-0.5"
        />
      </label>

      {polyStarMode === "star" && (
        <div className="flex items-center gap-1 text-[var(--toolbar-icon)]">
          <span className="opacity-60">New:</span> Ratio
          <RatioSlider
            label={RATIO_SLIDER_NAMES.newStars}
            value={polyStarRatio}
            // The setting is not a document write: every tick and click sets it, there is
            // no commit, and Escape in a drag puts the value of the press back.
            onPreview={onSetPolyStarRatio}
            onRelease={() => undefined}
            onSet={onSetPolyStarRatio}
            onCancel={onSetPolyStarRatio}
            onReturnFocus={onReturnFocus}
          />
        </div>
      )}
    </div>
  );
}
