// Tester acceptance cases for `specs/0143-ai-tile-pool` on the page's side, written from the
// specification and ADR 0078 §3, §4 with independent fake workers that follow the worker
// protocol (init, tiles, tile, add, assemble): the pool size formula (criterion 3), the same
// picture for N = 1 to 8 and any finishing order (criterion 5), a worker that fails to start,
// fails loading the model or fails a tile is dropped and its tile runs again (criterion 3),
// progress at most 10 times a second (criterion 7), Cancel terminates every worker at once and
// nothing is posted afterwards (criteria 8, 9), the pool exists only during a tiled Run
// (criteria 3, 6), and a Run left with one worker says so (criterion 10).
import { test } from "node:test";
import assert from "node:assert/strict";

import { AiRunner, type AiWorkerLike } from "../src/lib/aiJobs.ts";
import {
  MAX_POOL,
  POOL_MEMORY,
  WORKER_MEMORY,
  type PoolRequest,
  type PoolResponse,
  type PoolWorker,
  poolSize,
  runPool,
} from "../src/lib/aiPool.ts";

const MiB = 2 ** 20;

/** A small seeded random source. */
function rng(seed: number): () => number {
  let s = seed >>> 0 || 1;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return (s >>> 0) / 2 ** 32;
  };
}

type Behaviour = {
  /** Delay of a tile in ms. */
  delay?: (index: number) => number;
  /** Fail the n-th tile this worker gets (1-based). */
  failNth?: number;
  /** Refuse the model (memory) on `tiles`. */
  failTiles?: boolean;
  /** Fail `init` (the worker does not start). */
  failInit?: boolean;
};

/** A fake AI worker: a tile's pixels depend on the input and the index only; worker 0's assembly
 * places tiles by index and refuses a missing or doubled one. Records every message. */
class Fake implements PoolWorker, AiWorkerLike {
  onmessage: ((event: MessageEvent<PoolResponse>) => void) | null = null;
  terminated = false;
  log: string[] = [];
  computed: number[] = [];
  postedAfterTerminate = 0;
  private input: Uint8Array | null = null;
  private model: Uint8Array | null | undefined = undefined;
  private got = 0;
  private added = new Map<number, Uint8Array>();
  readonly b: Behaviour;
  constructor(b: Behaviour = {}) {
    this.b = b;
  }
  private send(m: PoolResponse | { kind: string }, ms = 0) {
    setTimeout(() => {
      if (!this.terminated) this.onmessage?.({ data: m } as MessageEvent<PoolResponse>);
    }, ms);
  }
  postMessage(message: PoolRequest | { type: string }): void {
    if (this.terminated) this.postedAfterTerminate += 1;
    const m = message as PoolRequest & { type: string; module?: unknown; rgba?: Uint8Array; model?: Uint8Array | null };
    this.log.push(m.type);
    switch (m.type) {
      case "init":
        this.send(this.b.failInit ? { kind: "error", message: "no memory" } : { kind: "ready" });
        return;
      case "tiles":
        if (this.b.failTiles) {
          this.send({ kind: "error", message: "memory refused" });
          return;
        }
        this.input = new Uint8Array(m.rgba as Uint8Array);
        this.model = m.model;
        this.send({ kind: "tilesReady" }, 1);
        return;
      case "tile": {
        const index = (m as { index: number }).index;
        this.got += 1;
        const ms = this.b.delay?.(index) ?? 1;
        if (!this.input) {
          this.send({ kind: "error", message: "no input picture" }, ms);
          return;
        }
        if (this.b.failNth === this.got) {
          this.send({ kind: "error", message: "tile failed" }, ms);
          return;
        }
        let sum = 0;
        for (const v of this.input) sum = (sum + v) % 251;
        this.computed.push(index);
        this.send({ kind: "tile", index, width: 1, height: 1, pixels: new Uint8Array([index, sum, 0x5a]) }, ms);
        return;
      }
      case "add": {
        const a = m as { index: number; pixels: Uint8Array };
        if (this.added.has(a.index)) this.added.set(-1, new Uint8Array());
        this.added.set(a.index, new Uint8Array(a.pixels));
        return;
      }
      case "assemble": {
        const out: number[] = [];
        const n = [...this.added.keys()].filter((k) => k >= 0).length;
        if (this.added.has(-1)) {
          this.send({ kind: "error", message: "doubled tile" });
          return;
        }
        for (let i = 0; i < n; i++) out.push(...(this.added.get(i) ?? []));
        if (out.length !== n * 3) {
          this.send({ kind: "error", message: "missing tile" });
          return;
        }
        this.added.clear();
        this.send({ kind: "done", pixels: new Uint8Array(out) });
        return;
      }
      default:
        this.send({ kind: "error", message: `unknown ${m.type}` });
    }
  }
  terminate(): void {
    this.terminated = true;
  }
}

const INPUT = () => ({ rgba: new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]), width: 2, height: 2, factor: 2 });
const TILES = 25;
const SUM = (1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15 + 16) % 251;
const EXPECTED = new Uint8Array(Array.from({ length: TILES }, (_, i) => [i, SUM, 0x5a]).flat());

/** Starts a fake worker the way the runner does before handing it to the pool. */
async function started(b: Behaviour): Promise<Fake | null> {
  const w = new Fake(b);
  const ok = await new Promise<boolean>((resolve) => {
    w.onmessage = (e) => resolve(e.data.kind === ("ready" as string));
    w.postMessage({ type: "init" } as unknown as PoolRequest);
  });
  if (!ok) w.terminate();
  return ok ? w : null;
}

async function runOn(behaviours: Behaviour[], tiles = TILES, now = () => 0, progress: (d: number, t: number) => void = () => undefined) {
  const first = new Fake(behaviours[0]);
  const extras = behaviours.slice(1).map((b) => started(b));
  const all = await Promise.all(extras.map((p) => p.then((w) => w)));
  const pixels = await runPool({
    modelKind: "upscale",
    input: INPUT(),
    output: { width: 4, height: 4 },
    tiles,
    first,
    firstHasModel: false,
    extras,
    readModel: async () => new Uint8Array([42, 42, 42]),
    progress,
    now,
  });
  return { pixels, first, extras: all };
}

test("criterion 3: N = min(C - 1, 8, floor(1 GiB / M)), at least 1", () => {
  const small = 1000 * 1000 * 4;
  const memoryBound = Math.floor(POOL_MEMORY / (WORKER_MEMORY + small));
  for (const [cores, want] of [
    [1, 1],
    [2, 1],
    [3, 2],
    [4, 3],
    [6, 5],
    [8, 7],
    [9, 8],
    [16, 8],
    [128, 8],
  ] as const) {
    assert.equal(poolSize(cores, small), Math.max(1, Math.min(cores - 1, MAX_POOL, memoryBound)), `${cores} cores`);
    assert.equal(poolSize(cores, small), want, `${cores} cores`);
  }
  assert.equal(MAX_POOL, 8);
  assert.equal(POOL_MEMORY, 2 ** 30, "1 GiB");
  for (const odd of [undefined, 0, -3, Number.NaN]) assert.equal(poolSize(odd, small), 1, `cores ${odd}`);
  // The memory term binds for big inputs and never goes below one.
  const big = 300 * MiB;
  assert.equal(poolSize(64, big), Math.max(1, Math.floor(POOL_MEMORY / (WORKER_MEMORY + big))));
  assert.ok(poolSize(64, big) < 8);
  assert.equal(poolSize(64, 2 * POOL_MEMORY), 1);
  for (let cores = 1; cores <= 64; cores++) {
    for (const bytes of [0, small, 64 * MiB, 500 * MiB]) {
      const n = poolSize(cores, bytes);
      assert.ok(Number.isInteger(n) && n >= 1 && n <= 8 && n <= Math.max(1, cores - 1), `${cores} ${bytes} -> ${n}`);
    }
  }
});

test("criterion 5: N = 1, 2, 3, 5, 8 and random finishing orders give the same picture, each tile once", async () => {
  for (const n of [1, 2, 3, 5, 8]) {
    for (const seed of [1, 7, 99]) {
      const r = rng(seed * 31 + n);
      const delays = Array.from({ length: n }, () => Array.from({ length: TILES }, () => Math.floor(r() * 8)));
      const run = await runOn(Array.from({ length: n }, (_, k) => ({ delay: (i: number) => delays[k][i] })));
      assert.deepEqual(run.pixels, EXPECTED, `N = ${n} seed ${seed}`);
      const workers = [run.first, ...run.extras.filter((w): w is Fake => w !== null)];
      const computed = workers.flatMap((w) => w.computed).sort((a, b) => a - b);
      assert.deepEqual(computed, Array.from({ length: TILES }, (_, i) => i), `N = ${n}: each tile computed once`);
      // Criterion 6: the extra workers end with the Run, worker 0 stays.
      assert.ok(!run.first.terminated, "worker 0 stays");
      assert.ok(workers.slice(1).every((w) => w.terminated), `N = ${n}: every extra worker terminated`);
      for (const w of workers) {
        const tiles = w.log.indexOf("tiles");
        const tile = w.log.indexOf("tile");
        if (tile >= 0) assert.ok(tiles >= 0 && tiles < tile, "the picture is posted before the first tile");
        assert.equal(w.log.filter((t) => t === "tiles").length, 1, "the picture is posted once per Run");
        assert.equal(w.postedAfterTerminate, 0);
      }
    }
  }
});

test("criterion 3: a worker that fails a tile, fails loading or fails to start is dropped and the tile runs again", async () => {
  const cases: [string, Behaviour[]][] = [
    ["an extra fails its 2nd tile", [{}, { failNth: 2 }, {}]],
    ["two extras fail their 1st tile", [{}, { failNth: 1 }, { failNth: 1, delay: () => 3 }, {}]],
    ["an extra refuses the model", [{}, { failTiles: true }, {}]],
    ["an extra does not start", [{}, { failInit: true }, {}]],
    ["every extra fails", [{}, { failNth: 1 }, { failTiles: true }, { failInit: true }]],
  ];
  for (const [name, behaviours] of cases) {
    const run = await runOn(behaviours);
    assert.deepEqual(run.pixels, EXPECTED, name);
    const workers = [run.first, ...run.extras.filter((w): w is Fake => w !== null)];
    const computed = workers.flatMap((w) => w.computed).sort((a, b) => a - b);
    assert.deepEqual(computed, Array.from({ length: TILES }, (_, i) => i), `${name}: every tile once in the result`);
    for (const [k, b] of behaviours.entries()) {
      const w = run.extras[k - 1];
      if (k > 0 && (b.failNth || b.failTiles) && w) assert.ok(w.terminated, `${name}: worker ${k} dropped`);
    }
  }
});

test("criterion 7: progress at most 10 times a second, monotonic, ending at all tiles", async () => {
  let clock = 0;
  const reports: { t: number; done: number; total: number }[] = [];
  // 100 tiles on 8 workers, a tile every 4 ms of the fake clock: 400 ms of work.
  await runOn(
    Array.from({ length: 8 }, () => ({ delay: () => 0 })),
    100,
    () => (clock += 4),
    (done, total) => reports.push({ t: clock, done, total }),
  );
  const last = reports[reports.length - 1];
  assert.deepEqual([last.done, last.total], [100, 100]);
  for (let i = 1; i < reports.length; i++) assert.ok(reports[i].done >= reports[i - 1].done, "monotonic");
  const notLast = reports.slice(0, -1);
  for (let i = 1; i < notLast.length; i++) {
    assert.ok(notLast[i].t - notLast[i - 1].t >= 100, `reports ${i - 1} and ${i} only ${notLast[i].t - notLast[i - 1].t} ms apart`);
  }
  assert.ok(reports.length <= Math.floor(clock / 100) + 2, `${reports.length} reports in ${clock} ms`);
});

/** A runner over fake workers on a machine of `cores`; `behave(k)` gives the k-th spawned worker. */
function runner(cores: number | undefined, behave: (k: number) => Behaviour = () => ({ delay: () => 2 })) {
  const spawned: Fake[] = [];
  const r = new AiRunner(
    () => {
      const w = new Fake(behave(spawned.length));
      spawned.push(w);
      return w;
    },
    async () => ({}) as WebAssembly.Module,
    { cores: () => cores, tileCount: () => TILES, now: () => 0, later: () => () => undefined },
  );
  return { r, spawned };
}

function tiledRun(r: AiRunner, workers: number[] = []) {
  return r.run("upscale", new Uint8Array([1]), INPUT(), () => undefined, {
    readModel: async () => new Uint8Array([1]),
    workers: (n) => workers.push(n),
  });
}

test("criteria 3, 6: no worker before a Run; Remove background uses one; a tiled Run builds the pool and ends it", async () => {
  const { r, spawned } = runner(8);
  assert.equal(spawned.length, 0, "no worker at app start");
  assert.equal(r.alive, 0);
  const bg = await r.run("background", new Uint8Array([1]), INPUT(), () => undefined, { readModel: async () => new Uint8Array([1]) }).catch((e: unknown) => e);
  // The fake answers `run` with an error; what counts here is that only worker 0 was spawned.
  void bg;
  assert.equal(spawned.length, 1, "Remove background is not tiled: one worker");
  const workers: number[] = [];
  const out = await tiledRun(r, workers);
  assert.deepEqual(out.pixels, EXPECTED);
  assert.deepEqual(workers, [poolSize(8, INPUT().rgba.byteLength)]);
  assert.equal(spawned.length, 1 + 6, "worker 0 plus 6 extras on 8 cores");
  assert.equal(r.alive, 1, "only worker 0 stays after the Run");
  assert.ok(spawned.slice(1).every((w) => w.terminated));
  // The next Run builds the pool again.
  await tiledRun(r);
  assert.equal(spawned.length, 1 + 6 + 6);
  assert.equal(r.alive, 1);
});

test("criteria 8, 9: Cancel terminates every worker at once, nothing is posted afterwards, the next Run starts fresh", async () => {
  const { r, spawned } = runner(8, () => ({ delay: () => 50 }));
  const run = tiledRun(r);
  let resolved = false;
  void run.then(
    () => (resolved = true),
    () => undefined,
  );
  await new Promise((res) => setTimeout(res, 80));
  assert.ok(spawned.length > 1, "the pool is running");
  const t0 = performance.now();
  r.cancel();
  const took = performance.now() - t0;
  assert.ok(took < 1000, `Cancel took ${took} ms`);
  assert.equal(r.alive, 0, "no worker stays behind");
  assert.ok(spawned.every((w) => w.terminated), "every pool worker terminated");
  const count = spawned.length;
  await new Promise((res) => setTimeout(res, 300));
  assert.ok(spawned.every((w) => w.postedAfterTerminate === 0), "nothing posted to a terminated worker");
  assert.equal(spawned.length, count, "no worker spawned after Cancel");
  assert.equal(resolved, false, "no result after Cancel");
  const again = await tiledRun(r);
  assert.deepEqual(again.pixels, EXPECTED);
  assert.ok(!spawned[count].terminated, "a fresh worker 0");
});

test("criteria 8, 9: Cancel while the extras are still starting leaves no worker behind", async () => {
  let release: () => void = () => undefined;
  const gate = new Promise<void>((res) => (release = res));
  const spawned: Fake[] = [];
  let first = true;
  const r = new AiRunner(
    () => {
      const w = new Fake({ delay: () => 5 });
      spawned.push(w);
      return w;
    },
    async () => {
      if (first) {
        first = false;
        return {} as WebAssembly.Module;
      }
      await gate;
      return {} as WebAssembly.Module;
    },
    { cores: () => 8, tileCount: () => TILES, now: () => 0, later: () => () => undefined },
  );
  void tiledRun(r).catch(() => undefined);
  await new Promise((res) => setTimeout(res, 20));
  r.cancel();
  release();
  await new Promise((res) => setTimeout(res, 50));
  assert.ok(spawned.length > 1);
  assert.ok(spawned.every((w) => w.terminated), "every worker, also the late ones, is terminated");
  assert.equal(r.alive, 0);
});

test("criterion 10: two cores or no core count is a pool of one, reported once", async () => {
  for (const cores of [1, 2, undefined]) {
    const { r, spawned } = runner(cores);
    const workers: number[] = [];
    const out = await tiledRun(r, workers);
    assert.deepEqual(out.pixels, EXPECTED);
    assert.deepEqual(workers, [1], `cores ${cores}: one report of a pool of one`);
    assert.equal(spawned.length, 1, "only worker 0");
  }
});

test("criterion 10: when every extra worker fails to start, the Run says it runs on one core", async () => {
  const { r, spawned } = runner(8, (k) => (k === 0 ? { delay: () => 2 } : { failInit: true }));
  const workers: number[] = [];
  const out = await tiledRun(r, workers);
  assert.deepEqual(out.pixels, EXPECTED, "the Run finishes on worker 0");
  assert.ok(spawned.slice(1).every((w) => w.terminated), "the failed workers are dropped");
  assert.equal(workers.at(-1), 1, `the pool ended up as one worker; the Run reported ${JSON.stringify(workers)}`);
});
