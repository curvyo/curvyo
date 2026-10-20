// Frage 13 (FONT-BROWSERKEEP), decided by the customer 2026-10-20, option B
// (`specs/0075-font-library` "Open questions"): at the first browser Add font the page asks the
// browser to keep its storage (`navigator.storage.persist()`). Granted or refused, Add goes on;
// refused fonts may be evicted when storage is tight, and texts then show the missing-font mark.

let asked = false;

/** Asks once per page for persistent storage; never waits for the answer and never throws. */
export function requestFontStorageKeep(storage: StorageManager | undefined = globalThis.navigator?.storage): void {
  if (asked) {
    return;
  }
  asked = true;
  try {
    void storage?.persist?.().catch(() => false);
  } catch {
    // A browser without the call keeps the default behaviour.
  }
}
