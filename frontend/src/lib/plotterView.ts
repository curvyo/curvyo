/** The part of a plotter page's view (`plotter` of `manufacturing_view_json`, `specs/0159`) the
 * Fixtures tab reads: the active mat (ADR 0098; the rest is the shared output page's). */
export interface PlotterView {
  mat: {
    size: string;
    sizes: { key: string; label: string }[];
    grip: string;
    gripWord: string;
    gripColour: string;
    grips: { key: string; word: string; colour: string }[];
    grid: boolean;
  } | null;
}
