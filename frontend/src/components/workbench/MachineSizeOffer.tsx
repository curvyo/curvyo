import { ArrowUpToLine } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { GLYPH, ICON_BUTTON } from "@/components/workbench/machineStyles";
import type { UnitSymbol } from "@/hooks/useDocumentPanel";
import { useLinkEntry } from "@/hooks/useLinkEntry";
import type { WorkbenchesApi } from "@/hooks/useWorkbenches";
import { machineReportsText, workAreaStateText } from "@/lib/workAreaText";
import { isTauri } from "@/platform/host";

interface Props {
  /** The chosen saved profile's id and connector address, as the Connector group has them. */
  profileId: string | null;
  address: string;
  unit: UnitSymbol;
  api: WorkbenchesApi["machine"];
  /** The work area the block shows, so a change of it re-reads the offer. */
  width: number;
  height: number;
  /** Where focus goes after the press and when the offer leaves under the focus. */
  onFocusProfile: () => void;
  /** Called after a press that changed the work area. */
  onUsed: () => void;
}

/**
 * The offer row of the Machine block (`specs/0131` UX notes §2): "Machine reports W × H" and the
 * symbol button Use machine size, while the connected GRBL controller's travel differs from the
 * copy's work area. The row keeps its 28 px place while the controller reports a usable travel;
 * without a connector, controller or usable travel (and in the browser build) there is no row.
 */
export function MachineSizeOffer({ profileId, address, unit, api, width, height, onFocusProfile, onUsed }: Props) {
  const labelId = useId();
  const live = isTauri && profileId !== null && address !== "";
  const entry = useLinkEntry(profileId, address, live, false);
  const travel = live ? (entry.controller?.settings?.travel ?? null) : null;
  // `width` and `height` are read so a press or an undo re-reads the offer from the session.
  void width;
  void height;
  const reported = travel !== null && api.sizeReported(travel[0], travel[1]);
  const offer = reported && travel !== null ? api.sizeOffer(travel[0], travel[1]) : null;
  const [announce, setAnnounce] = useState("");
  const focused = useRef(false);
  const had = useRef(false);
  useEffect(() => {
    if (had.current && offer === null && focused.current) {
      focused.current = false;
      onFocusProfile();
    }
    had.current = offer !== null;
  }, [offer, onFocusProfile]);
  if (!reported || travel === null) return null;
  return (
    <>
      {offer === null ? (
        <div aria-hidden data-machine-size-offer="" className="h-7 shrink-0" />
      ) : (
        <div data-machine-size-offer="" className="flex h-7 items-center justify-between gap-1">
          <span id={labelId} role="status" className="text-xs text-[var(--toolbar-icon)]">
            {machineReportsText(offer[0], offer[1], unit)}
          </span>
          <Tooltip side="left" content="Use machine size">
            <button
              type="button"
              aria-label="Use machine size"
              aria-describedby={labelId}
              className={ICON_BUTTON}
              onFocus={() => {
                focused.current = true;
              }}
              onBlur={() => {
                focused.current = false;
              }}
              onClick={() => {
                if (api.useSize(travel[0], travel[1])) {
                  onUsed();
                  setAnnounce(workAreaStateText(offer[0], offer[1], unit));
                }
                onFocusProfile();
              }}
            >
              <ArrowUpToLine {...GLYPH} />
            </button>
          </Tooltip>
        </div>
      )}
      {/* Only while the offer is gone: a returning offer clears the line, so it never reads a
          size the Work area no longer has and a later press is announced again. */}
      <span role="status" className="sr-only">
        {offer === null ? announce : ""}
      </span>
    </>
  );
}
