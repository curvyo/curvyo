// The side the shared controls' tooltips open (`docs/ui-elements.md` §15): left inside the
// properties panel, over the canvas; below elsewhere.
import { createContext, useContext } from "react";

/** True inside the properties panel (`PanelTooltipScope`). */
export const InPanelTooltips = createContext(false);

/** The side a shared control's tooltip opens: left in the panel, below elsewhere. */
export function useControlTooltipSide(): "left" | "bottom" {
  return useContext(InPanelTooltips) ? "left" : "bottom";
}
