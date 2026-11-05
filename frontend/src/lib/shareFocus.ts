// Where the focus goes after a membership act that replaces the Share tab's blocks
// (`specs/0097-membership-lifecycle` ux-notes 7.2 and 9): the next Removed block's heading after
// Leave, the Participants list after Sync again or the last Add back. A request counts only for
// a short while, so a block that is merely shown later never takes the focus.

/** How long a focus request waits for the block it names (ms). */
const WAIT_MS = 3000;

let heading = 0;
let participants = 0;

/** Asks the next Removed block to focus its heading. */
export function focusEndedHeading(): void {
  heading = Date.now();
}

/** Drops a request that will not be answered (the act failed). */
export function cancelEndedHeadingFocus(): void {
  heading = 0;
}

/** Whether a Removed block that just appeared takes the focus (once, and only soon after). */
export function takeEndedHeadingFocus(): boolean {
  const asked = heading !== 0 && Date.now() - heading < WAIT_MS;
  heading = 0;
  return asked;
}

/** Asks the Participants list to take the focus: the shown one now, else the next one. */
export function focusParticipants(): void {
  const shown = document.querySelector<HTMLElement>('ul[aria-label="Participants"]');
  if (shown) {
    window.setTimeout(() => shown.focus(), 0);
    return;
  }
  participants = Date.now();
}

/** Whether a Participants list that just appeared takes the focus (once, and only soon after). */
export function takeParticipantsFocus(): boolean {
  const asked = participants !== 0 && Date.now() - participants < WAIT_MS;
  participants = 0;
  return asked;
}
