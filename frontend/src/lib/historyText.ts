// The words and rules of undo and redo in the frontend (`specs/0020-undo-redo` criteria 1, 5, 6,
// 24): how long a notice stays, what the long-operation state says, which key presses are history
// keys. Pure, so `tests/historyText.test.ts` covers it; the session decides everything else.

/** The direction of a history command. */
export type HistoryDirection = "undo" | "redo";

/** How long a notice stays: 3 s, or 5 s above 70 characters (`0016` criterion 29). */
export function noticeDurationMs(text: string): number {
  return text.length > 70 ? 5000 : 3000;
}

/** The hint of an undo or redo while the Pen holds an unfinished path (criterion 6e). */
export const FINISH_PEN_NOTICE = "Finish the path (Enter) or cancel it (Esc) first.";

/** How long that hint stays. */
export const FINISH_PEN_NOTICE_MS = 2000;

/** After this long the long-operation state names itself in the notice slot. */
export const WORKING_NOTICE_DELAY_MS = 150;

/** The notice of the long-operation state. */
export function workingNotice(direction: HistoryDirection): string {
  return direction === "undo" ? "Undo: working..." : "Redo: working...";
}

/** A second trigger of the same direction within this long is the same press arriving by both
 * paths, the key event and the native menu accelerator (criterion 8: exactly one handler fires). */
export const DUPLICATE_PRESS_MS = 25;

/**
 * Whether a key press is undo or redo: Ctrl (Cmd) with the typed character z (Shift: redo) or,
 * off macOS, y. The character is matched, not the key's place, so a German keyboard works
 * (criterion 7). The session applies its gate; this only says whether to ask it.
 */
export function historyKeyDirection(event: {
  key: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
}, mac: boolean): HistoryDirection | null {
  if (!(event.ctrlKey || event.metaKey) || event.altKey) {
    return null;
  }
  const key = event.key.toLowerCase();
  if (key === "z") {
    return event.shiftKey ? "redo" : "undo";
  }
  if (key === "y" && !event.shiftKey && !mac) {
    return "redo";
  }
  return null;
}

/** Whether the platform is macOS (Cmd+Y is not a redo key there). */
export function isMac(platform: string): boolean {
  return /mac|iphone|ipad/i.test(platform);
}

/** The text of a notice the session returned, or `null` when it returned none. */
export function noticeText(raw: string): string | null {
  return raw === "" ? null : raw;
}

/** The names of the next undo and redo for the native Edit menu: `null` when a stack is empty. */
export function menuNames(undo: string, redo: string): { undo: string | null; redo: string | null } {
  return { undo: undo === "" ? null : undo, redo: redo === "" ? null : redo };
}
