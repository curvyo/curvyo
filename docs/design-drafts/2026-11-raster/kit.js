// Shared builders of the 0123 drafts 04 to 06: Lucide paths, the kind glyphs, panel rows and a
// real dither routine at draft scale, so the dot patterns are what the modes give.
const L = {
  eye: '<path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/>',
  power: '<path d="M12 2v10"/><path d="M18.4 6.6a9 9 0 1 1-12.77.04"/>',
  trash: '<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>',
  up: '<path d="m18 15-6-6-6 6"/>',
  down: '<path d="m6 9 6 6 6-6"/>',
  hash: '<line x1="4" x2="20" y1="9" y2="9"/><line x1="4" x2="20" y1="15" y2="15"/><line x1="10" x2="8" y1="3" y2="21"/><line x1="16" x2="14" y1="3" y2="21"/>',
  arrows: '<path d="m16 3 4 4-4 4"/><path d="M20 7H4"/><path d="m8 21-4-4 4-4"/><path d="M4 17h16"/>',
  contrast: '<circle cx="12" cy="12" r="10"/><path d="M12 18a6 6 0 0 0 0-12v12z"/>',
  swatch: '<path d="M11 17a4 4 0 0 1-8 0V5a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2Z"/><path d="M16.7 13H19a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2H7"/><path d="M7 17h.01"/><path d="m11 8 2.3-2.3a2.4 2.4 0 0 1 3.404.004L18.6 7.6a2.4 2.4 0 0 1 .026 3.434L9.9 19.8"/>',
  chev: '<path d="m6 9 6 6 6-6"/>',
  warn: '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
  octx: '<path d="M2.586 16.726A2 2 0 0 1 2 15.312V8.688a2 2 0 0 1 .586-1.414l4.688-4.688A2 2 0 0 1 8.688 2h6.624a2 2 0 0 1 1.414.586l4.688 4.688A2 2 0 0 1 22 8.688v6.624a2 2 0 0 1-.586 1.414l-4.688 4.688a2 2 0 0 1-1.414.586H8.688a2 2 0 0 1-1.414-.586z"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>',
  link: '<path d="M9 17H7A5 5 0 0 1 7 7h2"/><path d="M15 7h2a5 5 0 1 1 0 10h-2"/><line x1="8" x2="16" y1="12" y2="12"/>',
};
const ic = (name, size = 16, color = 'currentColor') =>
  `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="${color}" stroke-width="${(1.5 * 24) / size}" stroke-linecap="round" stroke-linejoin="round">${L[name]}</svg>`;

/** The kind glyphs of `OperationGlyph`, plus the new Image dot ramp (ux-notes 2.1). */
function glyph(kind, size = 16, color) {
  const c = color ? ` style="color:${color}"` : '';
  const open = `<svg width="${size}" height="${size}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"${c}>`;
  const hatch = '<line x1="2" y1="6" x2="10" y2="14" stroke-width="1"/><line x1="2" y1="2" x2="14" y2="14" stroke-width="1"/><line x1="6" y1="2" x2="14" y2="10" stroke-width="1"/>';
  switch (kind) {
    case 'cut': return `<svg width="${size}" height="${size}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" style="color:${color || 'var(--cut)'}"><rect x="2.25" y="2.25" width="11.5" height="11.5"/></svg>`;
    case 'cut_out': return `<svg width="${size}" height="${size}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" style="color:${color || 'var(--cut)'}"><rect x="2.25" y="2.25" width="11.5" height="11.5" stroke-dasharray="4 2"/></svg>`;
    case 'score': return `${open}<line x1="2" y1="8" x2="14" y2="8" stroke-dasharray="0.01 3" stroke-linecap="round"/></svg>`;
    case 'engrave': return `${open}${hatch}</svg>`;
    case 'engrave_outline': return `${open}${hatch}<rect x="0.75" y="0.75" width="14.5" height="14.5"/></svg>`;
    case 'image': {
      // Columns of 4, 3, 2 and 1 dots: a dither ramp, dark at the left.
      const cols = [[0, 1, 2, 3], [0, 1, 3], [0, 2], [1]];
      const dots = cols.flatMap((rows, x) => rows.map((y) => `<rect x="${1 + x * 4}" y="${1 + y * 4}" width="2" height="2"/>`)).join('');
      return `<svg width="${size}" height="${size}" viewBox="0 0 16 16" fill="currentColor" stroke="none"${c}>${dots}</svg>`;
    }
  }
  return '';
}

const KIND_WORD = { cut: 'Cut', score: 'Score', engrave: 'Engrave', engrave_outline: 'Engrave + Outline', image: 'Image', cut_out: 'Cut-out' };

/** One operation row of the Operations tab (lines 1 to 3, `OperationRow`). */
function row({ kind, name, digit, count, time, line3, sel = false, unf = false, body = '' }) {
  return `<div class="row${sel ? ' sel' : ''}${unf ? ' unf' : ''}">
    <div class="l1"><span style="width:16px;display:flex">${glyph(kind)}</span><span class="nm">${name}</span>
      <i>${ic('eye')}</i><i>${ic('power')}</i><i class="tr">${ic('trash')}</i><span class="g"></span><i>${ic('up')}</i><i>${ic('down')}</i></div>
    <div class="l2"><span class="ring">${digit}</span><span>${KIND_WORD[kind]} · ${count}</span><span class="tm">${time}</span></div>
    <div class="l3">${line3}</div>
    ${body}
  </div>`;
}

const field = (lb, v, u, w, extra = '') => `<div class="f${extra}" style="width:${w}px"><span class="lb">${lb}</span><span class="v">${v}</span><span class="u">${u}</span></div>`;

/** The upper part of an unfolded Engrave or Image row (`0111` 6.2). */
const upper = `
  <div class="pick">${ic('swatch')}<span class="lb">Material</span><span class="v">Birch plywood 3 mm</span>${ic('chev', 14)}</div>
  <div class="fr">${field('Power', '35', '%', 114)}${field('Passes', '1', '×', 114)}</div>
  <div class="fr">${field('Speed', '6000', 'mm/min', 232)}</div>
  <div class="fr">${field('Interval', '0.10', 'mm', 124)}</div>`;

const MODES = [
  ['threshold', 'Threshold', 'Threshold'], ['ordered', 'Ordered', 'Ordered'], ['floyd', 'Floyd', 'Floyd-Steinberg'],
  ['jarvis', 'Jarvis', 'Jarvis'], ['atkinson', 'Atkinson', 'Atkinson'], ['stucki', 'Stucki', 'Stucki'], ['grey', 'Grey', 'Grey'],
];

/** The Dither choice strip: `PresetStrip` look, a fixed break after the third cell (ux-notes 2.5). */
function ditherStrip(on, { focus = null, hover = null, mark = false } = {}) {
  const cells = MODES.map(([id, short, full], i) =>
    `${i === 3 ? '<span class="br"></span>' : ''}<span class="c${id === on ? ' on' : ''}${id === focus ? ' focus' : ''}${id === hover ? ' hov' : ''}" title="${full}">${short}</span>`).join('');
  return `<div class="strip${mark ? ' new' : ''}" role="radiogroup" aria-label="Dither">${cells}</div>`;
}

/** The Image settings group (ux-notes 2.4): strip, Brightness and Contrast, Gamma and Invert, the mode field last. */
function imageSettings(mode, opts = {}) {
  let last = '';
  if (mode === 'threshold') last = `<div class="fr${opts.markLast ? ' new' : ''}">${field('Threshold', '50', '%', 124)}</div>`;
  if (mode === 'grey') last = `<div class="fr${opts.markLast ? ' new' : ''}">${field('Lowest power', '10', '%', 132)}</div>`;
  return `<div class="hair"></div>
    ${ditherStrip(mode, opts)}
    ${opts.damaged ? '<div class="dmg">Damaged</div>' : ''}
    <div class="fr">${field('Brightness', '0', '%', 120)}${field('Contrast', '0', '%', 108)}</div>
    <div class="fr">${field('Gamma', '1.00', '', 100)}<span class="tg">${ic('contrast')}</span></div>
    ${last}`;
}

/** The lower part: Image has no Crosshatch (ux-notes 2.4); Engrave keeps it (`0111` 6.3). */
function lower(kind) {
  const cross = kind === 'engrave' ? `<span class="tg">${ic('hash')}</span>` : '';
  const sa = kind === 'engrave' ? 164 : 164;
  return `<div class="hair"></div>
    <div class="fr">${cross}<span class="tg on">${ic('arrows')}</span>${field('Scan angle', '0.0', '°', sa)}</div>
    <div class="fr">${field('Overscanning', '2', '%', 128)}</div>`;
}

// ---- dither at draft scale -------------------------------------------------------------
/** A grey "photo": sky ramp, a sun, two hills and a striped field (0 black .. 255 white). */
function photo(w, h) {
  const g = new Float32Array(w * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    let v = 120 + (y / h) * 120;
    const sx = x - w * 0.68, sy = y - h * 0.24;
    if (sx * sx + sy * sy < (h * 0.11) ** 2) v = 252;
    const hill = h * (0.58 - 0.18 * Math.exp(-(((x - w * 0.42) / (w * 0.16)) ** 2)) - 0.05 * Math.sin(x / (w * 0.07)));
    if (y > hill) v = 70 + 50 * Math.sin(x / 9 + y / 13) * 0.4 + (y - hill) * 0.4;
    if (y > h * 0.8) v = 110 + 60 * ((Math.floor(x / 5) % 2));
    g[y * w + x] = Math.max(0, Math.min(255, v));
  }
  return g;
}
/** A linear black-to-white ramp, left to right. */
function ramp(w, h) {
  const g = new Float32Array(w * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) g[y * w + x] = (255 * x) / (w - 1);
  return g;
}
const BAYER = [0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14, 46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25, 15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21];
const KERNELS = {
  floyd: [[1, 0, 7], [-1, 1, 3], [0, 1, 5], [1, 1, 1]], div_floyd: 16,
  atkinson: [[1, 0, 1], [2, 0, 1], [-1, 1, 1], [0, 1, 1], [1, 1, 1], [0, 2, 1]], div_atkinson: 8,
  jarvis: [[1, 0, 7], [2, 0, 5], [-2, 1, 3], [-1, 1, 5], [0, 1, 7], [1, 1, 5], [2, 1, 3], [-2, 2, 1], [-1, 2, 3], [0, 2, 5], [1, 2, 3], [2, 2, 1]], div_jarvis: 48,
  stucki: [[1, 0, 8], [2, 0, 4], [-2, 1, 2], [-1, 1, 4], [0, 1, 8], [1, 1, 4], [2, 1, 2], [-2, 2, 1], [-1, 2, 2], [0, 2, 4], [1, 2, 2], [2, 2, 1]], div_stucki: 42,
};
/** Returns 1 for a burned dot, 0 for none; Grey returns the power 0..1. */
function dither(src, w, h, mode, inside = () => true) {
  const g = Float32Array.from(src), out = new Float32Array(w * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const i = y * w + x;
    if (!inside(x, y)) continue;
    const v = g[i];
    if (mode === 'threshold') out[i] = v < 128 ? 1 : 0;
    else if (mode === 'ordered') out[i] = v < (BAYER[(y % 8) * 8 + (x % 8)] + 0.5) * 4 ? 1 : 0;
    else if (mode === 'grey') out[i] = v >= 254 ? 0 : 0.2 + 0.8 * (1 - v / 255);
    else {
      const burn = v < 128 ? 1 : 0;
      out[i] = burn;
      const err = v - (burn ? 0 : 255);
      for (const [dx, dy, wgt] of KERNELS[mode]) {
        const nx = x + dx, ny = y + dy;
        if (nx >= 0 && nx < w && ny < h) g[ny * w + nx] += (err * wgt) / KERNELS[`div_${mode}`];
      }
    }
  }
  return out;
}
/** Paints dots into a canvas: one dot per `px` screen pixels, pale orange on a transparent ground. */
function paintDots(canvas, src, w, h, mode, px, inside) {
  canvas.width = w * px; canvas.height = h * px;
  canvas.style.width = `${w * px}px`; canvas.style.height = `${h * px}px`;
  const ctx = canvas.getContext('2d');
  const d = dither(src, w, h, mode, inside);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const a = d[y * w + x];
    if (!a) continue;
    ctx.globalAlpha = a;
    ctx.fillStyle = '#C46A1C';
    ctx.fillRect(x * px, y * px, px, px);
  }
  ctx.globalAlpha = 1;
}
/** Paints the grey source itself (Operations view off, or the dimmed wrong-kind look). */
function paintGrey(canvas, src, w, h, px, alpha = 1, inside = () => true) {
  canvas.width = w * px; canvas.height = h * px;
  canvas.style.width = `${w * px}px`; canvas.style.height = `${h * px}px`;
  const ctx = canvas.getContext('2d');
  ctx.globalAlpha = alpha;
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    if (!inside(x, y)) continue;
    const v = Math.round(src[y * w + x]);
    ctx.fillStyle = `rgb(${v},${v},${v})`;
    ctx.fillRect(x * px, y * px, px, px);
  }
}
