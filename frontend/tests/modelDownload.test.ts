// The one network use of the app (`specs/0135` criteria 63 to 65, ADR 0076 §4, §5, §7): the
// request of a model download, resume by `Range`, restart on a 200, the pin checked before the
// file is committed, and the states a failed request leaves. A fake `fetch` stands for the host.
import { test } from "node:test";
import assert from "node:assert/strict";

import { downloadModel, downloadRequest, hasRoom, type PinnedModel } from "../src/lib/modelDownload.ts";
import { memoryModelStore } from "../src/lib/modelStore.ts";

const BYTES = new Uint8Array(Array.from({ length: 10_000 }, (_, i) => i % 251));

const MODEL: PinnedModel = {
  kind: "background",
  name: "u2netp",
  version: "1",
  bytes: BYTES.length,
  sha256: "ab".repeat(32),
  url: `https://models.curvyo.org/u2netp/1/${"ab".repeat(32)}.onnx`,
  licence_url: "https://models.curvyo.org/u2netp/1/LICENSE.txt",
  licence: "Apache-2.0",
  copyright: "",
  upstream: "",
  training_data: "",
  placeholder: false,
};

function body(bytes: Uint8Array, breakAfter?: number): ReadableStream<Uint8Array> {
  let sent = 0;
  return new ReadableStream({
    pull(controller) {
      if (breakAfter !== undefined && sent >= breakAfter) {
        controller.error(new Error("connection lost"));
        return;
      }
      const end = Math.min(bytes.length, sent + 1_000, breakAfter ?? Infinity);
      if (sent >= bytes.length) {
        controller.close();
        return;
      }
      controller.enqueue(bytes.slice(sent, end));
      sent = end;
    },
  });
}

interface Captured {
  url: string;
  init: RequestInit;
}

function host(answer: (init: RequestInit) => Response, captured: Captured[]) {
  return async (url: string, init: RequestInit) => {
    captured.push({ url, init });
    return answer(init);
  };
}

const sameBytes = (b: Uint8Array) => b.length === BYTES.length && b.every((v, i) => v === BYTES[i]);

test("criterion 63: the request has the fixed URL, no body, no credentials, no referrer", async () => {
  const captured: Captured[] = [];
  const store = memoryModelStore();
  const outcome = await downloadModel(
    MODEL,
    store,
    host(() => new Response(body(BYTES), { status: 200 }), captured),
    sameBytes,
    () => undefined,
  );
  assert.equal(outcome, "ready");
  assert.equal(captured.length, 1);
  const [{ url, init }] = captured;
  assert.equal(url, MODEL.url);
  assert.ok(!url.includes("?"));
  assert.equal(init.method, "GET");
  assert.equal(init.body, undefined);
  assert.equal(init.credentials, "omit");
  assert.equal(init.referrerPolicy, "no-referrer");
  assert.equal(init.redirect, "error");
  assert.equal(init.headers, undefined, "no header on a first download");
  assert.deepEqual(await store.status(MODEL.sha256), { state: "complete", bytes: BYTES.length });
});

test("criteria 63, 65: a broken download is kept partial and resumes with a Range request", async () => {
  const captured: Captured[] = [];
  const store = memoryModelStore();
  const first = await downloadModel(
    MODEL,
    store,
    host(() => new Response(body(BYTES, 6_000), { status: 200 }), captured),
    sameBytes,
    () => undefined,
  );
  assert.equal(first, "paused");
  const partial = await store.status(MODEL.sha256);
  assert.equal(partial.state, "partial");
  const stored = partial.state === "partial" ? partial.bytes : 0;
  assert.ok(stored > 0 && stored < BYTES.length);
  const second = await downloadModel(
    MODEL,
    store,
    host(
      () =>
        new Response(body(BYTES.slice(stored)), {
          status: 206,
          headers: { "Content-Range": `bytes ${stored}-${BYTES.length - 1}/${BYTES.length}` },
        }),
      captured,
    ),
    sameBytes,
    () => undefined,
  );
  assert.equal(second, "ready");
  assert.deepEqual(captured[1].init.headers, { Range: `bytes=${stored}-` }, "only the Range header");
});

test("a 200 answer to a resume restarts at zero", async () => {
  const store = memoryModelStore();
  await store.append(MODEL.sha256, 0, BYTES.slice(0, 3_000));
  const outcome = await downloadModel(MODEL, store, host(() => new Response(body(BYTES), { status: 200 }), []), sameBytes, () => undefined);
  assert.equal(outcome, "ready");
  assert.ok(sameBytes((await store.read(MODEL.sha256)) as Uint8Array));
});

test("ADR 0076 §5: a 206 that does not start at the stored length discards the partial file and restarts at zero", async () => {
  const store = memoryModelStore();
  await store.append(MODEL.sha256, 0, BYTES.slice(0, 3_000));
  const captured: Captured[] = [];
  const answer = (init: RequestInit) => {
    const range = (init.headers as Record<string, string> | undefined)?.Range;
    if (range) return new Response(body(BYTES.slice(2_000)), { status: 206, headers: { "Content-Range": `bytes 2000-${BYTES.length - 1}/${BYTES.length}` } });
    return new Response(body(BYTES), { status: 200 });
  };
  const outcome = await downloadModel(MODEL, store, host(answer, captured), sameBytes, () => undefined);
  assert.equal(outcome, "ready");
  assert.equal(captured.length, 2, "one resume, one restart");
  assert.equal((captured[0].init.headers as Record<string, string> | undefined)?.Range, "bytes=3000-");
  assert.equal((captured[1].init.headers as Record<string, string> | undefined)?.Range, undefined, "the restart asks from zero");
  assert.ok(sameBytes((await store.read(MODEL.sha256)) as Uint8Array));
});

test("criterion 64: a file whose checksum differs is removed and never committed", async () => {
  const store = memoryModelStore();
  const outcome = await downloadModel(MODEL, store, host(() => new Response(body(BYTES), { status: 200 }), []), () => false, () => undefined);
  assert.equal(outcome, "checksum");
  assert.deepEqual(await store.status(MODEL.sha256), { state: "absent" });
});

test("a file longer than its pin is cut off and removed", async () => {
  const store = memoryModelStore();
  const longer = new Uint8Array(BYTES.length + 5);
  const outcome = await downloadModel(MODEL, store, host(() => new Response(body(longer), { status: 200 }), []), () => true, () => undefined);
  assert.equal(outcome, "checksum");
  assert.deepEqual(await store.status(MODEL.sha256), { state: "absent" });
});

test("criterion 65 and ADR 0076 §7: host unreachable, withdrawn, server errors", async () => {
  const store = memoryModelStore();
  const offline = async () => {
    throw new TypeError("Failed to fetch");
  };
  assert.equal(await downloadModel(MODEL, store, offline, sameBytes, () => undefined), "unreachable");
  for (const [status, outcome] of [
    [404, "withdrawn"],
    [410, "withdrawn"],
    [503, "unreachable"],
    [403, "unreachable"],
  ] as const) {
    assert.equal(
      await downloadModel(MODEL, store, host(() => new Response(null, { status }), []), sameBytes, () => undefined),
      outcome,
      String(status),
    );
  }
});

test("ADR 0076 §8: the quota check asks for the size plus 16 MiB", async () => {
  const storage = (quota: number, usage: number) => ({ estimate: async () => ({ quota, usage }) }) as unknown as StorageManager;
  const mib = 1024 * 1024;
  assert.equal(await hasRoom(10 * mib, storage(100 * mib, 74 * mib)), true);
  assert.equal(await hasRoom(10 * mib, storage(100 * mib, 75 * mib)), false);
  assert.equal(await hasRoom(10 * mib, undefined), true, "no estimate: try");
  assert.deepEqual(downloadRequest(0).headers, undefined);
});

test("criterion 64: a complete answer shorter than the pin is not the pinned file: removed, checksum", async () => {
  const store = memoryModelStore();
  const short = BYTES.slice(0, 1_024);
  const outcome = await downloadModel(MODEL, store, host(() => new Response(body(short), { status: 200 }), []), () => true, () => undefined);
  assert.equal(outcome, "checksum");
  assert.deepEqual(await store.status(MODEL.sha256), { state: "absent" });
});
