// `specs/0130-fix-machine-profile-rename` criteria 2a and 3 in the front end: the order, names and
// glyphs of the two buttons of the "Differs from profile" row (Reset from profile first with
// `BookmarkCheck`, Update profile last with `RotateCcw`; Update hidden for the built-in), and the
// armed colour (Update profile in `--armed-accent`, every other armed press of the block in
// `--destructive`). Rendered on the server through the vite harness of `acceptance_0098_tester`.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
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

const SRC = path.resolve(import.meta.dirname, "..", "src");
const source = (rel: string) => readFileSync(path.join(SRC, rel), "utf8");

function machine(canUpdate: boolean) {
  return {
    copy: {
      id: "u1",
      name: "Generic laser copy",
      origin: "bottom_left",
      max_feed: 6000,
      max_power: 1000,
      laser_mode: true,
      width: 400,
      height: 300,
    },
    builtin: !canUpdate,
    state: "differs",
    can_update: canUpdate,
    name_notice: null,
    connection: null,
    library_notices: [],
    profiles: [
      { id: "generic-laser", name: "Generic laser", width: 400, height: 300, builtin: true, chosen: !canUpdate },
      { id: "u1", name: "Kitchen laser", width: 400, height: 300, builtin: false, chosen: canUpdate },
    ],
  };
}

/** The state row's buttons in DOM (and so Tab) order: accessible name and Lucide glyph id. */
async function stateRowButtons(canUpdate: boolean): Promise<{ name: string; glyph: string }[]> {
  // The block's module chain reads the host at load; a bare window stands in on the server.
  const g = globalThis as Record<string, unknown>;
  const hadWindow = "window" in g;
  g.window ??= {};
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const loaded = await server.ssrLoadModule("/src/components/workbench/MachineBlock.tsx");
  const html = dom.renderToString(
    react.createElement(
      tooltip.TooltipProvider as never,
      null,
      react.createElement(loaded.MachineBlock as never, {
        label: "Machine",
        machine: machine(canUpdate),
        workbenches: { machine: {}, setProfileWorkArea: () => "" },
      }),
    ) as never,
  );
  if (!hadWindow) delete g.window;
  const row = html.match(/<div data-machine-state-row=""[\s\S]*?<span role="status" class="sr-only">/);
  assert.ok(row, "the state row is rendered");
  return [...row[0].matchAll(/<button[^>]*aria-label="([^"]+)"[^>]*>\s*<svg[^>]*class="lucide lucide-([a-z0-9-]+)/g)].map(
    (m) => ({ name: m[1], glyph: m[2] }),
  );
}

test("2a: Differs from profile shows Reset from profile (BookmarkCheck) first, Update profile (RotateCcw) last", async () => {
  assert.deepEqual(await stateRowButtons(true), [
    { name: "Reset from profile", glyph: "bookmark-check" },
    { name: "Update profile", glyph: "rotate-ccw" },
  ]);
});

test("2a: for the built-in Update profile is hidden and Reset from profile is the only button", async () => {
  assert.deepEqual(await stateRowButtons(false), [{ name: "Reset from profile", glyph: "bookmark-check" }]);
});

test("2a: the tooltips and the armed text stay as built", () => {
  const block = source("components/workbench/MachineBlock.tsx");
  assert.match(block, /const ARMED_UPDATE = "Update profile\. Overwrites the saved profile\.";/);
  assert.match(block, /content="Reset from profile"/);
  assert.match(block, /content=\{update\.armed \? ARMED_UPDATE : "Update profile"\}/);
});

test("3: Update profile arms in --armed-accent, an alias of --editor-accent, with a white glyph", async () => {
  const styles = await server.ssrLoadModule("/src/components/workbench/machineStyles.ts");
  const accent = styles.ARMED_ACCENT_BUTTON as string;
  assert.match(accent, /bg-\[var\(--armed-accent\)\] text-white/);
  assert.doesNotMatch(accent, /--destructive|hover:bg/);
  assert.match(source("components/workbench/MachineBlock.tsx"), /update\.armed \? ARMED_ACCENT_BUTTON : ICON_BUTTON/);
  assert.match(source("index.css"), /--armed-accent: var\(--editor-accent\);/);
});

test("3: every other armed press of the Machine block stays --destructive", async () => {
  const styles = await server.ssrLoadModule("/src/components/workbench/machineStyles.ts");
  assert.match(styles.ARMED_DESTRUCTIVE_BUTTON as string, /bg-\[var\(--destructive\)\] text-white/);
  // Forget pairing (Connector group) and Delete in the profile list.
  assert.match(source("components/workbench/ConnectorGroup.tsx"), /arm\.armed \? ARMED_DESTRUCTIVE_BUTTON : ICON_BUTTON/);
  assert.match(source("components/workbench/MachineProfileList.tsx"), /bg-\[var\(--destructive\)\] text-white/);
  for (const file of ["components/workbench/ConnectorGroup.tsx", "components/workbench/MachineProfileList.tsx"]) {
    assert.doesNotMatch(source(file), /--armed-accent|ARMED_ACCENT_BUTTON/, file);
  }
});

test("3: white on the accent passes 4.5:1", () => {
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  const hex = source("index.css").match(/--editor-accent: #([0-9A-Fa-f]{6});/);
  assert.ok(hex, "--editor-accent is a hex value");
  const [r, g, b] = [0, 2, 4].map((i) => lin(Number.parseInt(hex[1].slice(i, i + 2), 16)));
  const luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  assert.ok(1.05 / (luminance + 0.05) >= 4.5, `white on the accent ${(1.05 / (luminance + 0.05)).toFixed(2)}:1`);
});
