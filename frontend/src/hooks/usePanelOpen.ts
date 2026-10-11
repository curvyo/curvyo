import { useCallback, useRef, useState } from "react";

import { readPanelCollapsed, writePanelCollapsed } from "@/lib/panelCollapsed";
import type { WasmSession } from "@/lib/editorSession";

/** The body's width, px (`docs/design-system.md`, "Properties panel"). The strip's 32 px never
 * toggles, so it is not part of the shift the view makes. */
export const PANEL_WIDTH_PX = 280;

export interface PanelOpenApi {
  /** Whether the body is shown. */
  open: boolean;
  /** Reads the current value now, also between a call to `setOpen` and the next render. */
  isOpen: () => boolean;
  /** Shows or hides the body. The canvas gains or loses the body's width, so the view keeps the
   * document where it is on screen (`0007` criterion 39), and the choice is saved for the OS
   * user (`0051` criterion 10). */
  setOpen: (next: boolean) => void;
}

/**
 * Whether the Properties panel's body is shown (`specs/0051-sidebar-and-no-menu/` criteria 7, 8,
 * 10). A first start shows it; the choice is read from the webview's storage once.
 */
export function usePanelOpen(getSession: () => WasmSession | null): PanelOpenApi {
  const [open, setOpenState] = useState(() => !readPanelCollapsed());
  const openRef = useRef(open);

  const isOpen = useCallback(() => openRef.current, []);
  const setOpen = useCallback(
    (next: boolean) => {
      if (openRef.current === next) {
        return;
      }
      openRef.current = next;
      getSession()?.keep_view_origin_for_panel_toggle(next ? -PANEL_WIDTH_PX : PANEL_WIDTH_PX);
      writePanelCollapsed(!next);
      setOpenState(next);
    },
    [getSession],
  );
  return { open, isOpen, setOpen };
}
