// The connector's job list as the page shows it (`specs/0132-connector-job-queue` UX notes): the
// rows, who may press what, the selection and the keys that move a job. Pure, so the tests load it
// without a bundler; the host's `JobList` message is the only input.
import { type StatusMessage, formatCount, formatSize } from "./connectorText.ts";

/** One job of the list; `position` is 1-based among the waiting jobs, 0 for an interrupted one. */
export interface ListEntry {
  job_id: string;
  name: string;
  lines: number;
  bytes: number;
  sent_by: string;
  position: number;
  run_next?: boolean;
  interrupted_at?: number | null;
}

/** The connector's `JobList` message. */
export interface JobListMessage {
  type: "JobList";
  current: ListEntry | null;
  waiting: ListEntry[];
  note?: string | null;
}

/** The most jobs the connector holds in its list (`specs/0132` criterion 6). */
export const LIST_LIMIT = 20;

/** The names of a press in the list, as `connector_list_command` takes them. */
export type ListCall =
  | { kind: "move"; jobId: string; from: number; to: number }
  | { kind: "remove"; jobId: string }
  | { kind: "runNext"; jobId: string; on: boolean };

/** What the app's role lets it do with a row (Frage 257 A; UX notes section 3). */
export interface Rights {
  move: boolean;
  flag: boolean;
  remove: boolean;
}

/** The sender as a row says it: "this app" for the app's own job. */
export function senderText(sentBy: string, me: string | null): string {
  return me !== null && sentBy === me ? "this app" : sentBy;
}

/** The second line of a row: "412 lines, 4.2 KiB, Phone". */
export function countsLine(entry: ListEntry, me: string | null): string {
  return `${formatCount(entry.lines)} lines, ${formatSize(entry.bytes)}, ${senderText(entry.sent_by, me)}`;
}

/** The interrupted row's second line: "Interrupted at line 1,234, Phone". */
export function interruptedLine(entry: ListEntry, me: string | null): string {
  return `Interrupted at line ${formatCount(entry.interrupted_at ?? 0)}, ${senderText(entry.sent_by, me)}`;
}

/** What a row says to a screen reader (UX notes section 7). */
export function rowText(entry: ListEntry, total: number): string {
  const base = `${entry.name}, ${formatCount(entry.lines)} lines, ${formatSize(entry.bytes)}, from ${entry.sent_by}`;
  if (entry.interrupted_at != null) return `${base}, interrupted at line ${formatCount(entry.interrupted_at)}`;
  const place = `${base}, ${entry.position} of ${total}`;
  return entry.run_next ? `${place}, runs next automatically` : place;
}

/** What a viewer may do with a row: an owner everything, a member the flag and Remove of its own
 * waiting job, nothing on an interrupted one but Remove for an owner. */
export function rights(owner: boolean, me: string | null, entry: ListEntry): Rights {
  const interrupted = entry.interrupted_at != null;
  const own = me !== null && entry.sent_by === me;
  return {
    move: owner && !interrupted,
    flag: !interrupted && (owner || own),
    remove: owner || (own && !interrupted),
  };
}

/** The jobs that wait, in order (an interrupted job first, outside the order). */
export function ordered(list: JobListMessage | null): ListEntry[] {
  return list === null ? [] : list.waiting.filter((e) => e.interrupted_at == null);
}

/** The first waiting job: what Start takes (criterion 4). */
export function firstWaiting(list: JobListMessage | null): ListEntry | null {
  return ordered(list)[0] ?? null;
}

/** "Start box.gcode": the Start slot's name when it takes the first waiting job, the name cut at
 * 40 characters (UX notes section 5). */
export function startName(entry: ListEntry): string {
  const name = entry.name.length > 40 ? `${entry.name.slice(0, 37)}...` : entry.name;
  return `Start ${name}`;
}

/** Whether the list is full: Send is greyed with "List full" (UX notes section 5, J10). */
export function listFull(list: JobListMessage | null): boolean {
  return list !== null && list.waiting.length >= LIST_LIMIT;
}

/** The selection after the list changed: its job stays; else the row now at its place (the last at
 * the end); `null` for an empty order. */
export function keepSelection(
  before: JobListMessage | null,
  after: JobListMessage | null,
  selected: string | null,
): string | null {
  const now = ordered(after);
  if (now.length === 0) return null;
  if (selected !== null && now.some((e) => e.job_id === selected)) return selected;
  const place = ordered(before).findIndex((e) => e.job_id === selected);
  if (place < 0) return now[0].job_id;
  return now[Math.min(place, now.length - 1)].job_id;
}

/** The target place of a move key or button for the selected job (1-based), `null` when it does not
 * move (UX notes section 4: one place, or first or last). */
export function moveTo(order: ListEntry[], selected: string, how: "up" | "down" | "first" | "last"): number | null {
  const at = order.findIndex((e) => e.job_id === selected);
  if (at < 0) return null;
  const target = how === "up" ? at - 1 : how === "down" ? at + 1 : how === "first" ? 0 : order.length - 1;
  if (target < 0 || target >= order.length || target === at) return null;
  return target + 1;
}

/** The cause a greyed move button gives (UX notes section 2). */
export function moveCause(order: ListEntry[], selected: string | null, how: "up" | "down"): string | null {
  const at = order.findIndex((e) => e.job_id === selected);
  if (at < 0) return "Interrupted";
  if (how === "up" && at === 0) return "First in the list";
  if (how === "down" && at === order.length - 1) return "Last in the list";
  return null;
}

/** Where a drag lands: the gap index (0 to n) nearest to the pointer's `y` among the row tops. */
export function dropGap(rowMiddles: number[], y: number): number {
  const at = rowMiddles.findIndex((m) => y < m);
  return at < 0 ? rowMiddles.length : at;
}

/** The 1-based target place of a drop of the row at place `from` into `gap`, `null` for no move. */
export function dropPlace(from: number, gap: number): number | null {
  const at = from - 1;
  const target = gap > at ? gap - 1 : gap;
  return target === at ? null : target + 1;
}

/** The hold that starts a drag on a touch screen, in ms (UX notes section 4). */
export const HOLD_MS = 400;
const DRAG_PX = 4;

/** Whether a press on the grip is a drag yet: a pointer after 4 px, a finger after the hold (it
 * moves nothing earlier: a move before the hold cancels, so the list can scroll). */
export function dragDecision(pointerType: string, heldMs: number, travelPx: number): "wait" | "drag" | "cancel" {
  if (pointerType !== "touch") return travelPx >= DRAG_PX ? "drag" : "wait";
  if (heldMs >= HOLD_MS) return "drag";
  return travelPx >= DRAG_PX ? "cancel" : "wait";
}

/** The sr-only line for this app's own new job: "Added to the job list: <name>, <k> of <n>" with
 * its place among the waiting jobs; `null` when nothing of this app's was added. `sending`: this
 * app's send is under way, so a first list is the result of it. */
export function addedAnnouncement(
  before: JobListMessage | null,
  after: JobListMessage | null,
  me: string | null,
  sending: boolean,
): string | null {
  if (after === null || me === null || (before === null && !sending)) return null;
  const known = new Set((before?.waiting ?? []).map((e) => e.job_id));
  const waiting = ordered(after);
  const place = waiting.findIndex((e) => e.sent_by === me && !known.has(e.job_id));
  return place < 0 ? null : `Added to the job list: ${waiting[place].name}, ${place + 1} of ${waiting.length}`;
}

/** The row that takes the focus after `removed` left `before` (the rows as they were) for `after`:
 * the row now at its place, the new last row when it was last; `null` when no row is left (the
 * focus then goes to the toolbar, UX notes section 6). */
export function focusAfterRemoval(before: ListEntry[], after: ListEntry[], removed: string): string | null {
  const at = before.findIndex((e) => e.job_id === removed);
  if (at < 0 || after.length === 0 || after.some((e) => e.job_id === removed)) return null;
  return after[Math.min(at, after.length - 1)].job_id;
}

/** Whether an armed Remove still targets the same row: its job is in `after` at the place it had in
 * `before` (UX notes section 3: it disarms when the job's place or presence changes). */
export function armHolds(before: JobListMessage | null, after: JobListMessage | null, armed: string): boolean {
  const was = before?.waiting.find((e) => e.job_id === armed);
  const now = after?.waiting.find((e) => e.job_id === armed);
  return was !== undefined && now !== undefined && was.position === now.position;
}

/** The sr-only line for a move of this app: "box.gcode, 1 of 3"; `null` when the job is not waiting. */
export function movedAnnouncement(after: JobListMessage | null, jobId: string): string | null {
  const waiting = ordered(after);
  const place = waiting.findIndex((e) => e.job_id === jobId);
  return place < 0 ? null : `${waiting[place].name}, ${place + 1} of ${waiting.length}`;
}

/** Whether this app's send shows as the list's last row: only while another job is held, where
 * the job it sends waits behind it; with nothing held the job is the current one. */
export function showsSendingRow(status: StatusMessage | null): boolean {
  return ["Ready", "AwaitingLocalStart", "Running", "Paused"].includes(status?.state.name ?? "");
}
