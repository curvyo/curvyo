// `specs/0089-trace-quality-and-colour` criterion 3: the thread CPU time of the wasm trace (the
// build the worker runs in both apps) for every run of the reference set. Reads the pictures and
// the manifest that the native harness `reference_table` writes to `target/trace-reference/`,
// traces each with `trace_job` and writes `wasm-times.json` there, which the next
// `reference_table` run merges as `cpu_wasm_ms`. Needs the wasm bindings (`npm run build:wasm`):
//
//   node --experimental-strip-types scripts/trace-reference-wasm.ts
//
// `CURVYO_TRACE_WASM_DIR` reads and writes another directory (criterion 30: the multicolour
// pictures of the `colour` test `criterion_30_wasm_manifest`, `target/trace-colour-wasm/`).

import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..", "..");
const DIR = process.env.CURVYO_TRACE_WASM_DIR ? path.resolve(ROOT, process.env.CURVYO_TRACE_WASM_DIR) : path.join(ROOT, "target", "trace-reference");
const BINDINGS = path.resolve(import.meta.dirname, "..", "src", "wasm-bindings");

interface Run {
  key: string;
  file: string;
  map: number[];
  settings: Record<string, number | string | null>;
}

/** The RGBA of a PAM (`RGB_ALPHA`) picture. */
function readPam(file: string): { width: number; height: number; rgba: Uint8Array } {
  const bytes = readFileSync(path.join(DIR, file));
  const end = bytes.indexOf("ENDHDR\n") + 7;
  const header = bytes.subarray(0, end).toString("latin1");
  const field = (key: string) => Number(header.split("\n").find((l) => l.startsWith(`${key} `))?.split(" ")[1]);
  return { width: field("WIDTH"), height: field("HEIGHT"), rgba: new Uint8Array(bytes.subarray(end)) };
}

const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
  initSync: (options: { module: BufferSource }) => unknown;
  trace_job: (rgba: Uint8Array, w: number, h: number, map: Float64Array, settings: string, progress: { report: (percent: number) => void }) => Float64Array;
};
bindings.initSync({ module: readFileSync(path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm")) });

const runs = JSON.parse(readFileSync(path.join(DIR, "manifest.json"), "utf8")) as Run[];
const times: Record<string, number> = {};
for (const run of runs) {
  const picture = readPam(run.file);
  const before = process.threadCpuUsage();
  bindings.trace_job(picture.rgba, picture.width, picture.height, new Float64Array(run.map), JSON.stringify(run.settings), { report() {} });
  const spent = process.threadCpuUsage(before);
  times[run.key] = Math.round((spent.user + spent.system) / 10) / 100;
}
writeFileSync(path.join(DIR, "wasm-times.json"), `${JSON.stringify(times, null, 2)}\n`);
console.log(`${runs.length} wasm traces timed, written to ${path.join(DIR, "wasm-times.json")}`);
