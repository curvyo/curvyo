// Which window a page is and which host commands it may call (`specs/0158-pen-display` slice 2a
// criterion 21, ADR 0096 A8, A10); pure, so a test reads it without a host.

/** Which window a page is (ADR 0096 A10): `pen` for the pen window, opened as `?window=pen`;
 * `main` otherwise, and always in the browser build, which has no pen window. */
export type WindowRole = "main" | "pen";

/** The role of a page at `search` (the location's query) in this build. */
export function roleOf(search: string, desktop: boolean): WindowRole {
  return desktop && new URLSearchParams(search).get("window") === "pen" ? "pen" : "main";
}

/** The answer to a call the pen page may not make, as the host gives it. */
export const NOT_AVAILABLE_IN_THIS_WINDOW = "not-available-in-this-window";

/** The commands the pen page may call: the same list as `curvyo-app::window_commands`
 * (`specs/0158-pen-display` slice 2a criterion 21). */
export const PEN_ALLOWED: ReadonlySet<string> = new Set([
  "pen_watch",
  "pen_unwatch",
  "pen_compression",
  "bridge_watch",
  "bridge_send",
  "pen_window_flags",
  "font_folder_list",
  "system_font_list",
  "font_bytes",
  "read_formats_file",
  "read_extensions_file",
  "list_machine_profiles",
  "read_machine_choice",
  "list_materials",
  "read_material_order",
  "read_retired_test_ids",
  "read_test_grid_choice",
  "read_material_photo",
]);

/** Whether a page of `role` may call `command`. */
export function windowAllows(role: WindowRole, command: string): boolean {
  return role === "main" || PEN_ALLOWED.has(command);
}

