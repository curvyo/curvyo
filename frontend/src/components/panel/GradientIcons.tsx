// The glyphs of the Gradient fill (`specs/0124-gradient-fill` UX notes §2, §3): the 16 px Gradient
// paint item and the Linear and Radial kind items, `currentColor` fades with no second colour.

import { useId } from "react";

const SVG = { width: 16, height: 16, viewBox: "0 0 16 16", fill: "none", "aria-hidden": true } as const;

/** A fade of `currentColor` from opaque to clear; `radial` fades from the centre out. */
function Fade({ id, radial }: { id: string; radial?: boolean }) {
  const stops = (
    <>
      <stop offset="0" stopColor="currentColor" />
      <stop offset="1" stopColor="currentColor" stopOpacity="0" />
    </>
  );
  return radial ? (
    <radialGradient id={id}>{stops}</radialGradient>
  ) : (
    <linearGradient id={id} x1="0" y1="0" x2="1" y2="0">
      {stops}
    </linearGradient>
  );
}

/** "Gradient fill": the paint square with a diagonal fade from the top left. */
export function GradientPaintIcon() {
  const id = useId();
  return (
    <svg {...SVG} stroke="currentColor" strokeWidth={1.5}>
      <defs>
        <linearGradient id={id} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="currentColor" />
          <stop offset="1" stopColor="currentColor" stopOpacity="0" />
        </linearGradient>
      </defs>
      <rect x="2.5" y="2.5" width="11" height="11" fill={`url(#${id})`} />
    </svg>
  );
}

/** "Linear": the paint square with a left-to-right fade. */
export function LinearGradientIcon() {
  const id = useId();
  return (
    <svg {...SVG} stroke="currentColor" strokeWidth={1.5}>
      <defs>
        <Fade id={id} />
      </defs>
      <rect x="2.5" y="2.5" width="11" height="11" fill={`url(#${id})`} />
    </svg>
  );
}

/** "Radial": a circle with a fade from the centre to the rim. */
export function RadialGradientIcon() {
  const id = useId();
  return (
    <svg {...SVG} stroke="currentColor" strokeWidth={1.5}>
      <defs>
        <Fade id={id} radial />
      </defs>
      <circle cx="8" cy="8" r="5.5" fill={`url(#${id})`} />
    </svg>
  );
}
