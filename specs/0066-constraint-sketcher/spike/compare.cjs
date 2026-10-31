// Run after cargo run and wasm-bindgen --target nodejs --out-dir pkg.
const fs = require('fs');
const m = require('./pkg/curvyo_0066_solver_spike.js');
const text = m.corpus_results();
fs.writeFileSync('evidence/wasm-results.json', text);
const native = JSON.parse(fs.readFileSync('evidence/native-results.json'));
const wasm = JSON.parse(text);
let numericValues = 0, maxDelta = 0;
function compare(a, b, path = '') {
  if (typeof a === 'number') {
    if (!Number.isFinite(b)) throw Error(path);
    maxDelta = Math.max(maxDelta, Math.abs(a-b));
    if (Math.abs(a-b) > 1e-6) throw Error(`${path}: ${a} vs ${b}`);
    numericValues++;
  } else if (a && typeof a === 'object') {
    for (const key of Object.keys(a)) compare(a[key], b[key], `${path}/${key}`);
  } else if (a !== b) throw Error(`${path}: ${a} vs ${b}`);
}
compare(native, wasm);
const result = {cases: native.length, numericValues, maxDelta, failed: wasm.filter(c => c.errors.length)};
fs.writeFileSync('evidence/wasm-comparison.json', JSON.stringify(result, null, 2));
console.log(result);
if (result.failed.length) process.exitCode = 1;
