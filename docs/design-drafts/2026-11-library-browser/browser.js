// Builders of the 0152 library browser drafts; glyphs not in ../2026-11-library/kit.js live here.
// Each page fills its figures with pane(...), then calls finish() before kit.js replaces the rest.
const G2 = {
  grid: '<rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/>',
  sliders: '<path d="M21 4h-7"/><path d="M10 4H3"/><path d="M21 12h-9"/><path d="M8 12H3"/><path d="M21 20h-5"/><path d="M12 20H3"/><path d="M14 2v4"/><path d="M8 10v4"/><path d="M16 18v4"/>',
  check: '<path d="M20 6 9 17l-5-5"/>',
  back: '<path d="m12 19-7-7 7-7"/><path d="M19 12H5"/>',
  files: '<path d="M20 7h-3a2 2 0 0 1-2-2V2"/><path d="M9 18a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h7l4 4v10a2 2 0 0 1-2 2Z"/><path d="M3 7.6v12.8A1.6 1.6 0 0 0 4.6 22h9.8"/>',
  min: '<path d="m14 10 7-7"/><path d="M20 10h-6V4"/><path d="m3 21 7-7"/><path d="M4 14h6v6"/>',
  lock: '<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
  pick: '<path d="M14 4.1 12 6"/><path d="m5.1 8-2.9-.8"/><path d="m6 12-1.9 2"/><path d="M7.2 2.2 8 5.1"/><path d="M9.037 9.69a.498.498 0 0 1 .653-.653l11 4.5a.5.5 0 0 1-.074.949l-4.349 1.041a1 1 0 0 0-.74.739l-1.04 4.35a.5.5 0 0 1-.95.074z"/>',
};
// Monochrome sample icons (Lucide-style strokes); colour icons are filled SVG of their own.
const P = {
  "arrow-left": '<path d="m12 19-7-7 7-7"/><path d="M19 12H5"/>',
  "arrow-right": '<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>',
  "arrow-left-circle": '<circle cx="12" cy="12" r="10"/><path d="M16 12H8"/><path d="m12 8-4 4 4 4"/>',
  heart: '<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>',
  "heart-outline": '<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/><path d="M12 9v4"/>',
  cloud: '<path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/>',
  "cloud-rain": '<path d="M4 14.899A7 7 0 1 1 15.71 8h1.79a4.5 4.5 0 0 1 2.5 8.242"/><path d="M16 14v6"/><path d="M8 14v6"/><path d="M12 16v6"/>',
  moon: '<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>',
  leaf: '<path d="M11 20A7 7 0 0 1 9.8 6.1C15.5 5 17 4.48 19 2c1 2 2 4.18 2 8 0 5.5-4.78 10-10 10Z"/><path d="M2 21c0-3 1.85-5.36 5.08-6C9.5 14.52 12 13 13 12"/>',
  umbrella: '<path d="M22 12a10.06 10.06 1 0 0-20 0Z"/><path d="M12 12v8a2 2 0 0 0 4 0"/><path d="M12 2v1"/>',
  music: '<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>',
  key: '<circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/>',
  anchor: '<path d="M12 22V8"/><path d="M5 12H2a10 10 0 0 0 20 0h-3"/><circle cx="12" cy="5" r="3"/>',
  gift: '<rect x="3" y="8" width="18" height="4" rx="1"/><path d="M12 8v13"/><path d="M19 12v7a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-7"/><path d="M7.5 8a2.5 2.5 0 0 1 0-5A4.8 8 0 0 1 12 8a4.8 8 0 0 1 4.5-5 2.5 2.5 0 0 1 0 5"/>',
  coffee: '<path d="M10 2v2"/><path d="M14 2v2"/><path d="M16 8a1 1 0 0 1 1 1v8a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4V9a1 1 0 0 1 1-1h14a4 4 0 1 1 0 8h-1"/><path d="M6 2v2"/>',
  camera: '<path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3z"/><circle cx="12" cy="13" r="3"/>',
  home: '<path d="M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8"/><path d="M3 10a2 2 0 0 1 .709-1.528l7-5.999a2 2 0 0 1 2.582 0l7 5.999A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
  star: '<path d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z"/>',
  bell: '<path d="M10.268 21a2 2 0 0 0 3.464 0"/><path d="M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326"/>',
};
const C = {
  "sun-colour": '<circle cx="12" cy="12" r="5" fill="#FFC83D"/><g stroke="#F7A21B" stroke-width="2" stroke-linecap="round"><path d="M12 1.5v3"/><path d="M12 19.5v3"/><path d="M1.5 12h3"/><path d="M19.5 12h3"/><path d="m4.6 4.6 2 2"/><path d="m17.4 17.4 2 2"/><path d="m4.6 19.4 2-2"/><path d="m17.4 6.6 2-2"/></g>',
  "red-heart": '<path d="M12 21 4.2 13.6C1.6 11 1.6 6.9 4.2 4.7c2.3-2 5.6-1.5 7.8 1 2.2-2.5 5.5-3 7.8-1 2.6 2.2 2.6 6.3 0 8.9Z" fill="#F0384A"/><path d="M7 6.5c-1.2.3-2 1.3-2.2 2.6" stroke="#FFA3AD" stroke-width="1.4" fill="none" stroke-linecap="round"/>',
  rainbow: '<path d="M2 18a10 10 0 0 1 20 0" fill="none" stroke="#E8453C" stroke-width="2.4"/><path d="M4.6 18a7.4 7.4 0 0 1 14.8 0" fill="none" stroke="#F9BB2D" stroke-width="2.4"/><path d="M7.2 18a4.8 4.8 0 0 1 9.6 0" fill="none" stroke="#4BAF4F" stroke-width="2.4"/><path d="M9.8 18a2.2 2.2 0 0 1 4.4 0" fill="none" stroke="#3D7BE0" stroke-width="2.4"/>',
  "cloud-sun": '<circle cx="9" cy="9" r="4.5" fill="#FFC83D"/><path d="M8 20h10.5a3.8 3.8 0 0 0 .2-7.6 5.6 5.6 0 0 0-10.6 1.6A3 3 0 0 0 8 20Z" fill="#E3E8F0" stroke="#AEB8C6" stroke-width=".8"/>',
  leaf: '<path d="M4 20C4 10 10 4 20 4c0 10-6 16-16 16Z" fill="#43A047"/><path d="M4 20 15 9" stroke="#2E7D32" stroke-width="1.4" stroke-linecap="round"/>',
  logo: '<rect x="2" y="2" width="9.5" height="9.5" fill="#F25022"/><rect x="12.5" y="2" width="9.5" height="9.5" fill="#7FBA00"/><rect x="2" y="12.5" width="9.5" height="9.5" fill="#00A4EF"/><rect x="12.5" y="12.5" width="9.5" height="9.5" fill="#FFB900"/>',
};
// Template and page thumbnails: a white page with a few cut lines.
const T = {
  box: '<rect x="6" y="3" width="28" height="22" fill="#fff" stroke="#BBB" stroke-width=".4"/><path d="M10 8h8v6h-8zM21 8h9v6h-9zM10 16h20v5H10z" fill="none" stroke="#3A3A3F" stroke-width=".6"/>',
  sign: '<rect x="4" y="5" width="32" height="18" fill="#fff" stroke="#BBB" stroke-width=".4"/><rect x="8" y="9" width="24" height="10" rx="3" fill="none" stroke="#3A3A3F" stroke-width=".6"/><path d="M12 14h16" stroke="#3A3A3F" stroke-width="1.4"/>',
  lid: '<rect x="11" y="2" width="18" height="24" fill="#fff" stroke="#BBB" stroke-width=".4"/><rect x="14" y="7" width="12" height="9" fill="none" stroke="#3A3A3F" stroke-width=".6"/><circle cx="20" cy="20" r="2" fill="none" stroke="#3A3A3F" stroke-width=".6"/>',
  tag: '<rect x="6" y="4" width="28" height="20" fill="#fff" stroke="#BBB" stroke-width=".4"/><path d="M11 9h14l5 5-5 5H11z" fill="none" stroke="#3A3A3F" stroke-width=".6"/><circle cx="14" cy="14" r="1.2" fill="none" stroke="#3A3A3F" stroke-width=".5"/>',
  coaster: '<rect x="6" y="2" width="28" height="24" fill="#fff" stroke="#BBB" stroke-width=".4"/><circle cx="20" cy="14" r="9" fill="none" stroke="#3A3A3F" stroke-width=".6"/><circle cx="20" cy="14" r="6" fill="none" stroke="#E07A1F" stroke-width=".5"/>',
};
const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;");

function art(item, px) {
  if (item.skeleton) {
    return `<span class="sk" style="width:${Math.round(px * 0.6)}px;height:${Math.round(px / 4)}px"></span>`;
  }
  if (item.broken) {
    return '<i data-g="alert" style="color:var(--panel-muted-fg)"></i>';
  }
  if (item.thumb) {
    const w = Math.min(px, 224);
    return `<svg width="${w}" height="${Math.round(w * 0.7)}" viewBox="0 0 40 28" aria-hidden="true">${T[item.thumb]}</svg>`;
  }
  const e = Math.round(px * 0.75);
  if (item.colour) {
    return `<svg width="${e}" height="${e}" viewBox="0 0 24 24" aria-hidden="true">${C[item.colour]}</svg>`;
  }
  return `<svg class="ic" width="${e}" height="${e}" viewBox="0 0 24 24" aria-hidden="true">${P[item.icon]}</svg>`;
}

const chip = (item) => (item.lic ? `<span class="lic">${esc(item.lic)}</span>` : "");

function tile(item, S) {
  const text = S >= 96 || item.always;
  const pc = item.pages ? `<span class="pc"><i data-g="files" data-s="12"></i>${item.pages}</span>` : "";
  const meta = item.broken
    ? `${chip(item)}<span class="src">Cannot be loaded</span>`
    : `${chip(item)}<span class="src">${esc(item.src)}</span>`;
  return `<div class="t ${item.st ?? ""}" style="width:${S + 16}px"><div class="pv" style="width:${S}px;height:${S}px">${art(item, S)}${pc}</div>${
    text ? `<div class="tn">${esc(item.name)}</div><div class="tm">${meta}</div>` : ""
  }</div>`;
}

function card(c) {
  // Rework 2026-11-25: no button; the bottom-right slot is a state (nothing, Loaded, spinner).
  const state = c.busy
    ? '<i data-g="loader" data-s="14" class="spin"></i>'
    : c.loaded
      ? '<span class="cm"><i data-g="check" data-s="14"></i>Loaded</span>'
      : "";
  const head = c.samples
    ? `<div class="smp">${c.samples.map((n) => `<svg class="ic" style="fill:none;stroke:currentColor;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round" width="24" height="24" viewBox="0 0 24 24">${P[n]}</svg>`).join("")}</div>`
    : `<div class="smp" style="align-items:center${c.ready ? ";opacity:.55" : ""}"><i data-g="${c.glyph}"></i>${c.lock ? '<i data-g="lock" data-s="12"></i>' : ""}</div>`;
  const line3 = c.fail
    ? `<div class="cm">${c.fail === "Offline" ? '<i data-g="wifioff" data-s="14"></i>Offline' : `<i data-g="calert" data-s="14" style="color:var(--destructive)"></i>${esc(c.fail)}`}</div>`
    : `<div class="cm">${esc(c.meta)}</div>`;
  const line4 = c.lic ? `<div class="cl"><span class="lic">${esc(c.lic)}</span><span class="src">${esc(c.by ?? "")}</span></div>` : "";
  return `<div class="card ${c.st ?? ""}"><div>${head}</div><div class="cn" style="${c.ready ? "color:var(--panel-muted-fg)" : ""}">${esc(c.name)}</div>${line3}${line4}<span class="cb">${state}</span></div>`;
}

function slider(w, v, lo = 48, hi = 256) {
  const inner = w - 16;
  const x = 8 + ((v - lo) / (hi - lo)) * inner;
  return `<span class="slider" style="width:${w}px"><span class="tr"></span><span class="fl" style="width:${x - 8}px"></span><span class="th2" style="left:${x - 7}px"></span></span>`;
}

function field(o) {
  const inner = o.q
    ? `<i data-g="search" style="color:var(--panel-muted-fg)"></i><span class="tx2">${esc(o.q)}</span>${o.online ? '<span class="in new"><i data-g="globe"></i></span>' : ""}<span class="in"><i data-g="x"></i></span>`
    : `<i data-g="search" style="color:var(--panel-muted-fg)"></i><span class="tx2 ph">Search library</span>`;
  return `<span class="field" style="${o.wide ? "max-width:480px;min-width:200px" : ""}">${inner}</span>`;
}

function viewStrip(view) {
  const cell = (n) => `<span class="${view === n.toLowerCase() ? "on" : ""}">${n}</span>`;
  return `<span class="strip2 vs new" style="grid-template-columns:repeat(2,auto)">${cell("Items")}${cell("Sets")}</span>`;
}

function header(o) {
  const view = o.view ?? "items";
  const back = o.back ? `<span class="ib new" style="width:24px;height:24px" title="${esc(typeof o.back === "string" ? o.back : "Back")}"><i data-g="back"></i></span>` : "";
  const subj = `<span class="subj">${back}${o.subj}</span>`;
  const btns = `<span class="ib">${o.full ? '<i data-g="min"></i>' : '<i data-g="max"></i>'}</span><span style="width:4px"></span><span class="ib"><i data-g="x"></i></span>`;
  const size = view === "items" ? `<span class="lbl">Size</span>${slider(o.wide ? 120 : 96, o.S)}<span class="ro">${o.S} px</span>` : "";
  const fld = o.guide ? "" : field({ ...o, wide: o.wide });
  if (o.wide) {
    return `<div class="bh"><div class="row">${viewStrip(view)}<span style="width:12px"></span>${fld}${fld ? '<span style="width:16px"></span>' : ""}${size}${size ? '<span style="width:16px"></span>' : ""}${subj}<span style="width:8px"></span>${btns}</div></div>`;
  }
  const filters = o.guide ? "" : `<span class="obtn ${o.band ? "on" : ""}"><i data-g="sliders"></i>Filters${o.active ? ` <span class="c">${o.active}</span>` : ""}</span><span style="width:12px"></span>`;
  return `<div class="bh"><div class="row">${viewStrip(view)}<span style="width:8px"></span>${fld || '<span style="flex:1"></span>'}<span style="width:4px"></span>${btns}</div><div class="row">${filters}${size}${size ? '<span style="width:12px"></span>' : ""}${subj}</div></div>`;
}

function strip(cells, on, cols, dis = []) {
  return `<div class="strip2" style="grid-template-columns:repeat(${cols},auto)">${cells
    .map((c) => `<span class="${c === on ? "on" : ""} ${dis.includes(c) ? "dis" : ""}">${c}</span>`)
    .join("")}</div>`;
}

function checks(rows) {
  return rows
    .map(
      (r) =>
        `<div class="ck ${r.on ? "on" : ""} ${r.k === 0 || r.k === "0" ? "zero" : ""} ${r.ind ? "ind" : ""}"><span class="cs">${r.on ? '<i data-g="check" data-s="14"></i>' : ""}</span><span class="n">${esc(r.n)}</span><span class="k">${r.k}</span></div>`,
    )
    .join("");
}

function facets(o) {
  const sec = (h, inner) => `<div class="fs"><div class="hd2">${h}</div>${inner}</div>`;
  if (o.view === "sets") {
    return `<div class="facets ${o.band ? "band" : "col"} new">
<div class="fh">Filters${o.clear ? '<span class="clr">Clear all</span>' : ""}</div>
${sec("Sources", checks(o.sources))}
${sec("Colour", strip(["All", "Monochrome", "Colour"], o.colour ?? "All", 3))}
${sec("Category", o.cats ? checks(o.cats) : '<div class="sb" style="padding:4px">No categories in the fetched items.</div>')}
</div>`;
  }
  return `<div class="facets ${o.band ? "band" : "col"} new">
<div class="fh">Filters${o.clear ? '<span class="clr">Clear all</span>' : ""}</div>
${sec("Sort", strip(["Best match", "Name", "Source", "Newest"], o.sort ?? "Name", 2, o.q ? [] : ["Best match"]))}
${sec("Sources", checks(o.sources))}
${sec("Colour", strip(["All", "Monochrome", "Colour"], o.colour ?? "All", 3))}
<div class="ck" style="padding:0 4px"><span class="n">Customizable</span><span class="sw ${o.cust ? "on" : ""}"></span></div>
${sec("Category", o.cats ? checks(o.cats) : '<div class="sb" style="padding:4px">No categories in the fetched items.</div>')}
</div>`;
}

const ACT_GLYPH = { "Show pages": "files", "Pick elements…": "pick" };

function detail(o) {
  const h = o.wide ? 113 : 89;
  const colW = o.wide ? 112 : 28;
  if (!o.item) {
    return `<div class="db new" style="height:${h}px;align-items:center"><span class="empty" style="margin:0">Nothing selected</span></div>`;
  }
  const px = o.wide ? 96 : 72;
  const it = o.item;
  // Rework 2026-11-25: no button loads; only Show pages and Pick elements… remain, the column is reserved.
  const acts = o.hoverOnly ? [] : it.actions ?? [];
  const buttons = acts
    .map((a) => (o.wide ? `<span class="obtn" style="width:112px">${a}</span>` : `<span class="obtn" style="width:28px;padding:0" title="${a}"><i data-g="${ACT_GLYPH[a]}"></i></span>`))
    .join("");
  const box = it.samples
    ? `<span style="display:flex;gap:4px">${it.samples.map((n) => `<svg class="ic" style="fill:none;stroke:currentColor;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round" width="${o.wide ? 28 : 20}" height="${o.wide ? 28 : 20}" viewBox="0 0 24 24">${P[n]}</svg>`).join("")}</span>`
    : art(it, px);
  const lines = `<div class="dl">${esc(it.place)}</div><div class="dk">${it.lic ? `<span class="link">${esc(it.lic)}</span> · ` : ""}${esc(it.by)}</div>${o.wide ? `<div class="dl">${esc(it.facts)}</div>` : ""}`;
  return `<div class="db new" style="height:${h}px"><div class="box" style="width:${px}px;height:${px}px">${box}</div><div class="dt"><div class="dn" style="${o.wide ? "" : "-webkit-line-clamp:1"}">${esc(it.name)}</div>${lines}</div><div class="bt" style="width:${colW}px">${buttons}</div></div>`;
}

function guide(o) {
  // The hosting guide: a place of the browser (rework 2026-11-25, UX notes 0.6). Outline text; the PO writes the final words.
  return `<div class="grid guide new" style="display:block;overflow:hidden;padding:16px"><div style="max-width:640px;font-size:14px;line-height:1.5">
<div style="font-size:20px;font-weight:600;margin-bottom:8px">Host your own library</div>
<p style="margin:0 0 12px">A library is a Git repository with one file <code>curvyo-catalogue.json</code> at its top and one folder per template.</p>
<div class="hd2" style="padding:0 0 4px">Folder of a template</div>
<pre class="code">box-with-lid/
  template.curvyo     the document
  thumbnail.png       at most 1 MiB
  template.toml       your source for the index</pre>
<div class="hd2" style="padding:8px 0 4px">Steps</div>
<ol style="margin:0 0 12px;padding-left:20px"><li>Create a repository, for example on GitHub.</li><li>Add one folder per template.</li><li>Run the index script of the example repository. It writes <code>curvyo-catalogue.json</code> with the hashes.</li><li>Push.</li><li>In Curvyo: Library tab, Add source…, Git catalogue, the address of the raw files:<br><code>https://raw.githubusercontent.com/&lt;owner&gt;/&lt;repo&gt;/main/</code></li></ol>
<div class="hd2" style="padding:0 0 4px;display:flex;align-items:center">Example<span class="obtn s24" style="margin-left:auto">Copy</span></div>
<pre class="code">{ "catalogue_format": 1, "name": "Maker Guild",
  "templates": [ { "path": "box-with-lid", "name": "Box with lid",
    "author": "Jane Doe", "tags": ["box", "customizable"],
    "licence": { "spdx": "CC-BY-4.0", "name": "CC BY 4.0", "url": "…" },
    "credit": "Jane Doe", "fonts": [], "size": 24576,
    "sha256": "…", "thumbnail_sha256": "…", "format_version": 61 } ] }</pre>
<p style="margin:8px 0 0"><span class="link">Example repository</span> · github.com/curvyo/templates</p>
<div class="hd2" style="padding:12px 0 4px">Private repositories</div>
<p style="margin:0">${o.token ? "Create a read-only access token and enter it under Address when you add the source." : "The repository must be public."} <span class="sb">(Frage 376 ${o.token ? "B" : "A"})</span></p>
</div></div>`;
}

function gridHtml(o) {
  if (o.guide) return guide(o);
  if (o.groups) {
    let g = o.empty ? `<div class="emp">${o.empty}</div>` : "";
    g += o.groups.map((gr) => `<div class="ghd">${gr.head}${gr.lock ? ' <i data-g="lock" data-s="12"></i>' : ""}</div>` + gr.cards.map(card).join("")).join("");
    g += `<div style="width:100%;height:28px;margin-top:16px;display:flex;align-items:center"><span class="hostlink new">Host your own library…</span></div>`;
    return `<div class="grid">${g}</div>`;
  }
  let h = o.empty ? `<div class="emp">${o.empty}${o.clearBtn ? '<div style="margin-top:12px"><span class="obtn">Clear all</span></div>' : ""}${o.showSets ? '<div style="margin-top:12px"><span class="obtn new">Show sets</span></div>' : ""}</div>` : "";
  h += (o.items ?? []).map((it) => tile(it, o.S)).join("");
  if (o.cards && o.cards.length) {
    h += `<div class="ghd">${o.cardsHead}</div>` + o.cards.map(card).join("");
  }
  return `<div class="grid">${h}</div>`;
}

function pane(o) {
  const body = o.guide
    ? `<div class="body">${gridHtml(o)}</div>`
    : o.wide
    ? `<div class="body">${facets(o)}${gridHtml(o)}</div>`
    : `${o.band ? facets({ ...o, band: true }) : ""}<div class="body">${gridHtml(o)}</div>`;
  return `<div class="pane new ${o.full ? "full" : ""}" style="${o.full ? "" : `width:${o.w}px`}">${o.full ? "" : '<span class="divider"></span>'}${header(o)}${body}${o.guide ? "" : detail(o)}</div>`;
}

function finish() {
  document.querySelectorAll("i[data-g]").forEach((el) => {
    if (!G2[el.dataset.g]) return;
    const s = el.dataset.s ? ` s${el.dataset.s}` : "";
    el.outerHTML = `<svg class="g${s}${el.className ? " " + el.className : ""}" viewBox="0 0 24 24" aria-hidden="true">${G2[el.dataset.g]}</svg>`;
  });
}

// Shared sample data.
const I = {
  al: { icon: "arrow-left", name: "arrow-left", src: "Lucide", lic: "ISC", place: "Iconify › Lucide", by: "Lucide Contributors", facts: "Monochrome · takes the object's colour", actions: [] },
  alc: { icon: "arrow-left-circle", name: "arrow-left-circle", src: "Tabler", lic: "MIT", place: "Iconify › Tabler Icons", by: "Paweł Kuna", facts: "Monochrome · takes the object's colour", actions: [] },
  ar: { icon: "arrow-right", name: "arrow-right", src: "Lucide", lic: "ISC" },
  an: { icon: "anchor", name: "anchor", src: "Material Symbols", lic: "Apache 2.0" },
  bell: { icon: "bell", name: "bell", src: "Lucide", lic: "ISC" },
  cam: { icon: "camera", name: "camera", src: "Tabler", lic: "MIT" },
  cl: { icon: "cloud", name: "cloud", src: "Material Symbols", lic: "Apache 2.0" },
  clr: { icon: "cloud-rain", name: "cloud-rain", src: "Lucide", lic: "ISC" },
  cof: { icon: "coffee", name: "coffee", src: "Lucide", lic: "ISC" },
  gift: { icon: "gift", name: "gift", src: "Tabler", lic: "MIT" },
  heart: { icon: "heart", name: "heart", src: "Lucide", lic: "ISC", place: "Iconify › Lucide", by: "Lucide Contributors", facts: "Monochrome · takes the object's colour", actions: [] },
  heart2: { icon: "heart-outline", name: "heart-plus", src: "Tabler", lic: "MIT" },
  home: { icon: "home", name: "home", src: "Material Symbols", lic: "Apache 2.0" },
  key: { icon: "key", name: "key", src: "Tabler", lic: "MIT" },
  leaf: { icon: "leaf", name: "leaf", src: "Lucide", lic: "ISC" },
  moon: { icon: "moon", name: "moon", src: "Material Symbols", lic: "Apache 2.0" },
  mus: { icon: "music", name: "music", src: "Lucide", lic: "ISC" },
  star: { icon: "star", name: "star", src: "Material Symbols", lic: "Apache 2.0" },
  sun: { icon: "sun", name: "sun", src: "Lucide", lic: "ISC" },
  umb: { icon: "umbrella", name: "umbrella", src: "Lucide", lic: "ISC" },
  csun: { colour: "sun-colour", name: "sun-with-rays", src: "Noto Emoji", lic: "Apache 2.0", place: "Iconify › Noto Emoji", by: "Google", facts: "Colour", actions: [] },
  cheart: { colour: "red-heart", name: "red-heart", src: "Noto Emoji", lic: "Apache 2.0", place: "Iconify › Noto Emoji", by: "Google", facts: "Colour", actions: [] },
  crain: { colour: "rainbow", name: "rainbow", src: "Noto Emoji", lic: "Apache 2.0" },
  ccs: { colour: "cloud-sun", name: "sun-behind-cloud", src: "Noto Emoji", lic: "Apache 2.0" },
  cleaf: { colour: "leaf", name: "leaf-fluttering", src: "Fluent Emoji", lic: "MIT" },
  logo: { colour: "logo", name: "microsoft", src: "Logos", lic: "CC0 1.0" },
  tbox: { thumb: "box", name: "Box with lid", src: "Curvyo catalogue", lic: "CC BY 4.0", pages: 2, place: "Curvyo catalogue", by: "Jane Doe", facts: "2 pages · 24 KB · Font not included", actions: ["Show pages"] },
  tsign: { thumb: "sign", name: "Door sign", src: "Curvyo catalogue", lic: "CC0 1.0" },
  ttag: { thumb: "tag", name: "Gift tag", src: "Curvyo catalogue", lic: "Licence unknown" },
  tco: { thumb: "coaster", name: "Coaster set", src: "Maker Guild", lic: "CC BY-SA 4.0", pages: 4 },
  dlid: { thumb: "lid", name: "Lid", src: "Box job", pages: 3, place: "Box job › Lid", by: "Box job", facts: "3 pages · changed 24 Nov 12:03", actions: ["Show pages"] },
  dfront: { thumb: "lid", name: "Front", src: "Lid", place: "Box job › Lid › Front", by: "Box job", facts: "210 × 297 mm · Drawing page", actions: ["Pick elements…"] },
};
const SETS = {
  lu: { samples: ["arrow-left", "heart", "bell"], name: "Lucide", meta: "1,540 icons · General", lic: "ISC", by: "Lucide Contributors", loaded: true, place: "Iconify › api.iconify.design", facts: "1,540 icons · General · Monochrome · Loaded" },
  tb: { samples: ["gift", "key", "camera"], name: "Tabler Icons", meta: "5,880 icons · General", lic: "MIT", by: "Paweł Kuna", loaded: true },
  no: { samples: ["sun", "star", "heart"], name: "Noto Emoji", meta: "3,710 icons · Emoji", lic: "Apache 2.0", by: "Google", loaded: true },
  mdi: { samples: ["home", "star", "bell"], name: "Material Design Icons", meta: "7,447 icons · Material", lic: "Apache 2.0", by: "Pictogrammers", place: "Iconify › api.iconify.design", facts: "7,447 icons · Material · Monochrome · Not loaded" },
  ph: { samples: ["heart", "leaf", "moon"], name: "Phosphor", meta: "9,072 icons · General", lic: "MIT", by: "Phosphor Icons" },
  wi: { samples: ["sun", "cloud", "umbrella"], name: "Weather Icons", meta: "219 icons · Thematic", lic: "OFL 1.1", by: "Erik Flowers" },
  fe: { samples: ["key", "camera", "gift"], name: "Feather Icons", meta: "286 icons · General", lic: "MIT", by: "Cole Bemis" },
};
