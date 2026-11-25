// Builders for the 0154 drafts: a laser page with one holder, its slots, the fitted ghost, the
// content of a filled slot, the Drop badge and the canvas notice. Real tokens from ../2026-11-library/kit.css.
const EXTRA = {
  magnet: '<path d="m12 15 4 4"/><path d="M2.352 10.648a1.205 1.205 0 0 0 0 1.704l2.296 2.296a1.205 1.205 0 0 0 1.704 0l6.029-6.029a1 1 0 1 1 3 3l-6.029 6.029a1.205 1.205 0 0 0 0 1.704l2.296 2.296a1.205 1.205 0 0 0 1.704 0l6.365-6.367A1 1 0 0 0 8.716 4.282z"/><path d="m5 8 4 4"/>',
  fill: '<path d="M12 3v17a1 1 0 0 1-1 1H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v6a1 1 0 0 1-1 1H3"/><path d="M16 19h6"/><path d="M19 22v-6"/>',
  srect: '<rect x="5" y="7" width="14" height="10" rx="0.5"/>',
  scirc: '<circle cx="12" cy="12" r="6"/>',
  group: '<path d="M3 7V5c0-1.1.9-2 2-2h2"/><path d="M17 3h2c1.1 0 2 .9 2 2v2"/><path d="M21 17v2c0 1.1-.9 2-2 2h-2"/><path d="M7 21H5c-1.1 0-2-.9-2-2v-2"/><rect width="7" height="5" x="7" y="7" rx="1"/><rect width="7" height="5" x="10" y="12" rx="1"/>',
  holder: '<rect x="2" y="4" width="20" height="16" rx="1"/><circle cx="8" cy="12" r="2.5"/><circle cx="16" cy="12" r="2.5"/>',
};
const g = (n, s = 16) => `<svg class="g" style="width:${s}px;height:${s}px" viewBox="0 0 24 24" aria-hidden="true">${EXTRA[n]}</svg>`;

const INK = "#3A3A3F", ACC = "#2F6FEE", GHOST = "color-mix(in srgb,#2F6FEE 60%,transparent)";
// The piece: a 60 x 40 mm page with a heart and a name, drawn in a box w x h at x, y.
function piece(x, y, w, h, mode) {
  const s = Math.min(w / 60, h / 40);
  const ox = x + (w - 60 * s) / 2, oy = y + (h - 40 * s) / 2;
  const stroke = mode === "ghost" ? GHOST : INK;
  const casing = mode === "ghost" ? `stroke="#fff" stroke-width="4"` : `stroke="none"`;
  const heart = `M${ox + 30 * s} ${oy + 31 * s}c${-9 * s} ${-6 * s} ${-14 * s} ${-10 * s} ${-14 * s} ${-15 * s}c0 ${-4 * s} ${3 * s} ${-7 * s} ${7 * s} ${-7 * s}c${3 * s} 0 ${5 * s} ${2 * s} ${7 * s} ${4 * s}c${2 * s} ${-2 * s} ${4 * s} ${-4 * s} ${7 * s} ${-4 * s}c${4 * s} 0 ${7 * s} ${3 * s} ${7 * s} ${7 * s}c0 ${5 * s} ${-5 * s} ${9 * s} ${-14 * s} ${15 * s}z`;
  let out = "";
  if (mode === "ghost") {
    out += `<rect x="${ox}" y="${oy}" width="${60 * s}" height="${40 * s}" fill="none" ${casing}/>`;
    out += `<rect x="${ox}" y="${oy}" width="${60 * s}" height="${40 * s}" fill="none" stroke="${stroke}" stroke-width="1.5"/>`;
    out += `<path d="${heart}" fill="none" ${casing}/>`;
  }
  out += `<path d="${heart}" fill="${mode === "ghost" ? "none" : "#E06C75"}" stroke="${stroke}" stroke-width="1.5"/>`;
  return out;
}

// A holder: board and slots in px. slot: {x,y,w,h,circle,name,state}; state: lit, ghost, filled,
// filledlit (refused), armed.
function holder(b, slots) {
  let o = `<rect x="${b.x}" y="${b.y}" width="${b.w}" height="${b.h}" fill="#DCDCE0" stroke="${INK}" stroke-width="1.5"/>`;
  o += `<text x="${b.x + 6}" y="${b.y + 14}" font-size="11" fill="${INK}">${b.name}</text>`;
  for (const s of slots) {
    const lit = s.state === "lit" || s.state === "ghost";
    const fill = lit ? "color-mix(in srgb,#2F6FEE 20%,#E8E8EB)" : "#E8E8EB";
    const line = lit || s.state === "filledlit" ? `stroke="${ACC}" stroke-width="2"` : `stroke="${INK}" stroke-width="1"`;
    o += s.circle
      ? `<circle cx="${s.x + s.w / 2}" cy="${s.y + s.h / 2}" r="${s.w / 2}" fill="${fill}" ${line}/>`
      : `<rect x="${s.x}" y="${s.y}" width="${s.w}" height="${s.h}" fill="${fill}" ${line}/>`;
    const filled = s.state === "filled" || s.state === "filledlit";
    if (!filled && s.name) o += `<text x="${s.x + s.w / 2}" y="${s.y + s.h / 2 + 4}" font-size="11" text-anchor="middle" fill="${INK}" opacity="${lit ? 0.35 : 1}">${s.name}</text>`;
    const inner = s.circle ? { x: s.x + s.w * 0.146, y: s.y + s.h * 0.146, w: s.w * 0.707, h: s.h * 0.707 } : s;
    if (filled) o += piece(inner.x, inner.y, inner.w, inner.h, "art");
    if (s.state === "ghost") o += piece(inner.x, inner.y, inner.w, inner.h, "ghost");
    if (s.big) o += piece(s.big.x, s.big.y, s.big.w, s.big.h, "ghost");
  }
  return o;
}
const cursor = (x, y) => `<svg style="position:absolute;left:${x}px;top:${y}px" width="14" height="20" viewBox="0 0 14 20"><path d="M1 1v15l4-4 3 6 2-1-3-6h5z" fill="#000" stroke="#fff"/></svg>`;
const badge = (x, y, glyph, word) => `<div class="badge" style="left:${x}px;top:${y}px">${EXTRA[glyph] ? g(glyph, 12) : `<i data-g="${glyph}" data-s="12"></i>`}${word}</div>`;
function scene(w, h, svg, extra = "") {
  return `<div class="area" style="width:${w}px;height:${h}px"><div class="pg" style="left:16px;top:16px;width:${w - 32}px;height:${h - 32}px"></div><svg style="position:absolute;left:0;top:0" width="${w}" height="${h}">${svg}</svg>${extra}</div>`;
}
// The standard holder of the drafts: 4 slots (30 x 30, 60 x 40, circle 25, 30 x 30); 1 mm = 2 px.
function jig(states, opts = {}) {
  const b = { x: 40, y: 40, w: 300, h: 150, name: "Holder 1" };
  const sl = [
    { x: 60, y: 76, w: 60, h: 60, name: "Slot 1" },
    { x: 140, y: 76, w: 120, h: 80, name: "Slot 2" },
    { x: 276, y: 86, w: 50, h: 50, circle: true, name: "Slot 3" },
  ];
  sl.forEach((s, i) => Object.assign(s, states[i] ?? {}));
  return holder(b, sl) + (opts.after ?? "");
}
