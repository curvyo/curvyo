import { EMPTY_SKETCH_VIEW, quantityText, type SketchView } from "./sketchView.ts";

/** Routes a single Rust view to semantic controls and independently moving annotations. */
export function createSketchPresentation() {
  let panel = EMPTY_SKETCH_VIEW;
  let annotations = EMPTY_SKETCH_VIEW.annotations;
  let signature = "";
  const panels = new Set<() => void>();
  const labels = new Set<() => void>();
  return {
    panel: () => panel,
    annotations: () => annotations,
    subscribePanel: (listener: () => void) => { panels.add(listener); return () => { panels.delete(listener); }; },
    subscribeAnnotations: (listener: () => void) => { labels.add(listener); return () => { labels.delete(listener); }; },
    publish(view: SketchView) {
      const { annotations: next, ...semantic } = view;
      const key = JSON.stringify({ ...semantic, constraints: semantic.constraints.map(c => ({ ...c, value: quantityText(c.value) })) });
      annotations = next;
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
