// Removes a throw-away browser profile. Killing the browser's main process leaves its helper
// processes writing into the profile for a moment, so the removal retries; a profile that is
// still busy afterwards is scratch data in a temporary folder and never fails a test.
import { rmSync } from "node:fs";

export function removeProfile(profile: string): void {
  try {
    rmSync(profile, { recursive: true, force: true, maxRetries: 20, retryDelay: 500 });
  } catch (error) {
    console.warn(`browser profile ${profile} not removed: ${String(error)}`);
  }
}
