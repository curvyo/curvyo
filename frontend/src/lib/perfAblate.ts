// Dev-only ablation of the editor page for the frame-pacing matrix (`tools/perf/measure.py
// --ablate`): one variant per app launch switches one part of the page off before the pan is
// measured. Inactive unless the frontend is built with `VITE_CURVYO_PERF` (the perf build) and the
// runner asked for a variant; a normal build always draws at the window's own pixel ratio.

const enabled = typeof import.meta.env?.VITE_CURVYO_PERF === "string" && import.meta.env.VITE_CURVYO_PERF.length > 0;

/** The variants the page itself applies. `measure.py` adds environment-only ones (WebKit flags)
 * that run the page as `baseline`. */
export const ABLATIONS = [
  "baseline",
  "no-rulers",
  "no-pageframes",
  "no-overlays",
  "canvas-only",
  "dpr1",
  "half-backing",
  "no-effects",
  "no-draw",
  "no-view",
  "idle",
] as const;
export type Ablation = (typeof ABLATIONS)[number];

let active: Ablation = "baseline";

/** The pixel ratio the canvas's backing store is sized with under `variant`: the window's own,
 * 1 (`dpr1`: the backing store is the CSS size) or half of it (`half-backing`: half the device
 * pixels in each direction). */
export function ablatedPixelRatio(variant: Ablation, raw: number): number {
  if (variant === "dpr1") return 1;
  if (variant === "half-backing") return raw / 2;
  return raw;
}

/** The ratio the editor sizes the canvas's backing store with (the raw one outside the perf build). */
export function perfPixelRatio(raw: number): number {
  return enabled ? ablatedPixelRatio(active, raw) : raw;
}

/** Whether the render loop skips `render()` (`no-draw`): the view still moves, nothing is drawn. */
export function perfSkipDraw(): boolean {
  return enabled && active === "no-draw";
}

/** The input the measured scenario sends under `variant`: a middle-button pan, pointer moves with
 * no button (the view stays, the hover and the ruler pointer marks still follow) or nothing. */
export function ablationInput(variant: Ablation): "pan" | "hover" | "none" {
  return variant === "no-view" ? "hover" : variant === "idle" ? "none" : "pan";
}

/** The CSS properties `no-effects` resets on the canvas and every ancestor, with the reset value. */
const EFFECTS: [string, string][] = [
  ["opacity", "1"], ["transform", "none"], ["will-change", "auto"], ["box-shadow", "none"],
  ["filter", "none"], ["mix-blend-mode", "normal"], ["backdrop-filter", "none"],
  ["-webkit-backdrop-filter", "none"], ["clip-path", "none"], ["mask", "none"], ["isolation", "auto"],
  ["transition-duration", "0s"], ["animation-name", "none"], ["contain", "none"], ["perspective", "none"],
];

function injectStyle(css: string) {
  const style = document.createElement("style");
  style.dataset.perfAblation = "";
  style.textContent = css;
  document.head.appendChild(style);
}

/** Elements over the canvas that are positioned out of flow and are not its ancestors. */
function overlaysOf(canvas: HTMLCanvasElement): HTMLElement[] {
  const box = canvas.getBoundingClientRect();
  const found: HTMLElement[] = [];
  for (const element of document.body.querySelectorAll<HTMLElement>("*")) {
    if (element.contains(canvas) || found.some((outer) => outer.contains(element))) continue;
    const position = getComputedStyle(element).position;
    if (position !== "absolute" && position !== "fixed") continue;
    const r = element.getBoundingClientRect();
    if (r.right > box.left && r.left < box.right && r.bottom > box.top && r.top < box.bottom) found.push(element);
  }
  return found;
}

/** Applies `variant` to the page (styles, the pixel ratio, the draw switch) and says what it
 * matched, so a variant that took no effect shows in the runner's table. `refit` resizes the
 * canvas's backing store to the new ratio. */
export function applyAblation(variant: Ablation, canvas: HTMLCanvasElement, refit: () => void): string {
  active = variant;
  switch (variant) {
    case "no-rulers": {
      // Not `display: none` on the grid cells: the viewport would move into the corner's cell.
      // The strips' canvases leave the layout, the cells stay (and paint nothing).
      const cells = document.querySelectorAll("[data-rulers]").length;
      injectStyle("[data-rulers] { visibility: hidden !important; } [data-rulers] > canvas { display: none !important; }");
      return `${cells} ruler cells hidden`;
    }
    case "no-pageframes": {
      const frames = document.querySelectorAll("[data-page-frames]").length;
      injectStyle("[data-page-frames] { display: none !important; }");
      return `${frames} page-frame layers hidden`;
    }
    case "no-overlays": {
      const overlays = overlaysOf(canvas);
      for (const element of overlays) element.style.setProperty("display", "none", "important");
      return `${overlays.length} positioned overlays hidden`;
    }
    case "canvas-only": {
      // Hidden, not removed: the canvas keeps its size and its backing store.
      canvas.dataset.perfCanvas = "";
      injectStyle("body * { visibility: hidden !important; } canvas[data-perf-canvas] { visibility: visible !important; }");
      return `${document.body.querySelectorAll("*").length - 1} elements hidden`;
    }
    case "dpr1":
    case "half-backing":
      refit();
      return `backing ratio ${perfPixelRatio(window.devicePixelRatio)}`;
    case "no-effects": {
      const found: string[] = [];
      for (let element: HTMLElement | null = canvas; element; element = element.parentElement) {
        const style = getComputedStyle(element);
        for (const [property, reset] of EFFECTS) {
          const value = style.getPropertyValue(property).trim();
          if (value && value !== reset) {
            found.push(`${element.tagName.toLowerCase()}.${property}=${value}`);
          }
          element.style.setProperty(property, reset, "important");
        }
      }
      return found.length > 0 ? `reset ${found.join("; ")}` : "none found on the canvas or its ancestors";
    }
    default:
      return variant;
  }
}
