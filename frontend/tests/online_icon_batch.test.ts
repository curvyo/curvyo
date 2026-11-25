// The icon bodies of one screen go out together (`specs/0138-template-library`, ADR 0085 decision
// 5): one request per set for everything asked within the window, at most 100 names each, and an
// icon already held is not asked again. The host call is a stub; nothing is sent.

import assert from "node:assert/strict";
import { test } from "node:test";

import { createIconBatcher } from "../src/lib/iconBatch.ts";

function setup() {
  const asked: { key: string; prefix: string; names: string[] }[] = [];
  let run: (() => void) | null = null;
  const batcher = createIconBatcher(
    async (key, prefix, names) => {
      asked.push({ key, prefix, names });
      return names.map((n) => [n, `<svg id="${n}"/>`] as [string, string]);
    },
    (go) => {
      run = go;
    },
  );
  return { asked, batcher, fire: () => run?.() };
}

test("cells of one window are one request per set", async () => {
  const { asked, batcher, fire } = setup();
  const pending = [
    batcher.request("s", "mdi", "home"),
    batcher.request("s", "mdi", "cog"),
    batcher.request("s", "mdi", "home"),
    batcher.request("s", "tabler", "home"),
  ];
  assert.equal(asked.length, 0, "nothing is asked before the window ends");
  fire();
  const svgs = await Promise.all(pending);
  assert.equal(asked.length, 2);
  assert.deepEqual(asked[0]?.names, ["home", "cog"]);
  assert.equal(svgs[0], '<svg id="home"/>');
  assert.equal(svgs[2], '<svg id="home"/>');
});

test("at most 100 names go in one request", async () => {
  const { asked, batcher, fire } = setup();
  const pending = Array.from({ length: 150 }, (_, n) => batcher.request("s", "mdi", `n${n}`));
  fire();
  await Promise.all(pending);
  assert.deepEqual(asked.map((a) => a.names.length), [100, 50]);
});

test("an icon held is not asked again, and forget lets it be asked", async () => {
  const { asked, batcher, fire } = setup();
  const first = batcher.request("s", "mdi", "home");
  fire();
  await first;
  assert.equal(await batcher.request("s", "mdi", "home"), '<svg id="home"/>');
  assert.equal(asked.length, 1);
  batcher.forget("s");
  const again = batcher.request("s", "mdi", "home");
  fire();
  await again;
  assert.equal(asked.length, 2);
});

test("a failed request gives empty pictures, not an error", async () => {
  const batcher = createIconBatcher(
    () => Promise.reject(new Error("offline")),
    (go) => go(),
  );
  assert.equal(await batcher.request("s", "mdi", "home"), "");
});
