// Reads the marker's ended record of a removed or left workspace from the host (`specs/0097`
// criterion 20); the facts are parsed by `endedRecord`.

import type { EndedMarker } from "@/lib/endedRecord";
import { invoke } from "@/platform/host";

/** The open workspace's ended record (or the synced workspace `workspace`'s), `null` when it
 * keeps none. */
export async function readWorkspaceEnded(workspace?: string): Promise<EndedMarker | null> {
  return invoke<EndedMarker | null>("workspace_marker_ended", workspace ? { workspace } : undefined).catch(() => null);
}
