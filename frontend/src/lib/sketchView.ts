export type SketchQuantity = { value: number; dimension: "Length" | "Angle" | "Scalar" };
export type SketchConstraint = { id: string; kind: string; entities: string[]; reference: boolean; expression: string | null; value: SketchQuantity | null; conflicting: boolean; redundant: boolean; selected: boolean };
export type SketchParameter = { id: string; name: string; dimension: SketchQuantity["dimension"]; expression: string | null; value: SketchQuantity | null };
export type SketchView = {
  draft_epoch?:number;
  can_resume?:boolean;
  pick_role?: string; arc_radius?: number | null;
  dimension_position?: {x:number;y:number} | null;
  constraint_tool?: string | null; dimension_action?: string | null;
  page?: string; mode?: string; cursor?: "crosshair" | "grab" | "grabbing" | "default"; degenerate?: boolean; modes?: {id:string;enabled:boolean;reason:string|null}[];
  workbench: "vector" | "sketch"; tool: string; active: string | null;
  sketches: { id: string; name: string; editable: boolean }[];
  entities: { id: string; kind: string; selected: boolean; construction: boolean; fully_constrained: boolean | null; origin: boolean }[];
  constraints: SketchConstraint[]; parameters: SketchParameter[];
  actions: { name: string; enabled: boolean; reason: string | null }[];
  status: string | { UnderConstrained: number } | null; stale: boolean; diagnosis?: string | null;
  error: string | null; pending_picks: number; origin_fixed?: boolean;
  edit_constraint: string | null; affected_constraints: string[];
  annotations: { preview?:boolean; id: string; x: number; y: number; text: string; dimension: boolean; selected: boolean; conflicting: boolean }[];
};
export const EMPTY_SKETCH_VIEW: SketchView = { workbench: "vector", tool: "select", active: null, sketches: [], entities: [], constraints: [], parameters: [], actions: [], status: null, stale: false, error: null, pending_picks: 0, edit_constraint: null, affected_constraints: [], annotations: [] };


export function quantityText(value: SketchQuantity | null) { return value ? `${Number(value.value.toPrecision(8))}${value.dimension === "Length" ? " mm" : value.dimension === "Angle" ? " rad" : ""}` : "—"; }
