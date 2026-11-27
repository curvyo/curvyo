import { Barcode, Box, Cog, Grid3x3, QrCode, Spline, SquareRoundCorner } from "lucide-react";
import type { ReactNode } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { GeneratorKindsApi } from "@/hooks/useGenerators";

const GLYPHS: Record<string, ReactNode> = {
  Barcode: <Barcode size={16} strokeWidth={1.5} aria-hidden />,
  Box: <Box size={16} strokeWidth={1.5} aria-hidden />,
  Cog: <Cog size={16} strokeWidth={1.5} aria-hidden />,
  Grid3x3: <Grid3x3 size={16} strokeWidth={1.5} aria-hidden />,
  QrCode: <QrCode size={16} strokeWidth={1.5} aria-hidden />,
  Spline: <Spline size={16} strokeWidth={1.5} aria-hidden />,
  SquareRoundCorner: <SquareRoundCorner size={16} strokeWidth={1.5} aria-hidden />,
};

/** The segment's width: 84 px, or wider for a longer name ("Rounded box"), so the text stays on
 * one line (16 px glyph, 4 px gap, 12 px padding each side). */
function segmentWidth(label: string): number {
  const context = document.createElement("canvas").getContext("2d");
  if (!context) {
    return 84;
  }
  context.font = `14px ${getComputedStyle(document.body).fontFamily}`;
  return Math.max(84, Math.ceil(context.measureText(label).width) + 44);
}

const BORDER = "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]";

/**
 * The Generators bar (`specs/0141-parametric-generators` UX notes §1, `0151` slice 1): one pill, a
 * group "Generator" of toggle buttons with a 16 px glyph each. None is pressed on entering the tool;
 * a press arms a segment, a press on the pressed one releases it, at most one is pressed. One Tab
 * stop (the pressed segment, else the first); Left, Right, Home and End move the focus and do not
 * arm; Space and Enter toggle. Nothing else: the parameters are in the panel once the object exists.
 */
export function GeneratorsBar({
  api,
  onReturnFocus,
}: {
  api: GeneratorKindsApi;
  onReturnFocus: () => void;
}) {
  if (api.kinds.length === 0) {
    return null;
  }
  const pressedAt = api.kinds.findIndex((kind) => kind.id === api.kind);
  const tabStop = pressedAt >= 0 ? pressedAt : 0;
  return (
    <div
      data-placement-keeps
      className="pointer-events-auto flex h-9 min-w-0 items-center rounded-lg px-2"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <div
        role="group"
        aria-label="Generator"
        className="inline-flex w-fit shrink-0 overflow-hidden rounded-md ring-1 ring-border"
        onKeyDown={(event) => {
          const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
          if (event.key !== "Home" && event.key !== "End" && step === undefined) {
            return;
          }
          const items = Array.from(event.currentTarget.querySelectorAll<HTMLElement>("button"));
          const at = items.indexOf(event.target as HTMLElement);
          const target =
            step !== undefined
              ? items[(at + step + items.length) % items.length]
              : event.key === "Home"
                ? items[0]
                : items[items.length - 1];
          event.preventDefault();
          event.stopPropagation();
          target?.focus();
        }}
      >
        {api.kinds.map((kind, index) => (
          <Tooltip key={kind.id} side="bottom" content={kind.loading ? "Loading font\u2026" : ""}>
            <button
              type="button"
              aria-pressed={kind.id === api.kind}
              tabIndex={index === tabStop ? 0 : -1}
              onClick={(event) => {
                api.toggleKind(kind.id);
                // `detail` is 0 for keyboard activation, 1 or more for a click.
                if (event.detail > 0) {
                  onReturnFocus();
                }
              }}
              style={{ width: segmentWidth(kind.segment), height: 28 }}
              className={`flex items-center justify-center bg-transparent text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] aria-pressed:!bg-[var(--toolbar-icon-active-bg)] aria-pressed:!text-[var(--toolbar-icon-active-fg)] ${index > 0 ? `border-l ${BORDER}` : ""}`}
            >
              <span className="flex items-center gap-1 text-sm whitespace-nowrap">
                {GLYPHS[kind.icon]}
                {kind.segment}
              </span>
            </button>
          </Tooltip>
        ))}
      </div>
    </div>
  );
}
