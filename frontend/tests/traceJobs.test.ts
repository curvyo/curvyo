// The trace worker runner (`frontend/src/lib/traceJobs.ts`, customer 2026-10-31): a worker that
// never starts or fails to instantiate the module fails its jobs instead of leaving them waiting
// ("Analysing..." for good); a worker that starts answers as before.
import { strict as assert } from "node:assert";
import { test } from "node:test";

import { TraceRunner, TraceWorkerFailed, type TraceRequest, type TraceResponse, type TraceWorkerLike } from "../src/lib/traceJobs.ts";

/** A fake worker: `answer` maps each request to the replies it posts (none: it stays silent). */
function fakeWorker(answer: (request: TraceRequest) => TraceResponse[]) {
  const made: { terminated: boolean }[] = [];
  const make = (): TraceWorkerLike => {
    const state = { terminated: false };
    made.push(state);
    const worker: TraceWorkerLike = {
      onmessage: null,
      onerror: null,
      postMessage(message: TraceRequest) {
        for (const reply of answer(message)) {
          setTimeout(() => {
            if (!state.terminated) {
              worker.onmessage?.({ data: reply } as MessageEvent<TraceResponse>);
            }
          }, 1);
        }
      },
      terminate() {
        state.terminated = true;
      },
    };
    return worker;
  };
  return { make, made };
}

const module = async () => ({}) as WebAssembly.Module;
const pixels = () => ({ width: 1, height: 1, rgba: new Uint8Array(4), map: [1, 0, 0, 1, 0, 0] });

test("a worker that never answers init fails its job after the start time, and is ended", async () => {
  const fake = fakeWorker(() => []);
  const runner = new TraceRunner(fake.make, module, 30);
  await assert.rejects(runner.defaults(pixels()), (e: unknown) => e instanceof TraceWorkerFailed && /no answer in 30 ms/.test(e.message));
  assert.equal(fake.made[0].terminated, true);
  assert.equal(runner.busy, false);
});

test("a worker whose module cannot be instantiated fails its job with the reason", async () => {
  const fake = fakeWorker((r) => (r.type === "init" ? [{ id: 0, kind: "ready", error: "LinkError: import missing" }] : []));
  const runner = new TraceRunner(fake.make, module, 5_000);
  await assert.rejects(runner.defaults(pixels()), (e: unknown) => e instanceof TraceWorkerFailed && /LinkError/.test(e.message));
  runner.dispose();
});

test("a worker that starts answers its jobs, and the next job after a failed start gets a fresh worker", async () => {
  let silent = true;
  const fake = fakeWorker((r) => {
    if (r.type === "init") {
      return silent ? [] : [{ id: 0, kind: "ready", error: "" }];
    }
    return !silent && r.type === "threshold" ? [{ id: r.id, kind: "threshold", threshold: 120, background: "", analysis: "{}", ms: 1 }] : [];
  });
  const runner = new TraceRunner(fake.make, module, 30);
  await assert.rejects(runner.defaults(pixels()), TraceWorkerFailed);
  silent = false;
  assert.deepEqual(await runner.defaults(pixels()), { threshold: 120, background: "", analysis: "{}" });
  assert.equal(fake.made.length, 2, "a fresh worker");
  // After the start time nothing fails: the ready answer ended the timer.
  await new Promise((r) => setTimeout(r, 60));
  assert.equal(fake.made[1].terminated, false);
  assert.deepEqual(await runner.defaults(pixels()), { threshold: 120, background: "", analysis: "{}" });
  assert.equal(fake.made.length, 2);
  runner.dispose();
});

// `specs/0091` round 2: the text lines read by a pool of workers.
const weights = { detector: new Uint8Array(1), recogniser: new Uint8Array(1), alphabet: "a" };
const crops = { lines: [{ quad: [[0, 0]], width: 1, samples: 144 }, { quad: [[0, 1]], width: 2, samples: 288 }, { quad: [[0, 2]], width: 1, samples: 144 }] };

/** A worker that crops three lines and reads each as `L<first sample>`; `failing` reads fail. */
function ocrWorker(failing = false) {
  const reads: (TraceRequest & { type: "ocr-read" })[] = [];
  const fake = fakeWorker((r) => {
    if (r.type === "init") {
      return [{ id: 0, kind: "ready", error: "" }];
    }
    if (r.type === "ocr-crops") {
      const samples = new Float32Array(4 * 48 * 3);
      samples[0] = 1;
      samples[48 * 3] = 2;
      samples[3 * 48 * 3] = 3;
      return [{ id: r.id, kind: "ocr-crops", lines: JSON.stringify(crops), samples, error: "", rgba: r.rgba, ms: 1 }];
    }
    if (r.type === "ocr-read") {
      reads.push(r);
      const share = (JSON.parse(r.lines) as typeof crops).lines;
      let at = 0;
      const lines = share.map((c) => {
        const v = r.samples[at];
        at += c.width * 48 * 3;
        return v === 2 ? null : { text: `L${v}` };
      });
      return failing ? [{ id: r.id, kind: "ocr-read", reads: "", memory: 0, ms: 1 }] : [{ id: r.id, kind: "progress", percent: 100 }, { id: r.id, kind: "ocr-read", reads: JSON.stringify(lines), memory: 64, ms: 1 }];
    }
    return [];
  });
  return { fake, reads };
}

test("0091 pool: the lines read by two workers come back in line order, the weights sent once per worker", async () => {
  const { fake, reads } = ocrWorker();
  const runner = new TraceRunner(fake.make, module, 5_000, 2);
  const seen: number[] = [];
  const out = await runner.ocr(pixels(), weights, (p) => seen.push(p));
  assert.deepEqual(JSON.parse(out.lines), { height: 1, lines: [{ text: "L1" }, { text: "L3" }], width: 1 });
  assert.equal(fake.made.length, 3, "the trace worker and two readers");
  assert.ok(seen.every((p, i) => i === 0 || p >= seen[i - 1]) && seen.at(-1) === 100, `${seen}`);
  assert.deepEqual(runner.poolMemory, [64, 64]);
  await runner.ocr(pixels(), weights);
  assert.deepEqual(reads.map((r) => r.weights !== null), [true, true, false, false]);
  runner.dispose();
});

test("0091 pool: a reader that fails fails the read with no text", async () => {
  const { fake } = ocrWorker(true);
  const runner = new TraceRunner(fake.make, module, 5_000, 2);
  await assert.rejects(runner.ocr(pixels(), weights), /could not read its lines/);
  runner.dispose();
});

/** A worker whose fit takes a share, asks for one face once, and finishes each line as `F<text>`
 * with the nearest faces it was handed. */
function fitWorker(failFinish = false) {
  const finishes: string[] = [];
  const fake = fakeWorker((r) => {
    if (r.type === "init") {
      return [{ id: 0, kind: "ready", error: "" }];
    }
    if (r.type === "fit") {
      return [{ id: r.id, kind: "fit", reply: JSON.stringify({ kind: "need", faces: [["a.ttf", 0]] }), ms: 1 }];
    }
    if (r.type === "fit-faces") {
      return [{ id: r.id, kind: "progress", percent: 100 }, { id: r.id, kind: "fit", reply: JSON.stringify({ kind: "searched", nearest: { face: r.faces.length } }), ms: 1 }];
    }
    if (r.type === "fit-finish") {
      finishes.push(r.nearest);
      return [{ id: r.id, kind: "fit", reply: failFinish ? JSON.stringify({ kind: "need" }) : JSON.stringify({ kind: "finished", lines: ["x"] }), ms: 1 }];
    }
    return [];
  });
  return { fake, finishes };
}

test("0091 pool: the fit's lines are shared, every share finishes with all nearest faces, the lines come back in order", async () => {
  const { fake, finishes } = fitWorker();
  const runner = new TraceRunner(fake.make, module, 5_000, 2);
  const request = JSON.stringify({ lines: [{ text: "a", outlines: [1] }, { text: "b", outlines: [1, 2] }] });
  const asked: string[][] = [];
  const out = JSON.parse(await runner.fit(request, async (faces) => {
    asked.push(faces.map((f) => f[0]));
    return [];
  })) as { kind: string; lines: unknown[] };
  assert.equal(out.kind, "finished");
  // Two shares of one line each.
  assert.deepEqual(out.lines, ["x", "x"]);
  assert.deepEqual(finishes, ['[{"face":0},{"face":0}]', '[{"face":0},{"face":0}]']);
  assert.equal(asked.length, 2, "each worker asks for its faces");
  runner.dispose();
});

test("0091 pool: a fitting worker that cannot finish fails the fit", async () => {
  const { fake } = fitWorker(true);
  const runner = new TraceRunner(fake.make, module, 5_000, 2);
  const request = JSON.stringify({ lines: [{ text: "a", outlines: [1] }, { text: "b", outlines: [1] }] });
  await assert.rejects(runner.fit(request, async () => []), /could not finish/);
  runner.dispose();
});
