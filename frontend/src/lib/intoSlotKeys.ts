// The rows and the keys of the Into slot list (`specs/0154-library-into-slots` criterion 10, UX
// notes 8.2): per holder its Fill row, then its slot rows; Enter takes the focused row only.
import type { SlotTargets } from "@/lib/intoSlot";
import { slotOptionLabel } from "@/lib/intoSlotWords";

/** One row of the list. */
export interface IntoSlotRow {
  key: string;
  kind: "fill" | "slot";
  id: string;
  label: string;
  filled: boolean;
}

/** The rows of `targets` in list order. */
export function intoSlotRows(targets: SlotTargets): IntoSlotRow[] {
  return targets.holders.flatMap((holder) => [
    ...(holder.filled < holder.of
      ? [
          {
            key: `fill-${holder.id}`,
            kind: "fill" as const,
            id: holder.id,
            label: `Fill empty slots of ${holder.name}, ${holder.of - holder.filled} of ${holder.of} slots`,
            filled: false,
          },
        ]
      : []),
    ...holder.slots.map((slot) => ({
      key: slot.id,
      kind: "slot" as const,
      id: slot.id,
      label: slotOptionLabel(slot.name, slot.shape, slot.size, slot.outcome, slot.filled),
      filled: slot.filled,
    })),
  ]);
}

/** What a key does in the list: move the focus, take the focused row, close, or nothing. */
export type IntoSlotKey = { move: number } | { take: number } | { close: true } | null;

/** The key `key` with the focus on row `focus` of `count`. A held Enter or Space (`repeat`)
 * takes nothing: the Enter that opened the list must not run on into its first row, the Fill
 * row (UX review 2026-11-26). */
export function intoSlotKey(key: string, repeat: boolean, focus: number, count: number): IntoSlotKey {
  const clamp = (to: number) => ({ move: Math.max(0, Math.min(count - 1, to)) });
  switch (key) {
    case "ArrowDown":
      return clamp(focus + 1);
    case "ArrowUp":
      return clamp(focus - 1);
    case "Home":
      return clamp(0);
    case "End":
      return clamp(count - 1);
    case "PageDown":
      return clamp(focus + 8);
    case "PageUp":
      return clamp(focus - 8);
    case "Enter":
    case " ":
      return repeat || focus < 0 || focus >= count ? null : { take: focus };
    case "Escape":
      return { close: true };
    default:
      return null;
  }
}
