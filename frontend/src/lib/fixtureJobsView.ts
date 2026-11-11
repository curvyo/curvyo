/**
 * The one decoder of the Fixture jobs section's JSON (`specs/0103-fixture-jobs` `adrs.md`
 * section 6): `curvyo-editor-wasm/src/session/fixtures/jobs/view_json.rs` builds it, and every
 * field is checked here at runtime. No cast: a value of the wrong shape decodes to `null`, and the
 * section shows nothing rather than a guess. A job kind this build does not know is a job with no
 * kind (an Unknown job: rename and delete only). The Rust contract test reads the same fixtures
 * (`curvyo-editor-wasm/tests/fixtures/fixture_jobs_view/`).
 */
import type { FixtureKind } from "@/lib/fixturesView";

/** The five kinds a job can have (`0101`'s symbols). */
export const JOB_KINDS = ["cut", "score", "engrave", "engrave_outline", "cut_out"] as const;
export type JobKindText = (typeof JOB_KINDS)[number];

/** A known job kind: its stored text and its word ("Engrave + Outline"). */
export interface JobKind {
  text: JobKindText;
  word: string;
}

/** The state of a part in a job, as the canvas draws it (criterion 18). */
export type PartState = "running" | "output_off" | "show_off" | "not_on_bed" | "unticked";
const PART_STATES: readonly PartState[] = ["running", "output_off", "show_off", "not_on_bed", "unticked"];

/** One job's row (criteria 4, 15, 21). */
export interface JobRow {
  /** The job's full id text: the calls take it back as it is. */
  id: string;
  name: string;
  /** `null` for a job whose kind this build does not know. */
  kind: JobKind | null;
  show: boolean;
  output: boolean;
  /** Whether its fixture is Active; `false` reads "not on the bed". */
  fixtureActive: boolean;
  /** Whether it runs (the run rule's answer). */
  runs: boolean;
  partsTicked: number;
  partsTotal: number;
  /** Its place among its fixture's jobs, from 0. */
  position: number;
  /** How many jobs its fixture has. */
  of: number;
}

/** One fixture's group: its heading and its jobs in order. */
export interface JobGroup {
  fixture: string;
  name: string;
  kind: FixtureKind;
  jobs: JobRow[];
}

/** One part's row in the open Parts list. */
export interface PartRow {
  /** The part's token (`board`, `slot:0`, `grid.lines`, ...), sent back by a tick or a Select. */
  token: string;
  name: string;
  ticked: boolean;
  /** The node "Select" selects: the slot, else the fixture. */
  select: string;
  state: PartState;
  /** The job's kind while the part runs; `null` otherwise. */
  kind: JobKindText | null;
}

/** The open Parts list of the selected job. */
export interface PartsView {
  job: string;
  rows: PartRow[];
}

/** The fixture the five add buttons add to. */
export interface AddTarget {
  fixture: string;
  name: string;
}

/** The whole Fixture jobs section of the active laser page. */
export interface FixtureJobsView {
  /** The jobs, Unknown ones included: the header "Fixture jobs (N)". */
  count: number;
  /** The one selected fixture; `null` hides the add buttons. */
  addTo: AddTarget | null;
  groups: JobGroup[];
  selected: string | null;
  parts: PartsView | null;
  /** How many jobs run: the summary's "1 fixture job runs.". */
  running: number;
  runningFixtures: string[];
  /** The damaged values of all the page's fixture jobs (criterion 22). */
  damaged: number;
  /** One notice per fixture with damaged job values: "1 value damaged on Holder 1 job.". */
  damageNotices: string[];
}

type Json = Record<string, unknown>;

const FIXTURE_KINDS: readonly FixtureKind[] = ["grid", "holder", "clamp", "stop", "screw"];

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asCount(value: unknown): number | null {
  return Number.isSafeInteger(value) && (value as number) >= 0 ? (value as number) : null;
}

function asOptionalText(value: unknown): string | null | undefined {
  if (value === null) return null;
  return typeof value === "string" ? value : undefined;
}

function asJobKind(value: unknown): JobKind | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || typeof value.word !== "string") return undefined;
  const text = JOB_KINDS.find((known) => known === value.text);
  // A kind text this build does not know: an Unknown job, not a broken row.
  return text === undefined ? null : { text, word: value.word };
}

function asPartKind(value: unknown): JobKindText | null | undefined {
  if (value === null) return null;
  const text = JOB_KINDS.find((known) => known === value);
  return text ?? null;
}

function asFixtureKind(value: unknown): FixtureKind {
  return FIXTURE_KINDS.find((known) => known === value) ?? "unknown";
}

function asJob(value: unknown, position: number): JobRow | null {
  if (!isObject(value)) return null;
  const kind = asJobKind(value.kind);
  const ticked = asCount(value.parts_ticked);
  const total = asCount(value.parts_total);
  const of = asCount(value.of);
  const at = asCount(value.position);
  if (
    kind === undefined ||
    typeof value.id !== "string" ||
    typeof value.name !== "string" ||
    typeof value.show !== "boolean" ||
    typeof value.output !== "boolean" ||
    typeof value.fixture_active !== "boolean" ||
    typeof value.runs !== "boolean" ||
    ticked === null ||
    total === null ||
    of === null ||
    at !== position
  ) {
    return null;
  }
  return {
    id: value.id,
    name: value.name,
    kind,
    show: value.show,
    output: value.output,
    fixtureActive: value.fixture_active,
    runs: value.runs,
    partsTicked: ticked,
    partsTotal: total,
    position,
    of,
  };
}

function asGroup(value: unknown): JobGroup | null {
  if (!isObject(value) || typeof value.fixture !== "string" || typeof value.name !== "string") {
    return null;
  }
  if (!Array.isArray(value.jobs)) return null;
  const jobs: JobRow[] = [];
  for (const raw of value.jobs) {
    const job = asJob(raw, jobs.length);
    if (job === null) return null;
    jobs.push(job);
  }
  return { fixture: value.fixture, name: value.name, kind: asFixtureKind(value.kind), jobs };
}

function asAddTarget(value: unknown): AddTarget | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || typeof value.fixture !== "string" || typeof value.name !== "string") {
    return undefined;
  }
  return { fixture: value.fixture, name: value.name };
}

function asPartRow(value: unknown): PartRow | null {
  if (!isObject(value)) return null;
  const kind = asPartKind(value.kind);
  const state = PART_STATES.find((known) => known === value.state);
  if (
    kind === undefined ||
    state === undefined ||
    typeof value.token !== "string" ||
    typeof value.name !== "string" ||
    typeof value.select !== "string" ||
    typeof value.ticked !== "boolean"
  ) {
    return null;
  }
  return { token: value.token, name: value.name, ticked: value.ticked, select: value.select, state, kind };
}

function asParts(value: unknown): PartsView | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || typeof value.job !== "string" || !Array.isArray(value.rows)) {
    return undefined;
  }
  const rows: PartRow[] = [];
  for (const raw of value.rows) {
    const row = asPartRow(raw);
    if (row === null) return undefined;
    rows.push(row);
  }
  return { job: value.job, rows };
}

/**
 * The view in `value` (already parsed JSON), or `null`: `null` itself (a page that is not an
 * active laser page), or any value that is not the view.
 */
export function decodeFixtureJobsView(value: unknown): FixtureJobsView | null {
  if (!isObject(value) || !Array.isArray(value.groups) || !Array.isArray(value.running_fixtures)) {
    return null;
  }
  const groups: JobGroup[] = [];
  for (const raw of value.groups) {
    const group = asGroup(raw);
    if (group === null) return null;
    groups.push(group);
  }
  const addTo = asAddTarget(value.add_to);
  const parts = asParts(value.parts);
  const selected = asOptionalText(value.selected);
  const count = asCount(value.count);
  const running = asCount(value.running);
  const damaged = asCount(value.damaged);
  if (
    addTo === undefined ||
    parts === undefined ||
    selected === undefined ||
    count === null ||
    running === null ||
    damaged === null ||
    !Array.isArray(value.damage_notices) ||
    !value.damage_notices.every((text) => typeof text === "string") ||
    !value.running_fixtures.every((id) => typeof id === "string")
  ) {
    return null;
  }
  return {
    count,
    addTo,
    groups,
    selected,
    parts,
    running,
    runningFixtures: value.running_fixtures as string[],
    damaged,
    damageNotices: value.damage_notices as string[],
  };
}

/** The view a JSON text holds; `null` for the text `null`, bad JSON or any other value. */
export function decodeFixtureJobsText(text: string): FixtureJobsView | null {
  try {
    return decodeFixtureJobsView(JSON.parse(text));
  } catch {
    return null;
  }
}
