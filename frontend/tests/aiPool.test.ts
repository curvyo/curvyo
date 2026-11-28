// The AI tile pool (`specs/0143` criteria 3, 5 to 10, 14; ADR 0078 §3, §4) with fake workers and a
// fake clock: the pool size, the same tile set and picture for N = 1, 3 and 8 whatever the
// finishing order, a worker that fails to start or fails a tile is dropped and its tile handed out
// again, progress at most 10 times a second, and Cancel or the end of a Run leaves no extra worker.
import { test } from "node:test";
import assert from "node:assert/strict";

import { AiCancelled, AiRunner, IDLE_MS, type AiWorkerLike } from "../src/lib/aiJobs.ts";
import { MAX_POOL, type PoolRequest, type PoolResponse, type PoolWorker, poolSize, runPool } from "../src/lib/aiPool.ts";

const MiB = 2 ** 20;

test("criterion 3: N = min(cores - 1, 8, 1 GiB / M), at least 1", () => {
  assert.equal(poolSize(8, 4 * MiB), 7);
  assert.equal(poolSize(4, 4 * MiB), 3);
  assert.equal(poolSize(2, 4 * MiB), 1);
  assert.equal(poolSize(1, 4 * MiB), 1);
  assert.equal(poolSize(undefined, 4 * MiB), 1);
  assert.equal(poolSize(32, 4 * MiB), MAX_POOL);
  assert.equal(poolSize(32, 64 * MiB), 8, "a 16 Mpx picture: 1 GiB / 128 MiB");
  assert.equal(poolSize(32, 200 * MiB), 3, "the memory bound");
  assert.equal(poolSize(32, 4096 * MiB), 1);
});

/** A fake pool worker: a tile takes `delay(index)` ms; worker 0's assembly places the tiles by
 * index and refuses a doubled or missing one, as `assemble` does. */
class FakeWorker implements PoolWorker, AiWorkerLike {
  onmessage: ((event: MessageEvent<PoolResponse>) => void) | null = null;
  terminated = false;
  tiles: number[] = [];
  added = new Map<number, Uint8Array>();
  models: (Uint8Array | null)[] = [];
  readonly delay: (index: number) => number;
  readonly failTile: number | null;
  constructor(delay: (index: number) => number, failTile: number | null = null) {
    this.delay = delay;
    this.failTile = failTile;
  }
  private reply(message: PoolResponse | { kind: string }, ms = 0) {
    setTimeout(() => {
      if (!this.terminated) this.onmessage?.({ data: message } as MessageEvent<PoolResponse>);
    }, ms);
  }
  postMessage(message: PoolRequest | { type: string }): void {
    const m = message as PoolRequest | { type: "init" } | { type: "run" };
    if (m.type === "init") this.reply({ kind: "ready" });
    else if (m.type === "tiles") {
      this.models.push(m.model);
      this.reply({ kind: "tilesReady" });
    } else if (m.type === "tile") {
      if (m.index === this.failTile) {
        this.reply({ kind: "error", message: "out of memory" }, this.delay(m.index));
        return;
      }
      this.tiles.push(m.index);
      this.reply({ kind: "tile", index: m.index, width: 1, height: 1, pixels: new Uint8Array([m.index, 7]) }, this.delay(m.index));
    } else if (m.type === "add") {
      if (this.added.has(m.index)) throw new Error("doubled tile");
      this.added.set(m.index, m.pixels);
    } else if (m.type === "assemble") {
      const out: number[] = [];
      for (let i = 0; i < this.added.size; i++) {
        const tile = this.added.get(i);
        if (!tile) {
          this.reply({ kind: "error", message: "missing tile" });
          return;
        }
        out.push(...tile);
      }
      this.reply({ kind: "done", pixels: new Uint8Array(out) });
    }
  }
  terminate(): void {
    this.terminated = true;
  }
}

function input() {
  return { rgba: new Uint8Array(16), width: 2, height: 2, factor: 2 };
}

async function pool(n: number, delays: (worker: number, index: number) => number, opts: { failTile?: [number, number]; noStart?: number } = {}) {
  const workers = Array.from({ length: n }, (_, k) => new FakeWorker((i) => delays(k, i), opts.failTile?.[0] === k ? opts.failTile[1] : null));
  const reports: number[] = [];
  let reads = 0;
  const pixels = await runPool({
    modelKind: "upscale",
    input: input(),
    output: { width: 4, height: 4 },
    tiles: 25,
    first: workers[0],
    firstHasModel: false,
    extras: workers.slice(1).map((w, k) => Promise.resolve(opts.noStart === k + 1 ? null : w)),
    readModel: async () => {
      reads += 1;
      return new Uint8Array([9, 9]);
    },
    progress: (done) => reports.push(done),
    now: () => 0,
  });
  return { pixels, workers, reports, reads };
}

const expected = new Uint8Array(Array.from({ length: 25 }, (_, i) => [i, 7]).flat());

test("criterion 5: N = 1, 3 and 8 give the same tile set and picture whatever the finishing order", async () => {
  const one = await pool(1, () => 1);
  const three = await pool(3, (k, i) => ((i * 7 + k * 3) % 5) + 1);
  const eight = await pool(8, (k, i) => ((i * 13 + k * 5) % 9) + 1);
  for (const run of [one, three, eight]) {
    assert.deepEqual(run.pixels, expected);
    const computed = run.workers.flatMap((w) => w.tiles).sort((a, b) => a - b);
    assert.deepEqual(computed, Array.from({ length: 25 }, (_, i) => i), "each tile once");
    assert.equal(run.reads, 1, "the model file is read once");
  }
  assert.ok(three.workers.every((w) => w.tiles.length > 0), "every worker took tiles");
  assert.ok(eight.workers.slice(1).every((w) => w.terminated), "criterion 6: the extras end with the Run");
  assert.ok(!eight.workers[0].terminated, "worker 0 stays");
  assert.ok(eight.workers.every((w) => w.models.length === 1 && w.models[0]?.length === 2), "each worker got the model");
});

test("criteria 3, 14: a worker that fails to start or fails a tile is dropped and its tile runs again", async () => {
  const noStart = await pool(3, () => 1, { noStart: 2 });
  assert.deepEqual(noStart.pixels, expected);
  assert.equal(noStart.workers[2].tiles.length, 0);
  const failing = await pool(3, () => 1, { failTile: [1, 4] });
  assert.deepEqual(failing.pixels, expected);
  assert.ok(failing.workers[1].terminated, "the failing worker is dropped");
  assert.ok(failing.workers[0].tiles.includes(4) || failing.workers[2].tiles.includes(4), "its tile runs on another worker");
});

test("worker 0 failing fails the Run and ends the extras", async () => {
  // Tile 0 always goes to worker 0 (it is ready first); a later index could be taken by worker 1
  // on a loaded machine, and then nothing fails.
  const workers = [new FakeWorker(() => 1, 0), new FakeWorker(() => 5)];
  await assert.rejects(
    runPool({
      modelKind: "denoise",
      input: input(),
      output: { width: 2, height: 2 },
      tiles: 25,
      first: workers[0],
      firstHasModel: true,
      extras: [Promise.resolve(workers[1])],
      readModel: async () => new Uint8Array(1),
      progress: () => undefined,
      now: () => 0,
    }),
    /out of memory/,
  );
  assert.ok(workers[1].terminated);
  assert.deepEqual(workers[0].models, [null], "worker 0 holding the model gets no file");
});

test("criterion 7: progress at most every 100 ms, the last tile always", async () => {
  let clock = 0;
  const reports: [number, number][] = [];
  const w = new FakeWorker(() => 0);
  await runPool({
    modelKind: "denoise",
    input: input(),
    output: { width: 2, height: 2 },
    tiles: 25,
    first: w,
    firstHasModel: true,
    extras: [],
    readModel: async () => new Uint8Array(1),
    progress: (done, total) => {
      reports.push([done, total]);
    },
    now: () => (clock += 30),
  });
  assert.deepEqual(reports[0], [0, 25]);
  assert.deepEqual(reports[reports.length - 1], [25, 25]);
  const elapsed = 30 * 26;
  assert.ok(reports.length <= Math.ceil(elapsed / 100) + 2, `${reports.length} reports in ${elapsed} ms`);
});

/** A runner over fake workers on a machine of `cores`. */
function runner(cores: number, delay = 2) {
  const spawned: FakeWorker[] = [];
  let idle: (() => void) | null = null;
  const r = new AiRunner(
    () => {
      const w = new FakeWorker(() => delay);
      spawned.push(w);
      return w;
    },
    async () => ({}) as WebAssembly.Module,
    {
      cores: () => cores,
      tileCount: () => 25,
      now: () => 0,
      later: (run, ms) => {
        assert.equal(ms, IDLE_MS);
        idle = run;
        return () => {
          idle = null;
        };
      },
    },
  );
  return { r, spawned, fireIdle: () => idle?.() };
}

test("criteria 6, 10: a Run on 8 cores uses 7 workers, the extras end with it, worker 0 after the idle time; 2 cores say one core", async () => {
  const { r, spawned, fireIdle } = runner(8);
  let workers = 0;
  const out = await r.run("upscale", null, input(), () => undefined, { readModel: async () => new Uint8Array(2), workers: (n) => (workers = n) });
  assert.equal(workers, 7);
  assert.equal(spawned.length, 7);
  assert.deepEqual(out.pixels, expected);
  assert.equal(r.alive, 1, "only worker 0 is left");
  assert.ok(spawned.slice(1).every((w) => w.terminated));
  fireIdle();
  assert.equal(r.alive, 0, "worker 0 ends after the idle time");

  const small = runner(2);
  let n = 0;
  await small.r.run("denoise", new Uint8Array(1), input(), () => undefined, { readModel: async () => new Uint8Array(1), workers: (k) => (n = k) });
  assert.equal(n, 1, "one worker: Running on one core");
  assert.equal(small.spawned.length, 1);
});

test("criteria 8, 9: Cancel during a Run terminates every worker", async () => {
  const { r, spawned } = runner(8, 50);
  const run = r.run("upscale", null, input(), () => undefined, { readModel: async () => new Uint8Array(2) }).catch((e: unknown) => e);
  await new Promise((done) => setTimeout(done, 20));
  assert.ok(r.alive > 1, "the pool is running");
  r.cancel();
  assert.equal(r.alive, 0);
  assert.ok(spawned.every((w) => w.terminated));
  assert.ok((await run) instanceof Error);
});

test("architect review: Cancel settles the pending Run, tiled or whole, so its input is freed", async () => {
  const { r } = runner(8, 50);
  const tiled = r.run("upscale", null, input(), () => undefined, { readModel: async () => new Uint8Array(2) });
  await new Promise((done) => setTimeout(done, 20));
  r.cancel();
  await assert.rejects(tiled, AiCancelled);
  const whole = runner(8);
  const run = whole.r.run("background", new Uint8Array(1), input(), () => undefined);
  await new Promise((done) => setTimeout(done, 5));
  whole.r.cancel();
  await assert.rejects(run, AiCancelled);
});
