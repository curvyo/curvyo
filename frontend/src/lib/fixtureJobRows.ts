/**
 * The Fixture jobs section's row logic (`specs/0103-fixture-jobs` `ux-notes.md` sections 3 to 5,
 * 7, 13): the words of the lines and names, the keys, where the arrows and a drop may go, and the
 * Parts list's rules. Pure: the components only draw what these return, and the session holds the
 * rules that decide what runs.
 */
import type { FixtureJobsView, JobGroup, JobKindText, JobRow, PartsView } from "@/lib/fixtureJobsView";

/** The five add buttons in the notes' order (section 3.3), with their words. */
export const ADD_KINDS: ReadonlyArray<readonly [JobKindText, string]> = [
  ["cut", "Cut"],
  ["cut_out", "Cut-out"],
  ["score", "Score"],
  ["engrave", "Engrave"],
  ["engrave_outline", "Engrave + Outline"],
];

/** The add button's name: it carries the fixture it adds to (section 3.3). */
export function addLabel(kind: JobKindText, fixtureName: string): string {
  const word = ADD_KINDS.find(([known]) => known === kind)?.[1] ?? kind;
  return `Add ${word} job to ${fixtureName}`;
}

/** The section header's text: "Fixture jobs (N)", unknown jobs included (section 3.1). */
export function sectionTitle(count: number): string {
  return `Fixture jobs (${count})`;
}

/** "no part ticked", "1 part", "2 parts" (section 4.1, line 2). */
export function partCountText(ticked: number): string {
  if (ticked === 0) return "no part ticked";
  return ticked === 1 ? "1 part" : `${ticked} parts`;
}

/** The job's second line: the kind word, then the first of not on the bed, Output off, the parts. */
export function jobLine2(row: JobRow): string {
  if (row.kind === null) return "Unknown fixture job";
  const rest = !row.fixtureActive ? "not on the bed" : !row.output ? "Output off" : partCountText(row.partsTicked);
  return `${row.kind.word} · ${rest}`;
}

/** The row's accessible name: "<name>, <kind>, <n> parts" plus the state words (section 13). */
export function jobAccessibleName(row: JobRow): string {
  if (row.kind === null) return `${row.name}, unknown fixture job`;
  const n = row.partsTicked;
  const parts = n === 1 ? "1 part" : `${n} parts`;
  const state = !row.fixtureActive ? ", not on the bed" : !row.output ? ", Output off" : "";
  return `${row.name}, ${row.kind.word}, ${parts}${state}`;
}

/** The Parts list's count: "2 of 5", ticked of the readable parts (section 5.1). */
export function partsHeader(parts: PartsView): string {
  const ticked = parts.rows.filter((row) => row.ticked).length;
  return `${ticked} of ${parts.rows.length}`;
}

/**
 * The Parts list of `row`, or `null`: only the selected job shows one, and an Unknown job never
 * does (criterion 21: it can only be renamed or deleted).
 */
export function partsFor(view: FixtureJobsView, row: JobRow): PartsView | null {
  if (view.selected !== row.id || row.kind === null || view.parts === null) return null;
  return view.parts.job === row.id ? view.parts : null;
}

/** The tick's tooltip and name: "Assign to <job>" or "Remove from <job>" (section 5.3). */
export function tickTitle(ticked: boolean, jobName: string): string {
  return ticked ? `Remove from ${jobName}` : `Assign to ${jobName}`;
}

/** The key of a job's armed trash (`useDeleteArm`): unique over the section. */
export function armKey(id: string): string {
  return `fjob:${id}`;
}

/** The jobs of all groups in section order, the order of the list and of the arrows' focus. */
export function flatJobs(groups: JobGroup[]): JobRow[] {
  return groups.flatMap((group) => group.jobs);
}

/**
 * The place an arrow or Alt-key moves a job to inside its own group, or `null` where it may not
 * go (the group's ends, or an id that is not there): a job never leaves its group (section 4.6).
 */
export function jobNeighbour(groups: JobGroup[], id: string, step: -1 | 1): number | null {
  for (const group of groups) {
    const at = group.jobs.findIndex((row) => row.id === id);
    if (at === -1) continue;
    const to = at + step;
    return to < 0 || to >= group.jobs.length ? null : to;
  }
  return null;
}

/**
 * Where a drag inside a group lands (section 4.6): `middles` are the group's rows' midpoints in
 * order, `source` the dragged row's place. Down lands past the last midpoint the pointer passed,
 * up above the first midpoint the pointer is over; otherwise the row stays where it is.
 */
export function landingPosition(source: number, middles: number[], pointerY: number): number {
  let at = source;
  middles.forEach((middle, index) => {
    if (index > source && pointerY >= middle) at = index;
    if (index < source && pointerY < middle) at = Math.min(at, index);
  });
  return at;
}

/** The summary line of the work when nothing runs (`operationRows.ts`). */
const NOTHING_RUNS = "Nothing runs. Assign items to an operation.";

/** The summary's fixture line, or `null` when no job runs (section 7). */
export function runningLine(running: number): string | null {
  if (running <= 0) return null;
  return running === 1 ? "1 fixture job runs." : `${running} fixture jobs run.`;
}

/** How many work operations run, read from the work line "Runs N operations, M items.". */
function workRunsOf(lines: ReadonlyArray<{ text: string }>): number {
  const match = /^Runs (\d+) operation/.exec(lines.find((line) => line.text.startsWith("Runs "))?.text ?? "");
  return match ? Number(match[1]) : 0;
}

/**
 * The summary's lines with the fixture line in place (section 7, criterion 17, J7): the work line
 * only when an operation runs, then the fixture line when a job runs, then the other lines; with
 * neither, "Nothing runs. Assign items to an operation.". `workRunning` defaults to the count the
 * work line reads. The fixture line is not a button (`count` null): its count button needs a
 * multi-select the session does not offer yet.
 */
export function summaryWithFixtures<T extends { text: string; count: unknown }>(
  lines: T[],
  fixtureRunning: number,
  workRunning: number = workRunsOf(lines),
): Array<T | { text: string; count: null }> {
  const others = lines.filter((line) => !line.text.startsWith("Runs ") && line.text !== NOTHING_RUNS);
  const workLine = lines.find((line) => line.text.startsWith("Runs "));
  const fixture = runningLine(fixtureRunning);
  const lead: Array<T | { text: string; count: null }> = [];
  if (workRunning > 0 && workLine) lead.push(workLine);
  if (fixture !== null) lead.push({ text: fixture, count: null });
  if (lead.length === 0) lead.push({ text: NOTHING_RUNS, count: null });
  return [...lead, ...others];
}

/** The notice of a work-item drop onto a job row (`specs/0103` criterion 10, Proposal). */
export const WORK_REFUSED = "Work items do not run in fixture jobs.";

/** Above this many jobs the rows are windowed against the panel's scroll (`ux-notes.md` 14). */
export const WINDOW_ABOVE = 60;
/** The overscan of the window, px (as the Operations list). */
export const OVERSCAN = 120;
/** A group heading's height, and the divider above a group after the first, px. */
export const HEADING_PX = 28;
export const DIVIDER_PX = 9;
/** A job row's fixed height, px (40, as `ux-notes.md` 4.1); its open Parts list adds the rest. */
export const ROW_PX = 40;
export const PART_ROW_PX = 28;
export const PARTS_HEAD_PX = 24;
export const PARTS_FOOT_PX = 4;

/** One line of the windowed list: a group heading, or a job row with its open Parts list. */
export type Entry =
  | { kind: "heading"; fixture: string; height: number }
  | { kind: "row"; job: JobRow; height: number };

/** The windowed entries of all groups in order, and the entry index of each job. */
export function entriesOf(groups: JobGroup[], parts: PartsView | null, selected: string | null) {
  const entries: Entry[] = [];
  const rowAt = new Map<string, number>();
  groups.forEach((group, groupAt) => {
    entries.push({ kind: "heading", fixture: group.fixture, height: HEADING_PX + (groupAt > 0 ? DIVIDER_PX : 0) });
    for (const job of group.jobs) {
      const open = parts !== null && parts.job === job.id && selected === job.id;
      const height = ROW_PX + (open ? PARTS_HEAD_PX + PART_ROW_PX * parts.rows.length + PARTS_FOOT_PX : 0);
      rowAt.set(job.id, entries.length);
      entries.push({ kind: "row", job, height });
    }
  });
  return { entries, rowAt };
}
