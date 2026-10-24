/** Badge diameter, px (`docs/design-system.md`, "Modifier badge"). */
const BADGE_PX = 16;
/** The badge centre's distance up and left of the carrier's start, px: clear of the outline. */
const BADGE_OFFSET_PX = 12;

interface PickSetBadgeProps {
  /** The start of the first picked carrier, canvas px; `null` without a pick set. */
  at: { x: number; y: number } | null;
}

/**
 * The "1" badge of the pick set (`specs/0080-text-on-path` UX notes §9): on the first picked
 * carrier, which gets the text; the others get text clones. The "Modifier badge" look (solid
 * accent ground, white ring), DOM, `pointer-events: none`, `aria-hidden`. Rust decides where it
 * sits (`Session::text_path_pick_badge`).
 */
export function PickSetBadge({ at }: PickSetBadgeProps) {
  if (!at) {
    return null;
  }
  return (
    <div
      aria-hidden
      data-pick-badge
      className="pointer-events-none absolute z-40 flex items-center justify-center rounded-full text-[10px] font-semibold leading-none"
      style={{
        left: at.x - BADGE_OFFSET_PX - BADGE_PX / 2,
        top: at.y - BADGE_OFFSET_PX - BADGE_PX / 2,
        width: BADGE_PX,
        height: BADGE_PX,
        background: "var(--editor-accent)",
        boxShadow: "0 0 0 1px #FFFFFF",
        color: "#FFFFFF",
      }}
    >
      1
    </div>
  );
}
