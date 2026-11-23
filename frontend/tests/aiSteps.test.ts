// The Run flow of an AI step (`specs/0135` criteria 47, 63 to 65; ADR 0076 §6): no request before
// the consent line, a partial file counts as consent, Download then Run stores the result, and
// Cancel keeps a partial download. Fake session, store, host and worker.
import { test } from "node:test";
import assert from "node:assert/strict";

import { AiSteps, type AiRun, type AiSession } from "../src/lib/aiSteps.ts";
import type { PinnedModel } from "../src/lib/modelDownload.ts";
import { memoryModelStore } from "../src/lib/modelStore.ts";

const FILE = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8]);
const MODEL: PinnedModel = {
  kind: "background",
  name: "u2netp",
  version: "1",
  bytes: FILE.length,
  sha256: "cd".repeat(32),
  url: "https://models.curvyo.org/u2netp/1/x.onnx",
  licence_url: "",
  licence: "Apache-2.0",
  copyright: "",
  upstream: "",
  training_data: "",
  placeholder: false,
};

function fakeSession() {
  const calls: string[] = [];
  const session: AiSession = {
    set_ai_model_state: (kind, state) => {
      calls.push(`state ${kind} ${state}`);
      return true;
    },
    ai_run_input_json: (row) => {
      calls.push(`input ${row}`);
      return JSON.stringify({ kind: "background", factor: 1, width: 2, height: 1, hash: "ef".repeat(32) });
    },
    take_ai_run_pixels: () => new Uint8Array(8),
    set_ai_progress: () => undefined,
    end_ai_run: () => calls.push("end"),
    store_ai_result: (row, hash, width, height, pixels, grey) => {
      calls.push(`store ${row} ${hash.slice(0, 2)} ${width}x${height} ${pixels.length} ${grey}`);
      return "";
    },
    remove_ai_result: () => true,
  };
  return { session, calls };
}

function fakeRunner(): AiRun & { models: (Uint8Array | null)[] } {
  const models: (Uint8Array | null)[] = [];
  return {
    models,
    hasModel: () => false,
    run: async (_kind, model, _input, progress) => {
      models.push(model);
      progress(100);
      return { pixels: new Uint8Array(320 * 320), width: 320, height: 320, grey: true };
    },
    cancel: () => undefined,
  };
}

function flow(fetchFn: (url: string, init: RequestInit) => Promise<Response>) {
  const { session, calls } = fakeSession();
  const runner = fakeRunner();
  const store = memoryModelStore();
  const steps = new AiSteps({
    session: () => session,
    models: [MODEL],
    store,
    fetch: fetchFn,
    verify: (_kind, bytes) => bytes.length === FILE.length,
    runner,
    storage: undefined,
    changed: () => undefined,
  });
  return { steps, calls, runner, store };
}

test("criterion 63: Run of a model not on the device asks first and fetches nothing", async () => {
  let fetched = 0;
  const { steps, calls } = flow(async () => {
    fetched += 1;
    return new Response(FILE, { status: 200 });
  });
  await steps.run(0, "background");
  assert.equal(fetched, 0);
  assert.ok(calls.includes("state background asking"));
  steps.dismiss("background");
  assert.ok(calls.includes("state background absent"));
});

test("criteria 47, 63, 64: Download on the consent line, then the Run stores its result", async () => {
  const { steps, calls, runner, store } = flow(async () => new Response(FILE, { status: 200 }));
  await steps.consent(0, "background");
  assert.deepEqual(await store.status(MODEL.sha256), { state: "complete", bytes: FILE.length });
  assert.ok(calls.includes("state background ready"));
  assert.ok(calls.includes(`store 0 ef 320x320 ${320 * 320} true`));
  assert.deepEqual([...(runner.models[0] ?? [])], [...FILE], "the worker got the cached file");
});

test("ADR 0076 §6: a partial file counts as consent, so Run resumes without asking", async () => {
  const { steps, calls, store } = flow(async (_url, init) => {
    const range = (init.headers as Record<string, string> | undefined)?.Range;
    assert.equal(range, "bytes=3-");
    return new Response(FILE.slice(3), { status: 206, headers: { "Content-Range": "bytes 3-7/8" } });
  });
  await store.append(MODEL.sha256, 0, FILE.slice(0, 3));
  await steps.run(1, "background");
  assert.ok(!calls.includes("state background asking"));
  assert.ok(calls.some((c) => c.startsWith("store 1")));
});

test("criterion 65: the host not reachable leaves the step able to try again, nothing stored", async () => {
  const { steps, calls } = flow(async () => {
    throw new TypeError("offline");
  });
  await steps.consent(0, "background");
  assert.ok(calls.includes("state background unreachable"));
  assert.ok(!calls.some((c) => c.startsWith("store")));
});

test("the pinned list is read lazily, never while the flow is built (the wasm may not be loaded yet)", async () => {
  let reads = 0;
  const { session } = fakeSession();
  const steps = new AiSteps({
    session: () => session,
    models: () => {
      reads += 1;
      return [MODEL];
    },
    store: memoryModelStore(),
    fetch: async () => new Response(FILE, { status: 200 }),
    verify: () => true,
    runner: fakeRunner(),
    storage: undefined,
    changed: () => undefined,
  });
  assert.equal(reads, 0);
  await steps.refresh();
  await steps.refresh();
  assert.equal(reads, 1, "read once, on first use");
});
