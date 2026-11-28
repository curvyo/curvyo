/** The thumb's diameter, px (`docs/ui-elements.md` §9). */
export const SLIDER_THUMB_PX = 14;
/** The track's inset at both ends, px: the thumb's half and 1 px, so the thumb never leaves the
 * hit area. */
export const SLIDER_INSET_PX = SLIDER_THUMB_PX / 2 + 1;
/** The track's ground: `--toolbar-icon` at 25 %. */
export const SLIDER_TRACK = "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)";

/**
 * The one look of a slider's track, fill and thumb (`docs/ui-elements.md` §9): a 4 px track, the
 * filled part `--editor-accent`, a 14 px white thumb with a 2 px `--editor-accent` ring, and the
 * focus ring around the thumb while its slider (a `group` element) has keyboard focus. Each slider
 * keeps its own pointer and key handling; `mixed` draws the track alone.
 */
export function SliderMarks({
  thumbLeft,
  track = SLIDER_TRACK,
  mixed = false,
}: {
  /** The thumb centre's x, px from the slider's left edge. */
  thumbLeft: number;
  track?: string;
  mixed?: boolean;
}) {
  return (
    <>
      <span
        aria-hidden
        className="pointer-events-none absolute top-1/2 h-1 -translate-y-1/2 rounded-full"
        style={{ left: SLIDER_INSET_PX, right: SLIDER_INSET_PX, background: track }}
      />
      {mixed ? null : (
        <>
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 h-1 -translate-y-1/2 rounded-full bg-[var(--editor-accent)]"
            style={{ left: SLIDER_INSET_PX, width: thumbLeft - SLIDER_INSET_PX }}
          />
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 rounded-full border-2 border-[var(--editor-accent)] bg-white group-hover:bg-[color-mix(in_srgb,var(--editor-accent)_20%,white)] group-focus-visible:shadow-[0_0_0_2px_var(--toolbar-bg),0_0_0_4px_var(--editor-accent)]"
            style={{
              width: SLIDER_THUMB_PX,
              height: SLIDER_THUMB_PX,
              left: thumbLeft - SLIDER_THUMB_PX / 2,
              marginTop: -SLIDER_THUMB_PX / 2,
            }}
          />
        </>
      )}
    </>
  );
}
