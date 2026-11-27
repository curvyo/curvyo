import type { OperationId } from "@/lib/operationsView";

/** The active plotter page as the session's `plotter_view_json` gives it (`specs/0159`). */
export interface PlotterView {
  id: string;
  page: string;
  profile: {
    id: string | null;
    name: string;
    outputFormat: "design_space_svg" | "hpgl";
    /** "Design Space SVG", "HPGL". */
    output: string;
    /** Design Space sets pressure and speed: the numbers are hidden (UX C13). */
    setsNumbers: boolean;
    maxSpeed: number;
    defaultPressure: number;
    defaultSpeed: number;
    matless: boolean;
    /** The cut area of the active mat, or the work area: "292.1 × 292.1 mm". */
    area: string;
  };
  profiles: { id: string; name: string; builtin: boolean }[];
  mat: {
    size: string;
    sizes: { key: string; label: string }[];
    grip: string;
    gripWord: string;
    gripColour: string;
    grips: { key: string; word: string; colour: string }[];
    grid: boolean;
  } | null;
  operations: PlotterRow[];
  kinds: { kind: "cut" | "draw"; label: string; description: string }[];
  checks: { severity: "block" | "warn"; text: string; items: OperationId[] }[];
  total: { time: string | null; runs: string; lengths: string };
  machineLine: string;
  exportAllowed: boolean;
  fileType: string;
  toolpath: boolean;
  notice: string | null;
}

/** One operation row. */
export interface PlotterRow {
  id: OperationId;
  name: string;
  kind: "cut" | "draw" | null;
  number: number;
  line2: string;
  line3: string;
  numbers: { pressure: number; speed: number; passes: number } | null;
  output: boolean;
  unfolds: boolean;
}

/** The plotter view of a manufacturing view, `null` on another page. */
export function plotterOf(view: { plotter?: PlotterView | null } | null): PlotterView | null {
  return view?.plotter ?? null;
}
