import { Link2 } from "lucide-react";
import { useEffect, useRef } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { ClonesApi } from "@/hooks/useClones";
import { cloneBlockLines } from "@/lib/cloneText";

const OUTLINE_BUTTON =
  "h-7 w-full rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

interface CloneBlockProps {
  clones: ClonesApi;
  onReturnFocus: () => void;
  /** The pressed button's box, for the notice that shows 4 px under it (UX notes "Focus"). */
  onPressedAt: (button: DOMRect) => void;
}

/** A button of the block: a mouse press returns the focus to the canvas, a key press keeps it
 * (and the block moves it where the UX notes say). Its tooltip is the panel's, at the left. */
function BlockButton({
  label,
  title,
  onPress,
  onReturnFocus,
  onHover,
  onPressedAt,
}: {
  label: string;
  title: string;
  onPressedAt: (button: DOMRect) => void;
  /** `keyboard` is true for Enter or Space. */
  onPress: (keyboard: boolean) => void;
  onReturnFocus: () => void;
  /** The targets show their hover box while the pointer or focus is on the button. */
  onHover?: (on: boolean) => void;
}) {
  // The button can go away without a pointerleave or a blur (the selection changed under it):
  // its hover box goes with it (UX review must-fix 3).
  const hoverRef = useRef(onHover);
  useEffect(() => {
    hoverRef.current = onHover;
  }, [onHover]);
  useEffect(() => () => hoverRef.current?.(false), []);
  return (
    <Tooltip side="left" content={title}>
      <button
        type="button"
        onPointerEnter={() => onHover?.(true)}
        onPointerLeave={() => onHover?.(false)}
        onFocus={() => onHover?.(true)}
        onBlur={() => onHover?.(false)}
        className={OUTLINE_BUTTON}
        onClick={(event) => {
          onHover?.(false);
          onPressedAt(event.currentTarget.getBoundingClientRect());
          const keyboard = event.detail === 0;
          onPress(keyboard);
          if (!keyboard) {
            onReturnFocus();
          }
        }}
      >
        {label}
      </button>
    </Tooltip>
  );
}

/**
 * The clone block of the Style tab (`specs/0049-linked-clone` criteria 22 and 56; UX notes "The
 * Style tab"): shown instead of the style sections when the selection holds only linked clones.
 * It names the original and offers Select original, Select root
 * original (chains only) and Unlink clone. Inline, no popup.
 */
export function CloneBlock({ clones, onReturnFocus, onPressedAt }: CloneBlockProps) {
  const { panel } = clones;
  if (!panel.clonesOnly) {
    return null;
  }
  const many = panel.count > 1;
  const { title, chain } = cloneBlockLines(panel);
  return (
    <div role="group" aria-label="Linked clone" className="flex flex-col gap-2">
      <div className="flex items-start gap-1 text-sm text-[var(--toolbar-icon)]">
        <Link2 aria-hidden size={16} className="mt-0.5 shrink-0" />
        <span className="min-w-0 break-words">{title}</span>
      </div>
      {chain !== "" && <p className="text-xs text-[var(--panel-muted-fg)]">{chain}</p>}
      <div className="mt-2 flex flex-col gap-2">
        <BlockButton
          label={many ? "Select originals" : "Select original"}
          title="Selects the object this clone follows."
          onPress={(keyboard) => clones.apply("select-original", "panel", keyboard)}
          onReturnFocus={onReturnFocus}
          onPressedAt={onPressedAt}
          onHover={(on) => clones.hover(on ? "original" : "")}
        />
        {panel.deepest >= 2 && (
          <BlockButton
            label={many ? "Select root originals" : "Select root original"}
            title="Selects the path at the end of the chain."
            onPress={(keyboard) => clones.apply("select-root", "panel", keyboard)}
            onReturnFocus={onReturnFocus}
          onPressedAt={onPressedAt}
            onHover={(on) => clones.hover(on ? "root" : "")}
          />
        )}
        <BlockButton
          label={many ? "Unlink clones" : "Unlink clone"}
          title="Turns the clone into an ordinary path. Shape and style stay."
          onPress={(keyboard) => clones.apply("unlink", "panel", keyboard)}
          onReturnFocus={onReturnFocus}
          onPressedAt={onPressedAt}
        />
      </div>
    </div>
  );
}

/** The origin line (criterion 23): Select clones, while a clone follows the selection. */
export function OriginLine({ clones, onReturnFocus, onPressedAt }: CloneBlockProps) {
  if (clones.panel.followers === 0) {
    return null;
  }
  return (
    <div className="flex flex-col gap-2">
      <BlockButton
        label={clones.panel.followers === 1 ? "Select clone" : "Select clones"}
        title="Selects every clone of this object."
        onPress={(keyboard) => clones.apply("select-clones", "panel", keyboard)}
        onReturnFocus={onReturnFocus}
        onPressedAt={onPressedAt}
      />
    </div>
  );
}
