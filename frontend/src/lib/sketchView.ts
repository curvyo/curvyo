export type SketchQuantity = { value: number; dimension: "Length" | "Angle" | "Scalar" };
export type SketchConstraint = { id: string; kind: string; entities: string[]; reference: boolean; expression: string | null; value: SketchQuantity | null; conflicting: boolean; redundant: boolean; selected: boolean };
export type SketchParameter = { id: string; name: string; dimension: SketchQuantity["dimension"]; expression: string | null; value: SketchQuantity | null };
/** The mode label at the segment start; `away_*` names the quadrant it sits in (y down). */
/** `error` is a refusal of the last canvas pick, said in red in the label. */
export type SketchModeLabel = { text: string; x: number; y: number; away_x: number; away_y: number; hint: string | null; error?: string | null };
/** A refused entry: the message line and the field it marks (`null`: no single field). */
export type SketchEntryError = { field: number | null; message: string };
export type SketchEntryField = { label: string; name: string; suffix: string; value: string };
/** The on-canvas entry box: Rust names the fields, their live values and the object anchor. */
/** `edit` changes the dimension `constraint` at its text; `reference_on` is its current state. */
export type SketchEntry = { kind: "construction" | "dimension" | "edit"; action: string | null; constraint?: string; reference_on?: boolean; group: string; fields: SketchEntryField[]; x: number; y: number; away_x: number; away_y: number; pointer_x: number; pointer_y: number; reference: boolean; error?: SketchEntryError | null };
/** A Constraints rail tool: what a press does now, and its tooltip (AC24). */
export type SketchRailTool = { name: string; label: string; state: "apply" | "arm" | "unavailable"; tooltip: string; pressed: boolean };
/** A dimension text or transient label, placed by Rust in CSS pixels (AC31). */
export type SketchAnnotation = { preview?: boolean; id: string; x: number; y: number; text: string; name?: string; dimension: boolean; selected: boolean; hovered?: boolean; reference?: boolean; fx?: boolean; editing?: boolean; conflicting: boolean; redundant?: boolean; glyph?: string };
/** A constraint symbol badge, or a thinned dot, placed by Rust (AC32). */
export type SketchSymbol = { id: string; glyph: string; index: number | null; name: string; x: number; y: number; selected: boolean; hovered: boolean; conflicting: boolean; redundant: boolean; dot: boolean };
export type SketchSymbolMore = { count: number; x: number; y: number };
/** Pointer-rate canvas chips, kept out of the Properties signature like the annotations. */
export type SketchCanvas = { mode_label: SketchModeLabel | null; entry: SketchEntry | null };
export type SketchView = {
  draft_epoch?:number;
  can_resume?:boolean;
  dimension_position?: {x:number;y:number} | null;
  constraint_tool?: string | null; dimension_action?: string | null;
  page?: string; mode?: string; cursor?: "crosshair" | "grab" | "grabbing" | "default"; degenerate?: boolean;
  canvas?: SketchCanvas;
  workbench: "vector" | "sketch"; tool: string; active: string | null;
  sketches: { id: string; name: string; editable: boolean }[];
  entities: { id: string; kind: string; selected: boolean; construction: boolean; fully_constrained: boolean | null; origin: boolean }[];
  constraints: SketchConstraint[]; parameters: SketchParameter[];
  actions: { name: string; enabled: boolean; reason: string | null }[];
  status: string | { UnderConstrained: number } | null; stale: boolean; diagnosis?: string | null;
  error: string | null; pending_picks: number; origin_fixed?: boolean;
  edit_constraint: string | null; affected_constraints: string[];
  annotations: SketchAnnotation[];
  rail?: SketchRailTool[];
  symbols?: SketchSymbol[];
  symbols_more?: SketchSymbolMore[];
};
export const EMPTY_SKETCH_VIEW: SketchView = { workbench: "vector", tool: "select", active: null, sketches: [], entities: [], constraints: [], parameters: [], actions: [], status: null, stale: false, error: null, pending_picks: 0, edit_constraint: null, affected_constraints: [], annotations: [] };


export function quantityText(value: SketchQuantity | null) { return value ? `${Number(value.value.toPrecision(8))}${value.dimension === "Length" ? " mm" : value.dimension === "Angle" ? " rad" : ""}` : "—"; }
