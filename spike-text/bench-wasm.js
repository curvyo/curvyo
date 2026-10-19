// Node benchmark of the wasm build: node spike-text/bench-wasm.js <pkg dir> <font>...
const path = require('path');
const fs = require('fs');
const m = require(path.resolve(process.argv[2], 'spike.js'));
const base = 'AVATAR Toffee: the office waffle flavour fits five fine fjords. ' +
  'Typewriters, LAVA, Yacht, We fly off. fi fl ff ffi ffl! ';
let text = '';
while ([...text].length < 500) text += base;
text = [...text].slice(0, 500).join('');
const stats = (v) => { v.sort((a, b) => a - b); return `min ${v[0].toFixed(3)} median ${v[v.length >> 1].toFixed(3)} p99 ${v[Math.floor(v.length * 0.99)].toFixed(3)}`; };
for (const font of process.argv.slice(3)) {
  const bytes = new Uint8Array(fs.readFileSync(font));
  const first = performance.now();
  m.shape_cold_js(bytes, text);
  const firstMs = performance.now() - first;
  const cold = [], warm = [], rb = [];
  for (let i = 0; i < 300; i++) { const t = performance.now(); m.shape_cold_js(bytes, text); cold.push(performance.now() - t); }
  const face = new m.WasmFace(bytes);
  for (let i = 0; i < 300; i++) { const t = performance.now(); face.shape(text); warm.push(performance.now() - t); }
  if (m.probe_rb) for (let i = 0; i < 300; i++) { const t = performance.now(); m.probe_rb(bytes, text); rb.push(performance.now() - t); }
  console.log(path.basename(font));
  console.log(`  first call (incl. JIT) ms: ${firstMs.toFixed(3)}`);
  console.log(`  harfrust+skrifa cold ms: ${stats(cold)}`);
  console.log(`  harfrust+skrifa warm ms: ${stats(warm)}`);
  if (rb.length) console.log(`  rustybuzz+ttf-parser cold ms: ${stats(rb)}`);
}
