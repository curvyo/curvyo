import type { ReactElement } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { tipHeading } from "@/lib/sliderNames";

interface SliderTipProps {
  /** The name of the value, the same text as the accessible name. */
  name: string;
  /** The current value with its unit ("0.50 mm", "80 %"), or `null` when it does not fit. */
  valueText: string | null;
  /** The gesture help, muted on the second line. */
  help: string;
  /** The side that does not cover the neighbouring fields: left in the panel, below a bar. */
  side: "top" | "right" | "bottom" | "left";
  children: ReactElement;
}

/**
 * The tooltip of a slider or a slider-like value field (`specs/0047-transform-polish/`
 * criterion 58, `docs/design-system.md`, "Slider"): the name of the value and its current value
 * on the first line, the gesture help muted on the second. The timing and the house style are
 * those of `Tooltip`.
 */
export function SliderTip({ name, valueText, help, side, children }: SliderTipProps) {
  return (
    <Tooltip
      side={side}
      content={
        <>
          <span className="block font-medium">{tipHeading(name, valueText)}</span>
          <span className="block opacity-70">{help}</span>
        </>
      }
    >
      {children}
    </Tooltip>
  );
}
