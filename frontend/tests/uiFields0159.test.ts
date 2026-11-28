// The shared field components of `specs/0159-ui-consistency` (F24, F28; `docs/ui-elements.md`
// §10, §11): one look for number, text, search and rename fields, the label and the unit inside.
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
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

async function render(file: string, name: string, props: Record<string, unknown>): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const module = await server.ssrLoadModule(`/src/components/ui/${file}`);
  return dom.renderToString(react.createElement(module[name] as never, props as never) as never);
}

const FIELD_LOOK = /h-7 min-w-0 rounded-\[5px\] border bg-white text-sm/;

test("NumberField: a text input, label inside the left edge, unit inside the right, the one look", async () => {
  const html = await render("number-field.tsx", "NumberField", {
    label: "Start",
    unit: "mm",
    value: "12",
    "aria-label": "Arc start",
    onChange: () => undefined,
  });
  assert.match(html, /type="text"/);
  assert.match(html, /inputMode="decimal"/);
  assert.match(html, FIELD_LOOK);
  assert.match(html, /text-right/);
  assert.match(html, /tabular-nums/);
  assert.match(html, /<span aria-hidden="true" class="[^"]*left-1\.5[^"]*">Start<\/span>/);
  assert.match(html, /<span aria-hidden="true" class="[^"]*right-1\.5[^"]*"[^>]*>mm<\/span>/);
  assert.doesNotMatch(html, /aria-invalid/);
});

test("NumberField: invalid gets the field-invalid border and aria-invalid; limited sits before the unit", async () => {
  const html = await render("number-field.tsx", "NumberField", {
    unit: "mm",
    limited: true,
    invalid: true,
    value: "x",
    "aria-label": "Corner radius",
    onChange: () => undefined,
  });
  assert.match(html, /aria-invalid="true"/);
  assert.match(html, /border-\[var\(--field-invalid\)\] shadow-\[inset_0_0_0_1px_var\(--field-invalid\)\]/);
  assert.match(html, />limited<\/span>/);
});

test("SearchField: the Search glyph inside, the clear button only while it holds text", async () => {
  const empty = await render("search-field.tsx", "SearchField", {
    value: "",
    onValue: () => undefined,
    "aria-label": "Search fonts",
  });
  assert.match(empty, /lucide-search/);
  assert.doesNotMatch(empty, /Clear search/);
  assert.match(empty, FIELD_LOOK);
  const typed = await render("search-field.tsx", "SearchField", {
    value: "Ro",
    onValue: () => undefined,
    "aria-label": "Search fonts",
  });
  assert.match(typed, /aria-label="Clear search"/);
  assert.match(typed, /lucide-x/);
});

test("RenameField and TextField: the accent rename border, the text field look", async () => {
  const rename = await render("rename-field.tsx", "RenameField", { value: "Cut", "aria-label": "Name", onChange: () => undefined });
  assert.match(rename, /h-6/);
  assert.match(rename, /border-\[var\(--editor-accent\)\]/);
  assert.match(rename, /rounded-\[5px\]/);
  const text = await render("text-field.tsx", "TextField", { value: "", "aria-label": "Code", mono: true, onChange: () => undefined });
  assert.match(text, /font-mono/);
  assert.match(text, /rounded-\[5px\] border bg-white text-xs/);
});

function files(dir: URL): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = new URL(name, dir);
    return statSync(path).isDirectory() ? files(new URL(`${name}/`, dir)) : name.endsWith(".tsx") ? [path.pathname] : [];
  });
}

test("No field look outside the shared ones: no native number input, search and tag fields shared", () => {
  for (const file of files(new URL("../src/components/", import.meta.url))) {
    if (file.includes("/components/ui/")) {
      continue;
    }
    assert.doesNotMatch(readFileSync(file, "utf8"), /<input\s[^<]*type="number"/, file);
  }
  const src = (file: string) => readFileSync(new URL(`../src/components/${file}`, import.meta.url), "utf8");
  for (const file of ["panel/FontSearchRow.tsx", "fontPreview/PreviewHeader.tsx", "libraryBrowser/BrowserHeader.tsx", "panel/LibraryTab.tsx"]) {
    assert.match(src(file), /<SearchField/, file);
  }
  for (const file of ["panel/OperationRow.tsx", "panel/MaterialRow.tsx", "panel/FixtureJobRow.tsx", "panel/FormatList.tsx", "panel/LayerRow.tsx", "panel/WorkspaceNameField.tsx", "panel/PagesBlock.tsx", "panel/FontTagStrip.tsx", "workbench/FixtureFields.tsx"]) {
    assert.match(src(file), /<Rename(Input|Field)\s/, file);
  }
});

test("No native title on a control: buttons, links and fields use the shared Tooltip (0159 F54)", () => {
  for (const file of files(new URL("../src/components/", import.meta.url))) {
    assert.doesNotMatch(readFileSync(file, "utf8"), /<(button|input|a|label)\s[^<]*?\stitle=/, file);
  }
});
