// `specs/0025-raster-trace` criterion 18 and `specs/0089` criterion 13a: the wasm build (the one
// the worker runs in both builds) traces the golden fixtures of `curvyo-vectorize-core` exactly like
// the native build: equal path and node counts, the golden points within 1e-6 mm (the tracer uses
// basic operations and `sqrt` only), the boxes within the sampling sag of this test. Needs the wasm
// bindings (`npm run build:wasm`).

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

const ROOT = path.resolve(import.meta.dirname, "..", "..");
const FIXTURES = path.join(ROOT, "curvyo-vectorize-core", "tests", "fixtures", "trace");
const BINDINGS = path.resolve(import.meta.dirname, "..", "src", "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");

interface Golden {
  file: string;
  map: number[];
  threshold: number;
  smooth: number;
  specks_mm: number;
  paths: number;
  nodes: number;
  boxes: number[][];
  points: number[][][];
}

/** The anchors of each outline in the worker's numbers (move, line and cubic end points). */
function anchors(wire: Float64Array): number[][][] {
  const out: number[][][] = [];
  let at = 7;
  for (let o = 0; o < wire[6]; o++) {
    const elements = wire[at++];
    const points: number[][] = [];
    for (let e = 0; e < elements; e++) {
      const tag = wire[at++];
      if (tag === 0 || tag === 1) {
        points.push([wire[at], wire[at + 1]]);
        at += 2;
      } else if (tag === 3) {
        points.push([wire[at + 4], wire[at + 5]]);
        at += 6;
      }
    }
    out.push(points);
  }
  return out;
}

/** The RGBA of a fixture: binary PGM (grey) or PAM (RGBA). */
function readPicture(file: string): { width: number; height: number; rgba: Uint8Array } {
  const bytes = readFileSync(path.join(FIXTURES, file));
  if (bytes.subarray(0, 3).toString("latin1") === "P7\n") {
    const end = bytes.indexOf("ENDHDR\n") + 7;
    const header = bytes.subarray(0, end).toString("latin1");
    const field = (key: string) => Number(header.split("\n").find((l) => l.startsWith(`${key} `))?.split(" ")[1]);
    return { width: field("WIDTH"), height: field("HEIGHT"), rgba: new Uint8Array(bytes.subarray(end)) };
  }
  const fields = bytes.subarray(0, 20).toString("latin1").split(/\s+/);
  const [width, height] = [Number(fields[1]), Number(fields[2])];
  const grey = bytes.subarray(bytes.length - width * height);
  const rgba = new Uint8Array(width * height * 4);
  grey.forEach((v, i) => rgba.set([v, v, v, 255], i * 4));
  return { width, height, rgba };
}

/** The tight box of the outlines in the worker's numbers (cubic extrema included). */
function boxes(wire: Float64Array): number[][] {
  const out: number[][] = [];
  let at = 7;
  const outlines = wire[6];
  for (let o = 0; o < outlines; o++) {
    const elements = wire[at++];
    let box = [Infinity, Infinity, -Infinity, -Infinity];
    const add = (x: number, y: number) => {
      box = [Math.min(box[0], x), Math.min(box[1], y), Math.max(box[2], x), Math.max(box[3], y)];
    };
    let last = [0, 0];
    for (let e = 0; e < elements; e++) {
      const tag = wire[at++];
      if (tag === 0 || tag === 1) {
        last = [wire[at], wire[at + 1]];
        add(last[0], last[1]);
        at += 2;
      } else if (tag === 3) {
        const p = [last, [wire[at], wire[at + 1]], [wire[at + 2], wire[at + 3]], [wire[at + 4], wire[at + 5]]];
        at += 6;
        for (let t = 0; t <= 1000; t++) {
          const s = t / 1000;
          const u = 1 - s;
          const w = [u * u * u, 3 * u * u * s, 3 * u * s * s, s * s * s];
          add(w.reduce((a, k, i) => a + k * p[i][0], 0), w.reduce((a, k, i) => a + k * p[i][1], 0));
        }
        last = p[3];
      }
    }
    out.push(box);
  }
  return out;
}

test("criterion 18: the wasm trace matches the native goldens", { skip: !existsSync(WASM) }, async () => {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    trace_job: (rgba: Uint8Array, w: number, h: number, map: Float64Array, settings: string, progress: { report: (percent: number) => void }) => Float64Array;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  const goldens = JSON.parse(readFileSync(path.join(FIXTURES, "goldens.json"), "utf8")) as Record<string, Golden>;
  for (const [name, golden] of Object.entries(goldens)) {
    const picture = readPicture(golden.file);
    const wire = bindings.trace_job(
      picture.rgba,
      picture.width,
      picture.height,
      new Float64Array(golden.map),
      JSON.stringify({ threshold: golden.threshold, smooth: golden.smooth, specks: golden.specks_mm }),
      { report() {} },
    );
    assert.equal(wire[0], 0, `${name}: traced`);
    assert.equal(wire[1], golden.paths, `${name}: paths`);
    assert.equal(wire[2], golden.nodes, `${name}: nodes`);
    const points = anchors(wire);
    golden.points.forEach((outline, i) => {
      outline.forEach((p, k) => {
        assert.ok(Math.abs(points[i][k][0] - p[0]) <= 1e-6 && Math.abs(points[i][k][1] - p[1]) <= 1e-6, `${name} point ${i}.${k}: ${points[i][k]} vs ${p}`);
      });
    });
    const actual = boxes(wire);
    golden.boxes.forEach((expected, i) => {
      expected.forEach((v, k) => {
        // The sampled box lies inside the tight one by at most the sampling step's sag.
        assert.ok(Math.abs(actual[i][k] - v) <= 0.01, `${name} box ${i}[${k}]: ${actual[i][k]} vs ${v}`);
      });
    });
  }
});

test("criterion 10: the wasm trace reports a rising percentage that ends at 100", { skip: !existsSync(WASM) }, async () => {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    trace_job: (rgba: Uint8Array, w: number, h: number, map: Float64Array, settings: string, progress?: { report: (percent: number) => void }) => Float64Array;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  const [name, golden] = Object.entries(JSON.parse(readFileSync(path.join(FIXTURES, "goldens.json"), "utf8")) as Record<string, Golden>)[0];
  const picture = readPicture(golden.file);
  const seen: number[] = [];
  const wire = bindings.trace_job(picture.rgba, picture.width, picture.height, new Float64Array(golden.map), JSON.stringify({ threshold: golden.threshold, smooth: golden.smooth, specks: golden.specks_mm }), {
    report: (percent) => seen.push(percent),
  });
  assert.equal(wire[0], 0, `${name}: traced`);
  assert.ok(seen.length > 1, `reports: ${seen}`);
  assert.ok(seen.every((p, i) => i === 0 || p > seen[i - 1]), `rising: ${seen}`);
  assert.equal(seen.at(-1), 100);
});

test("criteria 6, 45, 59, 60: the schema and the presets equal the copies in traceText", { skip: !existsSync(WASM) }, async () => {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    trace_params: () => string;
    trace_presets: () => string;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  const text = await import("../src/lib/traceText.ts");
  assert.deepEqual(JSON.parse(bindings.trace_params()), text.TRACE_PARAMS_FALLBACK);
  assert.deepEqual(JSON.parse(bindings.trace_presets()), text.TRACE_PRESETS_FALLBACK);
});
