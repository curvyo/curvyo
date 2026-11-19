import { LoaderCircle } from "lucide-react";
import { type KeyboardEvent, type ReactNode, useId } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { GLYPH, ICON_BUTTON } from "@/components/workbench/machineStyles";

const OFF = "aria-disabled:opacity-40 aria-disabled:hover:bg-transparent";

interface SymbolButtonProps {
  name: string;
  enabled: boolean;
  busy?: boolean;
  /** The tooltip's second line and the button's description: why it does not apply. */
  cause?: string | null;
  describedBy?: string;
  side?: "left" | "top";
  tabIndex?: number;
  buttonRef?: (node: HTMLButtonElement | null) => void;
  onFocus?: () => void;
  onKeyDown?: (event: KeyboardEvent) => void;
  onPress: () => void;
  children: ReactNode;
}

/** A 28 px symbol button of the Connector group and the Machine controls footer; when it does not
 * apply it is greyed and keeps its place, its tab stop and its tooltip, and a press does nothing
 * (`specs/0064` UX notes §3, `specs/0118` UX notes §3). */
export function SymbolButton({
  name,
  enabled,
  busy = false,
  cause = null,
  describedBy,
  side = "left",
  tabIndex,
  buttonRef,
  onFocus,
  onKeyDown,
  onPress,
  children,
}: SymbolButtonProps) {
  const causeId = useId();
  const tip =
    cause === null ? (
      name
    ) : (
      <>
        <div>{name}</div>
        <div className="text-muted-foreground">{cause}</div>
      </>
    );
  return (
    <>
      <Tooltip side={side} content={tip}>
        <button
          type="button"
          ref={buttonRef}
          aria-label={name}
          aria-disabled={!enabled}
          aria-busy={busy || undefined}
          aria-describedby={cause === null ? describedBy : causeId}
          tabIndex={tabIndex}
          className={`${ICON_BUTTON} ${OFF}`}
          onFocus={onFocus}
          onKeyDown={onKeyDown}
          onClick={() => {
            if (enabled) onPress();
          }}
        >
          {busy ? <LoaderCircle {...GLYPH} className="animate-spin" /> : children}
        </button>
      </Tooltip>
      {cause === null ? null : (
        <span id={causeId} className="sr-only">
          {cause}
        </span>
      )}
    </>
  );
}
