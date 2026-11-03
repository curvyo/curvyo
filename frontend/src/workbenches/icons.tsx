import { createElement } from "react";

import { glyphOf } from "@/workbenches/iconNames";

/** The 16 px glyph called `name`, at 1.5 px absolute stroke (`docs/design-system.md`). */
export function WorkbenchIcon({ name }: { name: string }) {
  return createElement(glyphOf(name), {
    size: 16,
    strokeWidth: 1.5,
    absoluteStrokeWidth: true,
    "aria-hidden": true,
  });
}
