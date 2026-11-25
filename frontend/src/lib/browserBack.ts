// When a key or button press means Back in the library browser (`specs/0152-library-browser`
// criterion 30, UX notes 0.3): Alt+Left, Cmd+[ on a Mac and the mouse's back button, anywhere in
// the app while the pane is open, except in a text field outside the pane (it keeps its own
// Alt+Left and Cmd+[ for the text).

/** What the decision needs of a key event. */
export interface BackKey {
  key: string;
  altKey: boolean;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
}

/** Whether `event` is the Back key: Alt+Left or Cmd+[ with no other modifier. */
export function isBackKey(event: BackKey): boolean {
  if (event.shiftKey || event.ctrlKey) {
    return false;
  }
  return (event.altKey && !event.metaKey && event.key === "ArrowLeft") || (event.metaKey && !event.altKey && event.key === "[");
}

/** Whether a Back key or the mouse's back button goes Back: the pane is open, and the target is
 * not a text field outside the pane. */
export function backApplies(paneOpen: boolean, targetInPane: boolean, targetIsTextField: boolean): boolean {
  return paneOpen && (targetInPane || !targetIsTextField);
}

/** Whether `target` is a text field (an input that takes text, a text area, an editable element). */
export function isTextField(target: EventTarget | null): boolean {
  if (typeof HTMLElement === "undefined" || !(target instanceof HTMLElement)) {
    return false;
  }
  if (target.isContentEditable || target instanceof HTMLTextAreaElement) {
    return true;
  }
  return (
    target instanceof HTMLInputElement &&
    !["button", "checkbox", "radio", "range", "submit", "reset", "color", "file", "image"].includes(target.type)
  );
}
