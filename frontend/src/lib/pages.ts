/** Plain display data produced by the Rust page controller. */
export interface PageRow {
  id: string;
  name: string;
  size: string;
  active: boolean;
  frameOnly: boolean;
  xText: string;
  yText: string;
  x: number;
  y: number;
  screenX: number;
  screenY: number;
  screenWidth: number;
  screenHeight: number;
}
export function readPages(json: string): PageRow[] {
  return JSON.parse(json) as PageRow[];
}
