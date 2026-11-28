/** The part of a CNC page's view (`cnc` of `manufacturing_view_json`, `specs/0161`) the Fixtures
 * and Tools tabs read: the stock and the tools (ADR 0098; the rest is the shared output page's). */
export interface CncView {
  stock: { width: number; height: number; thickness: number; material: string };
  tools: CncTool[];
}

/** One end mill of the page's tool list. */
export interface CncTool {
  id: string;
  /** "Flat 3.175 mm". */
  name: string;
  /** "2 flutes · 800 mm/min · 12,000 rpm · 1 mm down". */
  line2: string;
  numbers: {
    diameter_mm: number;
    flutes: number;
    feed_mm_min: number;
    plunge_mm_min: number;
    rpm: number;
    step_down_mm: number;
  };
}
