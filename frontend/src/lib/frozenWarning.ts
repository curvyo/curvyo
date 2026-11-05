// The warning inside Make admin's confirm while the room is frozen (`specs/0097` criterion 28):
// crossed demotions, mutual removals or one refused act, worded in Rust.

import type { FrozenRoom } from "@/lib/membershipState";
import { membership_frozen_demotion, membership_frozen_warning } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The warning, "" when the room is not frozen. */
export function frozenWarning(frozen: FrozenRoom | null): string {
  if (!frozen) {
    return "";
  }
  if (frozen.demoted && frozen.mutual.length >= 2) {
    return membership_frozen_demotion(JSON.stringify(frozen.mutual));
  }
  return membership_frozen_warning(JSON.stringify(frozen.mutual), frozen.refused);
}
