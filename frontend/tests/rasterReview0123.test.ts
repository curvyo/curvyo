// The UI fixes of the `specs/0123-raster-engraving` reviews (architect A3, UX U3 to U9): the
// Document strips keep their accepted keys and markup, the Dither strip's tooltips, its "Damaged"
// description, the test grid bar's Dither, the not-a-picture line and the cut numbers line.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { after, before, test } from "node:test";

import { asTarget } from "../src/lib/outputView.ts";
import { numbersTitle } from "../src/lib/outputText.ts";

const source = (file: string) => readFileSync(new URL(`../src/${file}`, import.meta.url), "utf8");

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

async function strip(props: Record<string, unknown>): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const presets = await server.ssrLoadModule("/src/components/panel/PresetStrip.tsx");
  const cells = ["a4", "a3", "letter"].map((id) => ({ id, content: id, label: id, tooltip: "" }));
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(presets.PresetStrip as never, {
      cells,
      value: "a3",
      onPress: () => undefined,
      onReturnFocus: () => undefined,
      ...props,
    }),
  );
  return dom.renderToString(element as never);
}

test("A3: a Document strip is the accepted one: horizontal with wrap, one row, no key wrapper", async () => {
  const html = await strip({ labelledBy: "size" });
  assert.match(html, /^<div role="radiogroup"[^>]*aria-labelledby="size"/, "the radiogroup is the root");
  assert.match(html, /aria-orientation="horizontal"/);
  assert.match(html, /class="flex w-\[244px\] flex-wrap gap-px overflow-hidden rounded-\[5px\] border"/);
  assert.doesNotMatch(html, /aria-describedby|class="contents"/);
  for (const file of ["components/panel/DocumentPresets.tsx", "components/panel/FormatForm.tsx"]) {
    assert.doesNotMatch(source(file), /fixedRows|describedBy|tooltipSide/, file);
  }
});

test("A3: only the Dither strips use the fixed rows (every arrow, no wrap, Home and End)", async () => {
  const html = await strip({ label: "Dither", fixedRows: { breakAfter: ["a3"] }, describedBy: "dmg" });
  assert.match(html, /^<div class="contents">/);
  assert.doesNotMatch(html, /aria-orientation/);
  assert.match(html, /aria-describedby="dmg"/);
  assert.equal(html.match(/class="flex flex-wrap gap-px"/g)?.length, 2, "two fixed rows");
  // Superseded by the Dither on the picture (2026-11-20): the strip is one component for the picture and the operation.
  assert.match(source("components/panel/DitherStrip.tsx"), /fixedRows=\{\{ breakAfter: STRIP_BREAK \}\}/);
});

test("U3, U5: a Dither cell has a tooltip only when its word is shortened; Damaged describes the strip", () => {
  // Superseded by the Dither on the picture (2026-11-20): one strip component for the picture and the operation.
  const group = source("components/panel/DitherStrip.tsx");
  assert.match(group, /tooltip: cellWord\(mode\.label\) === mode\.label \? "" : mode\.label/);
  assert.match(group, /describedBy=\{dither\.dither === null \? damagedId : undefined\}/);
  assert.match(group, /<p id=\{damagedId\}/);
});

test("U4, U9: the bar's Dither stands in two halves that wrap, and Home and End move and select", () => {
  const bar = source("components/TestGridBar.tsx");
  assert.match(bar, /<Segments<DitherMode>\s+halves/);
  assert.match(bar, /const ends = halves && \(event\.key === "Home" \|\| event\.key === "End"\)/);
  assert.match(bar, /HALVES_CLASS = `flex min-w-0 max-w-full flex-wrap/);
});

test("U6: the not-a-picture line selects its items and unfolds its operation's row", () => {
  const item = { peer: "1", counter: 2 };
  const op = { peer: "1", counter: 3 };
  assert.deepEqual(asTarget({ kind: "items", ids: [item], unfold: op }), { kind: "items", ids: [item], unfold: op });
  assert.deepEqual(asTarget({ kind: "items", ids: [], unfold: null }), { kind: "items", ids: [], unfold: null });
  assert.match(source("components/panel/OutputGroup.tsx"), /if \(target\.unfold !== null\) onUnfoldOperation\(target\.unfold\)/);
});

test("U7: a cut numbers line has the whole line as its title", () => {
  assert.equal(
    numbersTitle("90 %, 6,000 mm/min, 2 ×, 0.25 mm", ["power"], "Outline "),
    "Outline 90 % typed, 6,000 mm/min, 2 ×, 0.25 mm",
  );
  assert.match(source("components/panel/NumbersText.tsx"), /scrollWidth > span\.clientWidth/);
});
