import { Ungroup } from "lucide-react";
import { Tooltip } from "radix-ui";

import { EDIT_TILING, RELEASE_TILING, RELEASE_TILING_RULE, TILE, type TileSlot, tileRule } from "@/lib/tilingText";
import { buttonClass, GLYPH } from "@/components/ui/looks";
import { TOOLTIP_CLASS } from "@/components/ui/tooltip";

const BUTTON_CLASS = buttonClass("secondary", "bar");

const TOOLTIP_STYLE = { background: "var(--toolbar-bg)", color: "var(--toolbar-icon)", boxShadow: "var(--panel-elevation-shadow)" };

/** The Tile glyph: a solid square and two dashed squares repeating it (UX notes §1). */
function TileGlyph() {
  return (
    <svg viewBox="0 0 16 16" width={16} height={16} aria-hidden="true" fill="none" stroke="currentColor" strokeWidth={1.5}>
      <rect x={1} y={5.5} width={5} height={5} />
      <rect x={6.5} y={5.5} width={5} height={5} strokeDasharray="1.5 1" />
      <rect x={12} y={5.5} width={3.25} height={5} strokeDasharray="1.5 1" />
    </svg>
  );
}

function TipButton({ name, rule, disabled, onPress, children }: { name: string; rule: string; disabled?: boolean; onPress: (byKeyboard: boolean) => void; children: React.ReactNode }) {
  return (
    <Tooltip.Root>
      <Tooltip.Trigger asChild>
        <button
          type="button"
          data-select-command={name}
          aria-disabled={disabled}
          aria-label={name}
          className={`${BUTTON_CLASS} size-7 px-0`}
          onClick={(event) => {
            if (!disabled) {
              // `detail` is 0 for Space and Enter, 1 or more for a click.
              onPress(event.detail === 0);
            }
          }}
        >
          {children}
        </button>
      </Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content side="bottom" sideOffset={4} className={TOOLTIP_CLASS} style={TOOLTIP_STYLE}>
          <div className="font-semibold">{name}</div>
          <div className="text-[var(--panel-muted-fg)]">{rule}</div>
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}

/**
 * The Select bar's Tile slot (`specs/0069-tiling` criteria 3 and 26, UX notes §1): "Tile",
 * disabled with nothing selected or a tiling among other objects; "Edit tiling" and "Release
 * tiling" with one tiling selected.
 */
export function TileGroup({
  slot,
  onEnter,
  onRelease,
  onReturnFocus,
}: {
  slot: TileSlot;
  onEnter: (byKeyboard: boolean) => void;
  onRelease: () => void;
  onReturnFocus: () => void;
}) {
  const disabled = slot === "nothing" || slot === "with-others";
  return (
    <Tooltip.Provider delayDuration={400}>
      <div role="group" aria-label="Tiling" className="flex items-center gap-1 py-1">
        <TipButton name={slot === "edit" ? EDIT_TILING : TILE} rule={tileRule(slot)} disabled={disabled} onPress={onEnter}>
          <TileGlyph />
        </TipButton>
        {slot === "edit" ? (
          <TipButton
            name={RELEASE_TILING}
            rule={RELEASE_TILING_RULE}
            onPress={(byKeyboard) => {
              onRelease();
              if (!byKeyboard) {
                onReturnFocus();
              }
            }}
          >
            <Ungroup {...GLYPH} aria-hidden />
          </TipButton>
        ) : null}
      </div>
    </Tooltip.Provider>
  );
}
