import { Ban, Copy, Frame, Link2, Move } from "lucide-react";
import type { DropBadgeKind } from "@/lib/dragAnswer";

/** The drop badge beside the pointer (UX notes §2 "Drop badge"). */
export interface DropBadge {
  kind: DropBadgeKind;
  word: string;
  refused: boolean;
  x: number;
  y: number;
}

const GLYPHS: Record<DropBadgeKind, typeof Link2 | null> = {
  none: null,
  clone: Link2,
  not_on_sketch_page: Ban,
  no_sketch: Ban,
  move: Move,
  copy: Copy,
  placed_page: Frame,
  only_on_laser_page: Ban,
};

/** The drop badge (UX notes §2): DOM, `aria-hidden`, no pointer events; glyph and word. */
export function DropBadgePill({
  badge,
  position = "fixed",
}: {
  badge: DropBadge;
  position?: "fixed" | "absolute";
}) {
  const Glyph = GLYPHS[badge.kind] ?? Ban;
  return (
    <div
      aria-hidden
      data-drop-badge=""
      className={`pointer-events-none ${position} z-50 flex h-5 items-center gap-1 whitespace-nowrap rounded-lg border px-1.5 text-xs`}
      style={{
        left: badge.x,
        top: badge.y,
        background: "var(--toolbar-bg)",
        color: "var(--toolbar-icon)",
        borderColor: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
      }}
    >
      <Glyph size={12} />
      {badge.word}
    </div>
  );
}
