// The Share tab fits its panel (PR #281 feedback): nothing sticks out of the panel's sides and no
// button clips its own text, at the panel's one fixed width (design system "PropertiesPanel width").
const assert = require('node:assert/strict');

/** Fails with every element of `page`'s open tab panel that is wider than the panel allows. */
async function assertPanelFits(page, where) {
  const problems = await page.locator('[role=tabpanel]').evaluate((panel) => {
    const box = panel.getBoundingClientRect();
    const found = [];
    for (const element of panel.querySelectorAll('*')) {
      const r = element.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) continue;
      const name = `${element.tagName.toLowerCase()} "${(element.textContent || '').trim().slice(0, 40)}"`;
      if (r.right > box.right + 1 || r.left < box.left - 1) {
        found.push(`${name} sticks out (${Math.round(r.left)} to ${Math.round(r.right)}, panel ${Math.round(box.left)} to ${Math.round(box.right)})`);
      }
      if (element.tagName === 'BUTTON' && element.scrollWidth > element.clientWidth + 1) {
        found.push(`${name} clips its text (${element.scrollWidth} in ${element.clientWidth})`);
      }
    }
    return found;
  });
  assert.deepEqual(problems, [], `the Share tab fits its panel: ${where}`);
}

module.exports = { assertPanelFits };
