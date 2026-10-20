// Every `Tooltip` and `SliderTip` needs a `TooltipProvider` above it, or React throws and the
// whole screen goes white (the Star mode of the polygon/star tool did, with the Ratio slider's
// tooltip). The app has one provider at its root; this checks that statically over the source tree,
// and renders the components that hold a tooltip on the server, with and without the provider.

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { after, before, test } from "node:test";

const SRC = new URL("../src/", import.meta.url);

function sources(dir: URL, found: string[] = []): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "wasm-bindings") {
      continue;
    }
    const child = new URL(entry.name + (entry.isDirectory() ? "/" : ""), dir);
    if (entry.isDirectory()) {
      sources(child, found);
    } else if (/\.(ts|tsx)$/.test(entry.name)) {
      found.push(child.pathname);
    }
  }
  return found;
}

const files = sources(SRC);
const text = (path: string) => readFileSync(path, "utf8");
const rel = (path: string) => path.slice(SRC.pathname.length);

test("the app root renders the one TooltipProvider, around everything", () => {
  const app = text(new URL("App.tsx", SRC).pathname);
  assert.match(app, /import \{ TooltipProvider \} from "@\/components\/ui\/tooltip"/);
  assert.equal((app.match(/<TooltipProvider>/g) ?? []).length, 1);
  assert.equal((app.match(/<\/TooltipProvider>/g) ?? []).length, 1);
  // Everything the app renders is inside it: the provider opens right after `return (` of App.
  assert.match(app, /return \(\s*(\/\/[^\n]*\n\s*)*<TooltipProvider>\s*<div className="flex h-screen/);
  const main = text(new URL("main.tsx", SRC).pathname);
  assert.match(main, /<App \/>/, "main.tsx mounts App, which owns the provider");
});

test("no other component declares a TooltipProvider of its own", () => {
  const owners = files
    .filter((path) => /<TooltipProvider[ >]/.test(text(path)))
    .map(rel);
  assert.deepEqual(owners, ["App.tsx"]);
});

test("every file that renders a Tooltip or a SliderTip is a component under App", () => {
  const users = files.filter((path) =>
    /from "@\/components\/(ui\/tooltip|SliderTip)"/.test(text(path)),
  );
  assert.ok(users.length >= 10, `only ${users.length} tooltip users found`);
  for (const path of users) {
    assert.ok(path.endsWith(".tsx"), `${rel(path)} is not a component`);
    // Not a module that mounts its own React root (a root outside App has no provider).
    assert.doesNotMatch(text(path), /createRoot\(/, rel(path));
  }
});

// ---- server render of the components that hold a slider tooltip ----------------------

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

async function render(element: (m: Record<string, any>) => unknown, withProvider: boolean) {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const node = element({ react, tooltip });
  const wrapped = withProvider
    ? react.createElement(tooltip.TooltipProvider as never, null, node as never)
    : node;
  return dom.renderToString(wrapped as never);
}

async function component(path: string, name: string) {
  const loaded = await server.ssrLoadModule(path);
  return loaded[name] as never;
}

const noop = () => undefined;

test("Star mode of the polygon/star bar renders with the provider and throws without it", async () => {
  const ShapeToolbar = await component("/src/components/ShapeToolbar.tsx", "ShapeToolbar");
  const element = ({ react }: { react: typeof import("react") }) =>
    react.createElement(ShapeToolbar, {
      tool: "polygon-star",
      polyStarMode: "star",
      polyStarPointCount: 5,
      polyStarRatio: 0.5,
      onSetPolyStarMode: noop,
      onSetPolyStarPointCount: noop,
      onSetPolyStarRatio: noop,
      onReturnFocus: noop,
    });
  const html = await render(element as never, true);
  assert.match(html, /aria-label="Star inner ratio of new stars"/);
  await assert.rejects(render(element as never, false), /TooltipProvider/);
});

test("the Select bar's Ratio slider renders with the provider", async () => {
  const SelectToolbar = await component("/src/components/SelectToolbar.tsx", "SelectToolbar");
  const bar = {
    radiusShown: false,
    radiusMixed: false,
    radius: 0,
    radiusLimited: false,
    radiusStored: 0,
    radiusCorners: null,
    removeRoundingShown: false,
    removeRoundingEnabled: false,
    pointsShown: true,
    pointsMixed: false,
    points: 5,
    ratioShown: true,
    ratioMixed: false,
    ratio: 0.5,
    objectToPathShown: false,
  };
  const element = ({ react }: { react: typeof import("react") }) =>
    react.createElement(SelectToolbar, {
      scaleStrokeWidth: true,
      onSetScaleStrokeWidth: noop,
      scaleCornerRadius: false,
      onSetScaleCornerRadius: noop,
      linkCorners: true,
      onSetLinkCorners: noop,
      bar,
      onSetRadius: () => "committed",
      onRemoveRounding: noop,
      onSetPointCount: noop,
      onPreviewRatio: noop,
      onCommitRatio: noop,
      onSetRatio: noop,
      onCancelRatio: noop,
      onConvertToPaths: noop,
      flipEnabled: false,
      flip: {
        flip: noop,
        notice: null,
        announcement: { id: 0, text: "" },
        handleKeyOutcome: () => false,
        announceText: noop,
        clearAnnouncement: noop,
      },
      // The Weld group of `specs/0068-laserable-lettering/`.
      weld: { gapMm: 0.1, setGap: () => null, weld: noop, busy: false, notice: null },
      // The ellipse group and Curve field of `specs/0021-ellipse-arcs-and-shaping/`.
      arcBar: {
        state: {
          ellipseShown: true,
          kind: "",
          kindGrey: "no-arc",
          start: 45,
          startMixed: false,
          end: 45,
          endMixed: false,
          anglesGrey: "",
          curveShown: true,
          curve: 100,
          curveMixed: false,
          curveGrey: "",
          curveLimited: false,
          curveStored: 0,
        },
        setKind: noop,
        makeWhole: noop,
        setAngle: () => "committed",
        setCurve: () => "committed",
        previewCurve: noop,
        commitCurve: noop,
        cancelCurve: noop,
      },
      onReturnFocus: noop,
    });
  const html = await render(element as never, true);
  assert.match(html, /aria-label="Star inner ratio"/);
  assert.match(html, /aria-label="Arc kind"/);
  assert.match(html, /aria-label="Curve"/);
  await assert.rejects(render(element as never, false), /TooltipProvider/);
});

test("a value field renders with the provider and throws without it", async () => {
  const ValueField = await component("/src/components/panel/ValueField.tsx", "ValueField");
  const element = ({ react }: { react: typeof import("react") }) =>
    react.createElement(ValueField, {
      label: "Width",
      name: "Stroke width",
      text: "0.25",
      bar: 0.17,
      mixed: false,
      resettable: false,
      unit: "mm",
      valueNow: 0.25,
      unitWords: "millimetres",
      valueMax: 20,
      defaultText: "0.25 mm",
      messages: {},
      onPreview: noop,
      onSettle: noop,
      onStep: noop,
      onReset: noop,
      onSubmit: () => "committed",
      onReturnFocus: noop,
    });
  const html = await render(element as never, true);
  assert.match(html, /aria-label="Stroke width"/);
  await assert.rejects(render(element as never, false), /TooltipProvider/);
});
