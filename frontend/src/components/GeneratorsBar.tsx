import { Box, Cog, Spline, SquareRoundCorner } from "lucide-react";
import type { ReactNode } from "react";

import { ToggleGroup, type ToggleOption } from "@/components/ui/toggle-group";
import type { GeneratorKindsApi } from "@/hooks/useGenerators";

const GLYPHS: Record<string, ReactNode> = {
  Box: <Box size={16} strokeWidth={1.5} aria-hidden />,
  Cog: <Cog size={16} strokeWidth={1.5} aria-hidden />,
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

/**
 * The Generators bar (`specs/0141-parametric-generators` UX notes §1): one pill with a radio group
 * "Generator" of text segments with a 16 px glyph each, one Tab stop, the arrows move and select.
 * Nothing else: the parameters are in the panel once the object exists.
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
  const options: ToggleOption<string>[] = api.kinds.map((kind) => ({
    value: kind.id,
    label: kind.segment,
    tooltip: "",
    width: segmentWidth(kind.segment),
    icon: (
      <span className="flex items-center gap-1 text-sm whitespace-nowrap">
        {GLYPHS[kind.icon]}
        {kind.segment}
      </span>
    ),
  }));
  return (
    <div
      data-placement-keeps
      className="pointer-events-auto flex h-9 min-w-0 items-center rounded-lg px-2"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <ToggleGroup
        label="Generator"
        options={options}
        value={api.kind}
        onChange={api.setKind}
        itemWidth={84}
        bar
        tooltipSide="bottom"
        onReturnFocus={onReturnFocus}
      />
    </div>
  );
}
