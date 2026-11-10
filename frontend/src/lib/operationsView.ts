/**
 * The one decoder of the Operations tab's JSON (`specs/0101-laser-operations` `adrs.md`
 * decision 6): `curvyo-editor-wasm/src/session/operations/view_json.rs` builds it, and every
 * field is checked here at runtime. No cast: a value of the wrong shape or with an enum text
 * this build does not know decodes to `null`, and the tab shows nothing rather than a guess. The
 * Rust contract test reads the same fixtures (`curvyo-editor-wasm/tests/fixtures/operations_view/`).
 */

/** The node id as the session sends it: the JSON text is what the calls take back. */
export interface OperationId {
  peer: string;
  counter: number;
}

/** The five kinds a row can have. */
export const OPERATION_KINDS = ["cut", "score", "engrave", "engrave_outline", "cut_out"] as const;

/** A row's kind; `unknown` for a kind this build does not know (criterion 26). */
export type OperationKind = (typeof OPERATION_KINDS)[number] | "unknown";

/** The two sections of the list with Cuts last on. */
export type OperationSection = "marking" | "cutting";

/** A row's tick over the selection: none, some (mixed) or all of it. */
export type TickState = "none" | "some" | "all";

/** One operation's row, in run order. */
export interface OperationRow {
  id: OperationId;
  name: string;
  kind: OperationKind;
  /** `null` for an unknown operation. */
  section: OperationSection | null;
  show: boolean;
  output: boolean;
  marginMm: number;
  items: number;
  tick: TickState;
}

/** The summary counts (criterion 17). Nothing here counts a missing object (Frage 169 A). */
export interface OperationsSummary {
  operations: number;
  running: number;
  notAssigned: number;
  runsWhileHidden: number;
  outputOff: number;
}

/** The four counts of the summary, in the order the session lists them. */
export const COUNT_KEYS = ["running", "notAssigned", "runsWhileHidden", "outputOff"] as const;

/** A summary count's key. */
export type CountKey = (typeof COUNT_KEYS)[number];

/** The items behind each count, each once: what a count button selects (criterion 17). */
export type CountItems = Record<CountKey, OperationId[]>;

/** The name each count has in the session's calls (`select_operation_items`). */
export const COUNT_WIRE: Record<CountKey, string> = {
  running: "running",
  notAssigned: "not_assigned",
  runsWhileHidden: "runs_while_hidden",
  outputOff: "output_off",
};

/** One clone member row: read-only, assignable by path through its clone (criterion 11). */
export interface CloneMember {
  clone: OperationId;
  cloneName: string;
  member: OperationId;
  name: string;
  /** The operations that hold this member through its clone. */
  operations: OperationId[];
}

/** How the output page is drawn (criterion 18). */
export type ViewMode = "operations" | "drawn";

/** The whole Operations tab of the active laser page. */
export interface OperationsView {
  cutsLast: boolean;
  rows: OperationRow[];
  summary: OperationsSummary;
  items: CountItems;
  cloneMembers: CloneMember[];
  viewMode: ViewMode;
}

type Json = Record<string, unknown>;

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asId(value: unknown): OperationId | null {
  if (!isObject(value)) return null;
  if (typeof value.peer !== "string") return null;
  if (!Number.isSafeInteger(value.counter) || (value.counter as number) < 0) return null;
  return { peer: value.peer, counter: value.counter as number };
}

/** The node id a call sends back: the JSON text the session's bridge reads. */
export function operationIdText(id: OperationId): string {
  return JSON.stringify({ peer: id.peer, counter: id.counter });
}

/** A node id from the JSON text a call returns; `null` for anything else. */
export function decodeOperationId(text: string): OperationId | null {
  try {
    return asId(JSON.parse(text));
  } catch {
    return null;
  }
}

/** The id as one string, for comparisons and React keys. */
export function operationIdKey(id: OperationId): string {
  return `${id.peer}:${id.counter}`;
}

function asCount(value: unknown): number | null {
  return Number.isSafeInteger(value) && (value as number) >= 0 ? (value as number) : null;
}

/** A kind this build knows, else `unknown` (criterion 26, adrs.md decision 6: a kind string outside the contract is never a failed decode, as in Rust). */
function asKind(value: unknown): OperationKind {
  return typeof value === "string" && (OPERATION_KINDS as readonly string[]).includes(value)
    ? (value as OperationKind)
    : "unknown";
}

function asSection(value: unknown): OperationSection | null | undefined {
  if (value === null) return null;
  if (value === "marking" || value === "cutting") return value;
  return undefined;
}


function asTick(value: unknown): TickState | null {
  return value === "none" || value === "some" || value === "all" ? value : null;
}

function asRow(value: unknown): OperationRow | null {
  if (!isObject(value)) return null;
  const id = asId(value.id);
  const kind = asKind(value.kind);
  const section = asSection(value.section);
  const items = asCount(value.items);
  const tick = asTick(value.tick);
  if (
    id === null ||
    typeof value.name !== "string" ||
    section === undefined ||
    typeof value.show !== "boolean" ||
    typeof value.output !== "boolean" ||
    typeof value.margin_mm !== "number" ||
    !Number.isFinite(value.margin_mm) ||
    value.margin_mm < 0 ||
    items === null ||
    tick === null
  ) {
    return null;
  }
  // An unknown operation has no section (criterion 26); a known one has one.
  const known = kind !== "unknown";
  if (known && section === null) return null;
  return {
    id,
    name: value.name,
    kind,
    section: known ? section : null,
    show: value.show,
    output: value.output,
    marginMm: value.margin_mm,
    items,
    tick,
  };
}

function asIdList(value: unknown): OperationId[] | null {
  if (!Array.isArray(value)) return null;
  const ids: OperationId[] = [];
  for (const raw of value) {
    const id = asId(raw);
    if (id === null) return null;
    ids.push(id);
  }
  return ids;
}

function asItems(value: unknown): CountItems | null {
  if (!isObject(value)) return null;
  const running = asIdList(value.running);
  const notAssigned = asIdList(value.not_assigned);
  const runsWhileHidden = asIdList(value.runs_while_hidden);
  const outputOff = asIdList(value.output_off);
  if (running === null || notAssigned === null || runsWhileHidden === null || outputOff === null) {
    return null;
  }
  return { running, notAssigned, runsWhileHidden, outputOff };
}

function asCloneMember(value: unknown): CloneMember | null {
  if (!isObject(value)) return null;
  const clone = asId(value.clone);
  const member = asId(value.member);
  const operations = asIdList(value.operations);
  if (
    clone === null ||
    member === null ||
    operations === null ||
    typeof value.clone_name !== "string" ||
    typeof value.name !== "string"
  ) {
    return null;
  }
  return { clone, cloneName: value.clone_name, member, name: value.name, operations };
}

function asViewMode(value: unknown): ViewMode | null {
  return value === "operations" || value === "drawn" ? value : null;
}

function asSummary(value: unknown): OperationsSummary | null {
  if (!isObject(value)) return null;
  const operations = asCount(value.operations);
  const running = asCount(value.running);
  const notAssigned = asCount(value.not_assigned);
  const runsWhileHidden = asCount(value.runs_while_hidden);
  const outputOff = asCount(value.output_off);
  if (
    operations === null ||
    running === null ||
    notAssigned === null ||
    runsWhileHidden === null ||
    outputOff === null ||
    runsWhileHidden > running
  ) {
    return null;
  }
  return { operations, running, notAssigned, runsWhileHidden, outputOff };
}

/**
 * The view in `value` (already parsed JSON), or `null`: `null` itself (a page that is not an
 * active laser page with its extension on), or any value that is not the view.
 */
export function decodeOperationsView(value: unknown): OperationsView | null {
  if (!isObject(value) || typeof value.cuts_last !== "boolean" || !Array.isArray(value.rows)) {
    return null;
  }
  const rows: OperationRow[] = [];
  for (const raw of value.rows) {
    const row = asRow(raw);
    if (row === null) return null;
    rows.push(row);
  }
  const summary = asSummary(value.summary);
  const items = asItems(value.items);
  const viewMode = asViewMode(value.view_mode);
  if (summary === null || items === null || viewMode === null || !Array.isArray(value.clone_members)) {
    return null;
  }
  const cloneMembers: CloneMember[] = [];
  for (const raw of value.clone_members) {
    const member = asCloneMember(raw);
    if (member === null) return null;
    cloneMembers.push(member);
  }
  return { cutsLast: value.cuts_last, rows, summary, items, cloneMembers, viewMode };
}

/** The view a session's JSON text holds; `null` for bad JSON or a page without operations. */
export function decodeOperationsText(text: string): OperationsView | null {
  try {
    return decodeOperationsView(JSON.parse(text));
  } catch {
    return null;
  }
}
