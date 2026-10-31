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
