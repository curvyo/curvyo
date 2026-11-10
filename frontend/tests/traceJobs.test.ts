// The trace worker runner (`frontend/src/lib/traceJobs.ts`, customer 2026-10-31 and 2026-11-10): a
// worker that never starts or fails to instantiate the module fails its jobs instead of leaving them
// waiting ("Analysing..." for good); a worker that starts answers as before; a worker that errored is
// never asked again, a slow start and an unanswered analysis are tried once more on a fresh worker,
// and a job stopped before its worker started is never posted.
import { strict as assert } from "node:assert";
import { test } from "node:test";

import { TraceCancelled, TraceRunner, TraceWorkerFailed, type TraceRequest, type TraceResponse, type TraceWorkerLike } from "../src/lib/traceJobs.ts";

/** A fake worker: `answer` maps each request (and the number of the worker, from 0) to the replies
 * it posts (none: it stays silent). */
function fakeWorker(answer: (request: TraceRequest, worker: number) => TraceResponse[]) {
  const made: { terminated: boolean; got: string[] }[] = [];
  const make = (): TraceWorkerLike => {
    const state = { terminated: false, got: [] as string[] };
    const index = made.length;
    made.push(state);
    const worker: TraceWorkerLike = {
      onmessage: null,
      onerror: null,
      postMessage(message: TraceRequest) {
        state.got.push(message.type);
        for (const reply of answer(message, index)) {
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

test("a worker that never answers init is replaced once; the second that does not either fails the job, and both are ended", async () => {
  const fake = fakeWorker(() => []);
  const runner = new TraceRunner(fake.make, module, 30);
  await assert.rejects(runner.defaults(pixels()), (e: unknown) => e instanceof TraceWorkerFailed && /no answer in 30 ms/.test(e.message));
  assert.equal(fake.made.length, 2);
  assert.ok(fake.made.every((w) => w.terminated));
  assert.deepEqual(fake.made.map((w) => w.got), [["init"], ["init"]], "a job waits for its worker's start");
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
  assert.equal(fake.made.length, 3, "a fresh worker");
  // After the start time nothing fails: the ready answer ended the timer.
  await new Promise((r) => setTimeout(r, 60));
  assert.equal(fake.made[2].terminated, false);
  assert.deepEqual(await runner.defaults(pixels()), { threshold: 120, background: "", analysis: "{}" });
  assert.equal(fake.made.length, 3);
  runner.dispose();
});

// Customer 2026-11-10 ("Analysis failed", Failed and Retry, Analyse doing nothing): the worker is
// recovered instead of asked again.
const ready: TraceResponse = { id: 0, kind: "ready", error: "" };
const answered = (r: TraceRequest): TraceResponse[] =>
  r.type === "threshold" ? [{ id: r.id, kind: "threshold", threshold: 99, background: "", analysis: "{}", ms: 1 }] : r.type === "trace" ? [{ id: r.id, kind: "trace", wire: new Float64Array(7), ms: 1 }] : [];

test("a slow start is tried once more: the waiting job runs on the second worker, and nothing but init went to the first", async () => {
  const fake = fakeWorker((r, w) => (r.type === "init" ? (w === 0 ? [] : [ready]) : answered(r)));
  const runner = new TraceRunner(fake.make, module, 30);
  const out = await runner.trace(pixels(), "{}");
  assert.equal(out.wire.length, 7);
  assert.deepEqual(fake.made.map((w) => [w.terminated, w.got]), [[true, ["init"]], [false, ["init", "trace"]]]);
  runner.dispose();
});

test("a worker that answered with an error is ended: the next job (Retry, Analyse) runs on a fresh one", async () => {
  const fake = fakeWorker((r, w) => (r.type === "init" ? [ready] : w === 0 ? [{ id: (r as { id: number }).id, kind: "error", message: "RuntimeError: unreachable" }] : answered(r)));
  const runner = new TraceRunner(fake.make, module, 5_000);
  await assert.rejects(runner.trace(pixels(), "{}"), /unreachable/);
  assert.equal(fake.made[0].terminated, true, "the worker that trapped is not asked again");
  assert.deepEqual(await runner.defaults(pixels()), { threshold: 99, background: "", analysis: "{}" });
  assert.equal(fake.made.length, 2);
  runner.dispose();
});

test("a worker error event ends the worker and fails its job; the next job gets a fresh worker", async () => {
  const fake = fakeWorker((r) => (r.type === "init" ? [ready] : answered(r)));
  const made: TraceWorkerLike[] = [];
  const runner = new TraceRunner(() => {
    const w = fake.make();
    made.push(w);
    return w;
  }, module, 5_000);
  await runner.defaults(pixels());
  const job = runner.defaults(pixels());
  await new Promise((r) => setTimeout(r, 0));
  made[0].onerror?.({ message: "out of memory" } as ErrorEvent);
  await assert.rejects(job, /out of memory/);
  assert.equal(fake.made[0].terminated, true);
  assert.deepEqual(await runner.defaults(pixels()), { threshold: 99, background: "", analysis: "{}" });
  assert.equal(fake.made.length, 2);
  runner.dispose();
});

test("a job stopped while its worker starts is never posted, so the next job does not queue behind it", async () => {
  let compiled: () => void = () => undefined;
  // One compile for every job, as `editorModule` caches it.
  const compiling = new Promise<WebAssembly.Module>((resolve) => (compiled = () => resolve({} as WebAssembly.Module)));
  const slowModule = () => compiling;
  const fake = fakeWorker((r) => (r.type === "init" ? [ready] : answered(r)));
  const runner = new TraceRunner(fake.make, slowModule, 5_000);
  const stale = runner.trace(pixels(), "{}");
  runner.cancel();
  const next = runner.defaults(pixels());
  compiled();
  await assert.rejects(stale, TraceCancelled);
  assert.deepEqual(await next, { threshold: 99, background: "", analysis: "{}" });
  assert.deepEqual(fake.made.map((w) => w.got), [["init", "threshold"]]);
  runner.dispose();
});

test("an analysis the worker does not answer in time is asked again of a fresh worker, once", async () => {
  const fake = fakeWorker((r, w) => (r.type === "init" ? [ready] : w === 0 ? [] : answered(r)));
  const runner = new TraceRunner(fake.make, module, 5_000, 1, 30);
  assert.deepEqual(await runner.defaults(pixels()), { threshold: 99, background: "", analysis: "{}" });
  assert.deepEqual(fake.made.map((w) => [w.terminated, w.got]), [[true, ["init", "threshold"]], [false, ["init", "threshold"]]]);
  // Two silent workers: the analysis fails, it does not wait for good.
  const silent = fakeWorker((r) => (r.type === "init" ? [ready] : []));
  const stuck = new TraceRunner(silent.make, module, 5_000, 1, 30);
  await assert.rejects(stuck.defaults(pixels()), (e: unknown) => e instanceof TraceWorkerFailed && /no answer in 30 ms/.test(e.message));
  assert.equal(silent.made.length, 2);
  assert.equal(stuck.busy, false);
  runner.dispose();
});

test("the analysis's answer time counts from when the worker has it, not from the worker's start", async () => {
  const made: TraceWorkerLike[] = [];
  const fake = fakeWorker((r) => (r.type === "threshold" ? answered(r) : []));
  const runner = new TraceRunner(() => {
    const w = fake.make();
    made.push(w);
    // The start takes longer than the answer time.
    setTimeout(() => w.onmessage?.({ data: ready } as MessageEvent<TraceResponse>), 60);
    return w;
  }, module, 5_000, 1, 30);
  assert.deepEqual(await runner.defaults(pixels()), { threshold: 99, background: "", analysis: "{}" });
  assert.equal(fake.made.length, 1, "no second worker");
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
