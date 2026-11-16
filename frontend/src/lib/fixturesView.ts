/**
 * The one decoder of the Fixtures tab's JSON (`specs/0102-laser-fixtures` `adrs.md` section 7,
 * `specs/0113-fixture-sets-and-pins`): `curvyo-editor-wasm/src/session/fixtures/view_json.rs`
 * builds it, and every field is checked here at runtime. No cast: a value of the wrong shape, or a
 * missing field, decodes to `null`, and the tab shows nothing rather than a guess. A kind this
 * build does not know is `unknown` (criterion 32). The Rust contract test reads the same fixtures
 * (`curvyo-editor-wasm/tests/fixtures/fixtures_view/`).
 */

/** The six kinds a row can have, and `unknown` for a kind this build does not know. */
export type FixtureKind = "grid" | "holder" | "clamp" | "stop" | "screw" | "pin" | "unknown";

/** A slot's shape as the session names it. */
export type SlotShapeName = "rect" | "circle";

/** The grid's tape switches, one per edge (criterion 15). */
export interface TapesValues {
  bottom: boolean;
  left: boolean;
  top: boolean;
  right: boolean;
}

/** The grid's fields in mm and lines (criterion 15). */
export interface GridValues {
  spacingMm: number;
  majorEvery: number;
  tapes: TapesValues;
  originXMm: number;
  originYMm: number;
}

/** One slot's row (criteria 19, 20). */
export interface SlotRow {
  id: string;
  name: string;
  shape: SlotShapeName;
  cxMm: number;
  cyMm: number;
  wMm: number;
  hMm: number;
  /** Whether it lies past the board's edge ("Outside the board"). */
  outside: boolean;
}

/** One Jobs toggle of a pin or screw: the role job it switches and its Output (criterion 25). */
export interface RoleToggle {
  /** The role job's full id text: the Output switch takes it back as it is. */
  job: string;
  on: boolean;
}

/** The unfolded body of a pin or a screw (criteria 18 to 20, 25). */
export interface RoundValues {
  /** A pin's Diameter; `null` for a screw, which keeps its knobs. */
  diameterMm: number | null;
  columns: number;
  rows: number;
  columnSpacingMm: number;
  rowSpacingMm: number;
  /** The Centre mark's Size. */
  markMm: number;
  /** `null` hides the toggle: an old screw has no role jobs (criterion 39). */
  centreMark: RoleToggle | null;
  diameters: RoleToggle | null;
}

/** One fixture's row and its fields, in a set or among the job fixtures. */
export interface FixtureRow {
  /** The node id as its full id text: the calls take it back as it is. */
  id: string;
  name: string;
  kind: FixtureKind;
  /** Its own "On the bed" check. */
  active: boolean;
  /** Whether it lies on the bed: its own check and its set on this page (criterion 8). */
  onBed: boolean;
  visible: boolean;
  /** Its own Lock (criteria 31, 32); a set fixture's is kept in the profile. */
  locked: boolean;
  /** Its set is locked: the fixture is locked and its own Lock is not pressable (criterion 32). */
  lockedBySet: boolean;
  /** Line 2 of the row, worded by the session (UX notes 4.3, 5.2). */
  line2: string;
  /** The grid's fields; the other kinds have none: their handles size them. */
  grid: GridValues | null;
  /** The fields and Jobs toggles of a pin or a screw. */
  round: RoundValues | null;
  slots: SlotRow[];
  canAddSlot: boolean;
}

/** One machine set of the page (criteria 2 to 10). */
export interface SetRow {
  id: string;
  name: string;
  /** The "On this page" check. */
  onPage: boolean;
  /** Session state: the set is unfolded. */
  unfolded: boolean;
  /** The set's Lock (criterion 32): every fixture in it keeps its place. */
  locked: boolean;
  /** Whether its add bar shows: not while locked, read only or at the profile's limit. */
  canAdd: boolean;
  /** Whether the set's add bar offers Grid (a set holds one). */
  canAddGrid: boolean;
  line2: string;
  fixtures: FixtureRow[];
}

/** One page of the Apply list: another laser page with job fixtures (criterion 33). */
export interface ApplyCandidate {
  id: string;
  name: string;
  count: number;
}

/** The whole Fixtures tab of the active laser page. */
export interface FixturesView {
  sets: SetRow[];
  jobFixtures: FixtureRow[];
  /** How many sets lie on this page ("2 of 3 on this page"). */
  setsOnPage: number;
  canAddSet: boolean;
  /** The line of a page whose sets are read only here (criterion 46); `null` when they are not. */
  readOnly: string | null;
  /** The line of a page on Generic laser (criterion 3); `null` on any other. */
  profileHint: string | null;
  canAddJobFixture: boolean;
  /** The kind whose placement is armed; `null` without one. */
  armedKind: FixtureKind | null;
  applyCandidates: ApplyCandidate[];
  /** The page of the Apply list that is picked (its preview shows). */
  applyPicked: string | null;
  selected: string | null;
  selectedSlot: string | null;
  armed: string | null;
  damageNotices: string[];
  snappingFixtures: boolean;
  /** What a screen reader says of the selected fixture, with its sizes; `null` without one. */
  selectionAnnouncement: string | null;
}

type Json = Record<string, unknown>;

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function asCount(value: unknown): number | null {
  return Number.isSafeInteger(value) && (value as number) >= 0 ? (value as number) : null;
}

function asOptionalText(value: unknown): string | null | undefined {
  if (value === null) return null;
  return typeof value === "string" ? value : undefined;
}

function asOptionalNumber(value: unknown): number | null | undefined {
  if (value === null) return null;
  return isNumber(value) ? value : undefined;
}

/** The kind of a stored text; a text this build does not know is `unknown`, a non-text is `undefined`. */
export function asKind(value: unknown): FixtureKind | undefined {
  if (typeof value !== "string") return undefined;
  return value === "grid" ||
    value === "holder" ||
    value === "clamp" ||
    value === "stop" ||
    value === "screw" ||
    value === "pin"
    ? value
    : "unknown";
}

function asShape(value: unknown): SlotShapeName | null {
  return value === "rect" || value === "circle" ? value : null;
}

function asTapes(value: unknown): TapesValues | null {
  if (!isObject(value)) return null;
  const { bottom, left, top, right } = value;
  if (
    typeof bottom !== "boolean" ||
    typeof left !== "boolean" ||
    typeof top !== "boolean" ||
    typeof right !== "boolean"
  ) {
    return null;
  }
  return { bottom, left, top, right };
}

function asGrid(value: unknown): GridValues | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const majorEvery = asCount(value.major_every);
  const tapes = asTapes(value.tapes);
  if (
    majorEvery === null ||
    tapes === null ||
    !isNumber(value.spacing_mm) ||
    !isNumber(value.origin_x_mm) ||
    !isNumber(value.origin_y_mm)
  ) {
    return undefined;
  }
  return {
    spacingMm: value.spacing_mm,
    majorEvery,
    tapes,
    originXMm: value.origin_x_mm,
    originYMm: value.origin_y_mm,
  };
}

function asSlot(value: unknown): SlotRow | null {
  if (!isObject(value)) return null;
  const shape = asShape(value.shape);
  if (
    shape === null ||
    typeof value.id !== "string" ||
    typeof value.name !== "string" ||
    !isNumber(value.cx_mm) ||
    !isNumber(value.cy_mm) ||
    !isNumber(value.w_mm) ||
    !isNumber(value.h_mm) ||
    typeof value.outside !== "boolean"
  ) {
    return null;
  }
  return {
    id: value.id,
    name: value.name,
    shape,
    cxMm: value.cx_mm,
    cyMm: value.cy_mm,
    wMm: value.w_mm,
    hMm: value.h_mm,
    outside: value.outside,
  };
}

function asToggle(value: unknown): RoleToggle | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || typeof value.job !== "string" || typeof value.on !== "boolean") return undefined;
  return { job: value.job, on: value.on };
}

function asRound(value: unknown): RoundValues | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const diameter = asOptionalNumber(value.diameter_mm);
  const columns = asCount(value.columns);
  const rows = asCount(value.rows);
  const centreMark = asToggle(value.centre_mark);
  const diameters = asToggle(value.diameters);
  if (
    diameter === undefined ||
    columns === null ||
    rows === null ||
    centreMark === undefined ||
    diameters === undefined ||
    !isNumber(value.column_spacing_mm) ||
    !isNumber(value.row_spacing_mm) ||
    !isNumber(value.mark_mm)
  ) {
    return undefined;
  }
  return {
    diameterMm: diameter,
    columns,
    rows,
    columnSpacingMm: value.column_spacing_mm,
    rowSpacingMm: value.row_spacing_mm,
    markMm: value.mark_mm,
    centreMark,
    diameters,
  };
}

function asRow(value: unknown): FixtureRow | null {
  if (!isObject(value)) return null;
  const grid = asGrid(value.grid);
  const round = asRound(value.round);
  const kind = asKind(value.kind);
  if (
    kind === undefined ||
    typeof value.id !== "string" ||
    typeof value.name !== "string" ||
    typeof value.line2 !== "string" ||
    typeof value.active !== "boolean" ||
    typeof value.on_bed !== "boolean" ||
    typeof value.visible !== "boolean" ||
    typeof value.can_add_slot !== "boolean" ||
    typeof value.locked !== "boolean" ||
    typeof value.locked_by_set !== "boolean" ||
    grid === undefined ||
    round === undefined ||
    !Array.isArray(value.slots)
  ) {
    return null;
  }
  const slots: SlotRow[] = [];
  for (const raw of value.slots) {
    const slot = asSlot(raw);
    if (slot === null) return null;
    slots.push(slot);
  }
  return {
    id: value.id,
    name: value.name,
    kind,
    active: value.active,
    onBed: value.on_bed,
    visible: value.visible,
    locked: value.locked,
    lockedBySet: value.locked_by_set,
    line2: value.line2,
    grid,
    round,
    slots,
    canAddSlot: value.can_add_slot,
  };
}

function asRows(value: unknown): FixtureRow[] | null {
  if (!Array.isArray(value)) return null;
  const rows: FixtureRow[] = [];
  for (const raw of value) {
    const row = asRow(raw);
    if (row === null) return null;
    rows.push(row);
  }
  return rows;
}

function asSet(value: unknown): SetRow | null {
  if (!isObject(value)) return null;
  const fixtures = asRows(value.fixtures);
  if (
    fixtures === null ||
    typeof value.id !== "string" ||
    typeof value.name !== "string" ||
    typeof value.line2 !== "string" ||
    typeof value.on_page !== "boolean" ||
    typeof value.locked !== "boolean" ||
    typeof value.can_add !== "boolean" ||
    typeof value.unfolded !== "boolean" ||
    typeof value.can_add_grid !== "boolean"
  ) {
    return null;
  }
  return {
    id: value.id,
    name: value.name,
    onPage: value.on_page,
    locked: value.locked,
    canAdd: value.can_add,
    unfolded: value.unfolded,
    canAddGrid: value.can_add_grid,
    line2: value.line2,
    fixtures,
  };
}

function asCandidate(value: unknown): ApplyCandidate | null {
  if (!isObject(value)) return null;
  const count = asCount(value.count);
  if (count === null || typeof value.id !== "string" || typeof value.name !== "string") return null;
  return { id: value.id, name: value.name, count };
}

function asList<T>(value: unknown, one: (raw: unknown) => T | null): T[] | null {
  if (!Array.isArray(value)) return null;
  const out: T[] = [];
  for (const raw of value) {
    const item = one(raw);
    if (item === null) return null;
    out.push(item);
  }
  return out;
}

/**
 * The view in `value` (already parsed JSON), or `null`: `null` itself (a page that is not an
 * active laser page), or any value that is not the view.
 */
export function decodeFixturesView(value: unknown): FixturesView | null {
  if (!isObject(value)) return null;
  const sets = asList(value.sets, asSet);
  const jobFixtures = asRows(value.job_fixtures);
  const candidates = asList(value.apply_candidates, asCandidate);
  const setsOnPage = asCount(value.sets_on_page);
  const selected = asOptionalText(value.selected);
  const selectedSlot = asOptionalText(value.selected_slot);
  const armed = asOptionalText(value.armed);
  const armedText = asOptionalText(value.armed_kind);
  const applyPicked = asOptionalText(value.apply_picked);
  const announcement = asOptionalText(value.selection_announcement);
  const readOnly = asOptionalText(value.read_only);
  const profileHint = asOptionalText(value.profile_hint);
  if (
    sets === null ||
    jobFixtures === null ||
    candidates === null ||
    setsOnPage === null ||
    selected === undefined ||
    selectedSlot === undefined ||
    armed === undefined ||
    armedText === undefined ||
    applyPicked === undefined ||
    announcement === undefined ||
    readOnly === undefined ||
    profileHint === undefined ||
    typeof value.can_add_set !== "boolean" ||
    typeof value.can_add_job_fixture !== "boolean" ||
    typeof value.snapping_fixtures !== "boolean" ||
    !Array.isArray(value.damage_notices) ||
    !value.damage_notices.every((text) => typeof text === "string")
  ) {
    return null;
  }
  return {
    sets,
    jobFixtures,
    setsOnPage,
    canAddSet: value.can_add_set,
    readOnly,
    profileHint,
    canAddJobFixture: value.can_add_job_fixture,
    armedKind: armedText === null ? null : (asKind(armedText) ?? null),
    applyCandidates: candidates,
    applyPicked,
    selected,
    selectedSlot,
    armed,
    damageNotices: value.damage_notices as string[],
    snappingFixtures: value.snapping_fixtures,
    selectionAnnouncement: announcement,
  };
}

/** The view a JSON text holds; `null` for the text `null`, bad JSON or any other value. */
export function decodeFixturesText(text: string): FixturesView | null {
  try {
    return decodeFixturesView(JSON.parse(text));
  } catch {
    return null;
  }
}

/** The selected fixture's row, `null` without one (the tab's subject line names it, F16). */
export function selectedRow(view: FixturesView): FixtureRow | null {
  return view.selected === null ? null : (allRows(view).find((row) => row.id === view.selected) ?? null);
}

/** Every fixture row of the view, set fixtures first (sets in order), then the job fixtures. */
export function allRows(view: FixturesView): FixtureRow[] {
  return [...view.sets.flatMap((set) => set.fixtures), ...view.jobFixtures];
}

/**
 * The fixture whose Select bar shows (the Lock alone, `0113` criteria 31, 32) and the name of its
 * set; `null` without a selected fixture and on a page whose sets are read only (criterion 46).
 */
export function selectedLockBar(view: FixturesView | null): { row: FixtureRow; setName: string | null } | null {
  if (view === null || view.selected === null) return null;
  const job = view.jobFixtures.find((row) => row.id === view.selected);
  if (job) return { row: job, setName: null };
  if (view.readOnly !== null) return null;
  for (const set of view.sets) {
    const row = set.fixtures.find((candidate) => candidate.id === view.selected);
    if (row) return { row, setName: set.name };
  }
  return null;
}
