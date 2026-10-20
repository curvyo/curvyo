// A press on a dimmed command button (`aria-disabled`, rows "Flip buttons", "Guide (`0046`)",
// "Weld (`0068`)" of `docs/design-system.md`): it does nothing and leaves the tooltip open. Radix
// closes the tooltip on the trigger's pointerdown and click unless the event's default is
// prevented, so the dimmed button prevents both.

/** Prevents the default of a press on a dimmed button; `true` when the button is dimmed and
 * the press must do nothing else. */
export function dimmedPress(event: { preventDefault: () => void }, enabled: boolean): boolean {
  if (enabled) {
    return false;
  }
  event.preventDefault();
  return true;
}
