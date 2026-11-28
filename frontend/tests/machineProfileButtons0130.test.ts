// `specs/0130-fix-machine-profile-rename` criteria 2a and 3 in the front end, as ADR 0098 revised
// them: the "Differs from profile" row of the Machine group keeps Reset from profile with
// `BookmarkCheck` (Update profile left the page, UX unify U6), and the armed presses stay
// `--destructive`. Rendered on the server through the vite harness of `acceptance_0098_tester`.

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

/** The Machine group's state-row buttons in DOM order (ADR 0098 decision 4, UX unify 4.1). */
async function stateRowButtons(): Promise<{ name: string; glyph: string }[]> {
  const g = globalThis as Record<string, unknown>;
  const hadWindow = "window" in g;
  g.window ??= {};
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const loaded = await server.ssrLoadModule("/src/components/workbench/MachinesTab.tsx");
  const html = dom.renderToString(
    react.createElement(
      tooltip.TooltipProvider as never,
      null,
      react.createElement(loaded.MachineGroup as never, {
        picker: { family: "laser", id: "u1", name: "Kitchen laser", state: "differs", profiles: [] },
        workbenches: { output: {} },
        onOpenMachines: () => {},
        onReturnFocus: () => {},
      }),
    ) as never,
  );
  if (!hadWindow) delete g.window;
  const row = html.slice(html.indexOf("Differs from profile"));
  return [...row.matchAll(/<button[^>]*aria-label="([^"]+)"[^>]*>\s*<svg[^>]*class="lucide lucide-([a-z0-9-]+)/g)].map(
    (m) => ({ name: m[1], glyph: m[2] }),
  );
}

test("2a: Differs from profile shows Reset from profile (BookmarkCheck); Update profile left the page (U6)", async () => {
  assert.deepEqual(await stateRowButtons(), [{ name: "Reset from profile", glyph: "bookmark-check" }]);
  assert.match(source("components/workbench/MachinesTab.tsx"), /content="Reset from profile"/);
  assert.doesNotMatch(source("components/workbench/MachinesTab.tsx"), /Update profile/);
});

test("3: the armed presses of the machine controls stay --destructive", async () => {
  const styles = await server.ssrLoadModule("/src/components/workbench/machineStyles.ts");
  assert.match(styles.ARMED_DESTRUCTIVE_BUTTON as string, /bg-\[var\(--destructive\)\] text-white/);
  // Forget pairing (Connector group).
  assert.match(source("components/workbench/ConnectorGroup.tsx"), /arm\.armed \? ARMED_DESTRUCTIVE_BUTTON : ICON_BUTTON/);
  assert.doesNotMatch(source("components/workbench/ConnectorGroup.tsx"), /--armed-accent|ARMED_ACCENT_BUTTON/);
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
