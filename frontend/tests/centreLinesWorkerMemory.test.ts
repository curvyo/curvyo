// The Find centre lines worker's memory (`specs/0117-centre-lines` criterion 18, ADR 0063
// amendment 2026-11-18, W, Memory): the browser's wasm module in Node, a fresh instance as the
// worker has, takes the subjects of 537 traced whales (93,438 nodes) and builds the first
// painted preview at the defaults and at Optimise 50 % with inner areas; its linear memory stays
// at most 320 MB (306 MB measured). Needs `npm run build:wasm`; about a minute.
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { pathToFileURL } from "node:url";

const BINDINGS = path.resolve(import.meta.dirname, "..", "src", "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");
const WHALE = path.resolve(
  import.meta.dirname,
  "..",
  "..",
  "curvyo-geometry-core",
  "tests",
  "fixtures",
  "centre_lines",
  "whale_traced.txt",
);
const LIMIT_MB = 320;

type View = { kind: string; wire: Float64Array; free: () => void };
type Session = {
  import_begin: (origin: string, placed: boolean, x: number, y: number) => void;
  import_add: (bytes: Uint8Array, name: string) => void;
  import_finish: (force: boolean) => { free: () => void };
  set_tool: (tool: string) => void;
  centre_lines_open: () => View;
};
type Worker = {
  subjects: (wire: Float64Array, sink: { report: (percent: number) => void }) => boolean;
  finish: (overlap: number, thick: number, optimise: number, keepInner: boolean, offset: number, joinTips: boolean) => Float64Array;
};
type Bindings = {
  initSync: (options: { module: BufferSource }) => { memory: WebAssembly.Memory };
  WasmSession: new (peer: bigint) => Session;
  WasmCentreLinesWorker: new () => Worker;
};

/** A fresh instance of the built module (its own linear memory): the query makes Node load the
 * glue again. */
async function instance(tag: string): Promise<{ bindings: Bindings; memory: WebAssembly.Memory }> {
  const url = `${pathToFileURL(path.join(BINDINGS, "curvyo_editor_wasm.js")).href}?${tag}`;
  const bindings = (await import(url)) as Bindings;
  const { memory } = bindings.initSync({ module: readFileSync(WASM) });
  return { bindings, memory };
}

/** `copies` traced whales side by side as one SVG, one path each (as the native measurement in
 * `curvyo-editor-wasm/tests/centre_lines_cpu_time.rs`). */
function herd(copies: number): string {
  const subjects: [number, number, number, number, number, number][][][] = [];
  for (const line of readFileSync(WHALE, "utf8").split("\n")) {
    const words = line.trim().split(/\s+/);
    if (line.startsWith("#") || words[0] === "") {
      continue;
    }
    if (words[0] === "subject") {
      subjects.push([]);
    } else if (words[0] === "outline") {
      subjects[subjects.length - 1].push([]);
    } else {
      const outlines = subjects[subjects.length - 1];
      outlines[outlines.length - 1].push(words.map(Number) as [number, number, number, number, number, number]);
    }
  }
  const parts = ['<svg xmlns="http://www.w3.org/2000/svg" width="1700mm" height="6000mm" viewBox="0 0 1700 6000">'];
  for (let i = 0; i < copies; i += 1) {
    const [dx, dy] = [(i % 10) * 160, Math.floor(i / 10) * 100];
    for (const subject of subjects) {
      let d = "";
      for (const ring of subject) {
        d += `M${ring[0][0] + dx} ${ring[0][1] + dy}`;
        for (let k = 0; k < ring.length; k += 1) {
          const [px, py, , , ox, oy] = ring[k];
          const [qx, qy, hx, hy] = ring[(k + 1) % ring.length];
          d += `C${px + ox + dx} ${py + oy + dy} ${qx + hx + dx} ${qy + hy + dy} ${qx + dx} ${qy + dy}`;
        }
        d += "Z";
      }
      parts.push(`<path fill="black" d="${d}"/>`);
    }
  }
  parts.push("</svg>");
  return parts.join("");
}

test(
  "the centre lines worker stays within 320 MB at 93,438 nodes",
  { skip: !existsSync(WASM), timeout: 600_000 },
  async () => {
    const page = await instance("page");
    const session = new page.bindings.WasmSession(1n);
    session.import_begin("", false, 0, 0);
    session.import_add(new TextEncoder().encode(herd(537)), "herd.svg");
    session.import_finish(true).free();
    session.set_tool("node");
    const view = session.centre_lines_open();
    assert.equal(view.kind, "open");
    const wire = view.wire;
    view.free();

    const worker = await instance("worker");
    const centre = new worker.bindings.WasmCentreLinesWorker();
    assert.equal(centre.subjects(wire, { report: () => undefined }), true);
    const mb = () => worker.memory.buffer.byteLength / 1_000_000;
    const subjects = mb();
    // The defaults (NaN reads as the default), then Optimise 50 % with inner areas.
    const defaults = centre.finish(Number.NaN, Number.NaN, Number.NaN, false, Number.NaN, true);
    const first = mb();
    const inner = centre.finish(Number.NaN, Number.NaN, 0.5, true, Number.NaN, true);
    const second = mb();
    console.log(
      `worker memory: ${subjects.toFixed(0)} MB after the subjects, ${first.toFixed(0)} MB after the first preview ` +
        `(${((defaults.length * 8) / 1e6).toFixed(0)} MB), ${second.toFixed(0)} MB after Optimise 50 % with inner areas ` +
        `(${((inner.length * 8) / 1e6).toFixed(0)} MB)`,
    );
    assert.ok(second <= LIMIT_MB, `worker memory ${second.toFixed(0)} MB, limit ${LIMIT_MB} MB`);
  },
);
