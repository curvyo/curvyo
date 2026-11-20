// Shared builders of the 0135 drafts: step glyphs, the Adjustments group's parts and a draft-scale
// version of the presets, so the dots in the drafts are what the steps and the dither give.
// Needs ../2026-11-raster/kit.js (ic, field, glyph, dither, photo, ramp, paintDots, paintGrey).
Object.assign(L, {
  cols2: '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M12 3v18"/>',
  copy: '<rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>',
  paste: '<path d="M11 14h10"/><path d="M16 4h2a2 2 0 0 1 2 2v1.344"/><path d="m17 18 4-4-4-4"/><path d="M8 4H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 1.793-1.113"/><rect x="8" y="2" width="8" height="4" rx="1"/>',
  restore: '<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/>',
  stretch: '<path d="m18 8 4 4-4 4"/><path d="M2 12h20"/><path d="m6 8-4 4 4 4"/>',
  straight: '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M7 17 17 7"/>',
  x: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
  max: '<polyline points="15 3 21 3 21 9"/><polyline points="9 21 3 21 3 15"/><line x1="21" x2="14" y1="3" y2="10"/><line x1="3" x2="10" y1="21" y2="14"/>',
  min: '<polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" x2="21" y1="10" y2="3"/><line x1="3" x2="10" y1="21" y2="14"/>',
  crop: '<path d="M6 2v14a2 2 0 0 0 2 2h14"/><path d="M18 22V8a2 2 0 0 0-2-2H2"/>',
  check: '<path d="M20 6 9 17l-5-5"/>',
  link2: '<path d="M9 17H7A5 5 0 0 1 7 7h2"/><path d="M15 7h2a5 5 0 1 1 0 10h-2"/><line x1="8" x2="16" y1="12" y2="12"/>',
  unlink: '<path d="m18.84 12.25 1.72-1.71h-.02a5.004 5.004 0 0 0-.12-7.07 5.006 5.006 0 0 0-6.95 0l-1.72 1.71"/><path d="m5.17 11.75-1.71 1.71a5.004 5.004 0 0 0 .12 7.07 5.006 5.006 0 0 0 6.95 0l1.71-1.71"/><line x1="8" x2="8" y1="2" y2="5"/><line x1="2" x2="5" y1="8" y2="8"/><line x1="16" x2="16" y1="19" y2="22"/><line x1="19" x2="22" y1="16" y2="16"/>',
  download: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>',
});

/** The step glyphs, 16 x 16, ink, 1.5 px (ux-notes 4.2). AI steps carry the four-point spark. */
const SPARK = '<path d="M13 1.2v3.6M11.2 3h3.6" stroke-width="1.25"/>';
const STEP_GLYPH = {
  crop: '<path d="M4.5 1.5v9a1 1 0 0 0 1 1h9"/><path d="M11.5 14.5v-9a1 1 0 0 0-1-1h-9"/>',
  resize: '<rect x="1.75" y="1.75" width="12.5" height="12.5" stroke-dasharray="1.6 1.6" stroke-width="1"/><rect x="2.5" y="7.5" width="6" height="6"/><path d="M13 3L9.5 6.5M9.5 3.75V6.5h2.75"/>',
  upscale: '<rect x="1.75" y="4.25" width="10" height="10"/><rect x="2.75" y="9.5" width="3.75" height="3.75" stroke-dasharray="1.2 1.2" stroke-width="1"/><path d="M5.5 10.5L10 6M7 6h3v3"/>' + SPARK,
  ai_denoise: '<circle cx="3.5" cy="8" r=".6" fill="currentColor"/><circle cx="6" cy="5" r=".6" fill="currentColor"/><circle cx="5.5" cy="11.5" r=".6" fill="currentColor"/><path d="M8.5 13.5c1.5-3 4-3 5.5-1M8.5 9.5c1.5-3 4-3 5.5-1"/>' + SPARK,
  channel_mix: '<circle cx="6" cy="6" r="3.75"/><circle cx="10" cy="6" r="3.75"/><circle cx="8" cy="10" r="3.75"/>',
  denoise: '<circle cx="3.5" cy="4" r=".7" fill="currentColor"/><circle cx="6" cy="7.5" r=".7" fill="currentColor"/><circle cx="3.5" cy="11" r=".7" fill="currentColor"/><path d="M8.5 2v12"/><path d="M10.5 6h3.5M10.5 10h3.5"/>',
  levels: '<rect x="2" y="8" width="2" height="5" fill="currentColor" stroke="none"/><rect x="5" y="4" width="2" height="9" fill="currentColor" stroke="none"/><rect x="8" y="6" width="2" height="7" fill="currentColor" stroke="none"/><rect x="11" y="10" width="2" height="3" fill="currentColor" stroke="none"/><path d="M1.5 14.75h13"/>',
  curves: '<path d="M2 1.5v12.5h12.5"/><path d="M3.5 12.5C8 12.5 7.5 3 13.5 3"/>',
  local_contrast: '<rect x="2" y="2" width="5" height="5"/><rect x="9" y="2" width="5" height="5"/><rect x="2" y="9" width="5" height="5"/><rect x="9" y="9" width="5" height="5"/><path d="M2 7l5-5V7zM9 14l5-5v5zM9 7V2h5z" fill="currentColor" stroke="none"/><path d="M2 14V9h5z" fill="currentColor" stroke="none"/>',
  contours: '<path d="M2 12.5c2-6 4.5-6 6-3s4 3 6-4"/><path d="M2 8c1.5-3.5 3-4 4.5-3"/>',
  sharpen: '<path d="M8 1.5v13"/><path d="M2 5l3 3-3 3"/><path d="M14 5l-3 3 3 3"/>',
  remove_background: '<rect x="1.75" y="1.75" width="12.5" height="12.5" stroke-dasharray="2 1.6" stroke-width="1"/><circle cx="8" cy="6.5" r="2.5"/><path d="M3.5 14.25c.5-3.5 8.5-3.5 9 0"/>' + SPARK,
  unknown: '<rect x="2" y="2" width="12" height="12" rx="1" stroke-dasharray="2 2"/><path d="M6.5 6.2a1.6 1.6 0 1 1 2.2 1.5c-.5.2-.7.6-.7 1.1M8 11.2h.01"/>',
};
const sg = (kind, size = 16) => `<svg width="${size}" height="${size}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">${STEP_GLYPH[kind]}</svg>`;
const STEP_NAME = {
  crop: 'Crop', resize: 'Resize', upscale: 'Upscale', ai_denoise: 'AI denoise', channel_mix: 'Channel mix',
  denoise: 'Denoise', levels: 'Levels', curves: 'Curves', local_contrast: 'Local contrast', contours: 'Contours',
  sharpen: 'Sharpen', remove_background: 'Remove background', unknown: 'Unknown step',
};

/** The group header: name, then the symbol buttons (ux-notes 3.1). */
function groupHeader({ ba = false, copy = true, restore = true, restoreOff = false } = {}) {
  return `<div class="ghdr"><span class="t">Adjustments</span>
    <span class="ib${ba ? ' on' : ''}" title="Before and after">${ic('cols2')}</span>
    ${copy ? `<span class="ib" title="Copy adjustments">${ic('copy')}</span><span class="ib" title="Paste adjustments">${ic('paste')}</span>` : ''}
    ${restore ? `<span class="ib${restoreOff ? ' dis' : ''}" title="Restore original">${ic('restore')}</span>` : ''}</div>`;
}

const PRESETS = [
  ['photo', 'Photo', 'Photo (detailed)', 'Jarvis'], ['graphics', 'Graphics', 'Graphics (flat areas)', 'Ordered'],
  ['bw', 'Black and white', 'Black and white (hard edges)', 'Threshold 50 %'], ['lineart', 'Line art', 'Line art (outlines and handwriting)', 'Threshold 50 %'],
  ['dots', 'Dots', 'Dot raster (round dots)', 'Halftone'], ['newspaper', 'Newspaper', 'Newspaper (retro screen)', 'Lines'],
  ['expert', 'Expert', 'Expert', null],
];
/** The Preset choice strip: two fixed rows, a break after Black and white (ux-notes 3.2). */
function presetStrip(on, { focus = null, hover = null, mark = false } = {}) {
  const cells = PRESETS.map(([id, short, full], i) =>
    `${i === 3 ? '<span class="br"></span>' : ''}<span class="c${id === on ? ' on' : ''}${id === focus ? ' focus' : ''}${id === hover ? ' hov' : ''}" data-p="${id}" title="${full}">${short}</span>`).join('');
  return `<div class="strip${mark ? ' new' : ''}" role="radiogroup" aria-label="Preset">${cells}</div>`;
}
const presetTip = (id) => { const p = PRESETS.find((q) => q[0] === id); return p[3] ? `${p[2]} · Dither: ${p[3]}` : 'Expert · Dither unchanged'; };
/** The dither each preset sets on the picture (criterion 27; Expert leaves it). */
const PRESET_DITHER = { photo: 'jarvis', graphics: 'ordered', bw: 'threshold', lineart: 'threshold', dots: 'halftone', newspaper: 'lines' };

/** The nine Dither modes: [id, visible word, full name]; a tooltip only where the word differs. */
const MODES9 = [['threshold', 'Threshold', 'Threshold'], ['ordered', 'Ordered', 'Ordered'], ['floyd', 'Floyd', 'Floyd-Steinberg'],
  ['jarvis', 'Jarvis', 'Jarvis'], ['atkinson', 'Atkinson', 'Atkinson'], ['stucki', 'Stucki', 'Stucki'], ['grey', 'Grey', 'Grey'],
  ['halftone', 'Halftone', 'Halftone dots'], ['lines', 'Lines', 'Line screen']];
/** The nine-mode Dither strip: three fixed rows, breaks after Floyd and Grey (ux-notes 3.10, 10). */
function dither9(on, { focus = null, mark = false, label = null } = {}) {
  const cells = MODES9.map(([id, w, full], i) => `${i === 3 || i === 7 ? '<span class="br"></span>' : ''}<span class="c${id === on ? ' on' : ''}${id === focus ? ' focus' : ''}"${w !== full ? ` title="${full}"` : ''}>${w}</span>`).join('');
  return `<div class="strip${mark ? ' new' : ''}" role="radiogroup" aria-label="${label ?? 'Dither'}">${cells}</div>`;
}
/** The mode's own fields, one row, or nothing for the five error-diffusion and Ordered modes. */
function modeFields(mode, { mark = false } = {}) {
  const r = (inner) => `<div class="fr${mark ? ' new' : ''}">${inner}</div>`;
  if (mode === 'threshold') return r(field('Threshold', '50', '%', 124));
  if (mode === 'grey') return r(field('Lowest power', '10', '%', 132));
  if (mode === 'halftone') return r(field('Cell size', '0.80', 'mm', 124) + field('Angle', '45', '°', 104));
  if (mode === 'lines') return r(field('Line pitch', '1.20', 'mm', 124) + field('Angle', '45', '°', 104));
  return '';
}

/** The Strength row: slider, then the percent field, the ends under the slider (ux-notes 3.3). */
function strength(v, { focus = false, mark = false } = {}) {
  return `<div class="${mark ? 'new' : ''}"><div class="str"><div class="sl${focus ? ' focus' : ''}" role="slider" aria-label="Strength"><span class="tr"></span><span class="fi" style="width:${v}%"></span><span class="th" style="left:${v}%"></span></div>
    ${field('Strength', String(v), '%', 104)}</div>
    <div class="ends" style="width:144px"><span>Gentle</span><span>Strong</span></div></div>`;
}

/** One step row: glyph, name, power, trash, up, down; line 2 the numbers; optional body. */
function step({ kind, nums = '', off = false, body = '', ro = false, armed = false, up = true, down = true, cls = '', name }) {
  const nm = name ?? STEP_NAME[kind];
  const ctrls = ro ? '' : `<i title="${off ? 'Turn on' : 'Turn off'} ${nm}">${ic(off ? 'poweroff' : 'power')}</i>
      <i class="tr${armed ? ' armed' : ''}" title="${armed ? `Press again to remove ${nm}` : `Remove ${nm}`}">${ic('trash')}</i><span class="g"></span>
      ${up ? `<i title="Move ${nm} up">${ic('up')}</i>` : '<i></i>'}${down ? `<i title="Move ${nm} down">${ic('down')}</i>` : '<i></i>'}`;
  return `<div class="st${off ? ' off' : ''}${ro ? ' ro' : ''}${armed ? ' arm' : ''}${cls ? ` ${cls}` : ''}">
    <div class="a1"><span class="gl">${sg(kind)}</span><span class="nm">${nm}</span>${ctrls}</div>
    ${body ? `<div class="bd">${body}</div>` : `<div class="a2">${nums}</div>`}</div>`;
}
L.poweroff = '<path d="M18.36 6.64A9 9 0 0 1 20.77 15"/><path d="M6.16 6.16a9 9 0 1 0 12.68 12.68"/><path d="M12 2v4"/><path d="m2 2 20 20"/>';

/** The add toolbar: tone and detail in row 1, frame and AI in row 2 (ux-notes 3.5). */
function addBar({ preset = false, slice = 3 } = {}) {
  const tone = ['channel_mix', 'denoise', 'levels', 'curves', 'local_contrast', 'contours', 'sharpen']
    .filter((k) => slice >= 2 || !['channel_mix', 'curves'].includes(k));
  const frame = slice >= 2 ? ['crop', 'resize'] : [];
  const ai = slice >= 3 ? ['upscale', 'ai_denoise', 'remove_background'] : [];
  const b = (k) => `<span class="ib" title="Add ${STEP_NAME[k]}">${sg(k)}</span>`;
  const r1 = preset ? '' : `<div class="addb" role="toolbar" aria-label="Add adjustment">${tone.map(b).join('')}</div>`;
  const r2 = frame.length + ai.length ? `<div class="addb">${frame.map(b).join('')}${frame.length && ai.length ? '<span class="sep"></span>' : ''}${ai.map(b).join('')}</div>` : '';
  return r1 + r2;
}

// ---- graphs ----------------------------------------------------------------------------
function histogramOf(src) {
  const h = new Array(256).fill(0);
  for (const v of src) h[Math.max(0, Math.min(255, Math.round(v)))]++;
  return h;
}
/** Levels graph: hairline bars in muted ink, the black, gamma and white handles under it. */
function levelsGraph(src, black, white, gamma, w = 228, hgt = 64) {
  const h = histogramOf(src), max = Math.max(...h.slice(1, 255));
  let bars = '';
  for (let i = 0; i < 256; i++) {
    const x = 4 + (i / 255) * (w - 8), bh = Math.min(hgt - 14, ((hgt - 14) * h[i]) / max);
    if (bh > 0.2) bars += `<line x1="${x.toFixed(1)}" x2="${x.toFixed(1)}" y1="${hgt - 12}" y2="${(hgt - 12 - bh).toFixed(1)}" stroke="#5A5A5F" stroke-width="0.8"/>`;
  }
  const px = (p) => 4 + (p / 100) * (w - 8);
  const mid = black + (white - black) * Math.pow(0.5, 1 / gamma);
  const tri = (x, fill) => `<path d="M${x} ${hgt - 11}l5 8h-10z" fill="${fill}" stroke="#2F6FEE" stroke-width="1.25"/>`;
  return `<div class="graph" style="width:${w}px;height:${hgt}px"><svg width="${w}" height="${hgt}">${bars}
    <line x1="4" x2="${w - 4}" y1="${hgt - 12}" y2="${hgt - 12}" stroke="#3A3A3F" stroke-opacity=".25"/>
    ${tri(px(black), '#2F6FEE')}${tri(px(mid), '#fff')}${tri(px(white), '#fff')}</svg></div>`;
}
/** Curves graph: a square, quarter grid, the identity in muted ink, the curve, its points. */
function curvesGraph(points, sel, s = 120) {
  const P = (x, y) => [4 + (x / 100) * (s - 8), s - 4 - (y / 100) * (s - 8)];
  let grid = '';
  for (let i = 1; i < 4; i++) { const a = 4 + (i * (s - 8)) / 4; grid += `<line x1="${a}" x2="${a}" y1="4" y2="${s - 4}"/><line y1="${a}" y2="${a}" x1="4" x2="${s - 4}"/>`; }
  // monotone cubic (Fritsch-Carlson) through the points
  const xs = points.map((p) => p[0]), ys = points.map((p) => p[1]), n = points.length;
  const d = [], m = new Array(n).fill(0);
  for (let i = 0; i < n - 1; i++) d.push((ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]));
  m[0] = d[0]; m[n - 1] = d[n - 2];
  for (let i = 1; i < n - 1; i++) m[i] = d[i - 1] * d[i] <= 0 ? 0 : (d[i - 1] + d[i]) / 2;
  let path = '';
  for (let x = 0; x <= 100; x += 1) {
    let k = 0; while (k < n - 2 && x > xs[k + 1]) k++;
    const hh = xs[k + 1] - xs[k], t = (x - xs[k]) / hh, t2 = t * t, t3 = t2 * t;
    const y = (2 * t3 - 3 * t2 + 1) * ys[k] + (t3 - 2 * t2 + t) * hh * m[k] + (-2 * t3 + 3 * t2) * ys[k + 1] + (t3 - t2) * hh * m[k + 1];
    const [a, b] = P(x, Math.max(0, Math.min(100, y)));
    path += `${x ? 'L' : 'M'}${a.toFixed(1)} ${b.toFixed(1)}`;
  }
  const dots = points.map(([x, y], i) => { const [a, b] = P(x, y); return `<rect x="${a - 4}" y="${b - 4}" width="8" height="8" fill="${i === sel ? '#2F6FEE' : '#fff'}" stroke="#2F6FEE" stroke-width="1.5"/>`; }).join('');
  return `<div class="graph" style="width:${s}px;height:${s}px"><svg width="${s}" height="${s}"><g stroke="#3A3A3F" stroke-opacity=".18">${grid}</g>
    <line x1="4" y1="${s - 4}" x2="${s - 4}" y2="4" stroke="#5A5A5F" stroke-dasharray="2 3"/><path d="${path}" fill="none" stroke="#3A3A3F" stroke-width="1.5"/>${dots}</svg></div>`;
}

// ---- pictures and the draft-scale presets ------------------------------------------------
/** The customer's kind of test photo: a washed-out landscape (greys 95 to 185, mild noise). */
function flatPhoto(w, h) {
  const g = photo(w, h);
  let seed = 7; const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
  for (let i = 0; i < g.length; i++) g[i] = 95 + (g[i] / 255) * 90 + (rnd() - 0.5) * 10;
  return g;
}
function blur(src, w, h, r) {
  const tmp = new Float32Array(w * h), out = new Float32Array(w * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) { let s = 0, c = 0; for (let k = -r; k <= r; k++) { const xx = x + k; if (xx >= 0 && xx < w) { s += src[y * w + xx]; c++; } } tmp[y * w + x] = s / c; }
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) { let s = 0, c = 0; for (let k = -r; k <= r; k++) { const yy = y + k; if (yy >= 0 && yy < h) { s += tmp[yy * w + x]; c++; } } out[y * w + x] = s / c; }
  return out;
}
function percentile(src, p) { const h = histogramOf(src); let c = 0; const n = src.length; for (let i = 0; i < 256; i++) { c += h[i]; if (c >= (n * p) / 100) return i; } return 255; }
/** Photo preset at Strength s (0..100), draft scale: denoise, levels with median target, local contrast, sharpen. */
function photoPreset(src, w, h, s) {
  const lerp = (a, b) => a + ((b - a) * s) / 100;
  let g = Float32Array.from(src);
  const dn = blur(g, w, h, 1); const ds = lerp(0.1, 0.4);
  for (let i = 0; i < g.length; i++) g[i] = g[i] * (1 - ds) + dn[i] * ds;
  const clip = lerp(0.2, 2), lo = percentile(g, clip), hi = percentile(g, 100 - clip);
  for (let i = 0; i < g.length; i++) g[i] = Math.max(0, Math.min(255, ((g[i] - lo) / Math.max(1, hi - lo)) * 255));
  const med = percentile(g, 50) / 255, target = lerp(50, 42) / 100, gam = Math.log(target) / Math.log(Math.max(0.01, med));
  for (let i = 0; i < g.length; i++) g[i] = 255 * Math.pow(g[i] / 255, gam);
  const bl = blur(g, w, h, 6), lc = lerp(0.1, 0.9);
  for (let i = 0; i < g.length; i++) g[i] = Math.max(0, Math.min(255, g[i] + (g[i] - bl[i]) * lc));
  const b1 = blur(g, w, h, 1), sa = lerp(0.4, 2);
  for (let i = 0; i < g.length; i++) g[i] = Math.max(0, Math.min(255, g[i] + (g[i] - b1[i]) * sa));
  return g;
}
/** Halftone dots (cell n px, 45 degrees) and Line screen, draft scale (criteria 42, 43). */
function screenDither(src, w, h, n, kind) {
  const out = new Float32Array(w * h), c = Math.SQRT1_2;
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const u = (x * c + y * c) / n, v = (-x * c + y * c) / n;
    const fu = u - Math.floor(u) - 0.5, fv = v - Math.floor(v) - 0.5;
    const dark = 1 - src[y * w + x] / 255;
    const t = kind === 'lines' ? Math.abs(fv) * 2 : Math.min(1, Math.sqrt(fu * fu + fv * fv) / 0.7071);
    out[y * w + x] = t < dark ? 1 : 0;
  }
  return out;
}
function paintMap(canvas, map, w, h, px) {
  canvas.width = w * px; canvas.height = h * px; canvas.style.width = `${w * px}px`; canvas.style.height = `${h * px}px`;
  const ctx = canvas.getContext('2d'); ctx.fillStyle = '#C46A1C';
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) if (map[y * w + x]) ctx.fillRect(x * px, y * px, px, px);
}
/** Puts a tooltip above (or below) an element inside a positioned figure. */
function tipAt(fig, el, text, { dx = 0, below = false } = {}) {
  const r = el.getBoundingClientRect(), f = fig.getBoundingClientRect();
  fig.style.position = 'relative';
  fig.insertAdjacentHTML('beforeend', `<div class="tip" style="left:${r.left - f.left + dx}px;top:${below ? r.bottom - f.top + 6 : r.top - f.top - 26}px">${text}</div>`);
}
const figures = (id, screens) => {
  document.getElementById(id).innerHTML = screens.map(([t, s, p, a, w]) =>
    `<figure><figcaption>${t}<span>${s}</span></figcaption><div class="panel"${w ? ` style="width:${w}px"` : ''}>${p}</div>${a ? `<div class="ann">${a}</div>` : ''}</figure>`).join('');
};
