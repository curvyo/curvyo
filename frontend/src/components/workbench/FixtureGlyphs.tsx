import { CircleHelp } from "lucide-react";
import type { FixtureKind } from "@/lib/fixturesView";

/** The kind symbols of the fixtures (`specs/0102-laser-fixtures` UX notes section 4.0): a 16 px
 * grid, 1.5 px stroke, `currentColor`. The grid is `#`, the holder a board with two small
 * holes, the clamp two brackets with a bar between them, the stop a solid L 3 px thick, the screw
 * a head ring with a smaller concentric thread ring inside. */
export function FixtureGlyph({ kind, className }: { kind: FixtureKind; className?: string }) {
  if (kind === "unknown") {
    return <CircleHelp size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className={className} />;
  }
  return (
    <svg
      width={16}
      height={16}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="butt"
      aria-hidden
      className={className}
    >
      {kind === "grid" ? <path d="M6 2v12M10 2v12M2 6h12M2 10h12" /> : null}
      {kind === "holder" ? (
        <>
          <rect x="2.75" y="2.75" width="10.5" height="10.5" />
          <circle cx="6" cy="8" r="1.25" strokeWidth={1} />
          <circle cx="10" cy="8" r="1.25" strokeWidth={1} />
        </>
      ) : null}
      {kind === "clamp" ? <path d="M5 3H2.75v10H5M11 3h2.25v10H11M5 8h6" /> : null}
      {kind === "stop" ? <path d="M3 2h3v8h7v3H3z" fill="currentColor" stroke="none" /> : null}
      {kind === "screw" ? (
        <>
          <circle cx="8" cy="8" r="5.25" />
          <circle cx="8" cy="8" r="2.5" strokeWidth={1} />
        </>
      ) : null}
    </svg>
  );
}

/** The symbol of a slot: a small rectangle or circle outline. */
export function SlotGlyph({ shape, className }: { shape: "rect" | "circle"; className?: string }) {
  return (
    <svg width={16} height={16} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth={1.5} aria-hidden className={className}>
      {shape === "rect" ? <rect x="2.75" y="4.75" width="10.5" height="6.5" /> : <circle cx="8" cy="8" r="4.5" />}
    </svg>
  );
}
