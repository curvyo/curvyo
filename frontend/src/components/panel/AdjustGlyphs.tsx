import type { StepKindKey } from "@/lib/adjustmentsView";

/** The 16 x 16 step glyphs of the Adjustments group (`ux-notes.md` §3.5; draft 02 sheet): ink,
 * 1.5 px, round caps. A step this build cannot run has the dashed question box. */
export function AdjustGlyph({ kind }: { kind: StepKindKey | "unknown" }) {
  return (
    <svg
      aria-hidden
      width={16}
      height={16}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {kind === "levels" ? (
        <>
          <path d="M3 13V9M6 13V4M9 13V7M12 13V10" />
          <path d="M2 14.5h12" strokeWidth={1} />
        </>
      ) : null}
      {kind === "denoise" ? (
        <>
          <path d="M3 4h3M3 8h5M3 12h3" />
          <path d="M11 3v10" />
          <circle cx={13} cy={6} r={0.6} />
          <circle cx={13} cy={10} r={0.6} />
        </>
      ) : null}
      {kind === "local_contrast" ? (
        <>
          <rect x={2.5} y={2.5} width={4.5} height={4.5} />
          <rect x={9} y={9} width={4.5} height={4.5} />
          <path d="M9 4.8h4.5M2.5 11.2H7" strokeDasharray="1.5 1.5" />
        </>
      ) : null}
      {kind === "contours" ? <path d="M2 10c2-6 4-6 6-2s4 4 6-2" /> : null}
      {kind === "sharpen" ? (
        <>
          <path d="M2 4l4 4-4 4M14 4l-4 4 4 4" />
          <path d="M8 3v10" />
        </>
      ) : null}
      {kind === "crop" ? <path d="M4.5 1.5v10h10M1.5 4.5h10v10" /> : null}
      {kind === "resize" ? (
        <>
          <rect x={2.5} y={6.5} width={7} height={7} />
          <path d="M9 2h5v5M14 2l-5 5" />
        </>
      ) : null}
      {kind === "channel_mix" ? (
        <>
          <circle cx={6} cy={6} r={3.5} />
          <circle cx={10} cy={6} r={3.5} />
          <circle cx={8} cy={10} r={3.5} />
        </>
      ) : null}
      {kind === "curves" ? (
        <>
          <path d="M2.5 13.5c3 0 4-11 11-11" />
          <path d="M2.5 2.5v11h11" strokeWidth={1} />
        </>
      ) : null}
      {kind === "unknown" ? (
        <>
          <rect x={2.5} y={2.5} width={11} height={11} strokeDasharray="2 2" />
          <path d="M6.5 6.5a1.5 1.5 0 1 1 2 1.4c-.4.2-.5.5-.5 1M8 11.2v.1" />
        </>
      ) : null}
    </svg>
  );
}
