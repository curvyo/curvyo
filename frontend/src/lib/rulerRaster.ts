import { drawRulerBase, drawRulerForeground, drawRulerPointer, type RulerColours, type RulerData, type RulerStrip } from "./rulerDraw";

/** Static ruler pixels surround a separately painted moving cursor. */
export function createRulerRaster(strip: HTMLCanvasElement, colours: RulerColours) {
  const background = document.createElement("canvas");
  const foreground = document.createElement("canvas");
  return {
    rebuild(data: RulerData, geometry: RulerStrip) {
      const width = Math.round((geometry.horizontal ? geometry.length : geometry.thickness) * geometry.devicePixelRatio);
      const height = Math.round((geometry.horizontal ? geometry.thickness : geometry.length) * geometry.devicePixelRatio);
      for (const canvas of [strip, background, foreground]) { canvas.width = width; canvas.height = height; }
      const back = background.getContext("2d"); const front = foreground.getContext("2d");
      if (back && front) {
        drawRulerBase(back, data, geometry, colours);
        drawRulerForeground(front, data, geometry, colours);
      }
    },
    paint(geometry: RulerStrip) {
      const ctx = strip.getContext("2d");
      if (!ctx || !background.width || !background.height) return;
      ctx.drawImage(background, 0, 0);
      drawRulerPointer(ctx, geometry, colours);
      ctx.drawImage(foreground, 0, 0);
    },
  };
}
