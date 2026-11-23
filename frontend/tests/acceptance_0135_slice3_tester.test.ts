// Tester cases for `specs/0135-image-optimisation` slice 3 (criteria 47, 63 to 65), written from
// the specification before the implementation was read: the AI step flow with a fake session,
// fetch, store and worker. Nothing is requested when the rows are shown; the first Run asks
// before anything is downloaded; Cancel on the consent line downloads nothing; the one request is
// a plain GET of the fixed URL; a file that fails its pin is removed and never run; a cancelled
// download resumes with a Range request and no second consent; offline, withdrawn and storage
// states; only the download module calls the network for models.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";

import { AiSteps, type AiRun, type AiSession } from "../src/lib/aiSteps.ts";
import type { PinnedModel } from "../src/lib/modelDownload.ts";
import { memoryModelStore } from "../src/lib/modelStore.ts";

const BYTES = new Uint8Array(Array.from({ length: 50_000 }, (_, i) => (i * 13) % 256));
const SHA = "cd".repeat(32);
const MODEL: PinnedModel = {
  kind: "denoise",
  name: "scunet-color-real-psnr",
  version: "1",
  bytes: BYTES.length,
  sha256: SHA,
  url: `https://models.curvyo.org/scunet-color-real-psnr/1/${SHA}.onnx`,
  licence_url: "https://models.curvyo.org/scunet-color-real-psnr/1/LICENSE.txt",
  licence: "Apache-2.0",
  copyright: "",
  upstream: "",
  training_data: "",
  placeholder: false,
};

function stream(bytes: Uint8Array, from = 0, breakAt?: number, onChunk?: (sent: number) => void, signal?: AbortSignal | null): ReadableStream<Uint8Array> {
  let sent = from;
  return new ReadableStream({
    pull(c) {
      // A real fetch's body errors once its request's signal aborts.
      if (signal?.aborted) {
        c.error(new DOMException("aborted", "AbortError"));
        return;
      }
      if (breakAt !== undefined && sent >= breakAt) {
        c.error(new Error("lost"));
        return;
      }
      if (sent >= bytes.length) {
        c.close();
        return;
      }
      const end = Math.min(bytes.length, sent + 5_000, breakAt ?? Infinity);
      c.enqueue(bytes.slice(sent, end));
      sent = end;
      onChunk?.(sent);
    },
  });
}

interface Fixture {
  steps: AiSteps;
  states: [string, string, number][];
  requests: { url: string; init: RequestInit }[];
  runs: { kind: string; model: Uint8Array | null }[];
  stored: string[];
  store: ReturnType<typeof memoryModelStore>;
}

function fixture(opts: {
  answer?: (init: RequestInit, n: number) => Promise<Response> | Response;
  verify?: (bytes: Uint8Array) => boolean;
  quota?: number;
}): Fixture {
  const states: [string, string, number][] = [];
  const requests: { url: string; init: RequestInit }[] = [];
  const runs: { kind: string; model: Uint8Array | null }[] = [];
  const stored: string[] = [];
  const session: AiSession = {
    set_ai_model_state: (kind, state, bytes) => {
      states.push([kind, state, bytes]);
      return true;
    },
    ai_run_input_json: () => JSON.stringify({ kind: "denoise", factor: 1, width: 2, height: 1, hash: "ef".repeat(32) }),
    take_ai_run_pixels: () => new Uint8Array(8),
    set_ai_progress: () => undefined,
    end_ai_run: () => undefined,
    store_ai_result: (_row, hash) => {
      stored.push(hash);
      return "";
    },
    remove_ai_result: () => true,
  };
  let loaded = false;
  const runner: AiRun = {
    hasModel: () => loaded,
    run: async (kind, model) => {
      runs.push({ kind, model });
      loaded = true;
      return { pixels: new Uint8Array(8), width: 2, height: 1, grey: false };
    },
    cancel: () => undefined,
  };
  const store = memoryModelStore();
  const storage = opts.quota === undefined ? undefined : ({ estimate: async () => ({ quota: opts.quota, usage: 0 }), persist: async () => true } as unknown as StorageManager);
  const steps = new AiSteps({
    session: () => session,
    models: [MODEL],
    store,
    fetch: async (url, init) => {
      requests.push({ url, init });
      if (!opts.answer) throw new Error("no network in this test");
      return opts.answer(init, requests.length);
    },
    verify: (_kind, bytes) => (opts.verify ?? ((b) => b.length === BYTES.length && b.every((v, i) => v === BYTES[i])))(bytes),
    runner,
    storage,
    changed: () => undefined,
  });
  return { steps, states, requests, runs, stored, store };
}

const last = (f: Fixture) => f.states[f.states.length - 1];

test("criterion 63: showing the rows makes no request; the first Run asks and fetches nothing", async () => {
  const f = fixture({ answer: () => new Response(stream(BYTES), { status: 200 }) });
  await f.steps.refresh();
  await f.steps.refresh();
  assert.equal(f.requests.length, 0, "no request when the rows are shown");
  assert.deepEqual(last(f), ["denoise", "absent", 0]);
  await f.steps.run(0, "denoise");
  assert.equal(f.requests.length, 0, "consent first");
  assert.deepEqual(last(f), ["denoise", "asking", 0]);
  f.steps.dismiss("denoise");
  assert.deepEqual(last(f), ["denoise", "absent", 0]);
  assert.equal(f.requests.length, 0, "Cancel on the consent line downloads nothing");
  assert.equal(f.runs.length, 0);
});

test("criteria 47, 48, 63: Download makes one plain GET of the fixed URL, then the Run stores its result", async () => {
  const f = fixture({ answer: () => new Response(stream(BYTES), { status: 200 }) });
  await f.steps.run(0, "denoise");
  await f.steps.consent(0, "denoise");
  assert.equal(f.requests.length, 1);
  const { url, init } = f.requests[0];
  assert.equal(url, MODEL.url);
  assert.ok(!url.includes("?") && !url.includes("#"));
  assert.equal(init.method ?? "GET", "GET");
  assert.equal(init.body, undefined, "no body");
  assert.equal(init.credentials, "omit", "no cookie, no credentials");
  assert.equal(init.referrerPolicy, "no-referrer");
  const headers = new Headers(init.headers);
  assert.equal([...headers.keys()].length, 0, "no header on a first download");
  assert.equal(f.runs.length, 1);
  assert.deepEqual(f.runs[0].model, BYTES, "the verified file runs");
  assert.deepEqual(f.stored, ["ef".repeat(32)]);
  assert.deepEqual(await f.store.status(SHA), { state: "complete", bytes: BYTES.length });
  // The next Run needs no network.
  await f.steps.run(0, "denoise");
  assert.equal(f.requests.length, 1);
  assert.equal(f.runs.length, 2);
});

test("criterion 64: a file that fails its pin is removed, never run, and Try again downloads again", async () => {
  const f = fixture({ answer: () => new Response(stream(BYTES), { status: 200 }), verify: () => false });
  await f.steps.run(0, "denoise");
  await f.steps.consent(0, "denoise");
  assert.equal(last(f)[1], "checksum");
  assert.deepEqual(await f.store.status(SHA), { state: "absent" });
  assert.equal(f.runs.length, 0, "never run");
  await f.steps.consent(0, "denoise");
  assert.equal(f.requests.length, 2, "Try again downloads again");
  assert.equal(f.runs.length, 0);
});

test("criterion 64: a body longer than the pinned size is never kept", async () => {
  const longer = new Uint8Array(BYTES.length + 10);
  longer.set(BYTES);
  const f = fixture({ answer: () => new Response(stream(longer), { status: 200 }), verify: () => true });
  await f.steps.consent(0, "denoise");
  assert.equal(last(f)[1], "checksum");
  assert.deepEqual(await f.store.status(SHA), { state: "absent" });
  assert.equal(f.runs.length, 0);
});

test("criteria 63, 65: Cancel keeps a partial file that is never run; Run resumes with Range, no second consent", async () => {
  let steps: AiSteps | null = null;
  const f = fixture({
    answer: (init, n) => {
      if (n === 1) {
        return new Response(
          stream(
            BYTES,
            0,
            undefined,
            (sent) => {
              if (sent >= 20_000) steps?.cancel("denoise");
            },
            init.signal,
          ),
          { status: 200 },
        );
      }
      const range = new Headers(init.headers).get("Range");
      const from = Number(/bytes=(\d+)-/.exec(range ?? "")?.[1] ?? "0");
      return new Response(stream(BYTES, from), {
        status: 206,
        headers: { "Content-Range": `bytes ${from}-${BYTES.length - 1}/${BYTES.length}` },
      });
    },
  });
  steps = f.steps;
  await f.steps.consent(0, "denoise");
  const paused = await f.store.status(SHA);
  assert.equal(paused.state, "partial", JSON.stringify(f.states));
  assert.equal(last(f)[1], "paused");
  assert.equal(f.runs.length, 0, "a partial file is never run");
  const kept = paused.state === "partial" ? paused.bytes : 0;
  assert.ok(kept > 0 && kept < BYTES.length);
  await f.steps.refresh();
  assert.deepEqual(last(f), ["denoise", "paused", kept]);
  await f.steps.run(0, "denoise");
  assert.ok(!f.states.slice(-5).some((s) => s[1] === "asking"), "a partial file counts as consent");
  assert.equal(f.requests.length, 2);
  assert.equal(new Headers(f.requests[1].init.headers).get("Range"), `bytes=${kept}-`);
  assert.equal(f.runs.length, 1);
  assert.deepEqual(f.runs[0].model, BYTES, "the resumed file is whole and right");
});

test("criterion 65: offline and withdrawn hosts and too little storage never block or fail silently", async () => {
  const offline = fixture({});
  await offline.steps.consent(0, "denoise");
  assert.equal(last(offline)[1], "unreachable");
  assert.equal(offline.runs.length, 0);

  const gone = fixture({ answer: () => new Response("", { status: 404 }) });
  await gone.steps.consent(0, "denoise");
  assert.equal(last(gone)[1], "withdrawn");

  const broken = fixture({ answer: () => new Response("", { status: 500 }) });
  await broken.steps.consent(0, "denoise");
  assert.equal(last(broken)[1], "unreachable");

  const full = fixture({ answer: () => new Response(stream(BYTES), { status: 200 }), quota: 1_000 });
  await full.steps.consent(0, "denoise");
  assert.equal(last(full)[1], "storage");
  assert.equal(full.requests.length, 0, "nothing is fetched without room");
});

test("criterion 63: model requests come only from the download module, on a Run", () => {
  const src = path.resolve(import.meta.dirname, "..", "src");
  const files: string[] = [];
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      const p = path.join(dir, name);
      if (statSync(p).isDirectory()) {
        if (name !== "wasm-bindings") walk(p);
      } else if (/\.(ts|tsx)$/.test(name)) files.push(p);
    }
  };
  walk(src);
  const callers = files.filter((f) => /\bdownloadModel\(/.test(readFileSync(f, "utf8"))).map((f) => path.relative(src, f));
  assert.deepEqual(callers.sort(), ["lib/aiSteps.ts", "lib/modelDownload.ts"].sort());
  const hostNamed = files
    .filter((f) => readFileSync(f, "utf8").includes("models.curvyo.org"))
    .map((f) => path.relative(src, f));
  assert.deepEqual(hostNamed, [], "the URL comes from the wasm's pinned list only");
});
