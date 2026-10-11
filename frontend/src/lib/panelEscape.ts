/**
 * Whether an Escape that reached the Properties panel is the panel's to handle
 * (`specs/0017-style-panel-rework/` criterion 60, `specs/0047-transform-polish/` criterion 25).
 * A control that already acted on the key (a field that restored itself and returned the focus
 * where it belongs, such as the hex field opened from a colour slider) has called
 * `preventDefault`; the panel then leaves the focus alone instead of sending it to the canvas.
 */
export function panelHandlesEscape(event: {
  key: string;
  defaultPrevented: boolean;
}): boolean {
  return event.key === "Escape" && !event.defaultPrevented;
}
