// Which keys the pen window passes on (`specs/0158-pen-display` slice 2a criterion 22, UX 9.2):
// the shared ones work; letters of tools the pen window does not have, and the file chords other
// than Ctrl+S, do nothing.

/** The tool letters of the pen window: Pencil, Select, Node. */
const PEN_TOOL_KEYS = new Set(["p", "s", "n"]);
/** The chords that still work with Ctrl: undo, redo. */
const PEN_CTRL_KEYS = new Set(["z", "y"]);

/** Whether `key` (with Ctrl or Cmd as `ctrl`) reaches the canvas in the pen window. */
export function penKeyAllowed(key: string, ctrl: boolean, alt: boolean): boolean {
  if (key.length !== 1) return true;
  const letter = key.toLowerCase();
  if (!/[a-z]/.test(letter)) return !ctrl;
  if (ctrl) return PEN_CTRL_KEYS.has(letter);
  if (alt) return false;
  return PEN_TOOL_KEYS.has(letter);
}
