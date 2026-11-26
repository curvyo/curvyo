import { useRef } from "react";

import { PickGlyph } from "@/components/panel/PickGlyph";
import { PICK_COLOUR, PICK_KINDS, PICK_LABEL, moveInGrid, noPickText, type PickKind } from "@/lib/picks";

interface PickControlProps {
  /** The accessible name of the group: "Pick kind" (the bar) or "Use" (an operation). */
  label: string;
  /** The pressed pick, `null` while selected operations follow different ones. */
  value: PickKind | null;
  /** Whether the pick can be pressed; a pick that cannot is dimmed with its reason (default: all). */
  available?: (kind: PickKind) => boolean;
  onPress: (kind: PickKind) => void;
}

/**
 * The four pick buttons (`specs/0146` `ux-notes.md` 3): a 2 x 2 grid of symbol-and-word toggles,
 * exactly one pressed (none while mixed). Manual activation: the arrows move the focus only, Space
 * or Enter presses. A pick that cannot be taken is dimmed, still focusable, and says why.
 */
export function PickControl({ label, value, available, onPress }: PickControlProps) {
  const group = useRef<HTMLDivElement>(null);
  const stop = value ?? "best";
  return (
    <div ref={group} role="group" aria-label={label} className="grid w-[228px] grid-cols-2 gap-1">
      {PICK_KINDS.map((kind, index) => {
        const pressed = value === kind;
        const dimmed = !pressed && available !== undefined && !available(kind);
        const reason = dimmed ? noPickText(kind) : undefined;
        return (
          <button
            key={kind}
            type="button"
            data-pick={kind}
            aria-pressed={pressed}
            aria-disabled={dimmed || undefined}
            aria-description={reason}
            title={reason}
            tabIndex={kind === stop ? 0 : -1}
            onClick={() => {
              if (!dimmed) onPress(kind);
            }}
            onKeyDown={(event) => {
              const next = moveInGrid(index, event.key);
              if (next === null) return;
              event.preventDefault();
              group.current?.querySelector<HTMLButtonElement>(`[data-pick="${PICK_KINDS[next]}"]`)?.focus();
            }}
            className={`flex h-7 w-[112px] items-center gap-1 rounded-[5px] border px-2 text-left text-xs focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
              pressed
                ? "border-[var(--toolbar-icon-active-bg)] bg-[var(--toolbar-icon-active-bg)] font-semibold text-[var(--toolbar-icon-active-fg)]"
                : dimmed
                  ? "border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] text-[var(--lane-line)]"
                  : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
            }`}
          >
            <span style={{ color: pressed || dimmed ? undefined : PICK_COLOUR[kind] }} className="flex shrink-0">
              <PickGlyph kind={kind} />
            </span>
            <span className="min-w-0 truncate">{PICK_LABEL[kind]}</span>
          </button>
        );
      })}
    </div>
  );
}
