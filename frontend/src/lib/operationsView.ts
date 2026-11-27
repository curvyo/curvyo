/**
 * The one decoder of the Operations tab's JSON (`specs/0101-laser-operations` `adrs.md`
 * decision 6): `curvyo-editor-wasm/src/session/operations/view_json.rs` builds it, and every
 * field is checked here at runtime. No cast: a value of the wrong shape or with an enum text
 * this build does not know decodes to `null`, and the tab shows nothing rather than a guess. The
 * Rust contract test reads the same fixtures (`curvyo-editor-wasm/tests/fixtures/operations_view/`).
 */

import { asNumbersLines, type NumbersLine } from "./numbersLine.ts";
import {
  decodePick,
  decodeSettings,
  type OperationSettings,
  type PickMaterial,
} from "./operationSettingsView.ts";

/** The node id as the session sends it: the JSON text is what the calls take back. */
export interface OperationId {
  peer: string;
  counter: number;
}

/** The six kinds a row can have. */
export const OPERATION_KINDS = ["cut", "score", "engrave", "engrave_outline", "cut_out", "image"] as const;

/** A row's kind; `unknown` for a kind this build does not know (criterion 26). */
export type OperationKind = (typeof OPERATION_KINDS)[number] | "unknown";

/** The two sections of the list with Cuts last on. */
export type OperationSection = "marking" | "cutting";

/** A row's tick over the selection: none, some (mixed) or all of it. */
export type TickState = "none" | "some" | "all";

/** A row that runs: its ring (the row's digit) and its time ("3 min 21 s"). */
export interface RowRun {
  digit: number;
  time: string;
}

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
  /** "Group 3" or "2 groups" while selected groups run in it in part; else `null`. */
  groupInPart: string | null;
  /** The row is the unfolded one (`specs/0111` criterion 17). */
  unfolded: boolean;
  /** The Settings block (`specs/0028`); `null` for a test grid row and an unknown operation. */
  settings: OperationSettings | null;
  /** The source line of a row a test grid made (`specs/0112` UX 6); `null` for any other row. */
  source: RowSource | null;
  /** The run ring and time of a row that runs (`specs/0029` UX 6.1); `null` for a row that does not. */
  run: RowRun | null;
  /** Line 3 (and line 4 of Engrave + Outline): the effective numbers; `null` for Unknown or damaged
   * settings. */
  numbers: NumbersLine[] | null;
  /** "Cut 1, run 3, 3 min 21 s" (the session's wording). */
  accessibleName: string;
  /** The Tabs row of the item part while selected items run in this unfolded Cut or Cut-out
   * (`specs/0139` criterion 5); `null` otherwise. */
  itemTabs: ItemTabs | null;
}

/** The Tabs row of the item part: Own pressed, Operation's, or mixed (`null`). */
export interface ItemTabs {
  own: boolean | null;
  /** "4 tabs", "Own tabs (3)", "Own tabs on 2 of 3 items". */
  readout: string;
}

/** The width chip of the current tab handle (`specs/0139` criterion 6). */
export interface TabChip {
  widthMm: number;
  ownWidth: boolean;
}

function asItemTabs(value: unknown): ItemTabs | null | undefined {
  if (value === null || value === undefined) return null;
  if (!isObject(value) || typeof value.readout !== "string") return undefined;
  if (value.own !== null && typeof value.own !== "boolean") return undefined;
  return { own: value.own as boolean | null, readout: value.readout };
}

function asTabChip(value: unknown): TabChip | null | undefined {
  if (value === null || value === undefined) return null;
  if (!isObject(value) || typeof value.width_mm !== "number" || typeof value.own_width !== "boolean") return undefined;
  return { widthMm: value.width_mm, ownWidth: value.own_width };
}

/** Which grid and cell a derived row runs: "Birch 3 mm · K7Q2M column 3, row 2". */
export interface RowSource {
  grid: OperationId;
  /** A cell row has no material pick (G10); the label row has the ordinary one. */
  cell: boolean;
  text: string;
}

/** One Select-bar kind button: the kind and what a press does (`specs/0111` criterion 2). */
export interface KindButton {
  kind: Exclude<OperationKind, "unknown">;
  /** "Assigns to Engrave 1" or "Creates Cut 1": the accessible description, never the name. */
  description: string;
}

/** The read-only entered group's crumb (`specs/0111` criterion 11): the page, then the groups. */
export interface EnteredGroup {
  page: string;
  /** The entered groups' names, outermost first. */
  groups: string[];
}

/** The whole Operations tab of the active laser page. */
export interface OperationsView {
  cutsLast: boolean;
  rows: OperationRow[];
  /** Whether the page's Operations view toggle is pressed (`specs/0111` criterion 15). */
  operationsView: boolean;
  /** The five kind buttons in bar order; `null` while the Select bar hides them. */
  selectBarKinds: KindButton[] | null;
  /** The read-only entered group; `null` at the root. */
  enteredGroup: EnteredGroup | null;
  /** The materials of this machine the Material pick lists. */
  pick: PickMaterial[];
  /** The Tabs toggle of the Select bar (`specs/0139` UX 4.2): `null` while hidden, else on. */
  tabsTool: boolean | null;
  /** The width chip of the current tab handle; `null` without one. */
  tabChip: TabChip | null;
}

type Json = Record<string, unknown>;

function isObject(value: unknown): value is Json {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function asId(value: unknown): OperationId | null {
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

function asSource(value: unknown): RowSource | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const grid = asId(value.grid);
  if (grid === null || typeof value.cell !== "boolean" || typeof value.text !== "string") return undefined;
  return { grid, cell: value.cell, text: value.text };
}

function asRowRun(value: unknown): RowRun | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  const digit = asCount(value.digit);
  if (digit === null || typeof value.time !== "string") return undefined;
  return { digit, time: value.time };
}

function asRow(value: unknown): OperationRow | null {
  if (!isObject(value)) return null;
  const id = asId(value.id);
  const kind = asKind(value.kind);
  const section = asSection(value.section);
  const items = asCount(value.items);
  const tick = asTick(value.tick);
  const settings = decodeSettings(value.settings);
  const source = asSource(value.source);
  const run = asRowRun(value.run);
  const numbers = value.numbers === null ? null : (asNumbersLines(value.numbers) ?? undefined);
  const groupInPart = value.group_in_part === null ? null : typeof value.group_in_part === "string" ? value.group_in_part : undefined;
  const itemTabs = asItemTabs(value.item_tabs);
  if (
    itemTabs === undefined ||
    groupInPart === undefined ||
    run === undefined ||
    numbers === undefined ||
    typeof value.accessible_name !== "string" ||
    typeof value.unfolded !== "boolean" ||
    settings === undefined ||
    source === undefined ||
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
    groupInPart,
    unfolded: value.unfolded,
    settings,
    source,
    run,
    numbers,
    accessibleName: value.accessible_name,
    itemTabs,
  };
}

const BUTTON_KINDS = ["cut", "score", "engrave", "engrave_outline", "cut_out", "image"] as const;

function asKindButtons(value: unknown): KindButton[] | null | undefined {
  if (value === null) return null;
  if (!Array.isArray(value)) return undefined;
  const buttons: KindButton[] = [];
  for (const raw of value) {
    if (!isObject(raw) || typeof raw.description !== "string") return undefined;
    const kind = (BUTTON_KINDS as readonly unknown[]).includes(raw.kind) ? (raw.kind as KindButton["kind"]) : null;
    if (kind === null) return undefined;
    buttons.push({ kind, description: raw.description });
  }
  return buttons;
}

function asEnteredGroup(value: unknown): EnteredGroup | null | undefined {
  if (value === null) return null;
  if (!isObject(value) || typeof value.page !== "string" || !Array.isArray(value.groups)) return undefined;
  if (value.groups.length === 0 || !value.groups.every((name) => typeof name === "string")) return undefined;
  return { page: value.page, groups: value.groups as string[] };
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
  const pick = decodePick(value.pick);
  const selectBarKinds = asKindButtons(value.select_bar_kinds);
  const enteredGroup = asEnteredGroup(value.entered_group);
  const tabChip = asTabChip(value.tab_chip);
  const tabsTool = value.tabs_tool === null || value.tabs_tool === undefined ? null : value.tabs_tool;
  if (
    tabChip === undefined ||
    (tabsTool !== null && typeof tabsTool !== "boolean") ||
    pick === null ||
    typeof value.operations_view !== "boolean" ||
    selectBarKinds === undefined ||
    enteredGroup === undefined
  ) {
    return null;
  }
  return {
    cutsLast: value.cuts_last,
    rows,
    operationsView: value.operations_view,
    selectBarKinds,
    enteredGroup,
    pick,
    tabsTool: tabsTool as boolean | null,
    tabChip,
  };
}

/** The view a session's JSON text holds; `null` for bad JSON or a page without operations. */
export function decodeOperationsText(text: string): OperationsView | null {
  try {
    return decodeOperationsView(JSON.parse(text));
  } catch {
    return null;
  }
}
