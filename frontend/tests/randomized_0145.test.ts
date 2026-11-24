// `specs/0145-shape-spiro-handles` slice 3, criteria 21 and 22 and `ux-notes.md` section 13: the
// Select bar's Randomized group is a named two-decimal field and a "Re-roll seed" button that
// is greyed (still in the Tab order) while no selected shape has a value. Server render of the
// component, as `acceptance_0021_tester.test.ts` does.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

type Server = {
  ssrLoadModule(url: string): Promise<Record<string, unknown>>;
  close(): Promise<void>;
};
let server: Server;

before(async () => {
  const { createServer } = await import("vite");
  server = (await createServer({
    configFile: new URL("../vite.config.ts", import.meta.url).pathname,
    server: { middlewareMode: true },
    appType: "custom",
    logLevel: "silent",
  })) as unknown as Server;
});

after(async () => {
  await server.close();
});

const noop = () => undefined;

const STATE = {
  randomizedShown: true,
  randomized: 0.3,
  randomizedMixed: false,
  randomizedLimited: false,
  randomizedStored: 0,
  rerollEnabled: true,
};

async function html(state: typeof STATE): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const mod = await server.ssrLoadModule("/src/components/ArcCurveGroups.tsx");
  const arc = {
    state,
    setRandomized: () => "committed",
    previewRandomized: noop,
    rerollSeed: noop,
    commitCurve: noop,
    cancelCurve: noop,
  };
  return dom.renderToString(
    react.createElement(mod.RandomizedGroup as never, { arc, onReturnFocus: noop } as never),
  );
}

function tagsLabelled(markup: string, name: string): string[] {
  const re = new RegExp(`<[a-z]+[^>]*aria-label="${name}"[^>]*>`, "g");
  return [...markup.matchAll(re)].map((m) => m[0]);
}

test("criterion 21: the field is named Randomized and shows two decimals", async () => {
  const markup = await html(STATE);
  assert.equal(tagsLabelled(markup, "Randomized").length, 1);
  assert.match(markup, />Randomized</);
  assert.match(markup, /0\.30/);
  const mixed = await html({ ...STATE, randomizedMixed: true });
  assert.match(mixed, /Mixed/);
  const limited = await html({ ...STATE, randomized: 10, randomizedLimited: true, randomizedStored: 12.4 });
  assert.match(limited, /limited/);
});

test("criterion 22: Re-roll seed is a live button with a value, greyed but present without one", async () => {
  const live = tagsLabelled(await html(STATE), "Re-roll seed")[0];
  assert.ok(live, "the button");
  assert.doesNotMatch(live, /aria-disabled="true"/);
  const grey = tagsLabelled(await html({ ...STATE, randomized: 0, rerollEnabled: false }), "Re-roll seed")[0];
  assert.ok(grey, "never hidden");
  assert.match(grey, /aria-disabled="true"/);
  assert.doesNotMatch(grey, /\sdisabled(=|\s|>)/, "stays in the Tab order");
});
