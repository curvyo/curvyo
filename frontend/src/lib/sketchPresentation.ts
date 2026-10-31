import { EMPTY_SKETCH_VIEW, quantityText, type SketchCanvas, type SketchSymbol, type SketchSymbolMore, type SketchView } from "./sketchView.ts";

const NO_CANVAS: SketchCanvas = { mode_label: null, entry: null };

/** Routes a single Rust view to semantic controls and independently moving annotations. */
export function createSketchPresentation() {
  let panel = EMPTY_SKETCH_VIEW;
  let annotations = EMPTY_SKETCH_VIEW.annotations;
  let canvas = NO_CANVAS;
  let symbols: { badges: SketchSymbol[]; more: SketchSymbolMore[] } = { badges: [], more: [] };
  let entryOpen = false;
  let signature = "";
  const panels = new Set<() => void>();
  const labels = new Set<() => void>();
  return {
    panel: () => panel,
    annotations: () => annotations,
    canvas: () => canvas,
    /** Constraint symbols move with pan and zoom, like the annotations. */
    symbols: () => symbols,
    /** Whether the entry box is open: host focus state, opened by Tab on the canvas. */
    entryOpen: () => entryOpen,
    setEntryOpen(open: boolean) {
      if (entryOpen === open) return;
      entryOpen = open;
      labels.forEach(listener => listener());
    },
    subscribePanel: (listener: () => void) => { panels.add(listener); return () => { panels.delete(listener); }; },
    subscribeAnnotations: (listener: () => void) => { labels.add(listener); return () => { labels.delete(listener); }; },
    publish(view: SketchView) {
      const { annotations: next, canvas: chips, symbols: badges, symbols_more: more, ...semantic } = view;
      const key = JSON.stringify({ ...semantic, constraints: semantic.constraints.map(c => ({ ...c, value: quantityText(c.value) })) });
      annotations = next;
      symbols = { badges: badges ?? [], more: more ?? [] };
      canvas = chips ?? NO_CANVAS;
      if (!canvas.entry) entryOpen = false;
      if (key !== signature) {
        signature = key;
        panel = view;
        panels.forEach(listener => listener());
      }
      labels.forEach(listener => listener());
    },
  };
}
export type SketchPresentation = ReturnType<typeof createSketchPresentation>;
