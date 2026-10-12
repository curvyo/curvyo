// Whether an Open or Save dialog, or a write, is running (`specs/0051-sidebar-and-no-menu/`
// criterion 26 and 33). Its answer belongs to the project that was open when it started, so
// nothing that ends that project may start meanwhile: New, Close, Quit, a file handed over by the
// system and a window close wait, and a second Open or Save does not start. Pure, so
// `tests/busyGate.test.ts` covers the sequence.

/** One flag with an entry that fails while it is set. */
export interface BusyGate {
  /** Whether a dialog or a write is running. */
  isBusy: () => boolean;
  /** Starts one; `false` and no change when one is already running. */
  enter: () => boolean;
  /** The dialog or write ended, whatever its outcome. */
  leave: () => void;
}

/** A gate that starts free. */
export function createBusyGate(): BusyGate {
  let busy = false;
  return {
    isBusy: () => busy,
    enter: () => {
      if (busy) {
        return false;
      }
      busy = true;
      return true;
    },
    leave: () => {
      busy = false;
    },
  };
}
