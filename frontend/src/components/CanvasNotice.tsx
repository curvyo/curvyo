import { OpenGroups } from "@/components/OpenGroups";
import type { ContextAnnouncement } from "@/hooks/useGrouping";
import type { CanvasNotice as CanvasNoticeState } from "@/hooks/useHistory";

/** The widest and the usual height of a refusal shown beside the pointer, px. */
const NEAR_WIDTH = 280;
const NEAR_HEIGHT = 56;

interface CanvasNoticeProps {
  notice: CanvasNoticeState | null;
  /** How many groups deep the Select tool is (`specs/0023-groups`); the indicator is absent at 0. */
  groupDepth: number;
  onLeaveToLevel: (level: number) => void;
  /** What a screen reader hears when a group is entered or left. */
  announcement: ContextAnnouncement;
  /** A persistent line under the notices: the History preview's readout or banner (`0042`). */
  persistent?: React.ReactNode;
}

/**
 * The canvas notice slot (`docs/design-system.md`, "Canvas notice slot"): the line for everything a
 * key press or a rail command produces (undo, redo, the Pen hint, Group and Ungroup), centred in
 * the free part of the viewport right of the tool rail and 12 px above its bottom edge. A notice
 * takes no focus and has no controls; a status is announced politely and ends after 3 s (5 s above
 * 70 characters), a refusal is an alert and stays 8 s; the next notice replaces it. A refusal with
 * a point (`specs/0110` UX review U8) shows beside it instead, in the same look. Below it, while
 * a group is entered, the persistent indicator "Open groups" (`OpenGroups`), whose crumbs are
 * buttons.
 */
export function CanvasNotice({
  notice,
  groupDepth,
  onLeaveToLevel,
  announcement,
  persistent,
}: CanvasNoticeProps) {
  const shown = notice !== null && notice.source === "canvas" ? notice : null;
  const box = (
    <div
      key={shown?.id}
      className="rounded-md p-2 text-xs whitespace-pre-line"
      style={{
        background: "var(--toolbar-bg)",
        color: shown?.kind === "alert" ? "var(--field-invalid)" : "var(--toolbar-icon)",
        boxShadow:
          shown?.kind === "alert"
            ? "0 0 0 1px var(--field-invalid), var(--panel-elevation-shadow)"
            : "var(--panel-elevation-shadow)",
      }}
    >
      {shown?.text}
    </div>
  );
  const near = shown?.at;
  return (
    <div className="pointer-events-none absolute right-0 bottom-3 left-[var(--rail-right)] z-30 flex flex-col items-center gap-2 px-3">
      <div role="status" aria-live="polite">
        {shown && shown.kind === "status" ? box : null}
      </div>
      <div role="alert">
        {shown && shown.kind === "alert" && !near ? box : null}
        {shown && shown.kind === "alert" && near ? (
          // A refusal beside where the maker acted (`specs/0110` UX review U8): 14 px right of
          // and 18 px below the pointer, kept inside the window.
          <div
            data-near-pointer
            className="fixed max-w-[280px]"
            style={{
              left: Math.max(8, Math.min(near.x + 14, window.innerWidth - NEAR_WIDTH - 8)),
              top: near.y + 18 + NEAR_HEIGHT > window.innerHeight ? Math.max(8, near.y - NEAR_HEIGHT - 8) : near.y + 18,
            }}
          >
            {box}
          </div>
        ) : null}
      </div>
      <OpenGroups depth={groupDepth} onLeaveToLevel={onLeaveToLevel} />
      {persistent}
      <div role="status" aria-live="polite" className="sr-only">
        {announcement.text}
      </div>
    </div>
  );
}
