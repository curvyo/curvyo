// The workspaces Sync these could not switch on because of the server (`specs/0096-own-devices-sync`
// ux-notes 3.1): kept until a retry gets them in, the group's list names them, or the group ends;
// those a retry costs little are tried again as soon as the group is online again. The alert names
// why (PR #281 review): unreachable, refused by the server, or a share that failed here.

/** How one switch-on of Sync these ended: "offline" when the group itself was offline (nothing
 * was prepared or sent), "unreachable" when the room did not answer in time, "server-refused"
 * when the relay refused the connection, "share-failed" when sharing failed here before anything
 * was sent, "refused" for a local refusal before the workspace was prepared. */
export type SwitchOnResult =
  | "ok"
  | "offline"
  | "unreachable"
  | "server-refused"
  | "share-failed"
  | "no-server"
  | "member-only"
  | "refused";

/** Why the workspaces of one alert did not get in. */
export type SyncProblem = "unreachable" | "refused" | "local";

const PROBLEM: Partial<Record<SwitchOnResult, SyncProblem>> = {
  offline: "unreachable",
  unreachable: "unreachable",
  "server-refused": "refused",
  "share-failed": "local",
};

/** When a workspace that did not get in is tried again while the group stays online (ms). */
export const RETRY_MS = 30_000;

/** The workspaces whose switch-on waits for the server, with what a retry needs (`T`). */
export class UnreachableWorkspaces<T> {
  private readonly failed = new Map<string, { name: string; item: T; retry: boolean; problem: SyncProblem }>();
  private retryAt = 0;
  private retrying = false;
  private online = true;

  /** Records how the switch-on of `id` ended: the results of the server keep it. An "offline"
   * one is retried; an "unreachable" one only when `cheap` (a retry that does not prepare and
   * upload the workspace again); a refused or failed one waits for the maker. */
  record(id: string, name: string, item: T, result: SwitchOnResult, cheap: boolean): void {
    const problem = PROBLEM[result];
    if (problem) {
      this.failed.set(id, { name, item, retry: result === "offline" || (result === "unreachable" && cheap), problem });
    } else {
      this.failed.delete(id);
    }
  }

  /** One alert per problem with the names it concerns, so an outage shows one line; none while
   * the server address is unset (nothing to reach). */
  alerts(server: string): { problem: SyncProblem; names: string[] }[] {
    if (server === "") {
      return [];
    }
    const alerts: { problem: SyncProblem; names: string[] }[] = [];
    for (const { name, problem } of this.failed.values()) {
      const alert = alerts.find((a) => a.problem === problem);
      if (alert) {
        alert.names.push(name);
      } else {
        alerts.push({ problem, names: [name] });
      }
    }
    return alerts;
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
