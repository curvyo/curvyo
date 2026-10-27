// The font listing an import matches against (`specs/0024-svg-import-export` criterion 106): an
// SVG import or Open waits until every block is listed, so a face in the Curvyo folder or among
// the system fonts is not taken for missing on the first import after start.

import assert from "node:assert/strict";
import { test } from "node:test";

import { createFontListing } from "../src/lib/fontListing.ts";

/** A loader that resolves when the test says so, counting its calls. */
function deferred(value: string) {
  let resolve: () => void = () => {};
  const gate = new Promise<void>((r) => {
    resolve = r;
  });
  const loader = {
    calls: 0,
    release: () => resolve(),
    load: async () => {
      loader.calls += 1;
      await gate;
      return value;
    },
  };
  return loader;
}

test("an import started before the lists arrived matches only after every block is listed", async () => {
  const included = deferred("included");
  const curvyo = deferred("curvyo");
  const system = deferred("system");
  const store: Partial<Record<string, string>> = {};
  const ensure = createFontListing(
    { included: included.load, curvyo: curvyo.load, system: system.load },
    store,
    () => {},
  );
  // App start: only the included index is asked for.
  void ensure(["included"]);
  let matched: string[] | null = null;
  const importing = ensure().then(() => {
    matched = Object.keys(store).sort();
  });
  included.release();
  await new Promise((r) => setTimeout(r, 0));
  assert.equal(matched, null, "phase 1 must not run with the system and Curvyo lists missing");
  curvyo.release();
  system.release();
  await importing;
  assert.deepEqual(matched, ["curvyo", "included", "system"]);
  assert.equal(included.calls, 1, "each block is listed once");
});

test("a later import does not list again", async () => {
  const included = deferred("included");
  included.release();
  let notified = 0;
  const ensure = createFontListing({ included: included.load }, {}, () => {
    notified += 1;
  });
  await ensure();
  await ensure();
  assert.equal(included.calls, 1);
  assert.equal(notified, 1);
});

test("a block that cannot be listed does not hold the import", async () => {
  const store: Partial<Record<string, string>> = {};
  const ensure = createFontListing(
    {
      included: async () => "included",
      system: async () => {
        throw new Error("no fontconfig");
      },
    },
    store,
    () => {},
  );
  await ensure();
  assert.deepEqual(Object.keys(store), ["included"]);
});
