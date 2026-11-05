// The workspaces Sync these could not switch on because the server did not answer
// (`specs/0096-own-devices-sync` ux-notes 3.1): kept until a retry gets them in, the group's list
// names them, or the group ends; those a retry costs little are tried again as soon as the group is
// online again.

/** How one switch-on of Sync these ended: "offline" when the group itself was offline (nothing
 * was prepared or sent), "unreachable" when the room did not answer in time. */
export type SwitchOnResult = "ok" | "offline" | "unreachable" | "no-server" | "member-only" | "refused";

/** When a workspace that did not get in is tried again while the group stays online (ms). */
export const RETRY_MS = 30_000;

/** The workspaces whose switch-on waits for the server, with what a retry needs (`T`). */
export class UnreachableWorkspaces<T> {
  private readonly failed = new Map<string, { name: string; item: T; retry: boolean }>();
  private retryAt = 0;
  private retrying = false;
  private online = true;

  /** Records how the switch-on of `id` ended: only "offline" and "unreachable" keep it. An
   * "offline" one is retried; an "unreachable" one only when `cheap` (a retry that does not
   * prepare and upload the workspace again). */
  record(id: string, name: string, item: T, result: SwitchOnResult, cheap: boolean): void {
    if (result === "offline" || result === "unreachable") {
      this.failed.set(id, { name, item, retry: result === "offline" || cheap });
    } else {
      this.failed.delete(id);
    }
  }

  /** The names for the one alert; none while the server address is unset (nothing to reach). */
  names(server: string): string[] {
    return server === "" ? [] : [...this.failed.values()].map((f) => f.name);
  }

  /** Whether `id` waits for the server. */
  has(id: string): boolean {
    return this.failed.has(id);
  }

  /** Drops the workspaces the group's list names now (switched on or off on some device). */
  forget(ids: Iterable<string>): boolean {
    let changed = false;
    for (const id of ids) {
      changed = this.failed.delete(id) || changed;
    }
    return changed;
  }

  /** Drops every workspace (this device left or was removed); whether any waited. */
  clear(): boolean {
    const any = this.failed.size > 0;
    this.failed.clear();
    return any;
  }

  /** The network is back: the next look retries at once. */
  wake(): void {
    this.retryAt = 0;
  }

  /** What to retry now: every retried one that waits, once the group came online again or the
   * retry time passed; nothing while a retry runs or the group is offline. */
  due(online: boolean, now: number): T[] {
    const cameOnline = online && !this.online;
    this.online = online;
    const items = [...this.failed.values()].filter((f) => f.retry).map((f) => f.item);
    if (!online || this.retrying || items.length === 0 || (!cameOnline && now < this.retryAt)) {
      return [];
    }
    this.retrying = true;
    return items;
  }

  /** A retry is over; the next one waits [`RETRY_MS`]. */
  settled(now: number): void {
    this.retrying = false;
    this.retryAt = now + RETRY_MS;
  }
}
