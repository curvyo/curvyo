# Optional browser acceptance checks

Run from the repository root against a running development server. These scripts
use Playwright with Chromium; they do not add a production or CI dependency.
`CURVYO_BROWSER_URL` overrides the default `http://127.0.0.1:5173/`.

The story validation installed tooling under the ignored `target/browser-check`:

```sh
npm install --prefix target/browser-check --no-save --package-lock=false playwright
export PLAYWRIGHT_BROWSERS_PATH="$PWD/target/browser-check/browsers"
node target/browser-check/node_modules/playwright/cli.js install chromium
export NODE_PATH="$PWD/target/browser-check/node_modules"
node frontend/tests/browser/pages.cjs
node frontend/tests/browser/many-pages.cjs
node frontend/tests/browser/snapping.cjs
```

On the minimal validation host Chromium libraries additionally require
`LD_LIBRARY_PATH="$PWD/target/browser-check/runtime/usr/lib"`.
Create `target/browser-check` for screenshots when using another installation.
Only one agent should drive the browser at a time. Screenshots go to
`target/browser-check/pages.png` and `target/browser-check/many-pages.png`.

`pages.cjs` checks the page list, Add, active-frame visibility, rename commit and
Escape/F2 focus, active-only resize, reorder, frame-only state, coordinate unit
conversion, cancelled grip drag and deletion protection. `many-pages.cjs` checks
200 rows and frames, first/last activation and a drawing interaction without JS
errors. Its wall-clock duration is an observation; the CPU performance budget is
asserted separately by Rust's `pages::two_hundred_pages_cpu_budget`.

`snapping.cjs` checks dedicated rail navigation, child-icon Shift activation,
keyboard repeat and Shift+Space, collapsed-panel behavior, all target controls,
zone normalization and validation, native field and document undo, and sticky
settings during drawing. Its screenshot is `target/browser-check/snapping-verified.png`.

`snapping-feedback.cjs` covers PR225 axes (all four masks, exported coordinates,
settings persistence/undo), no-button hover and simple-click placement, and Alt
inversion after a Select move has begun including Ctrl+Alt without copying.
Run with the same optional runtime variables as above. Its exported project
checks use Python's standard-library zipfile module; screenshots under
`target/browser-check` include `axes-*-hover.png` and `alt-*-held.png`.
