/**
 * The decoder of the Materials section's JSON (`specs/0028-material-test-library` `adrs.md`
 * section 6): `curvyo-editor-wasm/src/session/materials/view_json.rs` builds it and every field is
 * checked here at runtime. A value of the wrong shape, or with an enum text this build does not
 * know, decodes to `null` and the section shows nothing rather than a guess. The Rust fixtures
 * `materials_view/*.json` pin both sides.
 */
import {
  asMaterialNumbers,
  asSetKind,
  asSetKinds,
  isObject,
  type MaterialNumbers,
  type SetKind,
} from "./operationSettingsView.ts";

/** A readable material row. */
export interface LoadedMaterialRow {
  state: "loaded";
  id: string;
  name: string;
  thicknessMm: number;
  kinds: SetKind[];
  /** How many operations of the open project point to it. */
  uses: number;
}

/** A damaged record (criterion 29). */
export interface DamagedMaterialRow {
  state: "damaged";
  id: string;
  reason: string;
}

/** A record of a newer format. */
export interface NewerMaterialRow {
  state: "newer";
  id: string;
  name: string | null;
}

/** One row of the section. */
export type MaterialRow = LoadedMaterialRow | DamagedMaterialRow | NewerMaterialRow;

/** One set of the selected material. */
export interface MaterialSetFields {
  kind: SetKind;
  numbers: MaterialNumbers;
  /** "Tested 2026-11-10, 5 × 5 grid, cell 60 %, 3000 mm/min" or "Edited by hand 2026-11-11". */
  source: string;
}

/** The fields of the selected material (UX notes 10.3). */
export interface SelectedMaterial {
  id: string;
  name: string;
  thicknessMm: number;
  uses: number;
  sets: MaterialSetFields[];
}

/** The whole Materials section of the active laser page. */
export interface MaterialsView {
  profileName: string;
  rows: MaterialRow[];
  selected: SelectedMaterial | null;
  /** The default name of a new material, "Material 1". */
  nextName: string;
  loadNotices: string[];
}

function count(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0;
}

function asRow(value: unknown): MaterialRow | null {
  if (!isObject(value) || typeof value.id !== "string") return null;
  const { id } = value;
  switch (value.state) {
    case "loaded": {
      const kinds = asSetKinds(value.kinds);
      if (
        typeof value.name !== "string" ||
        typeof value.thickness_mm !== "number" ||
        !Number.isFinite(value.thickness_mm) ||
        kinds === null ||
        !count(value.uses)
      ) {
        return null;
      }
      return { state: "loaded", id, name: value.name, thicknessMm: value.thickness_mm, kinds, uses: value.uses };
    }
    case "damaged":
      return typeof value.reason === "string" ? { state: "damaged", id, reason: value.reason } : null;
    case "newer":
      if (value.name === null || typeof value.name === "string") {
        return { state: "newer", id, name: value.name };
      }
      return null;
    default:
      return null;
  }
}

function asSelected(value: unknown): SelectedMaterial | null | undefined {
  if (value === null) return null;
  if (!isObject(value)) return undefined;
  if (
    typeof value.id !== "string" ||
    typeof value.name !== "string" ||
    typeof value.thickness_mm !== "number" ||
    !Number.isFinite(value.thickness_mm) ||
    !count(value.uses) ||
    !Array.isArray(value.sets)
  ) {
    return undefined;
  }
  const sets: MaterialSetFields[] = [];
  for (const raw of value.sets) {
    if (!isObject(raw)) return undefined;
    const kind = asSetKind(raw.kind);
    const numbers = asMaterialNumbers(raw.numbers);
    if (kind === null || numbers === null || typeof raw.source !== "string") return undefined;
    sets.push({ kind, numbers, source: raw.source });
  }
  return { id: value.id, name: value.name, thicknessMm: value.thickness_mm, uses: value.uses, sets };
}

/** The section in `value` (already parsed JSON), or `null` (another page, or any value that is not the view). */
export function decodeMaterialsView(value: unknown): MaterialsView | null {
  if (
    !isObject(value) ||
    typeof value.profile_name !== "string" ||
    typeof value.next_name !== "string" ||
    !Array.isArray(value.rows) ||
    !Array.isArray(value.load_notices)
  ) {
    return null;
  }
  const rows: MaterialRow[] = [];
  for (const raw of value.rows) {
    const row = asRow(raw);
    if (row === null) return null;
    rows.push(row);
  }
  if (!value.load_notices.every((notice) => typeof notice === "string")) return null;
  const selected = asSelected(value.selected);
  if (selected === undefined) return null;
  return {
    profileName: value.profile_name,
    rows,
    selected,
    nextName: value.next_name,
    loadNotices: value.load_notices as string[],
  };
}

/** The section a session's JSON text holds; `null` for bad JSON or another page. */
export function decodeMaterialsText(text: string): MaterialsView | null {
  try {
    return decodeMaterialsView(JSON.parse(text));
  } catch {
    return null;
  }
}
