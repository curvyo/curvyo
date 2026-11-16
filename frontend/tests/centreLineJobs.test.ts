// The Find centre lines worker jobs (`frontend/src/lib/centreLineJobs.ts`, `specs/0117` criterion
// 15b, ADR 0063 amendment 2026-11-18, W): one job in flight and one pending slot that newer
// requests overwrite; stale replies are dropped; a drag never terminates the worker; Apply waits
// for the newest preview and then asks for its result; close ends the worker; a worker silent
// for the stage timeout is ended as failed and the next request starts a fresh one.
import { strict as assert } from "node:assert";
import { test } from "node:test";

import { CentreLineJobs, type CentreOptionsWire } from "../src/lib/centreLineJobs.ts";
import type { TraceRequest, TraceResponse, TraceWorkerLike } from "../src/lib/traceJobs.ts";

/** A worker the test answers by hand: `got` lists what it was sent. */
function handWorker() {
  const made: { terminated: boolean; got: TraceRequest[]; worker: TraceWorkerLike }[] = [];
  const make = (): TraceWorkerLike => {
    const state = { terminated: false, got: [] as TraceRequest[], worker: null as unknown as TraceWorkerLike };
    const worker: TraceWorkerLike = {
      onmessage: null,
      onerror: null,
      postMessage(message: TraceRequest) {
        state.got.push(message);
        if (message.type === "init") {
          setTimeout(() => worker.onmessage?.({ data: { id: 0, kind: "ready", error: "" } } as MessageEvent<TraceResponse>), 0);
        }
      },
      terminate() {
        state.terminated = true;
      },
    };
    state.worker = worker;
    made.push(state);
    return worker;
  };
  const reply = (index: number, data: TraceResponse) => made[index].worker.onmessage?.({ data } as MessageEvent<TraceResponse>);
  return { make, made, reply };
}

const tick = () => new Promise((resolve) => setTimeout(resolve, 5));
const options = (overlap: number): CentreOptionsWire => [overlap, 6, 0.75, 0, 0.3];
const module = () => Promise.resolve({} as WebAssembly.Module);

function jobs(timeoutMs = 500) {
  const hand = handWorker();
  const previews: number[] = [];
  const failures: string[] = [];
  const runner = new CentreLineJobs(
    hand.make,
    module,
    {
      preview: (gen) => previews.push(gen),
      progress: () => undefined,
      failed: (message) => failures.push(message),
    },
    timeoutMs,
  );
  return { hand, previews, failures, runner };
}

const kinds = (requests: TraceRequest[]) => requests.map((r) => `${r.type}${"id" in r ? `:${r.id}` : ""}`);

test("a drag sends one job at a time, overwrites the pending one and drops stale replies", async () => {
  const { hand, previews, runner } = jobs();
  runner.request(options(0.2), new Float64Array([1, 0]));
  runner.request(options(0.1));
  runner.request(options(0.0));
  await tick();
  assert.deepEqual(kinds(hand.made[0].got), ["init", "centre-subjects:1", "centre-finish:1"]);
  assert.equal(runner.busy, true);
  hand.reply(0, { id: 1, kind: "centre-preview", wire: new Float64Array(0), ms: 1 });
  await tick();
  // Generation 1 is stale and dropped; 2 was overwritten by 3, which goes now.
  assert.deepEqual(previews, []);
  assert.deepEqual(kinds(hand.made[0].got).slice(3), ["centre-finish:3"]);
  hand.reply(0, { id: 3, kind: "centre-preview", wire: new Float64Array(0), ms: 1 });
  assert.deepEqual(previews, [3]);
  assert.equal(runner.busy, false);
  assert.equal(hand.made.length, 1);
  assert.equal(hand.made[0].terminated, false, "a drag never terminates the worker");
});

test("Apply waits for the newest preview, then asks for its result", async () => {
  const { hand, runner } = jobs();
  runner.request(options(0.2), new Float64Array([1, 0]));
  await tick();
  const result = runner.result();
  await tick();
  assert.equal(kinds(hand.made[0].got).includes("centre-result:1"), false);
  hand.reply(0, { id: 1, kind: "centre-preview", wire: new Float64Array(0), ms: 1 });
  await tick();
  assert.deepEqual(kinds(hand.made[0].got).slice(-1), ["centre-result:1"]);
  hand.reply(0, { id: 1, kind: "centre-result", wire: new Float64Array([7]) });
  assert.deepEqual([...((await result) ?? [])], [7]);
});

test("close ends the worker and a waiting Apply hears nothing to write", async () => {
  const { hand, runner } = jobs();
  runner.request(options(0.2), new Float64Array([1, 0]));
  await tick();
  const result = runner.result();
  runner.close();
  assert.equal(await result, null);
  assert.equal(hand.made[0].terminated, true);
  // The next request starts a fresh worker with the subjects again.
  runner.request(options(0.1), new Float64Array([1, 0]));
  await tick();
  assert.deepEqual(kinds(hand.made[1].got), ["init", "centre-subjects:2", "centre-finish:2"]);
});

test("a failed worker is ended and its failure is heard", async () => {
  const { hand, failures, runner } = jobs();
  runner.request(options(0.2), new Float64Array([1, 0]));
  await tick();
  hand.reply(0, { id: 1, kind: "error", message: "trap" });
  assert.deepEqual(failures, ["trap"]);
  assert.equal(hand.made[0].terminated, true);
});

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

test("a worker silent for the stage timeout is ended as failed, and Apply hears nothing to write", async () => {
  const { hand, failures, runner } = jobs(400);
  runner.request(options(0.2), new Float64Array([1, 0]));
  await tick();
  const result = runner.result();
  // The kernel hangs: no progress, no preview.
  await wait(1_000);
  assert.deepEqual(failures, ["centre lines worker timed out"]);
  assert.equal(hand.made[0].terminated, true);
  assert.equal(await result, null);
  assert.equal(runner.busy, false, "the preview is not left busy");
  // The next request starts a fresh worker with its subjects.
  runner.request(options(0.1), new Float64Array([1, 0]));
  await tick();
  assert.deepEqual(kinds(hand.made[1].got), ["init", "centre-subjects:2", "centre-finish:2"]);
  hand.reply(1, { id: 2, kind: "centre-preview", wire: new Float64Array(0), ms: 1 });
  await wait(1_000);
  assert.equal(failures.length, 1, "an answered job and the old worker's timer fail nothing");
});

test("progress keeps a long build alive, and a worker that never answers init times out", async () => {
  const { hand, failures, previews, runner } = jobs(400);
  runner.request(options(0.2), new Float64Array([1, 0]));
  for (let percent = 10; percent <= 60; percent += 10) {
    // Together longer than the timeout, each step well within it.
    await wait(100);
    hand.reply(0, { id: 1, kind: "progress", percent });
  }
  hand.reply(0, { id: 1, kind: "centre-preview", wire: new Float64Array(0), ms: 1 });
  assert.deepEqual(previews, [1]);
  assert.deepEqual(failures, []);
  await wait(1_000);
  assert.deepEqual(failures, [], "an idle worker owes nothing");

  const silent = new CentreLineJobs(
    () => ({ onmessage: null, onerror: null, postMessage() {}, terminate() {} }),
    module,
    { preview: () => undefined, progress: () => undefined, failed: (m) => failures.push(m) },
    400,
  );
  silent.request(options(0.2), new Float64Array([1, 0]));
  await wait(1_000);
  assert.deepEqual(failures, ["centre lines worker timed out"]);
});
