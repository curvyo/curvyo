import { OpenGroups } from "@/components/OpenGroups";
import type { ContextAnnouncement } from "@/hooks/useGrouping";
import type { CanvasNotice as CanvasNoticeState } from "@/hooks/useHistory";

interface CanvasNoticeProps {
  notice: CanvasNoticeState | null;
  /** How many groups deep the Select tool is (`specs/0023-groups`); the indicator is absent at 0. */
  groupDepth: number;
  onLeaveToLevel: (level: number) => void;
  /** What a screen reader hears when a group is entered or left. */
  announcement: ContextAnnouncement;
}

/**
 * The canvas notice slot (`docs/design-system.md`, "Canvas notice slot"): the line for everything a
 * key press or a rail command produces (undo, redo, the Pen hint, Group and Ungroup), centred in
 * the free part of the viewport right of the tool rail and 12 px above its bottom edge. A notice
 * takes no focus and has no controls; a status is announced politely and ends after 3 s (5 s above
 * 70 characters), a refusal is an alert and stays 8 s; the next notice replaces it. Below it, while
 * a group is entered, the persistent indicator "Open groups" (`OpenGroups`), whose crumbs are
 * buttons.
 */
export function CanvasNotice({
  notice,
  groupDepth,
  onLeaveToLevel,
  announcement,
}: CanvasNoticeProps) {
  const shown = notice !== null && notice.source === "canvas" ? notice : null;
  const box = (
    <div
      key={shown?.id}
      className="rounded-md p-2 text-xs"
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
  return (
    <div className="pointer-events-none absolute right-0 bottom-3 left-[var(--rail-right)] z-30 flex flex-col items-center gap-2 px-3">
      <div role="status" aria-live="polite">
        {shown && shown.kind === "status" ? box : null}
      </div>
      <div role="alert">{shown && shown.kind === "alert" ? box : null}</div>
      <OpenGroups depth={groupDepth} onLeaveToLevel={onLeaveToLevel} />
      <div role="status" aria-live="polite" className="sr-only">
        {announcement.text}
      </div>
    </div>
  );
}
