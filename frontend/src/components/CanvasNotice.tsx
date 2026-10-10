import type { CanvasNotice as CanvasNoticeState } from "@/hooks/useHistory";

interface CanvasNoticeProps {
  notice: CanvasNoticeState | null;
}

/**
 * The canvas notice slot (`docs/design-system.md`, "Canvas notice slot"): one line for everything a
 * key press produces (undo, redo, the Pen hint), centred in the free part of the viewport right of
 * the tool rail and 12 px above its bottom edge. It takes no focus, has no controls and is
 * announced politely; the next notice replaces it and the hook ends it after 3 s (5 s above 70
 * characters).
 */
export function CanvasNotice({ notice }: CanvasNoticeProps) {
  return (
    <div
      className="pointer-events-none absolute right-0 bottom-3 left-[var(--rail-right)] z-30 flex justify-center px-3"
      role="status"
      aria-live="polite"
    >
      {notice && notice.source === "canvas" ? (
        <div
          key={notice.id}
          className="rounded-md p-2 text-xs"
          style={{
            background: "var(--toolbar-bg)",
            color: "var(--toolbar-icon)",
            boxShadow: "var(--panel-elevation-shadow)",
          }}
        >
          {notice.text}
        </div>
      ) : null}
    </div>
  );
}
